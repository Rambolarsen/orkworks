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

/// Complete observation attempt for one prepared supervisor generation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationSnapshot {
    /// Generation whose registered set was observed.
    pub generation: GenerationId,
    /// Every root admitted by this supervisor, including roots already exited.
    pub roots: Vec<ProcessObservation>,
    /// Descendants found independently by the active native adapter.
    pub descendants: Vec<ProcessObservation>,
    /// Registered identities whose survival or exit remains ambiguous.
    pub unresolved_survivors: Vec<NativeIdentity>,
}

/// Returns true only after every registered identity has exited with no ambiguity.
#[must_use]
pub fn is_complete(snapshot: &ObservationSnapshot) -> bool {
    snapshot.unresolved_survivors.is_empty()
        && snapshot
            .roots
            .iter()
            .chain(&snapshot.descendants)
            .all(|process| {
                process.containment_membership == ContainmentMembership::Confirmed
                    && process.exit_state == ExitState::Exited
            })
}
