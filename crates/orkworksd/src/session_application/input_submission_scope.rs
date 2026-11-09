//! Offline-qualified capture adapter; production delivery does not call this yet.
use super::*;
use crate::runtime::input_submission::scope::{
    CaptureRefusal, OriginalInputScope, ScopeError, ScopeReservation,
};

impl SessionApplication {
    #[allow(dead_code)] // Production coordinator/recipe integration remains gated.
    fn capture_input_scope(
        &self,
        target: &RecommendationDeliveryTarget,
        reservation: ScopeReservation,
    ) -> Result<OriginalInputScope, CaptureRefusal> {
        let Ok(_projection) = self.state.projection_lock.lock() else {
            return Err(reservation.refuse(ScopeError::Unavailable));
        };
        let Ok(workspace) = self.state.workspace.lock() else {
            return Err(reservation.refuse(ScopeError::Unavailable));
        };
        let Some(workspace) = workspace
            .as_ref()
            .filter(|workspace| target.workspace.matches(workspace))
        else {
            return Err(reservation.refuse(ScopeError::Stale));
        };
        let Some(lease) = workspace.lease.as_ref() else {
            return Err(reservation.refuse(ScopeError::Stale));
        };
        let Ok(sessions) = self.state.sessions.lock() else {
            return Err(reservation.refuse(ScopeError::Unavailable));
        };
        let Some(handle) = sessions.get(&target.session_id) else {
            return Err(reservation.refuse(ScopeError::Stale));
        };
        if !handle.runtime.matches_identity(&target.runtime_identity)
            || !handle.runtime.control_tx.same_channel(&target.control_tx)
        {
            return Err(reservation.refuse(ScopeError::Stale));
        }
        reservation.capture(
            &workspace.path,
            lease,
            &target.session_id,
            &target.runtime_identity,
            &target.control_tx,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::super::*;
    use crate::runtime::input_submission::scope::{tests::budget, ScopeError};

    fn target(state: &Arc<AppState>) -> RecommendationDeliveryTarget {
        let workspace = state.workspace.lock().unwrap();
        let sessions = state.sessions.lock().unwrap();
        let runtime = &sessions.get("scope-session").unwrap().runtime;
        RecommendationDeliveryTarget {
            session_id: "scope-session".into(),
            runtime_identity: runtime.identity(),
            control_tx: runtime.control_tx.clone(),
            workspace: RecommendationDeliveryWorkspace::capture(workspace.as_ref().unwrap()),
        }
    }
    fn state(root: &Path) -> Arc<AppState> {
        let state = crate::test_support::test_app_state_with_workspace(root);
        state.workspace.lock().unwrap().as_mut().unwrap().lease =
            Some(Arc::new(WorkspaceLease::acquire(root).unwrap()));
        state.sessions.lock().unwrap().insert(
            "scope-session".into(),
            super::super::tests::attention_test_handle("scope-session", root),
        );
        state
    }

    #[test]
    fn input_scope_capture_retains_exact_original_target_after_selection_loss() {
        let root = tempfile::tempdir().unwrap();
        let state = state(root.path());
        let application = SessionApplication::new(state.clone());
        let target = target(&state);
        let budget = budget();
        let scope = application
            .capture_input_scope(&target, budget.reserve().unwrap())
            .unwrap_or_else(|e| panic!("{:?}", e.reason));
        // Replacing the store under the same retained lease models reopen:
        // store instance alone must not invent a new maintenance incarnation.
        let reopened =
            crate::workflow_observations::WorkflowObservationStore::open(root.path().to_path_buf())
                .unwrap();
        assert_ne!(reopened.instance_id(), target.workspace.store_instance);
        state
            .workspace
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .workflow_observations = reopened;
        let scope2_budget = crate::runtime::input_submission::scope::tests::budget();
        let scope2 = application
            .capture_input_scope(&target, scope2_budget.reserve().unwrap())
            .unwrap_or_else(|e| panic!("{:?}", e.reason));
        drop(scope2);
        let old = state.workspace.lock().unwrap().take().unwrap();
        assert!(scope.matches(
            &old.path,
            old.lease.as_ref().unwrap(),
            &target.session_id,
            &target.runtime_identity,
            &target.control_tx
        ));
        drop(old);
        assert!(WorkspaceLease::acquire(root.path()).is_err());
        drop(scope);
        assert!(WorkspaceLease::acquire(root.path()).is_ok());
        assert!(budget.reserve().is_ok());
    }

    #[test]
    fn input_scope_capture_refuses_missing_lease_runtime_and_stale_channel() {
        for mode in [
            "lease",
            "runtime",
            "identity",
            "channel",
            "workspace",
            "replaced-lease",
        ] {
            let root = tempfile::tempdir().unwrap();
            let state = state(root.path());
            let application = SessionApplication::new(state.clone());
            let mut target = target(&state);
            match mode {
                "lease" => {
                    state.workspace.lock().unwrap().as_mut().unwrap().lease = None;
                }
                "runtime" => {
                    state.sessions.lock().unwrap().clear();
                }
                "identity" => {
                    target.runtime_identity.run_generation += 1;
                }
                "channel" => {
                    target.control_tx = tokio::sync::mpsc::channel(1).0;
                }
                "replaced-lease" => {
                    let other = tempfile::tempdir().unwrap();
                    state.workspace.lock().unwrap().as_mut().unwrap().lease =
                        Some(Arc::new(WorkspaceLease::acquire(other.path()).unwrap()));
                }
                _ => {
                    target.workspace.path.push("different");
                }
            }
            let budget = budget();
            let refusal = match application.capture_input_scope(&target, budget.reserve().unwrap())
            {
                Err(e) => e,
                Ok(_) => panic!("stale {mode} captured"),
            };
            assert_eq!(refusal.reason, ScopeError::Stale);
            assert!(matches!(budget.reserve(), Err(ScopeError::Capacity)));
            drop(refusal.reservation);
            assert!(budget.reserve().is_ok());
        }
    }
}
