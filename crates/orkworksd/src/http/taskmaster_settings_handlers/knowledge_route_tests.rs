//! Real authenticated build_router and Electron-client boundary; Node is required.
use super::*;
use crate::taskmaster::knowledge::test_support::{fixture, TEST_PUBLIC_KEY};
use base64::{engine::general_purpose::STANDARD, Engine as _};

struct TestServer(Option<tokio::task::JoinHandle<()>>);
impl TestServer {
    async fn shutdown(mut self) {
        let task = self.0.take().unwrap();
        task.abort();
        let _ = task.await;
    }
}
impl Drop for TestServer {
    fn drop(&mut self) {
        if let Some(task) = &self.0 {
            task.abort();
        }
    }
}

async fn server(state: Arc<AppState>, fixture_key: bool) -> (String, TestServer) {
    let token = crate::test_support::open_plan_token::set("taskmaster-test-token").await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let mut router = crate::build_router(state);
    if fixture_key {
        router = router.layer(axum::Extension(KnowledgeTestKey(TEST_PUBLIC_KEY)));
    }
    let task = tokio::spawn(async move {
        // Keep environment ownership until the serving future has actually stopped.
        let _token = token;
        axum::serve(listener, router).await.unwrap();
    });
    (url, TestServer(Some(task)))
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
            .current_dir(env!("CARGO_MANIFEST_DIR"))
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
    server.shutdown().await;
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
    server.shutdown().await;
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
    server.shutdown().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn real_electron_synchronization_recovers_from_older_cache_but_refuses_equivocation_and_other_errors(
) {
    use crate::taskmaster::knowledge::test_support::{activation, bundle_value};
    for mode in ["recover", "equivocation", "unauthorized"] {
        let directory = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(directory.path());
        let root = runtime_for(&state).root_path().to_path_buf();
        let runtime = TaskmasterRuntime::open_with_test_key(root.clone(), TEST_PUBLIC_KEY);
        let mut current = bundle_value("valid-activation.json");
        current["sequence"] = serde_json::json!(2);
        current["version"] = serde_json::json!("sidecar-2");
        runtime
            .activate_knowledge(&activation(current.clone(), false))
            .unwrap();
        let cached = if mode == "equivocation" {
            current["pages"][0]["title"] = serde_json::json!("Conflicting same-sequence guidance");
            activation(current, false)
        } else {
            fixture("valid-activation.json")
        };
        let mut feed = bundle_value("valid-activation.json");
        feed["sequence"] = serde_json::json!(3);
        feed["version"] = serde_json::json!("feed-3");
        let starter_path = directory.path().join("starter.json");
        let feed_path = directory.path().join("feed.json");
        std::fs::write(&starter_path, cached).unwrap();
        std::fs::write(&feed_path, activation(feed, false)).unwrap();
        let cache_path = directory.path().join("electron-cache");
        let (url, server) = server(state, true).await;
        let port = url.rsplit(':').next().unwrap().to_owned();
        let modules =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../apps/desktop/electron");
        let output = tokio::task::spawn_blocking(move || {
            std::process::Command::new("node")
                .current_dir(env!("CARGO_MANIFEST_DIR"))
                .args(["--experimental-strip-types", "--input-type=module", "-e", r#"
                    import assert from 'node:assert/strict';
                    import { readFileSync } from 'node:fs';
                    import { join } from 'node:path';
                    import { pathToFileURL } from 'node:url';
                    const [modules, starterPath, feedPath, directory, port, mode, publicKey] = process.argv.slice(1);
                    const { taskmasterRequest } = await import(pathToFileURL(join(modules, 'taskmasterSettings.ts')).href);
                    const { KnowledgeUpdates, synchronizeKnowledge } = await import(pathToFileURL(join(modules, 'knowledgeUpdates.ts')).href);
                    const feed = JSON.parse(readFileSync(feedPath, 'utf8'));
                    let feedChecks = 0;
                    const updates = new KnowledgeUpdates({ directory, starterPath, publicKey,
                        feedUrl: 'https://example.test/knowledge/manifest.json',
                        fetcher: async url => {
                            feedChecks++;
                            const field = url.endsWith('manifest.json') ? 'manifestEnvelopeBase64' : 'bundleEnvelopeBase64';
                            return new Response(Buffer.from(feed[field], 'base64'));
                        },
                    });
                    const attempted = [];
                    let error;
                    try {
                        await synchronizeKnowledge(updates, async activation => {
                            const envelope = JSON.parse(Buffer.from(activation.bundleEnvelopeBase64, 'base64'));
                            attempted.push(JSON.parse(envelope.payload).sequence);
                            return taskmasterRequest(Number(port), mode === 'unauthorized' ? 'wrong-token' : 'taskmaster-test-token', 'knowledge', activation);
                        }, () => true);
                    } catch (failure) { error = failure.message; }
                    if (mode === 'recover') {
                        assert.equal(error, undefined, `older cache must reach feed recovery: ${error}; checks=${feedChecks}`);
                        assert.equal(feedChecks, 2);
                        assert.deepEqual(attempted, [1, 3]);
                        assert.equal(updates.status().version, 'feed-3');
                    } else {
                        assert.equal(feedChecks, 0, 'non-downgrade refusal must stop before checking the feed');
                        assert.deepEqual(attempted, [mode === 'equivocation' ? 2 : 1]);
                        assert.match(error, mode === 'equivocation' ? /equivocation/ : /Invalid Taskmaster response/);
                    }
                    process.stdout.write('verified');
                "#])
                .arg(modules).arg(starter_path).arg(feed_path).arg(cache_path).arg(port).arg(mode).arg(TEST_PUBLIC_KEY)
                .output().expect("Node and host fs-ext are required for real synchronization coverage")
        }).await.unwrap();
        assert!(
            output.status.success(),
            "{mode} synchronization failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert_eq!(output.stdout, b"verified");
        let restarted = TaskmasterRuntime::open_with_test_key(root, TEST_PUBLIC_KEY);
        assert_eq!(
            restarted.status(None).knowledge_version.as_deref(),
            Some(if mode == "recover" {
                "feed-3"
            } else {
                "sidecar-2"
            })
        );
        server.shutdown().await;
    }
}

#[tokio::test]
async fn authenticated_server_keeps_its_token_until_shutdown_while_another_test_waits() {
    let directory = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(directory.path());
    let (url, server) = server(state, true).await;
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let contender = tokio::spawn(async move {
        started_tx.send(()).unwrap();
        let _token = crate::test_support::open_plan_token::set("test-token").await;
        let _ = release_rx.await;
    });
    // This single-thread runtime polls the contender through token acquisition before waking us.
    started_rx.await.unwrap();
    let response = reqwest::Client::new()
        .get(format!("{url}/taskmaster/run-status"))
        .header("x-orkworks-open-plan-token", "taskmaster-test-token")
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    server.shutdown().await;
    release_tx.send(()).unwrap();
    contender.await.unwrap();
}

#[tokio::test]
async fn production_starter_activates_through_authenticated_route_and_reports_ready() {
    let directory = tempfile::tempdir().unwrap();
    let state = crate::test_support::test_app_state_with_workspace(directory.path());
    let runtime = runtime_for(&state);
    let mut settings = TaskmasterSettings::default();
    settings.selection = Some(
        serde_json::from_value(serde_json::json!({
            "provider":"ollama", "model":"fixture-model"
        }))
        .unwrap(),
    );
    runtime.replace_settings(settings).unwrap();
    assert_eq!(
        status_for(&state, &runtime).analysis_status,
        "knowledge_unavailable"
    );
    let (url, server) = server(state.clone(), false).await;
    let response = reqwest::Client::new()
        .post(format!("{url}/settings/taskmaster/knowledge"))
        .header("x-orkworks-open-plan-token", "taskmaster-test-token")
        .header("content-type", "application/json")
        .body(
            include_bytes!("../../../../../apps/desktop/resources/knowledge/starter.json").to_vec(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let result: serde_json::Value = response.json().await.unwrap();
    assert_eq!(result["analysisStatus"], "ready");
    assert_eq!(
        runtime.status(Some(directory.path())).remaining_evaluations,
        8
    );
    assert!(runtime_for(&state).brain_knowledge_availability().is_ok());
    server.shutdown().await;
}
