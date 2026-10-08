# Proposals Require Recurrence: Taskmaster Qualification Threshold Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the single-observation escape hatch so an `improve_workflow` proposal always requires at least two distinct qualifying observations sharing a fingerprint, ending the flood of one-evidence high-priority cards.

**Architecture:** The change is confined to the deterministic evaluator (`evaluate_workflow_improvements` in `crates/orkworksd/src/taskmaster/mod.rs`) and the eligibility wording in `specs/taskmaster.md` § Eligibility. Stored recommendations are not migrated; existing one-evidence cards persist until the user dismisses them or the evaluator's terminal-status rules supersede them.

**Tech Stack:** Rust (sidecar), markdown spec.

**Spec:** `specs/taskmaster.md` § Eligibility (lines ~548-553) — the authoritative threshold.

## Global Constraints

- Priority order stays `user > agent > peon > backend_inference > process > unknown > debug`; qualification thresholds are source-independent (already the case; do not add source gates).
- Keep the existing observation qualification gates untouched: confidence < `0.6` never qualifies; a `reportedImpact: high` observation additionally requires confidence ≥ `0.8` (`specs/taskmaster.md:553`, `crates/orkworksd/src/taskmaster/mod.rs:199-204`).
- Two inference results over the same unchanged Peon evidence window still count as one observation (fingerprint dedup already handles this; do not change it).
- Rust verification commands run with `--manifest-path crates/orkworksd/Cargo.toml` from the repo root; format with `cd crates/orkworksd && cargo fmt -- src/taskmaster/mod.rs` (never `cargo fmt -- <files>` from the repo root).
- Commits happen in the sibling worktree on branch `taskmaster-recurrence-floor`.

---

### Task 1: Update the spec eligibility wording

**Files:**
- Modify: `specs/taskmaster.md:546-553` (§ Eligibility)

**Interfaces:**
- Produces: the normative eligibility text the evaluator (Task 2) implements. No other spec section references the removed clause.

- [ ] **Step 1: Edit the eligibility bullet list**

Replace lines 548-551 (the "considers a cluster eligible when either: …" two-bullet list) with a single qualification clause:

```markdown
Taskmaster reevaluates five seconds after the latest accepted workflow observation in the active workspace, so a burst of related records can be considered together, and reconstructs its view from persisted observations after a restart. A deterministic evaluator considers a cluster eligible only when it contains at least two distinct observations sharing a fingerprint, each with confidence ≥ `0.6`; high-impact observations additionally require confidence ≥ `0.8`. A single observation, however confident or impactful, never proposal-qualifies — it remains stored as supporting context until a second distinct observation shares its fingerprint.
```

- [ ] **Step 2: Keep the following sentence intact**

Line 553's existing paragraph starting "Two inference results over the same unchanged Peon evidence window…" stays unchanged — it already covers fingerprint dedup and non-citation of below-threshold observations.

- [ ] **Step 3: Verify no stale references remain**

Run: `grep -n 'one observation' specs/taskmaster.md`
Expected: only matches that do not describe eligibility (e.g. dismissal-watermark prose). If an eligibility-adjacent match exists, update it.

- [ ] **Step 4: Commit**

```bash
git add specs/taskmaster.md
git commit -m "docs(taskmaster): require recurrence for improve_workflow proposals"
```

### Task 2: Evaluator — remove the single-observation escape hatch (TDD)

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/mod.rs:214-217` (implementation), `crates/orkworksd/src/taskmaster/mod.rs:947-962` (test rewrite), `crates/orkworksd/src/taskmaster/mod.rs:1149-1160` (fixture needs a second observation)

**Interfaces:**
- Consumes: `evaluate_workflow_improvements(observations: &[workflow_observations::WorkflowObservation], existing: &[Recommendation], workspace_id: &str, now: &str) -> Vec<Recommendation>` (signature unchanged).
- Produces: identical signature; only qualification semantics change — groups with fewer than two qualifying observations yield no proposal.

- [ ] **Step 1: Rewrite the behavior-pinning test (red)**

Replace `proposes_a_high_impact_single_observation_but_ignores_weak_evidence` (lines 946-962) with:

```rust
#[test]
fn ignores_single_and_weak_high_impact_evidence() {
    let high = observation("high", 1, "session-a", 0.9, Impact::High);
    let weak = observation("weak", 2, "session-b", 0.59, Impact::High);
    assert!(
        evaluate_workflow_improvements(
            &[high],
            &[],
            "workspace-1",
            "2026-08-21T12:00:00Z",
        )
        .is_empty()
    );
    assert!(
        evaluate_workflow_improvements(
            &[weak],
            &[],
            "workspace-1",
            "2026-08-21T12:00:00Z",
        )
        .is_empty()
    );
}
```

- [ ] **Step 2: Fix the single-observation fixture**

In `fix_prompt_includes_reference_snapshots_without_claiming_proactive_recurrence` (line 1152), add a second matching observation so the call still produces a proposal:

```rust
let mut recommendation = evaluate_workflow_improvements(
    &[
        observation("one", 1, "session-a", 0.9, Impact::High),
        observation("two", 2, "session-b", 0.9, Impact::High),
    ],
    &[],
    "workspace-1",
    "2026-09-09T00:00:00Z",
)
.remove(0);
```

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster`
Expected: `ignores_single_and_weak_high_impact_evidence` FAILS (today a 0.9 single high observation proposes) and `fix_prompt_includes_reference_snapshots_without_claiming_proactive_recurrence` FAILS (empty proposals → `.remove(0)` panics).

- [ ] **Step 4: Implement the minimal change (green)**

In `evaluate_workflow_improvements` (lines 212-219), delete the `high_impact_single` binding and the `&& !high_impact_single` conjunct so the gate reads:

```rust
let mut proposals = Vec::new();
for (fingerprint, mut qualifying) in groups {
    qualifying.sort_by_key(|observation| observation.sequence);
    if qualifying.len() < 2 {
        continue;
    }
```

Lines 199-204 (the `<0.6` skip and the high-impact `<0.8` skip) stay unchanged.

- [ ] **Step 5: Run the full sidecar test suite**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all PASS (all other evaluator tests already seed two or more observations; `session_application.rs` and `evaluator.rs`/`rollup_tests.rs` fixtures build recommendations directly, not through single-observation qualification).

- [ ] **Step 6: Format and commit**

```bash
cd crates/orkworksd && cargo fmt -- src/taskmaster/mod.rs
git add src/taskmaster/mod.rs
git commit -m "fix(taskmaster): proposals require two qualifying observations"
```

### Task 3: Verification and PR

**Files:**
- None new.

- [ ] **Step 1: One-shot verification**

Run: `bash scripts/verify-repo.sh`
Expected: all Rust, desktop, documentation, formatting, diff, and worktree checks pass; address failures individually if narrow.

- [ ] **Step 2: Doc currency check**

Run: `bash scripts/doc-check.sh`
Expected: no flagged files from this change (living prose that mentions the removed hatch in `docs/agents/architecture.md:468` and `docs/agents/peon-model-detection-troubleshooting.md:305` may flag — update those one-liners to describe the recurrence requirement).

- [ ] **Step 3: Commit, push, open PR**

Reference the verified noise pattern (206 single-evidence high-priority recommendations in this workspace's store) and the spec § Eligibility update in the PR body.

```bash
git push -u origin taskmaster-recurrence-floor
gh pr create --title "Taskmaster: require recurrence for improve_workflow proposals" --body "<see above>"
```

- [ ] **Step 4: Code-review gate**

Run `/code-review low` on the PR (diff-scoped). Address findings or note why each is intentional. This PR counts as a threshold/protocol change but stays small; escalate to medium only if findings warrant.

- [ ] **Step 5: Merge and cleanup**

Wait for required checks; the maintainer accounts for the review gate, then `gh pr merge <PR_NUMBER> --squash --admin`. Then:

```bash
bash scripts/finish-pr.sh <PR_NUMBER_OR_URL>
bash scripts/doc-check.sh
bash .claude/hooks/worktree-check.sh
```
