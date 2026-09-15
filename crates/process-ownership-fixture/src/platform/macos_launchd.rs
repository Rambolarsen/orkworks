//! macOS launchd control candidate.

use std::io;
use std::path::Path;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use crate::observation::ObservationSnapshot;
use crate::protocol::GenerationId;
use crate::supervisor::{LaunchSpec, PausedRoot};

use super::unix::PlatformError;

const COMPLETION_TIMEOUT: Duration = Duration::from_secs(5);

/// Temporary per-generation launchd job control.
#[derive(Debug)]
pub struct LaunchdDomain {
    generation: GenerationId,
    label: Option<String>,
    domain: Option<String>,
}

impl LaunchdDomain {
    pub(crate) const fn unsupported(generation: GenerationId) -> Self {
        Self {
            generation,
            label: None,
            domain: None,
        }
    }

    /// Bootstraps a temporary launchd job from a caller-created plist.
    ///
    /// The plist must already contain the fixture's verified executable and
    /// generation-specific label. The fixture never asks launchd to infer
    /// ownership from a PID or pathname.
    pub fn bootstrap(label: impl Into<String>, plist: &Path) -> Result<Self, PlatformError> {
        #[cfg(target_os = "macos")]
        {
            let label = label.into();
            validate_label(&label)?;
            if !plist.is_file() {
                return Err(PlatformError::Io {
                    operation: "validate launchd plist",
                    source: io::Error::new(io::ErrorKind::NotFound, "launchd plist is missing"),
                });
            }
            let domain = format!("gui/{}", unsafe { libc::geteuid() });
            run_launchctl(["bootstrap", &domain, &plist.to_string_lossy()])?;
            Ok(Self {
                generation: 0,
                label: Some(label),
                domain: Some(domain),
            })
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = (label.into(), plist);
            Err(PlatformError::UnsupportedPlatform {
                candidate: "launchd",
            })
        }
    }

    /// Returns the generation associated with this launchd candidate.
    #[must_use]
    pub const fn generation(&self) -> GenerationId {
        self.generation
    }

    /// Launchd cannot use the fixture's portable paused-root protocol.
    pub fn launch_paused(&mut self, _spec: LaunchSpec) -> Result<PausedRoot, PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            candidate: "launchd paused fixture",
        })
    }

    /// Returns unresolved until launchd has been bootstrapped and queried.
    pub fn observe(&mut self) -> Result<ObservationSnapshot, PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            candidate: "launchd observation",
        })
    }

    /// Stops the per-generation job and waits until launchd no longer reports it.
    pub fn kill_and_wait(&mut self) -> Result<(), PlatformError> {
        #[cfg(target_os = "macos")]
        {
            let domain = self
                .domain
                .as_deref()
                .ok_or(PlatformError::UnsupportedPlatform {
                    candidate: "launchd",
                })?;
            let label = self
                .label
                .as_deref()
                .ok_or(PlatformError::UnsupportedPlatform {
                    candidate: "launchd",
                })?;
            let target = format!("{domain}/{label}");
            run_launchctl(["kill", "SIGKILL", &target])?;
            let deadline = Instant::now() + COMPLETION_TIMEOUT;
            while Instant::now() < deadline {
                let status = Command::new("launchctl")
                    .args(["print", &target])
                    .status()
                    .map_err(|source| PlatformError::Io {
                        operation: "query launchd job completion",
                        source,
                    })?;
                if !status.success() {
                    self.label = None;
                    self.domain = None;
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(PlatformError::CompletionTimeout)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Err(PlatformError::UnsupportedPlatform {
                candidate: "launchd",
            })
        }
    }
}

#[cfg(target_os = "macos")]
fn validate_label(label: &str) -> Result<(), PlatformError> {
    if label.is_empty() || label.len() > 128 || label.contains('/') || label.contains('\0') {
        return Err(PlatformError::Io {
            operation: "validate launchd label",
            source: io::Error::new(io::ErrorKind::InvalidInput, "invalid launchd label"),
        });
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_launchctl<const N: usize>(args: [&str; N]) -> Result<(), PlatformError> {
    let status = Command::new("launchctl")
        .args(args)
        .status()
        .map_err(|source| PlatformError::Io {
            operation: "invoke launchctl",
            source,
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(PlatformError::Io {
            operation: "launchctl reported failure",
            source: io::Error::other(format!("exit status {status}")),
        })
    }
}
