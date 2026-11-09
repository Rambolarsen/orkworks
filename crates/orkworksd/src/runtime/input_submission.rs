//! Production starts remain unreported; a private optional writer bridge shares
//! fixed facts and unique reporters. Owner has no production caller, input capability,
//! payload/native handle, physical-completion proof or production resource profile.
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Instant;

#[derive(Clone, Copy)]
pub(super) struct Limits {
    pub records: usize,
    pub observers: usize,
    /// Declared context only: this module does not retain the input payload.
    pub input_bytes: usize,
    /// Actual vector backing capacity, excluding fixed allocator/Arc overhead.
    pub backing_bytes: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Context {
    pub root: u128,
    pub session: u128,
    pub incarnation: u64,
    pub slot: u64,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Flush {
    Succeeded,
    Failed,
    Unknown,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum InputFact {
    Unresolved,
    NoInputEffect,
    FullWrite { flush: Flush },
    PartialOrUnknown { reported: usize },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PhaseResult {
    Completed,
    NoEffect,
    Failed,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PhaseState {
    Pending,
    Settled(PhaseResult),
    Lost,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Error {
    InvalidLimits,
    TooLarge,
    Capacity,
    Duplicate,
    Expired,
    Stale,
    WrongRole,
    InvalidFact,
    Conflict,
    Protected,
    Pending,
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Snapshot {
    pub context: Context,
    pub input_bytes: usize,
    pub fact: InputFact,
    pub phases: [PhaseState; 4],
    pub protected: bool,
    pub expired: bool,
}

struct Entry {
    serial: u64,
    context: Context,
    input_bytes: usize,
    deadline: Instant,
    input: Option<InputFact>,
    checkpoint: Option<usize>,
    phases: [PhaseState; 4],
    protected: bool,
    live_reporters: usize,
}
impl Entry {
    fn input_fact(&self) -> InputFact {
        self.input.unwrap_or_else(|| match self.checkpoint {
            Some(reported) if reported == self.input_bytes => InputFact::FullWrite {
                flush: Flush::Unknown,
            },
            Some(reported) => InputFact::PartialOrUnknown { reported },
            None => InputFact::Unresolved,
        })
    }
    fn snapshot(&self, now: Instant) -> Snapshot {
        Snapshot {
            context: self.context,
            input_bytes: self.input_bytes,
            fact: self.input_fact(),
            phases: self.phases,
            protected: self.protected
                || self.input == Some(InputFact::Unresolved)
                || matches!(self.input_fact(), InputFact::PartialOrUnknown { .. })
                || self.phases.iter().any(|phase| {
                    matches!(
                        phase,
                        PhaseState::Lost | PhaseState::Settled(PhaseResult::Failed)
                    )
                }),
            expired: now >= self.deadline,
        }
    }
}
struct State {
    slots: Vec<Option<Entry>>,
    serial: u64,
    observers: usize,
}
struct Core {
    limits: Limits,
    state: Mutex<State>,
}
impl Core {
    fn lock(&self) -> Result<MutexGuard<'_, State>, Error> {
        self.state.lock().map_err(|_| Error::Unavailable)
    }
    fn entry<'a>(state: &'a mut State, key: Key) -> Result<&'a mut Entry, Error> {
        state
            .slots
            .get_mut(key.index)
            .and_then(Option::as_mut)
            .filter(|entry| entry.serial == key.serial)
            .ok_or(Error::Stale)
    }
}
#[derive(Clone, Copy, PartialEq, Eq)]
struct Key {
    index: usize,
    serial: u64,
}

pub(super) struct Owner {
    core: Arc<Core>,
}
pub(super) struct Submission {
    /// Preparation, writer, physical return, bookkeeping: fixed original roles.
    pub reporters: [Reporter; 4],
    pub observer: Observer,
}
/// Not Clone: each phase has exactly one reporting owner.
pub(super) struct Reporter {
    core: Arc<Core>,
    key: Key,
    phase: usize,
}
/// Not Clone: additional observers must reserve through Owner::observe.
pub(super) struct Observer {
    core: Arc<Core>,
    key: Key,
}

impl Owner {
    pub(super) fn new(limits: Limits) -> Result<Self, Error> {
        let bytes = limits
            .records
            .checked_mul(std::mem::size_of::<Option<Entry>>())
            .ok_or(Error::InvalidLimits)?;
        if limits.records == 0
            || limits.observers == 0
            || limits.input_bytes == 0
            || limits.backing_bytes == 0
            || bytes > limits.backing_bytes
            || limits.records.checked_mul(4).is_none()
        {
            return Err(Error::InvalidLimits);
        }
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(limits.records)
            .map_err(|_| Error::Unavailable)?;
        let actual = slots
            .capacity()
            .checked_mul(std::mem::size_of::<Option<Entry>>())
            .ok_or(Error::InvalidLimits)?;
        if actual > limits.backing_bytes {
            return Err(Error::InvalidLimits);
        }
        slots.resize_with(limits.records, || None);
        Ok(Self {
            core: Arc::new(Core {
                limits,
                state: Mutex::new(State {
                    slots,
                    serial: 0,
                    observers: 0,
                }),
            }),
        })
    }

    pub(super) fn admit(
        &self,
        context: Context,
        input_bytes: usize,
        deadline: Instant,
        now: Instant,
    ) -> Result<Submission, Error> {
        if now >= deadline {
            return Err(Error::Expired);
        }
        if input_bytes > self.core.limits.input_bytes {
            return Err(Error::TooLarge);
        }
        let mut state = self.core.lock()?;
        if state
            .slots
            .iter()
            .flatten()
            .any(|entry| entry.context == context)
        {
            return Err(Error::Duplicate);
        }
        if state.observers >= self.core.limits.observers {
            return Err(Error::Capacity);
        }
        let index = state
            .slots
            .iter()
            .position(Option::is_none)
            .ok_or(Error::Capacity)?;
        // Caller time may predate lock contention or a scheduler pause. Recheck
        // at the actual transition before reserving a record or handles.
        if Instant::now() >= deadline {
            return Err(Error::Expired);
        }
        let serial = state.serial.checked_add(1).ok_or(Error::Capacity)?;
        state.serial = serial;
        state.observers += 1;
        state.slots[index] = Some(Entry {
            serial,
            context,
            input_bytes,
            deadline,
            input: None,
            checkpoint: None,
            phases: [PhaseState::Pending; 4],
            protected: false,
            live_reporters: 4,
        });
        let key = Key { index, serial };
        Ok(Submission {
            reporters: std::array::from_fn(|phase| Reporter {
                core: self.core.clone(),
                key,
                phase,
            }),
            observer: Observer {
                core: self.core.clone(),
                key,
            },
        })
    }

    pub(super) fn observe(&self, context: Context) -> Result<Observer, Error> {
        let mut state = self.core.lock()?;
        if state.observers >= self.core.limits.observers {
            return Err(Error::Capacity);
        }
        let (index, entry) = state
            .slots
            .iter()
            .enumerate()
            .find_map(|(index, entry)| {
                entry
                    .as_ref()
                    .filter(|e| e.context == context)
                    .map(|entry| (index, entry))
            })
            .ok_or(Error::Stale)?;
        let key = Key {
            index,
            serial: entry.serial,
        };
        state.observers += 1;
        Ok(Observer {
            core: self.core.clone(),
            key,
        })
    }

    /// Consume the retained result only after all original obligations/handles settle.
    /// There is deliberately no unknown-resolution or force-clear operation.
    pub(super) fn retire(&self, observer: Observer) -> Result<Snapshot, Error> {
        if !Arc::ptr_eq(&self.core, &observer.core) {
            return Err(Error::Stale);
        }
        let result = {
            let mut state = self.core.lock()?;
            let entry = Core::entry(&mut state, observer.key)?;
            let snapshot = entry.snapshot(Instant::now());
            if snapshot.protected
                || matches!(
                    snapshot.fact,
                    InputFact::Unresolved | InputFact::PartialOrUnknown { .. }
                )
            {
                return Err(Error::Protected);
            }
            if entry.input.is_none()
                || entry.live_reporters != 0
                || entry.phases.iter().any(|p| *p == PhaseState::Pending)
            {
                return Err(Error::Pending);
            }
            state.slots[observer.key.index] = None;
            Ok(snapshot)
        };
        // Never run Observer::drop while holding the shared transition lock.
        drop(observer);
        result
    }
}

impl Observer {
    pub(super) fn snapshot(&self, now: Instant) -> Result<Snapshot, Error> {
        let mut state = self.core.lock()?;
        Ok(Core::entry(&mut state, self.key)?.snapshot(now))
    }
}
impl Drop for Observer {
    fn drop(&mut self) {
        if let Ok(mut state) = self.core.state.lock() {
            state.observers -= 1;
        }
    }
}
impl Reporter {
    /// Association only: these handles grant no native input authority.
    pub(super) fn validate_pair(&self, returned: &Self, input_bytes: usize) -> Result<(), Error> {
        if self.phase != 1 || returned.phase != 2 {
            return Err(Error::WrongRole);
        }
        if !Arc::ptr_eq(&self.core, &returned.core) || self.key != returned.key {
            return Err(Error::Stale);
        }
        let mut state = self.core.lock()?;
        let entry = Core::entry(&mut state, self.key)?;
        if entry.input_bytes != input_bytes {
            return Err(Error::InvalidFact);
        }
        if entry.protected {
            return Err(Error::Protected);
        }
        if entry.phases[..3].contains(&PhaseState::Settled(PhaseResult::NoEffect)) {
            return Err(Error::Conflict);
        }
        if entry.input.is_some()
            || entry.checkpoint.is_some()
            || entry.phases[1] != PhaseState::Pending
            || entry.phases[2] != PhaseState::Pending
        {
            return Err(Error::Conflict);
        }
        Ok(())
    }

    pub(super) fn checkpoint(&self, reported: usize) -> Result<(), Error> {
        let mut state = self.core.lock()?;
        let entry = Core::entry(&mut state, self.key)?;
        if self.phase != 1 {
            entry.protected = true;
            return Err(Error::WrongRole);
        }
        if entry.input.is_some() || entry.phases[1] != PhaseState::Pending {
            entry.protected = true;
            return Err(Error::Conflict);
        }
        if reported > entry.input_bytes
            || (reported == 0 && entry.input_bytes != 0)
            || entry.checkpoint.is_some_and(|previous| reported < previous)
        {
            entry.protected = true;
            return Err(Error::InvalidFact);
        }
        entry.checkpoint = Some(reported);
        if entry.phases[..3].contains(&PhaseState::Settled(PhaseResult::NoEffect)) {
            entry.protected = true;
            return Err(Error::Conflict);
        }
        Ok(())
    }

    pub(super) fn record_input(&self, fact: InputFact) -> Result<(), Error> {
        let mut state = self.core.lock()?;
        let entry = Core::entry(&mut state, self.key)?;
        if self.phase != 1 && !(self.phase == 0 && fact == InputFact::NoInputEffect) {
            entry.protected = true;
            return Err(Error::WrongRole);
        }
        if matches!(fact, InputFact::PartialOrUnknown { reported } if reported > entry.input_bytes)
        {
            entry.protected = true;
            return Err(Error::InvalidFact);
        }
        if entry.input == Some(fact) {
            return Ok(());
        }
        if entry.input.is_some() {
            entry.protected = true;
            return Err(Error::Conflict);
        }
        if let Some(prefix) = entry.checkpoint {
            let preserves = match fact {
                InputFact::FullWrite { .. } => true,
                InputFact::PartialOrUnknown { reported } => {
                    prefix < entry.input_bytes && reported >= prefix
                }
                _ => false,
            };
            if !preserves {
                entry.protected = true;
                return Err(Error::Conflict);
            }
        }
        let contradicts_no_effect = matches!(
            fact,
            InputFact::FullWrite { .. } | InputFact::PartialOrUnknown { .. }
        ) && entry.phases[..3]
            .contains(&PhaseState::Settled(PhaseResult::NoEffect));
        if entry.phases[self.phase] != PhaseState::Pending || contradicts_no_effect {
            // Keep the new direct writer fact when no earlier input fact exists,
            // but never permit contradictory physical claims to retire the record.
            entry.input = Some(fact);
            entry.protected = true;
            return Err(Error::Conflict);
        }
        entry.input = Some(fact);
        Ok(())
    }
    pub(super) fn finish(&self, result: PhaseResult) -> Result<(), Error> {
        let mut state = self.core.lock()?;
        let entry = Core::entry(&mut state, self.key)?;
        if self.phase < 3
            && result == PhaseResult::NoEffect
            && matches!(
                entry.input_fact(),
                InputFact::FullWrite { .. } | InputFact::PartialOrUnknown { .. }
            )
        {
            entry.protected = true;
            return Err(Error::Conflict);
        }
        let phase = &mut entry.phases[self.phase];
        match phase {
            PhaseState::Pending => *phase = PhaseState::Settled(result),
            PhaseState::Settled(previous) if *previous == result => {}
            _ => {
                entry.protected = true;
                return Err(Error::Conflict);
            }
        }
        Ok(())
    }
}
impl Drop for Reporter {
    fn drop(&mut self) {
        if let Ok(mut state) = self.core.state.lock() {
            if let Ok(entry) = Core::entry(&mut state, self.key) {
                entry.live_reporters -= 1;
                if entry.phases[self.phase] == PhaseState::Pending {
                    entry.phases[self.phase] = PhaseState::Lost;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "input_submission/tests.rs"]
mod tests;

#[cfg(test)]
pub(super) mod test_support {
    use super::Owner;
    pub(crate) fn poison(owner: &Owner) {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = owner.core.state.lock().unwrap();
            panic!("injected reporting mutex poison");
        }));
    }
}
