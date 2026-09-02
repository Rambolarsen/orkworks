# Substantial PR Review Reruns Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rerun Claude after substantial cumulative PR changes without paying for every push.

**Architecture:** A cheap eligibility job runs on PR lifecycle/update events, resolves the exact PR head, finds the last completed review marker, and counts the delta with a testable Bash helper. The Claude action runs only for a threshold crossing or explicit manual dispatch.

**Tech Stack:** GitHub Actions, Bash, GitHub CLI/API, `jq`, and `anthropics/claude-code-action@v1`.

## Global Constraints

- Automatic events: `opened`, `reopened`, `ready_for_review`, `synchronize`.
- Manual dispatch requires a PR number and may bypass the threshold, but not PR-state or relevant-code checks.
- Automatic threshold: more than 8 non-`.md` files or more than 500 non-`.md` changed lines.
- Relevant paths: `apps/desktop/` and `crates/orkworksd/` only.
- Marker: `### Automated code review` followed by `<!-- orkworks-automated-review: sha=<40 lowercase hex characters> -->`.
- Only `github-actions[bot]` markers establish a baseline; old markers never suppress newer heads.
- Docs-only PRs never invoke Claude; the workflow remains informational and does not replace `/code-review`.

---

### Task 1: Add and test the cumulative-delta helper

**Files:**
- Create: `scripts/pr-review-delta.sh`
- Create: `scripts/pr-review-delta.test.sh`

**Interfaces:** `pr-review-delta.sh FROM_SHA TO_SHA` prints `FILES LINES`, counting non-`.md` diff records below the two relevant paths. Numeric additions/deletions count toward `LINES`; binary `-` values count as zero lines but still count as a file.

- [ ] **Step 1: Write failing fixture tests.**

Use a temporary Git repository and assert exactly 8 files/500 lines, 9 files, 501 lines, docs-only changes, irrelevant paths, a binary file, and a code-to-code rename. The test must initially fail because the helper is absent.

- [ ] **Step 2: Run the failing test.**

```bash
bash scripts/pr-review-delta.test.sh
```

Expected: failure because `scripts/pr-review-delta.sh` does not exist.

- [ ] **Step 3: Implement and test the helper.**

Implement it with `set -euo pipefail`, two-argument validation, `git diff --numstat --find-renames`, and `awk` numeric checks so binary values do not corrupt the line total. Make it executable, then run the same test and expect all fixture assertions to pass.

- [ ] **Step 4: Commit.**

```bash
git add scripts/pr-review-delta.sh scripts/pr-review-delta.test.sh
git commit -m "test: add PR review delta helper"
```

### Task 2: Update the Claude workflow

**Files:**
- Modify: `.github/workflows/pr-review.yml`
- Modify: `scripts/pr-review-delta.test.sh`

**Interfaces:** The `changes` job outputs `pr_number`, `base_sha`, `head_sha`, `review_base`, `relevant_code_changed`, `over_review_threshold`, `force_review`, and `should_review`; the `review` job runs only when `should_review == 'true'`.

- [ ] **Step 1: Add eligibility truth-table coverage.**

Assert that automatic relevant deltas at or below both limits skip, automatic relevant deltas above either limit review, automatic docs-only changes skip, manual force plus relevant code reviews at any size, and manual force plus docs-only changes skip.

- [ ] **Step 2: Implement normalized context and gating.**

Add `workflow_dispatch` with required `pr_number`, the four PR event types, and `concurrency` keyed by PR number with `cancel-in-progress: true`. Resolve pull-request payload fields for PR events and query `gh pr view` for manual runs. Reject closed/draft/non-`main` PRs and mismatched checkout/head SHAs. Check out the exact head with full history.

Read paginated issue comments with `gh api --paginate --slurp`, sort by creation time, and select the newest `github-actions[bot]` comment containing the exact marker. Validate SHA format, commit existence, and ancestry; otherwise use the PR base. Call the helper and set `should_review` to automatic threshold or manual force, after requiring relevant code.

Update the Claude prompt so an old marker does not suppress a new head; a manual force may bypass the current-head idempotency check. Require the exact marker/header in the posted comment and add a post-action API check that fails if the current-head marker is absent.

- [ ] **Step 3: Validate and commit.**

```bash
bash scripts/pr-review-delta.test.sh
actionlint .github/workflows/pr-review.yml
git add .github/workflows/pr-review.yml scripts/pr-review-delta.test.sh
git commit -m "ci: rerun Claude review after substantial PR changes"
```

If `actionlint` is unavailable, record that limitation and use the available YAML parser; the helper tests remain mandatory.

### Task 3: Update documentation and verify the live PR

**Files:**
- Modify: `AGENTS.md`
- Modify: `.github/workflows/pr-review.yml`

- [ ] **Step 1: Update descriptions.**

Change the CI-routing paragraph and workflow comments from “newly opened PRs” to “opened PRs and substantial cumulative updates,” preserving the non-blocking/manual-review caveat and describing the threshold and manual fallback.

- [ ] **Step 2: Run repository gates.**

```bash
bash scripts/pr-review-delta.test.sh
bash scripts/doc-check.sh
git diff --check origin/main...HEAD
```

Confirm no application runtime files changed.

- [ ] **Step 3: Push and inspect PR #411.**

```bash
git push origin harness-config-design
gh pr checks 411
gh api repos/Rambolarsen/orkworks/actions/runs --method GET -f branch=harness-config-design -f event=pull_request -f per_page=10
```

Confirm a new PR CI run is created for the current head, eligibility reports the cumulative comparison, and a successful Claude run leaves one current-head marker. Report a separate queued Claude GitHub App suite independently from this repository workflow.
