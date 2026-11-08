use std::path::Path;

use serde_json::{Map, Value};

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
        write_preference(ctx, true)?;
        Ok(status(ctx))
    }

    fn uninstall(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationStatus, IntegrationError> {
        write_preference(ctx, false)?;
        Ok(status(ctx))
    }

    // Aider's completion notification cannot establish prompt authority. Use
    // the default no-op launch augmentation so Aider retains its own command.
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
        .as_ref()
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
            "Peon attention fallback",
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
                message: "Aider completion notifications do not identify prompts. Peon and terminal input provide attention fallback; Aider keeps its own notifications. No native session ID is available.".into(),
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
            if document.get("marker").and_then(Value::as_str) != Some(MARKER) {
                return Err(IntegrationError::InvalidConfig(
                    "Aider integration preference marker is missing or does not match.".into(),
                ));
            }
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
        .as_ref()
        .ok_or(IntegrationError::NoWorkspace)?;
    let (transaction, mut document, _) = read_preference(&metadata.root_path())?;
    if !enabled && transaction.current_bytes().is_empty() {
        return Ok(());
    }
    document.insert("version".into(), Value::from(1));
    document.insert("marker".into(), Value::String(MARKER.into()));
    document.insert("enabled".into(), Value::Bool(enabled));
    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| IntegrationError::InvalidConfig(error.to_string()))?;
    transaction.commit(&bytes)
}
