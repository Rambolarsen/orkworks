//! User approval application boundary; declaring or trusting an adapter does not run it.
use super::inference_trust::{AdapterIdentity, InferenceTrustStore, TrustRevision};
use crate::harness::{
    inference::InferenceCapability,
    store::{HarnessDocumentRevision, HarnessStore},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ApprovalRevision {
    pub document_revision: HarnessDocumentRevision,
    pub generation: String,
    pub digest: Option<String>,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ApprovalAction {
    Approve,
    Revoke,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ApprovalRequest {
    pub harness_id: String,
    pub action: ApprovalAction,
    pub expected_revision: ApprovalRevision,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdapterTrustView {
    pub id: String,
    pub name: String,
    pub definition: InferenceCapability,
    pub resolved_path: Option<String>,
    pub state: &'static str,
    pub revision: ApprovalRevision,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ApprovalError {
    Conflict,
    Invalid,
    Unavailable,
}

pub(crate) fn inspect_adapters(
    harnesses: &HarnessStore,
    trust: &InferenceTrustStore,
) -> Result<Vec<AdapterTrustView>, ApprovalError> {
    harnesses
        .with_locked_snapshot(|snapshot| {
            let path = std::env::var_os("PATH");
            let mut views = Vec::new();
            for id in snapshot.registry.ids() {
                let harness = snapshot
                    .registry
                    .get(id)
                    .ok_or(ApprovalError::Unavailable)?;
                let Some(definition) = harness.definition.inference.as_ref() else {
                    continue;
                };
                let identity = (!harness.definition.retired)
                    .then(|| {
                        AdapterIdentity::resolve(id, harness.origin, definition, path.as_deref())
                            .ok()
                    })
                    .flatten();
                let (state, generation) = if let Some(identity) = &identity {
                    let status = trust
                        .inspect(identity)
                        .map_err(|_| ApprovalError::Unavailable)?;
                    (
                        if status.approved {
                            "approved"
                        } else {
                            "approval_required"
                        },
                        status.revision.generation,
                    )
                } else {
                    (
                        "unavailable",
                        trust.generation().map_err(|_| ApprovalError::Unavailable)?,
                    )
                };
                views.push(AdapterTrustView {
                    id: id.into(),
                    name: harness.definition.name.clone(),
                    definition: definition.clone(),
                    resolved_path: identity
                        .as_ref()
                        .map(|identity| identity.resolved_path().to_string_lossy().into_owned()),
                    state,
                    revision: ApprovalRevision {
                        document_revision: snapshot
                            .document_revision
                            .clone()
                            .ok_or(ApprovalError::Unavailable)?,
                        generation: generation.to_string(),
                        digest: identity.as_ref().map(|identity| identity.digest().into()),
                    },
                });
            }
            Ok(views)
        })
        .map_err(|_| ApprovalError::Unavailable)?
}

pub(crate) fn change_approval(
    harnesses: &HarnessStore,
    trust: &InferenceTrustStore,
    request: ApprovalRequest,
) -> Result<(), ApprovalError> {
    if request
        .expected_revision
        .digest
        .as_ref()
        .is_some_and(|digest| {
            digest.len() != 64
                || !digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        })
    {
        return Err(ApprovalError::Invalid);
    }
    let generation: u64 = request
        .expected_revision
        .generation
        .parse()
        .map_err(|_| ApprovalError::Invalid)?;
    if generation.to_string() != request.expected_revision.generation {
        return Err(ApprovalError::Invalid);
    }
    harnesses
        .with_locked_snapshot(|snapshot| {
            if snapshot.document_revision.as_ref()
                != Some(&request.expected_revision.document_revision)
            {
                return Err(ApprovalError::Conflict);
            }
            let harness = snapshot
                .registry
                .get(&request.harness_id)
                .ok_or(ApprovalError::Invalid)?;
            let definition = harness
                .definition
                .inference
                .as_ref()
                .ok_or(ApprovalError::Invalid)?;
            match request.action {
                ApprovalAction::Approve => {
                    let digest = request
                        .expected_revision
                        .digest
                        .as_ref()
                        .ok_or(ApprovalError::Invalid)?;
                    if harness.definition.retired {
                        return Err(ApprovalError::Invalid);
                    }
                    let identity = AdapterIdentity::resolve(
                        &request.harness_id,
                        harness.origin,
                        definition,
                        std::env::var_os("PATH").as_deref(),
                    )
                    .map_err(|_| ApprovalError::Conflict)?;
                    trust
                        .approve(
                            &identity,
                            &TrustRevision {
                                generation,
                                digest: digest.clone(),
                            },
                        )
                        .map_err(trust_error)
                }
                ApprovalAction::Revoke => {
                    let identity = (!harness.definition.retired)
                        .then(|| {
                            AdapterIdentity::resolve(
                                &request.harness_id,
                                harness.origin,
                                definition,
                                std::env::var_os("PATH").as_deref(),
                            )
                            .ok()
                        })
                        .flatten();
                    if identity.as_ref().map(|identity| identity.digest())
                        != request.expected_revision.digest.as_deref()
                    {
                        return Err(ApprovalError::Conflict);
                    }
                    trust
                        .revoke(&request.harness_id, generation)
                        .map_err(trust_error)
                }
            }
        })
        .map_err(|_| ApprovalError::Unavailable)?
}

fn trust_error(error: String) -> ApprovalError {
    if error == "inference trust revision conflict" {
        ApprovalError::Conflict
    } else {
        ApprovalError::Unavailable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::definition::{BuiltinDocument, EMBEDDED_BUILTINS};
    use serde_json::json;
    use std::sync::Arc;

    fn fixture(root: &std::path::Path) -> (HarnessStore, InferenceTrustStore, serde_json::Value) {
        let raw = json!({"version":3,"overrides":{},"custom":[{"id":"custom-infer","name":"Custom", "launch":{"kind":"platform-shell","login":false},
            "inference":{"kind":"command","command":std::env::current_exe().unwrap(),"args":["{model}"],"input":"stdin","output":"result-json-v1"}}]});
        std::fs::write(
            root.join("harnesses.json"),
            serde_json::to_vec(&raw).unwrap(),
        )
        .unwrap();
        (
            HarnessStore::new(
                root.join("harnesses.json"),
                Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
            ),
            InferenceTrustStore::new(root.join("trust")),
            raw,
        )
    }

    #[test]
    fn inference_approval_lists_without_peon_and_rejects_reused_or_edited_revisions() {
        let dir = tempfile::tempdir().unwrap();
        let (harnesses, trust, mut raw) = fixture(dir.path());
        let view = inspect_adapters(&harnesses, &trust)
            .unwrap()
            .pop()
            .expect("custom adapter");
        assert_eq!(view.state, "approval_required");
        let request = ApprovalRequest {
            harness_id: view.id,
            action: ApprovalAction::Approve,
            expected_revision: view.revision,
        };
        change_approval(&harnesses, &trust, request.clone()).unwrap();
        assert_eq!(
            inspect_adapters(&harnesses, &trust).unwrap()[0].state,
            "approved"
        );
        assert_eq!(
            change_approval(&harnesses, &trust, request),
            Err(ApprovalError::Conflict)
        );
        let view = inspect_adapters(&harnesses, &trust).unwrap().pop().unwrap();
        raw["custom"][0]["inference"]["args"] = json!(["--changed", "{model}"]);
        std::fs::write(
            dir.path().join("harnesses.json"),
            serde_json::to_vec(&raw).unwrap(),
        )
        .unwrap();
        assert_eq!(
            change_approval(
                &harnesses,
                &trust,
                ApprovalRequest {
                    harness_id: view.id,
                    action: ApprovalAction::Approve,
                    expected_revision: view.revision
                }
            ),
            Err(ApprovalError::Conflict)
        );
        assert_eq!(
            inspect_adapters(&harnesses, &trust).unwrap()[0].state,
            "approval_required"
        );
    }

    #[test]
    fn inference_approval_allows_revocation_for_an_unavailable_executable() {
        let dir = tempfile::tempdir().unwrap();
        let (harnesses, trust, mut raw) = fixture(dir.path());
        raw["custom"][0]["inference"]["command"] = json!(dir.path().join("missing-executable"));
        std::fs::write(
            dir.path().join("harnesses.json"),
            serde_json::to_vec(&raw).unwrap(),
        )
        .unwrap();
        let view = inspect_adapters(&harnesses, &trust)
            .unwrap()
            .pop()
            .expect("unavailable adapter");
        assert_eq!(view.state, "unavailable");
        assert!(view.revision.digest.is_none());
        let mut request = ApprovalRequest {
            harness_id: view.id,
            action: ApprovalAction::Approve,
            expected_revision: view.revision,
        };
        assert_eq!(
            change_approval(&harnesses, &trust, request.clone()),
            Err(ApprovalError::Invalid)
        );
        request.action = ApprovalAction::Revoke;
        change_approval(&harnesses, &trust, request).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn inference_approval_rechecks_executable_resolution_after_inspection() {
        use std::os::unix::fs::{symlink, PermissionsExt};
        let dir = tempfile::tempdir().unwrap();
        let (harnesses, trust, mut raw) = fixture(dir.path());
        for name in ["first", "second"] {
            let target = dir.path().join(name);
            std::fs::write(&target, b"not executed").unwrap();
            std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700)).unwrap();
        }
        let link = dir.path().join("adapter");
        symlink(dir.path().join("first"), &link).unwrap();
        raw["custom"][0]["inference"]["command"] = json!(link);
        std::fs::write(
            dir.path().join("harnesses.json"),
            serde_json::to_vec(&raw).unwrap(),
        )
        .unwrap();
        let view = inspect_adapters(&harnesses, &trust).unwrap().pop().unwrap();
        std::fs::remove_file(&link).unwrap();
        symlink(dir.path().join("second"), &link).unwrap();
        assert_eq!(
            change_approval(
                &harnesses,
                &trust,
                ApprovalRequest {
                    harness_id: view.id,
                    action: ApprovalAction::Approve,
                    expected_revision: view.revision,
                }
            ),
            Err(ApprovalError::Conflict)
        );
        assert!(!dir.path().join("trust/inference-trust.json").exists());
    }

    #[test]
    fn inference_approval_rejects_malformed_digests_and_stale_available_revocation() {
        let dir = tempfile::tempdir().unwrap();
        let (harnesses, trust, _) = fixture(dir.path());
        let view = inspect_adapters(&harnesses, &trust).unwrap().pop().unwrap();
        let request = ApprovalRequest {
            harness_id: view.id,
            action: ApprovalAction::Approve,
            expected_revision: view.revision,
        };
        let mut malformed = request.clone();
        malformed.expected_revision.digest = Some("not-a-digest".into());
        assert_eq!(
            change_approval(&harnesses, &trust, malformed),
            Err(ApprovalError::Invalid)
        );
        change_approval(&harnesses, &trust, request).unwrap();
        let view = inspect_adapters(&harnesses, &trust).unwrap().pop().unwrap();
        for digest in [None, Some("c".repeat(64))] {
            let mut revision = view.revision.clone();
            revision.digest = digest;
            assert_eq!(
                change_approval(
                    &harnesses,
                    &trust,
                    ApprovalRequest {
                        harness_id: view.id.clone(),
                        action: ApprovalAction::Revoke,
                        expected_revision: revision
                    }
                ),
                Err(ApprovalError::Conflict)
            );
            assert_eq!(
                inspect_adapters(&harnesses, &trust).unwrap()[0].state,
                "approved"
            );
        }
    }
}
