# Taskmaster macOS Child-Launch Boundary Feasibility Plan

> **For agentic workers:** This is a research-only record. It does not authorize production child launches or changes to the runner implementation.

**Goal:** Determine whether a narrow macOS process boundary can contain a Taskmaster child launch and identify a supportable next design if the least intrusive prototype is unsuitable.

**Architecture:** Probe disposable native helpers under temporary Seatbelt profiles. Measure filesystem access, socket behavior, descendant policy behavior, and process ownership independently. Treat `sandbox-exec` as feasibility-only because Apple marks it deprecated; a passing fixture would not establish a production-ready interface.

**Tech Stack:** macOS ARM64, C fixture, Seatbelt profiles, native process and filesystem observations.

**Research reference:** [Taskmaster native child-launch boundary design](../specs/2026-09-25-taskmaster-native-child-launch-boundary-design.md), retained as historical context. The current proposed direction is [Taskmaster orchestrated child sessions](../specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md), which uses ordinary OrkWorks sessions and does not require native child confinement.

## Execution gate

The user approved a disposable macOS feasibility prototype. All fixtures ran under a unique `/private/tmp` directory, used no credentials or real coding harness, and were removed after checking that no fixture process remained. No OrkWorks runtime code was changed.

A result may guide a later spec/design review; it cannot set `native_boundary_proven` or `runner_eligible`.

## Spike result — 2026-09-26

**Outcome: no-go for using `sandbox-exec` as the child-launch boundary.**

The strict `deny default` profile aborted the helper with signal 6 before its first flushed output. Adding broad `file-read-metadata` access did not fix it. Adding broad `file-read*` access did make the helper run, but then the parent successfully read the outside-worktree sentinel. That control profile is not an acceptable filesystem boundary.

With the broad-read profile, the parent could create an IPv4 socket; its loopback `connect()` failed with `EPERM`. The prior fixture conflated socket and connect errors, so it did not show that socket creation was denied.

An exec'ed descendant that called `setsid()` behaved differently from its parent under that profile: its outside-sentinel read failed with `EPERM`, while the parent could read the sentinel. Both parent and descendant created sockets; both loopback `connect()` calls failed with `EPERM`. The cause of the file-read difference is unresolved, so it is not counted as proof of a reliable policy contract.

The sandbox launcher returned while the exec'ed descendant was still running as a reparented session leader (PPID 1). Sending `SIGTERM` to the exact recorded fixture PID stopped it; a follow-up process check confirmed it was gone. `sandbox-exec` itself did not own or clean up that detached descendant.

Apple's developer forum states that `sandbox-exec` is deprecated and its sandbox APIs are no longer supported ([Apple Developer Forums](https://developer.apple.com/forums/thread/661939)). Together with the failed strict profile, parent boundary failures, inconsistent parent/descendant observations, and missing process-tree ownership, this rules out continuing this prototype toward production. This is a no-go for the tested prototype, not proof that every possible Seatbelt profile fails.

## Reproducibility record

The following details were captured before the disposable fixture directory was removed:

- **Host:** macOS 26.6.2 (build 25G83), ARM64; Darwin 25.6.0. These were captured with `sw_vers` and `uname` during review.
- **Profile variants:** (1) `deny default`, allow execution of the exact fixture, process fork, sysctl reads, reads beneath `/usr/bin`, `/bin`, `/usr/lib`, `/System/Library`, `/Library/Apple/System/Library`, the fixture, worktree, and scratch; allow writes only beneath worktree and scratch; allow metadata for `/`, `/private`, `/private/tmp`, and the temporary root. (2) Variant 1 plus broad `file-read-metadata`. (3) Variant 1 plus broad `file-read*`. A separate control used `allow default` with one explicit `file-read*` deny for the outside sentinel.
- **Helper probes:** write a worktree file; read an outside sentinel; create an IPv4 TCP socket and connect to loopback port 9; fork, call `setsid()`, exec the same helper in descendant mode, repeat the outside-read and socket probes, write its PID, and sleep.
- **Commands:** compile with `clang <SPIKE_ROOT>/fixture.c -o <SPIKE_ROOT>/fixture`; run profiles with `sandbox-exec -f <PROFILE> <SPIKE_ROOT>/fixture <WORKTREE_FILE> <OUTSIDE_SENTINEL> <PID_FILE>`; inspect the recorded PID with `ps -p <PID> -o pid,ppid,pgid,sess,state,command`; terminate only that exact fixture PID with `kill -TERM <PID>` and confirm a later `ps -p <PID>` returns no process.
- **Strict profile observations:** variant 1 and variant 2 both exited 134 (signal 6) before any fixture output. Variant 3 ran.
- **Variant 3 parent output:** `allowed_write=1 errno=0`; `outside_read=1 errno=0`; `socket_created=1 socket_errno=0 connected=0 connect_errno=1`; then an exec descendant PID. Thus the parent could read the outside sentinel and create a socket, while its loopback connect failed with `EPERM`.
- **Variant 3 descendant output:** `outside_read=1 errno=1`; `socket_created=1 socket_errno=1 connected=0 connect_errno=1`. The outside read failed; socket creation succeeded. `socket_errno=1` is stale from the preceding failed `open()`, because a successful `socket()` need not clear `errno`. The loopback connect failed with `EPERM`, matching the parent. The file-read difference's cause was not established.
- **Lifecycle observation:** `sandbox-exec` returned with the exec'ed `setsid()` descendant still alive as PPID 1 and its own process-group leader. `SIGTERM` to the exact recorded PID succeeded; the subsequent process query returned no row.

The fixture source, exact temporary paths, and full command transcript were deleted during cleanup, so this summary is the retained evidence rather than a byte-for-byte replay artifact. Do not treat errno 1 as proof of which Seatbelt rule caused the descendant's denial.

## Follow-up spike — XPC access and launchd ownership

The user approved a second disposable macOS fixture. A manually compiled,
ad-hoc-signed App Sandbox app and embedded XPC service used a
security-scoped folder bookmark for `/private/tmp/orkworks-xpc-spike/assigned`.
The helper wrote within that folder; a direct `/bin/sh` child inherited the
same access. Both helper and child were denied a read of the outside
`orkworks/README.md` with `NSCocoaErrorDomain Code 257` / `Operation not
permitted`. This proves only the tested helper/direct-child fixture, not an
actual coding-tool CLI.

The XPC helper then started a descendant that called `setsid()` and exited.
The detached descendant remained alive after the XPC service exited. A
separate temporary launchd job started a small C helper that forked a child,
called `setsid()` in that child, and then exited normally. The child had a
different process-group ID and remained alive after the launch job exited.
The exact recorded fixture PIDs were terminated during cleanup, and follow-up
process checks confirmed no probe jobs or descendants remained. These results
show that the tested XPC and launchd mechanisms do not supply the strict
complete-process-tree owner required by the former boundary proposal.

All source, bundles, plists, and logs were created under `/private/tmp` and
removed. No repository implementation code was changed. The fixture did not
test an installed OpenCode CLI, credentials, resource ceilings, or any
packaged OrkWorks build.

## Decision

The tested `sandbox-exec` profile remains a no-go for production confinement.
The XPC fixture demonstrated scoped filesystem access for a basic helper and
direct child, while the launchd fixture demonstrated that a detached
`setsid()` descendant survives the job's normal exit. Together, these results
do not satisfy the previous strict native-boundary contract.

The selected product direction is ordinary OrkWorks child sessions launched
by a user-approved orchestrator plan. Native filesystem/process confinement,
virtual machines, and a separate durable process owner are outside that
scope. See the proposed replacement design. Reopen native-boundary research
only if a future product requirement explicitly calls for OS-enforced
restriction beyond normal user-level session permissions.
