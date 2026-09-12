//! Custom inference identity and serialized action guard; transport activation is separate.

use super::{
    canonical_workspace_key, effective_settings, reload_durable, validate_selection,
    EvaluationSnapshot, PersistenceGuard, TaskmasterRuntime, TaskmasterSettings,
};
use crate::harness::{
    inference::InferenceCapability,
    store::{HarnessSnapshot, HarnessStore},
};
use crate::providers::{custom_inference, ProviderOperationError, ProviderOperationErrorCode};
use crate::taskmaster::inference_trust::{AdapterIdentity, InferenceTrustStore};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

/// An immutable evaluation identity, never deserialized from caller-supplied data.
#[derive(Clone, Debug, Serialize)]
pub(crate) struct CapturedInference {
    root: PathBuf,
    workspace: String,
    runtime_generation: u64,
    settings: TaskmasterSettings,
    adapter: AdapterIdentity,
    capability: InferenceCapability,
    trust_generation: u64,
}

impl CapturedInference {
    fn matches_snapshot(&self, workspace: &Path, snapshot: &EvaluationSnapshot) -> bool {
        self.runtime_generation == snapshot.generation
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
    /// Retain the inspected native document revision through each mutation.
    /// Unbound snapshots carry no authority, even if a custom identity was lost.
    fn with_native_revision<T>(
        &self,
        harnesses: &HarnessStore,
        snapshot: &EvaluationSnapshot,
        apply: impl FnOnce() -> Result<T, String>,
    ) -> Result<Option<T>, String> {
        let Some(revision) = &snapshot.native_revision else {
            return Ok(None);
        };
        harnesses
            .with_locked_snapshot(|current| {
                if current.document_revision != revision.document_revision {
                    return Ok(None);
                }
                apply().map(Some)
            })
            .map_err(|_| "inference harness configuration unavailable".to_string())?
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
            self.with_native_revision(harnesses, snapshot, || {
                self.with_current_generation(snapshot.generation, apply)
            })
            .map(|result| result.unwrap_or(false))
        }
    }

    /// Stale custom results must not replace or clear current diagnostics.
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
                data.ledger.last_error = error;
                saved = super::write_json(&self.root.join("evaluations.json"), &data.ledger)
                    .map(|()| true);
            })?;
            saved
        } else {
            self.with_native_revision(harnesses, snapshot, || {
                self.record_error(snapshot.generation, error)
            })
            .map(|result| result.unwrap_or(false))
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
            self.with_native_revision(harnesses, snapshot, || {
                let _guard = PersistenceGuard::acquire(&self.root)?;
                let mut data = self
                    .data
                    .lock()
                    .map_err(|_| "taskmaster runtime lock unavailable")?;
                reload_durable(&self.root, &mut data);
                if data.ledger_readable && data.ledger.generation == snapshot.generation {
                    save(&mut data);
                }
                Ok(())
            })?;
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
        if let Some(captured) = &snapshot.custom_inference {
            if !captured.matches_snapshot(workspace, snapshot) {
                return Ok(false);
            }
            let mut reserved = Ok(false);
            self.with_current_custom_data(harnesses, captured, |data| {
                reserved = self.reserve_loaded(
                    data,
                    workspace,
                    now,
                    Some(cache_key),
                    Some(snapshot.generation),
                );
            })?;
            reserved
        } else {
            self.with_native_revision(harnesses, snapshot, || {
                self.reserve_current(workspace, now, Some(cache_key), Some(snapshot.generation))
            })
            .map(|result| result.unwrap_or(false))
        }
    }

    /// Invoke only the captured custom transport, revalidating through actual
    /// spawn. This does not reserve usage or accept output into recommendations;
    /// the scheduler must do both separately before this path can be activated.
    pub(crate) fn invoke_custom_inference(
        &self,
        harnesses: &HarnessStore,
        captured: &CapturedInference,
        prompt: String,
    ) -> Result<String, ProviderOperationError> {
        let stale = || ProviderOperationError {
            code: ProviderOperationErrorCode::StaleGeneration,
            message: "custom inference authorization or configuration changed".into(),
        };
        let selection = captured.settings.selection.as_ref().ok_or_else(stale)?;
        let prepared = custom_inference::prepare(
            &captured.capability,
            captured.adapter.resolved_path(),
            &selection.model,
            selection.reasoning_effort.as_deref(),
            prompt,
        )?;
        prepared.run_with_spawn(|command| {
            let mut child = None;
            let current = self
                .with_current_custom_inference(harnesses, captured, || {
                    child = Some(command.spawn());
                })
                .map_err(|_| ProviderOperationError {
                    code: ProviderOperationErrorCode::VerificationRequired,
                    message: "custom inference authorization could not be verified".into(),
                })?;
            if !current {
                return Err(stale());
            }
            child.ok_or_else(stale)
        })
    }

    /// Capture only an approved, currently selected custom definition. No process is run.
    pub(crate) fn capture_custom_inference(
        &self,
        harnesses: &HarnessStore,
        workspace: &Path,
        evaluation: &EvaluationSnapshot,
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
                    || data.ledger.generation != evaluation.generation
                    || !evaluation.settings.enabled
                    || effective_settings(&data.settings, &workspace) != evaluation.settings
                {
                    return Ok(None);
                }
                Ok(Some(CapturedInference {
                    root: self.root.clone(),
                    workspace,
                    runtime_generation: evaluation.generation,
                    settings: evaluation.settings.clone(),
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
