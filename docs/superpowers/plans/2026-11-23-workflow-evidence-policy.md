---
type: Implementation Plan
title: Workflow evidence policy implementation
description: Gated implementation and verification handoff for dual-view Peon grounding and age-bounded new workflow inputs.
tags: [taskmaster, peon, workflow, implementation, policy]
status: draft
workflow_status: completed
---

# Workflow evidence policy implementation

**Goal:** Enforce the [accepted policy](../specs/2026-11-23-workflow-evidence-policy-design.md)
without rewriting history or granting new authority. Tracks
[#1077](https://github.com/Rambolarsen/orkworks/issues/1077).

**Gate:** The owner accepted the concrete policy and authorized this plan on
2026-11-24 in direct response to the implementation approval question for #2024. Root `AGENTS.md` controls implementation sizing and review.
The root agent writes; execute inline after approval. No other coding harness
or unapproved model/native trial is needed.

## Investigated uncertainty and boundaries

- The existing ANSI parser already removes ESC payloads; reuse it only in the
  workflow evidence predicate. Inspect four current persistence call sites to
  avoid fixing only the live-file path.
- The shared recent-observation helper also feeds 128-row assessment/readiness.
  Add an explicit ordinary-analysis selector rather than changing those modes.
- Exact evaluation merges old proposed snapshots. Restrict the new age gate to
  new-ID construction, including successors; test grandfathered update branches.
- Ordinary provider replies can cross the cutoff. The selected-input identity
  and apply fence must prevent stale combined enrichment/proposal/rollup writes.
- Corpus labels measure support in a selected sample. Runtime tests and a model
  response fixture cannot supply missing current-usefulness evidence.

## Task 1: Dual-view Peon grounding

**Owner/files:** `crates/orkworksd/src/peon.rs` owns the shared predicate and
its unit tests. `session_application.rs` and
`session_application/sql_writes/lifecycle/history/{evidence.rs,sql_origin/batch.rs}`
own their existing persistence integration tests. No changes to agent HTTP
report validation, parser semantics or terminal capture.

**Deliverable:** `evidence_is_grounded` requires the existing raw contiguous
match plus the unchanged regular `strip_ansi` view match. Candidate text and
stored evidence are not normalized.

- [x] Write failing tests for BEL/ST OSC titles, hyperlink URL versus caption,
  DCS payload, split-row/incomplete payload, visible positive errors and
  raw-missing ANSI-interleaved strings. Include a printed quoted-error control
  that still passes, documenting the semantic limit.
- [x] Implement the shared predicate; exercise live/final/captured-file/SQL
  paths with real ESC strings and assert no observation or sequence advance
  for rejected candidates. Retain original candidate-index/replay receipts.
- [x] Verify accepted agent reports with non-terminal evidence remain accepted;
  situation/label/capacity and ordinary raw replay are unchanged.
- [x] Run focused Peon and observation integration suites before broader checks.
  Required result: all cases pass; controls fail when either raw or visible
  requirement is removed.

## Task 2: Recent evidence for new exact-family IDs

**Owner/files:** `taskmaster/context.rs` may own the shared pure age predicate;
`taskmaster/mod.rs` owns deterministic use and its tests. Keep existing
`recent_observations` behavior for unaffected consumers.

**Deliverable:** At a server-owned UTC `T`, recent means parsed RFC3339 age
between zero and 1,209,600 seconds inclusive. Only new-ID construction uses the
recent qualifying set; prior-ID updates follow their existing contract.

- [x] Test exact and fractional cutoff boundaries, offsets, future/invalid
  dates, both sources, insufficient recent rows, and recent rows hidden behind
  old/noisy rows. Inject time; no sleeps.
- [x] Test first creation and completed/dismissed/rolled-up terminal successors
  with all existing lineage/watermark conditions, plus grandfathered proposed
  and live-parent member updates retaining historical snapshots/counts.
- [x] Implement one shared predicate and deterministic branch selection.
  Derive new evidence/count/session/impact/confidence only from recent
  qualifying observations. Invalid evaluation time permits no proposed writes.
- [x] Verify observation bytes, counters/tombstones and existing recommendation
  records are unchanged by reads, restart and no-proposal passes.

## Task 3: Ordinary Brain selection and stale-response fence

**Owner/files:** `taskmaster/evaluator.rs` owns ordinary dispatch/relevance/
prompt/application; `taskmaster/context.rs` owns pure bounded selection.
Reuse existing runtime/workspace fencing; no new durable schema. Add tests
alongside the existing evaluator/currentness tests.

**Deliverable:** All ordinary observation-derived inputs use the same recent,
alphanumeric, latest-sixteen selection. The policy tag changes full-prompt cache
identity. Before combined effects, a current fenced selection identity must
match dispatch.

- [x] Test filter-before-bound with seventeen-plus rows, no observations,
  future/malformed times and unaffected 128-row readiness/assessment/manual-
  investigation contracts. Check ranking/excerpts and prompt use the same set.
- [x] Test a cutoff crossed while a fake provider is pending, replacement by
  new rows, deletion, workspace replacement, and an unchanged selection as time
  advances. Assert stale replies write no enrichment/proposal/rollup effects
  and perform no hidden retry; existing spent-call accounting remains.
- [x] Implement in-memory selection binding/currentness comparison under the
  existing apply owner; include `workflow-observation-input-v2` in ordinary
  prompt/cache input. Preserve native provider/settings/knowledge/rollup fences.
- [x] Verify restart/rollback compatibility without modifying frozen historical
  artifacts or claiming old model runs satisfy the new policy.

## Integration and acceptance checks

After approval, amend the authoritative `specs/orkworks-mvp.md` observation
adapter and `specs/taskmaster.md` eligibility/ordinary-analysis sections to the
accepted choices; update the relevant runbook's raw-plus-normalized grounding
and grandfathering statements. Read the Rust scoped guide, runtime contracts,
TDD and Rust skills before product edits.

Run focused suites, then `bash scripts/verify-repo.sh`; run docs checks/build,
`git diff --check`, `bash scripts/doc-check.sh` and
`bash .claude/hooks/worktree-check.sh`. Complete the current-code
`/code-review low` (escalate only if actual diff risk/size requires it) and the
normal PR/CI lifecycle. No live model call is required for deterministic
acceptance; empirical comparisons require their own finite authorization.
Keep #1077 open for its quality measurement criteria.

## Plan review

Disposition before owner review: **Ready for the scoped decision; not for execution**.
Coverage maps policy A to task 1, B–C to task 2, D to task 3, and compatibility/
documentation to their integration checks. No unrelated refactor, schema or
runtime activation is included.

| Dimension | Rating | Evidence / treatment |
| --- | ---: | --- |
| Dependencies | 2 | Existing parser and evaluator owners; shared 128-row consumers explicitly excluded |
| Blast radius | 3 | Peon evidence admission and ordinary Taskmaster inputs across persistence paths |
| State changes | 2 | New derived decisions; no codec/migration/history rewrite |
| Reversibility | 2 | Binary rollback reads all records, but restores noisy admission; old cards remain |
| Uncertainty | 3 | Semantic quality and current-usefulness benefits remain empirically unqualified |

Total 12/25 is a review aid, not approval. Remaining empirical uncertainty is
retained as an issue-level gate; deterministic acceptance must not claim it
resolved.

## Implementation verification — 2026-11-24

Implemented A–D after owner acceptance. The full repository verifier passed:
3,663 Rust tests (24 operator-only ignored), integration binaries, 32 reporter
tests, desktop type-check/native repair, 1,823 desktop tests, both builds and
2,583 docs checks. Focused RED/GREEN suites covered 93 evidence, 34 deterministic
recommendation and 100 evaluator tests. Existing prompt/audit fixtures were
aligned to their report clocks; policy and assertions were preserved.

The current-session `/code-review medium` found no actionable issue in the
changed code. Medium was selected for more than eight Rust files and the
application-currentness boundary. A fresh Luna review of the supplied literal
production patch also found no required issue; its shell host was unavailable,
so it is advisory rather than independent whole-branch verification.

The corpus hash remains unchanged. README's generic spec-change warning needs
no onboarding edit; domain claims are updated in the Taskmaster user guide and
Peon runbook. Worktree warnings concern other owners and were left untouched.
#1077 remains open for independent semantic/usefulness measurement and
same-window incident counting; these deterministic checks do not qualify it.
