use std::path::Path;

use serde_json::{Map, Value};

use super::{reporter_invocation, ReporterPlatform};
use crate::harness::integration::{
    ConfigFileTransaction, IntegrationActivation, IntegrationConfirmation, IntegrationContext,
    IntegrationCoverage, IntegrationDiagnostic, IntegrationError, IntegrationHandler,
    IntegrationOwnership, IntegrationRegistration, IntegrationStatus, ValidatedWorkspaceTarget,
};

const PREFERENCE_PATH: &str = "integrations/aider.json";
const MARKER: &str = "orkworks:harness-integration:v2:aider";

pub(crate) struct AiderHandler;
pub(crate) static HANDLER: AiderHandler = AiderHandler;

impl IntegrationHandler for AiderHandler {
    fn status(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        Ok(status(ctx))
    }

    fn install(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        let reporter = ctx
            .reporter_assets
            .reconcile(ReporterPlatform::current().asset_name())?;
        write_preference(ctx, true)?;
        let _ = reporter;
        Ok(status(ctx))
    }

    fn uninstall(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationStatus, IntegrationError> {
        write_preference(ctx, false)?;
        Ok(status(ctx))
    }

    fn augment_launch(
        &self,
        command: &mut crate::harness::CommandSpec,
        enabled: bool,
        reporter: Option<&Path>,
    ) -> Result<(), IntegrationError> {
        if !enabled {
            return Ok(());
        }
        let Some(reporter) = reporter else {
            return Ok(());
        };
        let expected = reporter_invocation(reporter, MARKER).shell_command;
        if command
            .args
            .windows(2)
            .any(|pair| pair[0] == "--notifications-command" && pair[1] == expected)
        {
            return Ok(());
        }
        if command
            .args
            .windows(2)
            .any(|pair| pair[0] == "--notifications-command")
        {
            return Err(IntegrationError::LaunchConflict);
        }
        command.args.push("--notifications-command".into());
        command.args.push(expected);
        Ok(())
    }

    fn launch_enabled(&self, metadata_root: Option<&Path>) -> Result<bool, IntegrationError> {
        metadata_root.map_or(Ok(false), launch_enabled)
    }
}

pub(crate) fn launch_enabled(metadata_root: &Path) -> Result<bool, IntegrationError> {
    read_preference(metadata_root).map(|(_, _, enabled)| enabled)
}

fn status(ctx: &IntegrationContext<'_>) -> IntegrationStatus {
    let preference = ctx
        .workspace_metadata
        .ok_or(IntegrationError::NoWorkspace)
        .and_then(|store| read_preference(&store.root_path()));
    let (registration, ownership, configured, diagnostics) = match preference {
        Ok((_, _, true)) => (
            IntegrationRegistration::Installed,
            IntegrationOwnership::OrkWorks,
            true,
            vec![],
        ),
        Ok((_, _, false)) => (
            IntegrationRegistration::Absent,
            IntegrationOwnership::None,
            false,
            vec![],
        ),
        Err(error) => (
            IntegrationRegistration::Error,
            IntegrationOwnership::None,
            false,
            vec![IntegrationDiagnostic {
                code: error.code().into(),
                message: "The Aider integration preference is malformed or unavailable and was not changed."
                    .into(),
                action: None,
            }],
        ),
    };
    let activation = if !ctx.enabled {
        IntegrationActivation::Disabled
    } else if !ctx.detected_tool.is_some_and(|tool| tool.compatible) {
        IntegrationActivation::Unknown
    } else if configured {
        IntegrationActivation::Active
    } else {
        IntegrationActivation::Disabled
    };
    let confirmation = matches!(
        registration,
        IntegrationRegistration::Absent | IntegrationRegistration::Installed
    )
    .then(|| {
        IntegrationConfirmation::new(
            "Aider",
            ctx.workspace,
            "Limited attention notifications",
            &[],
            false,
        )
        .ok()
    })
    .flatten();
    IntegrationStatus {
        harness_id: "aider".into(),
        enabled: ctx.enabled,
        tool_detected: ctx.detected_tool.is_some(),
        registration,
        ownership,
        activation,
        coverage: IntegrationCoverage::Limited,
        diagnostics: {
            let mut diagnostics = diagnostics;
            diagnostics.push(IntegrationDiagnostic {
                code: "no_native_session_id".into(),
                message: "Aider notifications report attention only; Aider has no native session ID contract.".into(),
                action: None,
            });
            diagnostics
        },
        confirmation,
    }
}

fn read_preference(
    root: &Path,
) -> Result<(ConfigFileTransaction, Map<String, Value>, bool), IntegrationError> {
    let target = ValidatedWorkspaceTarget::new(root, Path::new(PREFERENCE_PATH))?;
    let transaction = ConfigFileTransaction::open(target)?;
    let document = if transaction.current_bytes().is_empty() {
        Map::new()
    } else {
        crate::harness::integration::parse_json_object(transaction.current_bytes())?
    };
    let enabled = match document.get("version") {
        None => false,
        Some(version) if version == &Value::from(1) => {
            document.get("enabled").map_or(Ok(false), |enabled| {
                enabled.as_bool().ok_or_else(|| {
                    IntegrationError::InvalidConfig(
                        "Aider integration enabled must be a boolean.".into(),
                    )
                })
            })?
        }
        Some(_) => {
            return Err(IntegrationError::InvalidConfig(
                "Unsupported Aider integration preference version.".into(),
            ))
        }
    };
    Ok((transaction, document, enabled))
}

fn write_preference(ctx: &IntegrationContext<'_>, enabled: bool) -> Result<(), IntegrationError> {
    let metadata = ctx
        .workspace_metadata
        .ok_or(IntegrationError::NoWorkspace)?;
    let (transaction, mut document, _) = read_preference(&metadata.root_path())?;
    document.insert("version".into(), Value::from(1));
    document.insert("enabled".into(), Value::Bool(enabled));
    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| IntegrationError::InvalidConfig(error.to_string()))?;
    transaction.commit(&bytes)
}
