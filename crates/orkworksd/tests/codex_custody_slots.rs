use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::time::{Duration, Instant};

#[path = "../src/taskmaster/codex_capture_owner/slots.rs"]
mod slots;

use slots::StartupSlots;

struct Attempt {
    number: usize,
    dropped: Rc<RefCell<Vec<usize>>>,
}

impl Drop for Attempt {
    fn drop(&mut self) {
        self.dropped.borrow_mut().push(self.number);
    }
}

#[test]
fn failed_slot_retains_its_exact_owner_until_reap_and_cannot_be_reused() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(3);
    let dropped = Rc::new(RefCell::new(Vec::new()));
    let mut slots = StartupSlots::new(deadline);

    for number in 0..4 {
        let (slot, owner) = slots
            .begin(now, || Attempt {
                number,
                dropped: Rc::clone(&dropped),
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
    assert!(!slots
        .poll_reap(1, |owner| {
            assert_eq!(owner.number, 1);
            Ok(false)
        })
        .unwrap());
    let error = slots
        .poll_reap(1, |_| Err(io::ErrorKind::Interrupted.into()))
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::Interrupted);
    assert_eq!(slots.charged_count(), 4);
    assert!(dropped.borrow().is_empty());
    assert!(slots
        .begin(now, || panic!("replacement while draining"))
        .is_none());

    assert!(slots
        .poll_reap(1, |owner| {
            assert_eq!(owner.number, 1);
            Ok(true)
        })
        .unwrap());
    assert_eq!(&*dropped.borrow(), &[1]);
    assert_eq!(slots.charged_count(), 3);
    assert_eq!(slots.ready_count(), 3);
    assert!(slots.drain(1).is_none());
    assert!(!slots.poll_reap(1, |_| panic!("reaped twice")).unwrap());
    assert!(slots
        .begin(now, || panic!("replacement after reap"))
        .is_none());
}

#[test]
fn closing_startup_keeps_ready_workers_and_drains_unfinished_attempts() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(3);
    let mut slots = StartupSlots::new(deadline);
    let (ready, _) = slots.begin(now, || "ready").unwrap();
    let (late, _) = slots.begin(now, || "late").unwrap();
    assert!(slots.publish_ready(ready, now));
    assert!(!slots.poll_reap(late, |_| panic!("still starting")).unwrap());

    assert!(!slots.publish_ready(late, deadline));
    assert_eq!(slots.ready_count(), 1);
    assert_eq!(slots.charged_count(), 2);
    assert!(!slots.publish_ready(late, deadline));
    assert!(slots
        .begin(deadline, || panic!("past the deadline"))
        .is_none());
    assert!(slots
        .poll_reap(late, |owner| {
            assert_eq!(*owner, "late");
            Ok(true)
        })
        .unwrap());
    assert_eq!(slots.ready_count(), 1);
    assert_eq!(slots.charged_count(), 1);
}

#[test]
fn explicit_startup_close_is_irreversible_even_before_the_deadline() {
    let now = Instant::now();
    let mut slots = StartupSlots::new(now + Duration::from_secs(3));
    let (slot, _) = slots.begin(now, || ()).unwrap();
    slots.close_startup();

    assert!(slots.begin(now, || panic!("startup closed")).is_none());
    assert!(!slots.publish_ready(slot, now));
    assert_eq!(slots.ready_count(), 0);
    assert_eq!(slots.charged_count(), 1);
    assert!(slots.poll_reap(slot, |_| Ok(true)).unwrap());
    assert_eq!(slots.charged_count(), 0);
}

#[test]
fn confirmed_early_failure_leaves_only_the_other_three_startup_slots() {
    let now = Instant::now();
    let mut slots = StartupSlots::new(now + Duration::from_secs(3));
    let (failed, _) = slots.begin(now, || ()).unwrap();
    assert_eq!(failed, 0);
    assert!(slots.drain(failed).is_some());
    assert!(slots.poll_reap(failed, |_| Ok(true)).unwrap());

    for expected in 1..4 {
        let (slot, _) = slots.begin(now, || ()).unwrap();
        assert_eq!(slot, expected);
        assert!(slots.publish_ready(slot, now));
        assert!(!slots.poll_reap(slot, |_| panic!("still ready")).unwrap());
    }
    assert_eq!(slots.charged_count(), 3);
    assert_eq!(slots.ready_count(), 3);
    assert!(slots.begin(now, || panic!("replacement")).is_none());
}
