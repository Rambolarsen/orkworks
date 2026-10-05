//! Private native connection. Eligibility precedes any child or direct-launch augmentation.
use super::codex_approval::NativeStatus;
use crate::harness::{
    probe_cache::{VersionProbeCache, VersionProbeEpoch},
    CommandSpec,
};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
mod launch;
mod protocol;

// Shipping entries require independent production evidence, never a diagnostic spike.
const VERIFIED: &[CompatibilityRecord] = &[];
#[derive(Clone, Copy)]
struct CompatibilityRecord {
    version: &'static str,
    os: &'static str,
    arch: &'static str,
    user_agent_prefix: &'static str,
    protocol: &'static str,
    root_proof: &'static str,
    evidence: &'static str,
}

pub(crate) struct NativeObservation {
    pub(crate) status: NativeStatus,
    pub(crate) complete_singleton_root: bool,
    pub(crate) started_at: Instant,
}

/// Contains only fixed reason codes. Never attach source errors or wire data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeError {
    Disconnected,
    Authentication,
    Shape,
    UnsupportedRequest,
    Limit,
    Timeout,
    Stale,
    Root,
    Unavailable,
}
impl std::fmt::Display for NativeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Disconnected => "native-disconnected",
            Self::Authentication => "native-authentication",
            Self::Shape => "native-shape",
            Self::UnsupportedRequest => "native-unsupported-request",
            Self::Limit => "native-limit",
            Self::Timeout => "native-timeout",
            Self::Stale => "native-stale",
            Self::Root => "native-root",
            Self::Unavailable => "native-unavailable",
        })
    }
}
impl std::error::Error for NativeError {}

struct Route {
    shared: Vec<String>,
    resume: Option<String>,
}
fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && !value.starts_with('-')
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
fn parse_arguments(args: &[String]) -> Option<Route> {
    let mut shared = Vec::new();
    let mut resume = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-c" | "--config" | "--enable" | "--disable" => {
                let value = args.get(i + 1)?;
                if value.is_empty() || value.starts_with('-') || value.contains('\0') {
                    return None;
                }
                shared.extend_from_slice(&args[i..i + 2]);
                i += 2;
            }
            "resume" if resume.is_none() => {
                let id = args.get(i + 1)?;
                if !valid_id(id) {
                    return None;
                }
                resume = Some(id.clone());
                i += 2;
            }
            _ => return None,
        }
    }
    Some(Route { shared, resume })
}

pub(crate) struct NativeLaunchPlan {
    executable: PathBuf,
    configured_program: String,
    identity: launch::ExecutableIdentity,
    epoch: VersionProbeEpoch,
    cwd: PathBuf,
    route: Route,
    record: CompatibilityRecord,
}
impl NativeLaunchPlan {
    pub(crate) async fn start(
        self,
        execution_env: &[(String, String)],
    ) -> Result<OwnedNativeRuntime, String> {
        launch::start(self, execution_env)
            .await
            .map_err(|e| e.to_string())
    }
    fn revalidate(&self) -> Result<(), NativeError> {
        if !startup_current(
            &self.configured_program,
            &self.executable,
            &self.identity,
            &self.epoch,
        ) {
            return Err(NativeError::Unavailable);
        }
        Ok(())
    }
}

pub(crate) async fn eligible(
    command: &CommandSpec,
    cache: &VersionProbeCache,
) -> Result<Option<NativeLaunchPlan>, String> {
    eligible_with_records(command, cache, VERIFIED).await
}
async fn eligible_with_records(
    command: &CommandSpec,
    cache: &VersionProbeCache,
    records: &[CompatibilityRecord],
) -> Result<Option<NativeLaunchPlan>, String> {
    let Some(route) = parse_arguments(&command.args) else {
        return Ok(None);
    };
    // Windows stays on the direct path until suspended Job ownership has been verified.
    if !cfg!(unix)
        || !records
            .iter()
            .any(|r| r.os == std::env::consts::OS && r.arch == std::env::consts::ARCH)
    {
        return Ok(None);
    }
    let Some(executable) = launch::resolve(&command.program) else {
        return Ok(None);
    };
    if !launch::canonical_binary(&executable) {
        return Ok(None);
    }
    let Some(identity) = launch::ExecutableIdentity::read(&executable) else {
        return Ok(None);
    };
    let epoch = cache.epoch();
    let Some(record) = probe_record(command, cache, records, &executable, &identity, &epoch).await
    else {
        return Ok(None);
    };
    let plan = NativeLaunchPlan {
        executable,
        configured_program: command.program.clone(),
        identity,
        epoch,
        cwd: PathBuf::from(&command.cwd),
        route,
        record,
    };
    if plan.revalidate().is_err() {
        return Ok(None);
    }
    Ok(Some(plan))
}

fn startup_current(
    configured: &str,
    executable: &Path,
    identity: &launch::ExecutableIdentity,
    epoch: &VersionProbeEpoch,
) -> bool {
    epoch.is_current()
        && launch::resolve(configured).as_deref() == Some(executable)
        && launch::ExecutableIdentity::read(executable).as_ref() == Some(identity)
}
// Shared probe sequence; production enters only after canonical binary resolution.
async fn probe_record(
    command: &CommandSpec,
    cache: &VersionProbeCache,
    records: &[CompatibilityRecord],
    executable: &Path,
    identity: &launch::ExecutableIdentity,
    epoch: &VersionProbeEpoch,
) -> Option<CompatibilityRecord> {
    let current = || startup_current(&command.program, executable, identity, epoch);
    if !current() {
        return None;
    }
    let version = launch::version_fenced(executable, &command.cwd, identity, cache, &current).await;
    if !current() {
        return None;
    }
    let record = records
        .iter()
        .find(|r| {
            Some(r.version) == version.as_deref()
                && r.os == std::env::consts::OS
                && r.arch == std::env::consts::ARCH
                && r.protocol == "v2-thread-status-0.160"
                && !r.root_proof.is_empty()
                && !r.evidence.is_empty()
        })
        .copied()?;
    if !launch::features_fenced(executable, &command.cwd, &current).await || !current() {
        return None;
    }
    Some(record)
}

/// No Debug/Serialize: owns the memory-only TUI capability and authenticated client.
pub(crate) struct OwnedNativeRuntime {
    plan: NativeLaunchPlan,
    server: launch::OwnedProcess,
    endpoint: String,
    token: launch::Capability,
    tui_args: Vec<String>,
    observer: Option<protocol::Client>,
    retry_at: Instant,
    backoff: Duration,
}
impl OwnedNativeRuntime {
    pub(crate) fn tui_args(&self) -> &[String] {
        &self.tui_args
    }
    pub(crate) fn tui_environment(&self) -> (&str, &str) {
        (launch::TOKEN_ENV, self.token.value())
    }
    pub(crate) fn resolved_executable(&self) -> &Path {
        &self.plan.executable
    }
    pub(crate) fn revalidate_tui_executable(&self) -> Result<(), String> {
        self.plan.revalidate().map_err(|e| e.to_string())
    }
    pub(crate) fn is_alive(&self) -> bool {
        self.server.is_alive()
    }
    pub(crate) async fn observe(&mut self, root: &str) -> Result<NativeObservation, NativeError> {
        if !valid_id(root) || !self.is_alive() {
            self.observer = None;
            return Err(NativeError::Unavailable);
        }
        if self.observer.is_none() {
            if Instant::now() < self.retry_at {
                return Err(NativeError::Disconnected);
            }
            match protocol::Client::connect(&self.endpoint, self.token.value(), self.plan.record)
                .await
            {
                Ok(client) => {
                    self.observer = Some(client);
                    self.backoff = Duration::from_millis(250);
                }
                Err(error) => {
                    self.retry_at = Instant::now() + self.backoff;
                    self.backoff = (self.backoff * 2).min(Duration::from_secs(2));
                    return Err(error);
                }
            }
            // Reconnection is always surfaced before any snapshot can establish a new baseline.
            return Err(NativeError::Disconnected);
        }
        let result = self
            .observer
            .as_mut()
            .ok_or(NativeError::Disconnected)?
            .observe(root)
            .await;
        if result.is_err() {
            self.observer = None;
            self.retry_at = Instant::now() + self.backoff;
        }
        result
    }
    pub(crate) async fn shutdown(&mut self) {
        self.observer = None;
        self.server.shutdown().await;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn absent_verified_versions_platforms_options_and_wrappers_keep_direct_launch_unchanged()
    {
        let cache = VersionProbeCache::new();
        let command = CommandSpec {
            program: "codex".into(),
            args: vec![],
            cwd: "/private/tmp".into(),
        };
        assert!(eligible(&command, &cache).await.unwrap().is_none());
        let foreign = CompatibilityRecord {
            version: "0.160.0",
            os: "unsupported-os",
            arch: std::env::consts::ARCH,
            user_agent_prefix: "fixture",
            protocol: "v2-thread-status-0.160",
            root_proof: "fixture",
            evidence: "fixture",
        };
        assert!(eligible_with_records(&command, &cache, &[foreign])
            .await
            .unwrap()
            .is_none());
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("codex");
        std::fs::write(&executable, "#!/bin/sh\nexit 99\n").unwrap();
        crate::test_support::make_test_executable(&executable);
        let wrapper = CommandSpec {
            program: executable.display().to_string(),
            args: vec![],
            cwd: dir.path().display().to_string(),
        };
        let matching = CompatibilityRecord {
            os: std::env::consts::OS,
            ..foreign
        };
        assert!(eligible_with_records(&wrapper, &cache, &[matching])
            .await
            .unwrap()
            .is_none());
        assert_eq!(command.args, Vec::<String>::new()); // Metadata model is not a launch input and is never synthesized.
        for args in [
            vec!["--model", "selected"],
            vec!["resume", "id", "--profile", "custom"],
            vec!["--no-daemon"],
            vec!["--sandbox", "workspace-write"],
        ] {
            let supplied = CommandSpec {
                program: "/definitely/missing/codex".into(),
                args: args.into_iter().map(String::from).collect(),
                cwd: command.cwd.clone(),
            };
            assert!(eligible_with_records(&supplied, &cache, &[matching])
                .await
                .unwrap()
                .is_none());
        }
    }
    #[test]
    fn accepts_only_complete_mapped_routes_and_preserves_order() {
        let args = [
            "-c",
            "model=\"configured\"",
            "--enable",
            "feature",
            "resume",
            "saved-id",
            "--disable",
            "other",
        ]
        .map(String::from);
        let route = parse_arguments(&args).expect("mapped route");
        assert_eq!(
            route.shared,
            [
                "-c",
                "model=\"configured\"",
                "--enable",
                "feature",
                "--disable",
                "other"
            ]
        );
        assert_eq!(route.resume.as_deref(), Some("saved-id"));
        assert!(parse_arguments(&[]).is_some());
        for invalid in [
            vec!["--model", "x"],
            vec!["--sandbox", "workspace-write"],
            vec!["--remote", "ws://localhost"],
            vec!["resume", "--last"],
            vec!["resume", "id", "prompt"],
            vec!["--"],
            vec!["-c"],
            vec!["--enable", ""],
            vec!["prompt"],
            vec!["-p", "profile"],
        ] {
            assert!(
                parse_arguments(&invalid.into_iter().map(String::from).collect::<Vec<_>>())
                    .is_none()
            );
        }
    }
}
