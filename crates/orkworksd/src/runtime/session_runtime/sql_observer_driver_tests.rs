use super::*;
use crate::runtime::codex_approval_application::sql_tests::{fixture_with_root, ID};

#[cfg(unix)]
#[tokio::test]
async fn sql_observer_driver_uses_owned_rpc_and_sql_without_json_session_files() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let diagnostic = Arc::new(std::sync::Mutex::new(
        super::super::codex_approval_application::DiagnosticMetrics::default(),
    ));
    let generation = {
        let mut sessions = state.sessions.lock().unwrap();
        let h = sessions.get_mut(ID).unwrap();
        h.runtime.native_diagnostic = Some(diagnostic.clone());
        h.runtime.native_approval.as_mut().unwrap().diagnostic = Some(diagnostic.clone());
        h.runtime.run_generation()
    };
    let native_dir = tempfile::tempdir().unwrap();
    let (plan, environment) = super::super::codex_native::fixture_plan_and_env(
        native_dir.path(),
        &state.integration_probe_cache,
        false,
    );
    let native = plan.start(&environment).await.unwrap();
    let mut observer =
        NativeObserverControl::start_sql(state.clone(), ID.into(), generation, native, writes)
            .unwrap();
    let observed = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if diagnostic.lock().unwrap().observations > 0 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
    })
    .await;
    let failure = observer.shutdown().await;
    assert!(
        observed.is_ok(),
        "the observer did not finish an owned RPC through SQL"
    );
    assert_eq!(failure, None);
    let metrics = diagnostic.lock().unwrap();
    assert_eq!(
        metrics.last_status,
        Some(super::super::codex_approval::NativeStatus::Active)
    );
    assert!(metrics.singleton);
    drop(metrics);
    let row = store.get(ID.into()).await.unwrap().unwrap();
    assert_eq!(
        row["attention"], "needs_you",
        "production native-clear gate stays closed"
    );
    assert_eq!(row["metadataRevision"], "2");
    store.close().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn sql_observer_driver_storage_failure_does_not_signal_runtime_failure() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    store.close().await.unwrap();
    let native_dir = tempfile::tempdir().unwrap();
    let (plan, environment) = super::super::codex_native::fixture_plan_and_env(
        native_dir.path(),
        &state.integration_probe_cache,
        false,
    );
    let native = plan.start(&environment).await.unwrap();
    let mut observer =
        NativeObserverControl::start_sql(state, ID.into(), generation, native, writes).unwrap();
    let unavailable = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            if observer.storage_error.borrow().is_some() {
                break;
            }
            observer.storage_error.changed().await.unwrap();
        }
    })
    .await;
    assert!(unavailable.is_ok(), "storage failure was hidden");
    assert!(matches!(
        observer.storage_error.borrow().as_ref().unwrap().as_ref(),
        crate::session_store::StoreError::Unavailable
    ));
    assert!(
        observer.failed.borrow().is_none(),
        "storage unavailability cannot request PTY termination"
    );
    assert!(
        !observer.task.as_ref().unwrap().is_finished(),
        "storage errors must not shut down the owned server"
    );
    assert_eq!(observer.shutdown().await, None);
}

#[cfg(unix)]
#[tokio::test]
async fn sql_observer_driver_shutdown_does_not_wait_for_a_stalled_sql_read() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let native_dir = tempfile::tempdir().unwrap();
    let (plan, environment) = super::super::codex_native::fixture_plan_and_env(
        native_dir.path(),
        &state.integration_probe_cache,
        false,
    );
    let native = plan.start(&environment).await.unwrap();
    let paused = crate::session_store::test_worker::pause(&store);
    let mut observer =
        NativeObserverControl::start_sql(state.clone(), ID.into(), generation, native, writes)
            .unwrap();
    crate::session_store::test_worker::wait_for_get(&store, ID).await;
    assert!(state.projection_lock.try_lock().is_ok());
    assert!(state.workspace.try_lock().is_ok());
    assert!(state.sessions.try_lock().is_ok());
    let stopped =
        tokio::time::timeout(std::time::Duration::from_secs(2), observer.shutdown()).await;
    drop(paused);
    assert_eq!(stopped.unwrap(), None);
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
}

#[cfg(unix)]
#[tokio::test]
async fn sql_observer_driver_refuses_a_foreign_application_binding() {
    let _guard = crate::runtime::peon_runtime::diagnostic_test_guard();
    let (_dir, state, store, writes, _effect, _commands) = fixture_with_root("root").await;
    let generation = state.sessions.lock().unwrap()[ID].runtime.run_generation();
    let native_dir = tempfile::tempdir().unwrap();
    let (plan, environment) = super::super::codex_native::fixture_plan_and_env(
        native_dir.path(),
        &state.integration_probe_cache,
        false,
    );
    let native = plan.start(&environment).await.unwrap();
    let foreign_dir = tempfile::tempdir().unwrap();
    let foreign = crate::test_support::test_app_state_with_workspace(foreign_dir.path());
    assert!(matches!(
        NativeObserverControl::start_sql(foreign, ID.into(), generation, native, writes),
        Err(crate::session_store::StoreError::Conflict)
    ));
    assert_eq!(
        store.get(ID.into()).await.unwrap().unwrap()["metadataRevision"],
        "1"
    );
    store.close().await.unwrap();
}
