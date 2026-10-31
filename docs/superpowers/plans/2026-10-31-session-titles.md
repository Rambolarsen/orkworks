---
type: Implementation Plan
title: Contextual session titles
description: Follow native Codex names, refine fallback topics once, and preserve manual titles.
tags: [orkworks, sessions, peon]
status: stable
workflow_status: complete
---

# Contextual session titles

Owner approved the native-first approach on 2026-10-31. Track implementation in
[#1203](https://github.com/Rambolarsen/orkworks/issues/1203); the manual-title
slice relates to #473 without implementing status overrides.

## Outcome and boundaries

Recognizable topic titles improve when context becomes available. Native Codex
names follow the exact accepted identity on subsequent authenticated reports.
Peon uses a dedicated title prompt and at most one successful contextual
refinement after useful output. Explicit user titles remain until the user
restores automatic naming. Activity summaries never overwrite titles.

Use the existing read-only Codex store adapter and guards from ADR 0063. New
Claude/OpenCode adapters, native title write-back, full transcript ingestion,
status overrides, and ongoing topic-drift detection are outside this increment.

## Delivery

- [x] Update the MVP naming contract and record ADR 0098 before dependent code.
- [x] Reproduce native-refresh suppression and activity-to-title contamination.
  Keep the existing cooldown, exact identity, runtime generation and reset
  guards. Avoid durable writes for unchanged native names.
- [x] Give input-title inference its own prompt through the existing provider
  runner. Bound task input and current-runtime terminal context. Persist an
  additive default-false contextual-refinement marker so daemon restarts do
  not restart title churn. Queue the contextual pass only after concrete
  work evidence, preserving bootstrap handoff and label epoch checks.
- [x] Add a desktop-authorized `/sessions/:id/title` read/set/reset route,
  bound to the expected canonical workspace. SessionApplication owns the
  transaction and live mirror. Reset uses a descriptive stored input or the
  placeholder and re-enables normal inference/native enrichment. Manual set
  and automatic reset invalidate pending label epochs; manual titles survive
  conversation resets. Do not mutate attention, summaries or native identity.
- [x] Add an accessible title editor in session details with Save, Cancel and
  Use automatic title. Reuse the authenticated Electron transport and current
  workspace identity; ignore obsolete UI responses after a session switch.
- [x] Cover source precedence, stale work, restore, invalid titles, workspace
  mismatch and native identity/reset races. Run `scripts/verify-repo.sh`,
  documentation/worktree close-out, then required `/code-review low` before
  PR merge. Update domain and API docs with verified source behavior.

## Execution evidence

Native refresh suppression, redundant native writes, activity/title contamination
and manual reset behavior were reproduced and corrected with regression tests.
The real Electron shell scenario covers save, cancel, automatic restore and
obsolete responses after switching sessions. The full `verify-repo.sh` pipeline
passed, including 1,617 desktop tests and the docs build (776 documentation tests).
Its initial Rust pass had 2,339 passing unit tests plus script/SDK integration
tests. After the reviewed scheduling correction, the final sequential Rust run
passed 2,341 unit tests (15 opt-in tests ignored) and all seven script/SDK
integration tests.

An intermediate rerun failed
`real_electron_synchronization_recovers_from_older_cache_but_refuses_equivocation_and_other_errors`
because the concurrent desktop build had replaced `fs-ext` with Electron ABI 149
while Node 22 required ABI 127. Rebuilding `fs-ext` for Node 22 and rerunning the
entire Rust suite sequentially resolved it; no product-code change was needed.

The medium-effort fresh-context code review approved staged tree
`447d9df847a6463598ddace9ebb5a5fd046ddf2a`. Its sole Required P2 finding was
overly narrow contextual scheduling. The actual-loop regression
`ordinary_investigation_output_refines_title_without_history_checkpoint` failed
under the old history gate, then passed after separating lexical context readiness
from activity-history evidence. The title prompt decides whether the available
text adds task details; spinner/prose readiness cases are covered separately.
Unchanged code retained its prior review evidence; the correction was reviewed
and approved with no remaining actionable findings.

Documentation currency warning disposition: README needs no change because this
is a session naming policy within the existing architecture, covered by the
updated source-build user guide, domain/API concepts and ADR index. No installer
availability or overall milestone claim changed. Existing Rust dead-code/test
warnings and Vite bundle-size warnings are outside this diff.

## Compatibility and recovery

Legacy provenance stays protected from native replacement. The new boolean
defaults false and older readers ignore it. No schema rewrite or native store
mutation is required. Revert restores the old naming policy without data loss;
existing `user` provenance already protects manual names from Peon/native writes.
If native metadata is unavailable, preserve the current title. Failed disk writes
must never publish a successful title in the live projection.

## Review

Ready for implementation within the approved scope. Dependencies 2 (existing
sidecar/provider/desktop contracts), blast radius 3 (shared title paths), state
changes 2 (additive boolean and existing user source), reversibility 1 (revert,
no destructive migration), uncertainty 2 (native path already qualified; tests
must pin the live projection and bootstrap races). Total 10/25. No new native
capability or security authority is introduced.
