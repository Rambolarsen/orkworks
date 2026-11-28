use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[path = "../src/taskmaster/codex_capture_owner/launch.rs"]
mod launch;
#[path = "../src/taskmaster/codex_capture_owner/slots.rs"]
mod slots;

use launch::{LaunchOwner, SpawnedAttempt};

use slots::{AttemptOwner, StartupSlots};

#[derive(Default)]
struct Attempt {
    number: usize,
    polls: VecDeque<io::Result<bool>>,
    dropped: Option<Rc<RefCell<Vec<usize>>>>,
}

impl AttemptOwner for Attempt {
    type Exit = usize;

    fn try_reap(&mut self) -> io::Result<Option<usize>> {
        self.polls
            .pop_front()
            .expect("unexpected reap poll")
            .map(|reaped| reaped.then_some(self.number))
    }
}

impl Drop for Attempt {
    fn drop(&mut self) {
        if let Some(dropped) = &self.dropped {
            dropped.borrow_mut().push(self.number);
        }
    }
}

fn reaping_attempt() -> Attempt {
    Attempt {
        number: 0,
        polls: [Ok(true)].into(),
        dropped: None,
    }
}

#[test]
fn failed_slot_retains_its_exact_owner_until_reap_and_cannot_be_reused() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(3);
    let dropped = Rc::new(RefCell::new(Vec::new()));
    let mut slots = StartupSlots::new(deadline);

    for number in 0..4 {
        let polls = if number == 1 {
            [Ok(false), Err(io::ErrorKind::Interrupted.into()), Ok(true)].into()
        } else {
            VecDeque::new()
        };
        let (slot, owner) = slots
            .begin(now, || Attempt {
                number,
                polls,
                dropped: Some(Rc::clone(&dropped)),
            })
            .unwrap();
        assert_eq!(owner.number, number);
        assert!(slots.publish_ready(slot, now));
    }
    assert!(slots.begin(now, || panic!("fifth attempt")).is_none());
    assert_eq!(slots.ready_count(), 4);

    assert_eq!(slots.drain(1).unwrap().number, 1);
    assert_eq!(slots.ready_count(), 3);
    assert_eq!(slots.charged_count(), 4);
    assert_eq!(slots.poll_reap(1).unwrap(), None);
    let error = slots.poll_reap(1).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    assert_eq!(slots.charged_count(), 4);
    assert!(dropped.borrow().is_empty());
    assert!(slots
        .begin(now, || panic!("replacement while draining"))
        .is_none());

    assert_eq!(slots.poll_reap(1).unwrap(), Some(1));
    assert_eq!(&*dropped.borrow(), &[1]);
    assert_eq!(slots.charged_count(), 3);
    assert_eq!(slots.ready_count(), 3);
    assert!(slots.drain(1).is_none());
    assert_eq!(slots.poll_reap(1).unwrap(), None);
    assert!(slots
        .begin(now, || panic!("replacement after reap"))
        .is_none());
}

#[test]
fn closing_startup_keeps_ready_workers_and_drains_unfinished_attempts() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(3);
    let mut slots = StartupSlots::new(deadline);
    let (ready, _) = slots.begin(now, Attempt::default).unwrap();
    let (late, _) = slots.begin(now, reaping_attempt).unwrap();
    assert!(slots.publish_ready(ready, now));
    assert_eq!(slots.poll_reap(late).unwrap(), None);

    assert!(!slots.publish_ready(late, deadline));
    assert_eq!(slots.ready_count(), 1);
    assert_eq!(slots.charged_count(), 2);
    assert!(!slots.publish_ready(late, deadline));
    assert!(slots
        .begin(deadline, || panic!("past the deadline"))
        .is_none());
    assert_eq!(slots.poll_reap(late).unwrap(), Some(0));
    assert_eq!(slots.ready_count(), 1);
    assert_eq!(slots.charged_count(), 1);
}

#[test]
fn explicit_startup_close_is_irreversible_even_before_the_deadline() {
    let now = Instant::now();
    let mut slots = StartupSlots::new(now + Duration::from_secs(3));
    let (slot, _) = slots.begin(now, reaping_attempt).unwrap();
    slots.close_startup();

    assert!(slots.begin(now, || panic!("startup closed")).is_none());
    assert!(!slots.publish_ready(slot, now));
    assert_eq!(slots.ready_count(), 0);
    assert_eq!(slots.charged_count(), 1);
    assert_eq!(slots.poll_reap(slot).unwrap(), Some(0));
    assert_eq!(slots.charged_count(), 0);
}

#[test]
fn confirmed_early_failure_leaves_only_the_other_three_startup_slots() {
    let now = Instant::now();
    let mut slots = StartupSlots::new(now + Duration::from_secs(3));
    let (failed, _) = slots.begin(now, reaping_attempt).unwrap();
    assert_eq!(failed, 0);
    assert!(slots.drain(failed).is_some());
    assert_eq!(slots.poll_reap(failed).unwrap(), Some(0));

    for expected in 1..4 {
        let (slot, _) = slots.begin(now, Attempt::default).unwrap();
        assert_eq!(slot, expected);
        assert!(slots.publish_ready(slot, now));
        assert_eq!(slots.poll_reap(slot).unwrap(), None);
    }
    assert_eq!(slots.charged_count(), 3);
    assert_eq!(slots.ready_count(), 3);
    assert!(slots.begin(now, || panic!("replacement")).is_none());
}

struct DelayedChild {
    closed: bool,
    polls: usize,
}

impl AttemptOwner for DelayedChild {
    type Exit = usize;

    fn try_reap(&mut self) -> io::Result<Option<Self::Exit>> {
        assert!(self.closed);
        self.polls += 1;
        Ok((self.polls == 2).then_some(7))
    }
}

impl SpawnedAttempt for DelayedChild {
    fn close_channel(&mut self) {
        self.closed = true;
    }
}

#[test]
fn expired_slot_keeps_pending_launch_and_reaps_the_late_child() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(3);
    let mut slots = StartupSlots::new(deadline);
    let (release, gate) = std::sync::mpsc::channel();
    let (slot, owner) = slots.begin(now, LaunchOwner::default).unwrap();
    owner
        .start(move || {
            gate.recv_timeout(Duration::from_secs(4)).unwrap();
            Ok(DelayedChild {
                closed: false,
                polls: 0,
            })
        })
        .unwrap();
    assert!(owner.poll_spawned().unwrap().is_none());
    assert_eq!(
        owner
            .start(|| panic!("duplicate launch"))
            .unwrap_err()
            .kind(),
        io::ErrorKind::AlreadyExists
    );

    assert!(!slots.publish_ready(slot, deadline));
    assert_eq!(slots.charged_count(), 1);
    assert_eq!(slots.ready_count(), 0);
    assert_eq!(slots.poll_reap(slot).unwrap(), None);
    assert!(slots
        .begin(deadline, || panic!("new attempt after deadline"))
        .is_none());

    release.send(()).unwrap();
    let wait_until = Instant::now() + Duration::from_secs(3);
    loop {
        assert!(Instant::now() < wait_until);
        if let Some(exit) = slots.poll_reap(slot).unwrap() {
            assert_eq!(exit, 7);
            break;
        }
        assert_eq!(slots.charged_count(), 1);
        std::thread::yield_now();
    }
    assert_eq!(slots.charged_count(), 0);
    assert!(!slots.publish_ready(slot, deadline));
}

#[test]
fn failed_launch_report_keeps_the_slot_charged() {
    for disconnected in [false, true] {
        let now = Instant::now();
        let mut slots =
            StartupSlots::<LaunchOwner<DelayedChild>>::new(now + Duration::from_secs(3));
        let (slot, owner) = slots.begin(now, LaunchOwner::default).unwrap();
        owner
            .start(move || {
                assert!(!disconnected, "simulated loss of launch result");
                Err(io::ErrorKind::PermissionDenied.into())
            })
            .unwrap();
        slots.close_startup();

        let wait_until = Instant::now() + Duration::from_secs(3);
        loop {
            assert!(Instant::now() < wait_until);
            match slots.poll_reap(slot) {
                Ok(None) => std::thread::yield_now(),
                Err(error) => {
                    let expected = if disconnected {
                        io::ErrorKind::BrokenPipe
                    } else {
                        io::ErrorKind::PermissionDenied
                    };
                    assert_eq!(error.kind(), expected);
                    break;
                }
                Ok(Some(_)) => panic!("unconfirmed launch retired"),
            }
        }
        assert!(slots.poll_reap(slot).is_err());
        assert_eq!(slots.charged_count(), 1);
    }
}

struct DropNotice(std::sync::mpsc::Sender<()>);

impl Drop for DropNotice {
    fn drop(&mut self) {
        let _ = self.0.send(());
    }
}

#[test]
fn abandoning_a_pending_result_drops_the_late_owner() {
    let (release, gate) = std::sync::mpsc::channel();
    let (dropped, notice) = std::sync::mpsc::channel();
    let owner = LaunchOwner::default();
    owner
        .start(move || {
            gate.recv_timeout(Duration::from_secs(4)).unwrap();
            Ok(DropNotice(dropped))
        })
        .unwrap();

    drop(owner);
    release.send(()).unwrap();
    notice.recv_timeout(Duration::from_secs(3)).unwrap();
}
