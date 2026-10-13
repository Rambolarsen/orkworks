//! Real authenticated build_router and Electron-client boundary; Node is required.
use super::*;
use crate::taskmaster::knowledge::test_support::{fixture, TEST_PUBLIC_KEY};
use base64::{engine::general_purpose::STANDARD, Engine as _};

async fn server(state: Arc<AppState>, fixture_key: bool) -> (String, tokio::task::JoinHandle<()>) {
    std::env::set_var("ORKWORKS_OPEN_PLAN_TOKEN", "taskmaster-test-token");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let mut router = crate::build_router(state);
    if fixture_key {
        router = router.layer(axum::Extension(KnowledgeTestKey(TEST_PUBLIC_KEY)));
    }
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (url, task)
}

fn maximum_activation() -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"activationFormatVersion":1,
        "bundleEnvelopeBase64":STANDARD.encode(fixture("maximum-bundle-envelope.json")),
        "manifestEnvelopeBase64":STANDARD.encode(fixture("maximum-manifest-envelope.json"))}))
    .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_electron_client_maximum_activation_persists_through_authenticated_route() {
    let directory = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(directory.path());
    let root = runtime_for(&state).root_path().to_path_buf();
    let (url, server) = server(state, true).await;
    let bytes = maximum_activation();
    assert_eq!(bytes.len(), 5_592_491);
    let packet = directory.path().join("packet.json");
    std::fs::write(&packet, &bytes).unwrap();
    let client = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../apps/desktop/electron/taskmasterSettings.ts");
    let port = url.rsplit(':').next().unwrap().to_owned();
    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("node")
            .args(["--experimental-strip-types", "--input-type=module", "-e", r#"
                import { readFileSync } from 'node:fs';
                import { pathToFileURL } from 'node:url';
                const { taskmasterRequest } = await import(pathToFileURL(process.argv[1]).href);
                const payload = JSON.parse(readFileSync(process.argv[2], 'utf8'));
                const status = await taskmasterRequest(Number(process.argv[3]), 'taskmaster-test-token', 'knowledge', payload);
                if (status.knowledgeVersion !== 'maximum-envelope') throw new Error(JSON.stringify(status));
                process.stdout.write('activated');
            "#])
            .arg(client).arg(packet).arg(port).output().expect("Node is required for the real Electron transport test")
    }).await.unwrap();
    assert!(
        output.status.success(),
        "Electron transport failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(output.stdout, b"activated");
    let restarted = TaskmasterRuntime::open_with_test_key(root, TEST_PUBLIC_KEY);
    assert_eq!(
        restarted.status(None).knowledge_version.as_deref(),
        Some("maximum-envelope")
    );
    assert!(restarted.brain_knowledge_availability().is_ok());
    server.abort();
}

#[tokio::test]
async fn authenticated_raw_route_has_inclusive_limit_and_preserves_current_proof_on_rejection() {
    let directory = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(directory.path());
    let runtime = TaskmasterRuntime::open_with_test_key(
        runtime_for(&state).root_path().into(),
        TEST_PUBLIC_KEY,
    );
    let (url, server) = server(state, true).await;
    let endpoint = format!("{url}/settings/taskmaster/knowledge");
    let client = reqwest::Client::new();
    let request = |bytes: Vec<u8>| {
        client
            .post(&endpoint)
            .header("x-orkworks-open-plan-token", "taskmaster-test-token")
            .header("content-type", "application/json")
            .body(bytes)
    };
    let mut exact = maximum_activation();
    exact.resize(crate::taskmaster::knowledge::MAX_ACTIVATION_BYTES, b' ');
    assert_eq!(
        request(exact.clone()).send().await.unwrap().status(),
        StatusCode::OK
    );
    let key = crate::taskmaster::knowledge::test_key_sha256(TEST_PUBLIC_KEY).unwrap();
    let path = runtime
        .root_path()
        .join("knowledge")
        .join(key)
        .join("activation.json");
    let prior = std::fs::read(&path).unwrap();
    assert_eq!(prior, exact);
    let ledger = std::fs::read(runtime.root_path().join("evaluations.json")).unwrap();
    exact.push(b' ');
    assert_eq!(
        request(exact).send().await.unwrap().status(),
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let valid = fixture("valid-activation.json");
    let mut duplicate = valid[..valid.len() - 1].to_vec();
    duplicate.extend_from_slice(br#", "activationFormatVersion":1}"#);
    let mut swapped: serde_json::Value = serde_json::from_slice(&valid).unwrap();
    swapped["manifestEnvelopeBase64"] = swapped["bundleEnvelopeBase64"].clone();
    let mut truncated: serde_json::Value = serde_json::from_slice(&valid).unwrap();
    truncated["manifestEnvelopeBase64"] = serde_json::json!(STANDARD.encode(b"{"));
    let mut oversized: serde_json::Value = serde_json::from_slice(&valid).unwrap();
    oversized["bundleEnvelopeBase64"] =
        serde_json::json!(STANDARD.encode(vec![b' '; 2 * 1024 * 1024 + 1]));
    let invalid = [
        "unsigned-bundle.json",
        "wrong-key.json",
        "duplicate-payload-field.json",
        "wrong-policy.json",
        "duplicate-envelope-field.json",
    ]
    .into_iter()
    .map(fixture)
    .chain([
        fixture("valid-bundle-envelope.json"),
        duplicate,
        serde_json::to_vec(&swapped).unwrap(),
        serde_json::to_vec(&truncated).unwrap(),
        serde_json::to_vec(&oversized).unwrap(),
    ]);
    for bytes in invalid {
        assert_eq!(
            request(bytes).send().await.unwrap().status(),
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(std::fs::read(&path).unwrap(), prior);
        assert_eq!(
            std::fs::read(runtime.root_path().join("evaluations.json")).unwrap(),
            ledger
        );
    }
    assert_eq!(
        client
            .post(&endpoint)
            .body(valid)
            .send()
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(std::fs::read(&path).unwrap(), prior);
    server.abort();
}

#[tokio::test]
async fn production_authenticated_route_cannot_select_a_fixture_key() {
    let directory = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(directory.path());
    let root = runtime_for(&state).root_path().to_path_buf();
    let (url, server) = server(state, false).await;
    let response = reqwest::Client::new()
        .post(format!("{url}/settings/taskmaster/knowledge"))
        .header("x-orkworks-open-plan-token", "taskmaster-test-token")
        .body(fixture("valid-activation.json"))
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert!(!root.join("evaluations.json").exists());
    assert!(!root.join("knowledge").exists());
    server.abort();
}
