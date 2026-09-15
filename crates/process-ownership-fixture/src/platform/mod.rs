//! Native process-ownership mechanisms used by the proof fixture.

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{BreakawayResult, OwnerDomain, PlatformError};
