//! Supervisor-owned process observations used by the proof fixture.

use crate::protocol::{GenerationId, NativeIdentity};

/// Independently observed liveness for one registered process identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitState {
    /// The registered native identity is still live.
    Running,
    /// The retained process handle independently reported exit.
    Exited,
    /// Liveness or birth identity could not be established safely.
    Unresolved,
}

/// Membership evidence reported by the active platform adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainmentMembership {
    /// The root passed the adapter's containment attachment step.
    ///
    /// For the Task 2 host adapter this proves only the common admission sequence;
    /// Tasks 3 and 4 replace it with native platform containment evidence.
    Confirmed,
    /// The adapter could not establish containment membership.
    Unresolved,
}

/// One supervisor/OS observation of a registered root or descendant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessObservation {
    /// Native birth identity retained at admission.
    pub identity: NativeIdentity,
    /// Adapter-reported containment membership.
    pub containment_membership: ContainmentMembership,
    /// Independently observed process liveness.
    pub exit_state: ExitState,
}

/// Handle-backed observation for a root whose native birth identity was unavailable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnidentifiedProcessObservation {
    /// PID exposed only to diagnose the retained process handle.
    pub diagnostic_pid: u32,
    /// Conservative handle-backed liveness; a live unidentified root is unresolved.
    pub exit_state: ExitState,
}

/// Diagnostic from the existing bounded descendant observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DescendantObservationFailure {
    /// The observation budget expired.
    Deadline,
    /// The retained owner state could not be inspected.
    OwnerState,
    /// The Job Object process listing failed, with its Win32 code if available.
    JobCensus(Option<i32>),
    /// The Job Object process listing exceeded fixture bounds.
    CensusOverflow,
    /// A census entry could not be opened, with its Win32 code if available.
    OpenProcess(Option<i32>),
    /// The membership query failed, with its Win32 code if available.
    JobMembership(Option<i32>),
    /// The opened process was not in the retained Job Object.
    NotInJob,
    /// The nonblocking liveness check failed, with its Win32 code if available.
    Wait(Option<i32>),
    /// The liveness check did not report that the process was still running.
    NotRunning {
        /// Census entry whose nonblocking liveness check did not report running.
        diagnostic_pid: u32,
        /// PID-only hint for correlating this failure with the snapshot's roots.
        /// This does not establish native birth identity or cleanup authority.
        matches_registered_pid: bool,
    },
    /// Native identity capture failed, with its Win32 code if available.
    CaptureIdentity(Option<i32>),
}

/// Complete observation attempt for one prepared supervisor generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationSnapshot {
    /// Generation whose registered set was observed.
    pub generation: GenerationId,
    /// Every root admitted by this supervisor, including roots already exited.
    pub roots: Vec<ProcessObservation>,
    /// Descendants found independently by the active native adapter.
    pub descendants: Vec<ProcessObservation>,
    /// Roots retained after native identity capture could not complete safely.
    pub unidentified_roots: Vec<UnidentifiedProcessObservation>,
    /// Registered identities whose survival or exit remains ambiguous.
    pub unresolved_survivors: Vec<NativeIdentity>,
    /// Whether the platform adapter could not complete its ownership census.
    pub observation_unresolved: bool,
    /// Failure detail from this snapshot’s descendant census, when available.
    pub descendant_observation_failure: Option<DescendantObservationFailure>,
}

/// Returns true only after identified processes exited and no unidentified root remains.
#[must_use]
pub fn is_complete(snapshot: &ObservationSnapshot) -> bool {
    !snapshot.observation_unresolved
        && snapshot.unidentified_roots.is_empty()
        && snapshot.unresolved_survivors.is_empty()
        && snapshot
            .roots
            .iter()
            .chain(&snapshot.descendants)
            .all(|process| {
                process.containment_membership == ContainmentMembership::Confirmed
                    && process.exit_state == ExitState::Exited
            })
}
