//! Private run-local instruction selection; never native identity admission.

use crate::harness::{
    definition::{BuiltinDocument, DefinitionOrigin, EMBEDDED_BUILTINS},
    registry::ResolvedHarness,
    CommandSpec,
};
use std::io::Read;
use std::path::{Path, PathBuf};

// Remains off until a separately authorized actual-source native fixture passes.
const NATIVE_QUALIFIED: bool = false;
const REPORTER: &[u8] = include_bytes!("../../scripts/opencode-session-reporter.js");
const HELPER: &[u8] = include_bytes!("../../scripts/session-metadata.py");
const REPORTER_PATH: &str = ".opencode/plugins/orkworks-session-reporter.js";

#[derive(Clone, Debug)]
pub(crate) enum InstructionIntent {
    NewSession,
    ExactResume(String),
}

#[derive(Clone, Debug)]
pub(crate) struct LaunchRequest {
    intent: InstructionIntent,
    command: CommandSpec,
}
impl LaunchRequest {
    pub(crate) fn new(
        harness: &ResolvedHarness,
        intent: InstructionIntent,
        enabled: bool,
        command: &CommandSpec,
        model: Option<&str>,
    ) -> Option<Self> {
        if !enabled
            || harness.definition.id != "opencode"
            || harness.origin == DefinitionOrigin::Custom
        {
            return None;
        }
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).ok()?;
        let builtin = builtins.builtins.iter().find(|h| h.id == "opencode")?;
        let actual = &harness.definition;
        // Only source-owned templates/bindings qualify. Model selection and
        // unrelated display settings may vary, arbitrary wrappers/argv may not.
        if actual.launch != builtin.launch
            || actual.resume != builtin.resume
            || actual.integration != builtin.integration
            || actual.session_signals != builtin.session_signals
        {
            return None;
        }
        let expected = match &intent {
            InstructionIntent::NewSession => harness.build_launch(&command.cwd, model),
            InstructionIntent::ExactResume(id) => {
                if !valid_id(id) {
                    return None;
                }
                harness.build_resume(
                    crate::harness::ResumeStrategy::Exact,
                    &command.cwd,
                    Some(id),
                    None,
                    model,
                )?
            }
        };
        if expected != *command {
            return None;
        }
        Some(Self {
            intent,
            command: command.clone(),
        })
    }
}
fn qualified_version(version: &str) -> bool {
    // Effect 4.0.6, pinned by OpenCode 2.2.5, prints the CLI name and a v
    // prefix. Normalize presentation only; never accept an arbitrary version
    // token embedded in warnings, another command, or a prerelease banner.
    let plain = crate::peon::strip_ansi(version);
    matches!(plain.trim(), "2.2.5" | "opencode v2.2.5")
}

fn valid_id(id: &str) -> bool {
    id.len() > 4
        && id.len() <= 256
        && id.starts_with("ses_")
        && id[4..]
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}
fn file_matches(path: &Path, expected: &[u8]) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.len() != expected.len() as u64 {
        return false;
    }
    let Ok(file) = std::fs::File::open(path) else {
        return false;
    };
    let mut bytes = Vec::new();
    file.take(expected.len() as u64 + 1)
        .read_to_end(&mut bytes)
        .is_ok()
        && bytes == expected
}
fn reporter_matches(request: &LaunchRequest, reporter: &Path) -> bool {
    use crate::harness::integration::ValidatedWorkspaceTarget;
    if reporter != Path::new(&request.command.cwd).join(REPORTER_PATH) {
        return false;
    }
    let Ok(target) =
        ValidatedWorkspaceTarget::new(Path::new(&request.command.cwd), Path::new(REPORTER_PATH))
    else {
        return false;
    };
    target.require_local_or_ignored_untracked().is_ok() && file_matches(reporter, REPORTER)
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ExecutableSnapshot {
    path: PathBuf,
    length: u64,
    modified: std::time::SystemTime,
}
impl ExecutableSnapshot {
    fn read(path: &Path) -> Option<Self> {
        let path = std::fs::canonicalize(path).ok()?;
        let metadata = std::fs::metadata(&path).ok()?;
        if !metadata.is_file() {
            return None;
        }
        let mut header = [0; 8];
        std::fs::File::open(&path)
            .ok()?
            .read_exact(&mut header)
            .ok()?;
        // Qualified fixture is a native arm64 Mach-O, not a shell/Node wrapper.
        if header != [0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1] {
            return None;
        }
        Some(Self {
            path,
            length: metadata.len(),
            modified: metadata.modified().ok()?,
        })
    }
}

pub(crate) struct PreparedScope {
    request: LaunchRequest,
    root_id: String,
    reporter: PathBuf,
    helper: PathBuf,
    executable: ExecutableSnapshot,
}
impl PreparedScope {
    fn from_evidence(
        request: LaunchRequest,
        reporter: PathBuf,
        helper: PathBuf,
        executable: PathBuf,
        version: &str,
        qualified_platform: bool,
        random: impl FnOnce() -> Result<String, String>,
    ) -> Result<Option<Self>, String> {
        if !qualified_platform
            || !qualified_version(version)
            || !helper.is_absolute()
            || helper
                .to_str()
                .is_none_or(|p| p.len() > 4096 || p.chars().any(char::is_control))
            || !reporter_matches(&request, &reporter)
            || !file_matches(&helper, HELPER)
        {
            return Ok(None);
        }
        let Some(executable) = ExecutableSnapshot::read(&executable) else {
            return Ok(None);
        };
        let root_id = match &request.intent {
            InstructionIntent::NewSession => {
                let random = random()?;
                if random.len() != 32
                    || !random
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err("failed to generate secure worktree instruction selector".into());
                }
                format!("ses_{random}")
            }
            InstructionIntent::ExactResume(id) => id.clone(),
        };
        Ok(Some(Self {
            request,
            root_id,
            reporter,
            helper,
            executable,
        }))
    }
    pub(crate) fn apply(
        &self,
        cmd: &mut portable_pty::CommandBuilder,
        original: &CommandSpec,
    ) -> bool {
        if *original != self.request.command
            || !reporter_matches(&self.request, &self.reporter)
            || !file_matches(&self.helper, HELPER)
            || ExecutableSnapshot::read(&self.executable.path).as_ref() != Some(&self.executable)
        {
            return false;
        }
        cmd.get_argv_mut()[0] = self.executable.path.as_os_str().to_owned();
        if matches!(self.request.intent, InstructionIntent::NewSession) {
            cmd.args(["--session", &self.root_id]);
        }
        cmd.env("ORKWORKS_WORKTREE_INSTRUCTION_PROTOCOL", "1");
        cmd.env("ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID", &self.root_id);
        cmd.env("ORKWORKS_WORKTREE_INSTRUCTION_HELPER", &self.helper);
        true
    }
}

pub(crate) async fn prepare(
    request: Option<LaunchRequest>,
) -> Result<Option<PreparedScope>, String> {
    if !NATIVE_QUALIFIED {
        return Ok(None);
    }
    let Some(helper) =
        dirs::home_dir().map(|home| home.join(".orkworks/hook-scripts/session-metadata.py"))
    else {
        return Ok(None);
    };
    prepare_with_helper(request, helper).await
}

async fn prepare_with_helper(
    request: Option<LaunchRequest>,
    helper: PathBuf,
) -> Result<Option<PreparedScope>, String> {
    if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        return Ok(None);
    }
    let Some(request) = request else {
        return Ok(None);
    };
    let Some(tool) = crate::harness::detect::probe_installed_tool(&request.command.program) else {
        return Ok(None);
    };
    let Some(before) = ExecutableSnapshot::read(&tool.executable) else {
        return Ok(None);
    };
    let reporter = Path::new(&request.command.cwd).join(REPORTER_PATH);
    if !reporter_matches(&request, &reporter) || !file_matches(&helper, HELPER) {
        return Ok(None);
    }
    let version = crate::harness::detect::probe_tool_version(&before.path).await;
    #[cfg(test)]
    tests::record_fixture_preparation(serde_json::json!({
        "stage": "version", "returned": version.is_some(),
        "qualified": version.as_deref().is_some_and(qualified_version),
        "bytes": version.as_ref().map_or(0, String::len),
        "lines": version.as_ref().map_or(0, |value| value.lines().count()),
    }));
    let Some(version) = version else {
        return Ok(None);
    };
    if ExecutableSnapshot::read(&tool.executable).as_ref() != Some(&before) {
        return Ok(None);
    }
    PreparedScope::from_evidence(
        request,
        reporter,
        helper,
        before.path,
        &version,
        true,
        || {
            let token = super::terminal_runtime::new_workflow_report_token().map_err(|_| {
                "failed to generate secure worktree instruction selector".to_string()
            })?;
            Ok(token[..32].to_string())
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::definition::{
        BuiltinDocument, DefinitionOrigin, HarnessUserDocument, EMBEDDED_BUILTINS,
    };
    use crate::harness::registry::resolve_document;

    fn harness() -> crate::harness::registry::ResolvedHarness {
        resolve_document(
            &BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap(),
            &HarnessUserDocument::default(),
        )
        .unwrap()
        .get("opencode")
        .unwrap()
        .clone()
    }
    fn fixture(
        intent: InstructionIntent,
    ) -> (tempfile::TempDir, LaunchRequest, PathBuf, PathBuf, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let h = harness();
        let command = match &intent {
            InstructionIntent::NewSession => h.build_launch(dir.path().to_str().unwrap(), None),
            InstructionIntent::ExactResume(id) => h
                .build_resume(
                    crate::harness::ResumeStrategy::Exact,
                    dir.path().to_str().unwrap(),
                    Some(id),
                    None,
                    None,
                )
                .unwrap(),
        };
        let request = LaunchRequest::new(&h, intent, true, &command, None).unwrap();
        git2::Repository::init(dir.path()).unwrap();
        std::fs::write(
            dir.path().join(".gitignore"),
            ".opencode/plugins/orkworks-session-reporter.js\n",
        )
        .unwrap();
        let reporter = dir.path().join(REPORTER_PATH);
        std::fs::create_dir_all(reporter.parent().unwrap()).unwrap();
        let helper = dir.path().join("session-metadata.py");
        let executable = dir.path().join("opencode");
        std::fs::write(&reporter, REPORTER).unwrap();
        std::fs::write(&helper, HELPER).unwrap();
        std::fs::write(&executable, [0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1]).unwrap();
        (dir, request, reporter, helper, executable)
    }
    fn evidence(
        request: LaunchRequest,
        reporter: PathBuf,
        helper: PathBuf,
        exe: PathBuf,
        version: &str,
    ) -> Result<Option<PreparedScope>, String> {
        PreparedScope::from_evidence(request, reporter, helper, exe, version, true, || {
            Ok("0123456789abcdef0123456789abcdef".into())
        })
    }
    #[test]
    fn new_root_argv_and_env_select_the_same_fresh_id() {
        let (_dir, request, reporter, helper, exe) = fixture(InstructionIntent::NewSession);
        let original = request.command.clone();
        let prepared = evidence(request, reporter, helper, exe, "2.2.5")
            .unwrap()
            .unwrap();
        let mut cmd = portable_pty::CommandBuilder::new(&original.program);
        cmd.args(&original.args);
        assert!(prepared.apply(&mut cmd, &original));
        let argv = cmd.get_argv();
        assert_eq!(argv[argv.len() - 2], "--session");
        assert_eq!(argv.last().unwrap(), "ses_0123456789abcdef0123456789abcdef");
        assert_eq!(
            cmd.get_env("ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID")
                .unwrap(),
            argv.last().unwrap()
        );
        assert_eq!(
            cmd.get_env("ORKWORKS_WORKTREE_INSTRUCTION_PROTOCOL")
                .unwrap(),
            "1"
        );
    }
    #[test]
    fn pinned_cli_version_banner_qualifies_the_selected_root() {
        // OpenCode v2.2.5 pins Effect 4.0.6: CliOutput.formatVersion renders
        // the command name plus v-prefixed version, optionally with SGR.
        for version in [
            "opencode v2.2.5\n",
            "\x1b[1mopencode\x1b[0m \x1b[2mv\x1b[0m\x1b[1m2.2.5\x1b[0m\n",
        ] {
            let (_dir, request, reporter, helper, exe) = fixture(InstructionIntent::NewSession);
            let original = request.command.clone();
            let scope = evidence(request, reporter, helper, exe, version)
                .unwrap()
                .expect("pinned CLI banner must qualify");
            let mut cmd = portable_pty::CommandBuilder::new(&original.program);
            cmd.args(&original.args);
            assert!(scope.apply(&mut cmd, &original));
            assert_eq!(
                cmd.get_env("ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID")
                    .unwrap(),
                "ses_0123456789abcdef0123456789abcdef"
            );
        }
    }

    #[test]
    fn exact_resume_preserves_requested_argv() {
        let (_dir, request, reporter, helper, exe) =
            fixture(InstructionIntent::ExactResume("ses_requested".into()));
        let original = request.command.clone();
        let prepared = evidence(request, reporter, helper, exe, "2.2.5")
            .unwrap()
            .unwrap();
        let mut cmd = portable_pty::CommandBuilder::new(&original.program);
        cmd.args(&original.args);
        assert!(prepared.apply(&mut cmd, &original));
        assert_eq!(
            cmd.get_argv()[1..],
            original
                .args
                .iter()
                .map(std::ffi::OsString::from)
                .collect::<Vec<_>>()
        );
        assert_eq!(
            cmd.get_env("ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID")
                .unwrap(),
            "ses_requested"
        );
    }
    #[test]
    fn asset_drift_during_await_discards_scope_and_argv_together() {
        let (_dir, request, reporter, helper, exe) = fixture(InstructionIntent::NewSession);
        let original = request.command.clone();
        let prepared = evidence(request, reporter.clone(), helper, exe, "2.2.5")
            .unwrap()
            .unwrap();
        std::fs::write(reporter, "drift").unwrap();
        let mut cmd = portable_pty::CommandBuilder::new(&original.program);
        cmd.args(&original.args);
        let before = cmd.get_argv().clone();
        assert!(!prepared.apply(&mut cmd, &original));
        assert_eq!(*cmd.get_argv(), before);
        assert!(cmd
            .get_env("ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID")
            .is_none());
    }
    #[test]
    fn unsupported_version_platform_assets_and_wrapper_stay_unscoped() {
        let (_dir, request, reporter, helper, exe) = fixture(InstructionIntent::NewSession);
        for version in [
            "2.2.4",
            "2.2.5-beta",
            "2.2.6",
            "",
            "opencode v2.2.5-beta",
            "opencode v2.2.5+custom",
            "opencode v2.2.6",
            "other v2.2.5",
            "warning\nopencode v2.2.5",
            "opencode v2.2.5 extra",
        ] {
            assert!(evidence(
                request.clone(),
                reporter.clone(),
                helper.clone(),
                exe.clone(),
                version
            )
            .unwrap()
            .is_none());
        }
        assert!(PreparedScope::from_evidence(
            request.clone(),
            reporter.clone(),
            helper.clone(),
            exe.clone(),
            "2.2.5",
            false,
            || panic!("unqualified platform must not generate ID")
        )
        .unwrap()
        .is_none());
        std::fs::write(&exe, "#!/bin/sh\n").unwrap();
        assert!(evidence(
            request.clone(),
            reporter.clone(),
            helper.clone(),
            exe.clone(),
            "2.2.5"
        )
        .unwrap()
        .is_none());
        std::fs::remove_file(helper).unwrap();
        assert!(
            evidence(request, reporter, PathBuf::from("/missing"), exe, "2.2.5")
                .unwrap()
                .is_none()
        );
    }
    #[test]
    fn profile_overrides_custom_and_disabled_integrations_cannot_request_scope() {
        let h = harness();
        let command = h.build_launch("/repo", None);
        assert!(
            LaunchRequest::new(&h, InstructionIntent::NewSession, false, &command, None).is_none()
        );
        let mut changed = h.clone();
        changed.origin = DefinitionOrigin::Custom;
        assert!(LaunchRequest::new(
            &changed,
            InstructionIntent::NewSession,
            true,
            &command,
            None
        )
        .is_none());
        if let crate::harness::definition::LaunchCapability::CommandTemplate { args, .. } =
            &mut changed.definition.launch
        {
            args.push("--other".into());
        }
        assert!(LaunchRequest::new(
            &changed,
            InstructionIntent::NewSession,
            true,
            &command,
            None
        )
        .is_none());
        let mut changed_command = command.clone();
        changed_command.args.push("--continue".into());
        assert!(LaunchRequest::new(
            &h,
            InstructionIntent::NewSession,
            true,
            &changed_command,
            None
        )
        .is_none());
    }
    #[test]
    fn explicit_model_selection_keeps_the_source_owned_command_eligible() {
        let h = harness();
        let command = h.build_launch("/repo", Some("qwen3"));
        assert!(LaunchRequest::new(
            &h,
            InstructionIntent::NewSession,
            true,
            &command,
            Some("qwen3")
        )
        .is_some());
    }
    #[test]
    fn asset_bytes_outside_the_owned_workspace_target_cannot_qualify() {
        let (_dir, request, reporter, helper, exe) = fixture(InstructionIntent::NewSession);
        let foreign = reporter.with_file_name("foreign.js");
        std::fs::write(&foreign, REPORTER).unwrap();
        assert!(evidence(request, foreign, helper, exe, "2.2.5")
            .unwrap()
            .is_none());
    }
    #[test]
    fn eligible_randomness_failure_is_an_error_before_any_spawn() {
        let (_dir, request, reporter, helper, exe) = fixture(InstructionIntent::NewSession);
        assert!(PreparedScope::from_evidence(
            request,
            reporter,
            helper,
            exe,
            "2.2.5",
            true,
            || Err("OS randomness unavailable".into())
        )
        .is_err());
    }
    #[test]
    fn production_remains_disabled_before_actual_source_native_qualification() {
        assert!(!NATIVE_QUALIFIED);
    }
    // Fixture-only diagnostics retain static labels, booleans and counts.
    // Raw native output, paths, IDs, environment and credentials are excluded.
    pub(super) fn record_fixture_preparation(facts: serde_json::Value) {
        use std::io::Write;
        let Some(path) = std::env::var_os("ORKWORKS_FIXTURE_PREPARATION_FACTS") else {
            return;
        };
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("fixture preparation facts must be writable");
        let line = serde_json::to_vec(&facts).unwrap();
        assert!(
            file.metadata().unwrap().len() + line.len() as u64 + 1 <= 4096,
            "fixture preparation facts bound"
        );
        file.write_all(&line).unwrap();
        file.write_all(b"\n").unwrap();
    }

    // Select only within the separately owner-authorized native fixture. This
    // exports selector argv/env only, never a reporting capability or server.
    #[tokio::test]
    #[ignore = "owner-authorized native qualification fixture only"]
    async fn export_native_worktree_instruction_fixture() {
        let workspace = std::env::var("ORKWORKS_FIXTURE_WORKSPACE").expect("fixture workspace");
        let helper =
            PathBuf::from(std::env::var_os("ORKWORKS_FIXTURE_HELPER").expect("fixture helper"));
        let output = PathBuf::from(
            std::env::var_os("ORKWORKS_FIXTURE_SELECTOR_OUTPUT").expect("fixture output"),
        );
        let h = harness();
        let command = h.build_launch(&workspace, None);
        let request =
            LaunchRequest::new(&h, InstructionIntent::NewSession, true, &command, None).unwrap();
        let tool = crate::harness::detect::probe_installed_tool(&command.program);
        record_fixture_preparation(serde_json::json!({
            "stage": "preflight",
            "platformQualified": cfg!(all(target_os = "macos", target_arch = "aarch64")),
            "toolFound": tool.is_some(),
            "nativeExecutable": tool.as_ref().is_some_and(|tool| ExecutableSnapshot::read(&tool.executable).is_some()),
            "reporterCurrentOwned": reporter_matches(&request, &Path::new(&workspace).join(REPORTER_PATH)),
            "helperCurrent": file_matches(&helper, HELPER),
        }));
        let prepared = prepare_with_helper(Some(request), helper).await.unwrap();
        record_fixture_preparation(
            serde_json::json!({"stage": "prepared", "scopePrepared": prepared.is_some()}),
        );
        let prepared = prepared.expect("qualified native fixture");
        let mut cmd = portable_pty::CommandBuilder::new(&command.program);
        cmd.args(&command.args);
        assert!(prepared.apply(&mut cmd, &command));
        let argv: Vec<_> = cmd
            .get_argv()
            .iter()
            .map(|arg| arg.to_str().unwrap())
            .collect();
        let env: std::collections::BTreeMap<_, _> = [
            "ORKWORKS_WORKTREE_INSTRUCTION_PROTOCOL",
            "ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID",
            "ORKWORKS_WORKTREE_INSTRUCTION_HELPER",
        ]
        .into_iter()
        .map(|key| (key, cmd.get_env(key).unwrap().to_str().unwrap()))
        .collect();
        std::fs::write(
            output,
            serde_json::to_vec(&serde_json::json!({"argv":argv,"env":env})).unwrap(),
        )
        .unwrap();
    }
}

#[cfg(all(test, unix))]
pub(super) mod testing {
    use super::*;
    use std::sync::{Arc, Mutex};
    #[derive(Debug)]
    pub(crate) struct SpawnSnapshot {
        pub argv: Vec<std::ffi::OsString>,
        pub selector: Option<String>,
    }
    pub(crate) struct RuntimeFixture {
        pub entered: tokio::sync::Notify,
        pub release: tokio::sync::Notify,
        scope: Mutex<Option<PreparedScope>>,
        pub snapshot: Mutex<Option<SpawnSnapshot>>,
        pub reporter: PathBuf,
    }
    impl std::fmt::Debug for RuntimeFixture {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str("RuntimeFixture")
        }
    }
    impl RuntimeFixture {
        pub fn new(root: &Path) -> (Arc<Self>, CommandSpec) {
            let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
            let registry = crate::harness::registry::resolve_document(
                &builtins,
                &crate::harness::definition::HarnessUserDocument::default(),
            )
            .unwrap();
            let harness = registry.get("opencode").unwrap();
            let mut command = harness.build_launch(root.to_str().unwrap(), None);
            let mut request =
                LaunchRequest::new(harness, InstructionIntent::NewSession, true, &command, None)
                    .unwrap();
            git2::Repository::init(root).unwrap();
            std::fs::write(root.join(".gitignore"), format!("{REPORTER_PATH}\n")).unwrap();
            let reporter = root.join(REPORTER_PATH);
            std::fs::create_dir_all(reporter.parent().unwrap()).unwrap();
            std::fs::write(&reporter, REPORTER).unwrap();
            let helper = root.join("session-metadata.py");
            std::fs::write(&helper, HELPER).unwrap();
            let executable = root.join("fixture-never-executed");
            std::fs::write(&executable, [0xcf, 0xfa, 0xed, 0xfe, 0x0c, 0, 0, 1]).unwrap();
            // Safe even while the preparation seam is absent: this fixture
            // file is never executable and cannot launch a coding harness.
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o600)).unwrap();
            command.program = executable.to_str().unwrap().to_string();
            request.command = command.clone();
            let scope = PreparedScope::from_evidence(
                request,
                reporter.clone(),
                helper,
                executable,
                "2.2.5",
                true,
                || Ok("0123456789abcdef0123456789abcdef".into()),
            )
            .unwrap()
            .unwrap();
            (
                Arc::new(Self {
                    entered: tokio::sync::Notify::new(),
                    release: tokio::sync::Notify::new(),
                    scope: Mutex::new(Some(scope)),
                    snapshot: Mutex::new(None),
                    reporter,
                }),
                command,
            )
        }
        pub async fn prepare(&self) -> Option<PreparedScope> {
            self.entered.notify_one();
            self.release.notified().await;
            self.scope.lock().unwrap().take()
        }
        pub fn capture(&self, command: &portable_pty::CommandBuilder) {
            *self.snapshot.lock().unwrap() = Some(SpawnSnapshot {
                argv: command.get_argv().clone(),
                selector: command
                    .get_env("ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID")
                    .map(|value| value.to_str().unwrap().to_string()),
            });
        }
    }
}
