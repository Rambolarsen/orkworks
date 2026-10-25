---
type: Implementation Plan
title: Repository agent-readiness scoring and history
description: Proposed bounded delivery plan for repeatable Brain-based readiness ratings and an overall score.
tags: [orkworks, taskmaster, readiness, implementation]
status: draft
workflow_status: proposed
---

# Repository agent-readiness scoring and history

Tracking: [#1063](https://github.com/Rambolarsen/orkworks/issues/1063).
Specification: [repository agent readiness](../../../specs/repository-agent-readiness.md).
Decision: [ADR 0095](../../adr/0095-repository-agent-readiness-history.md).

Product intent is approved. Concrete protocol/storage and publisher delivery
remain proposed; no runtime implementation or publication is authorized by
this artifact. Continue inline using `executing-plans` after owner approval;
do not launch another coding harness. Use TDD for runtime changes and the scoped
Rust/desktop instructions. Keep existing workflow assessment behavior unchanged.

## Deliverables and checks

- [ ] **Complete reviewed rubric delivery.** Extend the Brain publisher/client
  schema with an exact format-2 fourteen-concept rubric descriptor and independent signed
  readiness capability. Review a sanitized proposed export and format-1 manifest compatibility before publishing; retain old-client compatible bundles. Ownership
  spans the Brain publisher and `taskmaster/knowledge.rs`,
  `taskmaster/runtime.rs` activation persistence and Electron
  knowledge transport/cache and generated packaged starter. Use publication
  tooling rather than manual generated asset edits. Tests reject missing,
  duplicate, unknown or incomplete rubric mappings, mismatched hashes,
  unsupported versions and tampered proof; new clients can use an eligible
  verified starter offline while existing workflow assessment retains its bundle.
  If publication is unavailable, surface readiness unavailable without fallback.

- [ ] **Readiness protocol and deterministic score.** Add focused
  `taskmaster/readiness_model.rs` and a readiness-specific discovery/schema module
  beside existing assessment modules. Reuse `assessment_context.rs` as the sole
  permission/evidence owner; do not duplicate traversal or weaken existing limits.
  Wire native Codex static response constraints through the existing typed
  inference boundary. Pin fourteen unique rows, integer/null combinations,
  supplied fact/rubric references, field/whole-response bounds and the exact
  formula. Tests cover 0/50/75/100, rounding boundaries, missing/duplicate areas,
  hallucinated references, absent versus truncated files, partial confidence and
  unsupported verification claims. A model's overall score must be rejected.

- [ ] **Manual orchestration and durable snapshots.** Add focused readiness
  scheduler/report/store/access modules under `taskmaster/`, registered in
  `taskmaster/mod.rs`. Reuse the shared runtime analysis lease and captured
  transport; do not reuse the active-recommendation admission gate for this
  read-only action. Use libgit2 for nullable HEAD and tri-state dirty metadata.
  Persist separate bounded run/history files exactly as specified. Deterministic
  transport tests prove zero calls on ineligible input, at most two calls,
  no retries, uncached explicit reruns, polling without dispatch, workspace and
  configuration/input race interruption, and no recommendation/store mutations.
  Store tests cover partial/full runs, 100-entry eviction, oversized/corrupt data,
  atomic replacement failure, completion idempotence and crash boundaries before
  and after history commit. Prior completed history survives failed new runs.

- [ ] **Historical access and comparison.** Integrate readiness reconciliation
  with existing settings-narrowing/startup ownership and lock order; extend every
  known workspace reconciliation, not only the selected one. Fixture-test
  excluded paths, lower context level, unopened workspace, pending narrowing,
  in-flight cancellation and re-expansion. Removed permissions leave no score,
  prose, citation or manifest-derived data exposed. Ordinary file changes retain
  historical snapshots and mark latest freshness accurately. Pure comparison
  tests cover comparable full runs, per-row and rounded-total deltas, partial
  coverage, incompatible rubric/provider/effort/access/schema versions, model
  variability for identical input and no cross-worktree aggregation.

- [ ] **Authenticated API, independent transport types and UI.** Add narrow
  `POST /taskmaster/assess-readiness` (empty body; run-ID admission) and
  `GET /taskmaster/readiness` (selected workspace's bounded history/run status)
  in the authenticated sidecar router. Both follow current workspace identity
  and authorization fences; no model-supplied target paths. Connect narrow
  Electron/preload methods and independent types on each side of the boundary.
  In `RecommendationsPanel.tsx`, add the manual action and a focused readiness
  summary/history component. Tests cover unauthorized/stale workspace requests,
  score details, baseline choice, partial/redacted/failed states, previous-result
  preservation, fixed progress, keyboard disclosure and comparison conditions.
  Verify the actual rendered layout at narrow/desktop widths with long evidence.
  Preserve one central context and existing Assess workflow/Analyze now behavior.

- [ ] **Quality evidence and release documentation.** With an eligible reviewed
  rubric, use isolated temporary fixture workspaces to compare a known documented
  baseline with one controlled improvement and a healthy no-change rerun. Run
  three independent uncached selected-provider trials per fixture; retain actual
  dispatch identity, model/effort, rubric and sanitized outputs. Every scored row
  must have semantically valid evidence; zero fabricated execution claims or
  false complete-coverage claims. The controlled change must improve its intended
  component in at least two of three paired runs without unrelated invented gains.
  Do not require identical scores across model reruns; record variability and
  block usefulness claims if the matrix fails. No live-user metadata resets.
  Update domain entities, architecture, user guide and spec implementation status
  only to match delivered source behavior; distinguish installer availability.

Run scoped tests as each deliverable lands. Before runtime PR handoff, run Rust
build/test/fmt, relevant desktop checks and `bash scripts/verify-repo.sh`, then
`bash scripts/doc-check.sh` and `bash .claude/hooks/worktree-check.sh`. Complete
current-diff `/code-review low` before any runtime merge. Docs-only design
verification uses OKF checks/docs build, diff checks and close-out hooks.

## Investigated risks and plan review

The least certain dependency was complete rubric availability. Inspecting the
packaged signed starter confirmed only three maturity-model concept pages and
`taskmaster-assessment-v1`; complete signed rubric delivery is now a prerequisite,
not an assumption that a prompt can compensate for. The specification now pins bundle format 2 with unchanged format-1 manifest
and activation, a strict rubric descriptor and canonical digest. Existing manifest
validation accepts unsupported entries and old selectors filter format 1; paired
old/new-client fixtures must establish this before the reviewed export publishes.

The main product blind spot is attributing a changed model judgment to a fix.
Comparison conditions, immutable input identity, confidence and repeated fixture
trials make that distinction visible. A bounded read-only collector cannot verify
every runtime capability, so an honest partial result remains valid. Historical
retention also requires different freshness behavior from actionable proposals;
we preserve history but extend access narrowing to derived scores themselves.

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 5 | Complete reviewed rubric requires coordinated Brain publication/client compatibility |
| Blast radius | 3 | New action reuses lease/collector; shared access-narrowing consumers need regression coverage |
| State changes | 3 | Additive versioned local files; existing assessment records unchanged |
| Reversibility | 4 | Before metadata, ordinary rollback; after retention, forward fixes preserve privacy fences |
| Uncertainty | 3 | Deterministic contracts are testable; score usefulness needs actual selected-model paired trials |

Total: 18/25; a risk description, not approval.

Review disposition: **Ready for owner concrete-design review**, after one
fresh-context correctness review and one builder/completeness review. Findings
were resolved by pinning publisher format/descriptor compatibility, readiness
wire/storage shapes and canonical identities, full captured inference guards,
collector reuse boundaries, admission serialization, conservative narrowing and
freshness triggers. The current implementation's installation-wide lease is
explicitly distinguished from the independent-instance proposal. No unchanged
approved workflow artifact was reopened. Owner concrete-design approval and
reviewed publisher export remain required before dependent implementation and
publication respectively.

Documentation review note: the doc-drift hook suggests README review because a
proposed ADR was added. README behavior/availability claims remain accurate;
no runtime or released feature is advertised by this proposal. The initial
worktree check called this uncommitted branch merged because its HEAD still
matched main; it is the owned active design checkout, not a cleanup target.
