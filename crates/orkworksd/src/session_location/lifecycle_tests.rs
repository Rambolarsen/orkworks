//! Reopen persisted metadata and use the real resume workflow without a coding tool.
use super::*;
use crate::harness::{ResumeMemory, ResumeState, ResumeStrategy};
use crate::test_support::{test_app_state_with_workspace, test_session_metadata};

const SESSION: &str = "worktree-lifecycle";

fn persisted_fixture() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let launch = dir.path().join("launch");
    let task = dir.path().join("task");
    let repo = git2::Repository::init(&launch).unwrap();
    repo.set_head("refs/heads/main").unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let sig = git2::Signature::now("test", "test@example.invalid").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
        .unwrap();
    repo.worktree("task", &task, None).unwrap();
    let task = task.canonicalize().unwrap();
    let state = test_app_state_with_workspace(&launch);
    let mut meta = test_session_metadata(
        SESSION,
        "Worktree lifecycle",
        launch.display().to_string(),
        "ended",
        "before",
        "before",
    );
    meta.cwd = launch.display().to_string();
    meta.harness = "opencode".into();
    meta.lifecycle = "dead".into();
    meta.lifecycle_phase = "ended".into();
    meta.worktree_binding =
        Some(WorktreeBinding::resolve(launch.to_str().unwrap(), task.to_str().unwrap()).unwrap());
    meta.resume = Some(ResumeMemory {
        state: ResumeState::Available,
        preferred_strategy: ResumeStrategy::LatestCwd,
        harness_session_id: None,
        latest_fallback: true,
        last_seen_at: Some("before".into()),
    });
    state
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .write_session(&meta);
    (dir, task)
}

#[test]
fn task_binding_survives_workspace_reload_and_missing_root() {
    let (dir, task) = persisted_fixture();
    let launch = dir.path().join("launch");
    let state = test_app_state_with_workspace(&launch);
    assert!(state.sessions.lock().unwrap().is_empty());
    assert!(state.peon.reported_cwd.read().unwrap().is_empty());
    let (root, source) = subject(&state, SESSION).unwrap();
    assert_eq!(root, task.canonicalize().unwrap());
    assert_eq!(source, GitLocationSource::Agent);
    let infos = crate::session_projection::SessionProjection::new(state.clone())
        .list()
        .unwrap();
    let info = infos.iter().find(|info| info.id == SESSION).unwrap();
    assert_eq!(info.branch.as_deref(), Some("task"));
    assert_eq!(Path::new(&info.cwd), launch);
    drop(state);

    std::fs::remove_dir_all(&task).unwrap();
    let restarted = test_app_state_with_workspace(&launch);
    assert!(
        subject(&restarted, SESSION).is_err(),
        "missing task root must not fall back to launch/main"
    );
    let stored = restarted
        .workspace
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .metadata
        .read_session(SESSION)
        .unwrap();
    assert_eq!(Path::new(&stored.worktree_binding.unwrap().root), task);
    assert_eq!(Path::new(&stored.cwd), launch);
}

#[tokio::test]
async fn resumed_runtime_keeps_task_binding_and_launch_directory() {
    let (dir, task) = persisted_fixture();
    let launch = dir.path().join("launch");
    let executable = dir.path().join("fake-resume");
    std::fs::write(
        &executable,
        "#!/bin/sh\npwd > resume-cwd.txt\nexec sleep 30\n",
    )
    .unwrap();
    crate::test_support::make_test_executable(&executable);
    let state = test_app_state_with_workspace(&launch);
    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            let patch = document.overrides.entry("opencode".into()).or_default();
            patch.resume = Some(Some(crate::harness::definition::ResumePatch {
                latest_cwd: Some(Some(crate::harness::CommandTemplate {
                    command: executable.display().to_string(),
                    args: vec![],
                })),
                ..Default::default()
            }));
            Ok(())
        })
        .unwrap();
    let info = crate::session_application::SessionApplication::new(state.clone())
        .resume_session(SESSION, Some(ResumeStrategy::LatestCwd))
        .await
        .unwrap();
    let live_location = subject(&state, SESSION);
    // Always terminate the disposable process before checking assertions.
    let cwd_file = launch.join("resume-cwd.txt");
    let observed_cwd = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if let Ok(cwd) = std::fs::read_to_string(&cwd_file) {
                if !cwd.trim().is_empty() {
                    break cwd;
                }
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    crate::runtime::session_runtime::send_runtime_command(
        &state,
        SESSION,
        crate::runtime::session_runtime::RuntimeCommand::Kill,
    )
    .await
    .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if state
                .sessions
                .lock()
                .unwrap()
                .get(SESSION)
                .is_some_and(|h| !h.resume_in_progress && h.info.lifecycle_phase == "ended")
            {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("disposable runtime must finish");

    assert_eq!(
        Path::new(
            observed_cwd
                .expect("fake resume must record its cwd")
                .trim()
        )
        .canonicalize()
        .unwrap(),
        launch.canonicalize().unwrap()
    );
    assert_eq!(Path::new(&info.cwd), launch);
    assert_eq!(Path::new(&info.worktree_binding.unwrap().root), task);
    let (live_root, live_source) = live_location.unwrap();
    assert_eq!(live_root, task);
    assert_eq!(live_source, GitLocationSource::Agent);
    let (root, source) = subject(&state, SESSION).unwrap();
    assert_eq!(root, task);
    assert_eq!(source, GitLocationSource::Agent);
    drop(state);
    let reopened = test_app_state_with_workspace(&launch);
    assert_eq!(
        subject(&reopened, SESSION).unwrap().0,
        task.canonicalize().unwrap()
    );
}
