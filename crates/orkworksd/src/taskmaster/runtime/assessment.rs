//! Manual assessment authority and proof checks.

use super::{
    canonical_workspace_key, proof_matches, reload_durable, EvaluationSnapshot, PersistenceGuard,
    TaskmasterRuntime,
};
use std::path::Path;

impl TaskmasterRuntime {
    /// Assessment requires a fresh durable signature proof and the explicit
    /// capability marker. A projected prompt bundle is never sufficient proof.
    pub(crate) fn assessment_knowledge_availability(&self) -> Result<(), &'static str> {
        let reason =
            "Brain assessment is unavailable until verified assessment guidance is available.";
        let _guard = PersistenceGuard::acquire(&self.root).map_err(|_| reason)?;
        let mut data = self.data.lock().map_err(|_| reason)?;
        reload_durable(&self.root, &mut data);
        let Some(proof) = data.verified_knowledge.as_ref() else {
            return Err(reason);
        };
        if !data.ledger_readable
            || !proof
                .bundle()
                .capabilities
                .iter()
                .any(|item| item == "taskmaster-assessment-v1")
        {
            return Err(reason);
        }
        Ok(())
    }

    pub(crate) fn assessment_evaluation_snapshot(
        &self,
        workspace: &Path,
    ) -> Result<EvaluationSnapshot, &'static str> {
        self.assessment_knowledge_availability()?;
        let snapshot = self
            .manual_evaluation_snapshot(workspace)
            .ok_or("Taskmaster assessment is unavailable for this workspace.")?;
        let _guard = PersistenceGuard::acquire(&self.root)
            .map_err(|_| "Taskmaster assessment configuration is unavailable.")?;
        let mut data = self
            .data
            .lock()
            .map_err(|_| "Taskmaster assessment configuration is unavailable.")?;
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable
            || data.ledger.generation != snapshot.generation
            || !proof_matches(&data, snapshot.knowledge_identity.as_ref())
            || canonical_workspace_key(workspace).is_none()
        {
            return Err("Taskmaster assessment configuration changed.");
        }
        Ok(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::knowledge::test_support::{activation, bundle_value};

    #[test]
    fn durable_assessment_gate_requires_the_exact_signed_capability() {
        for (capabilities, eligible) in [
            (serde_json::json!([]), false),
            (serde_json::json!(["taskmaster-assessment-v2"]), false),
            (serde_json::json!(["taskmaster-assessment-v1"]), true),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let runtime = TaskmasterRuntime::open_fixture(directory.path().into());
            let mut bundle = bundle_value("valid-activation.json");
            bundle["capabilities"] = capabilities;
            runtime
                .activate_knowledge(&activation(bundle, false))
                .unwrap();
            assert!(runtime.brain_knowledge_availability().is_ok());
            assert_eq!(
                runtime.assessment_knowledge_availability().is_ok(),
                eligible
            );
            let restarted = TaskmasterRuntime::open_fixture(directory.path().into());
            assert_eq!(
                restarted.assessment_knowledge_availability().is_ok(),
                eligible
            );
            std::fs::write(runtime.knowledge_path(), "corrupt").unwrap();
            assert!(runtime.assessment_knowledge_availability().is_err());
        }
    }
}
