---
type: Qualification Evidence
title: Codex capture descriptor inheritance on macOS
description: Temporary-file native evidence for a received-descriptor inheritance blocker before actual capture observer implementation.
tags: [orkworks, review, codex, macos, custody]
status: stable
---

# Codex capture descriptor inheritance on macOS

Date: 2026-11-22. Source context: `68b77224`, following owner approval of
[#1972](https://github.com/Rambolarsen/orkworks/pull/1972)'s capture architecture.
Track the unresolved qualification blocker in
[#1981](https://github.com/Rambolarsen/orkworks/issues/1981), under #1966.
This is a native temporary-file/standalone-Rust experiment, not an implemented
observer, race-free protocol, sidecar spawn audit or native Codex qualification.

## Executed observation

Host: macOS 26.6.2 arm64; rustc 1.96.0 (ac68faa20 2026-05-25).
A Python standard-library Unix datagram socket pair transferred one temporary-file
descriptor through SCM_RIGHTS. The sender descriptor was non-inheritable; the
received descriptor was inheritable. Python handed that received descriptor to
a minimal Rust parent with `pass_fds`; Rust then spawned its own child through
ordinary `std::process::Command`, with the child environment cleared.

```text
sender flags 1
received flags 0
parent descriptor flags 0
child descriptor flags 0
parent-set descriptor flags 1
child descriptor flags -1
```

Flags are macOS `fcntl(F_GETFD)`: 1 means FD_CLOEXEC, 0 means open/inheritable,
and -1 in this controlled child means the tested descriptor was not open.
The control sets FD_CLOEXEC **inside the Rust parent** before spawning. Setting
it only in Python before `pass_fds` would not be a valid control because
`pass_fds` deliberately permits that descriptor in its child. No concurrency
was exercised here: the two serial spawn orders expose why a receive-then-set
sequence alone cannot establish exclusion against another spawning thread.

Python did not expose MSG_CMSG_CLOEXEC, and the inspected SDK socket header
exposed SCM_RIGHTS/MSG_DONTWAIT but no matching MSG_CMSG_CLOEXEC macro. This is
not proof that every possible macOS API lacks an atomic mechanism. Such an
alternative needs actual supported-API and native-boundary qualification.

## Reproduction

Compile the following as a standalone Rust 2021 executable on macOS; it reads
only descriptor metadata. It is not a sidecar private worker implementation.

```rust
use std::process::Command;
unsafe extern "C" { fn fcntl(fd: i32, cmd: i32, ...) -> i32; }
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let fd: i32 = args[2].parse().unwrap();
    let command: i32 = args[3].parse().unwrap();
    if args[1] == "parent-set" {
        // SAFETY: macOS F_SETFD=2 sets FD_CLOEXEC=1 on this live fixture descriptor.
        assert_eq!(unsafe { fcntl(fd, 2, 1) }, 0);
    }
    // SAFETY: F_GETFD inspects the numeric descriptor without claiming ownership.
    let flags = unsafe { fcntl(fd, command) };
    println!("{} descriptor flags {}", args[1], flags);
    if args[1].starts_with("parent") {
        let output = Command::new(std::env::current_exe().unwrap())
            .env_clear()
            .args(["child", &args[2], &args[3]])
            .output().unwrap();
        print!("{}", String::from_utf8(output.stdout).unwrap());
        assert!(output.status.success());
    }
}

```

For example, save it as `/private/tmp/orkworks-capture-fd-probe.rs` and compile:

```sh
rtk proxy rustc --edition 2021 /private/tmp/orkworks-capture-fd-probe.rs -o /private/tmp/orkworks-capture-fd-probe
```

Then run this Python through `rtk proxy python3`. It transfers a temporary-file
handle, prints no file contents/paths or inherited environment, waits for both
Rust processes and closes every received descriptor/socket on return.

```python
import os, socket, tempfile, array, subprocess, fcntl
with tempfile.TemporaryFile() as original:
    os.set_inheritable(original.fileno(), False)
    a, b = socket.socketpair(socket.AF_UNIX, socket.SOCK_DGRAM)
    a.sendmsg([b'Q'], [(socket.SOL_SOCKET, socket.SCM_RIGHTS,
                      array.array('i', [original.fileno()]))])
    _, controls, _, _ = b.recvmsg(1, socket.CMSG_SPACE(array.array('i').itemsize))
    fds = array.array('i')
    for level, kind, raw in controls:
        if level == socket.SOL_SOCKET and kind == socket.SCM_RIGHTS:
            fds.frombytes(raw[:len(raw) - len(raw) % fds.itemsize])
    try:
        assert len(fds) == 1
        print('sender flags', fcntl.fcntl(original.fileno(), fcntl.F_GETFD))
        print('received flags', fcntl.fcntl(fds[0], fcntl.F_GETFD))
        for mode in ['parent', 'parent-set']:
            print(subprocess.check_output([
                '/private/tmp/orkworks-capture-fd-probe', mode,
                str(fds[0]), str(fcntl.F_GETFD)],
                pass_fds=tuple(fds), env={}, text=True), end='')
    finally:
        for fd in fds:
            os.close(fd)
        a.close()
        b.close()
```

## Disposition and required next proof

**Investigate: descriptor transfer into a multithreaded sidecar is unqualified.**
No actual captured resource was exposed by OrkWorks: the observer does not exist,
and only synthetic temporary-file descriptors were used. No installed hooks,
user configuration, coding tool, provider or runtime capability was inspected.
No product code or new process route is added by this evidence document.

The future receiver must prove that received directory/file custody cannot be
inherited by an unrelated concurrent child before close-on-exec is established.
Do not claim that source O_CLOEXEC, receiver fcntl after return, a socket's own
close-on-exec flag, successful same-object transfer or a serial child test proves
that boundary. Actual std/tokio/probe/PTY/helper spawn paths need audit; this
experiment does not claim they all behave like this minimal child.

A complete receive/spawn exclusion mechanism would need every relevant spawn
path and manual-launch compatibility covered. Keeping descriptors inside a
separate receiver/owner process instead changes owner lifetime, preview custody,
process/resource quotas and cancellation/drain. Neither is silently selected
here. #1981 must compare a concrete supported mechanism and obtain approval of
any material architecture change before dependent code. Existing #1972 approval
remains valid for its stated architecture, not either unreviewed resolution.
The fixed five-item inventory and merged fragment/isolation components remain
valid; actual observer, mutation enrollment, producer and Start remain gated.

An independent delta correctness review checked the reproduction/control and
approval/contract links. It found no material correction; it specifically verified
that serial descriptor evidence is not promoted to atomic receipt, concurrent
sidecar spawn exclusion or complete observer custody. No optional finding was
reported. The native result remains Investigate under #1981.
