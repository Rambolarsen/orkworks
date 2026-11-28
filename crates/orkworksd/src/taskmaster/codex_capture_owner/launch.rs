//! One supervised attempt for the dormant custody qualification.

use super::slots::AttemptOwner;
use std::cell::{Ref, RefCell};
use std::io;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub(super) trait SpawnedAttempt: AttemptOwner {
    /// Idempotently close the controller channel before polling for exit.
    fn close_channel(&mut self);
}

enum State<T> {
    Unstarted,
    Pending(Receiver<io::Result<T>>),
    Spawned(T),
    Unconfirmed(io::ErrorKind),
}

pub(super) struct LaunchOwner<T> {
    state: RefCell<State<T>>,
}

impl<T> Default for LaunchOwner<T> {
    fn default() -> Self {
        Self {
            state: RefCell::new(State::Unstarted),
        }
    }
}

impl<T: Send + 'static> LaunchOwner<T> {
    pub(super) fn start(
        &self,
        launch: impl FnOnce() -> io::Result<T> + Send + 'static,
    ) -> io::Result<()> {
        let mut state = self.state.borrow_mut();
        if !matches!(*state, State::Unstarted) {
            return Err(io::ErrorKind::AlreadyExists.into());
        }
        let (result, pending) = mpsc::sync_channel(1);
        // The slot retains the receiver. Once a result has been sent, the
        // launch task has handed off its owner and has no more process work.
        if let Err(error) = std::thread::Builder::new().spawn(move || {
            // If the supervisor itself is gone, drop the returned owner
            // here so its process cleanup guard remains responsible.
            let _ = result.send(launch());
        }) {
            *state = State::Unconfirmed(error.kind());
            return Err(error);
        }
        *state = State::Pending(pending);
        Ok(())
    }

    pub(super) fn poll_spawned(&self) -> io::Result<Option<Ref<'_, T>>> {
        collect(&mut self.state.borrow_mut())?;
        Ok(Ref::filter_map(self.state.borrow(), |state| match state {
            State::Spawned(owner) => Some(owner),
            _ => None,
        })
        .ok())
    }
}

impl<T: SpawnedAttempt> AttemptOwner for LaunchOwner<T> {
    type Exit = T::Exit;

    fn try_reap(&mut self) -> io::Result<Option<Self::Exit>> {
        let state = self.state.get_mut();
        collect(state)?;
        let State::Spawned(owner) = state else {
            return Ok(None);
        };
        owner.close_channel();
        owner.try_reap()
    }
}

fn collect<T>(state: &mut State<T>) -> io::Result<()> {
    let received = match state {
        State::Pending(pending) => pending.try_recv(),
        State::Unconfirmed(kind) => {
            return Err(io::Error::new(*kind, "launch outcome unconfirmed"))
        }
        State::Unstarted | State::Spawned(_) => return Ok(()),
    };
    let error = match received {
        Ok(Ok(owner)) => {
            *state = State::Spawned(owner);
            return Ok(());
        }
        Ok(Err(error)) => error,
        Err(TryRecvError::Empty) => return Ok(()),
        Err(TryRecvError::Disconnected) => {
            io::Error::new(io::ErrorKind::BrokenPipe, "launch result disconnected")
        }
    };
    *state = State::Unconfirmed(error.kind());
    Err(error)
}
