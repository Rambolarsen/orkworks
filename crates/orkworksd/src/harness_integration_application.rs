//! Application-owned orchestration for workspace harness integrations.
//!
//! Inspection and workspace listing are observations, but may demote stale
//! Claude/Copilot prompt authority when an owned prompt hook is missing or
//! drifted. They never install configuration or activate authority. Launch
//! readiness is a conservative query and does not revoke authority.

use crate::harness::integration::{
    binding_for_key, IntegrationContext, IntegrationError, IntegrationKey, IntegrationOwnership,
    IntegrationRegistration, IntegrationStatus, ReporterAssetResolver,
};
use crate::harness::registry::{ResolvedHarness, ResolvedIntegrationGroup};
use crate::harness::store::HarnessDocumentRevision;
use crate::{AppState, WorkspaceState};
use serde::Serialize;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum IntegrationTarget {
    Harness(String),
    Group(IntegrationKey),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum IntegrationMutation {
    Install,
    Repair,
    Uninstall,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct IntegrationRevisionExpectation {
    pub(crate) document_revision: Option<HarnessDocumentRevision>,
    pub(crate) active_harness_revision: u64,
    pub(crate) workspace_path: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum IntegrationMutationRequest {
    Harness {
        harness_id: String,
        mutation: IntegrationMutation,
    },
    Group {
        key: IntegrationKey,
        mutation: IntegrationMutation,
        expected: IntegrationRevisionExpectation,
    },
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GroupedIntegrationStatus {
    pub(crate) key: IntegrationKey,
    pub(crate) consumers: Vec<crate::harness::integration::IntegrationConsumer>,
    pub(crate) status: IntegrationStatus,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationCleanupResponse {
    pub(crate) status: &'static str,
    pub(crate) outcomes: Vec<GroupedIntegrationStatus>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub(crate) errors: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum IntegrationInspection {
    Harness(IntegrationStatus),
    Group(GroupedIntegrationStatus),
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IntegrationRevisionConflict {
    pub(crate) document_revision: Option<HarnessDocumentRevision>,
    pub(crate) active_harness_revision: u64,
}

#[derive(Debug)]
pub(crate) enum IntegrationApplicationError {
    NoWorkspace,
    UnknownHarness(String),
    UnknownIntegration(IntegrationKey),
    WorkspaceChanged,
    DefinitionChanged,
    RevisionConflict(IntegrationRevisionConflict),
    Configuration(IntegrationError),
    Infrastructure(String),
    AuthorityRevocation,
}

impl std::fmt::Display for IntegrationApplicationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoWorkspace => write!(f, "{}", IntegrationError::NoWorkspace),
            Self::UnknownHarness(id) => write!(f, "unknown harness id \"{id}\""),
            Self::UnknownIntegration(key) => {
                write!(
                    f,
                    "unknown integration key {}/{}",
                    key.adapter_id, key.target_id
                )
            }
            Self::WorkspaceChanged => write!(f, "workspace changed during this request; retry"),
            Self::DefinitionChanged => {
                write!(f, "harness definition changed during this request; retry")
            }
            Self::RevisionConflict(_) => {
                write!(f, "Integration state changed; reload before retrying.")
            }
            Self::Configuration(error) => write!(f, "{error}"),
            Self::Infrastructure(message) => write!(f, "{message}"),
            Self::AuthorityRevocation => write!(
                f,
                "integration changed, but prompt authority could not be safely revoked"
            ),
        }
    }
}

pub(crate) struct HarnessIntegrationApplication {
    state: Arc<AppState>,
    #[cfg(test)]
    revocation_result_hook: Option<Arc<dyn Fn(bool) -> bool + Send + Sync>>,
    #[cfg(test)]
    group_revalidation_hook: Option<Arc<dyn Fn(usize) + Send + Sync>>,
    #[cfg(test)]
    probe_revalidation_hook: Option<Arc<dyn Fn(&str) + Send + Sync>>,
}

impl HarnessIntegrationApplication {
    pub(crate) fn new(state: Arc<AppState>) -> Self {
        Self {
            state,
            #[cfg(test)]
            revocation_result_hook: None,
            #[cfg(test)]
            group_revalidation_hook: None,
            #[cfg(test)]
            probe_revalidation_hook: None,
        }
    }

    /// Observe integration state. For Claude and Copilot prompt integrations,
    /// inspection may revoke stale prompt authority if the owned hook is not
    /// ready; it never installs configuration or activates authority.
    pub(crate) async fn inspect(
        &self,
        target: IntegrationTarget,
    ) -> Result<IntegrationInspection, IntegrationApplicationError> {
        match target {
            IntegrationTarget::Harness(harness_id) => self
                .inspect_harness(&harness_id)
                .await
                .map(IntegrationInspection::Harness),
            IntegrationTarget::Group(key) => self
                .inspect_group(&key, None)
                .await
                .map(IntegrationInspection::Group),
        }
    }

    pub(crate) async fn mutate(
        &self,
        request: IntegrationMutationRequest,
    ) -> Result<IntegrationInspection, IntegrationApplicationError> {
        match request {
            IntegrationMutationRequest::Harness {
                harness_id,
                mutation,
            } => self
                .mutate_harness(&harness_id, mutation)
                .await
                .map(IntegrationInspection::Harness),
            IntegrationMutationRequest::Group {
                key,
                mutation,
                expected,
            } => self
                .mutate_group(&key, mutation, expected)
                .await
                .map(IntegrationInspection::Group),
        }
    }

    /// Observe each active integration. As with [`Self::inspect`], this may
    /// revoke stale Claude/Copilot prompt authority when an owned hook is not
    /// ready; it does not install configuration or activate authority.
    pub(crate) async fn list_workspace(
        &self,
    ) -> Result<Vec<GroupedIntegrationStatus>, IntegrationApplicationError> {
        self.list_workspace_inner().await
    }

    pub(crate) async fn reconcile_unreferenced(
        &self,
        keys: BTreeSet<IntegrationKey>,
        expected_workspace_path: Option<PathBuf>,
    ) -> IntegrationCleanupResponse {
        self.reconcile_unreferenced_inner(keys, expected_workspace_path)
            .await
    }

    pub(crate) fn prompt_attention_launch_ready(&self, harness_id: &str, executable: &str) -> bool {
        prompt_attention_launch_ready(&self.state, harness_id, executable)
    }

    fn revoke_snapshot(
        &self,
        snapshot: crate::session_application::PromptAuthorityRevokeSnapshot,
    ) -> Result<(), IntegrationApplicationError> {
        let real = crate::session_application::SessionApplication::new(self.state.clone())
            .revoke_prompt_authority_snapshot(snapshot)
            .is_ok();
        #[cfg(test)]
        let revoked = self
            .revocation_result_hook
            .as_ref()
            .map_or(real, |hook| hook(real));
        #[cfg(not(test))]
        let revoked = real;
        if revoked {
            Ok(())
        } else {
            Err(IntegrationApplicationError::AuthorityRevocation)
        }
    }

    async fn inspect_harness(
        &self,
        harness_id: &str,
    ) -> Result<IntegrationStatus, IntegrationApplicationError> {
        let mut snapshot = None;
        let result = self
            .with_revalidated_harness(harness_id, |harness, workspace, ctx| {
                let ready_before = harness
                    .definition
                    .integration
                    .as_ref()
                    .is_some_and(|binding| {
                        crate::harness::integration::prompt_attention_hook_ready(binding, &ctx)
                    });
                if matches!(harness_id, "claude-code" | "copilot") && !ready_before {
                    snapshot = Some(
                        self.session_app()
                            .snapshot_prompt_authority_for_harness(harness_id, &workspace.path),
                    );
                }
                let result = harness.integration_status(&ctx);
                let ready_after = harness
                    .definition
                    .integration
                    .as_ref()
                    .is_some_and(|binding| {
                        crate::harness::integration::prompt_attention_hook_ready(binding, &ctx)
                    });
                if matches!(harness_id, "claude-code" | "copilot")
                    && !ready_after
                    && snapshot.is_none()
                {
                    snapshot = Some(
                        self.session_app()
                            .snapshot_prompt_authority_for_harness(harness_id, &workspace.path),
                    );
                }
                result.map_err(IntegrationApplicationError::Configuration)
            })
            .await;
        self.finalize_snapshot(snapshot)?;
        result
    }

    async fn mutate_harness(
        &self,
        harness_id: &str,
        mutation: IntegrationMutation,
    ) -> Result<IntegrationStatus, IntegrationApplicationError> {
        let mut snapshot = None;
        let result = self
            .with_revalidated_harness(harness_id, |harness, workspace, ctx| {
                let ready_before = harness
                    .definition
                    .integration
                    .as_ref()
                    .is_some_and(|binding| {
                        crate::harness::integration::prompt_attention_hook_ready(binding, &ctx)
                    });
                if matches!(harness_id, "claude-code" | "copilot") && !ready_before {
                    snapshot = Some(
                        self.session_app()
                            .snapshot_prompt_authority_for_harness(harness_id, &workspace.path),
                    );
                }
                let result = apply_mutation(harness, &ctx, mutation);
                let ready_after = harness
                    .definition
                    .integration
                    .as_ref()
                    .is_some_and(|binding| {
                        crate::harness::integration::prompt_attention_hook_ready(binding, &ctx)
                    });
                if matches!(harness_id, "claude-code" | "copilot")
                    && !ready_after
                    && snapshot.is_none()
                {
                    snapshot = Some(
                        self.session_app()
                            .snapshot_prompt_authority_for_harness(harness_id, &workspace.path),
                    );
                }
                result.map_err(IntegrationApplicationError::Configuration)
            })
            .await;
        self.finalize_snapshot(snapshot)?;
        result
    }

    async fn inspect_group(
        &self,
        key: &IntegrationKey,
        expected: Option<IntegrationRevisionExpectation>,
    ) -> Result<GroupedIntegrationStatus, IntegrationApplicationError> {
        self.run_group_action(key, expected, "retry", |harness, ctx| {
            harness.integration_status(ctx)
        })
        .await
    }

    async fn mutate_group(
        &self,
        key: &IntegrationKey,
        mutation: IntegrationMutation,
        expected: IntegrationRevisionExpectation,
    ) -> Result<GroupedIntegrationStatus, IntegrationApplicationError> {
        let action = match mutation {
            IntegrationMutation::Install | IntegrationMutation::Repair => "action-needed",
            IntegrationMutation::Uninstall => "cleanup-needed",
        };
        self.run_group_action(key, Some(expected), action, |harness, ctx| {
            apply_mutation(harness, ctx, mutation)
        })
        .await
    }

    fn session_app(&self) -> crate::session_application::SessionApplication {
        crate::session_application::SessionApplication::new(self.state.clone())
    }

    fn finalize_snapshot(
        &self,
        snapshot: Option<crate::session_application::PromptAuthorityRevokeSnapshot>,
    ) -> Result<(), IntegrationApplicationError> {
        if let Some(snapshot) = snapshot {
            self.revoke_snapshot(snapshot)?;
        }
        Ok(())
    }

    async fn with_revalidated_harness<R>(
        &self,
        harness_id: &str,
        action: impl FnOnce(
            &ResolvedHarness,
            &WorkspaceState,
            &IntegrationContext<'_>,
        ) -> Result<R, IntegrationApplicationError>,
    ) -> Result<R, IntegrationApplicationError> {
        let workspace_path_at_start = self
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .map(|ws| ws.path.clone())
            .ok_or(IntegrationApplicationError::NoWorkspace)?;
        let harness = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned")
            .get(harness_id)
            .cloned()
            .ok_or_else(|| IntegrationApplicationError::UnknownHarness(harness_id.into()))?;
        let detected = crate::harness::detect::resolve_tool_gate(
            &self.state.integration_probe_cache,
            &harness.definition.id,
            &harness.launch_command(),
            harness.definition.min_version.as_ref(),
        )
        .await;
        #[cfg(test)]
        if let Some(hook) = &self.probe_revalidation_hook {
            hook(harness_id);
        }
        {
            let registry = self
                .state
                .harness_catalog
                .read()
                .expect("harness catalog lock poisoned");
            match registry.get(harness_id) {
                Some(current) if current.definition == harness.definition => {}
                _ => return Err(IntegrationApplicationError::DefinitionChanged),
            }
        }
        let _projection = self
            .state
            .projection_lock
            .lock()
            .expect("projection lock poisoned");
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard
            .as_ref()
            .ok_or(IntegrationApplicationError::NoWorkspace)?;
        if workspace.path != workspace_path_at_start {
            return Err(IntegrationApplicationError::WorkspaceChanged);
        }
        let reporter_assets =
            reporter_assets().map_err(IntegrationApplicationError::Infrastructure)?;
        let home = dirs::home_dir().ok_or_else(|| {
            IntegrationApplicationError::Infrastructure("couldn't resolve home directory".into())
        })?;
        let orkworks_root = home.join(".orkworks");
        let ctx = IntegrationContext {
            workspace: &workspace.path,
            workspace_metadata: Some(&workspace.metadata),
            orkworks_root: &orkworks_root,
            enabled: workspace_harness_enabled(workspace, &harness.definition.id),
            detected_tool: detected.as_ref(),
            reporter_assets: &reporter_assets,
        };
        action(&harness, workspace, &ctx)
    }

    async fn run_group_action(
        &self,
        key: &IntegrationKey,
        expected: Option<IntegrationRevisionExpectation>,
        failure_action: &'static str,
        action: impl FnOnce(
            &ResolvedHarness,
            &IntegrationContext<'_>,
        ) -> Result<IntegrationStatus, IntegrationError>,
    ) -> Result<GroupedIntegrationStatus, IntegrationApplicationError> {
        let mut snapshot = None;
        let result = self
            .with_revalidated_key(key, expected.as_ref(), |group, harness, ctx| {
                let ready_before = harness
                    .definition
                    .integration
                    .as_ref()
                    .is_some_and(|binding| {
                        crate::harness::integration::prompt_attention_hook_ready(binding, ctx)
                    });
                if !ready_before {
                    if let Some(harness_id) =
                        prompt_harness_for_integration_adapter(&key.adapter_id)
                    {
                        snapshot = Some(
                            self.session_app()
                                .snapshot_prompt_authority_for_harness(harness_id, ctx.workspace),
                        );
                    }
                }
                let result = action(harness, ctx);
                let ready_after = harness
                    .definition
                    .integration
                    .as_ref()
                    .is_some_and(|binding| {
                        crate::harness::integration::prompt_attention_hook_ready(binding, ctx)
                    });
                if !ready_after && snapshot.is_none() {
                    if let Some(harness_id) =
                        prompt_harness_for_integration_adapter(&key.adapter_id)
                    {
                        snapshot = Some(
                            self.session_app()
                                .snapshot_prompt_authority_for_harness(harness_id, ctx.workspace),
                        );
                    }
                }
                Ok((group.clone(), result))
            })
            .await;
        let response = match result {
            Ok((group, Ok(status))) => Ok(GroupedIntegrationStatus {
                key: key.clone(),
                consumers: group.consumers,
                status,
            }),
            Ok((group, Err(error))) => Ok(GroupedIntegrationStatus {
                key: key.clone(),
                status: grouped_integration_error_status(&group, &error, failure_action),
                consumers: group.consumers,
            }),
            Err(error) => Err(error),
        };
        self.finalize_snapshot(snapshot)?;
        response
    }

    async fn with_revalidated_key<R>(
        &self,
        key: &IntegrationKey,
        expected: Option<&IntegrationRevisionExpectation>,
        action: impl FnOnce(
            &ResolvedIntegrationGroup,
            &ResolvedHarness,
            &IntegrationContext<'_>,
        ) -> Result<R, IntegrationError>,
    ) -> Result<R, IntegrationApplicationError> {
        let workspace_path_at_start = self
            .state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .map(|ws| ws.path.clone())
            .ok_or(IntegrationApplicationError::NoWorkspace)?;
        if expected
            .and_then(|e| e.workspace_path.as_deref())
            .is_some_and(|p| p != workspace_path_at_start)
        {
            return Err(self.revision_conflict());
        }
        if binding_for_key(key).is_none() {
            return Err(IntegrationApplicationError::UnknownIntegration(key.clone()));
        }
        let initial_document_revision = self.document_revision_snapshot()?;
        let (initial_active_ids, initial_active_revision) = self.active_workspace_snapshot()?;
        if expected.is_some_and(|e| {
            e.document_revision != initial_document_revision
                || e.active_harness_revision != initial_active_revision
        }) {
            return Err(self.revision_conflict());
        }
        let group = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned")
            .integration_group_for_key(key, &initial_active_ids)
            .ok_or_else(|| IntegrationApplicationError::UnknownIntegration(key.clone()))?;
        let harness = group.representative.clone();
        let enabled = !group.consumers.is_empty();
        let detected = crate::harness::detect::resolve_tool_gate(
            &self.state.integration_probe_cache,
            &harness.definition.id,
            &harness.launch_command(),
            harness.definition.min_version.as_ref(),
        )
        .await;
        #[cfg(test)]
        if let Some(hook) = &self.probe_revalidation_hook {
            hook(&harness.definition.id);
        }
        let _projection = self
            .state
            .projection_lock
            .lock()
            .expect("projection lock poisoned");
        let current_document_revision = self.document_revision_snapshot()?;
        let (current_active_ids, current_active_revision) = self.active_workspace_snapshot()?;
        if current_document_revision != initial_document_revision
            || current_active_revision != initial_active_revision
            || expected.is_some_and(|e| {
                e.document_revision != current_document_revision
                    || e.active_harness_revision != current_active_revision
            })
        {
            return Err(self.revision_conflict());
        }
        let registry = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned");
        match registry.integration_group_for_key(key, &current_active_ids) {
            Some(current) if current.representative.definition == harness.definition => {}
            _ => return Err(self.revision_conflict()),
        }
        drop(registry);
        let workspace_guard = self.state.workspace.lock().unwrap();
        let workspace = workspace_guard
            .as_ref()
            .ok_or(IntegrationApplicationError::NoWorkspace)?;
        if workspace.path != workspace_path_at_start
            || expected
                .and_then(|e| e.workspace_path.as_deref())
                .is_some_and(|p| p != workspace.path)
        {
            return Err(IntegrationApplicationError::WorkspaceChanged);
        }
        let reporter_assets =
            reporter_assets().map_err(IntegrationApplicationError::Infrastructure)?;
        let home = dirs::home_dir().ok_or_else(|| {
            IntegrationApplicationError::Infrastructure("couldn't resolve home directory".into())
        })?;
        let ctx = IntegrationContext {
            workspace: &workspace.path,
            workspace_metadata: Some(&workspace.metadata),
            orkworks_root: &home.join(".orkworks"),
            enabled,
            detected_tool: detected.as_ref(),
            reporter_assets: &reporter_assets,
        };
        action(&group, &harness, &ctx).map_err(IntegrationApplicationError::Configuration)
    }

    fn active_workspace_snapshot(&self) -> Result<(Vec<String>, u64), IntegrationApplicationError> {
        let guard = self.state.workspace.lock().unwrap();
        let workspace = guard
            .as_ref()
            .ok_or(IntegrationApplicationError::NoWorkspace)?;
        let memory = workspace
            .metadata
            .read_workspace_memory()
            .unwrap_or_default();
        Ok((memory.active_harness_ids, memory.active_harness_revision))
    }

    fn document_revision_snapshot(
        &self,
    ) -> Result<Option<HarnessDocumentRevision>, IntegrationApplicationError> {
        self.state
            .harness_store
            .snapshot()
            .map(|snapshot| snapshot.document_revision)
            .map_err(|_| {
                IntegrationApplicationError::Infrastructure(
                    "couldn't load harness configuration".into(),
                )
            })
    }

    fn revision_conflict(&self) -> IntegrationApplicationError {
        IntegrationApplicationError::RevisionConflict(IntegrationRevisionConflict {
            document_revision: self.document_revision_snapshot().unwrap_or(None),
            active_harness_revision: self
                .active_workspace_snapshot()
                .map(|(_, revision)| revision)
                .unwrap_or_default(),
        })
    }

    async fn list_workspace_inner(
        &self,
    ) -> Result<Vec<GroupedIntegrationStatus>, IntegrationApplicationError> {
        let active_ids = {
            let guard = self.state.workspace.lock().unwrap();
            let workspace = guard
                .as_ref()
                .ok_or(IntegrationApplicationError::NoWorkspace)?;
            workspace
                .metadata
                .read_workspace_memory()
                .map(|memory| memory.active_harness_ids)
                .unwrap_or_default()
        };
        let groups = self
            .state
            .harness_catalog
            .read()
            .expect("harness catalog lock poisoned")
            .integration_groups(&active_ids)
            .map_err(IntegrationApplicationError::Configuration)?;
        let mut result = Vec::with_capacity(groups.len());
        let mut snapshots = Vec::new();
        let mut revalidation_error = None;
        for (group_index, group) in groups.into_iter().enumerate() {
            #[cfg(not(test))]
            let _ = group_index;
            #[cfg(test)]
            if let Some(hook) = &self.group_revalidation_hook {
                hook(group_index);
            }
            let key = group.key.clone();
            let mut snapshot = None;
            let operation = self
                .with_revalidated_key(&key, None, |group, harness, ctx| {
                    let status = harness.integration_status(ctx);
                    let ready = harness
                        .definition
                        .integration
                        .as_ref()
                        .is_some_and(|binding| {
                            crate::harness::integration::prompt_attention_hook_ready(binding, ctx)
                        });
                    if !ready {
                        if let Some(harness_id) =
                            prompt_harness_for_integration_adapter(&key.adapter_id)
                        {
                            snapshot =
                                Some(self.session_app().snapshot_prompt_authority_for_harness(
                                    harness_id,
                                    ctx.workspace,
                                ));
                        }
                    }
                    Ok((group.clone(), status))
                })
                .await;
            let (group, status) = match operation {
                Ok(pair) => pair,
                Err(error) => {
                    revalidation_error = Some(error);
                    break;
                }
            };
            let status = status
                .unwrap_or_else(|error| grouped_integration_error_status(&group, &error, "retry"));
            if let Some(snapshot) = snapshot {
                snapshots.push(snapshot);
            }
            result.push(GroupedIntegrationStatus {
                key,
                consumers: group.consumers,
                status,
            });
        }
        let mut revocation_failed = false;
        for snapshot in snapshots {
            if self.revoke_snapshot(snapshot).is_err() {
                revocation_failed = true;
            }
        }
        if revocation_failed {
            return Err(IntegrationApplicationError::AuthorityRevocation);
        }
        if let Some(error) = revalidation_error {
            return Err(error);
        }
        Ok(result)
    }

    async fn reconcile_unreferenced_inner(
        &self,
        keys: BTreeSet<IntegrationKey>,
        expected_workspace_path: Option<PathBuf>,
    ) -> IntegrationCleanupResponse {
        let expected = match (
            self.document_revision_snapshot(),
            self.active_workspace_snapshot(),
        ) {
            (Ok(document_revision), Ok((_, active_harness_revision))) => {
                Some(IntegrationRevisionExpectation {
                    document_revision,
                    active_harness_revision,
                    workspace_path: expected_workspace_path,
                })
            }
            _ => None,
        };
        let mut outcomes = Vec::new();
        let mut errors = Vec::new();
        for key in keys {
            let result = self
                .with_revalidated_key(&key, expected.as_ref(), |group, harness, ctx| {
                    let status = if ctx.enabled {
                        harness.integration_status(ctx)
                    } else {
                        let status = harness.integration_status(ctx)?;
                        if status.registration == IntegrationRegistration::Absent {
                            Ok(status)
                        } else if status.registration == IntegrationRegistration::Error
                            || status.ownership == IntegrationOwnership::Ambiguous
                        {
                            Ok(cleanup_needed_status(status))
                        } else {
                            harness.integration_uninstall(ctx)
                        }
                    };
                    Ok((group.clone(), status))
                })
                .await;
            match result {
                Ok((group, Ok(status))) => outcomes.push(GroupedIntegrationStatus {
                    key,
                    consumers: group.consumers,
                    status,
                }),
                Ok((group, Err(error))) => {
                    let action = if group.consumers.is_empty() {
                        "cleanup-needed"
                    } else {
                        "retry"
                    };
                    let status = grouped_integration_error_status(&group, &error, action);
                    outcomes.push(GroupedIntegrationStatus {
                        key,
                        consumers: group.consumers,
                        status,
                    });
                }
                Err(_) => errors.push(format!(
                    "Could not reconcile integration {}/{}; retry cleanup.",
                    key.adapter_id, key.target_id
                )),
            }
        }
        let status = if !errors.is_empty()
            || outcomes.iter().any(|outcome| {
                outcome
                    .status
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.action.as_deref() == Some("cleanup-needed"))
            }) {
            "cleanup-needed"
        } else {
            "complete"
        };
        IntegrationCleanupResponse {
            status,
            outcomes,
            errors,
        }
    }
}

#[cfg(test)]
#[path = "harness_integration_application_tests.rs"]
mod tests;

fn apply_mutation(
    harness: &ResolvedHarness,
    ctx: &IntegrationContext<'_>,
    mutation: IntegrationMutation,
) -> Result<IntegrationStatus, IntegrationError> {
    match mutation {
        IntegrationMutation::Install | IntegrationMutation::Repair => {
            harness.integration_install(ctx)
        }
        IntegrationMutation::Uninstall => harness.integration_uninstall(ctx),
    }
}

fn resolve_scripts_source_dir(exe_dir: Option<PathBuf>, manifest_dir: &Path) -> PathBuf {
    if let Some(dir) = exe_dir {
        let packaged = dir.join("scripts");
        if packaged.is_dir() {
            return packaged;
        }
    }
    manifest_dir.join("scripts")
}

fn scripts_source_dir() -> PathBuf {
    resolve_scripts_source_dir(
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf)),
        Path::new(env!("CARGO_MANIFEST_DIR")),
    )
}

fn stable_hook_scripts_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".orkworks").join("hook-scripts"))
}

pub(crate) fn reporter_assets() -> Result<ReporterAssetResolver, String> {
    let stable_dir = stable_hook_scripts_dir()
        .ok_or_else(|| "couldn't resolve home directory for the reporter scripts".to_string())?;
    Ok(ReporterAssetResolver {
        source_dir: scripts_source_dir(),
        stable_dir,
    })
}

fn workspace_harness_enabled(workspace: &WorkspaceState, harness_id: &str) -> bool {
    workspace
        .metadata
        .read_workspace_memory()
        .is_some_and(|memory| memory.active_harness_ids.iter().any(|id| id == harness_id))
}

fn prompt_harness_for_integration_adapter(adapter_id: &str) -> Option<&'static str> {
    match adapter_id {
        "claude" => Some("claude-code"),
        "copilot" => Some("copilot"),
        _ => None,
    }
}

fn prompt_attention_launch_ready(state: &AppState, harness_id: &str, executable: &str) -> bool {
    let workspace_guard = state.workspace.lock().unwrap();
    let Some(workspace) = workspace_guard.as_ref() else {
        return false;
    };
    let registry = state
        .harness_catalog
        .read()
        .expect("harness catalog lock poisoned");
    let Some(harness) = registry.get(harness_id) else {
        return false;
    };
    let Some(binding) = harness.definition.integration.as_ref() else {
        return false;
    };
    let Some(home) = dirs::home_dir() else {
        return false;
    };
    let Ok(reporter_assets) = reporter_assets() else {
        return false;
    };
    let detected_tool = crate::harness::integration::DetectedTool {
        executable: PathBuf::from(executable),
        version: None,
        compatible: true,
    };
    let ctx = IntegrationContext {
        workspace: &workspace.path,
        workspace_metadata: Some(&workspace.metadata),
        orkworks_root: &home.join(".orkworks"),
        enabled: workspace_harness_enabled(workspace, harness_id),
        detected_tool: Some(&detected_tool),
        reporter_assets: &reporter_assets,
    };
    crate::harness::integration::prompt_attention_hook_ready(binding, &ctx)
}

fn grouped_integration_error_status(
    group: &ResolvedIntegrationGroup,
    error: &IntegrationError,
    action: &'static str,
) -> IntegrationStatus {
    let enabled = !group.consumers.is_empty();
    IntegrationStatus {
        harness_id: group.representative.definition.id.clone(),
        enabled,
        tool_detected: false,
        registration: IntegrationRegistration::Error,
        ownership: IntegrationOwnership::None,
        activation: if enabled {
            crate::harness::integration::IntegrationActivation::Unknown
        } else {
            crate::harness::integration::IntegrationActivation::Disabled
        },
        coverage: crate::harness::integration::IntegrationCoverage::None,
        diagnostics: vec![crate::harness::integration::IntegrationDiagnostic {
            code: error.code().into(),
            message: error.to_string(),
            action: Some(action.into()),
        }],
        confirmation: None,
    }
}

fn cleanup_needed_status(mut status: IntegrationStatus) -> IntegrationStatus {
    if status.diagnostics.is_empty() {
        status
            .diagnostics
            .push(crate::harness::integration::IntegrationDiagnostic {
                code: "cleanup_needed".into(),
                message: "This integration needs manual cleanup before it can be reconciled."
                    .into(),
                action: Some("cleanup-needed".into()),
            });
    } else {
        for diagnostic in &mut status.diagnostics {
            diagnostic.action = Some("cleanup-needed".into());
        }
    }
    status
}
