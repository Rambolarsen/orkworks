//! Listener-wide API access control (ADR 0117).
//!
//! Every route except `/health` requires one of three credentials:
//!
//! - the renderer token (`ORKWORKS_RENDERER_TOKEN`), sent by the desktop
//!   renderer and Electron main as `x-orkworks-renderer-token`, or offered as
//!   an `orkworks.token.<token>` WebSocket subprotocol because browsers cannot
//!   set headers on a WebSocket handshake;
//! - the privileged desktop token (`x-orkworks-open-plan-token`), which stays
//!   in Electron main and is still checked again by the handlers that need it;
//! - a live per-session report token (`Authorization: Bearer`), accepted only
//!   on the routes a coding tool inside that session reports to.
//!
//! Requests carrying a browser `Origin` that is not the desktop renderer are
//! rejected before any credential is considered. Passing this layer never
//! replaces a handler's own authorization.

use axum::{
    extract::{Request, State},
    http::{header, HeaderMap, Method, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

pub(crate) const RENDERER_TOKEN_ENV: &str = "ORKWORKS_RENDERER_TOKEN";
pub(crate) const RENDERER_TOKEN_HEADER: &str = "x-orkworks-renderer-token";
/// Subprotocol the sidecar selects on authenticated WebSocket handshakes.
pub(crate) const WEBSOCKET_PROTOCOL: &str = "orkworks.v1";
const WEBSOCKET_TOKEN_PREFIX: &str = "orkworks.token.";
const DESKTOP_TOKEN_ENV: &str = "ORKWORKS_OPEN_PLAN_TOKEN";
const DESKTOP_TOKEN_HEADER: &str = "x-orkworks-open-plan-token";

#[derive(Clone)]
pub(crate) struct ApiAccess {
    /// `None` leaves the listener open. Only test fixtures can build that.
    renderer_token: Option<String>,
}

impl ApiAccess {
    pub(crate) fn enforced(renderer_token: String) -> Self {
        Self {
            renderer_token: Some(renderer_token),
        }
    }

    /// Reads the renderer token the desktop app supplied. A sidecar started
    /// without one fails closed: it gets a random token nobody knows, so only
    /// `/health`, desktop-token and session-token callers are served.
    pub(crate) fn from_env() -> Self {
        match std::env::var(RENDERER_TOKEN_ENV) {
            Ok(token) if !token.is_empty() => Self::enforced(token),
            _ => {
                tracing::warn!(
                    "{RENDERER_TOKEN_ENV} is not set; renderer API routes will reject every request"
                );
                let mut bytes = [0u8; 32];
                getrandom::fill(&mut bytes).expect("operating system randomness");
                Self::enforced(hex::encode(bytes))
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn open_for_tests() -> Self {
        Self {
            renderer_token: None,
        }
    }
}

/// True when `origin` is absent (native client), the packaged file renderer,
/// or a loopback HTTP origin (development renderer). Chromium serializes a
/// `file:` page's origin as `null` on HTTP requests and as `file://` on
/// WebSocket handshakes, so both are the packaged renderer.
pub(crate) fn origin_allowed(origin: Option<&str>) -> bool {
    let Some(origin) = origin else {
        return true;
    };
    if origin == "null" || origin == "file://" {
        return true;
    }
    url::Url::parse(origin).is_ok_and(|url| {
        url.scheme() == "http"
            && url.username().is_empty()
            && url.password().is_none()
            && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none()
    })
}

pub(crate) fn request_origin_allowed(headers: &HeaderMap) -> bool {
    match headers.get(header::ORIGIN) {
        None => true,
        Some(value) => value
            .to_str()
            .is_ok_and(|origin| origin_allowed(Some(origin))),
    }
}

pub(crate) fn constant_time_eq(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn header_matches(headers: &HeaderMap, name: &str, expected: &str) -> bool {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|candidate| constant_time_eq(candidate, expected))
}

fn websocket_protocol_matches(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get_all(header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|protocol| protocol.trim().strip_prefix(WEBSOCKET_TOKEN_PREFIX))
        .any(|candidate| constant_time_eq(candidate, expected))
}

fn desktop_token_matches(headers: &HeaderMap) -> bool {
    std::env::var(DESKTOP_TOKEN_ENV)
        .ok()
        .filter(|token| !token.is_empty())
        .is_some_and(|token| header_matches(headers, DESKTOP_TOKEN_HEADER, &token))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
}

/// Routes a coding tool reports to from inside its own session.
enum SessionRoute<'a> {
    /// Scoped to one session: the token must belong to that session.
    Session(&'a str),
    /// Taskmaster handoff routes: any live session token.
    Recommendation,
}

fn session_route<'a>(method: &Method, path: &'a str) -> Option<SessionRoute<'a>> {
    let segments: Vec<&str> = path.trim_matches('/').split('/').collect();
    match (method, segments.as_slice()) {
        (
            &Method::POST,
            ["sessions", id, "attention" | "harness-session" | "plan-path" | "workflow-observations"],
        ) => Some(SessionRoute::Session(id)),
        (&Method::GET | &Method::PATCH, ["sessions", id, "metadata"]) => {
            Some(SessionRoute::Session(id))
        }
        (&Method::GET, ["taskmaster", "recommendations"])
        | (&Method::GET, ["taskmaster", "recommendations", _])
        | (
            &Method::POST,
            ["taskmaster", "recommendations", _, "accept" | "complete" | "completion-packet"],
        ) => Some(SessionRoute::Recommendation),
        _ => None,
    }
}

fn session_token_authorizes(method: &Method, path: &str, headers: &HeaderMap) -> bool {
    let Some(token) = bearer_token(headers) else {
        return false;
    };
    match session_route(method, path) {
        Some(SessionRoute::Session(id)) => {
            crate::runtime::terminal_runtime::verify_workflow_report_token(id, token)
        }
        Some(SessionRoute::Recommendation) => {
            crate::runtime::terminal_runtime::workflow_report_session_for_token(token).is_some()
        }
        None => false,
    }
}

pub(crate) async fn require_api_access(
    State(access): State<ApiAccess>,
    request: Request,
    next: Next,
) -> Response {
    let headers = request.headers();
    if !request_origin_allowed(headers) {
        return StatusCode::FORBIDDEN.into_response();
    }
    let path = request.uri().path();
    if path == "/health" {
        return next.run(request).await;
    }
    let Some(renderer_token) = access.renderer_token.as_deref() else {
        return next.run(request).await;
    };
    if header_matches(headers, RENDERER_TOKEN_HEADER, renderer_token)
        || websocket_protocol_matches(headers, renderer_token)
        || desktop_token_matches(headers)
        || session_token_authorizes(request.method(), path, headers)
    {
        return next.run(request).await;
    }
    StatusCode::UNAUTHORIZED.into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::*;
    use axum::body::Body;
    use tower::ServiceExt;

    const TOKEN: &str = "renderer-test-token";

    fn router() -> axum::Router {
        let dir = tempfile::tempdir().unwrap();
        let state = test_app_state_with_workspace(dir.path());
        crate::build_router_with_access(state, ApiAccess::enforced(TOKEN.into()))
    }

    async fn status(request: axum::http::Request<Body>) -> StatusCode {
        router().oneshot(request).await.unwrap().status()
    }

    fn get(path: &str) -> axum::http::request::Builder {
        axum::http::Request::builder().method("GET").uri(path)
    }

    #[tokio::test]
    async fn health_needs_no_credential_and_other_routes_do() {
        assert_eq!(
            status(get("/health").body(Body::empty()).unwrap()).await,
            StatusCode::OK
        );
        for path in ["/sessions", "/harnesses", "/providers"] {
            assert_eq!(
                status(get(path).body(Body::empty()).unwrap()).await,
                StatusCode::UNAUTHORIZED,
                "{path}"
            );
            assert_eq!(
                status(
                    get(path)
                        .header(RENDERER_TOKEN_HEADER, "wrong")
                        .body(Body::empty())
                        .unwrap()
                )
                .await,
                StatusCode::UNAUTHORIZED,
                "{path}"
            );
            assert_eq!(
                status(
                    get(path)
                        .header(RENDERER_TOKEN_HEADER, TOKEN)
                        .body(Body::empty())
                        .unwrap()
                )
                .await,
                StatusCode::OK,
                "{path}"
            );
        }
    }

    #[tokio::test]
    async fn session_creation_and_terminal_attach_reject_missing_credentials() {
        let create = axum::http::Request::builder()
            .method("POST")
            .uri("/sessions")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"harnessId":"generic-shell"}"#))
            .unwrap();
        assert_eq!(status(create).await, StatusCode::UNAUTHORIZED);

        let attach = |protocol: Option<&str>| {
            let mut builder = get("/sessions/any/terminal")
                .header("connection", "upgrade")
                .header("upgrade", "websocket")
                .header("sec-websocket-version", "13")
                .header("sec-websocket-key", "dGhlIHNhbXBsZSBub25jZQ==");
            if let Some(protocol) = protocol {
                builder = builder.header("sec-websocket-protocol", protocol);
            }
            builder.body(Body::empty()).unwrap()
        };
        assert_eq!(status(attach(None)).await, StatusCode::UNAUTHORIZED);
        assert_eq!(
            status(attach(Some("orkworks.v1, orkworks.token.wrong"))).await,
            StatusCode::UNAUTHORIZED
        );
        // `oneshot` has no upgradable connection, so an authorized handshake
        // gets past this layer and is refused by the WebSocket extractor.
        assert_ne!(
            status(attach(Some(&format!(
                "orkworks.v1, orkworks.token.{TOKEN}"
            ))))
            .await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn foreign_browser_origins_are_rejected_even_with_a_valid_token() {
        for (origin, expected) in [
            ("https://evil.example", StatusCode::FORBIDDEN),
            ("http://localhost.evil.example", StatusCode::FORBIDDEN),
            ("http://localhost:5173", StatusCode::OK),
            ("null", StatusCode::OK),
        ] {
            assert_eq!(
                status(
                    get("/sessions")
                        .header("origin", origin)
                        .header(RENDERER_TOKEN_HEADER, TOKEN)
                        .body(Body::empty())
                        .unwrap()
                )
                .await,
                expected,
                "{origin}"
            );
        }
        assert_eq!(
            status(
                get("/health")
                    .header("origin", "https://evil.example")
                    .body(Body::empty())
                    .unwrap()
            )
            .await,
            StatusCode::FORBIDDEN
        );
    }

    #[tokio::test]
    async fn cors_does_not_grant_foreign_origins() {
        let preflight = |origin: &str| {
            axum::http::Request::builder()
                .method("OPTIONS")
                .uri("/sessions")
                .header("origin", origin)
                .header("access-control-request-method", "POST")
                .header("access-control-request-headers", RENDERER_TOKEN_HEADER)
                .body(Body::empty())
                .unwrap()
        };
        let foreign = router()
            .oneshot(preflight("https://evil.example"))
            .await
            .unwrap();
        assert!(foreign
            .headers()
            .get("access-control-allow-origin")
            .is_none());
        let renderer = router()
            .oneshot(preflight("http://localhost:5173"))
            .await
            .unwrap();
        assert_eq!(
            renderer.headers()["access-control-allow-origin"],
            "http://localhost:5173"
        );
    }

    #[tokio::test]
    async fn session_tokens_authorize_only_their_own_reporting_routes() {
        let session = "api-auth-session";
        let other = "api-auth-other-session";
        let token = "api-auth-report-token";
        crate::runtime::terminal_runtime::set_workflow_report_token(session, token.into());
        let bearer = format!("Bearer {token}");
        let request = |method: &str, path: String| {
            axum::http::Request::builder()
                .method(method)
                .uri(path)
                .header("authorization", &bearer)
                .header("content-type", "application/json")
                .body(Body::from("{}"))
                .unwrap()
        };

        for route in ["attention", "harness-session", "plan-path"] {
            assert_ne!(
                status(request("POST", format!("/sessions/{session}/{route}"))).await,
                StatusCode::UNAUTHORIZED,
                "{route}"
            );
            assert_eq!(
                status(request("POST", format!("/sessions/{other}/{route}"))).await,
                StatusCode::UNAUTHORIZED,
                "{route}"
            );
        }
        assert_ne!(
            status(request("GET", "/taskmaster/recommendations".into())).await,
            StatusCode::UNAUTHORIZED
        );
        for (method, path) in [
            ("GET", "/sessions".to_string()),
            ("POST", "/sessions".to_string()),
            ("GET", format!("/sessions/{session}/terminal")),
            ("GET", format!("/sessions/{session}/terminal-output")),
            ("DELETE", format!("/sessions/{session}")),
            ("POST", "/harnesses".to_string()),
            ("POST", "/workspace".to_string()),
        ] {
            assert_eq!(
                status(request(method, path.clone())).await,
                StatusCode::UNAUTHORIZED,
                "{method} {path}"
            );
        }

        crate::runtime::terminal_runtime::clear_workflow_report_token(session);
        assert_eq!(
            status(request("POST", format!("/sessions/{session}/attention"))).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[test]
    fn origin_policy_matches_the_desktop_renderer_only() {
        assert!(origin_allowed(None));
        assert!(origin_allowed(Some("null")));
        // Electron sends exactly this on WebSocket handshakes from the packaged app.
        assert!(origin_allowed(Some("file://")));
        assert!(origin_allowed(Some("http://127.0.0.1:5173")));
        assert!(!origin_allowed(Some("https://localhost:5173")));
        assert!(!origin_allowed(Some("file:///tmp/page.html")));
    }
}
