# Multi-workspace validation plan

Status: proposed validation contract; not an implementation plan or a test report
Date: 2026-09-13
Spec: [Concurrent workspaces](../../specs/multi-workspace.md)
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
Both sidecars share one disposable application-global root for settings, usage
ledger, analysis lease, knowledge and reporters; workspace metadata/overrides
remain in separate canonical workspace subtrees. Assert equal resolved global
paths and distinct workspace paths before testing cross-workspace behavior.
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
| View waiting A, then return to B without answering A | Viewing leaves A's attention state unchanged; background aggregate returns to one until A reports working or ends |
| Switch back to A | Restore selected session and replay, then receive continuing output; one terminal attachment is visible |
| Sequentially open and close more than ten locations, then relaunch | Every opened location remains remembered without requiring eleven concurrent sidecars; only last focused opens; Forget removes only a closed location's shortcut |
| B fails readiness or a switch is superseded while A is focused | A remains usable with unchanged analysis permission and durable last focus; relaunch restores A |
| Selected session ends while backgrounded | Return shows its historical replay rather than silently selecting another session |
| Open A through an alias, including simultaneous requests | One canonical workspace, sidecar, settings key, metadata history and session owner; existing overrides/history preserved; no spurious reconciliation |
| Windows directory aliases and case-sensitive distinct directories | Junction/symlink and equivalent drive/case/separator/extended-path aliases resolve using OS identity; equivalent UNC spellings coalesce only when identity is proven; distinct directories stay distinct |
| Directory disappears or is replaced during resolution/open | Fail visibly before adopting a different identity; no raw-path fallback or duplicate metadata store |
| Replace directory after Electron check but before sidecar adoption | Retained directory handle must match expected OS identity before metadata loading/reconciliation; replacement cannot inherit original registry/settings/history |
| Open separate Git worktrees of the same repository | Separate workspace runtimes and metadata; no worktree creation or deletion |
| A and B have opposing workflow instructions and settings | Each model request and recommendation contains only permitted facts and effective settings for its own workspace |
| Delayed session/settings/review request overlaps a switch | Mutation stays bound to its originating workspace; stale foreground response is discarded |
| Recommendation accept/plan-review submission overlaps focus revoke | Stale epoch/target rejected before PTY write; admitted write finishes before revoke acknowledgement/focus commit; no duplicate prompt or false accepted state |
| PTY write result becomes unknown during revocation | Recommendation stays visibly `executing`/unresolved, with no rollback or blind retry; revoke cannot acknowledge until pending write is drained/cancelled with no possible later write; timeout preserves current focus |
| Global definition/settings edit with both open | Both affected sidecars refresh; workspace overrides remain intact; partial apply failure identifies its workspace |
| A-only Taskmaster settings edit | B's settings, diagnostics, recommendations and unrelated cache remain unchanged |
| A reserves usage or shared ledger fails | B reflects shared remaining evaluations/ledger error; workspace analysis/cache diagnostics remain scoped |
| A and B submit settings patches from the same document revision | First succeeds; second conflicts without settings/cache/diagnostic/budget mutation; refreshing and explicitly retrying preserves both workspace edits |
| Two processes concurrently save scoped Taskmaster settings | Disk-lock/revision contract prevents clobbering independently of Electron's queue; readers see a complete old or new settings document |
| Simultaneous first reporter installation and repair | Complete valid shared reporter bytes; both local integrations retain session-bound routing |
| A's inference is blocked, then switch to B | A's pending result is invalidated, cancellation targets inference only, and B cannot acquire execution lease until A exits |
| A revoke times out while A is still alive | No B grant or focus commit; bounded visible failure preserves A; late acknowledgements cannot complete the abandoned switch |
| B activation response is lost after focus commit | B remains visibly/durably focused; activation cannot precede renderer focus acknowledgement; reconcile same epoch without extra evaluation/refund; switching away requires confirmed B revoke/exit |
| Focus persistence or renderer acknowledgement fails | No destination activation; pre-commit failure preserves A, post-commit renderer failure recovers into selected B with analysis suspended |
| Focus-memory replacement fails before/after publication or read-back fails | Complete old/new records only; read-back selects recovery, unknown outcome keeps analysis suspended; stale writers cannot overwrite newer focus |
| Focused A crashes, then select ready B | Confirm A process exit before B permission; UI can move, but new model calls wait for proof old inference exited, not merely a released lock |
| Refocus repeatedly after an inference reservation | No reservation refund, interval bypass, duplicate accepted output, or budget reset |
| Crash background A | B's port, input and settings remain usable; A's last attention count is stale, not zero, excluded from confirmed total, with an accessible unavailable indicator and independent bounded recovery |
| Select starting/recovering, unavailable, or closing A while B is focused | Existing bounded readiness is awaited, explicit error/Retry shown, or selection disabled respectively; failed/abandoned selection preserves B and last focus; recovery never steals focus |
| Open or recover A without focus, inject observations, and advance debounce/periodic timers | Evidence persists but no deterministic recommendation change, usage reservation, or provider call occurs; after focus, evaluation is eligible only under normal interval/cache/budget rules |
| Retry or late exit from obsolete A generation | No change to B or A's replacement token/port; external metadata owner is never terminated |
| Cancel close confirmation | No session termination, focus change or history deletion |
| Close an unavailable workspace | Unknown liveness is shown explicitly and requires confirmation; failed polling is not treated as zero sessions |
| Sidecar crashes leaving registered PTY/inference descendants | Surviving owner proves generation-bound membership, terminates only owned children within deadlines, and acknowledges exit; recovery cannot overlap surviving sessions |
| Process ownership registration fails during spawn | Child never executes; failure is visible and cannot leave an unregistered descendant |
| External sidecar owns A's lease; app open/retry conflicts, then close and quit | Foreign sidecar and sentinel heartbeats continue; foreign lease remains held and metadata untouched; only our attempted runtime is cleaned up |
| Owned child survives graceful cleanup and termination deadlines | Runtime remains visibly unresolved; cleanup never broadens to the foreign sidecar/sentinel or reuses a stale PID as ownership |
| Confirm close A with running sessions | Only A's owned sessions/inference stop; flush and process exit precede closed state; A remains remembered |
| Close ready A with no non-terminal sessions | Atomic admission gate confirms empty set; no confirmation or session termination; sidecar/inference cleanup still completes |
| Close A during session `ending` | Include session in confirmation; await bounded finalization/terminal transition before successful cleanup or report cleanup failure |
| Session ends or another starts during close dialog | Recheck current generation/session set; broadened termination requires refreshed confirmation |
| Race start/resume with final close/quit snapshot | Request is either admitted and included in confirmation, or rejected by the atomic gate; Cancel releases gates without terminating sessions |
| Close focused workspace or final open workspace | Select remaining ready open MRU or picker; do not start a closed remembered workspace |
| Close focused A while remaining entries are non-ready | Choose most recent ready entry only; with none ready, clear focus and show picker/statuses; later recovery does not auto-select; a chosen entry failing before commit returns to picker |
| Repeated app quit, cancel, then confirm | One dialog at a time; cancel preserves work; confirmation stops all owned runtimes and leaves no fixture child |
| Close overlaps quit/restart in either order | One owner of gates/dialog/cleanup; queued quit rediscovers after close; Cancel cannot release another operation's gates |
| Add/Open or recovery is resolving at quit discovery | Registry barrier includes registered attempt or rejects admission before spawn/adoption; no escaped runtime; Cancel releases gates in order without replaying blocked opens |
| Restart and install with background sessions | Aggregate confirmation includes all runtimes; Cancel preserves sessions; install waits for exit and requires install-specific authorization |
| Quit with an unavailable background runtime and no known live sessions | One confirmation still lists that workspace with uncertain count; no cleanup before confirmation |
| Close last window on Windows/Linux, then cancel | Same window, terminal attachment and controls survive; repeated close/quit coalesces; macOS window close retains its existing app-lifetime behavior |
| Relaunch after confirmed quit | Only the last focused workspace opens; no coding session resumes automatically |
| Cold start, first picker selection, or focused-runtime recovery | Fresh epoch follows ready -> durable focus/renderer adoption -> activation without source revoke; normal budget/interval apply; background recovery remains suspended |
| Force-terminate Electron with A focused and B background, then immediately relaunch | Ownership boundary cleans all old sidecars/descendants; new generation waits for cleanup proof, no hidden B process/lease survives, only last-focused A opens, no session resumes; foreign sentinel survives |
| No last location, or missing/inaccessible last location on startup | Picker and remembered list remain available; no sidecar spawn, backend port, workspace lease or repo/home fallback until explicit selection |

## Layers and platform coverage

1. Pure controller/registry tests with controllable promises and clocks prove
   identity capture, focus ordering, stale-response rejection, counts and dialogs.
2. Rust tests with temporary stores and native fixture children prove leases,
   settings isolation, cancellation, bounded cleanup and atomic publication.
3. Native Electron tests with two real sidecars prove the complete switch,
   attention, close, crash and quit journeys. Exercise Windows and macOS, plus
   native Linux Electron for the required last-window close/cancel/restart journey;
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
Include forced Electron exit and immediate relaunch: the supervisor or OS owner
must clean every prior background generation before new runtime adoption.
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

Increase fixture load beyond four workspaces in disposable supported-machine
environments until measured warning pressure is reached; record observed limits,
not a supported workspace maximum. Use controlled constraints/fault injection for
exhaustion rather than destabilizing the owner's machine. Verify one dismissible
warning per pressure episode, opening remains available, recovery clears it, and
existing work is never automatically paused or closed. Missing pressure telemetry
must not claim healthy state. Document native signals, sampling windows and
warning/recovery thresholds from measurements before release. Inject spawn/resource
failure and prove cleanup only of the attempted runtime, continued control of
existing sessions, and successful retry after recovery. Alias opens coalesce;
remembered closed entries add no processes.

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
