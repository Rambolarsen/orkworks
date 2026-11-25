---
type: Qualification Evidence
title: Codex custody spawn audit and isolated receiver witness
description: Source launch-boundary audit and finite macOS process-separation evidence for the unresolved descriptor custody gate.
tags: [orkworks, review, codex, custody, macos]
status: stable
---

# Codex custody spawn audit and isolated receiver witness

Date: 2026-11-22. Audit base: `5320f358`.
Track [#1981](https://github.com/Rambolarsen/orkworks/issues/1981).
The [earlier native observation](ordinary-review-codex-descriptor-qualification.md)
proved inheritable receipt, not a complete sidecar worker or spawn census.
This follow-up audits the production launch families and compares an isolated
receiver primitive. It implements no observer, startup pool or native coding-tool
launch; the [proposed resolution](../superpowers/specs/2026-11-22-review-codex-custody-owner-design.md)
still needs owner approval and actual source/lifecycle qualification.

## Actual production launch boundaries

Paths below are relative to `crates/orkworksd/src/` at the audit base. The search
covered Command construction, spawn/output/status dispatch, PTY builders,
fork/posix_spawn, descriptor flags and potential common exclusion. Test-only
cases and HTTP status methods were excluded after inspecting their context.
This is a source-family audit, not tracing every dependency syscall.

| Production family | Actual source boundary | What its existing guard supplies |
| --- | --- | --- |
| Tool/help/OpenCode store probes | harness/detect.rs:134, runtime/codex_launch.rs:37, opencode_session_store.rs:28 → probe_process.rs:391 | Shared bounded probe owner/process group and cleanup; no receive/spawn exclusion |
| Background provider execution | providers.rs:1441 and :1512 | Provider dispatch/cancellation gates around std Command; not a process-wide descriptor fence |
| Model discovery/app-server | providers.rs:3329 and :3427 | Direct std child ownership and output handling outside the probe dispatcher |
| Custom Taskmaster inference | taskmaster/runtime/inference.rs:396; providers/custom_inference.rs prepares the command | Current catalog/settings/dispatch identity guard, not exclusion shared by other launch families |
| Native Codex sidecar | runtime/codex_native/launch.rs:132 (OwnedProcess::spawn) | Process-group ownership; executable/model checks do not cover ancillary receipt |
| Session PTY | runtime/session_runtime.rs:2770 | portable-pty 0.8.1 spawn_command, with its own child pre_exec cleanup |
| Git context helper | git.rs:59–77 | Direct current-executable std child, with timeout/terminate/reap owner |
| Qualification CLI Git | taskmaster/improvement/qualification.rs:548 and :1460 | Direct std output calls; not application capture authority |

The inspected production families use std Command, or portable-pty's std Command
conversion. Explicit tokio::process::Command uses found in this scan were test
fixtures, not a separate production receiver/spawn gate. No SCM_RIGHTS receiver,
common spawn lock, atfork registration or process-wide close-default policy was
found in the sidecar source. This does not assert every transitive native library
can never spawn; any global-fence proposal would need that additional coverage.

portable-pty 0.8.1's `src/unix.rs:118–144,194–250` converts the builder to std
Command and calls close_random_fds in child pre_exec. That cleanup enumerates
/dev/fd and ignores failures. It does not qualify direct inference/helper paths,
and is not reused as an infallible custody guarantee. Existing source O_CLOEXEC
and F_DUPFD_CLOEXEC uses likewise do not turn an unrelated received descriptor
into an atomic receipt. Environment filtering is independent of descriptor tables.

## Supported API and bootstrap boundary

The inspected SDK `sys/socket.h` contains no MSG_CMSG_CLOEXEC macro. SDK
`sys/spawn.h:62` exposes POSIX_SPAWN_CLOEXEC_DEFAULT, but a per-spawn attribute is
not a process-wide policy. SDK `sys/unistd.h:216–219` declares dup3/pipe2 with a
macOS 27.0 availability annotation; these headers do not qualify either API on
the tested macOS 26.6.2 host. No assertion of API impossibility is made.

Primary background sources:

- [Apple XNU unp_externalize](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/kern/uipc_usrreq.c#L2263): the inspected path allocates descriptors in the receiving process. It supports the process-table distinction; it is not evidence matching this host's exact kernel build.
- [Apple spawn attributes](https://github.com/apple-oss-distributions/xnu/blob/f6217f891ac0bb64f3d375211650a4c1ff8ca1ea/bsd/sys/spawn.h#L67) define the close-default flag; using it comprehensively would require changing or proving all relevant launch paths.
- [Rust 1.96 Unix pipes](https://github.com/rust-lang/rust/blob/1.96.0/library/std/src/sys/pipe/unix.rs#L30) create then mark close-on-exec on the Apple branch.
- [Rust 1.96 socket pairs](https://github.com/rust-lang/rust/blob/1.96.0/library/std/src/sys/net/connection/socket/unix.rs#L120) similarly use separate creation/flag calls there.

Consequently, a fresh socket pair or `Stdio::piped` cannot simply be declared an
atomic custody bootstrap on this host. Moving resource descriptors to another
process without qualifying the controller channel can merely move the gap.
The proposed pool seals every channel before concurrent controlled launches,
and never creates a replacement channel during normal operation. That source
bootstrap proof is still pending; the primitive below pre-seals its channels.

## Executed finite native witness

Host: macOS 26.6.2 arm64, Rust 1.96.0. A temporary file with non-inheritable sender
handle is sent through SCM_RIGHTS either into the controller process (positive
control) or into an isolated Python receiver which never spawns after receipt.
Channels are sealed before any unrelated child is launched. The controller
starts a standalone Rust parent with `close_fds=False`; Rust starts its own
ordinary Command child with an empty environment. `/dev/fd` opens and metadata
compare the temporary file's device/inode, not merely a reused descriptor number.

```text
local receive-first ['PARENT 1', 'CHILD 1']
local spawn-first ['PARENT 0', 'CHILD 0']
isolated receive-first ['PARENT 0', 'CHILD 0']
isolated spawn-first ['PARENT 0', 'CHILD 0']
```

The spawn-first parent acknowledges READY and waits while receipt completes;
it then starts the Rust child. The receiver retains the actual object through
the child observation and acknowledges matching device/inode with an inheritable
received descriptor. Every observed result matched the independent literal above.
The positive control checks that Python launch policy did not hide the inheritance.

This is an ordered primitive witness, **not a concurrent Rust receiver stress
test or actual chosen application boundary**. Only supplied fixture descriptors
below 128 and a finite child scan of 3..255 are covered. It does not qualify
four-worker bootstrap, live root provenance, no-follow traversal, actor protocol,
resource read deadlines, pool reset/drain, source/native eligibility or Start.
No installed/user hooks, coding tool, provider or report capability is read.
In the four successful runs, both immediate children and the Rust Command child
were waited/reaped, descriptors/sockets closed, and temporary files removed.
The reproduction attempts immediate-child cleanup on failure; killing the Rust
parent does not itself terminate its grandchild, and a cleanup timeout can prevent
later cleanup statements from running. No descendant-safe failure-cleanup proof
is claimed by this fixture; that remains an actual worker qualification gate. No product code or private route is
added. Finite process separation supports the proposed choice; actual #1981
non-inheritance at its chosen source boundary remains unchecked.

## Reproduction

Save this Rust fixture as `/private/tmp/orkworks-custody-spawn-witness.rs`:

```rust
use std::{fs::File, io::{self, Write}, os::unix::fs::MetadataExt, process::Command};
fn count(dev: u64, ino: u64) -> usize {
    (3..256).filter(|fd| File::open(format!("/dev/fd/{fd}"))
        .and_then(|file| file.metadata()).is_ok_and(|m| m.dev() == dev && m.ino() == ino)).count()
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dev = args[2].parse().unwrap();
    let ino = args[3].parse().unwrap();
    if args[1] == "parent-wait" {
        println!("READY"); io::stdout().flush().unwrap();
        let mut line=String::new(); io::stdin().read_line(&mut line).unwrap();
        assert_eq!(line, "GO\n");
    }
    if args[1] == "child" { println!("CHILD {}", count(dev,ino)); return; }
    println!("PARENT {}",count(dev,ino));
    let out=Command::new(std::env::current_exe().unwrap()).env_clear()
        .args(["child",&args[2],&args[3]]).output().unwrap();
    assert!(out.status.success());
    print!("{}",String::from_utf8(out.stdout).unwrap());
}

```

Compile with the pinned compiler and run the Python controller below through
`rtk proxy python3` after saving it to a temporary script:

```sh
rtk proxy rustc --edition 2021 /private/tmp/orkworks-custody-spawn-witness.rs -o /private/tmp/orkworks-custody-spawn-witness
```

```python
import os,sys,socket,array,tempfile,subprocess,json,fcntl,platform
BINARY='/private/tmp/orkworks-custody-spawn-witness'
WORKER=r"""
import os,sys,socket,array,json
s=socket.socket(fileno=int(sys.argv[1])); s.settimeout(5)
fds=array.array('i')
try:
    _,anc,flags,_=s.recvmsg(1,socket.CMSG_SPACE(array.array('i').itemsize))
    assert not flags & socket.MSG_CTRUNC
    for level,kind,data in anc:
        if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:
            fds.frombytes(data[:len(data)-len(data)%fds.itemsize])
    assert len(fds)==1 and fds[0]<128
    m=os.fstat(fds[0])
    s.send(json.dumps({'dev':m.st_dev,'ino':m.st_ino,'inheritable':os.get_inheritable(fds[0])}).encode())
    assert s.recv(1)==b'Q'
finally:
    for fd in fds:os.close(fd)
    s.close()
"""
def probe(f,wait):
    m=os.fstat(f.fileno())
    return subprocess.Popen([BINARY,'parent-wait' if wait else 'parent',str(m.st_dev),str(m.st_ino)],
        close_fds=False,env={},stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
def finish(p,waiting):
    if waiting:
        out,err=p.communicate('GO\n',timeout=5)
    else:out,err=p.communicate(timeout=5)
    assert p.returncode==0,(out,err)
    return out.strip().splitlines()
def run(remote,receive_first):
    p=None;worker=None;fds=array.array('i');a=b=None
    with tempfile.TemporaryFile() as f:
        assert f.fileno()<128 and not os.get_inheritable(f.fileno())
        a,b=socket.socketpair(socket.AF_UNIX,socket.SOCK_DGRAM)
        a.settimeout(5);b.settimeout(5)
        # All channels are sealed before any unrelated child is launched.
        assert not os.get_inheritable(a.fileno()) and not os.get_inheritable(b.fileno())
        try:
            if remote:
                worker=subprocess.Popen([sys.executable,'-c',WORKER,str(b.fileno())],pass_fds=(b.fileno(),),
                    env={},stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
                b.close();b=None
            if not receive_first:
                p=probe(f,True);assert p.stdout.readline().strip()=='READY'
            a.sendmsg([b'F'],[(socket.SOL_SOCKET,socket.SCM_RIGHTS,array.array('i',[f.fileno()]))])
            if remote:
                ack=json.loads(a.recv(256));m=os.fstat(f.fileno())
                assert (ack['dev'],ack['ino'])==(m.st_dev,m.st_ino) and ack['inheritable']
            else:
                _,anc,flags,_=b.recvmsg(1,socket.CMSG_SPACE(array.array('i').itemsize));assert not flags & socket.MSG_CTRUNC
                for level,kind,data in anc:
                    if level==socket.SOL_SOCKET and kind==socket.SCM_RIGHTS:fds.frombytes(data[:len(data)-len(data)%fds.itemsize])
                assert len(fds)==1 and fds[0]<128 and os.get_inheritable(fds[0])
            if p is None:p=probe(f,False)
            actual=finish(p,not receive_first)
            expected=['PARENT 1','CHILD 1'] if not remote and receive_first else ['PARENT 0','CHILD 0']
            assert actual==expected,(remote,receive_first,actual)
            if remote:a.send(b'Q');worker.wait(timeout=5);assert worker.returncode==0
            return actual
        finally:
            if p is not None and p.poll() is None:p.kill();p.communicate(timeout=5)
            if worker is not None and worker.poll() is None:worker.kill();worker.wait(timeout=5)
            for fd in fds:os.close(fd)
            if a is not None:a.close()
            if b is not None:b.close()
print(platform.platform())
for remote in [False,True]:
    for first in [True,False]:
        print('isolated' if remote else 'local','receive-first' if first else 'spawn-first',run(remote,first))

```

## Fresh review — 2026-11-23

A fresh contract review verified the launch-family/main source claims, native
reproduction scope and parent-contract boundaries. Its required correction was
applied above: successful-run cleanup is distinguished from attempted immediate-
child cleanup and unqualified descendant failure cleanup. The four native outcomes
are unchanged. No stronger process ownership/cleanup claim is inferred.
