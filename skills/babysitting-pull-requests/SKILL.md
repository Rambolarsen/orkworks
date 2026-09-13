---
name: babysitting-pull-requests
description: Use when babysitting an open pull request after it is opened, or when monitoring one for review comments, CI changes, requested changes, or merge readiness.
---

# Babysitting Pull Requests

## Overview

Opening a pull request starts an active lifecycle; it is not completion. Keep
the PR moving until it is merged or closed, or until the bounded self-babysit
budget requires an explicit handoff.

## Start with a complete inventory

Resolve the concrete PR number and current head commit. `gh pr view` is useful
for state and checks, but its `comments` and `reviews` fields do **not** expose
all inline review comments. First query the PR metadata, then query every
comment and check channel:

```bash
gh pr view <pr> --json state,isDraft,mergeable,mergeStateStatus,reviewDecision,reviewRequests,baseRefOid,baseRefName,headRefOid,headRefName,headRepository,isCrossRepository
gh api --paginate repos/<owner>/<repo>/issues/<pr>/comments
gh api --paginate repos/<owner>/<repo>/pulls/<pr>/comments
gh api --paginate repos/<owner>/<repo>/pulls/<pr>/reviews
gh pr checks <pr>
```

After the channel queries finish, fetch `headRefOid` again. If it differs
from the first metadata result, discard the pass and restart the complete
inventory for the new head so comments or checks from a concurrent push are
not missed.

The pull-request comments endpoint is mandatory: it contains line comments and
thread replies that can carry actionable feedback. When thread resolution
state matters, inspect the review threads in the forge UI or its thread API as
well. Review bodies can also contain suppressed or summarized comments; inspect
those items instead of treating the review summary as informational only.

## Vet and disposition every item

For each comment or review not already handled:

1. Read the complete thread and inspect the referenced diff, specs, and tests.
2. Classify it as actionable, informational, duplicate, or already handled.
3. Vet actionable feedback against the codebase and requirements. Fix it, or
   push back with a specific reason and evidence; do not accept suggestions
   blindly.
4. Reply in the inline thread for line-level feedback. Record a disposition
   for informational or duplicate comments too.
5. If uncertain whether feedback is correct, keep the human partner informed
   and ask for direction before making a consequential choice.
6. Before changing the tree, verify that this session owns the exact head
   branch (`headRefName` and head repository) or has the owner's explicit
   authorization to modify and push it. Same-repository status alone is not
   branch ownership evidence. If the branch is foreign or ownership is
   uncertain and authorization is absent, do not commit or push; report the
   blocker and hand off instead.
7. If the tree changes, verify the fix, push it, and refresh CI and comments.

Do not report “no comments” unless the complete inventory was performed. Do
not declare the PR complete or merge while an actionable comment lacks a
disposition. If the bounded babysit budget expires, an explicit handoff is
allowed only after reporting the unresolved comments and checks; it is not a
completion claim.

## Re-review after a substantial feedback-driven change

Decide whether the response to a comment is substantial: changes to behavior,
architecture, interfaces, requirements, security, multiple files, or a
material part of the implementation qualify. A typo, formatting-only edit,
isolated documentation clarification, or other trivial correction does not.
When uncertain whether a change is substantial, tell the human partner and ask
for direction.

When feedback causes a substantial change, trigger a fresh automated review of
the new head before declaring the feedback cycle complete. For Codex, add a
top-level PR comment containing `@codex review`:

```bash
gh pr comment <pr> --body '@codex review'
```

Record the current `headRefOid` and the trigger time. Poll the PR review list,
review-comment list, and Codex summary until a Codex review result exists whose
`commit_id` is that current head. A trigger comment or a review for an older
head is not completion. If the current-head result does not appear before the
bounded babysit budget expires, report the review as unresolved and hand off
with the observed head, trigger time, and last review state.

For the repository's custom automated review workflow, dispatch it manually
when needed and only when the PR is open, non-draft, targets `main`, uses a
head repository matching the current repository, and has relevant code under
`apps/desktop/` or `crates/orkworksd/`:

```bash
gh workflow run pr-review.yml -f pr_number=<pr>
```

The workflow skips documentation-only and fork PRs. A manual dispatch can
force a relevant-code review below the normal size threshold, but it cannot
make a documentation-only or fork PR eligible.

`gh workflow run` may return no run URL or ID. Before dispatching, snapshot the
existing IDs with `gh run list --workflow pr-review.yml --event workflow_dispatch`
and note the dispatch time. Dispatch the workflow, then list the same workflow
and event with `--json databaseId,createdAt,displayTitle,status,conclusion` until
a new run appears after that time. Confirm its workflow and logs identify the
requested PR before watching it; if more than one candidate is ambiguous, do
not watch an arbitrary run—keep the human partner informed. Once identified,
wait for that exact run to finish with `gh run watch <run-id> --exit-status`
(or repeatedly query that run's status and conclusion), then
rescan all PR comment channels because the review comment is asynchronous.
`gh pr checks` alone is not proof that this manually dispatched review ran or
finished.

For PRs targeting `main`, request or re-request GitHub's native Copilot review
through the PR's Reviewer controls. The equivalent API request is:

```bash
gh api -X POST repos/<owner>/<repo>/pulls/<pr>/requested_reviewers \
  -f 'reviewers[]=copilot-pull-request-reviewer[bot]'
```

Verify that a **new** Copilot review appears whose `commit_id` matches the
current `headRefOid`. A retained `reviewRequests` entry only proves that a
request is present; it does not prove that Copilot reviewed the new head. If a
current-head review does not appear, report the re-review as unconfirmed and
keep the human partner in the loop; do not claim that Copilot reviewed the
change or retry blindly.

Treat every resulting comment as a new item and run this skill again. Copilot
does not necessarily re-review new pushes unless the repository ruleset is
configured to review new pushes, so explicitly request the re-review after a
substantial change.

For a substantial feedback-driven change touching `apps/desktop/` or
`crates/orkworksd/`, also rerun the repository's mandatory manual review on the
new head with `/code-review low`, escalating the effort only under the review
gate's documented risk or size conditions. Automated Codex, Copilot, and
custom-workflow reviews do not replace that manual gate. Docs-only changes do
not need this code-review rerun.

## Feedback-cycle budget

The feedback loop has a separate finite budget from the wall-clock babysit
budget. By default, allow at most **three substantial review cycles per PR
lifecycle**, shared across sessions. Count one cycle when reviewer feedback
causes a substantial change and the agent requests fresh review for that new
head. Trivial typo, formatting-only, or isolated documentation clarifications
do not consume a cycle.

Before triggering fresh review, read the prior handoff or PR conversation and
record the cycle number, limit, head SHA, and review trigger (for example,
`substantial review cycle 2/3; head <sha>; Codex and Copilot requested`). Do
not reset this count merely because a new session adopts the PR. If the limit
is reached, stop the feedback loop and ask the human partner for an explicit,
finite extension before triggering another review. Record the approved number
of additional cycles; never grant an open-ended extension or silently start a
new cycle.

The cycle cap does not permit completion or merge with undispositioned
feedback. At the cap, hand off with the unresolved comments, current head,
review/CI evidence, cycle count, and the exact human decision needed.

## Review-service availability

Each substantial cycle permits at most one Codex request and one Copilot
request. A quota or rate-limit response, service cap, permission failure, or
missing current-head result is an external review blocker; it is not a reason
to start another cycle, switch accounts, spawn another coding harness, or retry
blindly. Record the reviewer, head SHA, request time, response or timeout, and
last observed state, then keep the human partner informed. The PR cannot be
called review-complete while a required current-head review is unavailable;
the human must explicitly decide whether to wait, provide a finite retry
authorization, or use an applicable alternative review path.

## Continuation contract

An open PR is not being babysat unless an active check or a real continuation
has been established. Before ending a check-in while the PR is still open:

1. If the host exposes a wake-up or scheduled-resume mechanism, schedule the
   next check before responding. Use a 20–30 minute cadence within the
   bounded budget, and carry the PR number, last verified `headRefOid`, last
   check timestamp, CI/review state, and unresolved items into the resumed
   session.
2. If no wake-up mechanism is available, continue polling only while this
   session is actively running. When the turn ends, state explicitly that
   babysitting has stopped and provide the exact handoff evidence; do not say
   “waiting,” “still watching,” or “I’ll keep checking.”
3. On every scheduled or manually resumed check, start with the complete
   inventory above. A resumed session must not assume that the old head,
   comments, reviews, or checks are still current.

Never claim background monitoring without a scheduled continuation that the
host actually accepted. A queued review request is not a continuation.

## Re-check loop and stopping conditions

After opening the PR, after each push, and after CI or review activity settles,
repeat the complete inventory. Use the bounded self-babysit budget defined by
`starting-work`, the feedback-cycle budget, and the continuation contract
above. If no wake-up mechanism is available, report the exact last-checked
commit, timestamp, checks, and unresolved items instead of implying background
monitoring.

Stop only when the PR is merged or closed, the user explicitly stops or takes
over, permissions prevent progress, or the documented babysit budget expires.
