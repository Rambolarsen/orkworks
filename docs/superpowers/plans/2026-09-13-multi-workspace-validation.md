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

## Acceptance matrix

| Scenario | Required observation |
| --- | --- |
| Open A, start its fixture, open B | A retains PID, port and uninterrupted numbered output; B owns different runtime identity and port |
| A reports need for input while B is focused | Switcher count increases once; B keeps focus; expanded A row identifies the attention |
| A returns to working or ends | Its count clears from current state; no accumulating notification |
| Switch back to A | Restore selected session and replay, then receive continuing output; one terminal attachment is visible |
| Selected session ends while backgrounded | Return shows its historical replay rather than silently selecting another session |
| Open A through an alias, including simultaneous requests | One canonical workspace and one sidecar; no spurious reconciliation |
| Open separate Git worktrees of the same repository | Separate workspace runtimes and metadata; no worktree creation or deletion |
| A and B have opposing workflow instructions and settings | Each model request and recommendation contains only permitted facts and effective settings for its own workspace |
| Delayed session/settings/review request overlaps a switch | Mutation stays bound to its originating workspace; stale foreground response is discarded |
| Global definition/settings edit with both open | Both affected sidecars refresh; workspace overrides remain intact; partial apply failure identifies its workspace |
| A-only Taskmaster settings edit | B's settings, diagnostics, recommendations and unrelated cache remain unchanged |
| Simultaneous first reporter installation and repair | Complete valid shared reporter bytes; both local integrations retain session-bound routing |
| A's inference is blocked, then switch to B | A's pending result is invalidated, cancellation targets inference only, and B cannot acquire execution lease until A exits |
| Refocus repeatedly after an inference reservation | No reservation refund, interval bypass, duplicate accepted output, or budget reset |
| Crash background A | B's port, input and settings remain usable; A shows unavailable/stale attention and independent bounded recovery |
| Retry or late exit from obsolete A generation | No change to B or A's replacement token/port; external metadata owner is never terminated |
| Cancel close confirmation | No session termination, focus change or history deletion |
| Close an unavailable workspace | Unknown liveness is shown explicitly and requires confirmation; failed polling is not treated as zero sessions |
| Confirm close A with running sessions | Only A's owned sessions/inference stop; flush and process exit precede closed state; A remains remembered |
| Session ends or another starts during close dialog | Recheck current generation/session set; broadened termination requires refreshed confirmation |
| Close focused workspace or final open workspace | Select remaining open MRU or picker; do not start a closed remembered workspace |
| Repeated app quit, cancel, then confirm | One dialog at a time; cancel preserves work; confirmation stops all owned runtimes and leaves no fixture child |
| Relaunch after confirmed quit | Only the last focused workspace opens; no coding session resumes automatically |
| Missing last location on startup | Picker and remembered list remain available; no silent project substitution |

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

Run the failure scenarios on native Windows after repairing the two known
failure families. Reuse #525's relevant process fixtures; do not claim that its
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
