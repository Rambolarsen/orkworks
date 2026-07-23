use std::path::Path;

use super::REPORTER_ASSET;
use crate::harness::integration::{
    IntegrationActivation, IntegrationConfirmation, IntegrationContext, IntegrationCoverage,
    IntegrationDiagnostic, IntegrationError, IntegrationHandler, IntegrationOwnership,
    IntegrationRegistration, IntegrationStatus,
};

pub(crate) struct AiderHandler;
pub(crate) static HANDLER: AiderHandler = AiderHandler;

impl IntegrationHandler for AiderHandler {
    fn status(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        Ok(status(ctx))
    }

    fn install(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        let metadata = ctx
            .workspace_metadata
            .ok_or(IntegrationError::NoWorkspace)?;
        // Reconcile the code-owned reporter before declaring that the launch
        // augmentation is installed. The reporter itself is supplied by Task 7.
        ctx.reporter_assets.reconcile(REPORTER_ASSET)?;
        metadata.set_aider_notifications(true)?;
        Ok(status(ctx))
    }

    fn uninstall(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationStatus, IntegrationError> {
        let metadata = ctx
            .workspace_metadata
            .ok_or(IntegrationError::NoWorkspace)?;
        metadata.set_aider_notifications(false)?;
        Ok(status(ctx))
    }

    fn augment_launch(
        &self,
        command: &mut crate::harness::CommandSpec,
        enabled: bool,
        reporter: Option<&Path>,
    ) {
        if !enabled
            || command.args.windows(2).any(|pair| {
                pair[0] == "--notifications-command"
                    && reporter.is_some_and(|path| pair[1] == path.to_string_lossy())
            })
        {
            return;
        }
        let Some(reporter) = reporter else {
            return;
        };
        command.args.push("--notifications-command".into());
        command.args.push(reporter.to_string_lossy().into_owned());
    }
}

fn status(ctx: &IntegrationContext<'_>) -> IntegrationStatus {
    let enabled = ctx
        .workspace_metadata
        .and_then(|store| store.read_workspace_memory())
        .and_then(|memory| memory.aider_notifications)
        .is_some_and(|preference| preference.version == 1 && preference.enabled);
    IntegrationStatus {
        harness_id: "aider".into(),
        enabled,
        tool_detected: ctx.detected_tool.is_some(),
        registration: if enabled { IntegrationRegistration::Installed } else { IntegrationRegistration::Absent },
        ownership: if enabled { IntegrationOwnership::OrkWorks } else { IntegrationOwnership::None },
        activation: if enabled { IntegrationActivation::Active } else { IntegrationActivation::Disabled },
        coverage: IntegrationCoverage::Limited,
        diagnostics: vec![IntegrationDiagnostic {
            code: "no_native_session_id".into(),
            message: "Aider notifications report attention only; Aider has no native session ID contract."
                .into(),
            action: None,
        }],
        confirmation: IntegrationConfirmation::new(
            "Aider",
            ctx.workspace,
            "Limited attention notifications",
            &[Path::new("workspace.json")],
            false,
        )
        .ok(),
    }
}
