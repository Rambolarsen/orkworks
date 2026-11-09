//! Fixed local recipe bookkeeping, never a native/domain execution grant.
use super::coordinator::{Charges, RouteKey};
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(usize)]
pub(in crate::runtime) enum Slot {
    Lookup,
    Reserve,
    Prepare,
    Dispatch,
    Bookkeeping,
    Reset,
    Tuple,
    Label,
    InitialClaim,
    Notify,
    FinalClaim,
    GraphAction,
    Event,
    Dispose,
}
const SLOTS: usize = Slot::Dispose as usize + 1;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum Outcome {
    Done,
    NoEffect,
    Busy,
    Notified,
    Failed,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum SlotState {
    Pending,
    InFlight,
    Returned(Outcome),
    Rejected,
    Lost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) enum GraphAction {
    Complete,
    Cancel,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::runtime) struct RecipeSnapshot {
    pub slots: [SlotState; SLOTS],
    pub admitted: [bool; SLOTS],
    pub job_live: bool,
    pub tickets: usize,
}
pub(in crate::runtime) struct Recipe {
    pub route: RouteKey,
    pub charge: Charges,
    slots: [SlotState; SLOTS],
    admitted: [bool; SLOTS],
    job_live: bool,
    tickets: usize,
    conflict: bool,
}
impl Recipe {
    pub(in crate::runtime) fn new(route: RouteKey, charge: Charges) -> Self {
        Self {
            route,
            charge,
            slots: [SlotState::Pending; SLOTS],
            admitted: [false; SLOTS],
            job_live: true,
            tickets: 0,
            conflict: false,
        }
    }
    pub(in crate::runtime) fn snapshot(&self) -> RecipeSnapshot {
        RecipeSnapshot {
            slots: self.slots,
            admitted: self.admitted,
            job_live: self.job_live,
            tickets: self.tickets,
        }
    }
    pub(in crate::runtime) fn protected(&self) -> bool {
        self.conflict
            || (self.slots[Slot::Reserve as usize] == SlotState::Returned(Outcome::Done)
                && matches!(
                    self.slots[Slot::GraphAction as usize],
                    SlotState::Rejected | SlotState::Returned(Outcome::NoEffect)
                ))
            || self.slots.iter().any(|s| {
                matches!(
                    s,
                    SlotState::Lost | SlotState::Returned(Outcome::Failed | Outcome::Unknown)
                )
            })
    }
    pub(super) fn validate_writer_admission(&self) -> Result<(), Error> {
        let dispatch = self.slots[Slot::Dispatch as usize];
        // Persist admission separately from loss: an already enqueued original
        // operation may start/finish after expiry or coordinator/ticket loss.
        if self.admitted[Slot::Dispatch as usize]
            && dispatch != SlotState::Returned(Outcome::NoEffect)
        {
            return Ok(());
        }
        Err(if dispatch == SlotState::Pending {
            Error::Pending
        } else {
            Error::Protected
        })
    }

    fn done(&self, slot: Slot) -> bool {
        matches!(
            self.slots[slot as usize],
            SlotState::Returned(Outcome::Done | Outcome::NoEffect)
        )
    }
    fn settled(&self, slot: Slot) -> bool {
        self.done(slot) || self.slots[slot as usize] == SlotState::Rejected
    }
    pub(in crate::runtime) fn retirable(&self) -> bool {
        !self.protected() && !self.job_live && self.tickets == 0
            && self.slots.iter().all(|s| matches!(s, SlotState::Returned(_) | SlotState::Rejected))
            // A committed recommendation reservation remains owed its graph action.
            && (self.slots[Slot::Reserve as usize] != SlotState::Returned(Outcome::Done)
                || self.slots[Slot::GraphAction as usize] == SlotState::Returned(Outcome::Done))
    }
    fn ready(&self, slot: Slot) -> bool {
        match slot {
            Slot::Lookup => true,
            Slot::Reserve => self.done(Slot::Lookup),
            Slot::Prepare => {
                self.slots[Slot::Reserve as usize] == SlotState::Returned(Outcome::Done)
            }
            Slot::Dispatch => self.done(Slot::Prepare),
            Slot::Bookkeeping => self.done(Slot::Dispatch),
            Slot::Reset | Slot::Tuple | Slot::Label => self.done(Slot::Bookkeeping),
            Slot::InitialClaim => {
                self.done(Slot::Bookkeeping)
                    && [Slot::Reset, Slot::Tuple, Slot::Label]
                        .iter()
                        .all(|s| self.settled(*s))
            }
            Slot::Notify => {
                self.slots[Slot::InitialClaim as usize] == SlotState::Returned(Outcome::Busy)
            }
            Slot::FinalClaim => {
                self.slots[Slot::Notify as usize] == SlotState::Returned(Outcome::Notified)
            }
            Slot::GraphAction => {
                self.slots[Slot::InitialClaim as usize] == SlotState::Returned(Outcome::Done)
                    || self.slots[Slot::FinalClaim as usize] == SlotState::Returned(Outcome::Done)
            }
            Slot::Event => {
                self.slots[Slot::GraphAction as usize] == SlotState::Returned(Outcome::Done)
            }
            Slot::Dispose => self.slots[..Slot::Dispose as usize]
                .iter()
                .all(|s| matches!(s, SlotState::Returned(_) | SlotState::Rejected)),
        }
    }
}

fn physical_ready(entry: &Entry, include_bookkeeping: bool) -> Result<(), Error> {
    if entry.protected {
        return Err(Error::Protected);
    }
    for (index, phase) in entry.phases.iter().enumerate() {
        if !include_bookkeeping && index == 3 {
            continue;
        }
        match phase {
            PhaseState::Pending => return Err(Error::Pending),
            PhaseState::Lost => return Err(Error::Protected),
            PhaseState::Settled(PhaseResult::Failed) if index != 1 => return Err(Error::Protected),
            _ => {}
        }
    }
    match entry.input_fact() {
        InputFact::FullWrite { .. } | InputFact::NoInputEffect => Ok(()),
        _ => Err(Error::Protected),
    }
}
fn graph_action(entry: &Entry) -> Result<GraphAction, Error> {
    if entry.recipe.as_ref().is_none_or(Recipe::protected) {
        return Err(Error::Protected);
    }
    // Unknown input remains protected even when local bookkeeping is still pending.
    let action = match entry.input_fact() {
        InputFact::FullWrite { .. } => GraphAction::Complete,
        InputFact::NoInputEffect => GraphAction::Cancel,
        _ => return Err(Error::Protected),
    };
    physical_ready(entry, true)?;
    let recipe = entry.recipe.as_ref().unwrap();
    if !recipe.ready(Slot::InitialClaim) {
        return Err(Error::Pending);
    }
    Ok(action)
}

/// Unique original job capability. Observers cannot construct or clone it.
pub(in crate::runtime) struct Job {
    core: Arc<Core>,
    key: Key,
}
/// One predeclared local slot. It provides no actual module/native grant.
pub(in crate::runtime) struct Ticket {
    core: Arc<Core>,
    key: Key,
    slot: Slot,
    returned: bool,
}
impl Job {
    pub(super) fn new(core: Arc<Core>, key: Key) -> Self {
        Self { core, key }
    }
    pub(in crate::runtime) fn graph_action(&self) -> Result<GraphAction, Error> {
        let mut state = self.core.lock()?;
        graph_action(Core::entry(&mut state, self.key)?)
    }
    pub(in crate::runtime) fn claim(&mut self, slot: Slot) -> Result<Ticket, Error> {
        let mut state = self.core.lock()?;
        let entry = Core::entry(&mut state, self.key)?;
        let recipe = entry.recipe.as_ref().ok_or(Error::Unsupported)?;
        if recipe.protected() {
            return Err(Error::Protected);
        }
        if entry.recipe.as_ref().unwrap().slots[slot as usize] != SlotState::Pending {
            return Err(Error::Conflict);
        }
        if Instant::now() >= entry.deadline {
            return Err(Error::Expired);
        }
        if !recipe.ready(slot) {
            return Err(Error::Pending);
        }
        if slot == Slot::Bookkeeping {
            physical_ready(entry, false)?;
        }
        if matches!(
            slot,
            Slot::InitialClaim | Slot::FinalClaim | Slot::GraphAction
        ) {
            graph_action(entry)?;
        }
        let recipe = entry.recipe.as_mut().unwrap();
        let tickets = recipe.tickets.checked_add(1).ok_or(Error::Capacity)?;
        recipe.slots[slot as usize] = SlotState::InFlight;
        recipe.admitted[slot as usize] = true;
        recipe.tickets = tickets;
        Ok(Ticket {
            core: self.core.clone(),
            key: self.key,
            slot,
            returned: false,
        })
    }
    /// Permanently refuse an unstarted slot, never revoke an in-flight effect.
    pub(in crate::runtime) fn reject(&mut self, slot: Slot) -> Result<(), Error> {
        let mut state = self.core.lock()?;
        let recipe = Core::entry(&mut state, self.key)?
            .recipe
            .as_mut()
            .ok_or(Error::Unsupported)?;
        if recipe.slots[slot as usize] != SlotState::Pending {
            return Err(Error::Conflict);
        }
        recipe.slots[slot as usize] = SlotState::Rejected;
        Ok(())
    }
}
impl Drop for Job {
    fn drop(&mut self) {
        if let Ok(mut state) = self.core.lock() {
            if let Ok(entry) = Core::entry(&mut state, self.key) {
                if let Some(recipe) = entry.recipe.as_mut() {
                    recipe.job_live = false;
                    for slot in &mut recipe.slots {
                        if *slot == SlotState::Pending {
                            *slot = SlotState::Lost;
                        }
                    }
                }
            }
        }
    }
}
impl Ticket {
    // Actual producer completion adapters are intentionally unavailable. Only
    // local cfg(test) transitions can supply absent producer return values.
    #[cfg(test)]
    pub(in crate::runtime) fn fixture_finish(mut self, outcome: Outcome) -> Result<(), Error> {
        {
            let mut state = self.core.lock()?;
            let recipe = Core::entry(&mut state, self.key)?
                .recipe
                .as_mut()
                .ok_or(Error::Unsupported)?;
            let valid = match outcome {
                Outcome::Busy => matches!(self.slot, Slot::InitialClaim | Slot::FinalClaim),
                Outcome::Notified => self.slot == Slot::Notify,
                Outcome::Done | Outcome::NoEffect => self.slot != Slot::Notify,
                Outcome::Failed | Outcome::Unknown => true,
            };
            if !valid || recipe.slots[self.slot as usize] != SlotState::InFlight {
                recipe.conflict = true;
                return Err(Error::Conflict);
            }
            recipe.slots[self.slot as usize] = SlotState::Returned(outcome);
            recipe.tickets -= 1;
            if self.slot == Slot::FinalClaim && outcome == Outcome::Busy {
                recipe.conflict = true;
            }
            self.returned = true;
        }
        Ok(())
    }
}
impl Drop for Ticket {
    fn drop(&mut self) {
        if self.returned {
            return;
        }
        if let Ok(mut state) = self.core.lock() {
            if let Ok(entry) = Core::entry(&mut state, self.key) {
                if let Some(recipe) = entry.recipe.as_mut() {
                    if recipe.slots[self.slot as usize] == SlotState::InFlight {
                        recipe.slots[self.slot as usize] = SlotState::Lost;
                        recipe.tickets -= 1;
                    }
                }
            }
        }
    }
}
