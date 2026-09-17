# Independent workspace instance validation plan

Status: partially verified implementation contract; native ownership gates remain open
Date: 2026-09-17
Spec: [Independent workspace instances](../../specs/multi-workspace.md)
Decision: [ADR 0060](../adr/0060-independent-workspace-instances.md)
Tracking: [#541](https://github.com/Rambolarsen/orkworks/issues/541)

## Current implementation audit (2026-09-18)

Tasks 1–4 of the independent-instance implementation are complete on the
`issue-545-production-seam-audit` branch. The desktop lifecycle, installation
history, generation gates, canonical identity, lease-first adoption, opaque
409 conflict mapping, and fail-closed picker/unresolved outcomes have focused
test coverage. The process-ownership fixture also passes its available host
tests, but fixture success is not production process-ownership proof.

Verified locally:

- desktop workspace/lifecycle wiring: 54 tests passed;
- Rust canonical identity: 5 tests passed;
- Rust workspace/session-handler adoption: 110 tests passed;
- process-ownership fixture: 53 tests passed;
- both Electron TypeScript configurations, Rust formatting, and diff checks.

Not verified and intentionally still gated:

- two real Electron instances with independent profiles and sidecars;
- native crash/relaunch cleanup for every production process family;
- production routing through a proven cross-platform owner boundary;
- native Windows/macOS/Linux acceptance rows unavailable in this checkout.

Until issue [#545](https://github.com/Rambolarsen/orkworks/issues/545) has
native evidence and a production seam audit, the acceptance rows below that
require crash-surviving ownership remain unverified. The implementation must
remain fail-closed for replacement and relaunch after unexpected ownership
loss.

## Test setup

Use disposable workspaces A, B, and C with distinct names, instructions,
settings, metadata, and recognizable fixture output. Use deterministic native
coding-tool fixtures that emit numbered output, wait for controlled input, and
expose owned child handles. Use a fixture inference executable that records
start, cancellation, and exit without provider login or paid calls.

Run each OrkWorks instance as a real process with an isolated Electron profile
where required. For history-concurrency scenarios, point two instances at the
same disposable application-data root while keeping workspace metadata roots
distinct. Verify every root before launch; never run mutation or crash tests
against the user's home store. Keep process handles and heartbeat channels in
the harness, not in shared history.

Foreign-owner scenarios use a harness-owned sidecar holding a fixture workspace
lease and an unrelated sentinel child. Neither belongs to the instance under
test, and only the harness cleans them up.

## Acceptance matrix

### Installation history

| Scenario | Required observation |
| --- | --- |
| First launch with no history | Picker is shown; no sidecar, backend port, workspace lease, or repository/home fallback starts |
| Open A, exit, and relaunch | A is persisted as the last-path hint and appears once at the front of recent paths; no coding session resumes automatically |
| Select B after A | History changes only after B's lease, readiness, and restoration complete; B moves to the front and A remains remembered |
| Fail B validation, lease, spawn, readiness, or restoration | No successful-open history entry is written for B |
| History write fails after B is ready | B remains ready and shows a storage diagnostic; runtime is not rolled back |
| Two instances update history concurrently | Locked read-modify-write preserves successful paths with increasing revisions; neither writer publishes peer runtime state |
| Open an alias of A | Canonical history has one deduplicated entry; display spelling does not create a second identity |
| Add more than 20 paths or exceed 64 KiB | Oldest entries are evicted until both bounds hold and current path stays first |
| Forget a path | Only its shortcut is removed; project files, metadata, sessions, and settings remain |
| Read malformed history | Source bytes remain untouched; picker loads empty in memory and displays a diagnostic until explicit rebuild |
| Ambiguous atomic replacement | Read-back accepts only the expected revision and record; old/unreadable state leaves runtime unchanged |
| Inspect persisted history | Only version, revision, last path, and recent paths exist; no PID, port, lease, health, owner, peer, or open-state field exists |

### One-instance lifecycle

| Scenario | Required observation |
| --- | --- |
| Validate inaccessible B while A is ready | A remains usable; no cleanup gate or B process begins |
| Cancel A-to-B switch before cleanup | A's terminal, sessions, authority, and controls remain unchanged |
| Switch from A to B | A gates new work and fully exits before B starts; B is ready only after lease-backed restoration |
| Race start/resume or analysis with A cleanup | Request is admitted and included in cleanup, or rejected by the atomic gate |
| A cleanup or ownership proof fails | State becomes unresolved; B never starts; no successful-close claim is emitted |
| A closes, then B lease conflicts | Attempted B runtime is cleaned; picker shows opaque retryable conflict; A is not silently reopened |
| B spawn/readiness/restoration fails | Only attempted B runtime is cleaned; picker remains; A is not silently reopened |
| Failed B cleanup leaves an owned descendant | State becomes unresolved and another destination cannot start |
| Late event from old A or failed B | It cannot publish readiness, mutate history, accept input, or replace state |
| Repeated switch, retry, close, or quit | Requests serialize and identical requests coalesce; no operation clears another's gate |
| Close with no non-terminal work | Atomic gate confirms empty set, cleanup completes, and picker is shown without a running-work confirmation |
| Close during session finalization | Live work is included and bounded finalization completes, or cleanup remains unresolved |
| Quit and cancel | Current workspace and terminal remain ready and unchanged |
| Quit and confirm | Only this instance's owned sidecar and descendants stop; another instance and foreign sentinel survive |

### Independent-instance isolation

| Scenario | Required observation |
| --- | --- |
| Run instance 1 on A and instance 2 on B | Each owns one sidecar, port, token, generation, session set, terminal, Peon, and Taskmaster context |
| Attention or waiting state occurs in A | B receives no attention count, popup, focus change, terminal attachment, or status mutation for A |
| Close or crash instance 1 | B remains usable; no peer cleanup or activation runs |
| Quit instance 2 while instance 1 remains active | Only B stops; A continues and its lease remains held |
| Both receive A as startup hint | Each independently attempts A's lease; one may conflict without discovering or activating the other |
| Concurrent global settings edit | Existing settings revision/atomicity rules govern the result; history adds no false coordination |
| Inspect files and IPC during both runs | No peer endpoint, process identity, focus, health, attention, or cleanup authority is published |

### Identity, lease, and request routing

| Scenario | Required observation |
| --- | --- |
| Simultaneous equivalent aliases | Filesystem identity coalesces to one runtime, lease, settings key, and metadata history |
| Windows aliases and case-sensitive directories | Junction/symlink, drive, separator, case, UNC, and extended forms coalesce only when OS identity proves it; distinct directories remain distinct |
| Separate Git worktrees | Separate identities and metadata; no worktree creation or deletion |
| Remove or replace B during resolution | Open fails visibly without raw-path fallback or unverified adoption |
| Replace B after Electron validation | Sidecar identity check fails before metadata loading, reconciliation, or lease adoption |
| Foreign sidecar owns B | Foreign sidecar and sentinel survive; metadata is untouched; requester cleans only its attempt |
| Delayed request overlaps close | Request and response remain bound to their original generation; stale publication is rejected |
| Generated prompt overlaps cleanup | Admitted write completes before acknowledgement or is rejected before writing; uncertainty is never blindly retried |
| Previously admitted keyboard bytes arrive during close | Bytes target only their original session and are not rerouted |

### Crash-surviving ownership prerequisite

| Scenario | Required observation |
| --- | --- |
| Process registration fails | Child never executes; operation fails closed |
| Sidecar crashes with PTY/inference descendants | Native owner proves generation membership, terminates only registered children within bounds, and acknowledges exit |
| Owned child survives deadlines | Instance remains unresolved and cannot report close success or launch a replacement |
| Foreign owner and sentinel coexist | Cleanup never broadens by PID, name, executable path, port, or workspace path |
| Force-terminate then relaunch | New instance waits for old-generation exit or authorized containment before adoption |
| Force-terminate one of two instances | Other instance and its descendants remain untouched |
| Unsupported ownership platform | Spawn, replacement, or relaunch requiring missing proof fails closed |

## Layers and platform coverage

1. Pure history and state-machine tests prove bounds, revisions, close-then-open
   ordering, deterministic outcomes, and stale-generation rejection.
2. Rust tests with temporary stores and native children prove canonical identity,
   lease-before-adoption, foreign-owner isolation, and bounded cleanup.
3. Native Electron tests with real sidecars prove picker, switching,
   restoration, close, quit, independent-process isolation, and crash/relaunch
   journeys on Windows, macOS, and Linux. Mocks do not satisfy this layer.
4. An owner-controlled smoke test with real coding tools checks hooks and login
   preservation without recording credentials or private prompts.

Run native failure scenarios only with disposable roots. Close the known
Windows defect families [#543](https://github.com/Rambolarsen/orkworks/issues/543)
and [#544](https://github.com/Rambolarsen/orkworks/issues/544) before treating
their paths as evidence.

Before enabling unavailable-sidecar cleanup, replacement, or relaunch adoption,
complete [#545](https://github.com/Rambolarsen/orkworks/issues/545) with native
Windows and macOS evidence plus portable Linux coverage. A sidecar handle, PID
list, unproven process group, or released lease is insufficient. Include every
production process family that can detach or leave descendants.

## Resource and durability experiment

Measure one instance and two independent instances with identical fixtures in
idle, active terminal-output, and active Peon-inference conditions. Warm up for
30 seconds and sample for 60 seconds, three repetitions. Record machine, OS,
revision, build mode, per-process RSS/CPU, child count, inference concurrency,
and output backlog. Keep debug and packaged results separate.

Repeat A-to-B switching 20 times and verify no monotonic child, handle,
temporary-file, or memory growth beyond measured steady state. Inject
destination resource failure and prove the picker remains usable, the attempt
is cleaned, and retry works after recovery. Set performance thresholds from
native measurements before release; this contract makes no unmeasured claim.

## Existing evidence, not completion

The checked-in process-ownership evidence bundle records five Windows native
fixture pass rows, explicit accepted fixture/protocol rows, and unresolved or
unsupported macOS/Linux/production-seam rows. It does not prove production
routing or authorize crash relaunch. Re-run the native matrix against the
implementation revision and attach native artifacts before marking issue #541
complete.
