# Substantial PR Review Reruns

## Context

The repository's `.github/workflows/pr-review.yml` workflow uses
`anthropics/claude-code-action@v1` and currently runs only for the `opened`
pull-request event. If implementation commits are added after a documentation
first pass, the workflow never reviews the resulting code. Triggering Claude on
every `synchronize` event would correct that gap but would spend review budget
on every small push. This design targets that GitHub Actions workflow, not a
separate Claude GitHub App check suite.

## Decision

Run the inexpensive eligibility job on PR open, reopen, ready-for-review, and
update events. Invoke Claude only when the non-documentation changes since the
last completed automated review exceed the repository's existing review-gate
threshold: more than 8 changed files or more than 500 changed lines.

The workflow records the reviewed head SHA in this exact machine-readable
marker near the start of its automated comment:

```text
### Automated code review
<!-- orkworks-automated-review: sha=<40 lowercase hex characters> -->
```

On the next update, the eligibility job compares that SHA to the current PR
head. If no valid marker exists, it compares the PR base to the head, which
provides a one-time catch-up review for PRs that were opened before code was
added. If a force-push makes the marker unavailable from the current head, the
comparison falls back to the PR base. The Claude prompt skips only when a
marker matches the current head SHA; an older review comment must never suppress
a new review.

Rapid updates use one concurrency group per PR with cancellation enabled. A
newer push therefore supersedes an in-progress eligibility or review run.
Manual workflow dispatch remains available with a PR number input for an
intentional review when the threshold has not been reached. Manual dispatch
sets an explicit force flag, but still requires an open, non-draft PR targeting
`main` with at least one changed file under `apps/desktop/` or
`crates/orkworksd/`; documentation-only PRs are never sent to Claude.

## Workflow contract

`.github/workflows/pr-review.yml` will:

1. Trigger on `opened`, `reopened`, `ready_for_review`, and `synchronize`, plus
   `workflow_dispatch` with a required PR number.
2. Resolve one normalized PR context for both trigger types. For pull-request
   events use the event payload; for manual dispatch query the PR API and reject
   closed, draft, non-`main` PRs or a head SHA that does not match the checked
   out ref. The prompt uses this normalized PR number and head SHA rather than
   assuming `github.event.pull_request` exists.
3. Check out the resolved PR head with full history.
4. Read all paginated PR issue comments and select the newest comment by
   `github-actions[bot]` containing the exact marker and a full lowercase SHA.
   A missing, malformed, or non-ancestor marker falls back to the PR base.
5. Count changed non-`.md` files and additions plus deletions between the
   marker (or base) and the current head, restricted to `apps/desktop/` and
   `crates/orkworksd/`. The calculation handles binary files and renames
   without treating their `-` numstat values as line counts.
6. Run Claude automatically only when the file or line threshold is exceeded;
   a manual dispatch may bypass that threshold but not the relevant-code and
   PR-state checks. The prompt includes the current full SHA in the marker and
   skips only an exact current-head marker.
7. After Claude returns successfully, verify that a bot-authored comment for
   the current head contains the exact marker. A missing marker fails the job
   so a later update cannot mistake an unmarked result for a completed review.

The existing review prompt, permissions, secret, and Claude action remain in
place. The prompt's idempotency check changes from “any automated review
comment” to “a marker for this exact head”. The workflow never mutates
repository code or merges a PR.

## Failure behavior and safety

- Missing or malformed review markers cause a base-to-head comparison rather
  than silently suppressing review.
- A marker is accepted only from `github-actions[bot]`, because the workflow
  posts through `gh pr comment` using `GITHUB_TOKEN`, and must contain a full
  lowercase commit SHA.
- Comment lookup uses API pagination and comment creation order; edited
  comments do not create a second review baseline.
- A stale or non-ancestor marker falls back to the PR base.
- Documentation-only changes never invoke Claude.
- The cheap eligibility job may run for every update; the Claude action may
  run automatically only when the cumulative threshold is reached. An
  explicit manual dispatch may invoke it below the threshold.
- A canceled or failed review without a verified current-head marker does not
  advance the baseline, so the next substantial update retries from the last
  completed marker or PR base.
- `concurrency.group` is keyed by PR number and `cancel-in-progress: true`;
  the newest push owns the only eligible review run.

## Verification

Keep the numstat/counting logic in a small testable helper at
`scripts/pr-review-delta.sh`, with `scripts/pr-review-delta.test.sh` covering:

- exactly 8 files and exactly 500 changed lines (no automatic review);
- 9 files and 501 changed lines (automatic review);
- documentation-only changes;
- a valid marker baseline, a missing marker, and a non-ancestor marker;
- binary and renamed files; and
- manual force below the threshold while still rejecting docs-only PRs.

Run `bash scripts/pr-review-delta.test.sh` and validate the workflow with
`actionlint .github/workflows/pr-review.yml` when `actionlint` is available.
Run `bash scripts/doc-check.sh` as the repository documentation gate. Push the
change to the open PR and confirm that the updated head creates a PR CI run,
the review workflow's eligibility job reports the expected cumulative
comparison, and a successful Claude run leaves the exact current-head marker.
Update the root `AGENTS.md` CI-routing paragraph and the workflow comments to
describe substantial-update reruns. The local implementation test suite
remains the source of truth for the feature PR; this CI-only change does not
alter application runtime behavior.
