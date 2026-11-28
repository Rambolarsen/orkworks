//! Owned worker accounting during dormant custody qualification.

use std::io;
use std::time::Instant;

enum Slot<T> {
    Unused,
    Starting(T),
    Ready(T),
    Draining(T),
    Retired,
}

/// The retained attempt supplies the result for its own child.
pub(super) trait AttemptOwner {
    type Exit;

    fn try_reap(&mut self) -> io::Result<Option<Self::Exit>>;
}

pub(super) struct StartupSlots<T> {
    deadline: Instant,
    closed: bool,
    slots: [Slot<T>; 4],
}

impl<T: AttemptOwner> StartupSlots<T> {
    pub(super) fn new(deadline: Instant) -> Self {
        Self {
            deadline,
            closed: false,
            slots: std::array::from_fn(|_| Slot::Unused),
        }
    }

    /// Creates an unstarted owner only when there is room to reserve it.
    /// The caller begins the attempt through the returned owner.
    pub(super) fn begin(
        &mut self,
        now: Instant,
        new_owner: impl FnOnce() -> T,
    ) -> Option<(usize, &T)> {
        if now >= self.deadline {
            self.close_startup();
        }
        if self.closed {
            return None;
        }
        let (index, slot) = self
            .slots
            .iter_mut()
            .enumerate()
            .find(|(_, slot)| matches!(slot, Slot::Unused))?;
        *slot = Slot::Starting(new_owner());
        let Slot::Starting(owner) = slot else {
            unreachable!()
        };
        Some((index, owner))
    }

    /// Called after validating readiness against the exact owned attempt.
    pub(super) fn publish_ready(&mut self, index: usize, now: Instant) -> bool {
        if now >= self.deadline {
            self.close_startup();
            return false;
        }
        let Some(slot @ Slot::Starting(_)) = self.slots.get_mut(index) else {
            return false;
        };
        let Slot::Starting(owner) = std::mem::replace(slot, Slot::Retired) else {
            unreachable!()
        };
        *slot = Slot::Ready(owner);
        true
    }

    pub(super) fn close_startup(&mut self) {
        self.closed = true;
        for slot in &mut self.slots {
            if let slot @ Slot::Starting(_) = slot {
                move_to_draining(slot);
            }
        }
    }

    pub(super) fn drain(&mut self, index: usize) -> Option<&T> {
        let slot = self.slots.get_mut(index)?;
        if matches!(slot, Slot::Starting(_) | Slot::Ready(_)) {
            move_to_draining(slot);
        }
        match slot {
            Slot::Draining(owner) => Some(owner),
            _ => None,
        }
    }

    /// Poll the exact draining owner and return its confirmed exit once.
    pub(super) fn poll_reap(&mut self, index: usize) -> io::Result<Option<T::Exit>> {
        let Some(Slot::Draining(owner)) = self.slots.get_mut(index) else {
            return Ok(None);
        };
        let Some(exit) = owner.try_reap()? else {
            return Ok(None);
        };
        self.slots[index] = Slot::Retired;
        Ok(Some(exit))
    }

    pub(super) fn ready_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| matches!(slot, Slot::Ready(_)))
            .count()
    }

    pub(super) fn charged_count(&self) -> usize {
        self.slots
            .iter()
            .filter(|slot| !matches!(slot, Slot::Unused | Slot::Retired))
            .count()
    }
}

fn move_to_draining<T>(slot: &mut Slot<T>) {
    let owner = match std::mem::replace(slot, Slot::Retired) {
        Slot::Starting(owner) | Slot::Ready(owner) => owner,
        _ => unreachable!(),
    };
    *slot = Slot::Draining(owner);
}
