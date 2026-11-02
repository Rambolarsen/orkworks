use super::*;
use crate::metadata::hook_observation::Admission;
use std::time::Duration;

struct Fixture {
    _path: FakePath,
    _home: FakeHome,
    _bin: tempfile::TempDir,
    _home_dir: tempfile::TempDir,
    dir: tempfile::TempDir,
    state: Arc<crate::AppState>,
    owner: Arc<crate::metadata::hook_observation::Store>,
}
fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    git2::Repository::init(dir.path()).unwrap();
    std::fs::write(dir.path().join(".gitignore"), ".codex/hooks.json\n").unwrap();
    let home = tempfile::tempdir().unwrap();
    let _home = FakeHome::set(home.path());
    let bin = tempfile::tempdir().unwrap();
    install_fake_command(
        bin.path(),
        "codex",
        "#!/bin/sh\nprintf 'codex-cli 0.150.1\\n'\n",
    );
    let _path = FakePath::prepend(bin.path());
    let state = test_app_state_with_workspace(dir.path());
    SessionApplication::new(state.clone())
        .set_active_harnesses(vec!["codex".into()])
        .unwrap();
    let owner = {
        let mut ws = state.workspace.lock().unwrap();
        let ws = ws.as_mut().unwrap();
        ws.lease = Some(Arc::new(
            crate::workspace_runtime::WorkspaceLease::acquire(&ws.metadata.root_path()).unwrap(),
        ));
        ws.metadata.hook_observation_owner()
    };
    Fixture {
        _path,
        _home,
        _bin: bin,
        _home_dir: home,
        dir,
        state,
        owner,
    }
}
fn request() -> IntegrationMutationRequest {
    IntegrationMutationRequest::Harness {
        harness_id: "codex".into(),
        mutation: IntegrationMutation::Install,
    }
}
// An independent OS watchdog must release the file owner even if the executor
// itself is incorrectly blocked by the adapter.
async fn stalled_clear(route: &str) {
    let fixture = fixture();
    let state = fixture.state.clone();
    let owner = fixture.owner.clone();
    if route == "cleanup" {
        HarnessIntegrationApplication::new(state.clone())
            .mutate(IntegrationMutationRequest::Harness {
                harness_id: "codex".into(),
                mutation: IntegrationMutation::Install,
            })
            .await
            .unwrap();
        SessionApplication::new(state.clone())
            .set_active_harnesses(vec![])
            .unwrap();
    }
    owner
        .write(&crate::metadata::CodexHookObservation {
            fingerprint: "a".repeat(64),
            observed_at: "2026-01-01T00:00:00Z".into(),
        })
        .unwrap();
    let old = owner.stamp();
    let Admission::Claimed(held) = owner.stamp().try_claim() else {
        panic!("file owner should be free");
    };
    let checking = state.clone();
    let watchdog = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let entered = loop {
            if matches!(old.clone().try_claim(), Admission::Obsolete) {
                break true;
            }
            if std::time::Instant::now() >= deadline {
                break false;
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        let projection_available = checking.projection_lock.try_lock().is_ok();
        let workspace_available = checking.workspace.try_lock().is_ok();
        drop(held);
        (entered, projection_available, workspace_available)
    });
    let app = HarnessIntegrationApplication::new(state.clone());
    let key = IntegrationKey {
        adapter_id: "codex".into(),
        target_id: "workspace".into(),
    };
    let result = match route {
        "install" | "uninstall" => app
            .mutate(IntegrationMutationRequest::Harness {
                harness_id: "codex".into(),
                mutation: if route == "install" {
                    IntegrationMutation::Install
                } else {
                    IntegrationMutation::Uninstall
                },
            })
            .await
            .unwrap(),
        "repair" | "group-uninstall" => app
            .mutate(grouped_request(
                &state,
                key.clone(),
                if route == "repair" {
                    IntegrationMutation::Repair
                } else {
                    IntegrationMutation::Uninstall
                },
            ))
            .await
            .unwrap(),
        "cleanup" => {
            let mut response = app
                .reconcile_unreferenced(BTreeSet::from([key]), None)
                .await;
            assert_eq!(response.status, "complete");
            IntegrationInspection::Group(response.outcomes.pop().unwrap())
        }
        _ => unreachable!(),
    };
    let (entered, projection_available, workspace_available) = watchdog.join().unwrap();
    assert!(
        entered,
        "the real install did not reach observation clearing"
    );
    assert!(
        projection_available,
        "install retained the projection guard while waiting for the observation file"
    );
    assert!(
        workspace_available,
        "install retained the workspace guard while waiting for the observation file"
    );
    let status = match result {
        IntegrationInspection::Harness(status) => status,
        IntegrationInspection::Group(group) => group.status,
    };
    let registration = if matches!(route, "install" | "repair") {
        IntegrationRegistration::Installed
    } else {
        IntegrationRegistration::Absent
    };
    assert_eq!(status.registration, registration);
    if registration == IntegrationRegistration::Installed {
        assert_eq!(status.activation, IntegrationActivation::NeedsTrust);
    }
    assert!(
        owner.read().is_none(),
        "the clear must remove the original observation"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_install_waits_for_file_owner_without_application_guards() {
    stalled_clear("install").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_absent_uninstall_waits_without_application_guards() {
    stalled_clear("uninstall").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_group_repair_waits_without_application_guards() {
    stalled_clear("repair").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_group_uninstall_waits_without_application_guards() {
    stalled_clear("group-uninstall").await;
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_cleanup_waits_without_application_guards() {
    stalled_clear("cleanup").await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_rejects_same_path_owner_replacement_during_probe() {
    let fixture = fixture();
    let changing = fixture.state.clone();
    let mut app = HarnessIntegrationApplication::new(fixture.state.clone());
    app.probe_revalidation_hook = Some(Arc::new(move |_| {
        let mut ws = changing.workspace.lock().unwrap();
        let ws = ws.as_mut().unwrap();
        ws.metadata = crate::metadata::MetadataStore::new(&ws.metadata.root_path());
    }));
    let result = app.mutate(request()).await;
    assert!(
        matches!(result, Err(IntegrationApplicationError::WorkspaceChanged)),
        "same-path replacement was admitted: {result:?}"
    );
    assert!(!fixture.dir.path().join(".codex/hooks.json").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_rejects_same_path_owner_replacement_in_grouped_probe() {
    let fixture = fixture();
    let changing = fixture.state.clone();
    let mut app = HarnessIntegrationApplication::new(fixture.state.clone());
    let key = IntegrationKey {
        adapter_id: "codex".into(),
        target_id: "workspace".into(),
    };
    let request = grouped_request(&fixture.state, key, IntegrationMutation::Install);
    app.probe_revalidation_hook = Some(Arc::new(move |_| {
        let mut ws = changing.workspace.lock().unwrap();
        let ws = ws.as_mut().unwrap();
        ws.metadata = crate::metadata::MetadataStore::new(&ws.metadata.root_path());
    }));
    let result = app.mutate(request).await;
    assert!(
        matches!(result, Err(IntegrationApplicationError::WorkspaceChanged)),
        "same-path replacement was admitted: {result:?}"
    );
    assert!(!fixture.dir.path().join(".codex/hooks.json").exists());
}

fn hold_clear(
    owner: &Arc<crate::metadata::hook_observation::Store>,
) -> (
    tokio::sync::oneshot::Receiver<bool>,
    std::sync::mpsc::Sender<()>,
    std::thread::JoinHandle<()>,
) {
    let old = owner.stamp();
    let Admission::Claimed(held) = owner.stamp().try_claim() else {
        panic!("file owner should be free");
    };
    let (sent, entered) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::channel();
    let thread = std::thread::spawn(move || {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        let clearing = loop {
            if matches!(old.clone().try_claim(), Admission::Obsolete) {
                break true;
            }
            if std::time::Instant::now() >= deadline {
                break false;
            }
            std::thread::sleep(Duration::from_millis(1));
        };
        let _ = sent.send(clearing);
        let _ = released.recv_timeout(Duration::from_secs(10));
        drop(held);
    });
    (entered, release, thread)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_overlap_is_bounded_and_retryable_after_completion() {
    let fixture = fixture();
    let (entered, release, thread) = hold_clear(&fixture.owner);
    let state = fixture.state.clone();
    let first = tokio::spawn(async move {
        HarnessIntegrationApplication::new(state)
            .mutate(request())
            .await
    });
    assert!(entered.await.unwrap());
    let second = tokio::time::timeout(
        Duration::from_millis(250),
        HarnessIntegrationApplication::new(fixture.state.clone()).mutate(
            IntegrationMutationRequest::Harness {
                harness_id: "codex".into(),
                mutation: IntegrationMutation::Uninstall,
            },
        ),
    )
    .await;
    let _ = release.send(());
    thread.join().unwrap();
    first.await.unwrap().unwrap();
    assert!(
        matches!(
            second,
            Ok(Err(IntegrationApplicationError::Configuration(
                IntegrationError::RevisionChanged
            )))
        ),
        "overlapping action queued or executed: {second:?}"
    );
    let result = HarnessIntegrationApplication::new(fixture.state.clone())
        .mutate(IntegrationMutationRequest::Harness {
            harness_id: "codex".into(),
            mutation: IntegrationMutation::Uninstall,
        })
        .await
        .unwrap();
    let IntegrationInspection::Harness(status) = result else {
        panic!("expected harness status");
    };
    assert_eq!(status.registration, IntegrationRegistration::Absent);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_cancelled_claim_keeps_lease_and_definitive_clear_on_old_workspace() {
    let fixture = fixture();
    fixture
        .owner
        .write(&crate::metadata::CodexHookObservation {
            fingerprint: "a".repeat(64),
            observed_at: "before".into(),
        })
        .unwrap();
    let lease = Arc::downgrade(
        fixture
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .lease
            .as_ref()
            .unwrap(),
    );
    let (entered, release, thread) = hold_clear(&fixture.owner);
    let state = fixture.state.clone();
    let first = tokio::spawn(async move {
        HarnessIntegrationApplication::new(state)
            .mutate(request())
            .await
    });
    assert!(entered.await.unwrap());
    first.abort();
    assert!(first.await.unwrap_err().is_cancelled());
    let replacement = tempfile::tempdir().unwrap();
    swap_workspace(&fixture.state, replacement.path());
    assert!(
        lease.upgrade().is_some(),
        "claimed old-workspace action lost its lease"
    );
    let _ = release.send(());
    thread.join().unwrap();
    tokio::time::timeout(Duration::from_secs(5), async {
        while lease.upgrade().is_some() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert!(fixture.owner.read().is_none());
    assert!(fixture.dir.path().join(".codex/hooks.json").exists());
    assert!(!replacement.path().join(".codex/hooks.json").exists());
    assert!(
        fixture.owner.try_integration().is_some(),
        "completed action retained its admission slot"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_clear_failure_is_typed_after_config_commit_and_releases_admission() {
    let fixture = fixture();
    let path = fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .codex_hook_observation_path();
    std::fs::create_dir(&path).unwrap();
    let result = HarnessIntegrationApplication::new(fixture.state.clone())
        .mutate(request())
        .await;
    assert!(
        matches!(
            result,
            Err(IntegrationApplicationError::Configuration(
                IntegrationError::Io(_)
            ))
        ),
        "clear failure was acknowledged as success: {result:?}"
    );
    assert!(
        fixture.dir.path().join(".codex/hooks.json").exists(),
        "a clear error must not imply config rollback"
    );
    assert!(path.is_dir());
    assert!(fixture.owner.try_integration().is_some());
    std::fs::remove_dir(path).unwrap();
    let result = HarnessIntegrationApplication::new(fixture.state.clone())
        .mutate(IntegrationMutationRequest::Harness {
            harness_id: "codex".into(),
            mutation: IntegrationMutation::Uninstall,
        })
        .await
        .unwrap();
    let IntegrationInspection::Harness(status) = result else {
        panic!("expected harness status");
    };
    assert_eq!(status.registration, IntegrationRegistration::Absent);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_unchanged_install_preserves_observed_trust() {
    let fixture = fixture();
    let app = HarnessIntegrationApplication::new(fixture.state.clone());
    app.mutate(request()).await.unwrap();
    let reporter = fixture
        ._home_dir
        .path()
        .join(".orkworks/hook-scripts/report-harness-event.sh");
    let observation = crate::metadata::CodexHookObservation {
        fingerprint: crate::harness::integrations::current_codex_hook_fingerprint(&reporter)
            .unwrap(),
        observed_at: "before-repeat".into(),
    };
    fixture.owner.write(&observation).unwrap();
    let result = app.mutate(request()).await.unwrap();
    let IntegrationInspection::Harness(status) = result else {
        panic!("expected harness status");
    };
    assert_eq!(status.activation, IntegrationActivation::Active);
    assert_eq!(fixture.owner.read(), Some(observation));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_rejects_a_revoked_lease_after_probe() {
    let fixture = fixture();
    let changing = fixture.state.clone();
    let mut app = HarnessIntegrationApplication::new(fixture.state.clone());
    app.probe_revalidation_hook = Some(Arc::new(move |_| {
        changing.workspace.lock().unwrap().as_mut().unwrap().lease = None;
    }));
    assert!(matches!(
        app.mutate(request()).await,
        Err(IntegrationApplicationError::WorkspaceChanged)
    ));
    assert!(!fixture.dir.path().join(".codex/hooks.json").exists());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_real_pty_input_continues_while_clear_waits() {
    use crate::runtime::session_runtime::{RuntimeCommand, SessionRuntime};
    use std::io::{Read, Write};
    let fixture = fixture();
    let id = "settings-input";
    let created = "2026-01-01T00:00:00Z";
    let (runtime, mut commands) = SessionRuntime::live(24, 80);
    let mut info = crate::test_support::test_session_info(
        id,
        "Input",
        fixture.dir.path().display().to_string(),
        "running",
        created,
    );
    info.lifecycle = "alive".into();
    info.lifecycle_phase = "active".into();
    let mut metadata = crate::test_support::test_session_metadata(
        id,
        "Input",
        fixture.dir.path().display().to_string(),
        "running",
        created,
        created,
    );
    metadata.lifecycle = "alive".into();
    metadata.lifecycle_phase = "active".into();
    fixture
        .state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .try_write_session(&metadata)
        .unwrap();
    fixture.state.sessions.lock().unwrap().insert(
        id.into(),
        crate::SessionHandle {
            info,
            runtime,
            kill_tx: tokio::sync::watch::channel(false).0,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
            active_work_hook: false,
        },
    );
    let (entered, release, thread) = hold_clear(&fixture.owner);
    let state = fixture.state.clone();
    let install = tokio::spawn(async move {
        HarnessIntegrationApplication::new(state)
            .mutate(request())
            .await
    });
    assert!(entered.await.unwrap());
    let pair = portable_pty::native_pty_system()
        .openpty(portable_pty::PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .unwrap();
    let mut child = pair
        .slave
        .spawn_command(portable_pty::CommandBuilder::new("/bin/cat"))
        .unwrap();
    drop(pair.slave);
    let mut reader = pair.master.try_clone_reader().unwrap();
    let mut writer = pair.master.take_writer().unwrap();
    let (seen, observed) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut buf = [0; 4096];
        let mut bytes = Vec::new();
        while let Ok(count) = reader.read(&mut buf) {
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buf[..count]);
            if bytes.windows(14).any(|part| part == b"settings-input") || bytes.len() > 16384 {
                break;
            }
        }
        let _ = seen.send(bytes);
    });
    let delivery = tokio::task::spawn_blocking(move || match commands.blocking_recv().unwrap() {
        RuntimeCommand::Input {
            data,
            accepted: Some(accepted),
        } => {
            let result = writer
                .write_all(data.as_bytes())
                .and_then(|()| writer.flush())
                .map_err(|_| ());
            let _ = accepted.send(result);
        }
        _ => panic!("expected ordinary input"),
    });
    let delivered = tokio::time::timeout(
        Duration::from_secs(1),
        crate::runtime::terminal_runtime::submit_approved_input(
            &fixture.state,
            id,
            "settings-input\r".into(),
        ),
    )
    .await;
    let echoed = observed.recv_timeout(Duration::from_secs(1));
    let _ = child.kill();
    let _ = child.wait();
    drop(pair.master);
    reader.join().unwrap();
    delivery.await.unwrap();
    let _ = release.send(());
    thread.join().unwrap();
    install.await.unwrap().unwrap();
    assert_eq!(delivered.unwrap(), Ok(()));
    assert!(echoed
        .unwrap()
        .windows(14)
        .any(|part| part == b"settings-input"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn codex_settings_cancelled_probe_never_admits_config_work() {
    let fixture = fixture();
    let entered = fixture._bin.path().join("entered");
    let release = fixture._bin.path().join("release");
    install_fake_command(fixture._bin.path(), "codex", &format!(
        "#!/bin/sh\n: > '{}'\nwhile [ ! -f '{}' ]; do /bin/sleep 0.01; done\nprintf 'codex-cli 0.150.1\\n'\n",
        entered.display(), release.display()));
    let state = fixture.state.clone();
    let action = tokio::spawn(async move {
        HarnessIntegrationApplication::new(state)
            .mutate(request())
            .await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while !entered.exists() {
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    action.abort();
    assert!(action.await.unwrap_err().is_cancelled());
    std::fs::write(release, "").unwrap();
    // The real probe uses kill_on_drop; cancellation may terminate its child
    // before it writes anything else. Only application admission matters here.
    assert!(!fixture.dir.path().join(".codex/hooks.json").exists());
    assert!(fixture.owner.try_integration().is_some());
}
