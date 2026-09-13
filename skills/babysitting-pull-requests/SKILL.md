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
gh pr view <pr> --json state,isDraft,mergeable,mergeStateStatus,reviewDecision,headRefOid,baseRefOid
gh api --paginate repos/<owner>/<repo>/issues/<pr>/comments
gh api --paginate repos/<owner>/<repo>/pulls/<pr>/comments
gh api --paginate repos/<owner>/<repo>/pulls/<pr>/reviews
gh pr checks <pr>
```

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
6. If the tree changes, verify the fix, push it, and refresh CI and comments.

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

For the repository's custom automated review workflow, dispatch it manually
when needed:

```bash
gh workflow run pr-review.yml -f pr_number=<pr>
```

For PRs targeting `main`, request or re-request GitHub's native Copilot review
through the PR's Reviewer controls. The equivalent API request is:

```bash
gh api -X POST repos/<owner>/<repo>/pulls/<pr>/requested_reviewers \
  -f 'reviewers[]=copilot-pull-request-reviewer[bot]'
```

Treat every resulting comment as a new item and run this skill again. Copilot
does not necessarily re-review new pushes unless the repository ruleset is
configured to review new pushes, so explicitly request the re-review after a
substantial change.

## Re-check loop and stopping conditions

After opening the PR, after each push, and after CI or review activity settles,
repeat the complete inventory. Use the bounded self-babysit budget defined by
`starting-work`; use the host wake-up mechanism when available. If no wake-up
mechanism is available, report the exact last-checked commit, timestamp, checks,
and unresolved items instead of implying background monitoring.

Stop only when the PR is merged or closed, the user explicitly stops or takes
over, permissions prevent progress, or the documented babysit budget expires.
