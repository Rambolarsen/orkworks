use super::*;

#[cfg(unix)]
#[test]
fn legacy_branch_document_reads_reject_replacement_symlinks() {
    let (root, state, _store, _rx) = fixture();
    let reference = branch_documents::document(
        root.path(),
        "specs/current.md",
        crate::metadata::PlanSource::UserSelected,
        true,
    )
    .unwrap()
    .reference;
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .mutate_session(
            "runtime-review",
            crate::session_application::WriteScope::NONE,
            |meta| {
                meta.plan_path = Some(reference);
                Ok(())
            },
        )
        .unwrap();
    std::fs::rename(
        root.path().join("specs/current.md"),
        root.path().join("specs/target.md"),
    )
    .unwrap();
    std::os::unix::fs::symlink("target.md", root.path().join("specs/current.md")).unwrap();
    assert!(SessionApplication::new(state)
        .read_plan_content("runtime-review")
        .is_err());
}

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
async fn branch_document_pin_reset_retains_workspace_ownership_through_the_write() {
    let (root, state, store, _rx) = fixture();
    let identity = branch_documents::identity(root.path()).unwrap();
    store
        .report(
            &identity,
            branch_documents::document(
                root.path(),
                "specs/current.md",
                crate::metadata::PlanSource::UserSelected,
                true,
            )
            .unwrap(),
        )
        .unwrap();
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    *store.before_pin_reset.lock().unwrap() = Some(Box::new(move || {
        let _ = entered_tx.send(());
        release_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
    }));
    let reset_state = state.clone();
    let reset = std::thread::spawn(move || {
        SessionApplication::new(reset_state).reset_document_pin(
            "runtime-review",
            DocumentKind::Design,
            None,
        )
    });
    entered_rx.await.unwrap();
    let adoption_fenced = state.projection_lock.try_lock().is_err();
    let owner_retained = state.workspace.try_lock().is_err();
    release_tx.send(()).unwrap();
    reset.join().unwrap().unwrap();
    assert!(
        adoption_fenced && owner_retained,
        "pin mutation must retain adoption fence and workspace owner"
    );
}

#[test]
fn branch_document_content_rejects_a_slot_replaced_after_its_read() {
    let (root, state, store, _rx) = fixture();
    std::fs::write(root.path().join("specs/new.md"), "# New pin\n").unwrap();
    let identity = branch_documents::identity(root.path()).unwrap();
    let old = branch_documents::document(
        root.path(),
        "specs/current.md",
        crate::metadata::PlanSource::UserSelected,
        true,
    )
    .unwrap();
    let expected = old.selection_id(Some(&identity));
    store.report(&identity, old).unwrap();
    let newer = branch_documents::document(
        root.path(),
        "specs/new.md",
        crate::metadata::PlanSource::UserSelected,
        true,
    )
    .unwrap();
    let reporting = store.clone();
    *store.before_content_return.lock().unwrap() = Some(Box::new(move || {
        reporting.report(&identity, newer).unwrap()
    }));
    assert!(SessionApplication::new(state)
        .read_document_content(
            "runtime-review",
            Some(DocumentKind::Design),
            Some(&expected)
        )
        .is_err());
}

#[test]
fn branch_document_content_rejects_an_effective_worktree_switch_after_read() {
    let (root, state, store, _rx) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .unwrap();
    let siblings = tempfile::tempdir().unwrap();
    let other = siblings.path().join("other");
    repo.worktree("other", &other, None).unwrap();
    let moving = state.clone();
    *store.before_content_return.lock().unwrap() = Some(Box::new(move || {
        moving.peon.reported_cwd.write().unwrap().insert(
            "runtime-review".into(),
            other.to_string_lossy().into_owned(),
        );
    }));
    assert!(SessionApplication::new(state)
        .read_document_content("runtime-review", Some(DocumentKind::Design), None)
        .is_err());
}

#[tokio::test]
async fn branch_document_sibling_read_rejects_a_session_branch_switch_after_read() {
    let (root, state, store, _rx) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .unwrap();
    let siblings = tempfile::tempdir().unwrap();
    let other = siblings.path().join("other");
    repo.worktree("other", &other, None).unwrap();
    std::fs::create_dir(other.join("specs")).unwrap();
    let path = other.join("specs/current.md");
    std::fs::write(&path, "# Sibling\n").unwrap();
    let app = SessionApplication::new(state.clone());
    let selected = app
        .select_plan(
            "runtime-review",
            crate::session_application::PlanSelection {
                printed_path: path.to_string_lossy().into_owned(),
            },
        )
        .await
        .unwrap();
    let switching_root = root.path().to_owned();
    *store.before_content_return.lock().unwrap() = Some(Box::new(move || {
        git2::Repository::open(switching_root)
            .unwrap()
            .set_head("refs/heads/new-task")
            .unwrap();
    }));
    assert!(app
        .read_document_content(
            "runtime-review",
            selected.kind,
            selected.selection_id.as_deref()
        )
        .is_err());
}

#[tokio::test]
async fn branch_document_terminal_paths_follow_the_effective_sibling_worktree() {
    // Both checkouts contain the path: launch-first resolution picks the wrong one.
    for explicit in [false, true] {
        let (root, state, _store, _rx) = fixture();
        let repo = git2::Repository::open(root.path()).unwrap();
        let tree = repo
            .find_tree(repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
            .unwrap();
        let siblings = tempfile::tempdir().unwrap();
        let other = siblings.path().join("other");
        repo.worktree("other", &other, None).unwrap();
        std::fs::create_dir(other.join("specs")).unwrap();
        std::fs::write(other.join("specs/current.md"), "# Effective design\n").unwrap();
        state.peon.reported_cwd.write().unwrap().insert(
            "runtime-review".into(),
            other.to_string_lossy().into_owned(),
        );
        let app = SessionApplication::new(state.clone());
        if explicit {
            app.select_plan(
                "runtime-review",
                crate::session_application::PlanSelection {
                    printed_path: "specs/current.md".into(),
                },
            )
            .await
            .unwrap();
        } else {
            assert!(app.persist_printed_plan_fallback("runtime-review", "specs/current.md"));
        }
        let ws = state.workspace.lock().unwrap();
        let meta = ws
            .as_ref()
            .unwrap()
            .metadata
            .read_session("runtime-review")
            .unwrap();
        assert_eq!(
            PathBuf::from(meta.plan_path.unwrap().worktree_root.unwrap()),
            other.canonicalize().unwrap(),
            "explicit={explicit}"
        );
    }
}

#[tokio::test]
async fn legacy_branch_document_access_rejects_a_new_nested_repository() {
    let (root, state, _store, mut receiver) = fixture();
    let reference = branch_documents::document(
        root.path(),
        "specs/current.md",
        crate::metadata::PlanSource::UserSelected,
        true,
    )
    .unwrap()
    .reference;
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .mutate_session(
            "runtime-review",
            crate::session_application::WriteScope::NONE,
            |meta| {
                meta.plan_path = Some(reference);
                Ok(())
            },
        )
        .unwrap();
    git2::Repository::init(root.path().join("specs")).unwrap();
    let app = SessionApplication::new(state);
    assert!(app.read_plan_content("runtime-review").is_err());
    let request = app.request_plan_review("runtime-review");
    tokio::pin!(request);
    tokio::select! {
        result = &mut request => assert!(result.is_err()),
        _ = receiver.recv() => panic!("legacy review crossed repository boundary"),
    }
}

#[tokio::test]
async fn unbound_branch_document_selection_reads_and_reviews_the_sibling() {
    let (root, state, _store, mut receiver) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .unwrap();
    let siblings = tempfile::tempdir().unwrap();
    let other = siblings.path().join("other");
    repo.worktree("other", &other, None).unwrap();
    std::fs::create_dir(other.join("specs")).unwrap();
    let path = other.join("specs/other.md");
    std::fs::write(&path, "# Sibling design\n").unwrap();
    let app = SessionApplication::new(state.clone());
    let selection = app
        .select_plan(
            "runtime-review",
            crate::session_application::PlanSelection {
                printed_path: path.to_string_lossy().into_owned(),
            },
        )
        .await
        .unwrap();
    assert_eq!(selection.branch.as_deref(), Some("other"));
    assert_eq!(
        app.read_document_content(
            "runtime-review",
            selection.kind,
            selection.selection_id.as_deref()
        )
        .unwrap(),
        "# Sibling design\n"
    );
    let expected_id = selection.selection_id.clone();
    let mut request = tokio::spawn(async move {
        app.request_document_review(
            "runtime-review",
            selection.kind,
            selection.selection_id.as_deref(),
        )
        .await
    });
    let command = tokio::select! {
        command = receiver.recv() => command.unwrap(),
        result = &mut request => panic!("sibling review failed before enqueue: {:?}", result.unwrap()),
    };
    let crate::runtime::session_runtime::RuntimeCommand::Input { data, accepted, .. } = command
    else {
        panic!("expected review input");
    };
    assert!(data.contains(path.to_str().unwrap()));
    accepted.unwrap().send(Ok(())).unwrap();
    request.await.unwrap().unwrap();
    std::fs::write(path, "# Changed sibling\n").unwrap();
    assert!(SessionApplication::new(state)
        .request_document_review(
            "runtime-review",
            Some(DocumentKind::Design),
            expected_id.as_deref()
        )
        .await
        .is_err());
    assert!(receiver.try_recv().is_err());
}

#[test]
fn untyped_branch_document_read_skips_an_unavailable_pin_in_another_slot() {
    let (root, state, store, _rx) = fixture();
    let path = root.path().join("docs/superpowers/plans/missing.md");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "# Plan\n").unwrap();
    let identity = branch_documents::identity(root.path()).unwrap();
    store
        .report(
            &identity,
            branch_documents::document(
                root.path(),
                "docs/superpowers/plans/missing.md",
                crate::metadata::PlanSource::UserSelected,
                true,
            )
            .unwrap(),
        )
        .unwrap();
    std::fs::remove_file(path).unwrap();
    let app = SessionApplication::new(state);
    assert_eq!(
        app.read_document_content("runtime-review", None, None)
            .unwrap(),
        "# Design\n"
    );
    assert!(app
        .read_document_content("runtime-review", Some(DocumentKind::Implementation), None)
        .is_err());
}

#[tokio::test]
async fn branch_document_explicit_binding_rejects_terminal_paths_in_a_sibling() {
    let (root, state, _store, _rx) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .unwrap();
    let siblings = tempfile::tempdir().unwrap();
    let other = siblings.path().join("other");
    repo.worktree("other", &other, None).unwrap();
    std::fs::create_dir(other.join("specs")).unwrap();
    let path = other.join("specs/other.md");
    std::fs::write(&path, "# Other\n").unwrap();
    let binding = crate::session_location::WorktreeBinding::resolve(
        root.path().to_str().unwrap(),
        root.path().to_str().unwrap(),
    )
    .unwrap();
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .mutate_session(
            "runtime-review",
            crate::session_application::WriteScope::NONE,
            |meta| {
                meta.worktree_binding = Some(binding);
                Ok(())
            },
        )
        .unwrap();
    let app = SessionApplication::new(state.clone());
    assert!(app
        .select_plan(
            "runtime-review",
            crate::session_application::PlanSelection {
                printed_path: path.to_string_lossy().into_owned()
            }
        )
        .await
        .is_err());
    assert!(!app.persist_printed_plan_fallback("runtime-review", path.to_str().unwrap()));
    let reference = branch_documents::document(
        &other,
        "specs/other.md",
        crate::metadata::PlanSource::UserSelected,
        true,
    )
    .unwrap()
    .reference;
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .mutate_session(
            "runtime-review",
            crate::session_application::WriteScope::NONE,
            |meta| {
                meta.plan_path = Some(reference);
                Ok(())
            },
        )
        .unwrap();
    assert!(
        app.read_plan_content("runtime-review").is_err(),
        "legacy reads must also respect the explicit binding"
    );
    assert_ne!(
        app.read_document_content("runtime-review", None, None)
            .ok()
            .as_deref(),
        Some("# Other\n")
    );
}

#[test]
fn branch_document_hook_reports_follow_the_effective_sibling_worktree() {
    let (root, state, _store, _rx) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .unwrap();
    let sibling = tempfile::tempdir().unwrap();
    let task = sibling.path().join("task");
    repo.worktree("other-task", &task, None).unwrap();
    std::fs::create_dir(task.join("specs")).unwrap();
    std::fs::write(task.join("specs/current.md"), "# Task design\n").unwrap();
    state
        .peon
        .reported_cwd
        .write()
        .unwrap()
        .insert("runtime-review".into(), task.to_string_lossy().into_owned());
    let app = SessionApplication::new(state.clone());
    app.report_plan_path("runtime-review", "specs/current.md")
        .unwrap();
    let ws = state.workspace.lock().unwrap();
    let meta = ws
        .as_ref()
        .unwrap()
        .metadata
        .read_session("runtime-review")
        .unwrap();
    assert_eq!(
        PathBuf::from(meta.plan_path.unwrap().worktree_root.unwrap()),
        task.canonicalize().unwrap()
    );
}

#[test]
fn branch_document_binding_survives_removal_of_the_original_launch_worktree() {
    let (root, state, _store, _rx) = fixture();
    let repo = git2::Repository::open(root.path()).unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .unwrap();
    let siblings = tempfile::tempdir().unwrap();
    let launch = siblings.path().join("launch");
    let task = siblings.path().join("task");
    repo.worktree("launch", &launch, None).unwrap();
    repo.worktree("task-docs", &task, None).unwrap();
    std::fs::create_dir(task.join("specs")).unwrap();
    std::fs::write(task.join("specs/current.md"), "# Bound design\n").unwrap();
    let binding = crate::session_location::WorktreeBinding::resolve(
        launch.to_str().unwrap(),
        task.to_str().unwrap(),
    )
    .unwrap();
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .mutate_session(
            "runtime-review",
            crate::session_application::WriteScope::NONE,
            |meta| {
                meta.cwd = launch.to_string_lossy().into_owned();
                meta.worktree_binding = Some(binding.clone());
                Ok(())
            },
        )
        .unwrap();
    {
        let mut sessions = state.sessions.lock().unwrap();
        let info = &mut sessions.get_mut("runtime-review").unwrap().info;
        info.cwd = launch.to_string_lossy().into_owned();
        info.worktree_binding = Some(binding);
    }
    std::fs::remove_dir_all(launch).unwrap();
    assert_eq!(
        SessionApplication::new(state)
            .read_document_content("runtime-review", Some(DocumentKind::Design), None)
            .unwrap(),
        "# Bound design\n"
    );
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
    let selection_id = doc.selection_id(None);
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
