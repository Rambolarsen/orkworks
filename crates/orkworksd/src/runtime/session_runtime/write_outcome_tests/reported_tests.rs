use super::*;
use crate::runtime::input_submission::{
    self, Context, Error as ReportError, Limits, Observer, Owner, PhaseResult, PhaseState,
    Reporter, Snapshot, Submission,
};
use std::time::{Duration, Instant};

struct Fixture {
    owner: Owner,
    observer: Option<Observer>,
    _preparation: Reporter,
    _bookkeeping: Reporter,
    context: Context,
}
impl Fixture {
    fn snapshot(&self) -> Snapshot {
        self.observer
            .as_ref()
            .unwrap()
            .snapshot(Instant::now())
            .unwrap()
    }
}
fn fixture() -> (Fixture, (Reporter, Reporter)) {
    fixture_for(3)
}
fn fixture_for(bytes: usize) -> (Fixture, (Reporter, Reporter)) {
    let owner = Owner::new(Limits {
        records: 1,
        observers: 2,
        input_bytes: 16,
        backing_bytes: 64 * 1024,
    })
    .unwrap();
    let context = Context {
        root: 1,
        session: 2,
        incarnation: 3,
        slot: 4,
    };
    let now = Instant::now();
    let Submission {
        reporters: [preparation, writer, returned, bookkeeping],
        observer,
    } = owner
        .admit(context, bytes, now + Duration::from_secs(60), now)
        .unwrap();
    (
        Fixture {
            owner,
            observer: Some(observer),
            _preparation: preparation,
            _bookkeeping: bookkeeping,
            context,
        },
        (writer, returned),
    )
}
fn input(writer: Box<dyn Write + Send>, reports: (Reporter, Reporter)) -> ReportedPtyInput {
    ReportedPtyInput {
        writer,
        data: "abc".into(),
        accepted: None,
        stop_rx: tokio::sync::watch::channel(false).1,
        reports,
        after_ack: None,
    }
}
fn start(input: ReportedPtyInput) -> PendingPtyWrite {
    PendingPtyWrite::start_reported(input)
        .unwrap_or_else(|error| panic!("unexpected refusal: {:?}", error.reason))
}
async fn joined(
    mut pending: PendingPtyWrite,
) -> Result<(Box<dyn Write + Send>, PtyWriteOutcome), tokio::task::JoinError> {
    pending.finish_join().await
}
fn pending_application_phases(snapshot: &Snapshot) {
    assert_eq!(snapshot.phases[0], PhaseState::Pending);
    assert_eq!(snapshot.phases[3], PhaseState::Pending);
}

#[tokio::test]
async fn full_checkpoint_survives_flush_panic_and_actual_failed_join() {
    let (fixture, reports) = fixture();
    let (hold, observing, release) = gate();
    let mut writer = BlockingWriter::plain();
    writer.flush = Some(hold);
    writer.panic_flush = true;
    let pending = start(input(Box::new(writer), reports));
    entered(observing).await;
    let snapshot = fixture.snapshot();
    assert_eq!(
        snapshot.fact,
        InputFact::FullWrite {
            flush: Flush::Unknown
        }
    );
    assert_eq!(snapshot.phases, [PhaseState::Pending; 4]);
    drop(release);
    assert!(joined(pending).await.is_err());
    let snapshot = fixture.snapshot();
    assert_eq!(
        snapshot.fact,
        InputFact::FullWrite {
            flush: Flush::Unknown
        }
    );
    assert_eq!(snapshot.phases[1], PhaseState::Lost);
    assert_eq!(snapshot.phases[2], PhaseState::Settled(PhaseResult::Failed));
    assert!(snapshot.protected);
    pending_application_phases(&snapshot);
}

struct PrefixPanic {
    calls: usize,
    second: Option<Hold>,
}
impl Write for PrefixPanic {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.calls += 1;
        if self.calls == 1 {
            assert_eq!(bytes, b"abc");
            return Ok(1);
        }
        assert_eq!(bytes, b"bc");
        self.second.take().unwrap().wait();
        panic!("injected second write panic");
    }
    fn flush(&mut self) -> io::Result<()> {
        panic!("partial input cannot flush");
    }
}
#[tokio::test]
async fn retained_partial_checkpoint_and_first_call_panic_remain_distinct() {
    let (fixture, reports) = fixture();
    let (hold, observing, release) = gate();
    let pending = start(input(
        Box::new(PrefixPanic {
            calls: 0,
            second: Some(hold),
        }),
        reports,
    ));
    entered(observing).await;
    assert_eq!(
        fixture.snapshot().fact,
        InputFact::PartialOrUnknown { reported: 1 }
    );
    drop(release);
    assert!(joined(pending).await.is_err());
    let snapshot = fixture.snapshot();
    assert_eq!(snapshot.fact, InputFact::PartialOrUnknown { reported: 1 });
    assert!(snapshot.protected);
    pending_application_phases(&snapshot);

    let (fixture, reports) = self::fixture();
    let mut writer = BlockingWriter::plain();
    writer.panic_write = true;
    assert!(joined(start(input(Box::new(writer), reports)))
        .await
        .is_err());
    assert_eq!(fixture.snapshot().fact, InputFact::Unresolved);
}

#[tokio::test]
async fn lost_ack_and_flush_error_retain_full_fact_and_distinct_phase_results() {
    let (fixture, reports) = fixture();
    let mut writer = ScriptedWriter::new([Step::Count(3)]);
    writer.fail_flush = true;
    let (accepted, receiver) = tokio::sync::oneshot::channel();
    drop(receiver);
    let mut request = input(Box::new(writer), reports);
    request.accepted = Some(accepted);
    let (_, outcome) = joined(start(request)).await.unwrap();
    assert_eq!(outcome.legacy_result(), Ok(()));
    assert_eq!(outcome.reporting_error, None);
    let snapshot = fixture.snapshot();
    assert_eq!(
        snapshot.fact,
        InputFact::FullWrite {
            flush: Flush::Failed
        }
    );
    assert_eq!(snapshot.phases[1], PhaseState::Settled(PhaseResult::Failed));
    assert_eq!(
        snapshot.phases[2],
        PhaseState::Settled(PhaseResult::Completed)
    );
    assert!(snapshot.protected);
    pending_application_phases(&snapshot);
}

#[tokio::test]
async fn post_ack_panic_cannot_reverse_terminal_input_or_fake_return() {
    let (fixture, reports) = fixture();
    let (hold, observing, release) = gate();
    let (accepted, receiver) = tokio::sync::oneshot::channel();
    let mut request = input(Box::new(BlockingWriter::plain()), reports);
    request.accepted = Some(accepted);
    request.after_ack = Some(Box::new(move || {
        hold.wait();
        panic!("post ack panic");
    }));
    let pending = start(request);
    entered(observing).await;
    assert_eq!(receiver.await.unwrap(), Ok(()));
    let snapshot = fixture.snapshot();
    assert_eq!(
        snapshot.fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
    assert_eq!(
        snapshot.phases[1],
        PhaseState::Settled(PhaseResult::Completed)
    );
    assert_eq!(snapshot.phases[2], PhaseState::Pending);
    drop(release);
    assert!(joined(pending).await.is_err());
    let snapshot = fixture.snapshot();
    assert_eq!(
        snapshot.fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
    assert_eq!(snapshot.phases[2], PhaseState::Settled(PhaseResult::Failed));
    assert!(snapshot.protected);
    pending_application_phases(&snapshot);
}

#[tokio::test]
async fn pending_and_observer_loss_keep_late_writer_fact_with_return_lost() {
    let (mut fixture, reports) = fixture();
    let (hold, observing, release) = gate();
    let mut writer = BlockingWriter::plain();
    writer.flush = Some(hold);
    let (accepted, receiver) = tokio::sync::oneshot::channel();
    let mut request = input(Box::new(writer), reports);
    request.accepted = Some(accepted);
    let pending = start(request);
    entered(observing).await;
    drop(fixture.observer.take());
    drop(pending);
    drop(release);
    assert_eq!(receiver.await.unwrap(), Ok(()));
    fixture.observer = Some(fixture.owner.observe(fixture.context).unwrap());
    let snapshot = fixture.snapshot();
    assert_eq!(
        snapshot.fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
    assert_eq!(
        snapshot.phases[1],
        PhaseState::Settled(PhaseResult::Completed)
    );
    assert_eq!(snapshot.phases[2], PhaseState::Lost);
    assert!(snapshot.protected);
    pending_application_phases(&snapshot);
    assert_eq!(
        fixture.owner.retire(fixture.observer.take().unwrap()),
        Err(ReportError::Protected)
    );
}

struct CountedWriter {
    hold: Option<Hold>,
    count: usize,
    calls: Arc<std::sync::atomic::AtomicUsize>,
    flushes: Arc<std::sync::atomic::AtomicUsize>,
}
impl Write for CountedWriter {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if let Some(hold) = self.hold.take() {
            hold.wait();
        }
        Ok(self.count)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flushes.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}
#[tokio::test]
async fn unavailable_checkpoint_stops_later_io_without_erasing_direct_bytes() {
    for (count, fact, legacy) in [
        (1, InputFact::PartialOrUnknown { reported: 1 }, Err(())),
        (
            3,
            InputFact::FullWrite {
                flush: Flush::Unknown,
            },
            Ok(()),
        ),
    ] {
        let (fixture, reports) = fixture();
        let (hold, observing, release) = gate();
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let flushes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let pending = start(input(
            Box::new(CountedWriter {
                hold: Some(hold),
                count,
                calls: calls.clone(),
                flushes: flushes.clone(),
            }),
            reports,
        ));
        entered(observing).await;
        input_submission::test_support::poison(&fixture.owner);
        drop(release);
        let (_, outcome) = joined(pending).await.unwrap();
        assert_eq!(outcome.fact, fact);
        assert_eq!(outcome.legacy_result(), legacy);
        assert_eq!(outcome.reporting_error, Some(ReportError::Unavailable));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(flushes.load(Ordering::SeqCst), 0);
        assert_eq!(
            fixture.observer.as_ref().unwrap().snapshot(Instant::now()),
            Err(ReportError::Unavailable)
        );
    }
}

#[tokio::test]
async fn refused_reported_start_returns_all_inputs_before_dispatch() {
    for mode in ["length", "roles", "spent", "core"] {
        let (_fixture, mut reports) = fixture();
        let (_other_fixture, mut other_reports) = fixture();
        let expected = match mode {
            "roles" => {
                reports = (reports.1, reports.0);
                ReportError::WrongRole
            }
            "spent" => {
                reports.1.finish(PhaseResult::Completed).unwrap();
                ReportError::Conflict
            }
            "core" => {
                std::mem::swap(&mut reports.1, &mut other_reports.1);
                ReportError::Stale
            }
            _ => ReportError::InvalidFact,
        };
        let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let flushes = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let mut request = input(
            Box::new(CountedWriter {
                hold: None,
                count: 3,
                calls: calls.clone(),
                flushes: flushes.clone(),
            }),
            reports,
        );
        if mode == "length" {
            request.data.push('!');
        }
        let data = request.data.clone(); // Fixture expectation only, no product payload copy.
        let (accepted, receiver) = tokio::sync::oneshot::channel();
        request.accepted = Some(accepted);
        let (stop_tx, stop_rx) = tokio::sync::watch::channel(true);
        request.stop_rx = stop_rx;
        let refusal = match PendingPtyWrite::start_reported(request) {
            Err(error) => error,
            Ok(_) => panic!("invalid pair dispatched"),
        };
        assert_eq!(refusal.reason, expected);
        assert_eq!(refusal.input.data, data);
        assert!(*refusal.input.stop_rx.borrow());
        refusal.input.accepted.unwrap().send(Err(())).unwrap();
        assert_eq!(receiver.await.unwrap(), Err(()));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        assert_eq!(flushes.load(Ordering::SeqCst), 0);
        drop((stop_tx, refusal.input.writer, refusal.input.reports));
    }
}

#[tokio::test]
async fn empty_reported_input_checkpoints_before_flush() {
    let (fixture, reports) = fixture_for(0);
    let (hold, observing, release) = gate();
    let mut writer = BlockingWriter::plain();
    writer.flush = Some(hold);
    let mut request = input(Box::new(writer), reports);
    request.data.clear();
    let pending = start(request);
    entered(observing).await;
    assert_eq!(
        fixture.snapshot().fact,
        InputFact::FullWrite {
            flush: Flush::Unknown
        }
    );
    assert_eq!(fixture.snapshot().phases, [PhaseState::Pending; 4]);
    drop(release);
    assert!(joined(pending).await.is_ok());
    let snapshot = fixture.snapshot();
    assert_eq!(
        snapshot.phases[1],
        PhaseState::Settled(PhaseResult::Completed)
    );
    assert_eq!(
        snapshot.phases[2],
        PhaseState::Settled(PhaseResult::Completed)
    );
    pending_application_phases(&snapshot);
}

#[tokio::test]
async fn normal_no_effect_and_partial_failure_publish_distinct_writer_results() {
    for stopped in [true, false] {
        let (fixture, reports) = fixture();
        let writer = if stopped {
            ScriptedWriter::new(vec![])
        } else {
            ScriptedWriter::new(vec![Step::Count(1), Step::Error(io::ErrorKind::BrokenPipe)])
        };
        let mut request = input(Box::new(writer), reports);
        request.stop_rx = tokio::sync::watch::channel(stopped).1;
        let (_, outcome) = joined(start(request)).await.unwrap();
        assert_eq!(outcome.legacy_result(), Err(()));
        assert_eq!(outcome.reporting_error, None);
        let snapshot = fixture.snapshot();
        if stopped {
            assert_eq!(snapshot.fact, InputFact::NoInputEffect);
            assert_eq!(
                snapshot.phases[1],
                PhaseState::Settled(PhaseResult::NoEffect)
            );
        } else {
            assert_eq!(snapshot.fact, InputFact::PartialOrUnknown { reported: 1 });
            assert_eq!(snapshot.phases[1], PhaseState::Settled(PhaseResult::Failed));
        }
        assert_eq!(
            snapshot.phases[2],
            PhaseState::Settled(PhaseResult::Completed)
        );
        pending_application_phases(&snapshot);
    }
}

#[tokio::test]
async fn bound_original_lease_survives_coordinator_and_waiter_loss_during_flush() {
    let root = tempfile::tempdir().unwrap();
    let scope = input_submission::scope::tests::capture(root.path());
    let owner = Owner::new(Limits {
        records: 1,
        observers: 1,
        input_bytes: 16,
        backing_bytes: 65536,
    })
    .unwrap();
    let now = Instant::now();
    let Submission {
        reporters: [preparation, writer, returned, bookkeeping],
        observer,
    } = owner
        .admit_bound(
            Context {
                root: 1,
                session: 2,
                incarnation: 3,
                slot: 1,
            },
            3,
            now + Duration::from_secs(60),
            now,
            scope,
        )
        .unwrap_or_else(|e| panic!("{:?}", e.reason));
    let (hold, observing, release) = gate();
    let mut native = BlockingWriter::plain();
    native.flush = Some(hold);
    let pending = start(input(Box::new(native), (writer, returned)));
    entered(observing).await;
    assert_eq!(
        observer.snapshot(Instant::now()).unwrap().fact,
        InputFact::FullWrite {
            flush: Flush::Unknown
        }
    );
    drop((owner, observer, preparation, bookkeeping));
    assert!(crate::workspace_runtime::WorkspaceLease::acquire(root.path()).is_err());
    drop(release);
    let (_, outcome) = joined(pending).await.unwrap();
    assert_eq!(
        outcome.fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
    // Last Core destruction releases the lease, not a claim of whole-job completion.
    assert!(crate::workspace_runtime::WorkspaceLease::acquire(root.path()).is_ok());
}
