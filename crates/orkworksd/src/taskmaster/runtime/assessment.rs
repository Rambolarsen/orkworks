//! Manual assessment authority and proof checks.

use super::{
    canonical_workspace_key, proof_matches, reload_durable, EvaluationSnapshot, PersistenceGuard,
    TaskmasterRuntime,
};
use std::path::Path;

impl TaskmasterRuntime {
    pub(crate) fn readiness_evaluation_snapshot(
        &self,
        workspace: &Path,
    ) -> Result<
        (
            EvaluationSnapshot,
            crate::taskmaster::readiness_rubric::ReadinessRubric,
        ),
        String,
    > {
        let snapshot = self
            .manual_evaluation_snapshot(workspace)
            .ok_or("Taskmaster readiness configuration is unavailable")?;
        self.bind_readiness_evaluation(snapshot)
    }

    fn bind_readiness_evaluation(
        &self,
        snapshot: EvaluationSnapshot,
    ) -> Result<
        (
            EvaluationSnapshot,
            crate::taskmaster::readiness_rubric::ReadinessRubric,
        ),
        String,
    > {
        let _guard = PersistenceGuard::acquire(&self.root)?;
        let mut data = self
            .data
            .lock()
            .map_err(|_| "Readiness knowledge is unavailable")?;
        reload_durable(&self.root, &mut data);
        if !data.ledger_readable
            || data.ledger.generation != snapshot.generation
            || !proof_matches(&data, snapshot.knowledge_identity.as_ref())
        {
            return Err("Readiness configuration or guidance changed during capture".into());
        }
        let proof = data
            .verified_knowledge
            .as_ref()
            .ok_or("Verified readiness guidance is unavailable")?;
        if proof.bundle().format_version != 2
            || !proof
                .bundle()
                .capabilities
                .iter()
                .any(|id| id == "taskmaster-readiness-v1")
        {
            return Err("Complete verified readiness guidance is unavailable".into());
        }
        let rubric = proof
            .bundle()
            .readiness_rubric
            .clone()
            .ok_or("Readiness rubric is missing")?;
        Ok((snapshot, rubric))
    }

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

#[cfg(test)]
mod readiness_identity_tests {
    use super::*;
    use crate::taskmaster::{
        knowledge::test_support::{activation, bundle_value},
        readiness_rubric::CONCEPT_IDS,
    };
    fn bundle(version: &str, sequence: u64) -> serde_json::Value {
        let mut value = bundle_value("valid-activation.json");
        value["formatVersion"] = serde_json::json!(2);
        value["version"] = serde_json::json!(version);
        value["sequence"] = serde_json::json!(sequence);
        value["capabilities"] =
            serde_json::json!(["taskmaster-assessment-v1", "taskmaster-readiness-v1"]);
        value["readinessRubric"] = serde_json::json!({"id":"brain-agent-readiness","revision":sequence,"concepts":CONCEPT_IDS.iter().map(|id|serde_json::json!({"conceptId":id,"pageId":value["pages"][0]["id"],"pageSha256":value["pages"][0]["sha256"],"levels":["Implicit","Discoverable","Usable","Integrated","Adaptive"]})).collect::<Vec<_>>()});
        value
    }
    #[test]
    fn rubric_binding_rejects_activation_between_evaluation_and_proof_capture() {
        let root = tempfile::tempdir().unwrap();
        let workspace = tempfile::tempdir().unwrap();
        let runtime = TaskmasterRuntime::open_fixture(root.path().into());
        runtime
            .activate_knowledge(&activation(bundle("before", 1), false))
            .unwrap();
        let mut settings = crate::taskmaster::runtime::TaskmasterSettings::default();
        settings.selection = Some(
            serde_json::from_value(serde_json::json!({"provider":"codex","model":"fixture"}))
                .unwrap(),
        );
        runtime.replace_settings(settings).unwrap();
        let before = runtime
            .manual_evaluation_snapshot(workspace.path())
            .unwrap();
        runtime
            .activate_knowledge(&activation(bundle("after", 2), false))
            .unwrap();
        assert!(runtime.bind_readiness_evaluation(before).is_err());
        let (current, rubric) = runtime
            .readiness_evaluation_snapshot(workspace.path())
            .unwrap();
        assert_eq!(current.knowledge_identity.unwrap().version, "after");
        assert_eq!(rubric.revision, 2);
    }
}
