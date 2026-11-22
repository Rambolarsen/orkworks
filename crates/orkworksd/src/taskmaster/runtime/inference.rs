//! Custom inference identity and serialized action guard; transport activation is separate.

use super::{
    canonical_workspace_key, effective_settings, proof_matches, reload_durable, validate_selection,
    EvaluationReservation, EvaluationSnapshot, PersistenceGuard, TaskmasterRuntime,
    TaskmasterSettings,
};
use crate::harness::{
    inference::InferenceCapability,
    store::{HarnessSnapshot, HarnessStore},
};
use crate::providers::{
    custom_inference,
    native_inference::{NativeProfile, NativeResponseShape},
    InferenceControl, ProviderDispatchGate, ProviderManager, ProviderOperationError,
    ProviderOperationErrorCode,
};
use crate::taskmaster::{
    inference_trust::{AdapterIdentity, InferenceTrustStore},
    provider_catalog::{self, Availability, Transport},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProviderBindError {
    CatalogUnavailable,
    MissingSelection,
    ProviderUnavailable,
    ProviderChanged,
}

pub(crate) struct AuthorizedInferenceRequest<'a> {
    pub(crate) prompt: String,
    pub(crate) codex_response_shape: Option<NativeResponseShape>,
    pub(crate) control: Option<&'a InferenceControl>,
}

#[derive(Debug)]
pub(crate) enum AuthorizedInferenceError {
    MissingTransport,
    Rejected(ProviderOperationError),
    Provider(ProviderOperationError),
}

/// An immutable evaluation identity, never deserialized from caller-supplied data.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct CapturedInference {
    root: PathBuf,
    workspace: String,
    runtime_generation: u64,
    settings: TaskmasterSettings,
    knowledge_identity: Option<super::KnowledgeIdentity>,
    adapter: AdapterIdentity,
    capability: InferenceCapability,
    trust_generation: u64,
}

impl CapturedInference {
    fn matches_snapshot(&self, workspace: &Path, snapshot: &EvaluationSnapshot) -> bool {
        self.knowledge_identity == snapshot.knowledge_identity
            && self.runtime_generation == snapshot.generation
            && self.settings == snapshot.settings
            && canonical_workspace_key(workspace).as_ref() == Some(&self.workspace)
    }

    /// Include this in the eventual prompt cache key; approval cycles invalidate it.
    pub(crate) fn cache_identity(&self) -> Result<String, String> {
        let bytes =
            serde_json::to_vec(self).map_err(|_| "cannot encode inference evaluation identity")?;
        Ok(format!("{:x}", Sha256::digest(bytes)))
    }
}

fn resolve(snapshot: &HarnessSnapshot, id: &str) -> Option<(AdapterIdentity, InferenceCapability)> {
    let harness = snapshot.registry.get(id)?;
    if harness.definition.retired {
        return None;
    }
    let capability = harness.definition.inference.as_ref()?;
    let identity = AdapterIdentity::resolve(
        id,
        harness.origin,
        capability,
        std::env::var_os("PATH").as_deref(),
    )
    .ok()?;
    Some((identity, capability.clone()))
}

impl TaskmasterRuntime {
    /// Hold the native harness revision and Taskmaster snapshot identity
    /// through a provider side effect such as process spawn or HTTP send.
    pub(crate) fn with_current_native_evaluation(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        snapshot: &EvaluationSnapshot,
        apply: impl FnOnce(),
    ) -> Result<bool, String> {
        self.with_current_native_data(harnesses, workspace, snapshot, |_| apply())
    }

    fn with_current_native_data(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        snapshot: &EvaluationSnapshot,
        apply: impl FnOnce(&mut super::RuntimeData),
    ) -> Result<bool, String> {
        let Some(revision) = &snapshot.native_revision else {
            return Ok(false);
        };
        let Some(selection) = snapshot.settings.selection.as_ref() else {
            return Ok(false);
        };
        if revision.profile.id() != selection.provider {
            return Ok(false);
        }
        let Some(workspace_key) = canonical_workspace_key(workspace) else {
            return Ok(false);
        };

        harnesses
            .with_locked_snapshot(|current| {
                if current.document_revision != revision.document_revision {
                    return Ok(false);
                }
                let _guard = PersistenceGuard::acquire(&self.root)?;
                let mut data = self.data.lock().expect("taskmaster runtime lock poisoned");
                reload_durable(&self.root, &mut data);
                if !data.ledger_readable
                    || data.ledger.generation != snapshot.generation
                    || effective_settings(&data.settings, &workspace_key) != snapshot.settings
                    || !proof_matches(&data, snapshot.knowledge_identity.as_ref())
                {
                    return Ok(false);
                }
                apply(&mut data);
                Ok(true)
            })
            .map_err(|_| "native inference configuration unavailable".to_string())?
    }

    /// Preserve custom authority through final commit, including its binding to
    /// the evaluation's settings and workspace. Never downgrade a custom identity
    /// to the legacy generation-only path when verification fails.
    pub(crate) fn with_current_evaluation(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        snapshot: &EvaluationSnapshot,
        apply: impl FnOnce(),
    ) -> Result<bool, String> {
        if let Some(captured) = &snapshot.custom_inference {
            if !captured.matches_snapshot(workspace, snapshot) {
                return Ok(false);
            }
            self.with_current_custom_inference(harnesses, captured, apply)
        } else {
            self.with_current_native_evaluation(harnesses, workspace, snapshot, apply)
        }
    }

    /// Validate failure identity without writing the legacy global error field.
    pub(crate) fn record_evaluation_error(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        snapshot: &EvaluationSnapshot,
        error: Option<String>,
    ) -> Result<bool, String> {
        if let Some(captured) = &snapshot.custom_inference {
            if !captured.matches_snapshot(workspace, snapshot) {
                return Ok(false);
            }
            let mut saved = Ok(false);
            self.with_current_custom_data(harnesses, captured, |data| {
                saved = Ok(data.ledger_readable);
            })?;
            saved
        } else {
            let _ = error;
            self.with_current_native_evaluation(harnesses, workspace, snapshot, || {})
        }
    }

    /// Cache only an accepted result, revalidating its identity before persistence.
    pub(crate) fn record_evaluation_success(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        snapshot: &EvaluationSnapshot,
        cache_key: &str,
    ) -> Result<bool, String> {
        let Some(workspace_key) = canonical_workspace_key(workspace) else {
            return Ok(false);
        };
        let mut saved = Ok(false);
        let save = |data: &mut super::RuntimeData| {
            data.ledger
                .workspace_cache_keys
                .insert(workspace_key, cache_key.into());
            saved =
                super::write_json(&self.root.join("evaluations.json"), &data.ledger).map(|()| true);
        };
        if let Some(captured) = &snapshot.custom_inference {
            if !captured.matches_snapshot(workspace, snapshot) {
                return Ok(false);
            }
            self.with_current_custom_data(harnesses, captured, save)?;
        } else {
            self.with_current_native_data(harnesses, workspace, snapshot, save)?;
        }
        saved
    }

    /// Reserve under the same trust/definition boundary as spawn and acceptance.
    /// `cache_key` is an internal evaluator value from `snapshot.cache_key(prompt)`,
    /// not a caller-supplied API value; this method does not receive prompt text.
    pub(crate) fn reserve_snapshot(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        now: &str,
        cache_key: &str,
        snapshot: &EvaluationSnapshot,
    ) -> Result<bool, String> {
        self.reserve_snapshot_with_interval(
            harnesses,
            workspace,
            now,
            cache_key,
            snapshot,
            false,
            |reserve| reserve(),
        )
        .map(EvaluationReservation::is_reserved)
    }

    #[cfg(test)]
    pub(crate) fn reserve_manual_snapshot(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        now: &str,
        cache_key: &str,
        snapshot: &EvaluationSnapshot,
    ) -> Result<bool, String> {
        self.reserve_snapshot_with_interval(
            harnesses,
            workspace,
            now,
            cache_key,
            snapshot,
            true,
            |reserve| reserve(),
        )
        .map(EvaluationReservation::is_reserved)
    }

    /// Enter workspace admission only after harness/persistence/data locks.
    pub(crate) fn reserve_manual_snapshot_with_admission(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        now: &str,
        cache_key: &str,
        snapshot: &EvaluationSnapshot,
        admit: impl FnOnce(
            &mut dyn FnMut() -> Result<EvaluationReservation, String>,
        ) -> Result<EvaluationReservation, String>,
    ) -> Result<EvaluationReservation, String> {
        self.reserve_snapshot_with_interval(
            harnesses, workspace, now, cache_key, snapshot, true, admit,
        )
    }

    fn reserve_snapshot_with_interval(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        now: &str,
        cache_key: &str,
        snapshot: &EvaluationSnapshot,
        manual: bool,
        admit: impl FnOnce(
            &mut dyn FnMut() -> Result<EvaluationReservation, String>,
        ) -> Result<EvaluationReservation, String>,
    ) -> Result<EvaluationReservation, String> {
        if let Some(captured) = &snapshot.custom_inference {
            if !captured.matches_snapshot(workspace, snapshot) {
                return Ok(EvaluationReservation::ContextChanged);
            }
            let mut reserved = Ok(EvaluationReservation::Skipped);
            let current = self.with_current_custom_data(harnesses, captured, |data| {
                reserved = admit(&mut || {
                    self.reserve_loaded(
                        data,
                        workspace,
                        now,
                        Some(cache_key),
                        Some(snapshot.generation),
                        manual,
                        manual,
                        manual,
                    )
                });
            })?;
            if !current {
                return Ok(EvaluationReservation::ContextChanged);
            }
            reserved
        } else {
            let mut reserved = Ok(EvaluationReservation::Skipped);
            let current =
                self.with_current_native_data(harnesses, workspace, snapshot, |data| {
                    reserved = admit(&mut || {
                        self.reserve_loaded(
                            data,
                            workspace,
                            now,
                            Some(cache_key),
                            Some(snapshot.generation),
                            manual,
                            manual,
                            manual,
                        )
                    });
                })?;
            if !current {
                return Ok(EvaluationReservation::ContextChanged);
            }
            reserved
        }
    }

    /// Inspect the provider catalog against current trust state, verify the
    /// selected provider is ready, and bind its transport onto `snapshot`.
    pub(crate) fn bind_ready_evaluation_provider(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        snapshot: &mut EvaluationSnapshot,
    ) -> Result<(), ProviderBindError> {
        let trust = InferenceTrustStore::new(self.root.clone());
        let providers = provider_catalog::inspect(harnesses, Some(&trust))
            .map_err(|_| ProviderBindError::CatalogUnavailable)?;
        let selection = snapshot
            .settings
            .selection
            .as_ref()
            .ok_or(ProviderBindError::MissingSelection)?;
        if provider_catalog::evaluation_availability(&providers, selection) != Availability::Ready {
            return Err(ProviderBindError::ProviderUnavailable);
        }
        let transport = providers
            .iter()
            .find(|provider| provider.id == selection.provider)
            .map(|provider| provider.transport.clone())
            .ok_or(ProviderBindError::ProviderUnavailable)?;
        if !self.bind_evaluation_transport(harnesses, workspace, snapshot, transport) {
            return Err(ProviderBindError::ProviderChanged);
        }
        Ok(())
    }

    /// Bind once before collecting permitted context. A custom selection that loses
    /// its capability or approval must abort, never use the native provider branch.
    pub(crate) fn bind_evaluation_transport(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        snapshot: &mut EvaluationSnapshot,
        transport: Transport,
    ) -> bool {
        snapshot.custom_inference = None;
        snapshot.native_revision = None;
        match transport {
            Transport::Native(revision) => {
                if !harnesses
                    .with_locked_snapshot(|current| {
                        current.document_revision == revision.document_revision
                    })
                    .unwrap_or(false)
                {
                    return false;
                }
                snapshot.native_revision = Some(revision);
                true
            }
            Transport::Unsupported => false,
            Transport::Custom => {
                let capture = if snapshot.settings.enabled {
                    self.capture_custom_inference(harnesses, workspace, snapshot)
                } else {
                    self.capture_manual_custom_inference(harnesses, workspace, snapshot)
                };
                let Ok(Some(captured)) = capture else {
                    return false;
                };
                snapshot.custom_inference = Some(captured);
                true
            }
        }
    }

    /// Invoke either the bound custom or native transport while holding
    /// Taskmaster inference authority (`HarnessStore -> PersistenceGuard ->
    /// RuntimeData`) symmetrically across `before_spawn` and actual dispatch.
    ///
    /// `before_spawn` runs *inside* that authority guard and therefore must
    /// never re-enter `TaskmasterRuntime` authority checks; it only enforces
    /// inner caller guards (such as workspace/input freshness) and invokes the
    /// supplied `start` callback while holding them.
    pub(crate) fn invoke_authorized_inference(
        &self,
        providers: &ProviderManager,
        harnesses: &HarnessStore,
        workspace: &Path,
        snapshot: &EvaluationSnapshot,
        request: AuthorizedInferenceRequest<'_>,
        before_spawn: &mut ProviderDispatchGate<'_>,
    ) -> Result<String, AuthorizedInferenceError> {
        let Some(selection) = snapshot.settings.selection.as_ref() else {
            return Err(AuthorizedInferenceError::MissingTransport);
        };
        if let Some(captured) = snapshot.custom_inference.as_ref() {
            if !captured.matches_snapshot(workspace, snapshot) {
                return Err(AuthorizedInferenceError::Rejected(ProviderOperationError {
                    code: ProviderOperationErrorCode::StaleGeneration,
                    message: "custom inference authorization or configuration changed".into(),
                }));
            }
            let mut gate_rejected = false;
            let mut gate = |start: &mut dyn FnMut() -> std::io::Result<()>| {
                let result = before_spawn(start);
                if result.is_none() {
                    gate_rejected = true;
                }
                result
            };
            self.invoke_custom_inference_inner(
                harnesses,
                captured,
                request.prompt,
                &mut gate,
                request.control,
            )
            .map_err(|error| {
                if gate_rejected
                    || matches!(
                        error.code,
                        ProviderOperationErrorCode::StaleGeneration
                            | ProviderOperationErrorCode::VerificationRequired
                    )
                {
                    AuthorizedInferenceError::Rejected(error)
                } else {
                    AuthorizedInferenceError::Provider(error)
                }
            })
        } else if let Some(native) = snapshot.native_revision.as_ref() {
            let mut gate_rejected = false;
            let mut gate = |start: &mut dyn FnMut() -> std::io::Result<()>| {
                let result = self.run_native_dispatch_gate(
                    harnesses,
                    workspace,
                    snapshot,
                    before_spawn,
                    start,
                );
                if result.is_none() {
                    gate_rejected = true;
                }
                result
            };
            let shape = (native.profile == NativeProfile::Codex)
                .then_some(request.codex_response_shape)
                .flatten();
            let result = if let Some(control) = request.control {
                providers.invoke_native_taskmaster_prompt_controlled(
                    native.profile,
                    &selection.model,
                    selection.reasoning_effort.as_deref(),
                    selection.ollama_base_url.as_deref(),
                    request.prompt,
                    shape,
                    &mut gate,
                    control,
                )
            } else {
                providers.invoke_native_taskmaster_prompt_with_response_shape(
                    native.profile,
                    &selection.model,
                    selection.reasoning_effort.as_deref(),
                    selection.ollama_base_url.as_deref(),
                    request.prompt,
                    shape,
                    &mut gate,
                )
            };
            result.map_err(|error| {
                if gate_rejected {
                    AuthorizedInferenceError::Rejected(error)
                } else {
                    AuthorizedInferenceError::Provider(error)
                }
            })
        } else {
            Err(AuthorizedInferenceError::MissingTransport)
        }
    }

    pub(crate) fn run_native_dispatch_gate(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        snapshot: &EvaluationSnapshot,
        before_spawn: &mut ProviderDispatchGate<'_>,
        start: &mut dyn FnMut() -> std::io::Result<()>,
    ) -> Option<std::io::Result<()>> {
        let mut dispatch_result = None;
        let current = self
            .with_current_native_evaluation(harnesses, workspace, snapshot, || {
                dispatch_result = before_spawn(start);
            })
            .unwrap_or(false);
        current.then_some(dispatch_result).flatten()
    }

    /// Invoke only the captured custom transport, revalidating through actual
    /// spawn. This does not reserve usage or accept output into recommendations;
    /// the scheduler must do both separately before this path can be activated.
    #[cfg(test)]
    pub(crate) fn invoke_custom_inference(
        &self,
        harnesses: &HarnessStore,
        captured: &CapturedInference,
        prompt: String,
    ) -> Result<String, ProviderOperationError> {
        let mut gate = |start: &mut dyn FnMut() -> std::io::Result<()>| Some(start());
        self.invoke_custom_inference_inner(harnesses, captured, prompt, &mut gate, None)
    }

    fn invoke_custom_inference_inner(
        &self,
        harnesses: &HarnessStore,
        captured: &CapturedInference,
        prompt: String,
        dispatch_gate: &mut crate::providers::ProviderDispatchGate<'_>,
        control: Option<&crate::providers::InferenceControl>,
    ) -> Result<String, ProviderOperationError> {
        let stale = || ProviderOperationError {
            code: ProviderOperationErrorCode::StaleGeneration,
            message: "custom inference authorization or configuration changed".into(),
        };
        if !self
            .with_current_custom_inference(harnesses, captured, || {})
            .map_err(|_| ProviderOperationError {
                code: ProviderOperationErrorCode::VerificationRequired,
                message: "custom inference authorization could not be verified".into(),
            })?
        {
            return Err(stale());
        }
        let selection = captured.settings.selection.as_ref().ok_or_else(stale)?;
        let prepared = custom_inference::prepare(
            &captured.capability,
            captured.adapter.resolved_path(),
            &selection.model,
            selection.reasoning_effort.as_deref(),
            prompt,
        )?;
        prepared.run_with_spawn_and_prepare_controlled(
            |command, prepare_child| {
                let mut child = None;
                let mut dispatched = false;
                let current = self
                    .with_current_custom_inference(harnesses, captured, || {
                        let mut spawn = || {
                            if control.is_some_and(|c| c.is_cancelled() || c.remaining().is_zero())
                            {
                                return Err(std::io::Error::new(
                                    std::io::ErrorKind::Interrupted,
                                    "cancelled",
                                ));
                            }
                            child = Some(command.spawn().and_then(|mut child| {
                                prepare_child(&mut child)?;
                                Ok(child)
                            }));
                            child
                                .as_ref()
                                .expect("spawn result was recorded")
                                .as_ref()
                                .map(|_| ())
                                .map_err(|error| {
                                    std::io::Error::new(error.kind(), error.to_string())
                                })
                        };
                        dispatched = dispatch_gate(&mut spawn).is_some();
                    })
                    .map_err(|_| ProviderOperationError {
                        code: ProviderOperationErrorCode::VerificationRequired,
                        message: "custom inference authorization could not be verified".into(),
                    })?;
                if !current {
                    return Err(stale());
                }
                if !dispatched {
                    return Err(ProviderOperationError {
                        code: ProviderOperationErrorCode::StaleGeneration,
                        message: "workspace changed before custom inference was dispatched".into(),
                    });
                }
                child.ok_or_else(stale)
            },
            control,
        )
    }

    /// Capture only an approved, currently selected custom definition. No process is run.
    pub(crate) fn capture_custom_inference(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        evaluation: &EvaluationSnapshot,
    ) -> Result<Option<CapturedInference>, String> {
        self.capture_custom_inference_with_disabled(harnesses, workspace, evaluation, false)
    }

    pub(crate) fn capture_manual_custom_inference(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        evaluation: &EvaluationSnapshot,
    ) -> Result<Option<CapturedInference>, String> {
        self.capture_custom_inference_with_disabled(harnesses, workspace, evaluation, true)
    }

    fn capture_custom_inference_with_disabled(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        evaluation: &EvaluationSnapshot,
        allow_disabled: bool,
    ) -> Result<Option<CapturedInference>, String> {
        let Some(workspace) = canonical_workspace_key(workspace) else {
            return Ok(None);
        };
        let Some(selection) = evaluation.settings.selection.as_ref() else {
            return Ok(None);
        };
        validate_selection(selection)?;
        harnesses
            .with_locked_snapshot(|snapshot| {
                let guard = PersistenceGuard::acquire(&self.root)?;
                let Some((adapter, capability)) = resolve(&snapshot, &selection.provider) else {
                    return Ok(None);
                };
                if selection.reasoning_effort.is_some()
                    && capability.reasoning_effort_args().is_none()
                {
                    return Ok(None);
                }
                let trust = InferenceTrustStore::new(self.root.clone())
                    .inspect_guarded(&guard, &adapter)?;
                if !trust.approved {
                    return Ok(None);
                }
                let mut data = self
                    .data
                    .lock()
                    .map_err(|_| "taskmaster runtime lock unavailable")?;
                reload_durable(&self.root, &mut data);
                if !data.ledger_readable
                    || !proof_matches(&data, evaluation.knowledge_identity.as_ref())
                    || data.ledger.generation != evaluation.generation
                    || (!allow_disabled && !evaluation.settings.enabled)
                    || effective_settings(&data.settings, &workspace) != evaluation.settings
                {
                    return Ok(None);
                }
                Ok(Some(CapturedInference {
                    root: self.root.clone(),
                    workspace,
                    runtime_generation: evaluation.generation,
                    settings: evaluation.settings.clone(),
                    knowledge_identity: evaluation.knowledge_identity.clone(),
                    adapter,
                    capability,
                    trust_generation: trust.revision.generation,
                }))
            })
            .map_err(|_| "inference harness configuration unavailable".to_string())?
    }

    /// Revalidate and execute one short synchronous action under the mutation locks.
    /// Use for spawning a process or committing a validated result, never waiting
    /// for inference completion. The action must not call other locking store APIs.
    /// The caller still checks live workspace identity and model evidence inside
    /// the action; this guard does not validate recommendation content.
    pub(crate) fn with_current_custom_inference(
        &self,
        harnesses: &HarnessStore,
        captured: &CapturedInference,
        apply: impl FnOnce(),
    ) -> Result<bool, String> {
        self.with_current_custom_data(harnesses, captured, |_| apply())
    }

    fn with_current_custom_data(
        &self,
        harnesses: &HarnessStore,
        captured: &CapturedInference,
        apply: impl FnOnce(&mut super::RuntimeData),
    ) -> Result<bool, String> {
        if self.root != captured.root {
            return Ok(false);
        }
        harnesses
            .with_locked_snapshot(|snapshot| {
                let guard = PersistenceGuard::acquire(&self.root)?;
                let Some((adapter, _)) = resolve(&snapshot, captured.adapter.harness_id()) else {
                    return Ok(false);
                };
                if adapter != captured.adapter {
                    return Ok(false);
                }
                let trust = InferenceTrustStore::new(self.root.clone())
                    .inspect_guarded(&guard, &adapter)?;
                if !trust.approved || trust.revision.generation != captured.trust_generation {
                    return Ok(false);
                }
                let mut data = self
                    .data
                    .lock()
                    .map_err(|_| "taskmaster runtime lock unavailable")?;
                reload_durable(&self.root, &mut data);
                if !data.ledger_readable
                    || !proof_matches(&data, captured.knowledge_identity.as_ref())
                    || data.ledger.generation != captured.runtime_generation
                    || effective_settings(&data.settings, &captured.workspace) != captured.settings
                {
                    return Ok(false);
                }
                apply(&mut data);
                Ok(true)
            })
            .map_err(|_| "inference harness configuration unavailable".to_string())?
    }
}

#[cfg(test)]
mod tests;
