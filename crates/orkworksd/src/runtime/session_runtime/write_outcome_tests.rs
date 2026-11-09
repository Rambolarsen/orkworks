use super::*;
use crate::runtime::input_submission::{Flush, InputFact};
use std::collections::VecDeque;
use std::io::{self, Write};

// These adapters control native Write return values; the production loop makes
// every decision. Exact supplied slices detect prefix reuse or skipped bytes.
enum Step {
    Count(usize),
    Error(io::ErrorKind),
    StopAfter(usize),
    InterruptedThenStop,
}
struct ScriptedWriter {
    steps: VecDeque<Step>,
    calls: Vec<Vec<u8>>,
    flushes: usize,
    fail_flush: bool,
    stopped: Arc<AtomicBool>,
}
impl ScriptedWriter {
    fn new(steps: impl IntoIterator<Item = Step>) -> Self {
        Self {
            steps: steps.into_iter().collect(),
            calls: Vec::new(),
            flushes: 0,
            fail_flush: false,
            stopped: Arc::new(AtomicBool::new(false)),
        }
    }
    fn run(&mut self) -> PtyWriteOutcome {
        let stopped = self.stopped.clone();
        write_pty_input(self, b"abc", || stopped.load(Ordering::Acquire))
    }
}
impl Write for ScriptedWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.calls.push(data.to_vec());
        match self.steps.pop_front().expect("unexpected write invocation") {
            Step::Count(n) => Ok(n),
            Step::Error(kind) => Err(kind.into()),
            Step::StopAfter(n) => {
                self.stopped.store(true, Ordering::Release);
                Ok(n)
            }
            Step::InterruptedThenStop => {
                self.stopped.store(true, Ordering::Release);
                Err(io::ErrorKind::Interrupted.into())
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        self.flushes += 1;
        if self.fail_flush {
            Err(io::ErrorKind::Other.into())
        } else {
            Ok(())
        }
    }
}

#[test]
fn full_short_and_interrupted_writes_keep_exact_remaining_bytes() {
    let mut writer = ScriptedWriter::new([
        Step::Count(1),
        Step::Error(io::ErrorKind::Interrupted),
        Step::Count(2),
    ]);
    let outcome = writer.run();
    assert_eq!(outcome.input_bytes, 3);
    assert_eq!(
        outcome.fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
    assert_eq!(outcome.legacy_result(), Ok(()));
    assert_eq!(
        writer.calls,
        [b"abc".to_vec(), b"bc".to_vec(), b"bc".to_vec()]
    );
    assert_eq!(writer.flushes, 1);
}

#[test]
fn errors_and_zero_returns_never_fabricate_no_input_effect() {
    for (steps, prefix, calls) in [
        (
            vec![Step::Error(io::ErrorKind::BrokenPipe)],
            0,
            vec![b"abc".to_vec()],
        ),
        (vec![Step::Count(0)], 0, vec![b"abc".to_vec()]),
        (
            vec![Step::Count(1), Step::Error(io::ErrorKind::BrokenPipe)],
            1,
            vec![b"abc".to_vec(), b"bc".to_vec()],
        ),
        (
            vec![Step::Count(1), Step::Count(0)],
            1,
            vec![b"abc".to_vec(), b"bc".to_vec()],
        ),
    ] {
        let mut writer = ScriptedWriter::new(steps);
        let outcome = writer.run();
        assert_eq!(
            outcome.fact,
            InputFact::PartialOrUnknown { reported: prefix }
        );
        assert_eq!(outcome.legacy_result(), Err(()));
        assert_eq!(writer.calls, calls);
        assert_eq!(writer.flushes, 0);
    }
}

#[test]
fn stop_before_invocation_is_the_only_nonempty_no_effect_result() {
    let mut writer = ScriptedWriter::new([]);
    writer.stopped.store(true, Ordering::Release);
    let outcome = writer.run();
    assert_eq!(outcome.fact, InputFact::NoInputEffect);
    assert_eq!(outcome.legacy_result(), Err(()));
    assert!(writer.calls.is_empty());
    assert_eq!(writer.flushes, 0);

    let mut writer = ScriptedWriter::new([Step::InterruptedThenStop]);
    let outcome = writer.run();
    assert_eq!(outcome.fact, InputFact::PartialOrUnknown { reported: 0 });
    assert_eq!(outcome.legacy_result(), Err(()));
    assert_eq!(writer.calls, [b"abc".to_vec()]);
}

#[test]
fn stop_after_partial_and_final_write_preserves_actual_byte_truth() {
    let mut writer = ScriptedWriter::new([Step::StopAfter(1)]);
    let outcome = writer.run();
    assert_eq!(outcome.fact, InputFact::PartialOrUnknown { reported: 1 });
    assert_eq!(outcome.legacy_result(), Err(()));
    assert_eq!(writer.flushes, 0);
    assert_eq!(writer.calls, [b"abc".to_vec()]);

    let mut writer = ScriptedWriter::new([Step::StopAfter(3)]);
    let outcome = writer.run();
    assert_eq!(
        outcome.fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
    assert_eq!(outcome.legacy_result(), Ok(()));
    assert_eq!(writer.flushes, 1);
    assert_eq!(writer.calls, [b"abc".to_vec()]);
}

#[test]
fn malformed_counts_cannot_increase_known_prefix_or_panic() {
    for (steps, prefix) in [
        (vec![Step::Count(4)], 0),
        (vec![Step::Count(1), Step::Count(usize::MAX)], 1),
    ] {
        let mut writer = ScriptedWriter::new(steps);
        let outcome = writer.run();
        assert_eq!(
            outcome.fact,
            InputFact::PartialOrUnknown { reported: prefix }
        );
        assert_eq!(outcome.legacy_result(), Err(()));
        assert_eq!(writer.flushes, 0);
    }
}

#[test]
fn flush_failure_keeps_full_write_and_successful_legacy_ack() {
    let mut writer = ScriptedWriter::new([Step::Count(3)]);
    writer.fail_flush = true;
    let outcome = writer.run();
    assert_eq!(
        outcome.fact,
        InputFact::FullWrite {
            flush: Flush::Failed
        }
    );
    assert_eq!(outcome.legacy_result(), Ok(()));
    assert_eq!(writer.calls, [b"abc".to_vec()]);
    assert_eq!(writer.flushes, 1);
}

#[test]
fn empty_input_has_zero_byte_truth_and_actual_flush_disposition() {
    for (fail, flush) in [(false, Flush::Succeeded), (true, Flush::Failed)] {
        let mut writer = ScriptedWriter::new([]);
        writer.fail_flush = fail;
        let outcome = write_pty_input(&mut writer, b"", || true);
        assert_eq!(outcome.input_bytes, 0);
        assert_eq!(outcome.fact, InputFact::FullWrite { flush });
        assert_eq!(outcome.legacy_result(), Ok(()));
        assert!(writer.calls.is_empty());
        assert_eq!(writer.flushes, 1);
    }
}

// Release-on-drop makes assertion failures unblock their physical test worker.
struct Release(Option<std::sync::mpsc::SyncSender<()>>);
impl Drop for Release {
    fn drop(&mut self) {
        if let Some(sender) = self.0.take() {
            let _ = sender.try_send(());
        }
    }
}
struct Hold {
    entered: tokio::sync::oneshot::Sender<()>,
    released: std::sync::mpsc::Receiver<()>,
}
impl Hold {
    fn wait(self) {
        let _ = self.entered.send(());
        self.released
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("fixture release watchdog");
    }
}
fn gate() -> (Hold, tokio::sync::oneshot::Receiver<()>, Release) {
    let (entered, observing) = tokio::sync::oneshot::channel();
    let (release, released) = std::sync::mpsc::sync_channel(1);
    (
        Hold { entered, released },
        observing,
        Release(Some(release)),
    )
}
async fn entered(receiver: tokio::sync::oneshot::Receiver<()>) {
    tokio::time::timeout(std::time::Duration::from_secs(10), receiver)
        .await
        .unwrap()
        .unwrap();
}
struct BlockingWriter {
    write: Option<Hold>,
    flush: Option<Hold>,
    panic_write: bool,
    panic_flush: bool,
}
impl BlockingWriter {
    fn plain() -> Self {
        Self {
            write: None,
            flush: None,
            panic_write: false,
            panic_flush: false,
        }
    }
}
impl Write for BlockingWriter {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if let Some(hold) = self.write.take() {
            hold.wait();
        }
        assert!(!self.panic_write, "injected write panic");
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        if let Some(hold) = self.flush.take() {
            hold.wait();
        }
        assert!(!self.panic_flush, "injected flush panic");
        Ok(())
    }
}

#[tokio::test]
async fn lost_ack_receiver_cannot_erase_the_direct_join_result() {
    let (accepted, receiver) = tokio::sync::oneshot::channel();
    drop(receiver);
    let mut pending = PendingPtyWrite::start(
        Box::new(BlockingWriter::plain()),
        "abc".into(),
        Some(accepted),
        tokio::sync::watch::channel(false).1,
    );
    let (_, outcome) = (&mut pending.task).await.unwrap();
    assert_eq!(outcome.input_bytes, 3);
    assert_eq!(
        outcome.fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
}

#[tokio::test]
async fn acknowledgement_does_not_certify_held_blocking_closure_return() {
    let (hold, observing, release) = gate();
    let (accepted, receiver) = tokio::sync::oneshot::channel();
    let mut pending = PendingPtyWrite::start_inner(
        Box::new(BlockingWriter::plain()),
        "abc".into(),
        Some(accepted),
        tokio::sync::watch::channel(false).1,
        Some(Box::new(move || hold.wait())),
    );
    entered(observing).await;
    assert_eq!(receiver.await.unwrap(), Ok(()));
    assert!(
        !pending.task.is_finished(),
        "the exact closure is held after ack"
    );
    drop(release);
    let (_, outcome) = (&mut pending.task).await.unwrap();
    assert_eq!(
        outcome.fact,
        InputFact::FullWrite {
            flush: Flush::Succeeded
        }
    );
}

#[tokio::test]
async fn stop_during_native_call_cannot_fake_return_or_undo_final_bytes() {
    for in_flush in [false, true] {
        let (hold, observing, release) = gate();
        let mut writer = BlockingWriter::plain();
        if in_flush {
            writer.flush = Some(hold);
        } else {
            writer.write = Some(hold);
        }
        let (accepted, mut receiver) = tokio::sync::oneshot::channel();
        let mut pending = PendingPtyWrite::start(
            Box::new(writer),
            "abc".into(),
            Some(accepted),
            tokio::sync::watch::channel(false).1,
        );
        entered(observing).await;
        pending.cancel();
        assert!(
            !pending.task.is_finished(),
            "entered native call has not returned"
        );
        assert!(matches!(
            receiver.try_recv(),
            Err(tokio::sync::oneshot::error::TryRecvError::Empty)
        ));
        drop(release);
        let (_, outcome) = (&mut pending.task).await.unwrap();
        assert_eq!(receiver.await.unwrap(), Ok(()));
        assert_eq!(
            outcome.fact,
            InputFact::FullWrite {
                flush: Flush::Succeeded
            }
        );
    }
}

#[tokio::test]
async fn writer_and_flush_panics_remain_join_errors_without_fabricated_facts() {
    for in_flush in [false, true] {
        let mut writer = BlockingWriter::plain();
        writer.panic_write = !in_flush;
        writer.panic_flush = in_flush;
        let (accepted, receiver) = tokio::sync::oneshot::channel();
        let mut pending = PendingPtyWrite::start(
            Box::new(writer),
            "abc".into(),
            Some(accepted),
            tokio::sync::watch::channel(false).1,
        );
        assert!(
            receiver.await.is_err(),
            "panic cannot fabricate a compatibility ack"
        );
        let result = (&mut pending.task).await;
        assert!(matches!(result, Err(error) if error.is_panic()));
    }
}
