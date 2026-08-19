use std::path::Path;

use super::FragmentState;
use crate::harness::integration::{
    ConfigFileTransaction, IntegrationActivation, IntegrationConfirmation, IntegrationContext,
    IntegrationCoverage, IntegrationDiagnostic, IntegrationError, IntegrationHandler,
    IntegrationOwnership, IntegrationRegistration, IntegrationStatus, ValidatedWorkspaceTarget,
};

const RELATIVE_PATH: &str = ".opencode/plugins/orkworks-session-reporter.js";
const MARKER_LINE: &str = "// orkworks:harness-integration:v2:opencode";
const PLUGIN_SOURCE: &str = include_str!("../../../scripts/opencode-session-reporter.js");

pub(crate) struct OpenCodeHandler;
pub(crate) static HANDLER: OpenCodeHandler = OpenCodeHandler;

impl IntegrationHandler for OpenCodeHandler {
    fn status(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        match load(ctx) {
            Ok((_, bytes)) => Ok(status_from_bytes(ctx, &bytes)),
            Err(error) => Ok(error_status(ctx, &error)),
        }
    }

    fn install(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        let (transaction, bytes) = load(ctx)?;
        match content_state(&bytes) {
            FragmentState::Installed => return Ok(status_from_bytes(ctx, &bytes)),
            FragmentState::Ambiguous => return Err(IntegrationError::OwnershipAmbiguous),
            FragmentState::Absent | FragmentState::Drifted => {}
        }
        transaction.commit(PLUGIN_SOURCE.as_bytes())?;
        Ok(status_from_bytes(ctx, PLUGIN_SOURCE.as_bytes()))
    }

    fn uninstall(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationStatus, IntegrationError> {
        let (transaction, bytes) = load(ctx)?;
        match content_state(&bytes) {
            FragmentState::Absent => return Ok(status_from_bytes(ctx, &[])),
            FragmentState::Ambiguous => return Err(IntegrationError::OwnershipAmbiguous),
            FragmentState::Installed | FragmentState::Drifted => {}
        }
        transaction.commit_removal()?;
        Ok(status_from_bytes(ctx, &[]))
    }
}

/// Reads the plugin file's current bytes through the same confinement and
/// git-safety checks every other workspace integration uses. Unlike the
/// JSON handlers, there is no shared document to merge into: the plugin
/// file's entire byte content is OrkWorks-owned, so [`content_state`]
/// classifies ownership from the whole file rather than a JSON marker field.
fn load(
    ctx: &IntegrationContext<'_>,
) -> Result<(ConfigFileTransaction, Vec<u8>), IntegrationError> {
    let target = ValidatedWorkspaceTarget::new(ctx.workspace, Path::new(RELATIVE_PATH))?;
    target.require_local_or_ignored_untracked()?;
    let transaction = ConfigFileTransaction::open(target)?;
    let bytes = transaction.current_bytes().to_vec();
    Ok((transaction, bytes))
}

/// Classifies the plugin file by exact byte match against [`PLUGIN_SOURCE`],
/// falling back to a first-line marker comment so a byte-for-byte drift
/// (e.g. content installed by an older OrkWorks version) is still
/// recognized as OrkWorks-owned and safe to reconcile. A present file
/// without the marker belongs to something else and must not be touched.
fn content_state(bytes: &[u8]) -> FragmentState {
    if bytes.is_empty() {
        return FragmentState::Absent;
    }
    if bytes == PLUGIN_SOURCE.as_bytes() {
        return FragmentState::Installed;
    }
    let first_line = bytes.split(|&byte| byte == b'\n').next().unwrap_or(&[]);
    // Strip a trailing \r so a CRLF-written drift (e.g. an older OrkWorks
    // version writing on Windows) still matches the marker instead of being
    // classified Ambiguous and refused reconciliation.
    let first_line = first_line.strip_suffix(b"\r").unwrap_or(first_line);
    if first_line == MARKER_LINE.as_bytes() {
        FragmentState::Drifted
    } else {
        FragmentState::Ambiguous
    }
}

fn status_from_bytes(ctx: &IntegrationContext<'_>, bytes: &[u8]) -> IntegrationStatus {
    let (registration, ownership, mut diagnostics) = match content_state(bytes) {
        FragmentState::Absent => (
            IntegrationRegistration::Absent,
            IntegrationOwnership::None,
            vec![],
        ),
        FragmentState::Installed => (
            IntegrationRegistration::Installed,
            IntegrationOwnership::OrkWorks,
            vec![],
        ),
        FragmentState::Drifted => (
            IntegrationRegistration::Drifted,
            IntegrationOwnership::OrkWorks,
            vec![IntegrationDiagnostic {
                code: "owned_fragment_drifted".into(),
                message: "The OrkWorks-installed OpenCode session reporter plugin differs from the supported shape.".into(),
                action: Some("reconcile".into()),
            }],
        ),
        FragmentState::Ambiguous => (
            IntegrationRegistration::Drifted,
            IntegrationOwnership::Ambiguous,
            vec![IntegrationDiagnostic {
                code: "ownership_ambiguous".into(),
                message: "A plugin file already occupies this path and was not installed by OrkWorks.".into(),
                action: None,
            }],
        ),
    };
    let activation = if !ctx.enabled {
        IntegrationActivation::Disabled
    } else if registration == IntegrationRegistration::Drifted
        || ownership == IntegrationOwnership::Ambiguous
    {
        IntegrationActivation::Unknown
    } else if ctx.detected_tool.is_none() {
        diagnostics.push(IntegrationDiagnostic {
            code: "tool_not_detected".into(),
            message: "OpenCode was not detected, so integration activation is unknown.".into(),
            action: None,
        });
        IntegrationActivation::Unknown
    } else if ctx.detected_tool.is_some_and(|tool| !tool.compatible) {
        diagnostics.push(IntegrationDiagnostic {
            code: "unsupported_tool_version".into(),
            message: "The detected OpenCode version is not eligible for this integration.".into(),
            action: None,
        });
        IntegrationActivation::NeedsTrust
    } else if registration == IntegrationRegistration::Installed {
        IntegrationActivation::Active
    } else {
        IntegrationActivation::Disabled
    };
    let confirmation = if matches!(ownership, IntegrationOwnership::Ambiguous) {
        None
    } else {
        IntegrationConfirmation::new(
            "OpenCode",
            ctx.workspace,
            "Native session ID capture",
            &[Path::new(RELATIVE_PATH)],
            false,
        )
        .ok()
    };
    IntegrationStatus {
        harness_id: "opencode".into(),
        enabled: ctx.enabled,
        tool_detected: ctx.detected_tool.is_some(),
        registration,
        ownership,
        activation,
        coverage: IntegrationCoverage::Limited,
        diagnostics,
        confirmation,
    }
}

fn error_status(ctx: &IntegrationContext<'_>, error: &IntegrationError) -> IntegrationStatus {
    IntegrationStatus {
        harness_id: "opencode".into(),
        enabled: ctx.enabled,
        tool_detected: ctx.detected_tool.is_some(),
        registration: IntegrationRegistration::Error,
        ownership: IntegrationOwnership::None,
        activation: IntegrationActivation::Unknown,
        coverage: IntegrationCoverage::Limited,
        diagnostics: vec![IntegrationDiagnostic {
            code: error.code().into(),
            message: error.to_string(),
            action: None,
        }],
        confirmation: None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::harness::integration::ReporterAssetResolver;

    fn context<'a>(workspace: &'a Path, resolver: &'a ReporterAssetResolver) -> IntegrationContext<'a> {
        IntegrationContext {
            workspace,
            workspace_metadata: None,
            orkworks_root: workspace,
            enabled: true,
            detected_tool: None,
            reporter_assets: resolver,
        }
    }

    fn gitignored_workspace() -> tempfile::TempDir {
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        fs::write(
            workspace.path().join(".gitignore"),
            ".opencode/plugins/orkworks-session-reporter.js\n",
        )
        .unwrap();
        workspace
    }

    fn resolver(workspace: &Path) -> ReporterAssetResolver {
        ReporterAssetResolver {
            source_dir: workspace.join("unused-source"),
            stable_dir: workspace.join("unused-stable"),
        }
    }

    #[test]
    fn install_writes_the_plugin_and_is_idempotent() {
        let workspace = gitignored_workspace();
        let resolver = resolver(workspace.path());
        let ctx = context(workspace.path(), &resolver);

        let installed = HANDLER.install(&ctx).unwrap();
        assert_eq!(installed.registration, IntegrationRegistration::Installed);
        let path = workspace.path().join(RELATIVE_PATH);
        assert_eq!(fs::read_to_string(&path).unwrap(), PLUGIN_SOURCE);

        // Idempotent: a second install with the file already in place must
        // not error and must report Installed without rewriting anything.
        let reinstalled = HANDLER.install(&ctx).unwrap();
        assert_eq!(reinstalled.registration, IntegrationRegistration::Installed);
    }

    #[test]
    fn status_reports_absent_before_install() {
        let workspace = gitignored_workspace();
        let resolver = resolver(workspace.path());
        let ctx = context(workspace.path(), &resolver);

        let status = HANDLER.status(&ctx).unwrap();

        assert_eq!(status.registration, IntegrationRegistration::Absent);
        assert_eq!(status.ownership, IntegrationOwnership::None);
    }

    #[test]
    fn install_then_uninstall_removes_the_plugin_file() {
        let workspace = gitignored_workspace();
        let resolver = resolver(workspace.path());
        let ctx = context(workspace.path(), &resolver);
        HANDLER.install(&ctx).unwrap();
        let path = workspace.path().join(RELATIVE_PATH);
        assert!(path.exists());

        let uninstalled = HANDLER.uninstall(&ctx).unwrap();

        assert_eq!(uninstalled.registration, IntegrationRegistration::Absent);
        assert!(!path.exists());

        // Idempotent: uninstalling an already-absent target succeeds.
        let uninstalled_again = HANDLER.uninstall(&ctx).unwrap();
        assert_eq!(uninstalled_again.registration, IntegrationRegistration::Absent);
    }

    #[test]
    fn drifted_content_is_reconciled_on_reinstall() {
        let workspace = gitignored_workspace();
        let resolver = resolver(workspace.path());
        let ctx = context(workspace.path(), &resolver);
        let path = workspace.path().join(RELATIVE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, format!("{MARKER_LINE}\nstale content\n")).unwrap();

        let status = HANDLER.status(&ctx).unwrap();
        assert_eq!(status.registration, IntegrationRegistration::Drifted);
        assert_eq!(status.ownership, IntegrationOwnership::OrkWorks);

        let reinstalled = HANDLER.install(&ctx).unwrap();
        assert_eq!(reinstalled.registration, IntegrationRegistration::Installed);
        assert_eq!(fs::read_to_string(&path).unwrap(), PLUGIN_SOURCE);
    }

    #[test]
    fn crlf_drifted_content_is_still_recognized_as_owned() {
        let workspace = gitignored_workspace();
        let resolver = resolver(workspace.path());
        let ctx = context(workspace.path(), &resolver);
        let path = workspace.path().join(RELATIVE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, format!("{MARKER_LINE}\r\nstale content\r\n")).unwrap();

        let status = HANDLER.status(&ctx).unwrap();
        assert_eq!(status.registration, IntegrationRegistration::Drifted);
        assert_eq!(status.ownership, IntegrationOwnership::OrkWorks);

        let reinstalled = HANDLER.install(&ctx).unwrap();
        assert_eq!(reinstalled.registration, IntegrationRegistration::Installed);
        assert_eq!(fs::read_to_string(&path).unwrap(), PLUGIN_SOURCE);
    }

    #[test]
    fn a_foreign_plugin_file_is_never_touched() {
        let workspace = gitignored_workspace();
        let resolver = resolver(workspace.path());
        let ctx = context(workspace.path(), &resolver);
        let path = workspace.path().join(RELATIVE_PATH);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "export default { hooks: {} };\n").unwrap();

        let status = HANDLER.status(&ctx).unwrap();
        assert_eq!(status.ownership, IntegrationOwnership::Ambiguous);

        assert!(matches!(
            HANDLER.install(&ctx).unwrap_err(),
            IntegrationError::OwnershipAmbiguous
        ));
        assert!(matches!(
            HANDLER.uninstall(&ctx).unwrap_err(),
            IntegrationError::OwnershipAmbiguous
        ));
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "export default { hooks: {} };\n"
        );
    }

    #[test]
    fn install_refuses_an_unignored_target() {
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        let resolver = resolver(workspace.path());
        let ctx = context(workspace.path(), &resolver);

        let error = HANDLER.install(&ctx).unwrap_err();

        assert_eq!(error.code(), "not_ignored_target");
        assert!(!workspace.path().join(RELATIVE_PATH).exists());
    }

    #[test]
    fn plugin_source_reports_the_session_created_hook_to_the_harness_session_endpoint() {
        assert!(PLUGIN_SOURCE.contains("session.created"));
        assert!(PLUGIN_SOURCE.contains("ORKWORKS_PORT"));
        assert!(PLUGIN_SOURCE.contains("ORKWORKS_SESSION_ID"));
        assert!(PLUGIN_SOURCE.contains("/sessions/${orkworksSessionId}/harness-session"));
        assert!(PLUGIN_SOURCE.contains("source: \"opencode_hook\""));
        assert!(PLUGIN_SOURCE.starts_with(MARKER_LINE));
    }
}
