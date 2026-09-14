# Taskmaster rollup review fixes implementation plan

> **For agentic workers:** Execute the scoped regression/fix tasks below using the existing Superpowers TDD and verification workflows.

**Goal:** Resolve the five current-head findings on PR #549, tracked by #555.
**Architecture:** Preserve the existing evaluator, graph transaction, and detail API. Enforce provider-input and cache identity at request preparation; reconcile omitted groups under the existing workspace lock; validate persisted IDs; fetch member evidence only when expanded.
**Tech Stack:** Rust/Tokio/serde; React/TypeScript; existing Node/Electron test fixtures.
**Spec:** `specs/taskmaster.md`; `docs/superpowers/specs/2026-09-13-taskmaster-recommendation-rollups-design.md`.

## Constraints and checkpoint

- 128 KiB complete rollup-bearing input; 64 KiB model output; at most 32 supplied families and eight output clusters.
- Model output remains untrusted. No new API, provider, action authority, or dependency.
- Workspace-instance, generation, evidence, and parent-state checks apply equally to empty results.
- Only supplied complete active-parent groups may be dissolved. Unsupplied and terminal groups remain unchanged.
- Complete member evidence stays behind the existing detail API; parent projections remain bounded.
- Least-confident assumptions: the combined prompt currently escapes the fragment bound; an empty successful result bypasses both revalidation and dissolution. Code inspection confirms both paths. Reject an oversized composed request before reservation; reconcile empty results through the normal locked transaction.
- Project blind spot: representative parent evidence is not the complete audit source. Load member records on expansion and reject stale completions when the card/workspace changes.

## Routing and review budget

Root owns all implementation, integration, and publishing. PR #549 was merged at 2026-09-14T11:42:53Z (f28f2eb7), so these same authorized fixes will land through a follow-up PR based on current main; the merged branch is no longer the integration target. Two read-only agents independently inspect the UI and store invariants; neither depends on the other's result. After integration, use separate correctness and completeness review questions against the final diff. Maximum four subagent jobs, one review round; no autonomous rework loop. PR #549 reached its terminal state with three substantial review cycles recorded. Preserve that history; do not request another review on the merged PR. The follow-up PR starts its own bounded review lifecycle.

## Task 1: request boundary and cache identity

Files: `crates/orkworksd/src/taskmaster/evaluator.rs`, `evaluator/rollup_tests.rs`.

- [x] Add boundary regressions against `compose_provider_prompt` using a valid rollup request and UTF-8 legacy text so the final composition is exactly 128 KiB and then exceeds it by one byte. The former succeeds, the latter returns an error.
- [x] Add cache-key regressions against `provider_cache_key`: identical inputs match; a different workspace instance or prompt version differs; no-rollup evaluation preserves `snapshot.cache_key(prompt)`.
- [x] Run focused tests and observe missing behavior.
- [x] Extract composition into `compose_provider_prompt(legacy: String, request: Option<&RollupEvaluationRequest>) -> Result<String, String>`. Record errors and return before `reserve_snapshot`.
- [x] Add `provider_cache_key(snapshot, prompt, request)` combining the existing snapshot key with workspace instance, rollup prompt version, and family snapshot hash when present.
- [x] Run focused tests.

## Task 2: authoritative empty/subset clustering results

Files: `session_application.rs`, `taskmaster/evaluator.rs`, `taskmaster/evaluator/rollup_tests.rs`.

- [x] Seed two active parents; return one retained cluster and check that the other supplied parent becomes superseded and its members proposed.
- [x] Return an empty result; check transactional release and retention of historical parent/member evidence. Repeat with stale token/evidence and assert no mutation. Supply only one complete parent group and assert unsupplied groups unchanged.
- [x] Observe failing assertions.
- [x] Remove empty-result success bypasses. Move superseded-parent release outside the cluster loop, using supplied active-parent IDs minus returned parent IDs. Preserve assigned members and record expected hashes for every changed parent/member under the existing workspace lock.
- [x] Run focused tests.

## Task 3: persisted parent identity

Files: `taskmaster/store.rs` and its existing test fixtures.

- [x] Persist a parent with matching member/dedupe relationships but an incorrect parent ID; store reads must return `GraphInvariant`. Verify correct IDs still read successfully.
- [x] Observe the old validator accepting the malformed graph.
- [x] Compare every nonempty parent member list against `stable_rollup_id`; normalize through that canonical helper and retain existing duplicate-member validation.
- [x] Convert synthetic test parents to valid derived IDs; do not weaken malformed-graph assertions.
- [x] Run store and rollup tests.

## Task 4: member-family evidence

Files: `apps/desktop/src/components/RecommendationsPanel.tsx`, focused component/test helpers as needed.

- [x] Add behavioral coverage: collapsed cards make no member calls; expansion retrieves each listed ID; evidence omitted by the parent projection appears under the correct family with source/time; stale completions are discarded; failure exposes retry.
- [x] Observe failure before implementation.
- [x] Use the existing backend URL and `getTaskmasterRecommendation` API. Render each loaded family's complete evidence under its own heading/details group. Keep exact-family cards unchanged and action controls only on the parent card.
- [x] Run focused desktop tests/typecheck.

## Integration and completion

- [x] Run Rust fmt, full tests, and build; desktop full tests, typecheck, and build.
- [x] Review correctness and requirement completeness against the integrated diff, including partial batches, UTF-8 byte counting, and stale member detail loads. Manual `/code-review medium`: approve, no findings. Independent completeness review: all five live paths implemented; requested stale-evidence/terminal/partial-batch regression gaps were added and all 26 evaluator tests pass.
- [ ] Commit only owned files; push the isolated `rollup-review-fixes` branch and open a follow-up PR into current main, linking #549 and #555.
- [ ] Reply in all five original review threads with the follow-up PR and evidence; record validation and manual review on the new PR.
- [ ] Monitor the new PR head/checks and automated reviews according to the PR skill, preserving the three-cycle and wall-clock limits.
