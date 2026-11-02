//! Owned Codex Settings actions. Admission is the existing validated cutoff;
//! no application guard escapes it into the filesystem operation.
use super::*;
use crate::harness::definition::IntegrationBinding;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Inspect,
    Mutate(IntegrationMutation),
    Cleanup,
}
impl Action {
    pub(super) fn mutates(self) -> bool {
        !matches!(self, Self::Inspect)
    }
}
pub(super) fn apply(
    harness: &ResolvedHarness,
    ctx: &IntegrationContext<'_>,
    action: Action,
) -> Result<IntegrationStatus, IntegrationError> {
    match action {
        Action::Inspect => harness.integration_status(ctx),
        Action::Mutate(mutation) => apply_mutation(harness, ctx, mutation),
        Action::Cleanup => {
            let status = harness.integration_status(ctx)?;
            if ctx.enabled || status.registration == IntegrationRegistration::Absent {
                Ok(status)
            } else if status.registration == IntegrationRegistration::Error
                || status.ownership == IntegrationOwnership::Ambiguous
            {
                Ok(cleanup_needed_status(status))
            } else {
                harness.integration_uninstall(ctx)
            }
        }
    }
}

pub(super) enum Execution {
    Completed(Result<IntegrationStatus, IntegrationError>),
    Codex(OwnedAction),
}
impl Execution {
    pub(super) async fn finish(
        self,
    ) -> Result<Result<IntegrationStatus, IntegrationError>, IntegrationApplicationError> {
        match self {
            Self::Completed(result) => Ok(result),
            Self::Codex(action) => tokio::task::spawn_blocking(move || action.run())
                .await
                .map_err(|_| {
                    IntegrationApplicationError::Infrastructure(
                        "integration action did not return a definitive result".into(),
                    )
                }),
        }
    }
}
/// Detect replacement even when a workspace is reopened at the same path.
pub(super) struct Origin {
    path: PathBuf,
    root: PathBuf,
    lease: Option<Arc<crate::workspace_runtime::WorkspaceLease>>,
    observations: Arc<crate::metadata::hook_observation::Store>,
}
impl Origin {
    pub(super) fn capture(ws: &WorkspaceState) -> Self {
        Self {
            path: ws.path.clone(),
            root: ws.metadata.root_path(),
            lease: ws.lease.clone(),
            observations: ws.metadata.hook_observation_owner(),
        }
    }
    pub(super) fn matches(&self, ws: &WorkspaceState) -> bool {
        self.path == ws.path
            && self.root == ws.metadata.root_path()
            && Arc::ptr_eq(&self.observations, &ws.metadata.hook_observation_owner())
            && match (&self.lease, &ws.lease) {
                (Some(before), Some(now)) => Arc::ptr_eq(before, now),
                (None, None) => true,
                _ => false,
            }
    }
}

pub(super) fn is_codex(harness: &ResolvedHarness) -> bool {
    matches!(
        harness.definition.integration,
        Some(IntegrationBinding::Codex)
    )
}
pub(super) struct OwnedAction {
    harness: ResolvedHarness,
    workspace: PathBuf,
    metadata: crate::metadata::IntegrationMetadata,
    orkworks_root: PathBuf,
    enabled: bool,
    detected: Option<crate::harness::integration::DetectedTool>,
    assets: ReporterAssetResolver,
    action: Action,
    _lease: Arc<crate::workspace_runtime::WorkspaceLease>,
    _claim: crate::metadata::hook_observation::IntegrationClaim,
}
impl OwnedAction {
    pub(super) fn capture(
        harness: &ResolvedHarness,
        workspace: &WorkspaceState,
        ctx: &IntegrationContext<'_>,
        action: Action,
    ) -> Result<Self, IntegrationError> {
        let lease = workspace
            .lease
            .clone()
            .ok_or(IntegrationError::RevisionChanged)?;
        let metadata = ctx
            .workspace_metadata
            .clone()
            .ok_or(IntegrationError::NoWorkspace)?;
        let claim = metadata
            .claim_codex_integration()
            .ok_or(IntegrationError::RevisionChanged)?;
        Ok(Self {
            harness: harness.clone(),
            workspace: ctx.workspace.to_owned(),
            metadata,
            orkworks_root: ctx.orkworks_root.to_owned(),
            enabled: ctx.enabled,
            detected: ctx.detected_tool.cloned(),
            assets: ctx.reporter_assets.clone(),
            action,
            _lease: lease,
            _claim: claim,
        })
    }
    fn run(self) -> Result<IntegrationStatus, IntegrationError> {
        let ctx = IntegrationContext {
            workspace: &self.workspace,
            workspace_metadata: Some(self.metadata.clone()),
            orkworks_root: &self.orkworks_root,
            enabled: self.enabled,
            detected_tool: self.detected.as_ref(),
            reporter_assets: &self.assets,
        };
        apply(&self.harness, &ctx, self.action)
    }
}
