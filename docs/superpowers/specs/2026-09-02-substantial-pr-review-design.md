# Substantial PR Review Reruns

## Context

The automated Claude review workflow currently runs only for the `opened`
pull-request event. If implementation commits are added after a documentation
first pass, the workflow never reviews the resulting code. Triggering Claude on
every `synchronize` event would correct that gap but would spend review budget
on every small push.

## Decision

Run the inexpensive eligibility job on PR open, reopen, ready-for-review, and
update events. Invoke Claude only when the non-documentation changes since the
last completed automated review exceed the repository's existing review-gate
threshold: more than 8 changed files or more than 500 changed lines.

The workflow records the reviewed head SHA in a machine-readable marker in its
automated comment. On the next update, the eligibility job compares that SHA to
the current PR head. If no marker exists, it compares the PR base to the head,
which provides a one-time catch-up review for PRs that were opened before code
was added. If a force-push makes the marker unavailable from the current head,
the comparison falls back to the PR base.

Rapid updates use one concurrency group per PR with cancellation enabled. A
newer push therefore supersedes an in-progress eligibility or review run.
Manual workflow dispatch remains available with a PR number input for an
intentional review when the threshold has not been reached.

## Workflow contract

`.github/workflows/pr-review.yml` will:

1. Trigger on `opened`, `reopened`, `ready_for_review`, and `synchronize`, plus
   `workflow_dispatch` with a required PR number.
2. Check out the PR head with full history.
3. Read the latest bot-authored review marker from PR issue comments.
4. Count changed non-`.md` files and additions plus deletions between the
   marker (or base) and the current head, restricted to `apps/desktop/` and
   `crates/orkworksd/`.
5. Run Claude only when the file or line threshold is exceeded. The prompt
   includes the current full SHA in the marker and continues to identify the
   result as an informational first pass, not the required manual review.

The existing review prompt, permissions, secret, and Claude action remain in
place. The workflow never mutates repository code or merges a PR.

## Failure behavior and safety

- Missing or malformed review markers cause a base-to-head comparison rather
  than silently suppressing review.
- A marker is accepted only from a GitHub bot comment and must contain a full
  lowercase commit SHA.
- A stale or non-ancestor marker falls back to the PR base.
- Documentation-only changes never invoke Claude.
- The cheap eligibility job may run for every update; the Claude action may
  run automatically only when the cumulative threshold is reached. An
  explicit manual dispatch may invoke it below the threshold.

## Verification

Validate the workflow YAML and shell logic locally, then run the repository's
existing documentation check. Push the change to the open PR and confirm that
the updated head creates a PR CI run and that the review workflow's eligibility
job reports the expected cumulative comparison. The local implementation test
suite remains the source of truth for the feature PR; this CI-only change does
not alter application runtime behavior.
