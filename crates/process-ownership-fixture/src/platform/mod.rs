//! Native process-ownership mechanisms used by the proof fixture.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{
    electron_parent_arguments, run_helper_from_args, BreakawayResult, ForcedParentCompletion,
    ForcedParentReady, OwnerDomain, PlatformError,
};
