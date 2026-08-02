//! Closed, evidence-gated handlers for built-in workspace integrations.
//!
//! A handler may only own the structurally selected fragment which carries its
//! exact marker.  It never searches recursively and never deletes broad hook
//! arrays.

mod aider;
mod claude;
mod codex;
mod copilot;
mod gemini;
mod opencode;

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use crate::harness::definition::IntegrationBinding;
use crate::harness::integration::{
    ConfigFileTransaction, IntegrationActivation, IntegrationConfirmation, IntegrationContext,
    IntegrationCoverage, IntegrationDiagnostic, IntegrationError, IntegrationHandler,
    IntegrationOwnership, IntegrationRegistration, IntegrationStatus, ValidatedWorkspaceTarget,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ReporterPlatform {
    Posix,
    WindowsPowerShell,
}

impl ReporterPlatform {
    pub(crate) fn current() -> Self {
        if cfg!(windows) {
            Self::WindowsPowerShell
        } else {
            Self::Posix
        }
    }

    pub(crate) fn asset_name(self) -> &'static str {
        match self {
            Self::Posix => "report-harness-event.sh",
            Self::WindowsPowerShell => "report-harness-event.ps1",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ReporterInvocation {
    pub program: String,
    pub args: Vec<String>,
    pub shell_command: String,
}

pub(crate) fn handler(binding: &IntegrationBinding) -> &'static dyn IntegrationHandler {
    match binding {
        IntegrationBinding::Claude => &claude::HANDLER,
        IntegrationBinding::Codex => &codex::HANDLER,
        IntegrationBinding::OpenCode => &opencode::HANDLER,
        IntegrationBinding::Gemini => &gemini::HANDLER,
        IntegrationBinding::Copilot => &copilot::HANDLER,
        IntegrationBinding::Aider => &aider::HANDLER,
    }
}

#[derive(Clone)]
pub(crate) struct ToolHookContract {
    pub harness_id: &'static str,
    pub tool_name: &'static str,
    pub relative_path: &'static str,
    /// Kept in the code-owned contract for review/auditing. Each structural
    /// probe uses the same literal because hook schemas carry it differently.
    #[allow(dead_code)]
    pub ownership_marker: &'static str,
    pub coverage: IntegrationCoverage,
    pub activation: IntegrationActivation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FragmentState {
    Absent,
    Installed,
    Drifted,
    Ambiguous,
}

pub(crate) struct JsonHookHandler {
    contract: ToolHookContract,
    probe: fn(&Map<String, Value>, &Path) -> Result<FragmentState, IntegrationError>,
    merge: fn(&mut Map<String, Value>, &Path) -> Result<(), IntegrationError>,
    remove: fn(&mut Map<String, Value>) -> Result<FragmentState, IntegrationError>,
    reconcile: fn(
        &crate::harness::integration::ReporterAssetResolver,
    ) -> Result<PathBuf, IntegrationError>,
}

impl JsonHookHandler {
    pub(crate) const fn new(
        contract: ToolHookContract,
        probe: fn(&Map<String, Value>, &Path) -> Result<FragmentState, IntegrationError>,
        merge: fn(&mut Map<String, Value>, &Path) -> Result<(), IntegrationError>,
        remove: fn(&mut Map<String, Value>) -> Result<FragmentState, IntegrationError>,
        reconcile: fn(
            &crate::harness::integration::ReporterAssetResolver,
        ) -> Result<PathBuf, IntegrationError>,
    ) -> Self {
        Self {
            contract,
            probe,
            merge,
            remove,
            reconcile,
        }
    }

    fn target(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<ValidatedWorkspaceTarget, IntegrationError> {
        ValidatedWorkspaceTarget::new(ctx.workspace, Path::new(self.contract.relative_path))
    }

    fn base_status(
        &self,
        ctx: &IntegrationContext<'_>,
        registration: IntegrationRegistration,
        ownership: IntegrationOwnership,
        activation: IntegrationActivation,
        diagnostics: Vec<IntegrationDiagnostic>,
    ) -> IntegrationStatus {
        IntegrationStatus {
            harness_id: self.contract.harness_id.into(),
            enabled: ctx.enabled,
            tool_detected: ctx.detected_tool.is_some(),
            registration,
            ownership,
            activation,
            coverage: self.contract.coverage.clone(),
            diagnostics,
            confirmation: IntegrationConfirmation::new(
                self.contract.tool_name,
                ctx.workspace,
                "Limited harness notifications",
                &[Path::new(self.contract.relative_path)],
                true,
            )
            .ok(),
        }
    }

    fn error_status(
        &self,
        ctx: &IntegrationContext<'_>,
        error: &IntegrationError,
    ) -> IntegrationStatus {
        let mut status = self.base_status(
            ctx,
            IntegrationRegistration::Error,
            IntegrationOwnership::None,
            IntegrationActivation::Unknown,
            vec![IntegrationDiagnostic {
                code: error.code().into(),
                message: error.to_string(),
                action: None,
            }],
        );
        status.confirmation = None;
        status
    }

    fn status_from_document(
        &self,
        ctx: &IntegrationContext<'_>,
        document: &Map<String, Value>,
        reporter: &Path,
    ) -> Result<IntegrationStatus, IntegrationError> {
        let (registration, ownership, diagnostics) = match (self.probe)(document, reporter)? {
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
                    message:
                        "An OrkWorks-owned integration fragment differs from the supported shape."
                            .into(),
                    action: Some("reconcile".into()),
                }],
            ),
            FragmentState::Ambiguous => (
                IntegrationRegistration::Drifted,
                IntegrationOwnership::Ambiguous,
                vec![IntegrationDiagnostic {
                    code: "ownership_ambiguous".into(),
                    message: "A different OrkWorks integration marker occupies this hook location."
                        .into(),
                    action: None,
                }],
            ),
        };
        let mut diagnostics = diagnostics;
        let activation = if !ctx.enabled {
            IntegrationActivation::Disabled
        } else if registration == IntegrationRegistration::Drifted
            || ownership == IntegrationOwnership::Ambiguous
        {
            IntegrationActivation::Unknown
        } else if ctx.detected_tool.is_none() {
            diagnostics.push(IntegrationDiagnostic {
                code: "tool_not_detected".into(),
                message: "The coding tool was not detected, so integration activation is unknown."
                    .into(),
                action: None,
            });
            IntegrationActivation::Unknown
        } else if ctx.detected_tool.is_some_and(|tool| !tool.compatible) {
            diagnostics.push(IntegrationDiagnostic {
                code: "unsupported_tool_version".into(),
                message: "The detected coding tool version is not eligible for this integration."
                    .into(),
                action: None,
            });
            IntegrationActivation::NeedsTrust
        } else if registration == IntegrationRegistration::Installed {
            self.contract.activation.clone()
        } else {
            IntegrationActivation::Disabled
        };
        let mut status = self.base_status(
            ctx,
            registration.clone(),
            ownership.clone(),
            activation,
            diagnostics,
        );
        if matches!(registration, IntegrationRegistration::Error)
            || matches!(ownership, IntegrationOwnership::Ambiguous)
        {
            status.confirmation = None;
        }
        Ok(status)
    }

    fn load(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<(ConfigFileTransaction, Map<String, Value>, PathBuf), IntegrationError> {
        let target = self.target(ctx)?;
        target.require_local_or_ignored_untracked()?;
        let transaction = ConfigFileTransaction::open(target)?;
        let document = if transaction.current_bytes().is_empty() {
            Map::new()
        } else {
            crate::harness::integration::parse_json_object(transaction.current_bytes())?
        };
        let reporter = ctx
            .reporter_assets
            .stable_path(ReporterPlatform::current().asset_name())?;
        Ok((transaction, document, reporter))
    }
}

impl IntegrationHandler for JsonHookHandler {
    fn status(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        let result = self.load(ctx).and_then(|(_, document, reporter)| {
            self.status_from_document(ctx, &document, &reporter)
        });
        Ok(match result {
            Ok(status) => status,
            Err(error) => self.error_status(ctx, &error),
        })
    }

    fn install(&self, ctx: &IntegrationContext<'_>) -> Result<IntegrationStatus, IntegrationError> {
        let (transaction, mut document, reporter) = self.load(ctx)?;
        match (self.probe)(&document, &reporter)? {
            FragmentState::Installed => {
                return self.status_from_document(ctx, &document, &reporter)
            }
            FragmentState::Ambiguous => return Err(IntegrationError::OwnershipAmbiguous),
            FragmentState::Absent | FragmentState::Drifted => {}
        }
        let reporter = (self.reconcile)(ctx.reporter_assets)?;
        (self.merge)(&mut document, &reporter)?;
        let replacement = serde_json::to_vec_pretty(&document)
            .map_err(|error| IntegrationError::InvalidConfig(error.to_string()))?;
        transaction.commit(&replacement)?;
        self.status(ctx)
    }

    fn uninstall(
        &self,
        ctx: &IntegrationContext<'_>,
    ) -> Result<IntegrationStatus, IntegrationError> {
        let (transaction, mut document, reporter) = self.load(ctx)?;
        match (self.remove)(&mut document)? {
            FragmentState::Absent => return self.status_from_document(ctx, &document, &reporter),
            FragmentState::Ambiguous => return Err(IntegrationError::OwnershipAmbiguous),
            FragmentState::Installed | FragmentState::Drifted => {}
        }
        let replacement = serde_json::to_vec_pretty(&document)
            .map_err(|error| IntegrationError::InvalidConfig(error.to_string()))?;
        transaction.commit(&replacement)?;
        self.status(ctx)
    }
}

pub(crate) fn reconcile_current(
    resolver: &crate::harness::integration::ReporterAssetResolver,
) -> Result<PathBuf, IntegrationError> {
    resolver.reconcile(ReporterPlatform::current().asset_name())
}

#[allow(dead_code)] // Read by generic integration routes in Task 8.
pub(crate) fn generic_shell_status(
    harness_id: &str,
    _workspace: &Path,
    enabled: bool,
    tool_detected: bool,
) -> IntegrationStatus {
    IntegrationStatus {
        harness_id: harness_id.into(),
        enabled,
        tool_detected,
        registration: IntegrationRegistration::Unsupported,
        ownership: IntegrationOwnership::None,
        activation: IntegrationActivation::NotApplicable,
        coverage: IntegrationCoverage::None,
        diagnostics: vec![IntegrationDiagnostic {
            code: "no_deterministic_integration".into(),
            message: "Generic shell has no deterministic workspace integration mechanism.".into(),
            action: None,
        }],
        confirmation: None,
    }
}

pub(crate) fn reporter_invocation_for_platform(
    platform: ReporterPlatform,
    path: &Path,
    marker: &str,
) -> ReporterInvocation {
    match platform {
        ReporterPlatform::Posix => ReporterInvocation {
            program: path.to_string_lossy().into_owned(),
            args: vec!["--marker".into(), marker.into()],
            shell_command: format!(
                "{} --marker {}",
                shell_quote(&path.to_string_lossy()),
                shell_quote(marker)
            ),
        },
        ReporterPlatform::WindowsPowerShell => {
            let args = vec![
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-ExecutionPolicy".into(),
                "Bypass".into(),
                "-File".into(),
                path.to_string_lossy().into_owned(),
                "-Marker".into(),
                marker.into(),
            ];
            ReporterInvocation {
                program: "powershell.exe".into(),
                shell_command: format!(
                    "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File {} -Marker {}",
                    powershell_quote(&path.to_string_lossy()),
                    powershell_quote(marker)
                ),
                args,
            }
        }
    }
}

pub(crate) fn reporter_invocation(path: &Path, marker: &str) -> ReporterInvocation {
    reporter_invocation_for_platform(ReporterPlatform::current(), path, marker)
}

fn shell_quote(value: &str) -> String {
    // Close the quoted string, emit a literal quote via the standard
    // close-escape-reopen idiom (`'\''`), then reopen — NOT `'\"'\"'`, which
    // embeds a literal backslash and leaves the quoting unbalanced.
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn powershell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::harness::definition::{BuiltinDocument, IntegrationBinding, EMBEDDED_BUILTINS};
    use crate::harness::integration::{
        IntegrationContext, IntegrationOwnership, IntegrationRegistration, ReporterAssetResolver,
    };
    use crate::metadata::MetadataStore;

    #[test]
    fn report_harness_event_defaults_to_waiting_attention_and_accepts_a_status_override() {
        let script = include_str!("../../../scripts/report-harness-event.sh");
        assert!(script.contains("ORKWORKS_SESSION_ID"));
        assert!(script.contains("ORKWORKS_PORT"));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/attention"));
        assert!(script.contains("status=\"waiting_for_input\""));
        assert!(script.contains("--status"));
        assert!(script.contains("\"status\":sys.argv[1]"));
    }

    #[test]
    fn report_harness_event_captures_claude_session_id_only_for_claude_marker() {
        let script = include_str!("../../../scripts/report-harness-event.sh");
        assert!(script.contains("claude-code"));
        assert!(script.contains("session_id"));
        assert!(script.contains("/sessions/$ORKWORKS_SESSION_ID/harness-session"));
        assert!(script.contains("\"source\":\"claude_hook\""));
    }

    #[test]
    fn report_harness_event_bounds_every_curl_with_a_timeout() {
        let script = include_str!("../../../scripts/report-harness-event.sh");
        let max_time_count = script.matches("--max-time").count();
        assert_eq!(
            max_time_count, 2,
            "both possible curl calls must cap their own runtime so a stuck orkworksd cannot \
             hang the harness's own hook mechanism"
        );
    }

    #[test]
    fn report_harness_event_parses_the_marker_flag() {
        let script = include_str!("../../../scripts/report-harness-event.sh");
        assert!(script.contains("--marker"));
    }

    #[test]
    fn report_harness_event_forwards_reported_cwd_for_claude_marker() {
        let script = include_str!("../../../scripts/report-harness-event.sh");
        assert!(script.contains("data.get(\"cwd\")"));
        assert!(script.contains("payload[\"cwd\"] = cwd"));
    }

    /// Runs the real `report-harness-event.sh` with the given stdin payload
    /// under `bash -x`, returning its stderr trace (which `set -x` writes
    /// each executed command to, including the constructed JSON payloads).
    fn run_report_harness_event_sh_trace(marker: &str, stdin_payload: &str) -> String {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let script_path = concat!(env!("CARGO_MANIFEST_DIR"), "/scripts/report-harness-event.sh");
        let mut child = Command::new("bash")
            .arg("-x")
            .arg(script_path)
            .arg("--marker")
            .arg(marker)
            .env("ORKWORKS_SESSION_ID", "test-session")
            .env("ORKWORKS_PORT", "1") // unroutable; curl fails fast, harmless (`|| true`)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn report-harness-event.sh");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(stdin_payload.as_bytes())
            .unwrap();
        let output = child.wait_with_output().expect("script should run to completion");
        assert!(output.status.success(), "script exited non-zero: {output:?}");
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    #[test]
    fn report_harness_event_runs_and_forwards_cwd_from_a_real_claude_payload() {
        let trace = run_report_harness_event_sh_trace(
            "orkworks:harness-integration:v2:claude-code",
            r#"{"session_id":"abc123","cwd":"/tmp/some/worktree"}"#,
        );
        assert!(
            trace.contains(r#""cwd": "/tmp/some/worktree""#),
            "expected the cwd from stdin to appear in the constructed attention payload; trace:\n{trace}"
        );
        assert!(
            trace.contains(r#"{"harnessSessionId":"abc123","source":"claude_hook","confidence":0.98}"#),
            "expected session_id to still be forwarded when both fields are present; trace:\n{trace}"
        );
    }

    #[test]
    fn report_harness_event_forwards_session_id_without_shifting_when_cwd_is_absent() {
        // Regression test: the cwd/session_id extraction is a single
        // delimited-line parse (not two `python3` calls), and an earlier
        // version used a whitespace delimiter that `read` silently strips
        // when the first field is empty — shifting session_id into the cwd
        // variable instead. This payload has no "cwd" key at all.
        let trace = run_report_harness_event_sh_trace(
            "orkworks:harness-integration:v2:claude-code",
            r#"{"session_id":"abc123"}"#,
        );
        let attention_payload_line = trace
            .lines()
            .find(|line| line.contains("attention_payload="))
            .unwrap_or_else(|| panic!("expected an attention_payload= trace line; trace:\n{trace}"));
        assert!(
            !attention_payload_line.contains(r#""cwd""#),
            "attention payload should omit cwd entirely when none was reported; line:\n{attention_payload_line}"
        );
        assert!(
            trace.contains(r#"{"harnessSessionId":"abc123","source":"claude_hook","confidence":0.98}"#),
            "session_id must still be forwarded correctly, not shifted into the cwd slot; trace:\n{trace}"
        );
    }

    #[test]
    fn report_harness_event_ps1_always_posts_generic_attention() {
        let script = include_str!("../../../scripts/report-harness-event.ps1");
        assert!(script.contains("ORKWORKS_SESSION_ID"));
        assert!(script.contains("ORKWORKS_PORT"));
        assert!(script.contains("/sessions/$sessionId/attention"));
        assert!(script.contains("waiting_for_input"));
    }

    #[test]
    fn report_harness_event_ps1_captures_claude_session_id_only_for_claude_marker() {
        let script = include_str!("../../../scripts/report-harness-event.ps1");
        assert!(script.contains("claude-code"));
        assert!(script.contains("session_id"));
        assert!(script.contains("/sessions/$sessionId/harness-session"));
        assert!(script.contains("claude_hook"));
    }

    #[test]
    fn report_harness_event_ps1_bounds_every_request_with_a_timeout() {
        let script = include_str!("../../../scripts/report-harness-event.ps1");
        let timeout_count = script.matches("-TimeoutSec").count();
        assert_eq!(
            timeout_count, 2,
            "both possible requests must cap their own runtime so a stuck orkworksd cannot \
             hang the harness's own hook mechanism"
        );
    }

    #[test]
    fn report_harness_event_ps1_parses_the_marker_parameter() {
        let script = include_str!("../../../scripts/report-harness-event.ps1");
        // The Rust side invokes this with a literal `-Marker <value>` flag
        // (see reporter_invocation_for_platform's WindowsPowerShell arm),
        // which PowerShell's CmdletBinding param() block binds to `$Marker`
        // without that dash ever appearing in the script source itself —
        // so assert on the param declaration, the literal counterpart of
        // the .sh version's `--marker` string match.
        assert!(script.contains("[string]$Marker"));
    }

    // Every prior test above (and the .sh equivalents) reads asset content
    // via a synthetic ReporterAssetResolver pointed at a throwaway tempdir,
    // never the real crates/orkworksd/scripts/ directory a handler's
    // reconcile step actually copies from in production. That's exactly why
    // this issue (copilot.rs needing report-harness-event.ps1, which never
    // existed) shipped with zero CI signal: nothing asserted that every
    // reporter asset any handler could reference is really present on disk.
    // Both known asset names are enumerated by ReporterPlatform — a bare
    // existence check on both closes the gap for every current and future
    // handler without needing per-handler knowledge of which asset(s) it
    // reconciles.
    #[test]
    fn every_reporter_asset_exists_in_the_real_scripts_directory() {
        let scripts_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts");
        for platform in [ReporterPlatform::Posix, ReporterPlatform::WindowsPowerShell] {
            let path = scripts_dir.join(platform.asset_name());
            assert!(
                path.is_file(),
                "{platform:?} reporter asset missing from the real scripts/ directory: {}",
                path.display()
            );
        }
    }

    struct Case {
        name: &'static str,
        binding: IntegrationBinding,
        target: &'static str,
    }

    fn json_cases() -> [Case; 3] {
        [
            Case {
                name: "claude",
                binding: IntegrationBinding::Claude,
                target: ".claude/settings.local.json",
            },
            Case {
                name: "gemini",
                binding: IntegrationBinding::Gemini,
                target: ".gemini/settings.json",
            },
            Case {
                name: "copilot",
                binding: IntegrationBinding::Copilot,
                target: ".github/copilot/settings.local.json",
            },
        ]
    }

    #[test]
    fn json_handler_conformance_matrix_preserves_unrelated_configuration() {
        for case in json_cases() {
            let workspace = tempfile::tempdir().unwrap();
            git2::Repository::init(workspace.path()).unwrap();
            fs::write(
                workspace.path().join(".gitignore"),
                format!("{}\n", case.target),
            )
            .unwrap();
            let target = workspace.path().join(case.target);
            fs::create_dir_all(target.parent().unwrap()).unwrap();
            fs::write(&target, r#"{"unrelated":{"keep":true}}"#).unwrap();
            let assets = tempfile::tempdir().unwrap();
            fs::write(
                assets.path().join(ReporterPlatform::Posix.asset_name()),
                "#!/bin/sh\n",
            )
            .unwrap();
            fs::write(
                assets
                    .path()
                    .join(ReporterPlatform::WindowsPowerShell.asset_name()),
                "# noop\n",
            )
            .unwrap();
            let stable = tempfile::tempdir().unwrap();
            let reporter = ReporterAssetResolver {
                source_dir: assets.path().to_path_buf(),
                stable_dir: stable.path().join("hook-scripts"),
            };
            let context = IntegrationContext {
                workspace: workspace.path(),
                workspace_metadata: None,
                orkworks_root: stable.path(),
                enabled: true,
                detected_tool: None,
                reporter_assets: &reporter,
            };

            let absent = handler(&case.binding).status(&context).unwrap();
            assert_eq!(
                absent.registration,
                IntegrationRegistration::Absent,
                "{} absent",
                case.name
            );
            let first = handler(&case.binding).install(&context).unwrap();
            let second = handler(&case.binding).install(&context).unwrap();
            assert_eq!(
                first.registration,
                IntegrationRegistration::Installed,
                "{} install",
                case.name
            );
            assert_eq!(
                second.registration,
                IntegrationRegistration::Installed,
                "{} repeated install",
                case.name
            );
            let persisted: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
            assert_eq!(
                persisted["unrelated"]["keep"], true,
                "{} preservation",
                case.name
            );
            let removed = handler(&case.binding).uninstall(&context).unwrap();
            let removed_again = handler(&case.binding).uninstall(&context).unwrap();
            assert_eq!(
                removed.registration,
                IntegrationRegistration::Absent,
                "{} uninstall",
                case.name
            );
            assert_eq!(
                removed_again.registration,
                IntegrationRegistration::Absent,
                "{} repeated uninstall",
                case.name
            );
            let persisted: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
            assert_eq!(
                persisted["unrelated"]["keep"], true,
                "{} round trip",
                case.name
            );
        }
    }

    #[test]
    fn error_status_surfaces_the_specific_integration_error_message() {
        // libgit2's ignore resolution consults the real machine's global
        // excludes file regardless of the HOME env var (it isn't resolved
        // the way `dirs::home_dir()` is elsewhere in this codebase). A
        // developer machine that happens to globally ignore one of these
        // paths (e.g. a personal `**/.claude/settings.local.json` rule)
        // would silently make that case pass for the wrong reason.
        // Override the test repo's own `core.excludesFile` to a guaranteed-
        // empty file so this test only ever sees the target's real
        // local-ignore state.
        for case in json_cases() {
            let workspace = tempfile::tempdir().unwrap();
            let repo = git2::Repository::init(workspace.path()).unwrap();
            let empty_excludes = workspace.path().join("empty-global-excludes");
            fs::write(&empty_excludes, "").unwrap();
            repo.config()
                .unwrap()
                .set_str("core.excludesFile", empty_excludes.to_str().unwrap())
                .unwrap();
            // Deliberately no .gitignore entry for `case.target`, so
            // `require_local_or_ignored_untracked` rejects it as not ignored.
            let assets = tempfile::tempdir().unwrap();
            fs::write(
                assets.path().join(ReporterPlatform::Posix.asset_name()),
                "#!/bin/sh\n",
            )
            .unwrap();
            fs::write(
                assets
                    .path()
                    .join(ReporterPlatform::WindowsPowerShell.asset_name()),
                "# noop\n",
            )
            .unwrap();
            let stable = tempfile::tempdir().unwrap();
            let reporter = ReporterAssetResolver {
                source_dir: assets.path().to_path_buf(),
                stable_dir: stable.path().join("hook-scripts"),
            };
            let context = IntegrationContext {
                workspace: workspace.path(),
                workspace_metadata: None,
                orkworks_root: stable.path(),
                enabled: true,
                detected_tool: None,
                reporter_assets: &reporter,
            };

            let status = handler(&case.binding).status(&context).unwrap();
            assert_eq!(
                status.registration,
                IntegrationRegistration::Error,
                "{}",
                case.name
            );
            assert_eq!(
                status.diagnostics[0].message,
                "Integration configuration is not ignored by Git and will not be edited automatically.",
                "{}",
                case.name
            );
        }
    }

    #[test]
    fn malformed_ambiguous_and_uneligible_json_targets_are_never_mutated() {
        let case = Case {
            name: "claude",
            binding: IntegrationBinding::Claude,
            target: ".claude/settings.local.json",
        };
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        let target = workspace.path().join(case.target);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, "{}").unwrap();
        let assets = tempfile::tempdir().unwrap();
        fs::write(
            assets.path().join(ReporterPlatform::current().asset_name()),
            "#!/bin/sh\n",
        )
        .unwrap();
        let stable = tempfile::tempdir().unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: stable.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };
        let empty_excludes = workspace.path().join("empty-excludes");
        fs::write(&empty_excludes, "").unwrap();
        git2::Repository::open(workspace.path())
            .unwrap()
            .config()
            .unwrap()
            .set_str("core.excludesfile", empty_excludes.to_str().unwrap())
            .unwrap();
        assert_eq!(
            handler(&case.binding).install(&context).unwrap_err().code(),
            "not_ignored_target"
        );

        fs::write(&target, "{bad json").unwrap();
        fs::write(
            workspace.path().join(".gitignore"),
            format!("{}\n", case.target),
        )
        .unwrap();
        let status = handler(&case.binding).status(&context).unwrap();
        assert_eq!(status.registration, IntegrationRegistration::Error);
        assert_eq!(fs::read_to_string(&target).unwrap(), "{bad json");
        assert_eq!(
            handler(&case.binding).install(&context).unwrap_err().code(),
            "invalid_config"
        );

        fs::write(&target, "{}").unwrap();
        let repository = git2::Repository::open(workspace.path()).unwrap();
        let mut index = repository.index().unwrap();
        index.add_path(Path::new(case.target)).unwrap();
        index.write().unwrap();
        assert_eq!(
            handler(&case.binding).install(&context).unwrap_err().code(),
            "tracked_target"
        );
        index.remove_path(Path::new(case.target)).unwrap();
        index.write().unwrap();

        fs::write(&target, r#"{"hooks":{"Notification":[{"hooks":[{"type":"command","args":["orkworks:harness-integration:v2:other"]}]}]}}"#).unwrap();
        assert_eq!(
            handler(&case.binding).install(&context).unwrap_err().code(),
            "ownership_ambiguous"
        );
        assert!(fs::read_to_string(&target).unwrap().contains("v2:other"));
    }

    #[test]
    fn unsupported_bindings_do_not_touch_workspace_files() {
        for binding in [IntegrationBinding::Codex, IntegrationBinding::OpenCode] {
            let workspace = tempfile::tempdir().unwrap();
            let assets = tempfile::tempdir().unwrap();
            let stable = tempfile::tempdir().unwrap();
            let reporter = ReporterAssetResolver {
                source_dir: assets.path().to_path_buf(),
                stable_dir: stable.path().join("hook-scripts"),
            };
            let context = IntegrationContext {
                workspace: workspace.path(),
                workspace_metadata: None,
                orkworks_root: stable.path(),
                enabled: true,
                detected_tool: None,
                reporter_assets: &reporter,
            };
            assert_eq!(
                handler(&binding).install(&context).unwrap().registration,
                IntegrationRegistration::Unsupported
            );
            assert!(fs::read_dir(workspace.path()).unwrap().next().is_none());
        }
    }

    #[test]
    fn copilot_uses_top_level_inline_hook_version_and_preserves_unrelated_settings() {
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        fs::write(
            workspace.path().join(".gitignore"),
            ".github/copilot/settings.local.json\n",
        )
        .unwrap();
        let target = workspace.path().join(".github/copilot/settings.local.json");
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(&target, r#"{"unrelated":{"keep":true}}"#).unwrap();
        let assets = tempfile::tempdir().unwrap();
        fs::write(
            assets.path().join(ReporterPlatform::Posix.asset_name()),
            "#!/bin/sh\n",
        )
        .unwrap();
        fs::write(
            assets
                .path()
                .join(ReporterPlatform::WindowsPowerShell.asset_name()),
            "# noop\n",
        )
        .unwrap();
        let stable = tempfile::tempdir().unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: stable.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };
        handler(&IntegrationBinding::Copilot)
            .install(&context)
            .unwrap();
        let document: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
        assert_eq!(document["version"], 1);
        assert!(document["hooks"].get("version").is_none());
        assert_eq!(document["unrelated"]["keep"], true);
        let hook = &document["hooks"]["notification"][0];
        assert!(hook.get("bash").is_some());
        assert!(hook.get("powershell").is_some());

        fs::write(&target, r#"{"version":2,"unrelated":{"keep":true}}"#).unwrap();
        let before = fs::read_to_string(&target).unwrap();
        assert_eq!(
            handler(&IntegrationBinding::Copilot)
                .install(&context)
                .unwrap_err()
                .code(),
            "invalid_config"
        );
        assert_eq!(fs::read_to_string(target).unwrap(), before);
    }

    #[test]
    fn disabled_and_unknown_activation_remain_independent_axes() {
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        fs::write(
            workspace.path().join(".gitignore"),
            ".gemini/settings.json\n",
        )
        .unwrap();
        let assets = tempfile::tempdir().unwrap();
        let stable = tempfile::tempdir().unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: stable.path(),
            enabled: false,
            detected_tool: None,
            reporter_assets: &reporter,
        };
        let status = handler(&IntegrationBinding::Gemini)
            .status(&context)
            .unwrap();
        assert_eq!(status.registration, IntegrationRegistration::Absent);
        assert_eq!(
            status.activation,
            crate::harness::integration::IntegrationActivation::Disabled
        );
        let detected = crate::harness::integration::DetectedTool {
            executable: std::path::PathBuf::from("gemini"),
            version: Some("unsupported".into()),
            compatible: false,
        };
        let unsupported_context = IntegrationContext {
            enabled: true,
            detected_tool: Some(&detected),
            ..context
        };
        assert_eq!(
            handler(&IntegrationBinding::Gemini)
                .status(&unsupported_context)
                .unwrap()
                .registration,
            IntegrationRegistration::Absent
        );
        assert_eq!(
            handler(&IntegrationBinding::Gemini)
                .status(&unsupported_context)
                .unwrap()
                .activation,
            IntegrationActivation::NeedsTrust
        );
    }

    #[test]
    fn resolved_generic_shell_status_is_explicitly_unsupported() {
        let workspace = tempfile::tempdir().unwrap();
        let assets = tempfile::tempdir().unwrap();
        let stable = tempfile::tempdir().unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: stable.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let registry =
            crate::harness::registry::resolve_document(&builtins, &Default::default()).unwrap();
        let status = registry
            .get("generic-shell")
            .unwrap()
            .integration_status(&context)
            .unwrap();
        assert_eq!(status.registration, IntegrationRegistration::Unsupported);
        assert_eq!(status.ownership, IntegrationOwnership::None);
    }

    #[test]
    fn aider_launch_persists_workspace_owned_enablement_and_augments_once() {
        let workspace = tempfile::tempdir().unwrap();
        let metadata = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(metadata.path());
        let assets = tempfile::tempdir().unwrap();
        fs::write(
            assets.path().join(ReporterPlatform::current().asset_name()),
            "#!/bin/sh\n",
        )
        .unwrap();
        let stable = tempfile::tempdir().unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: Some(&store),
            orkworks_root: stable.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };
        assert_eq!(
            handler(&IntegrationBinding::Aider)
                .install(&context)
                .unwrap()
                .registration,
            IntegrationRegistration::Installed
        );
        let preference: Value = serde_json::from_slice(
            &fs::read(metadata.path().join("integrations/aider.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(preference["version"], 1);
        assert_eq!(preference["enabled"], true);

        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let registry =
            crate::harness::registry::resolve_document(&builtins, &Default::default()).unwrap();
        let aider = registry.get("aider").unwrap();
        let path = reporter
            .stable_path(ReporterPlatform::current().asset_name())
            .unwrap();
        let mut command = aider.build_launch("/workspace", None);
        aider
            .augment_launch_for_integration(&mut command, true, Some(&path))
            .unwrap();
        aider
            .augment_launch_for_integration(&mut command, true, Some(&path))
            .unwrap();
        assert_eq!(
            command
                .args
                .windows(2)
                .filter(|pair| pair[0] == "--notifications-command")
                .count(),
            1
        );
        assert_eq!(
            handler(&IntegrationBinding::Aider)
                .uninstall(&context)
                .unwrap()
                .ownership,
            IntegrationOwnership::None
        );
    }

    #[test]
    fn reporter_rendering_is_explicit_for_posix_and_powershell() {
        let posix = reporter_invocation_for_platform(
            ReporterPlatform::Posix,
            Path::new("/stable path/report-harness-event.sh"),
            "marker'value",
        );
        assert_eq!(posix.program, "/stable path/report-harness-event.sh");
        assert_eq!(posix.args, vec!["--marker", "marker'value"]);
        assert!(posix.shell_command.contains("'\\''"));

        let powershell = reporter_invocation_for_platform(
            ReporterPlatform::WindowsPowerShell,
            Path::new(r"C:\stable path\report-harness-event.ps1"),
            "marker'value",
        );
        assert_eq!(powershell.program, "powershell.exe");
        assert_eq!(powershell.args[0], "-NoProfile");
        assert!(powershell.args.contains(&"-File".into()));
        assert!(powershell.shell_command.contains("powershell.exe"));
        assert!(powershell.shell_command.contains("-File"));
        assert!(powershell.shell_command.contains("''"));
    }

    #[cfg(unix)]
    #[test]
    fn shell_quote_produces_syntactically_valid_shell_for_apostrophe_containing_paths() {
        // A real-world trigger: a home directory or workspace path containing
        // an apostrophe (e.g. "/Users/o'brien/..."), which shell_quote must
        // still embed as one syntactically valid, single shell token — this
        // is what feeds Copilot's `bash`/`powershell` hook fields and
        // Gemini's `command` field (copilot.rs, gemini.rs), both of which
        // execute `shell_command` through an actual shell.
        let invocation = reporter_invocation_for_platform(
            ReporterPlatform::Posix,
            Path::new("/Users/o'brien/.orkworks/hook-scripts/report-harness-event.sh"),
            "orkworks:harness-integration:v2:gemini",
        );

        let status = std::process::Command::new("sh")
            .arg("-n")
            .arg("-c")
            .arg(&invocation.shell_command)
            .status()
            .expect("sh must be available to validate shell syntax");
        assert!(
            status.success(),
            "shell_command is not valid shell syntax: {}",
            invocation.shell_command
        );
    }

    #[test]
    fn aider_does_not_enable_launch_augmentation_when_reporter_reconciliation_fails() {
        let workspace = tempfile::tempdir().unwrap();
        let metadata = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(metadata.path());
        let assets = tempfile::tempdir().unwrap();
        let stable = tempfile::tempdir().unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: Some(&store),
            orkworks_root: stable.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };
        assert!(handler(&IntegrationBinding::Aider)
            .install(&context)
            .is_err());
        assert!(!metadata.path().join("integrations/aider.json").exists());
    }

    #[test]
    fn aider_refuses_malformed_preference_without_overwriting_it() {
        let workspace = tempfile::tempdir().unwrap();
        let metadata = tempfile::tempdir().unwrap();
        fs::create_dir_all(metadata.path().join("integrations")).unwrap();
        let preference = metadata.path().join("integrations/aider.json");
        fs::write(&preference, "{bad json").unwrap();
        let store = MetadataStore::new(metadata.path());
        let assets = tempfile::tempdir().unwrap();
        fs::write(
            assets.path().join(ReporterPlatform::current().asset_name()),
            "#!/bin/sh\n",
        )
        .unwrap();
        let stable = tempfile::tempdir().unwrap();
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join("hook-scripts"),
        };
        let context = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: Some(&store),
            orkworks_root: stable.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };
        let status = handler(&IntegrationBinding::Aider)
            .status(&context)
            .unwrap();
        assert_eq!(status.registration, IntegrationRegistration::Error);
        assert!(status.confirmation.is_none());
        assert!(handler(&IntegrationBinding::Aider)
            .uninstall(&context)
            .is_err());
        assert_eq!(fs::read_to_string(preference).unwrap(), "{bad json");
    }

    #[test]
    fn aider_launch_preserves_an_unrelated_notification_command_as_a_conflict() {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let registry =
            crate::harness::registry::resolve_document(&builtins, &Default::default()).unwrap();
        let aider = registry.get("aider").unwrap();
        let mut command = aider.build_launch("/workspace", None);
        command
            .args
            .extend(["--notifications-command".into(), "user-command".into()]);
        let error = aider
            .augment_launch_for_integration(
                &mut command,
                true,
                Some(Path::new("/stable/report-harness-event.sh")),
            )
            .unwrap_err();
        assert_eq!(error.code(), "launch_conflict");
        assert_eq!(
            command
                .args
                .windows(2)
                .filter(|pair| pair[0] == "--notifications-command")
                .count(),
            1
        );
    }
}
