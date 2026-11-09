use super::*;
use std::time::{Duration, Instant};

fn limits(records: usize, observers: usize) -> Limits {
    Limits {
        records,
        observers,
        input_bytes: 64,
        backing_bytes: 64 * 1024,
    }
}
fn context(slot: u64) -> Context {
    Context {
        root: 1,
        session: 2,
        incarnation: 3,
        slot,
    }
}
fn submit(owner: &Owner, slot: u64) -> Submission {
    let now = Instant::now();
    owner
        .admit(context(slot), 4, now + Duration::from_secs(1), now)
        .unwrap()
}
fn finish(roles: &[Reporter; 4]) {
    for role in roles {
        role.finish(PhaseResult::Completed).unwrap();
    }
}

#[test]
fn observer_loss_does_not_release_original_record() {
    let owner = Owner::new(limits(1, 1)).unwrap();
    let Submission {
        observer,
        reporters,
    } = submit(&owner, 1);
    drop(observer);
    assert!(matches!(
        owner.admit(
            context(2),
            4,
            Instant::now() + Duration::from_secs(1),
            Instant::now()
        ),
        Err(Error::Capacity)
    ));
    let observer = owner.observe(context(1)).unwrap();
    assert_eq!(
        observer.snapshot(Instant::now()).unwrap().fact,
        InputFact::Unresolved
    );
    drop(reporters);
    assert!(observer.snapshot(Instant::now()).unwrap().protected);
    assert_eq!(owner.retire(observer), Err(Error::Protected));
}

#[test]
fn full_write_and_flush_failure_do_not_settle_physical_return() {
    let owner = Owner::new(limits(1, 2)).unwrap();
    let Submission {
        observer,
        reporters,
    } = submit(&owner, 1);
    reporters[1]
        .record_input(InputFact::FullWrite {
            flush: Flush::Failed,
        })
        .unwrap();
    assert_eq!(
        observer.snapshot(Instant::now()).unwrap().fact,
        InputFact::FullWrite {
            flush: Flush::Failed
        }
    );
    reporters[0].finish(PhaseResult::Completed).unwrap();
    reporters[1].finish(PhaseResult::Completed).unwrap();
    reporters[3].finish(PhaseResult::Completed).unwrap();
    let another = owner.observe(context(1)).unwrap();
    assert_eq!(owner.retire(another), Err(Error::Pending));
    reporters[2].finish(PhaseResult::Completed).unwrap();
    // Settled reporters still retain their charged handles until physically dropped.
    let another = owner.observe(context(1)).unwrap();
    assert_eq!(owner.retire(another), Err(Error::Pending));
    drop(reporters);
    assert_eq!(
        owner.retire(observer).unwrap().fact,
        InputFact::FullWrite {
            flush: Flush::Failed
        }
    );
    assert!(submit(&owner, 2).observer.snapshot(Instant::now()).is_ok());
}

#[test]
fn wrong_role_and_conflicting_facts_protect_prior_truth() {
    let owner = Owner::new(limits(2, 2)).unwrap();
    let a = submit(&owner, 1);
    assert_eq!(
        a.reporters[2].record_input(InputFact::NoInputEffect),
        Err(Error::WrongRole)
    );
    assert!(a.observer.snapshot(Instant::now()).unwrap().protected);
    let b = submit(&owner, 2);
    b.reporters[1]
        .record_input(InputFact::FullWrite {
            flush: Flush::Succeeded,
        })
        .unwrap();
    assert_eq!(
        b.reporters[0].record_input(InputFact::NoInputEffect),
        Err(Error::Conflict)
    );
    assert_eq!(
        b.observer.snapshot(Instant::now()).unwrap().fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
    assert!(b.observer.snapshot(Instant::now()).unwrap().protected);
}

#[test]
fn partial_unknown_and_failed_bookkeeping_are_not_retirable() {
    for fact in [
        InputFact::Unresolved,
        InputFact::PartialOrUnknown { reported: 0 },
        InputFact::PartialOrUnknown { reported: 2 },
    ] {
        let owner = Owner::new(limits(1, 1)).unwrap();
        let Submission {
            observer,
            reporters,
        } = submit(&owner, 1);
        reporters[1].record_input(fact).unwrap();
        assert!(observer.snapshot(Instant::now()).unwrap().protected);
        finish(&reporters);
        drop(reporters);
        assert_eq!(owner.retire(observer), Err(Error::Protected));
    }
    let owner = Owner::new(limits(1, 1)).unwrap();
    let Submission {
        observer,
        reporters,
    } = submit(&owner, 1);
    reporters[0].record_input(InputFact::NoInputEffect).unwrap();
    reporters[3].finish(PhaseResult::Failed).unwrap();
    drop(reporters);
    assert!(observer.snapshot(Instant::now()).unwrap().protected);
    assert_eq!(owner.retire(observer), Err(Error::Protected));
}

#[test]
fn expiry_rejects_new_admission_but_not_original_reports() {
    let owner = Owner::new(limits(1, 1)).unwrap();
    let now = Instant::now();
    assert!(matches!(
        owner.admit(context(1), 4, now, now),
        Err(Error::Expired)
    ));
    let Submission {
        observer,
        reporters,
    } = owner
        .admit(context(1), 4, now + Duration::from_secs(1), now)
        .unwrap();
    assert!(
        observer
            .snapshot(now + Duration::from_secs(2))
            .unwrap()
            .expired
    );
    reporters[0].record_input(InputFact::NoInputEffect).unwrap();
    for role in &reporters {
        role.finish(PhaseResult::NoEffect).unwrap();
    }
    drop(reporters);
    assert_eq!(
        owner.retire(observer).unwrap().fact,
        InputFact::NoInputEffect
    );
}

#[test]
fn duplicate_context_and_observer_limits_do_not_replace_original() {
    let owner = Owner::new(limits(2, 2)).unwrap();
    let a = submit(&owner, 1);
    let now = Instant::now();
    assert!(matches!(
        owner.admit(context(1), 4, now + Duration::from_secs(1), now),
        Err(Error::Duplicate)
    ));
    let observer = owner.observe(context(1)).unwrap();
    assert!(matches!(owner.observe(context(1)), Err(Error::Capacity)));
    drop(observer);
    assert!(owner.observe(context(1)).is_ok());
    assert_eq!(a.observer.snapshot(now).unwrap().context, context(1));
}

#[test]
fn retired_observers_are_stale_and_still_count_toward_capacity() {
    let owner = Owner::new(limits(1, 2)).unwrap();
    let Submission {
        observer,
        reporters,
    } = submit(&owner, 1);
    let stale = owner.observe(context(1)).unwrap();
    reporters[0].record_input(InputFact::NoInputEffect).unwrap();
    finish(&reporters);
    drop(reporters);
    owner.retire(observer).unwrap();
    let next = submit(&owner, 1);
    assert_eq!(stale.snapshot(Instant::now()), Err(Error::Stale));
    assert!(matches!(owner.observe(context(1)), Err(Error::Capacity)));
    drop(stale);
    assert!(owner.observe(context(1)).is_ok());
    assert_eq!(
        next.observer.snapshot(Instant::now()).unwrap().fact,
        InputFact::Unresolved
    );
}

#[test]
fn finite_limits_and_checked_storage_reject_before_admission() {
    assert!(matches!(
        Owner::new(limits(0, 1)),
        Err(Error::InvalidLimits)
    ));
    assert!(matches!(
        Owner::new(limits(1, 0)),
        Err(Error::InvalidLimits)
    ));
    let mut budget = limits(1, 1);
    budget.backing_bytes = 1;
    assert!(matches!(Owner::new(budget), Err(Error::InvalidLimits)));
    assert!(matches!(
        Owner::new(limits(usize::MAX, 1)),
        Err(Error::InvalidLimits)
    ));
    let owner = Owner::new(limits(1, 1)).unwrap();
    let now = Instant::now();
    assert!(matches!(
        owner.admit(context(1), 65, now + Duration::from_secs(1), now),
        Err(Error::TooLarge)
    ));
    assert!(owner
        .admit(context(1), 64, now + Duration::from_secs(1), now)
        .is_ok());
}

#[test]
fn invalid_prefix_and_conflicting_completion_remain_protected() {
    let owner = Owner::new(limits(1, 1)).unwrap();
    let a = submit(&owner, 1);
    assert_eq!(
        a.reporters[1].record_input(InputFact::PartialOrUnknown { reported: 5 }),
        Err(Error::InvalidFact)
    );
    a.reporters[2].finish(PhaseResult::Completed).unwrap();
    a.reporters[2].finish(PhaseResult::Completed).unwrap();
    assert_eq!(
        a.reporters[2].finish(PhaseResult::NoEffect),
        Err(Error::Conflict)
    );
    assert!(a.observer.snapshot(Instant::now()).unwrap().protected);
}

#[test]
fn concurrent_reports_preserve_record_after_observer_disconnect() {
    let owner = Owner::new(limits(1, 1)).unwrap();
    let Submission {
        observer,
        reporters,
    } = submit(&owner, 1);
    drop(observer);
    std::thread::scope(|scope| {
        for (index, reporter) in reporters.into_iter().enumerate() {
            scope.spawn(move || {
                if index == 1 {
                    reporter
                        .record_input(InputFact::FullWrite {
                            flush: Flush::Unknown,
                        })
                        .unwrap();
                }
                reporter.finish(PhaseResult::Completed).unwrap();
            });
        }
    });
    let observer = owner.observe(context(1)).unwrap();
    assert_eq!(
        owner.retire(observer).unwrap().fact,
        InputFact::FullWrite {
            flush: Flush::Unknown
        }
    );
}

#[test]
fn no_effect_phases_cannot_contradict_a_known_full_write() {
    for phase in 0..3 {
        for fact_first in [false, true] {
            let owner = Owner::new(limits(1, 1)).unwrap();
            let submission = submit(&owner, 1);
            if fact_first {
                submission.reporters[1]
                    .record_input(InputFact::FullWrite {
                        flush: Flush::Succeeded,
                    })
                    .unwrap();
                assert_eq!(
                    submission.reporters[phase].finish(PhaseResult::NoEffect),
                    Err(Error::Conflict)
                );
            } else {
                submission.reporters[phase]
                    .finish(PhaseResult::NoEffect)
                    .unwrap();
                assert_eq!(
                    submission.reporters[1].record_input(InputFact::FullWrite {
                        flush: Flush::Succeeded
                    }),
                    Err(Error::Conflict)
                );
            }
            assert!(
                submission
                    .observer
                    .snapshot(Instant::now())
                    .unwrap()
                    .protected
            );
        }
    }
}

#[test]
fn empty_input_is_byte_truth_only_and_does_not_settle_other_phases() {
    let owner = Owner::new(limits(1, 1)).unwrap();
    let now = Instant::now();
    let Submission {
        observer,
        reporters,
    } = owner
        .admit(context(1), 0, now + Duration::from_secs(1), now)
        .unwrap();
    reporters[1]
        .record_input(InputFact::FullWrite {
            flush: Flush::Succeeded,
        })
        .unwrap();
    assert_eq!(observer.snapshot(now).unwrap().input_bytes, 0);
    finish(&reporters);
    drop(reporters);
    assert_eq!(
        owner.retire(observer).unwrap().fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
}
