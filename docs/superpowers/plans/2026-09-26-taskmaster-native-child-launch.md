# Taskmaster macOS Child-Launch Boundary Feasibility Plan

> **For agentic workers:** This is a research-only record. It does not authorize production child launches or changes to the runner implementation.

**Goal:** Determine whether a narrow macOS process boundary can contain a Taskmaster child launch and identify a supportable next design if the least intrusive prototype is unsuitable.

**Architecture:** Probe disposable native helpers under temporary Seatbelt profiles. Measure filesystem access, socket behavior, descendant policy behavior, and process ownership independently. Treat `sandbox-exec` as feasibility-only because Apple marks it deprecated; a passing fixture would not establish a production-ready interface.

**Tech Stack:** macOS ARM64, C fixture, Seatbelt profiles, native process and filesystem observations.

**Design reference:** [Taskmaster native child-launch boundary design](../specs/2026-09-25-taskmaster-native-child-launch-boundary-design.md). That accepted design specifies Linux. This investigation does not qualify macOS or alter the accepted design.

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

## Decision gate

Do not invest further in repairing this `sandbox-exec` profile. A production proposal needs both a supported enforcement mechanism and an owner that can prove complete descendant cleanup.

The next design review should compare:

1. **App Sandbox with a helper or XPC boundary and scoped worktree access.** Apple's model passes sandbox capabilities to directly spawned helpers; bookmarks can transfer access to a selected resource to another process ([helper inheritance](https://developer.apple.com/documentation/security/discovering-and-diagnosing-app-sandbox-violations?changes=_7), [file bookmarks](https://developer.apple.com/documentation/security/accessing-files-from-the-macos-app-sandbox?changes=_4)). The investigation must establish app-wide sandbox/signing impact; whether the actual helper and external coding CLI can run under the model; how exactly one selected worktree's access reaches each process; whether the CLI can resolve/use that access without OrkWorks changes; and whether previous workspace grants or inherited entitlements expose broader access. Apple's documented privilege separation uses a separate XPC service/helper when the child needs different capabilities from the app.
2. **A per-child virtual machine.** This gives a clearer OS boundary but adds packaging, startup, resource, and worktree-sharing costs.
3. **A Windows candidate.** Reconsider only as an explicit architecture choice; do not silently retarget the accepted Linux spec or assume existing Job Object use supplies filesystem/network isolation.

The least intrusive next investigation is to verify whether App Sandbox can confine OrkWorks' actual helper/CLI process chain while granting a single selected worktree. If it cannot, compare the VM and Windows alternatives before writing a replacement child-launch spec. No production design is selected by this plan.
