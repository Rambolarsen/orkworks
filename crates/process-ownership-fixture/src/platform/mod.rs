//! Native process-ownership mechanisms used by the proof fixture.

#[cfg(unix)]
mod macos_launchd;

#[cfg(unix)]
mod unix;

#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub use unix::{
    PlatformError, ProcessGroupDomain, RegisteredRootDomain, UnixCandidate, UnixCandidateKind,
};

#[cfg(unix)]
pub use macos_launchd::LaunchdDomain;

#[cfg(windows)]
pub use windows::{
    electron_parent_arguments, run_helper_from_args, BreakawayResult, ForcedParentCompletion,
    ForcedParentReady, OwnerDomain, PlatformError,
};
