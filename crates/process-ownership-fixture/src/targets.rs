//! Closed target behaviors executed by the process-ownership fixture.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::thread;
use std::time::Duration;

use thiserror::Error;

use crate::protocol::Role;

/// Single-byte release signal accepted by the fixture's pre-target gate.
pub const RELEASE_EXEC_BYTE: u8 = 0x7f;

const TARGET_FLAG: &str = "--fixture-target";

/// Closed set of adversarial fixture target behaviors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetBehavior {
    /// PTY-shaped direct process root.
    Pty,
    /// Inference-shaped direct process root.
    Inference,
    /// Root intended to create an ordinary forked descendant.
    Forked,
    /// Root intended to create a separate process group.
    NewGroup,
    /// Root intended to exercise daemonization.
    Daemonized,
    /// Root intended to exercise reparenting.
    Reparented,
    /// Target that emits no readiness or child diagnostics.
    Silent,
}

impl TargetBehavior {
    /// Stable command-line spelling used only inside this fixture executable.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pty => "pty",
            Self::Inference => "inference",
            Self::Forked => "forked",
            Self::NewGroup => "new-group",
            Self::Daemonized => "daemonized",
            Self::Reparented => "reparented",
            Self::Silent => "silent",
        }
    }
}

impl FromStr for TargetBehavior {
    type Err = TargetError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pty" => Ok(Self::Pty),
            "inference" => Ok(Self::Inference),
            "forked" => Ok(Self::Forked),
            "new-group" => Ok(Self::NewGroup),
            "daemonized" => Ok(Self::Daemonized),
            "reparented" => Ok(Self::Reparented),
            "silent" => Ok(Self::Silent),
            _ => Err(TargetError::InvalidBehavior),
        }
    }
}

/// Invalid target command or target-side fixture failure.
#[derive(Debug, Error)]
pub enum TargetError {
    /// The fixture target arguments did not have the exact closed shape.
    #[error("invalid fixture target arguments")]
    InvalidArguments,
    /// The target role was not one of the protocol's closed roles.
    #[error("invalid fixture target role")]
    InvalidRole,
    /// The target behavior was not one of the closed fixture behaviors.
    #[error("invalid fixture target behavior")]
    InvalidBehavior,
    /// The target lifetime was not a valid millisecond count.
    #[error("invalid fixture target lifetime")]
    InvalidLifetime,
    /// The release gate closed or supplied the wrong signal.
    #[error("fixture target was not released by its supervisor")]
    ReleaseDenied,
    /// The diagnostic marker could not be written.
    #[error("fixture target marker failed")]
    Marker(#[source] io::Error),
    /// A platform-specific descendant topology could not be created.
    #[error("fixture descendant launch failed")]
    Descendant(#[source] io::Error),
}

/// Builds the exact argument vector accepted by the fixture target entry point.
#[must_use]
pub fn arguments(
    role: Role,
    behavior: TargetBehavior,
    marker: &Path,
    lifetime: Duration,
) -> Vec<OsString> {
    vec![
        OsString::from(TARGET_FLAG),
        OsString::from(role_name(role)),
        OsString::from(behavior.as_str()),
        marker.as_os_str().to_owned(),
        OsString::from(lifetime.as_millis().to_string()),
    ]
}

/// Handles the closed fixture target command line when present.
///
/// Returns `Ok(false)` for the supervisor's ordinary no-argument entry point.
/// The target behavior itself starts only after the one-byte release gate.
///
/// # Errors
///
/// Returns [`TargetError`] for malformed arguments, a closed/incorrect release
/// gate, or a marker write failure.
pub fn run_from_args(args: impl IntoIterator<Item = OsString>) -> Result<bool, TargetError> {
    let mut args = args.into_iter();
    let Some(flag) = args.next() else {
        return Ok(false);
    };
    if flag != OsStr::new(TARGET_FLAG) {
        return Err(TargetError::InvalidArguments);
    }
    let role = parse_role(args.next().ok_or(TargetError::InvalidArguments)?)?;
    let behavior = parse_behavior(args.next().ok_or(TargetError::InvalidArguments)?)?;
    let marker = PathBuf::from(args.next().ok_or(TargetError::InvalidArguments)?);
    let lifetime = parse_lifetime(args.next().ok_or(TargetError::InvalidArguments)?)?;
    if args.next().is_some() {
        return Err(TargetError::InvalidArguments);
    }

    let mut release = [0_u8; 1];
    io::stdin()
        .read_exact(&mut release)
        .map_err(|_| TargetError::ReleaseDenied)?;
    if release[0] != RELEASE_EXEC_BYTE {
        return Err(TargetError::ReleaseDenied);
    }
    run(role, behavior, &marker, lifetime)?;
    Ok(true)
}

/// Executes one closed target behavior after supervisor release.
///
/// The Task 2 target records execution and stays live for direct-root
/// observation. Platform-specific descendant/session behavior is added by the
/// native adapters in Tasks 3 and 4; this function emits no ownership evidence.
///
/// # Errors
///
/// Returns [`TargetError::Marker`] when the diagnostic marker cannot be written.
pub fn run(
    role: Role,
    behavior: TargetBehavior,
    marker: &Path,
    lifetime: Duration,
) -> Result<(), TargetError> {
    let diagnostic = format!("role={} behavior={}\n", role_name(role), behavior.as_str());
    fs::write(marker, diagnostic).map_err(TargetError::Marker)?;
    #[cfg(windows)]
    run_windows_behavior(role, behavior, marker, lifetime)?;
    thread::sleep(lifetime);
    Ok(())
}

#[cfg(windows)]
fn run_windows_behavior(
    role: Role,
    behavior: TargetBehavior,
    marker: &Path,
    lifetime: Duration,
) -> Result<(), TargetError> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;

    use windows_sys::Win32::System::Threading::{
        CREATE_BREAKAWAY_FROM_JOB, CREATE_NEW_PROCESS_GROUP,
    };

    match (role, behavior) {
        (Role::Sidecar, TargetBehavior::Forked) => {
            spawn_windows_descendant(
                Role::Pty,
                TargetBehavior::Reparented,
                &related_marker(marker, "pty"),
                lifetime,
                CREATE_NEW_PROCESS_GROUP,
            )?;
            spawn_windows_descendant(
                Role::Inference,
                TargetBehavior::Inference,
                &related_marker(marker, "inference"),
                lifetime,
                0,
            )?;
        }
        (Role::Pty, TargetBehavior::Reparented) => {
            let root = marker
                .to_string_lossy()
                .strip_suffix(".pty")
                .map(PathBuf::from)
                .unwrap_or_else(|| marker.to_path_buf());
            spawn_windows_descendant(
                Role::Inference,
                TargetBehavior::Silent,
                &related_marker(&root, "nested-inference"),
                lifetime,
                0,
            )?;
        }
        (Role::Sidecar, TargetBehavior::Daemonized) => {
            let result_marker = related_marker(marker, "breakaway");
            let escaped_marker = related_marker(marker, "escaped");
            let mut command =
                Command::new(std::env::current_exe().map_err(TargetError::Descendant)?);
            command
                .args(arguments(
                    Role::Inference,
                    TargetBehavior::Silent,
                    &escaped_marker,
                    lifetime,
                ))
                .env_clear()
                .stdin(Stdio::piped())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_BREAKAWAY_FROM_JOB);
            match command.spawn() {
                Ok(mut child) => {
                    if let Some(mut release_gate) = child.stdin.take() {
                        release_gate
                            .write_all(&[RELEASE_EXEC_BYTE])
                            .map_err(TargetError::Descendant)?;
                    }
                    fs::write(result_marker, format!("escaped:{}", child.id()))
                        .map_err(TargetError::Marker)?;
                }
                Err(error) => fs::write(
                    result_marker,
                    format!("rejected:{}", error.raw_os_error().unwrap_or_default()),
                )
                .map_err(TargetError::Marker)?,
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(windows)]
fn spawn_windows_descendant(
    role: Role,
    behavior: TargetBehavior,
    marker: &Path,
    lifetime: Duration,
    creation_flags: u32,
) -> Result<(), TargetError> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;

    let mut child = Command::new(std::env::current_exe().map_err(TargetError::Descendant)?)
        .args(arguments(role, behavior, marker, lifetime))
        .env_clear()
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(creation_flags)
        .spawn()
        .map_err(TargetError::Descendant)?;
    child
        .stdin
        .take()
        .ok_or_else(|| io::Error::other("descendant release gate unavailable"))
        .map_err(TargetError::Descendant)?
        .write_all(&[RELEASE_EXEC_BYTE])
        .map_err(TargetError::Descendant)?;
    Ok(())
}

#[cfg(windows)]
fn related_marker(root: &Path, suffix: &str) -> PathBuf {
    let mut marker = root.as_os_str().to_owned();
    marker.push(format!(".{suffix}"));
    marker.into()
}

fn parse_role(value: OsString) -> Result<Role, TargetError> {
    match value.to_str() {
        Some("sidecar") => Ok(Role::Sidecar),
        Some("pty") => Ok(Role::Pty),
        Some("inference") => Ok(Role::Inference),
        _ => Err(TargetError::InvalidRole),
    }
}

fn parse_behavior(value: OsString) -> Result<TargetBehavior, TargetError> {
    value.to_str().ok_or(TargetError::InvalidBehavior)?.parse()
}

fn parse_lifetime(value: OsString) -> Result<Duration, TargetError> {
    let milliseconds = value
        .to_str()
        .ok_or(TargetError::InvalidLifetime)?
        .parse::<u64>()
        .map_err(|_| TargetError::InvalidLifetime)?;
    Ok(Duration::from_millis(milliseconds))
}

const fn role_name(role: Role) -> &'static str {
    match role {
        Role::Sidecar => "sidecar",
        Role::Pty => "pty",
        Role::Inference => "inference",
    }
}
