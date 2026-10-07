use super::{CompatibilityRecord, NativeError, NativeObservation, NativeStatus};
use serde_json::{json, Value};
use std::time::Instant;

use futures_util::{SinkExt, StreamExt};
use std::time::Duration;
use tokio_tungstenite::tungstenite::{
    client::IntoClientRequest,
    http::{header::AUTHORIZATION, HeaderValue},
    protocol::{Message, WebSocketConfig},
    Error as WsError,
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream};
const LIMIT: usize = 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(2);
const FRESHNESS: Duration = Duration::from_millis(300);
type Socket = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

pub(super) struct Client {
    socket: Socket,
    next_id: u64,
}
fn auth_request(
    endpoint: &str,
    token: &str,
) -> Result<tokio_tungstenite::tungstenite::http::Request<()>, NativeError> {
    let mut request = endpoint
        .into_client_request()
        .map_err(|_| NativeError::Shape)?;
    let mut header = HeaderValue::from_str(&format!("Bearer {token}"))
        .map_err(|_| NativeError::Authentication)?;
    header.set_sensitive(true);
    request.headers_mut().insert(AUTHORIZATION, header);
    Ok(request)
}
fn wire_error(error: WsError) -> NativeError {
    match error {
        WsError::Capacity(_) => NativeError::Limit,
        WsError::Http(response) if response.status().as_u16() == 401 => NativeError::Authentication,
        _ => NativeError::Disconnected,
    }
}
impl Client {
    pub(super) async fn connect(
        endpoint: &str,
        token: &str,
        record: CompatibilityRecord,
    ) -> Result<Self, NativeError> {
        // Pinned 0.160 initialize_processor exempts only these upstream
        // backend identities from global originator/UA and implicit gateway
        // login mutation. Do not use a custom originating client name here.
        let passive_name = match record.protocol {
            "v2-thread-status-0.160" => "codex_app_server_daemon",
            _ => return Err(NativeError::Shape),
        };
        // Endpoint is created by our private loopback-port owner, never by user input.
        let request = auth_request(endpoint, token)?;
        let config = WebSocketConfig {
            max_message_size: Some(LIMIT),
            max_frame_size: Some(LIMIT),
            ..Default::default()
        };
        tokio::time::timeout(TIMEOUT,async {
            let (socket,_)=tokio_tungstenite::connect_async_with_config(request,Some(config),false).await.map_err(wire_error)?;
            let mut client=Self {socket,next_id:1};
            let result=client.request("initialize",json!({"clientInfo":{"name":passive_name,"version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":false}})).await?;
            let object=result.as_object().ok_or(NativeError::Shape)?;
            if object.len()!=4 || object.get("platformOs").and_then(Value::as_str)!=Some(record.os) || object.get("platformFamily").and_then(Value::as_str)!=Some("unix") || !object.get("codexHome").and_then(Value::as_str).is_some_and(|p|std::path::Path::new(p).is_absolute()) || !object.get("userAgent").and_then(Value::as_str).is_some_and(|v|v==record.user_agent_prefix || v.strip_prefix(record.user_agent_prefix).is_some_and(|tail|tail.starts_with(' '))) { return Err(NativeError::Shape); }
            client.socket.send(Message::Text(json!({"method":"initialized"}).to_string())).await.map_err(wire_error)?;
            Ok(client)
        }).await.map_err(|_|NativeError::Timeout)?
    }
    async fn request(&mut self, method: &str, params: Value) -> Result<Value, NativeError> {
        if !matches!(method, "initialize" | "thread/loaded/list" | "thread/read") {
            return Err(NativeError::UnsupportedRequest);
        }
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).ok_or(NativeError::Limit)?;
        self.socket
            .send(Message::Text(
                json!({"id":id,"method":method,"params":params}).to_string(),
            ))
            .await
            .map_err(wire_error)?;
        for _ in 0..=64 {
            let frame = self
                .socket
                .next()
                .await
                .ok_or(NativeError::Disconnected)?
                .map_err(wire_error)?;
            let text = match frame {
                Message::Text(text) => text,
                Message::Close(_) => return Err(NativeError::Disconnected),
                Message::Ping(_) | Message::Pong(_) => continue,
                _ => return Err(NativeError::Shape),
            };
            if text.len() > LIMIT {
                return Err(NativeError::Limit);
            }
            let value: Value = serde_json::from_str(&text).map_err(|_| NativeError::Shape)?;
            let object = value.as_object().ok_or(NativeError::Shape)?;
            if object.contains_key("method") {
                if object.contains_key("id") {
                    return Err(NativeError::UnsupportedRequest);
                }
                if object.len() != 2
                    || object.get("method").and_then(Value::as_str) != Some("thread/status/changed")
                {
                    return Err(NativeError::Shape);
                }
                let params = object
                    .get("params")
                    .and_then(Value::as_object)
                    .ok_or(NativeError::Shape)?;
                if params.len() != 2
                    || !params
                        .get("threadId")
                        .and_then(Value::as_str)
                        .is_some_and(super::valid_id)
                {
                    return Err(NativeError::Shape);
                }
                status(params.get("status").ok_or(NativeError::Shape)?)?;
                continue;
            }
            if object.len() != 2 || object.get("id").and_then(Value::as_u64) != Some(id) {
                return Err(NativeError::Shape);
            }
            return object.get("result").cloned().ok_or(NativeError::Shape);
        }
        Err(NativeError::Limit)
    }
    pub(super) async fn observe(&mut self, root: &str) -> Result<NativeObservation, NativeError> {
        let started_at = Instant::now();
        tokio::time::timeout(TIMEOUT, async {
            let before = loaded(
                &self
                    .request("thread/loaded/list", json!({"limit":64,"cursor":null}))
                    .await?,
            )?;
            let status = root_status(
                &self
                    .request("thread/read", json!({"threadId":root,"includeTurns":false}))
                    .await?,
                root,
            )?;
            let after = loaded(
                &self
                    .request("thread/loaded/list", json!({"limit":64,"cursor":null}))
                    .await?,
            )?;
            if started_at.elapsed() > FRESHNESS {
                return Err(NativeError::Stale);
            }
            let complete_singleton_root = before.len() == 1 && before[0] == root && after == before;
            Ok(NativeObservation {
                status,
                complete_singleton_root,
                started_at,
            })
        })
        .await
        .map_err(|_| NativeError::Timeout)?
    }
}
fn loaded(value: &Value) -> Result<Vec<String>, NativeError> {
    if value.get("nextCursor") != Some(&Value::Null) {
        return Err(NativeError::Shape);
    }
    let ids = value
        .get("data")
        .and_then(Value::as_array)
        .ok_or(NativeError::Shape)?;
    if ids.len() > 64 {
        return Err(NativeError::Limit);
    }
    let mut out = Vec::with_capacity(ids.len());
    for id in ids {
        let id = id
            .as_str()
            .filter(|id| super::valid_id(id))
            .ok_or(NativeError::Shape)?;
        if out.iter().any(|other| other == id) {
            return Err(NativeError::Shape);
        }
        out.push(id.to_owned());
    }
    Ok(out)
}
fn root_status(value: &Value, root: &str) -> Result<NativeStatus, NativeError> {
    let thread = value
        .get("thread")
        .and_then(Value::as_object)
        .ok_or(NativeError::Shape)?;
    if thread.get("id").and_then(Value::as_str) != Some(root)
        || thread.get("sessionId").and_then(Value::as_str) != Some(root)
        || !matches!(
            thread.get("source").and_then(Value::as_str),
            Some("cli" | "mcp")
        )
        || thread.get("parentThreadId").is_some_and(|v| !v.is_null())
    {
        return Err(NativeError::Root);
    }
    // Creator version is persisted metadata, not the version of the running server.
    if !thread
        .get("cliVersion")
        .and_then(Value::as_str)
        .is_some_and(|v| !v.is_empty() && v.len() <= 128 && !v.chars().any(char::is_control))
    {
        return Err(NativeError::Shape);
    }
    status(thread.get("status").ok_or(NativeError::Shape)?)
}
fn status(value: &Value) -> Result<NativeStatus, NativeError> {
    let object = value.as_object().ok_or(NativeError::Shape)?;
    match object.get("type").and_then(Value::as_str) {
        Some("active") if object.len() == 2 => {
            let flags = object
                .get("activeFlags")
                .and_then(Value::as_array)
                .ok_or(NativeError::Shape)?;
            if flags.len() > 2 {
                return Err(NativeError::Shape);
            }
            let mut approval = false;
            let mut input = false;
            for flag in flags {
                match flag.as_str() {
                    Some("waitingOnApproval") if !approval => approval = true,
                    Some("waitingOnUserInput") if !input => input = true,
                    _ => return Err(NativeError::Shape),
                }
            }
            Ok(if input {
                NativeStatus::UserInputPending
            } else if approval {
                NativeStatus::ApprovalPending
            } else {
                NativeStatus::Active
            })
        }
        Some("idle" | "notLoaded" | "systemError") if object.len() == 1 => {
            Ok(NativeStatus::Unavailable)
        }
        _ => Err(NativeError::Shape),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::{SinkExt, StreamExt};
    use std::time::Duration;
    use tokio_tungstenite::tungstenite::Message;
    fn record() -> CompatibilityRecord {
        CompatibilityRecord {
            version: "0.160.0",
            os: std::env::consts::OS,
            arch: std::env::consts::ARCH,
            user_agent_prefix: "fixture/0.160.0",
            protocol: "v2-thread-status-0.160",
            root_proof: "fixture",
            evidence: "fixture",
        }
    }
    async fn fixture(fault: &'static str) -> (String, tokio::task::JoinHandle<()>) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("ws://{}", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            let (tcp, _) = listener.accept().await.unwrap();
            let mut socket=tokio_tungstenite::accept_hdr_async(tcp,|request:&tokio_tungstenite::tungstenite::handshake::server::Request,response:tokio_tungstenite::tungstenite::handshake::server::Response| {
                if request.headers().get("Authorization").and_then(|v|v.to_str().ok())!=Some("Bearer fixture-secret") {
                    return Err(tokio_tungstenite::tungstenite::http::Response::builder().status(401).body(Some("unauthorized".into())).unwrap());
                }
                Ok(response)
            }).await;
            let Ok(ref mut socket) = socket else {
                return;
            };
            let mut lists = 0;
            while let Some(Ok(Message::Text(frame))) = socket.next().await {
                let request: Value = serde_json::from_str(&frame).unwrap();
                let method = request["method"].as_str().unwrap();
                if method == "initialized" {
                    assert!(request.get("id").is_none());
                    continue;
                }
                let result = match method {
                    "initialize" => {
                        if fault == "passive-init" {
                            assert_eq!(
                                request["params"]["clientInfo"]["name"],
                                "codex_app_server_daemon"
                            );
                            assert_eq!(
                                request["params"]["capabilities"],
                                json!({"experimentalApi":false})
                            );
                        }
                        json!({"userAgent":"fixture/0.160.0","codexHome":"/fixture","platformFamily":"unix","platformOs":std::env::consts::OS})
                    }
                    "thread/loaded/list" => {
                        assert_eq!(request["params"], json!({"limit":64,"cursor":null}));
                        lists += 1;
                        match fault {
                            "cursor" => json!({"data":["root"],"nextCursor":"more"}),
                            "missing-cursor" => json!({"data":["root"]}),
                            "overflow" => {
                                json!({"data":(0..65).map(|i|format!("id-{i}")).collect::<Vec<_>>(),"nextCursor":null})
                            }
                            "change" if lists == 2 => {
                                json!({"data":["root","child"],"nextCursor":null})
                            }
                            _ => json!({"data":["root"],"nextCursor":null}),
                        }
                    }
                    "thread/read" => {
                        assert_eq!(
                            request["params"],
                            json!({"threadId":"root","includeTurns":false})
                        );
                        json!({"thread":{"id":if fault=="child" {"child"} else {"root"},"sessionId":"root","parentThreadId":null,"source":"mcp","cliVersion":"0.160.0","status":{"type":"active","activeFlags":["waitingOnApproval"]}}})
                    }
                    _ => panic!("observer emitted unsupported method"),
                };
                if method == "thread/loaded/list" && lists == 1 {
                    match fault {
                        "request" => {
                            socket.send(Message::Text(json!({"id":99,"method":"item/commandExecution/requestApproval","params":{}}).to_string())).await.unwrap();
                            match tokio::time::timeout(Duration::from_secs(3),socket.next()).await.expect("observer did not close its connection") {
                                None | Some(Ok(Message::Close(_))) => {},
                                Some(Err(WsError::ConnectionClosed|WsError::AlreadyClosed|WsError::Protocol(tokio_tungstenite::tungstenite::error::ProtocolError::ResetWithoutClosingHandshake))) => {},
                                Some(Err(WsError::Io(error))) if matches!(error.kind(),std::io::ErrorKind::ConnectionReset|std::io::ErrorKind::ConnectionAborted|std::io::ErrorKind::UnexpectedEof) => {},
                                _ => panic!("observer sent a forbidden application response before close"),
                            }
                            return;
                        }
                        "malformed" => {
                            let _ = socket.send(Message::Text("invalid".into())).await;
                            return;
                        }
                        "oversized" => {
                            let _ = socket
                                .send(Message::Text("x".repeat(1024 * 1024 + 1)))
                                .await;
                            return;
                        }
                        "timeout" => {
                            tokio::time::sleep(Duration::from_secs(3)).await;
                            return;
                        }
                        "stale" => tokio::time::sleep(Duration::from_millis(350)).await,
                        "unknown-notification" => {
                            let _ = socket
                                .send(Message::Text(
                                    json!({"method":"unknown","params":{}}).to_string(),
                                ))
                                .await;
                            return;
                        }
                        _ => {}
                    }
                }
                let id = if fault == "wrong-id" && method == "thread/loaded/list" {
                    json!(99)
                } else {
                    request["id"].clone()
                };
                if socket
                    .send(Message::Text(json!({"id":id,"result":result}).to_string()))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        });
        (endpoint, task)
    }
    #[tokio::test]
    async fn authenticated_observer_reads_only_a_complete_exact_singleton_root() {
        let (endpoint, task) = fixture("").await;
        let mut client = Client::connect(&endpoint, "fixture-secret", record())
            .await
            .expect("authenticated fixture");
        let observation = client.observe("root").await.unwrap();
        assert_eq!(observation.status, NativeStatus::ApprovalPending);
        assert!(observation.complete_singleton_root);
        drop(client);
        task.abort();
    }
    #[tokio::test]
    async fn passive_initialize_uses_pinned_non_originating_identity_without_capabilities() {
        let (endpoint, task) = fixture("passive-init").await;
        let client = Client::connect(&endpoint, "fixture-secret", record()).await;
        drop(client);
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .unwrap()
            .unwrap();
    }

    #[tokio::test]
    async fn approval_request_receives_no_application_reply_before_observer_closes() {
        let (endpoint, task) = fixture("request").await;
        let mut client = Client::connect(&endpoint, "fixture-secret", record())
            .await
            .unwrap();
        assert_eq!(
            client.observe("root").await.err(),
            Some(NativeError::UnsupportedRequest)
        );
        drop(client);
        tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .expect("approval fixture did not finish")
            .expect("approval fixture assertion failed");
    }
    #[tokio::test]
    async fn malicious_fixture_client_reply_is_detected_by_the_no_reply_assertion() {
        let (endpoint, task) = fixture("request").await;
        let mut client = Client::connect(&endpoint, "fixture-secret", record())
            .await
            .unwrap();
        assert_eq!(
            client.observe("root").await.err(),
            Some(NativeError::UnsupportedRequest)
        );
        // Negative control is entirely test-only and connected only to fixture().
        // No production observer response path is added or modified.
        client
            .socket
            .send(Message::Text(json!({"id":99,"result":null}).to_string()))
            .await
            .unwrap();
        drop(client);
        let error = tokio::time::timeout(Duration::from_secs(3), task)
            .await
            .expect("approval fixture did not finish")
            .expect_err("fixture accepted a forbidden application reply");
        let panic = error.into_panic();
        assert_eq!(
            panic.downcast_ref::<&str>().copied(),
            Some("observer sent a forbidden application response before close")
        );
    }
    #[tokio::test]
    async fn unsafe_wire_cycles_disconnect_without_answering_requests() {
        for (fault, expected) in [
            ("wrong-id", NativeError::Shape),
            ("malformed", NativeError::Shape),
            ("oversized", NativeError::Limit),
            ("timeout", NativeError::Timeout),
            ("cursor", NativeError::Shape),
            ("missing-cursor", NativeError::Shape),
            ("overflow", NativeError::Limit),
            ("child", NativeError::Root),
            ("unknown-notification", NativeError::Shape),
            ("stale", NativeError::Stale),
        ] {
            let (endpoint, task) = fixture(fault).await;
            let mut client = Client::connect(&endpoint, "fixture-secret", record())
                .await
                .unwrap();
            assert_eq!(
                client.observe("root").await.err(),
                Some(expected),
                "{fault}"
            );
            drop(client);
            task.abort();
        }
    }
    #[tokio::test]
    async fn changed_loaded_set_cannot_establish_singleton_authority() {
        let (endpoint, task) = fixture("change").await;
        let mut client = Client::connect(&endpoint, "fixture-secret", record())
            .await
            .unwrap();
        assert!(
            !client
                .observe("root")
                .await
                .unwrap()
                .complete_singleton_root
        );
        drop(client);
        task.abort();
    }
    #[tokio::test]
    async fn missing_or_wrong_auth_is_rejected() {
        for token in ["", "wrong"] {
            let (endpoint, task) = fixture("").await;
            assert!(matches!(
                Client::connect(&endpoint, token, record()).await,
                Err(NativeError::Authentication)
            ));
            task.abort();
        }
    }

    #[test]
    fn authorization_headers_do_not_debug_the_capability() {
        let request = auth_request("ws://127.0.0.1:1234", "fixture-secret-sentinel").unwrap();
        assert!(!format!("{request:?}").contains("fixture-secret-sentinel"));
        assert!(request.headers()[AUTHORIZATION].is_sensitive());
    }
    #[test]
    fn complete_pages_require_explicit_terminal_cursor_and_bounded_ids() {
        assert_eq!(
            loaded(&json!({"data":["root"],"nextCursor":null})).unwrap(),
            ["root"]
        );
        for bad in [
            json!({"data":["root"]}),
            json!({"data":["root"],"nextCursor":"next"}),
            json!({"data":vec!["root";65],"nextCursor":null}),
            json!({"data":["root","root"],"nextCursor":null}),
            json!({"data":[3],"nextCursor":null}),
        ] {
            assert!(loaded(&bad).is_err());
        }
    }
    #[test]
    fn root_requires_full_identity_version_and_status_shape() {
        let root = json!({"thread":{"id":"root","sessionId":"root","source":"mcp","cliVersion":"0.160.0","parentThreadId":null,"status":{"type":"active","activeFlags":[]}}});
        assert_eq!(root_status(&root, "root").unwrap(), NativeStatus::Active);
        for (key, value) in [
            ("id", json!("child")),
            ("sessionId", json!("other")),
            ("parentThreadId", json!("root")),
            ("source", json!({"subAgent":"spawn"})),
            ("cliVersion", json!(3)),
            ("status", json!({"type":"active","activeFlags":["unknown"]})),
        ] {
            let mut bad = root.clone();
            bad["thread"][key] = value;
            assert!(root_status(&bad, "root").is_err());
        }
        let mut resumed = root.clone();
        resumed["thread"]["cliVersion"] = json!("0.100.0");
        assert_eq!(root_status(&resumed, "root").unwrap(), NativeStatus::Active);
        for (flags, status) in [
            (vec!["waitingOnApproval"], NativeStatus::ApprovalPending),
            (vec!["waitingOnUserInput"], NativeStatus::UserInputPending),
            (
                vec!["waitingOnApproval", "waitingOnUserInput"],
                NativeStatus::UserInputPending,
            ),
        ] {
            let mut value = root.clone();
            value["thread"]["status"]["activeFlags"] = json!(flags);
            assert_eq!(root_status(&value, "root").unwrap(), status);
        }
    }
}
