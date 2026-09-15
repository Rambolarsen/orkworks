# Process Ownership Proof Design

Status: revised draft for issue #545; Task 7 evidence recorded, production integration blocked

Tracking: [issue #545](https://github.com/Rambolarsen/orkworks/issues/545)

## Goal

Prove an ownership boundary that can clean every OrkWorks-owned sidecar,
session PTY, and inference descendant after the sidecar or Electron exits,
without touching a foreign sidecar or sentinel process. The proof must be
strong enough to support the unavailable-runtime cleanup and relaunch rules in
the proposed concurrent-workspace specification.

This work does not implement concurrent workspaces, runtime recovery, close,
quit, or replacement behavior. It produces a native fixture, records the
selected mechanisms and limitations, and updates the implementation plan only
after the evidence is complete.

## Contract under test

The owner boundary must satisfy all of these properties:

1. Electron establishes one owner/supervisor generation before any sidecar,
   PTY, or inference child is allowed to execute. The supervisor is outside
   Electron's kill domain and retains the live ownership state after a sidecar
   exits.
2. Every owned process root is admitted through an authenticated, one-use
   launch ticket issued by the supervisor. The ticket is bound to the
   generation, role, executable identity, and request nonce. A registration or
   containment failure fails closed before the target executes.
3. Owner-channel loss freezes new admission, drains in-flight launch attempts,
   and causes bounded cleanup of only the registered generation. Cleanup uses a
   five-second graceful phase followed by a five-second owned-termination
   phase; those monotonic deadlines are supervisor-owned and cannot be
   extended or shortened by the caller. It returns an explicit acknowledgement
   only after all owned processes have exited, or an unresolved-survivor result
   after the deadline.
4. A foreign sidecar holding the same workspace lease, its metadata bytes and
   revision, and an unrelated sentinel remain alive and unchanged throughout
   owned cleanup and cleanup failure.
5. A new generation can locate the previous supervisor only through an
   authenticated per-generation rendezvous record and live local IPC. It may
   adopt only an authenticated complete-exit acknowledgement. Missing,
   unreachable, malformed, stale, or ambiguous rendezvous is unresolved, never
   an empty generation.
6. The evidence covers a PTY-shaped child that creates a separate terminal
   session, an inference child, and descendants that outlive the sidecar. It
   also covers forked, new-process-group, daemonized, reparented, and
   deliberately non-reporting descendants.
7. The mechanism does not infer ownership from a persisted PID, executable
   name, working directory, or released metadata lease. Native process
   identity and liveness must come from the live supervisor/OS boundary.
8. The supervisor's control endpoint is non-inheritable by owned targets, and
   losing the Electron-side channel cannot be masked by a descendant retaining
   an inherited descriptor or handle.

## Selected direction

Use a small native owner/supervisor process launched by Electron. Electron
retains the authenticated control channel and the supervisor owns the
generation's containment state. The supervisor, rather than the sidecar,
launches or authorizes every process root that can create a PTY or inference
descendant. The sidecar receives generation-bound launch capabilities and cannot
spawn an unregistered child.

The first implementation artifact is a disposable fixture with the same
control and failure transitions. The fixture is necessary but not sufficient:
the eventual production implementation must route the actual PTY and inference
launch seams through the proven boundary before #545 can be closed. Production
multi-workspace cleanup and recovery remain later tasks.

Task 7 records a bounded ruling rather than a production selection. The Windows
Job Object fixture is demonstrated by a hosted native run with all eight
`windows_job` tests passing, including suspended registration, breakaway
rejection, handle inheritance checks, and actual `TerminateProcess` parent
termination. No macOS or portable-Linux native owner mechanism passed the
required matrix: macOS launch-dependent rows are rejected or unsupported by the
host adapter, and no native Linux runtime was available. The Windows result is
therefore fixture evidence only and cannot close the cross-platform or
production gates.

### Windows hypothesis

The supervisor creates a private Windows Job Object with
`JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and without breakaway permissions. It
starts the sidecar in the job while suspended, assigns it to the job, and only
then resumes it. Children created by ordinary Win32 process creation inherit
the job; the supervisor retains the job handle after the sidecar exits. On
owner-channel loss it terminates the job, waits for the job to become signaled,
and reports the complete-exit acknowledgement.

The fixture must include a sidecar child that creates both a PTY-like process
and a nested inference process, and must attempt a breakaway path. A breakaway
that succeeds without an explicit supported boundary is a failed proof, not a
warning. The supervisor's control handles and endpoints must also be
non-inheritable by those children.

### macOS and portable Unix hypothesis

Do not treat one supervisor process group as sufficient. The existing
`portable-pty` Unix implementation calls `setsid()` for the PTY child, so the
PTY session is outside the supervisor's ordinary process group. The fixture
must therefore validate an explicit supervisor-owned launch/registration
boundary for each process root, with launch identity retained by the live
supervisor and cleanup performed through that boundary.

The experiment will compare only as candidates; a candidate is rejected unless
it passes the complete descendant and forced-termination matrix:

- a supervisor-owned process-group/session boundary, including a PTY child that
  calls `setsid()` and a child that creates a new process group;
- a supervisor that launches and tracks each process root with native process
  identity and recursively reaps only its registered descendants. This option
  must prove birth identity, ancestry, liveness, and PID-reuse resistance for
  every lookup; PID/name/path matching alone cannot pass; and
- the platform service mechanism available on macOS (`launchd`) as a control,
  including its process-group cleanup behavior.

The result must identify which mechanism, if any, survives the PTY/session,
daemonization, and reparenting cases. Linux may additionally use pidfds or
cgroup-v2 evidence where available, but Linux-specific strength cannot be
substituted for the macOS result. `PR_SET_PDEATHSIG` alone is not sufficient
because it is a Linux parent-death signal for one process, not proof of an
entire descendant boundary. If the macOS fixture cannot prove ownership after
`setsid()` or intentional reparenting, the prerequisite remains unresolved and
the limitation is brought back to spec review.

## Fixture protocol

The fixture contains an owner, a sidecar-shaped child, owned PTY/inference
fixtures, and a foreign sentinel. It communicates over private pipes or local
sockets with newline-delimited JSON messages bounded to 64 KiB each:

- `prepare(generation)` creates the ownership domain and returns a
  supervisor-generated rendezvous nonce plus an authenticated rendezvous
  record; it launches no target;
- `issue_launch_ticket(role, executable_identity, request_nonce)` returns a
  supervisor-generated, one-use ticket bound to the prepared generation;
- `spawn(ticket, paused)` creates the process root paused or behind an exec
  gate, records its native launch identity independently, and releases it only
  after containment and registration succeed;
- `ready(role, ticket)` is a diagnostic target report only. It cannot prove
  ordering or ownership;
- `observe()` returns supervisor/OS observations of registered roots,
  descendants, birth identities, containment membership, and exit state;
- `owner_lost` is driven by actual control-channel EOF after the parent fixture
  is force-terminated, as well as by an orderly close;
- `cleanup()` freezes admission, drains in-flight launches, performs the
  supervisor-owned five-second graceful and five-second owned-termination
  phases, and returns either `acknowledged` with an independently observed
  complete-exit set or `unresolved` with independently observed survivors;
- `foreign_status` reads the foreign sidecar heartbeat, lease ownership, and
  metadata bytes/revision without asking the foreign process to self-attest;
- `rendezvous_status(generation, nonce)` authenticates the prior supervisor's
  live state or complete-exit receipt; and
- `adopt(generation, nonce)` is rejected unless that receipt is authenticated
  and complete. Any unavailable, malformed, stale, or unknown result keeps
  adoption unresolved.

The supervisor, not the fixture target, creates each process identity. PIDs are
diagnostic fields only; they are never the authority for selecting a cleanup
target. The fixture deliberately includes targets that omit or falsify their
`ready` and `children` messages, so passing depends on supervisor/OS evidence.

## Evidence matrix

The fixture must pass these scenarios before mechanism selection is recorded:

| Scenario | Required result |
| --- | --- |
| Normal owner loss | Owned sidecar, PTY fixture, inference fixture, and nested descendants exit; foreign sentinel survives. |
| Sidecar crashes first | Supervisor still cleans the owned PTY and inference descendants. |
| Registration or containment failure | Target is held paused or behind the exec gate, never executes, and no owned descendant appears. |
| Admission races with owner loss | Admission freezes, in-flight launches drain, late tickets are rejected, and cleanup covers the resulting closed set. |
| PTY creates a separate session/group | Cleanup still reaches the PTY-shaped tree; otherwise the candidate mechanism is rejected. |
| Forked, new-group, daemonized, reparented, or silent descendant | Independent observation either proves ownership and cleanup or rejects the candidate; target self-reports cannot make it pass. |
| Owned child ignores graceful cleanup | The five-second graceful phase escalates to the five-second owned-termination phase; a survivor remains unresolved. |
| Supervisor dies while descendants remain | No complete-exit receipt is produced; relaunch remains blocked/unresolved and never guesses from PIDs. |
| Foreign owner holds the workspace lease | Foreign owner heartbeat, lease ownership, and metadata bytes/revision remain unchanged; only the attempted generation is cleaned. |
| Forged, replayed, stale, or cross-generation ticket | Supervisor rejects it before execution and records no owned root. |
| Late obsolete-generation event | It cannot mutate the replacement or initiate cleanup of a different generation. |
| Immediate relaunch | Authenticated rendezvous returns complete-exit before adoption; unavailable or ambiguous rendezvous blocks adoption. |
| Forced Electron termination | Force-kill the Electron-like parent on Windows and macOS; owner-channel loss cleans both focused/background generations and foreign sentinel survives. |
| Two open generations A and B | Both sidecar-shaped roots, their PTY/inference descendants, and background inference are independently owned, observed, and cleaned without cross-generation effects. |
| Inference admission while an older root survives | A new inference root is rejected until the prior generation has an authenticated complete-exit receipt; a released lease or missing PID is not sufficient. |
| Production launch-seam audit | The actual `portable_pty` session launch and provider/inference launch paths are shown to require the proven ticket and registration boundary; any bypass leaves #545 open. |

Run the fixture on Windows and macOS, plus portable Linux coverage. Force the
Electron-like parent with `TerminateProcess` on Windows and `SIGKILL` on macOS;
do not substitute an orderly channel close for this scenario. Record OS,
architecture, build mode, mechanism, test command, supervisor/OS observations,
launch-ticket outcomes, raw process identities, cleanup latency, and any
survivor. Include the actual forced-parent-termination command and prove the
supervisor is outside that kill domain. Do not record user credentials or
private prompts.

## Boundaries and limitations

- The owner boundary is process containment, not a filesystem sandbox. It does
  not make Git or workspace path operations atomic.
- A metadata lease is still required for single-writer workspace ownership,
  but it is not evidence that processes exited.
- Persisted PIDs are never used for adoption or cleanup.
- The fixture must distinguish an unavailable owner from a confirmed empty
  owner. Failure to observe liveness is unresolved, not zero children.
- A launch ticket, rendezvous nonce, and complete-exit receipt are
  generation-bound and one-use. The persisted rendezvous record may locate a
  live supervisor, but it cannot by itself prove process ownership or exit.
- Cleanup admission and acknowledgement are serialized with launch and owner
  loss. A census taken before the admission barrier is not complete evidence.
- The fixture's sidecar-shaped process is not evidence that the production
  `portable_pty` and inference launch paths use the boundary. #545 remains
  open until those production seams are audited or integrated with the proven
  owner protocol.
- A fixture supervisor must not accept caller-selected rendezvous nonces,
  executable identities, or reusable launch tickets; those values are issued
  and checked by the supervisor for the prepared generation.
- No unavailable-runtime cleanup or automatic recovery may report success until
  the native evidence and the corresponding ADR 0056 update are complete.

## Task 7 evidence ruling

The canonical matrix is the fixture-local
[`evidence/matrix.json`](../../../crates/process-ownership-fixture/evidence/matrix.json),
with one row for each of the 16 required scenarios on Windows, macOS, and
portable Linux. The companion evidence record copies the exact commands and
records the result vocabulary (`pass`, `accepted`, `unsupported`, or
`unresolved`). A `pass` is reserved for rows with independent native
observation and forced-parent proof; contract-only rows use `accepted` and do
not claim OS containment.

The matrix contains five Windows native passes from the hosted 8/8 run, ten
platform-neutral or fixture-contract acceptances, one unresolved macOS
production-seam audit row, and 32 unsupported rows. The counts are deliberately
not inflated by the macOS six-test candidate rejection, the 47-test fixture
run, or the 14 launch-dependent Task 5 skips. Task 4's rejection of
ProcessGroup, RegisteredRoot, and launchd candidates remains parked. Task 5's
launch-dependent rows remain skipped; its foreign-owner helper is a heartbeat
thread rather than an independent scheduler process, and the compatibility
adapter's inability to forcibly cancel an arbitrary blocking external adapter
call remains a fixture qualification.

Task 6 found direct production roots for the Electron sidecar, PTY, provider
and inference runners, discovery, and harness probes outside the proven owner
boundary. No `processSupervisor.ts` or `process_ownership.rs` was added. Issue
#545 remains open until a native Unix mechanism is demonstrated and every
production root is routed through an accepted owner boundary.

## References

- [Concurrent workspaces](../../../specs/multi-workspace.md)
- [ADR 0056](../../adr/0056-one-sidecar-per-open-workspace.md)
- [Multi-workspace validation contract](../../validation/multi-workspace.md)
- [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
- [`launchd.plist(5)`](https://keith.github.io/xcode-man-pages/launchd.plist.5.html)
- [`PR_SET_PDEATHSIG(2)`](https://man7.org/linux/man-pages/man2/pr_set_pdeathsig.2const.html)
- [Linux cgroup v2](https://www.kernel.org/doc/html/latest/admin-guide/cgroup-v2.html)
