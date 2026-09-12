//! Read-only selection projection. Custom execution requires current approval.
use super::inference_trust::{AdapterIdentity, InferenceTrustStore};
use crate::harness::{definition::ModelCapability, store::HarnessStore};
use crate::providers::native_inference::NativeProfile;
use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Availability {
    Ready,
    UnsupportedCapability,
    UnsupportedReasoningEffort,
    ApprovalRequired,
    Unavailable,
}

impl Availability {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::UnsupportedCapability => "unsupported_capability",
            Self::UnsupportedReasoningEffort => "unsupported_reasoning_effort",
            Self::ApprovalRequired => "approval_required",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskmasterProvider {
    pub id: String,
    pub label: String,
    pub state: Availability,
    pub models: Vec<String>,
    pub supports_reasoning_effort: bool,
    #[serde(skip)]
    pub transport: Transport,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct NativeRevision {
    pub document_revision: Option<crate::harness::store::HarnessDocumentRevision>,
    pub profile: NativeProfile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Transport {
    Native(NativeRevision),
    Custom,
    Unsupported,
}

pub(crate) fn inspect(
    harnesses: &HarnessStore,
    trust: Option<&InferenceTrustStore>,
) -> Result<Vec<TaskmasterProvider>, ()> {
    harnesses
        .with_locked_snapshot(|snapshot| {
            let path = std::env::var_os("PATH");
            let mut providers = Vec::new();
            for id in snapshot.registry.ids() {
                let Some(harness) = snapshot.registry.get(id) else {
                    continue;
                };
                let definition = &harness.definition;
                let native = NativeProfile::resolve(&snapshot, id);
                let (state, supports_reasoning_effort) =
                    if let Some(capability) = &definition.inference {
                        let status = (!definition.retired)
                            .then(|| {
                                let identity = AdapterIdentity::resolve(
                                    id,
                                    harness.origin,
                                    capability,
                                    path.as_deref(),
                                )
                                .ok()?;
                                trust?.inspect(&identity).ok()
                            })
                            .flatten();
                        let state = match status {
                            Some(status) if status.approved => Availability::Ready,
                            Some(_) => Availability::ApprovalRequired,
                            None => Availability::Unavailable,
                        };
                        (state, capability.reasoning_effort_args().is_some())
                    } else if definition.retired {
                        (Availability::Unavailable, false)
                    } else if native.is_some() {
                        (Availability::Ready, true)
                    } else {
                        (Availability::UnsupportedCapability, false)
                    };
                providers.push(TaskmasterProvider {
                    id: id.into(),
                    label: definition.name.clone(),
                    state,
                    models: match &definition.models {
                        Some(ModelCapability::Static { models }) => models.clone(),
                        _ => Vec::new(),
                    },
                    supports_reasoning_effort,
                    transport: if definition.inference.is_some() {
                        Transport::Custom
                    } else if let Some(profile) = native {
                        Transport::Native(NativeRevision {
                            document_revision: snapshot.document_revision.clone(),
                            profile,
                        })
                    } else {
                        Transport::Unsupported
                    },
                });
            }
            // HTTP inference is code-owned and is not an interactive harness.
            for native in crate::providers::builtin_provider_registry() {
                if !providers.iter().any(|provider| provider.id == native.id) {
                    providers.push(TaskmasterProvider {
                        id: native.id,
                        label: native.label,
                        state: Availability::Ready,
                        models: native.static_models,
                        supports_reasoning_effort: false,
                        transport: Transport::Native(NativeRevision {
                            document_revision: snapshot.document_revision.clone(),
                            profile: NativeProfile::Ollama,
                        }),
                    });
                }
            }
            providers
        })
        .map_err(|_| ())
}

pub(crate) fn selection_availability(providers: &[TaskmasterProvider], id: &str) -> Availability {
    providers
        .iter()
        .find(|provider| provider.id == id)
        .map_or(Availability::UnsupportedCapability, |provider| {
            provider.state
        })
}

pub(crate) fn evaluation_availability(
    providers: &[TaskmasterProvider],
    selection: &super::runtime::TaskmasterSelection,
) -> Availability {
    let state = selection_availability(providers, &selection.provider);
    if state == Availability::Ready
        && selection.reasoning_effort.is_some()
        && providers.iter().any(|provider| {
            provider.id == selection.provider && !provider.supports_reasoning_effort
        })
    {
        Availability::UnsupportedReasoningEffort
    } else {
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::definition::{BuiltinDocument, EMBEDDED_BUILTINS};
    use serde_json::json;
    use std::sync::Arc;

    #[test]
    fn native_override_cannot_fall_through_to_legacy_ready_state() {
        let directory = tempfile::tempdir().unwrap();
        let harnesses = HarnessStore::new(
            directory.path().join("harnesses.json"),
            Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
        );
        let trust = InferenceTrustStore::new(directory.path().join("trust"));
        for (patch, expected) in [
            (json!({}), Availability::Ready),
            (json!({"name":"Renamed Codex"}), Availability::Ready),
            (
                json!({"peon":{"timeoutSecs":10}}),
                Availability::UnsupportedCapability,
            ),
            (
                json!({"launch":{"args":["--arbitrary"]}}),
                Availability::UnsupportedCapability,
            ),
            (
                json!({"inference":null}),
                Availability::UnsupportedCapability,
            ),
            (
                json!({"inference":{"kind":"command","command":std::env::current_exe().unwrap(),"args":["{model}"],"input":"stdin","output":"result-json-v1"}}),
                Availability::ApprovalRequired,
            ),
        ] {
            std::fs::write(
                directory.path().join("harnesses.json"),
                serde_json::to_vec(&json!({"version":3,"custom":[],"overrides":{"codex":patch}}))
                    .unwrap(),
            )
            .unwrap();
            let providers = inspect(&harnesses, Some(&trust)).unwrap();
            assert_eq!(selection_availability(&providers, "codex"), expected);
            assert_eq!(
                providers
                    .iter()
                    .find(|provider| provider.id == "codex")
                    .unwrap()
                    .transport,
                if expected == Availability::ApprovalRequired {
                    Transport::Custom
                } else if expected == Availability::Ready {
                    Transport::Native(NativeRevision {
                        document_revision: harnesses.snapshot().unwrap().document_revision,
                        profile: NativeProfile::Codex,
                    })
                } else {
                    Transport::Unsupported
                }
            );
            assert_eq!(
                selection_availability(&providers, "ollama"),
                Availability::Ready
            );
        }
    }

    #[test]
    fn missing_executable_or_unreadable_trust_is_unavailable_without_losing_selection() {
        let directory = tempfile::tempdir().unwrap();
        let harnesses = HarnessStore::new(
            directory.path().join("harnesses.json"),
            Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
        );
        let trust_root = directory.path().join("trust");
        let trust = InferenceTrustStore::new(trust_root.clone());
        for (command, corrupt) in [
            (directory.path().join("missing"), false),
            (std::env::current_exe().unwrap(), true),
        ] {
            std::fs::write(directory.path().join("harnesses.json"), serde_json::to_vec(&json!({"version":3,"overrides":{},"custom":[{
                "id":"custom-infer","name":"Custom", "launch":{"kind":"platform-shell","login":false},
                "models":{"kind":"command","command":"must-not-run","args":[]},
                "inference":{"kind":"command","command":command,"args":["{model}"],"input":"stdin","output":"result-json-v1"}
            }]})).unwrap()).unwrap();
            if corrupt {
                std::fs::create_dir_all(&trust_root).unwrap();
                std::fs::write(trust_root.join("inference-trust.json"), b"not json").unwrap();
            }
            let providers = inspect(&harnesses, Some(&trust)).unwrap();
            let custom = providers
                .iter()
                .find(|provider| provider.id == "custom-infer")
                .unwrap();
            assert_eq!(custom.state, Availability::Unavailable);
            assert_eq!(custom.transport, Transport::Custom);
            assert!(custom.models.is_empty());
            assert_eq!(
                selection_availability(&providers, "codex"),
                Availability::Ready
            );
        }
    }

    #[test]
    fn ollama_does_not_advertise_ignored_reasoning_effort() {
        let directory = tempfile::tempdir().unwrap();
        let harnesses = HarnessStore::new(
            directory.path().join("harnesses.json"),
            Arc::new(BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap()),
        );
        let providers = inspect(&harnesses, None).unwrap();
        assert!(
            !providers
                .iter()
                .find(|provider| provider.id == "ollama")
                .unwrap()
                .supports_reasoning_effort
        );
    }
}
