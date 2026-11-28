#![cfg(all(target_os = "macos", target_arch = "aarch64", debug_assertions))]

use std::cell::RefCell;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::{
    net::UnixStream,
    process::{CommandExt, ExitStatusExt},
};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

#[path = "../src/taskmaster/codex_capture_owner/bootstrap_frame.rs"]
mod bootstrap_frame;
#[path = "../src/taskmaster/codex_capture_owner/channels.rs"]
mod channels;
#[path = "../src/taskmaster/codex_capture_owner/deadline_io.rs"]
mod deadline_io;
#[path = "../src/taskmaster/codex_capture_owner/launch.rs"]
mod launch;
#[path = "../src/taskmaster/codex_capture_owner/qualification_contract.rs"]
mod qualification_contract;
#[path = "../src/taskmaster/codex_capture_owner/slots.rs"]
mod slots;

use bootstrap_frame::{Direction, FRAME_SIZE};
use launch::{LaunchOwner, SpawnedAttempt};
use qualification_contract::{qualification_tag, WORKER_ARG};
use slots::{AttemptOwner, StartupSlots};

#[test]
#[ignore = "requires the explicit compile-time macOS qualification build"]
fn sidecar_worker_keeps_its_channel_after_startup_and_drains_on_eof() {
    assert!(
        qualification_contract::enabled(),
        "rebuild with ORKWORKS_CUSTODY_QUALIFICATION=1"
    );
    let [ready, wrong_tag, extra_data, buffered_data] = channels::seal_empty_channels().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let tag = qualification_tag();

    let (mut stream, mut child) = start(ready, deadline).unwrap();
    exchange(&mut stream, &tag, deadline);
    assert!(
        child.0.try_wait().unwrap().is_none(),
        "ready worker must stay alive"
    );
    drop(stream);
    assert!(child.wait_until(deadline).unwrap().success());

    let (mut stream, mut child) = start(wrong_tag, deadline).unwrap();
    let nonce = *uuid::Uuid::new_v4().as_bytes();
    let mut mismatched_tag = tag;
    mismatched_tag[0] ^= 1;
    deadline_io::write_all(
        &mut stream,
        &bootstrap_frame::encode(Direction::Challenge, &nonce, &mismatched_tag),
        deadline,
    )
    .unwrap();
    expect_eof(&mut stream, deadline);
    assert_eq!(child.wait_until(deadline).unwrap().code(), Some(2));

    let (mut stream, mut child) = start(extra_data, deadline).unwrap();
    exchange(&mut stream, &tag, deadline);
    deadline_io::write_all(&mut stream, &[0], deadline).unwrap();
    expect_eof(&mut stream, deadline);
    assert_eq!(child.wait_until(deadline).unwrap().code(), Some(2));

    // Queue the complete frame and surplus byte before exec so the case
    // never depends on scheduler order.
    buffered_data.0.set_nonblocking(true).unwrap();
    let nonce = *uuid::Uuid::new_v4().as_bytes();
    let mut bytes = bootstrap_frame::encode(Direction::Challenge, &nonce, &tag).to_vec();
    bytes.push(0);
    deadline_io::write_all(&buffered_data.0, &bytes, deadline).unwrap();
    let (mut stream, mut child) = start(buffered_data, deadline).unwrap();
    expect_eof(&mut stream, deadline);
    assert_eq!(child.wait_until(deadline).unwrap().code(), Some(2));

    check_entry_rejections(&tag);
}

#[test]
#[ignore = "requires the explicit compile-time macOS qualification build"]
fn sidecar_workers_reach_readiness_and_drain_independently() {
    assert!(
        qualification_contract::enabled(),
        "rebuild with ORKWORKS_CUSTODY_QUALIFICATION=1"
    );
    let pairs = channels::seal_empty_channels().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let tag = qualification_tag();

    // Start all four before any can receive a challenge.
    let mut workers = pairs
        .into_iter()
        .map(|pair| start(pair, deadline).unwrap())
        .collect::<Vec<_>>();
    for (stream, _) in &mut workers {
        exchange(stream, &tag, deadline);
    }

    for (_, child) in &mut workers {
        assert!(
            child.0.try_wait().unwrap().is_none(),
            "worker exited after readiness"
        );
    }
    while let Some((stream, mut child)) = workers.pop() {
        drop(stream);
        assert!(child.wait_until(deadline).unwrap().success());
        for (_, sibling) in &mut workers {
            assert!(
                sibling.0.try_wait().unwrap().is_none(),
                "closing another worker's channel ended this worker"
            );
        }
    }
}

#[test]
#[ignore = "requires the explicit compile-time macOS qualification build"]
fn sidecar_failed_slot_is_charged_until_child_reap() {
    assert!(qualification_contract::enabled());
    let pairs = channels::seal_empty_channels().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    let tag = qualification_tag();
    let mut slots = StartupSlots::new(deadline);

    for pair in pairs {
        let (slot, attempt) = slots.begin(Instant::now(), SlotWorker::default).unwrap();
        attempt.start_and_exchange(pair, &tag, deadline);
        assert!(slots.publish_ready(slot, Instant::now()));
    }
    slots.close_startup();

    let failed = 1;
    assert!(slots.drain(failed).is_some());
    assert!(slots.poll_reap(failed).unwrap().is_none());
    assert_eq!(slots.ready_count(), 3);
    assert_eq!(slots.charged_count(), 4);
    assert!(slots
        .begin(Instant::now(), || panic!("replacement"))
        .is_none());

    let cleanup_deadline = Instant::now() + Duration::from_secs(3);
    slots.drain(failed).unwrap().terminate();
    assert_eq!(
        wait_for_slot_reap(&mut slots, failed, cleanup_deadline).signal(),
        Some(libc::SIGKILL)
    );
    assert_eq!(slots.charged_count(), 3);
    assert_eq!(slots.ready_count(), 3);
    assert!(slots.drain(failed).is_none());

    for slot in [0, 2, 3] {
        slots.drain(slot).unwrap().close();
        assert!(wait_for_slot_reap(&mut slots, slot, cleanup_deadline).success());
    }
    assert_eq!(slots.charged_count(), 0);
    assert!(slots
        .begin(Instant::now(), || panic!("replacement"))
        .is_none());
}

#[test]
#[ignore = "requires the explicit compile-time macOS qualification build"]
fn sidecar_late_launch_stays_owned_after_startup_deadline() {
    assert!(qualification_contract::enabled());
    let deadline = Instant::now() + Duration::from_secs(3);
    let [healthy_pair, delayed_pair, _unused_one, _unused_two] =
        channels::seal_empty_channels().unwrap();
    let tag = qualification_tag();
    let mut slots = StartupSlots::new(deadline);
    let (healthy, attempt) = slots.begin(Instant::now(), LaunchOwner::default).unwrap();
    attempt
        .start(move || SlotWorker::spawn(healthy_pair))
        .unwrap();
    loop {
        assert!(Instant::now() < deadline);
        if let Some(worker) = attempt.poll_spawned().unwrap() {
            worker.confirm_readiness(&tag, deadline);
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    assert!(slots.publish_ready(healthy, Instant::now()));

    let (release, gate) = std::sync::mpsc::channel();
    let (entered, entry) = std::sync::mpsc::channel();
    let (delayed, attempt) = slots.begin(Instant::now(), LaunchOwner::default).unwrap();
    attempt
        .start(move || {
            entered.send(()).map_err(io::Error::other)?;
            gate.recv_timeout(Duration::from_secs(10))
                .map_err(io::Error::other)?;
            SlotWorker::spawn(delayed_pair)
        })
        .unwrap();
    entry
        .recv_timeout(deadline.saturating_duration_since(Instant::now()))
        .unwrap();
    assert!(attempt.poll_spawned().unwrap().is_none());

    std::thread::sleep(deadline.saturating_duration_since(Instant::now()));
    assert!(slots
        .begin(Instant::now(), || panic!("startup closed"))
        .is_none());
    assert!(!slots.publish_ready(delayed, Instant::now()));
    assert_eq!(slots.ready_count(), 1);
    assert_eq!(slots.charged_count(), 2);
    assert!(slots.poll_reap(delayed).unwrap().is_none());

    let cleanup_deadline = Instant::now() + Duration::from_secs(3);
    release.send(()).unwrap();
    assert_eq!(
        wait_for_slot_reap(&mut slots, delayed, cleanup_deadline).code(),
        Some(2)
    );
    assert_eq!(slots.charged_count(), 1);
    assert_eq!(slots.ready_count(), 1);
    assert!(!slots.publish_ready(delayed, Instant::now()));
    slots.drain(healthy).unwrap();
    assert!(wait_for_slot_reap(&mut slots, healthy, cleanup_deadline).success());
    assert_eq!(slots.charged_count(), 0);
    assert!(slots
        .begin(Instant::now(), || panic!("replacement"))
        .is_none());
}

#[derive(Default)]
struct SlotWorker {
    running: RefCell<Option<RunningSlotWorker>>,
}

struct RunningSlotWorker {
    controller: Option<UnixStream>,
    child: OwnedChild,
}

impl SlotWorker {
    fn spawn(pair: (UnixStream, UnixStream)) -> io::Result<Self> {
        let (controller, child) = spawn_with_input(pair, LaunchInput::Normal)?;
        Ok(Self {
            running: RefCell::new(Some(RunningSlotWorker {
                controller: Some(controller),
                child,
            })),
        })
    }

    fn start_and_exchange(
        &self,
        pair: (UnixStream, UnixStream),
        tag: &[u8; 32],
        deadline: Instant,
    ) {
        assert!(self.running.borrow().is_none());
        let (controller, child) = start(pair, deadline).unwrap();
        *self.running.borrow_mut() = Some(RunningSlotWorker {
            controller: Some(controller),
            child,
        });
        self.confirm_readiness(tag, deadline);
    }

    fn confirm_readiness(&self, tag: &[u8; 32], deadline: Instant) {
        let mut running = self.running.borrow_mut();
        let worker = running.as_mut().unwrap();
        exchange(worker.controller.as_mut().unwrap(), tag, deadline);
        assert!(worker.child.0.try_wait().unwrap().is_none());
    }

    fn terminate(&self) {
        let mut running = self.running.borrow_mut();
        running.as_mut().unwrap().child.0.kill().unwrap();
    }

    fn close(&self) {
        let mut running = self.running.borrow_mut();
        let worker = running.as_mut().unwrap();
        assert!(worker.child.0.try_wait().unwrap().is_none());
        drop(worker.controller.take());
    }
}

impl AttemptOwner for SlotWorker {
    type Exit = ExitStatus;

    fn try_reap(&mut self) -> io::Result<Option<ExitStatus>> {
        match self.running.get_mut() {
            Some(worker) => worker.child.0.try_wait(),
            None => Ok(None),
        }
    }
}

impl SpawnedAttempt for SlotWorker {
    fn close_channel(&mut self) {
        if let Some(worker) = self.running.get_mut() {
            drop(worker.controller.take());
        }
    }
}

fn wait_for_slot_reap<T: AttemptOwner<Exit = ExitStatus>>(
    slots: &mut StartupSlots<T>,
    slot: usize,
    deadline: Instant,
) -> ExitStatus {
    while Instant::now() < deadline {
        if let Some(status) = slots.poll_reap(slot).unwrap() {
            return status;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    panic!("worker slot {slot} did not reap before the cleanup deadline");
}

fn check_entry_rejections(tag: &[u8; 32]) {
    // These are independent refusal checks after the startup trials have
    // closed and reaped. All blank pairs are sealed before either launch.
    let [env_case, inventory_case, fixture_only, _unused] =
        channels::seal_empty_channels().unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    for (pair, input) in [
        (env_case, LaunchInput::ExtraEnvironment),
        (inventory_case, LaunchInput::ExtraEndpoint(&fixture_only.1)),
    ] {
        let nonce = *uuid::Uuid::new_v4().as_bytes();
        pair.0.set_nonblocking(true).unwrap();
        let challenge = bootstrap_frame::encode(Direction::Challenge, &nonce, tag);
        deadline_io::write_all(&pair.0, &challenge, deadline).unwrap();
        let (mut stream, mut child) = start_with_input(pair, input, deadline).unwrap();
        expect_eof(&mut stream, deadline);
        assert_eq!(child.wait_until(deadline).unwrap().code(), Some(2));
    }
}

fn worker_command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_orkworksd"));
    command
        .arg(WORKER_ARG)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command
}

enum LaunchInput<'a> {
    Normal,
    ExtraEnvironment,
    ExtraEndpoint(&'a UnixStream),
}

fn start(
    pair: (UnixStream, UnixStream),
    deadline: Instant,
) -> io::Result<(UnixStream, OwnedChild)> {
    start_with_input(pair, LaunchInput::Normal, deadline)
}

fn start_with_input(
    pair: (UnixStream, UnixStream),
    input: LaunchInput<'_>,
    deadline: Instant,
) -> io::Result<(UnixStream, OwnedChild)> {
    assert!(Instant::now() < deadline);
    let owner = spawn_with_input(pair, input)?;
    assert!(Instant::now() < deadline);
    Ok(owner)
}

fn spawn_with_input(
    (controller, worker): (UnixStream, UnixStream),
    input: LaunchInput<'_>,
) -> io::Result<(UnixStream, OwnedChild)> {
    let mut command = worker_command();
    let fd = worker.as_raw_fd();
    command.arg(fd.to_string());
    let additional = match input {
        LaunchInput::Normal => None,
        LaunchInput::ExtraEnvironment => {
            command.env("ORKWORKS_QUALIFICATION_UNEXPECTED", "fixture");
            None
        }
        LaunchInput::ExtraEndpoint(endpoint) => Some(endpoint.as_raw_fd()),
    };
    // SAFETY: each selected test endpoint stays open through spawn, and fcntl
    // is async-signal-safe. Only the explicitly selected fixture descriptors
    // have their child flags changed.
    unsafe {
        command.pre_exec(move || {
            for fd in [Some(fd), additional].into_iter().flatten() {
                if libc::fcntl(fd, libc::F_SETFD, 0) < 0 {
                    return Err(io::Error::last_os_error());
                }
            }
            Ok(())
        });
    }
    let child = OwnedChild(command.spawn()?);
    controller.set_nonblocking(true)?;
    Ok((controller, child))
}

fn exchange(stream: &mut UnixStream, tag: &[u8; 32], deadline: Instant) {
    let nonce = *uuid::Uuid::new_v4().as_bytes();
    deadline_io::write_all(
        stream,
        &bootstrap_frame::encode(Direction::Challenge, &nonce, tag),
        deadline,
    )
    .unwrap();
    let mut answer = [0; FRAME_SIZE];
    deadline_io::read_exact(stream, &mut answer, deadline).unwrap();
    assert!(Instant::now() < deadline);
    assert!(bootstrap_frame::matches(
        &answer,
        Direction::Ready,
        &nonce,
        tag
    ));
}

fn expect_eof(stream: &mut UnixStream, deadline: Instant) {
    match deadline_io::read(stream, &mut [0], deadline) {
        Ok(0) => {}
        Err(error) if error.kind() == io::ErrorKind::ConnectionReset => {}
        result => panic!("worker sent data or failed to close: {result:?}"),
    }
}

struct OwnedChild(Child);

impl OwnedChild {
    fn wait_until(&mut self, deadline: Instant) -> io::Result<ExitStatus> {
        while Instant::now() < deadline {
            if let Some(status) = self.0.try_wait()? {
                return Ok(status);
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        Err(io::ErrorKind::TimedOut.into())
    }
}

impl Drop for OwnedChild {
    fn drop(&mut self) {
        if !matches!(self.0.try_wait(), Ok(Some(_))) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}
