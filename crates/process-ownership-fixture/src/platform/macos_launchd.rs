//! macOS launchd control candidate.

use std::path::Path;

use crate::observation::ObservationSnapshot;
use crate::protocol::GenerationId;
use crate::supervisor::{LaunchSpec, PausedRoot};

use super::unix::PlatformError;

/// Explicitly unsupported launchd candidate for this fixture.
#[derive(Debug)]
pub struct LaunchdDomain {
    generation: GenerationId,
}

impl LaunchdDomain {
    /// Reports that launchd is not enabled by this fixture.
    ///
    /// No launchctl command is issued. A generation-bound bootstrap/bootout
    /// lifecycle remains an unresolved candidate until it has native tests.
    pub fn bootstrap(label: impl Into<String>, plist: &Path) -> Result<Self, PlatformError> {
        let _ = (label.into(), plist);
        Err(PlatformError::UnsupportedPlatform {
            candidate: "launchd",
        })
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

    /// Reports unsupported; no launchd observation is implemented.
    pub fn observe(&mut self) -> Result<ObservationSnapshot, PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            candidate: "launchd observation",
        })
    }

    /// Reports that launchd cleanup is not enabled by this fixture.
    pub fn kill_and_wait(&mut self) -> Result<(), PlatformError> {
        Err(PlatformError::UnsupportedPlatform {
            candidate: "launchd",
        })
    }
}
