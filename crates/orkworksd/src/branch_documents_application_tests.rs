use super::*;

fn fixture() -> (
    tempfile::TempDir,
    Arc<AppState>,
    Arc<crate::branch_documents_store::Store>,
    tokio::sync::mpsc::Receiver<crate::runtime::session_runtime::RuntimeCommand>,
) {
    let root = tempfile::tempdir().unwrap();
    let repo = git2::Repository::init(root.path()).unwrap();
    repo.set_head("refs/heads/task").unwrap();
    std::fs::create_dir(root.path().join("specs")).unwrap();
    std::fs::write(root.path().join("specs/current.md"), "# Design\n").unwrap();
    let state = crate::test_support::test_app_state_with_workspace(root.path());
    let id = "runtime-review";
    let mut meta = crate::test_support::test_session_metadata(
        id,
        "Review",
        root.path().to_str().unwrap(),
        "running",
        "now",
        "now",
    );
    meta.cwd = root.path().to_string_lossy().into_owned();
    meta.lifecycle = "alive".into();
    meta.lifecycle_phase = "active".into();
    let store = {
        let ws = state.workspace.lock().unwrap();
        let ws = ws.as_ref().unwrap();
        ws.metadata.write_session(&meta);
        ws.metadata.branch_documents.clone()
    };
    let (runtime, old_rx) = crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    let (kill_tx, _) = tokio::sync::watch::channel(false);
    state.sessions.lock().unwrap().insert(
        id.into(),
        crate::SessionHandle {
            info: crate::session_types::SessionInfo::baseline_from_metadata(&meta),
            active_work_hook: false,
            kill_tx,
            output_buffer: crate::peon::RingBuffer::new(200),
            scan_buf: String::new(),
            pending_work_signal: None,
            runtime,
            terminal_attached: false,
            resume_in_progress: false,
            capacity: Default::default(),
        },
    );
    (root, state, store, old_rx)
}

#[tokio::test]
async fn typed_review_delivers_the_displayed_document_and_audits_after_acknowledgement() {
    let (_root, state, _store, mut control_rx) = fixture();
    let app = SessionApplication::new(state.clone());
    let mut request = tokio::spawn(async move {
        app.request_document_review("runtime-review", Some(DocumentKind::Design), None)
            .await
    });
    let command = tokio::select! {
        command = control_rx.recv() => command.unwrap(),
        result = &mut request => panic!("review failed before enqueue: {:?}", result.unwrap()),
    };
    let crate::runtime::session_runtime::RuntimeCommand::Input { data, accepted, .. } = command
    else {
        panic!("expected review input");
    };
    assert!(data.contains("specs/current.md"));
    accepted.unwrap().send(Ok(())).unwrap();
    request.await.unwrap().unwrap();
    assert!(state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_events("runtime-review")
        .iter()
        .any(|event| event.event_type == "plan_review_requested"));
}

#[tokio::test]
async fn typed_detached_review_preserves_the_session_local_legacy_selection() {
    let (root, state, _store, mut control_rx) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let signature = git2::Signature::now("Test", "test@example.com").unwrap();
    let commit = repo
        .commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
        .unwrap();
    repo.set_head_detached(commit).unwrap();
    let doc = branch_documents::document(
        root.path(),
        "specs/current.md",
        crate::metadata::PlanSource::UserSelected,
        true,
    )
    .unwrap();
    {
        let ws = state.workspace.lock().unwrap();
        ws.as_ref()
            .unwrap()
            .metadata
            .mutate_session(
                "runtime-review",
                crate::session_application::WriteScope::NONE,
                |meta| {
                    meta.plan_path = Some(doc.reference.clone());
                    Ok(())
                },
            )
            .unwrap();
    }
    let app = SessionApplication::new(state);
    let selection_id = doc.selection_id();
    assert_eq!(
        app.read_document_content(
            "runtime-review",
            Some(DocumentKind::Design),
            Some(&selection_id)
        )
        .unwrap(),
        "# Design\n"
    );
    let mut request = tokio::spawn(async move {
        app.request_document_review(
            "runtime-review",
            Some(DocumentKind::Design),
            Some(&selection_id),
        )
        .await
    });
    let command = tokio::select! {
        command = control_rx.recv() => command.unwrap(),
        result = &mut request => panic!("detached review failed before enqueue: {:?}", result.unwrap()),
    };
    let crate::runtime::session_runtime::RuntimeCommand::Input { data, accepted, .. } = command
    else {
        panic!("expected review input");
    };
    assert!(data.contains("specs/current.md"));
    accepted.unwrap().send(Ok(())).unwrap();
    request.await.unwrap().unwrap();
}

#[tokio::test]
async fn review_never_retargets_a_resumed_runtime_after_slow_document_inspection() {
    let (_root, state, store, mut old_rx) = fixture();
    let id = "runtime-review";
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    *store.before_scan.lock().unwrap() = Some(Box::new(move || {
        let _ = entered_tx.send(());
        release_rx.recv().unwrap();
    }));
    let app = SessionApplication::new(state.clone());
    let request = tokio::spawn(async move {
        app.request_document_review(id, Some(DocumentKind::Design), None)
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(5), entered_rx)
        .await
        .unwrap()
        .unwrap();
    let (replacement, mut replacement_rx) =
        crate::runtime::session_runtime::SessionRuntime::live(24, 80);
    state.sessions.lock().unwrap().get_mut(id).unwrap().runtime = replacement;
    release_tx.send(()).unwrap();
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(5), request)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    assert!(old_rx.try_recv().is_err());
    assert!(replacement_rx.try_recv().is_err());
    assert!(state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_events(id)
        .iter()
        .all(|e| e.event_type != "plan_review_requested"));
}
