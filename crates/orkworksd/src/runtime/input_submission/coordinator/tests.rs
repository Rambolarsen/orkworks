use super::*;
use crate::runtime::input_submission::recipe::{GraphAction, Outcome, Slot, SlotState};
use crate::runtime::input_submission::{
    scope, Context, Error, Flush, InputFact, PhaseResult, Submission,
};
use crate::runtime::session_runtime::{RuntimeCommand, RuntimeIdentity};
use crate::workspace_runtime::WorkspaceLease;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) fn profile() -> Profile {
    Profile {
        records: super::super::Limits {
            records: 2,
            observers: 6,
            input_bytes: 64,
            backing_bytes: 131072,
        },
        roots: 2,
        root_backing_bytes: 65536,
        scopes: scope::ScopeLimits {
            scopes: 6,
            bytes_per_scope: 4096,
            backing_bytes: 24576,
            lease_refs: 6,
            sender_refs: 6,
        },
        limit: Charges {
            bytes: 8192,
            handles: 16,
            notifications: 8,
        },
        per_job: Charges {
            bytes: 4096,
            handles: 8,
            notifications: 4,
        },
    }
}
pub(crate) struct Source {
    path: std::path::PathBuf,
    lease: Arc<WorkspaceLease>,
    identity: RuntimeIdentity,
    tx: tokio::sync::mpsc::Sender<RuntimeCommand>,
}
impl Source {
    pub(crate) fn new(path: &std::path::Path) -> Self {
        Self {
            path: path.to_path_buf(),
            lease: Arc::new(WorkspaceLease::acquire(path).unwrap()),
            identity: RuntimeIdentity {
                runtime_instance_id: "fixture-runtime".into(),
                run_generation: 1,
            },
            tx: tokio::sync::mpsc::channel(1).0,
        }
    }
    pub(crate) fn capture(&self, coordinator: &Coordinator) -> scope::OriginalInputScope {
        coordinator
            .reserve_scope()
            .unwrap()
            .capture(&self.path, &self.lease, "session", &self.identity, &self.tx)
            .unwrap_or_else(|e| panic!("{:?}", e.reason))
    }
}
pub(crate) fn context(slot: u64) -> Context {
    Context {
        root: 1,
        session: 2,
        incarnation: 1,
        slot,
    }
}
pub(crate) fn setup(path: &std::path::Path) -> (Coordinator, Route, Source) {
    let coordinator = Coordinator::fixture_new(profile()).unwrap();
    let source = Source::new(path);
    let route = coordinator
        .fixture_register(1, 1, source.capture(&coordinator))
        .unwrap_or_else(|e| panic!("{:?}", e.reason));
    (coordinator, route, source)
}
pub(crate) fn submit(
    coordinator: &Coordinator,
    route: &Route,
    source: &Source,
    slot: u64,
) -> CoordinatedSubmission {
    coordinator
        .admit(
            route,
            context(slot),
            3,
            Instant::now() + Duration::from_secs(60),
            source.capture(coordinator),
        )
        .unwrap_or_else(|e| panic!("{:?}", e.reason))
}
pub(crate) fn local_finish(job: &mut Job, slot: Slot, outcome: Outcome) {
    job.claim(slot)
        .unwrap_or_else(|e| panic!("{slot:?}: {e:?}"))
        .fixture_finish(outcome)
        .unwrap();
}
pub(crate) fn local_dispatch(job: &mut Job) {
    for slot in [Slot::Lookup, Slot::Reserve, Slot::Prepare, Slot::Dispatch] {
        local_finish(job, slot, Outcome::Done);
    }
}

#[test]
fn unavailable_production_profile_and_invalid_limits_never_admit() {
    let root = tempfile::tempdir().unwrap();
    let coordinator = Coordinator::new(profile()).unwrap();
    let source = Source::new(root.path());
    assert!(
        matches!(coordinator.fixture_register(1, 1, source.capture(&coordinator)), Err(e) if e.reason == Error::Unsupported)
    );
    let mut bad = profile();
    bad.per_job.bytes = usize::MAX;
    assert!(matches!(Coordinator::new(bad), Err(Error::InvalidLimits)));
    let mut bad = profile();
    bad.roots = usize::MAX;
    assert!(matches!(Coordinator::new(bad), Err(Error::InvalidLimits)));
}

#[test]
fn closing_retains_original_jobs_but_refuses_new_admission_and_route_removal() {
    let root = tempfile::tempdir().unwrap();
    let (coordinator, route, source) = setup(root.path());
    let mut admitted = submit(&coordinator, &route, &source, 1);
    coordinator.close(&route).unwrap();
    assert!(
        matches!(coordinator.admit(&route, context(2), 3, Instant::now()+Duration::from_secs(60), source.capture(&coordinator)), Err(e) if e.reason == Error::Closing)
    );
    assert_eq!(coordinator.remove(&route), Err(Error::Pending));
    // Required original slots remain spendable during Closing.
    local_finish(&mut admitted.job, Slot::Lookup, Outcome::Done);
    drop(source);
    drop(admitted.submission.observer);
    assert!(WorkspaceLease::acquire(root.path()).is_err());
    drop(admitted.job);
    let snapshot = coordinator
        .observe(context(1))
        .unwrap()
        .snapshot(Instant::now())
        .unwrap();
    assert_eq!(
        snapshot.recipe.unwrap().slots[Slot::Reserve as usize],
        SlotState::Lost
    );
    assert!(snapshot.protected);
}

#[test]
fn finite_job_charge_and_original_scope_survive_loss_and_refusal() {
    let root = tempfile::tempdir().unwrap();
    let mut p = profile();
    p.limit = p.per_job;
    let coordinator = Coordinator::fixture_new(p).unwrap();
    let source = Source::new(root.path());
    let route = coordinator
        .fixture_register(1, 1, source.capture(&coordinator))
        .unwrap_or_else(|e| panic!("{:?}", e.reason));
    let first = submit(&coordinator, &route, &source, 1);
    let refusal = match coordinator.admit(
        &route,
        context(2),
        3,
        Instant::now() + Duration::from_secs(60),
        source.capture(&coordinator),
    ) {
        Err(e) => e,
        Ok(_) => panic!("unreserved second job"),
    };
    assert_eq!(refusal.reason, Error::Capacity);
    drop((first, source));
    assert!(WorkspaceLease::acquire(root.path()).is_err());
    assert_eq!(
        coordinator.retire(coordinator.observe(context(1)).unwrap()),
        Err(Error::Protected)
    );
    drop(refusal.scope);
}

#[test]
fn unique_recipe_slots_obey_dependencies_and_lost_ticket_is_protected() {
    let root = tempfile::tempdir().unwrap();
    let (coordinator, route, source) = setup(root.path());
    let mut admitted = submit(&coordinator, &route, &source, 1);
    assert!(matches!(
        admitted.job.claim(Slot::Dispatch),
        Err(Error::Pending)
    ));
    let ticket = admitted
        .job
        .claim(Slot::Lookup)
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert!(matches!(
        admitted.job.claim(Slot::Lookup),
        Err(Error::Conflict)
    ));
    drop(ticket);
    let snapshot = admitted
        .submission
        .observer
        .snapshot(Instant::now())
        .unwrap();
    assert_eq!(
        snapshot.recipe.unwrap().slots[Slot::Lookup as usize],
        SlotState::Lost
    );
    assert!(snapshot.protected);
}

#[test]
fn expiry_does_not_erase_an_admitted_slot_or_admit_a_new_one() {
    let root = tempfile::tempdir().unwrap();
    let (coordinator, route, source) = setup(root.path());
    let deadline = Instant::now() + Duration::from_secs(60);
    let mut admitted = coordinator
        .admit(
            &route,
            context(1),
            3,
            deadline,
            source.capture(&coordinator),
        )
        .unwrap_or_else(|e| panic!("{:?}", e.reason));
    let ticket = admitted
        .job
        .claim(Slot::Lookup)
        .unwrap_or_else(|e| panic!("{e:?}"));
    // Deterministic state boundary, no sleep/racy timing oracle.
    coordinator.fixture_expire(context(1));
    ticket.fixture_finish(Outcome::Done).unwrap();
    assert!(matches!(
        admitted.job.claim(Slot::Reserve),
        Err(Error::Expired)
    ));
    assert_eq!(
        admitted
            .submission
            .observer
            .snapshot(Instant::now())
            .unwrap()
            .recipe
            .unwrap()
            .slots[Slot::Lookup as usize],
        SlotState::Returned(Outcome::Done)
    );
}

fn local_physical(submission: &Submission, fact: InputFact) {
    // Local transition inputs only. This helper never qualifies a producer.
    submission.reporters[1].record_input(fact).unwrap();
    for (index, reporter) in submission.reporters.iter().enumerate() {
        let outcome = if index == 1
            && fact
                == (InputFact::FullWrite {
                    flush: Flush::Failed,
                }) {
            PhaseResult::Failed
        } else if fact == InputFact::NoInputEffect && index == 1 {
            PhaseResult::NoEffect
        } else {
            PhaseResult::Completed
        };
        reporter.finish(outcome).unwrap();
    }
}
fn local_bookkeeping(job: &mut Job) {
    local_finish(job, Slot::Bookkeeping, Outcome::Done);
    for slot in [Slot::Reset, Slot::Tuple, Slot::Label] {
        job.reject(slot).unwrap();
    }
}

#[test]
fn local_graph_claims_have_one_notification_and_no_automatic_retry() {
    let root = tempfile::tempdir().unwrap();
    let (coordinator, route, source) = setup(root.path());
    let mut admitted = submit(&coordinator, &route, &source, 1);
    local_dispatch(&mut admitted.job);
    assert!(matches!(
        admitted.job.claim(Slot::Bookkeeping),
        Err(Error::Pending)
    ));
    local_physical(
        &admitted.submission,
        InputFact::FullWrite {
            flush: Flush::Succeeded,
        },
    );
    local_bookkeeping(&mut admitted.job);
    assert_eq!(admitted.job.graph_action(), Ok(GraphAction::Complete));
    local_finish(&mut admitted.job, Slot::InitialClaim, Outcome::Busy);
    assert!(matches!(
        admitted.job.claim(Slot::FinalClaim),
        Err(Error::Pending)
    ));
    local_finish(&mut admitted.job, Slot::Notify, Outcome::Notified);
    local_finish(&mut admitted.job, Slot::FinalClaim, Outcome::Busy);
    assert!(matches!(
        admitted.job.claim(Slot::Notify),
        Err(Error::Conflict | Error::Protected)
    ));
    assert!(
        admitted
            .submission
            .observer
            .snapshot(Instant::now())
            .unwrap()
            .protected
    );
    assert_eq!(
        coordinator.retire(coordinator.observe(context(1)).unwrap()),
        Err(Error::Protected)
    );
}

#[test]
fn local_rich_graph_selection_preserves_full_flush_failure_and_unknowns() {
    for (fact, expected) in [
        (
            InputFact::FullWrite {
                flush: Flush::Failed,
            },
            Ok(GraphAction::Complete),
        ),
        (InputFact::NoInputEffect, Ok(GraphAction::Cancel)),
        (
            InputFact::PartialOrUnknown { reported: 1 },
            Err(Error::Protected),
        ),
    ] {
        let root = tempfile::tempdir().unwrap();
        let (coordinator, route, source) = setup(root.path());
        let mut admitted = submit(&coordinator, &route, &source, 1);
        local_dispatch(&mut admitted.job);
        local_physical(&admitted.submission, fact);
        if fact == (InputFact::PartialOrUnknown { reported: 1 }) {
            assert!(matches!(
                admitted.job.claim(Slot::Bookkeeping),
                Err(Error::Protected)
            ));
        } else {
            local_bookkeeping(&mut admitted.job);
        }
        assert_eq!(admitted.job.graph_action(), expected);
        assert_eq!(
            admitted
                .submission
                .observer
                .snapshot(Instant::now())
                .unwrap()
                .fact,
            fact
        );
    }
}

#[test]
fn local_completed_recipe_releases_only_after_job_and_reporters_drop() {
    let root = tempfile::tempdir().unwrap();
    let (coordinator, route, source) = setup(root.path());
    let mut admitted = submit(&coordinator, &route, &source, 1);
    local_dispatch(&mut admitted.job);
    local_physical(
        &admitted.submission,
        InputFact::FullWrite {
            flush: Flush::Succeeded,
        },
    );
    local_bookkeeping(&mut admitted.job);
    local_finish(&mut admitted.job, Slot::InitialClaim, Outcome::Done);
    admitted.job.reject(Slot::Notify).unwrap();
    admitted.job.reject(Slot::FinalClaim).unwrap();
    local_finish(&mut admitted.job, Slot::GraphAction, Outcome::Done);
    admitted.job.reject(Slot::Event).unwrap();
    local_finish(&mut admitted.job, Slot::Dispose, Outcome::Done);
    assert_eq!(
        coordinator.retire(coordinator.observe(context(1)).unwrap()),
        Err(Error::Pending)
    );
    drop(admitted.job);
    let Submission {
        reporters,
        observer,
    } = admitted.submission;
    drop(reporters);
    coordinator.retire(observer).unwrap();
    coordinator.remove(&route).unwrap();
    drop((source, route));
    assert!(WorkspaceLease::acquire(root.path()).is_ok());
}

#[test]
fn foreign_scope_and_stale_route_cannot_rebind_an_original_job() {
    let root = tempfile::tempdir().unwrap();
    let (coordinator, route, source) = setup(root.path());
    let foreign_budget = scope::ScopeBudget::new(profile().scopes).unwrap();
    let foreign = foreign_budget
        .reserve()
        .unwrap()
        .capture(
            &source.path,
            &source.lease,
            "session",
            &source.identity,
            &source.tx,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.reason));
    assert!(
        matches!(coordinator.admit(&route, context(1), 3, Instant::now()+Duration::from_secs(60), foreign), Err(e) if e.reason == Error::Stale)
    );
    let old_key = route.key;
    coordinator.remove(&route).unwrap();
    drop(route);
    // Explicit forged stale routing fixture; production Route is never Clone.
    let route = Route {
        core: coordinator.owner.core.clone(),
        key: old_key,
    };
    let replacement = coordinator
        .fixture_register(1, 2, source.capture(&coordinator))
        .unwrap_or_else(|e| panic!("{:?}", e.reason));
    assert_eq!(coordinator.close(&route), Err(Error::Stale));
    assert!(
        matches!(coordinator.admit(&replacement, context(1), 3, Instant::now()+Duration::from_secs(60), source.capture(&coordinator)), Err(e) if e.reason == Error::Stale)
    );
    // The old entry API cannot bypass the coordinator's route/resource checks.
    assert!(
        matches!(coordinator.owner.admit_bound(context(1), 3, Instant::now()+Duration::from_secs(60), Instant::now(), source.capture(&coordinator)), Err(e) if e.reason == Error::Unsupported)
    );
}

#[test]
fn permanent_refusal_cannot_cancel_a_spent_slot_or_resend_it() {
    let root = tempfile::tempdir().unwrap();
    let (coordinator, route, source) = setup(root.path());
    let mut admitted = submit(&coordinator, &route, &source, 1);
    let ticket = admitted
        .job
        .claim(Slot::Lookup)
        .unwrap_or_else(|e| panic!("{e:?}"));
    assert_eq!(admitted.job.reject(Slot::Lookup), Err(Error::Conflict));
    ticket.fixture_finish(Outcome::Done).unwrap();
    admitted.job.reject(Slot::Reserve).unwrap();
    assert!(matches!(
        admitted.job.claim(Slot::Reserve),
        Err(Error::Conflict)
    ));
    assert!(matches!(
        admitted.job.claim(Slot::Prepare),
        Err(Error::Pending)
    ));
}

#[test]
fn closing_and_original_admission_have_one_atomic_transition() {
    let root = tempfile::tempdir().unwrap();
    let (coordinator, route, source) = setup(root.path());
    let scope = source.capture(&coordinator);
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|threads| {
        let close = threads.spawn(|| {
            barrier.wait();
            coordinator.close(&route).unwrap();
        });
        barrier.wait();
        let admitted = coordinator.admit(
            &route,
            context(1),
            3,
            Instant::now() + Duration::from_secs(60),
            scope,
        );
        close.join().unwrap();
        match admitted {
            Ok(mut admitted) => {
                // Admission won: original predeclared continuation survives Closing.
                local_finish(&mut admitted.job, Slot::Lookup, Outcome::Done);
                assert_eq!(coordinator.remove(&route), Err(Error::Pending));
            }
            Err(refusal) => assert_eq!(refusal.reason, Error::Closing),
        }
        assert!(
            matches!(coordinator.admit(&route, context(2), 3, Instant::now()+Duration::from_secs(60), source.capture(&coordinator)), Err(e) if e.reason==Error::Closing)
        );
    });
}

#[test]
fn graph_reservation_cannot_retire_by_rejecting_its_required_settlement() {
    let root = tempfile::tempdir().unwrap();
    let (coordinator, route, source) = setup(root.path());
    let mut admitted = submit(&coordinator, &route, &source, 1);
    local_dispatch(&mut admitted.job);
    local_physical(&admitted.submission, InputFact::NoInputEffect);
    for slot in [
        Slot::Bookkeeping,
        Slot::Reset,
        Slot::Tuple,
        Slot::Label,
        Slot::InitialClaim,
        Slot::Notify,
        Slot::FinalClaim,
        Slot::GraphAction,
        Slot::Event,
        Slot::Dispose,
    ] {
        admitted.job.reject(slot).unwrap();
    }
    drop(admitted.job);
    let Submission {
        observer,
        reporters,
    } = admitted.submission;
    drop(reporters);
    assert!(observer.snapshot(Instant::now()).unwrap().protected);
    assert_eq!(coordinator.retire(observer), Err(Error::Protected));
    assert_eq!(coordinator.remove(&route), Err(Error::Pending));
}

#[test]
fn retired_route_handles_remain_charged_until_their_last_owner_drops() {
    let root = tempfile::tempdir().unwrap();
    let mut p = profile();
    p.roots = 1;
    let coordinator = Coordinator::fixture_new(p).unwrap();
    let source = Source::new(root.path());
    let route = coordinator
        .fixture_register(1, 1, source.capture(&coordinator))
        .unwrap_or_else(|e| panic!("{:?}", e.reason));
    coordinator.remove(&route).unwrap();
    assert!(
        matches!(coordinator.fixture_register(2, 1, source.capture(&coordinator)), Err(e) if e.reason == Error::Capacity)
    );
    drop(route);
    assert!(coordinator
        .fixture_register(2, 1, source.capture(&coordinator))
        .is_ok());
}
