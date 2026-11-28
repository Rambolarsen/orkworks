#![cfg(target_os = "macos")]

use std::io;
use std::os::fd::{AsRawFd, RawFd};
use std::os::unix::net::UnixStream;
use std::process::{Command, ExitStatus, Stdio};
use std::thread;
use std::time::{Duration, Instant};

#[path = "../src/taskmaster/codex_capture_owner/channels.rs"]
mod channels;

#[path = "../src/taskmaster/codex_capture_owner/bootstrap_frame.rs"]
mod bootstrap_frame;

#[path = "../src/taskmaster/codex_capture_owner/deadline_io.rs"]
mod deadline_io;

#[path = "support/custody_worker_probe.rs"]
mod worker_probe;

const FIXTURE_FDS: &str = "ORKWORKS_CUSTODY_FIXTURE_FDS";
const CHILD_CHECKED: i32 = 42;

#[test]
fn four_empty_channels_are_sealed_before_child_exec() {
    verify_partial_pairs_are_closed();
    let pairs = channels::seal_empty_channels().expect("seal empty worker channels");
    assert_eq!(pairs.len(), 4);
    let descriptors: Vec<_> = pairs
        .iter()
        .flat_map(|(parent, child)| [parent.as_raw_fd(), child.as_raw_fd()])
        .collect();
    assert!(descriptors.iter().all(|fd| *fd > 2));

    verify_closed_in_exec_child(&descriptors, "check_sealed_descriptors_after_exec")
        .expect("fixture child must confirm all descriptor checks");

    for &fd in &descriptors {
        // SAFETY: the parent still owns each of these descriptors.
        let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
        assert!(flags >= 0 && flags & libc::FD_CLOEXEC != 0);
    }

    // Keep the negative control after sealing and the first exec. A separate
    // libtest case could spawn concurrently with the startup check.
    let empty_selection = verify_closed_in_exec_child(&descriptors, "no_such_descriptor_check")
        .expect_err("empty child selection must not count as verification");
    assert_eq!(empty_selection.code(), Some(0));

    worker_probe::check_owned_workers(&pairs);
    worker_probe::verify_exact_stream_frames();
}

fn verify_closed_in_exec_child(descriptors: &[RawFd], filter: &str) -> Result<(), ExitStatus> {
    let expected_closed = descriptors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",");
    let mut child = Command::new(std::env::current_exe().expect("fixture executable"))
        .args(["--exact", filter, "--ignored"])
        .env_clear()
        .env(FIXTURE_FDS, expected_closed)
        .stdin(Stdio::null())
        .spawn()
        .expect("start the ordinary fixture child");
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Libtest returns success when its filter selects no tests.
                return if status.code() == Some(CHILD_CHECKED) {
                    Ok(())
                } else {
                    Err(status)
                };
            }
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            result => {
                let _ = child.kill();
                child.wait().expect("reap the fixture child");
                panic!("fixture child did not finish: {result:?}");
            }
        }
    }
}

fn verify_partial_pairs_are_closed() {
    let mut opened = Vec::new();
    let result = channels::seal_empty_channels_using(|| {
        if opened.len() == 4 {
            return Err(io::Error::other("fixture pair allocation failed"));
        }
        let (parent, child) = UnixStream::pair()?;
        opened.extend([parent.as_raw_fd(), child.as_raw_fd()]);
        Ok((parent, child))
    });
    assert_eq!(
        result
            .expect_err("partial channel setup must fail")
            .to_string(),
        "fixture pair allocation failed"
    );
    assert_eq!(opened.len(), 4);
    for fd in opened {
        assert_closed(fd);
    }
}

#[test]
#[ignore = "run only in an exec child launched by the channel fixture"]
fn check_sealed_descriptors_after_exec() {
    let descriptors: Vec<RawFd> = std::env::var(FIXTURE_FDS)
        .expect("fixture descriptors")
        .split(',')
        .map(|fd| fd.parse().expect("fixture descriptor number"))
        .collect();
    assert_eq!(descriptors.len(), 8);
    for fd in descriptors {
        assert_closed(fd);
    }
    std::process::exit(CHILD_CHECKED);
}

#[test]
#[ignore = "runs only as a selected synthetic worker"]
fn check_selected_worker_channel() {
    worker_probe::run_child();
}

fn assert_closed(fd: RawFd) {
    // SAFETY: F_GETFD takes no pointer and an invalid descriptor is
    // reported as EBADF. No descriptor is adopted or closed here.
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFD) };
    let error = io::Error::last_os_error();
    assert_eq!(flags, -1, "descriptor {fd} remains open");
    assert_eq!(error.raw_os_error(), Some(libc::EBADF));
}

#[test]
fn bootstrap_frames_bind_message_direction_nonce_and_build() {
    use bootstrap_frame::{encode, matches, Direction};

    let nonce = [17; 16];
    let build = [34; 32];
    let mut hello = b"OWCO\x01\x01".to_vec();
    hello.extend([17; 16]);
    hello.extend([34; 32]);
    assert_eq!(
        encode(Direction::Challenge, &nonce, &build).as_slice(),
        hello
    );
    assert!(matches(&hello, Direction::Challenge, &nonce, &build));
    assert!(!matches(&hello, Direction::Ready, &nonce, &build));

    let mut ready = hello.clone();
    ready[5] = 2;
    assert_eq!(encode(Direction::Ready, &nonce, &build).as_slice(), ready);
    assert!(matches(&ready, Direction::Ready, &nonce, &build));

    for offset in [0, 4, 5, 6, 22] {
        let mut changed = ready.clone();
        changed[offset] ^= 16;
        assert!(!matches(&changed, Direction::Ready, &nonce, &build));
    }
    for length in 0..ready.len() {
        assert!(!matches(&ready[..length], Direction::Ready, &nonce, &build));
    }
    ready.push(0);
    assert!(!matches(&ready, Direction::Ready, &nonce, &build));
}
