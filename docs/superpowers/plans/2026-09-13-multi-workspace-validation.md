# Multi-workspace validation plan

Status: proposed validation contract; not an implementation plan or a test report
Date: 2026-09-13
Spec: [Concurrent workspaces](../../../specs/multi-workspace.md)
Tracking: [#541](https://github.com/Rambolarsen/orkworks/issues/541)

## Test setup

Use disposable workspaces A and B with distinct names, instructions, settings,
and recognizable public fixture text. Use deterministic native fixture coding
tools that emit numbered output, report attention through the real hook route,
wait for controlled input, and expose owned child PIDs for cleanup assertions.
Use a fixture inference executable that records start/cancel/exit events and
returns controlled evidence-bound JSON; no provider login or paid model is
required for deterministic tests.

All fixture processes must use isolated application metadata, settings, reporter
storage, and credentials. Verify every metadata root before launch; do not run
mutation tests against the user's home store. If the application cannot provide
isolated roots, implement a bounded test-environment seam before the live tests.
Record process ownership before injecting failures and terminate only fixture
processes. Tests must clean up their own temporary directories and children.

For foreign-owner cases, the test harness separately owns a sidecar holding a
fixture workspace lease and an unrelated sentinel child. Neither is registered
as an Electron-owned runtime. Keep their process handles and heartbeat channels
to prove survival through app actions; only the harness cleans them up afterward.

## Acceptance matrix

| Scenario | Required observation |
| --- | --- |
| Open A, start its fixture, open B | A retains PID, port and uninterrupted numbered output; B owns different runtime identity and port |
| A reports need for input while B is focused | Switcher count increases once; B keeps focus; expanded A row identifies the attention |
| A returns to working or ends | Its count clears from current state; no accumulating notification |
| Switch back to A | Restore selected session and replay, then receive continuing output; one terminal attachment is visible |
| Open more than ten locations, close some, then relaunch | Every successfully opened location remains remembered; only last focused opens; explicit Forget on a closed location removes only its shortcut |
| B fails readiness or a switch is superseded while A is focused | A remains usable with unchanged analysis permission and durable last focus; relaunch restores A |
| Selected session ends while backgrounded | Return shows its historical replay rather than silently selecting another session |
| Open A through an alias, including simultaneous requests | One canonical workspace, sidecar, settings key, metadata history and session owner; existing overrides/history preserved; no spurious reconciliation |
| Windows directory aliases and case-sensitive distinct directories | Junction/symlink and equivalent drive/case/separator/extended-path aliases resolve using OS identity; equivalent UNC spellings coalesce only when identity is proven; distinct directories stay distinct |
| Directory disappears or is replaced during resolution/open | Fail visibly before adopting a different identity; no raw-path fallback or duplicate metadata store |
| Open separate Git worktrees of the same repository | Separate workspace runtimes and metadata; no worktree creation or deletion |
| A and B have opposing workflow instructions and settings | Each model request and recommendation contains only permitted facts and effective settings for its own workspace |
| Delayed session/settings/review request overlaps a switch | Mutation stays bound to its originating workspace; stale foreground response is discarded |
| Global definition/settings edit with both open | Both affected sidecars refresh; workspace overrides remain intact; partial apply failure identifies its workspace |
| A-only Taskmaster settings edit | B's settings, diagnostics, recommendations and unrelated cache remain unchanged |
| A and B submit settings patches from the same document revision | First succeeds; second conflicts without settings/cache/diagnostic/budget mutation; refreshing and explicitly retrying preserves both workspace edits |
| Two processes concurrently save scoped Taskmaster settings | Disk-lock/revision contract prevents clobbering independently of Electron's queue; readers see a complete old or new settings document |
| Simultaneous first reporter installation and repair | Complete valid shared reporter bytes; both local integrations retain session-bound routing |
| A's inference is blocked, then switch to B | A's pending result is invalidated, cancellation targets inference only, and B cannot acquire execution lease until A exits |
| A revoke times out while A is still alive | No B grant or focus commit; bounded visible failure preserves A; late acknowledgements cannot complete the abandoned switch |
| B grant response is lost, then switch is abandoned | Reconcile B by confirmed revoke/exit before any A regrant; obsolete epoch commands cannot restore B permission |
| Focused A crashes, then select ready B | Confirm A process exit before B permission; UI can move, but new model calls wait for proof old inference exited, not merely a released lock |
| Refocus repeatedly after an inference reservation | No reservation refund, interval bypass, duplicate accepted output, or budget reset |
| Crash background A | B's port, input and settings remain usable; A shows unavailable/stale attention and independent bounded recovery |
| Open or recover A without focus, inject observations, and advance debounce/periodic timers | Evidence persists but no deterministic recommendation change, usage reservation, or provider call occurs; after focus, evaluation is eligible only under normal interval/cache/budget rules |
| Retry or late exit from obsolete A generation | No change to B or A's replacement token/port; external metadata owner is never terminated |
| Cancel close confirmation | No session termination, focus change or history deletion |
| Close an unavailable workspace | Unknown liveness is shown explicitly and requires confirmation; failed polling is not treated as zero sessions |
| Sidecar crashes leaving registered PTY/inference descendants | Surviving owner proves generation-bound membership, terminates only owned children within deadlines, and acknowledges exit; recovery cannot overlap surviving sessions |
| Process ownership registration fails during spawn | Child never executes; failure is visible and cannot leave an unregistered descendant |
| External sidecar owns A's lease; app open/retry conflicts, then close and quit | Foreign sidecar and sentinel heartbeats continue; foreign lease remains held and metadata untouched; only our attempted runtime is cleaned up |
| Owned child survives graceful cleanup and termination deadlines | Runtime remains visibly unresolved; cleanup never broadens to the foreign sidecar/sentinel or reuses a stale PID as ownership |
| Confirm close A with running sessions | Only A's owned sessions/inference stop; flush and process exit precede closed state; A remains remembered |
| Close ready A with no live/creating sessions | Atomic admission gate confirms empty set; no confirmation or session termination; sidecar/inference cleanup still completes |
| Session ends or another starts during close dialog | Recheck current generation/session set; broadened termination requires refreshed confirmation |
| Race start/resume with final close/quit snapshot | Request is either admitted and included in confirmation, or rejected by the atomic gate; Cancel releases gates without terminating sessions |
| Close focused workspace or final open workspace | Select remaining open MRU or picker; do not start a closed remembered workspace |
| Repeated app quit, cancel, then confirm | One dialog at a time; cancel preserves work; confirmation stops all owned runtimes and leaves no fixture child |
| Quit with an unavailable background runtime and no known live sessions | One confirmation still lists that workspace with uncertain count; no cleanup before confirmation |
| Close last window on Windows/Linux, then cancel | Same window, terminal attachment and controls survive; repeated close/quit coalesces; macOS window close retains its existing app-lifetime behavior |
| Relaunch after confirmed quit | Only the last focused workspace opens; no coding session resumes automatically |
| No last location, or missing/inaccessible last location on startup | Picker and remembered list remain available; no sidecar spawn, backend port, workspace lease or repo/home fallback until explicit selection |

## Layers and platform coverage

1. Pure controller/registry tests with controllable promises and clocks prove
   identity capture, focus ordering, stale-response rejection, counts and dialogs.
2. Rust tests with temporary stores and native fixture children prove leases,
   settings isolation, cancellation, bounded cleanup and atomic publication.
3. Native Electron tests with two real sidecars prove the complete switch,
   attention, close, crash and quit journeys. Exercise Windows and macOS;
   run portable backend coverage on Linux as well. Mocks do not satisfy this layer.
4. A small owner-controlled smoke test with real coding tools verifies hook
   behavior and login/environment preservation. Do not record credentials or
   private prompts. This does not replace repeatable fixture coverage.

Run the failure scenarios on native Windows after closing the two known
failure families with evidence: [#543](https://github.com/Rambolarsen/orkworks/issues/543)
and [#544](https://github.com/Rambolarsen/orkworks/issues/544). Before implementing
unavailable-runtime cleanup/recovery, demonstrate the surviving process-owner
registration and exit protocol on Windows and macOS, with portable Linux coverage,
and record the selected mechanisms in ADR 0056; track this prerequisite in
[#545](https://github.com/Rambolarsen/orkworks/issues/545). A sidecar-held PTY handle, PID
list, process group without proven containment, or released lease is insufficient.
Reuse #525's relevant process fixtures; do not claim that its
existing custom-inference coverage covers every native coding tool or Electron.

## Performance experiment

Compare 1, 2, and 4 open workspaces with identical fixtures. Measure separately:
idle, active terminal output without inference, and active Peon inference.
Warm up for 30 seconds and sample for 60 seconds per condition, three repetitions.
Record Electron and per-sidecar working set/RSS, CPU, owned child count, inference
concurrency, and output backlog. Identify machine, OS, build revision and build
mode. Do not combine debug and packaged-release results.

For each condition, measure 30 switches between already-ready workspaces and
report median/p95 click-to-visible-session latency. Measure 30 attention events
and report median/p95 hook-acceptance-to-indicator latency. Repeat open/close
20 times and verify no cumulative child-process, handle or memory growth beyond
the measured steady-state variation. Record raw samples and their units.

The structural gates are uninterrupted background work, no orphaned owned
processes, no unbounded backlog, and no monotonic leak across cycles. Resource
and latency thresholds must be set from the measured supported-machine baseline
and reviewed before release; this proposal makes no unmeasured speed/memory claim.
If active Peon contention causes those gates to fail, design bounded scheduling
as an explicit follow-up rather than silently pausing background work.

## Existing evidence, not release completion

At 7c61883, Windows: 38 focused desktop tests passed, and an in-memory
two-lifecycle mock check passed. The Rust taskmaster:: filter returned 70 passed,
8 failed, 2 ignored. The lease contention and representative recommendation-store
failures reproduced as individual tests. No real two-sidecar journey or resource
experiment has run. Re-run against the eventual implementation revision and
attach results before marking acceptance criteria complete.
