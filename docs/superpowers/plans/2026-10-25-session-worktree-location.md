---
type: Implementation Plan
title: Session task-worktree location
description: Delivery and verification plan for explicit task-worktree reporting.
tags: [sessions, git, worktrees, verification]
status: draft
---

# Session task-worktree location

Pending owner approval of the [design](../specs/2026-10-25-session-worktree-location-design.md).
Goal: report worktree B from a session launched in A/main, then inspect B in
Details and both Changes comparisons without changing launch/resume identity.
Implement inline in the owned `session-worktree-location` branch. No concurrent
writers or new coding harnesses. [Issue #1056](https://github.com/Rambolarsen/orkworks/issues/1056).

## Delivery

1. **Binding and authenticated mutation.** In `metadata.rs`,
   `metadata/agent_patch.rs`, `http/session_metadata_handlers.rs` and
   `session_types.rs`, add the optional sidecar-resolved binding and narrow
   `agentMetadata.worktreePath` patch from the design. First add failing tests
   using real linked worktrees for canonicalization/same-repository checks,
   set/clear/omit, wrong token, ended session, stale revision and atomic reject.
   Preserve unrelated metadata/provenance and latest-record binding across
   projection, lifecycle and legacy write paths. Extend domain-entity and
   runtime contract documentation and record the accepted ADR before code.
2. **Shared observation and viewer freshness.** Update `session_view.rs`,
   `session_projection.rs` and `http/git_changes_handlers.rs` to prefer and
   revalidate the binding; add source labels to shared API types and existing
   Details/Changes rendering. First demonstrate A/main versus B/feature in
   authenticated list/base/capture/patch tests. Test that a hook/process report
   of A cannot replace B, removal is unavailable, C invalidates B snapshots,
   clear restores legacy precedence, and history/restart retain B. Renderer
   tests must prove an in-flight B response cannot appear after switching to C,
   and a failed C refresh cannot retain B's capture.
3. **Agent reporting and delivery.** Update `scripts/session-metadata.py`,
   both `scripts/report-harness-event` implementations and integration fixtures
   under `crates/orkworksd/`. Add failing real-helper/router tests before the
   narrow command, with direct and explicit-proxy paths, clear and revision
   conflict. Test marker-free worktree-only instructions and commands while
   general bootstrap/get/patch remain marker-gated, combined SessionStart JSON,
   missing credentials, and stable/packaged helper reconciliation. Verify the
   existing `session_env_overrides_with_prompt_generation` supplies session ID,
   port and token without the API-only marker. Separate shared transport from
   the general Client marker check; test no-marker worktree reporting succeeds
   and every general command still rejects, including crafted argument paths.
   Update
   `skills/starting-work/SKILL.md` to report the owned task worktree where the
   helper is available, without changing other sessions or weakening sandbox
   policy. Qualify an installed helper from a real caller context; report any
   unavailable Windows/live-agent evidence separately from fixture results.

## Checks and handoff

- Run focused tests red then green for each delivery. Rust guidelines apply to
  all Rust changes; keep Electron and renderer types independently defined.
- Install frozen desktop dependencies, then `bash scripts/verify-repo.sh`,
  Rust build/test/fmt and relevant helper/desktop regression suites. Follow CI
  routing; docs checks/build cover contracts, ADR and indexes.
- Reproduce A/main -> report B/feature -> inspect B -> command in A -> inspect B
  with the real authenticated helper and viewer route. Qualify runtime delivery
  separately; a fixture alone does not prove agent compliance.
- Complete `/code-review low` of the current code diff before merge, then
  required checks, recommendation tie-off if applicable, squash PR and owned
  worktree cleanup under the existing repository workflow.

## Risk assessment and plan review

Dependencies 3: known metadata/helper/renderer interfaces; no new dependency.
Blast radius 3: shared location resolver; legacy precedence tested explicitly.
State changes 3: additive optional binding; same-lane mutation and preservation.
Reversibility 3: local revert restores legacy behavior; older binary rewrites may
lose bindings, so mixed-version writers are unsupported.
Uncertainty 3: tool versus launch mismatch reproduced; agent instruction
compliance still needs installed-context qualification. Total 15/25; judgment,
not an approval threshold.

Planning investigation rejected command-string parsing and most-recent-worktree
inference because neither establishes session ownership. Native hook CWD alone
is insufficient (ADR 0032). The current metadata helper checks the API-only
marker in Client construction; the worktree command must separate that narrow
authority without bypassing the general-command gate. End/history, missing-root,
legacy rewrite and non-editing commands in A are included above.

Review disposition: Ready for owner design approval, not implementation.
The one-pass read-only builder check raised marker-free credential delivery;
source inspection confirmed the existing session environment override supplies
the three credentials independently of the API-only marker. The design and
step 3 now name that source and require isolated marker-free command tests.
No other material builder question remained. Approval is required
for this new binding/schema contract; the bugfix request authorizes preparation
and subsequent implementation after that concrete gate.
