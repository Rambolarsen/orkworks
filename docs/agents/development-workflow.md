---
type: Process Guide
title: Development workflow reference
description: Detailed issue prioritization, assumption discipline, decision tracking, and maintenance checks.
tags: [orkworks, workflow, agents, documentation]
status: stable
---

# Development workflow reference

The repository root guide remains authoritative for universal obligations.
This concept provides the detailed rationale and maintenance reference behind
those rules.

## Issue prioritization and scope

All implementation work is tracked in the
[GitHub issue board](https://github.com/Rambolarsen/orkworks/issues). The current
user-directed priority is the agent hierarchy, orchestrator preparation and
configuration-learning initiative: [#738](https://github.com/Rambolarsen/orkworks/issues/738),
#740–#746 and their tracked prerequisites, including #610 scope alignment.
Directly related bugs and stabilization are part of this focus. Pick related
bugs/stabilization and ready prerequisites before dependent feature work, then
break ties by user impact. A related issue identifies the affected contract or
component and links to this initiative; a broad Taskmaster label alone is not
sufficient. This priority does not authorize blocked runtime work: spec review,
capability evidence, dependencies and implementation approval remain binding.

When no work in that focus is actionable, prioritize GitHub Copilot harness
work, including resume, model selection, capacity signals, native voice,
session-ID capture and attention/integration coverage. Once none is actionable,
prioritize stabilization: user-visible bugs, regressions, failing tests, and
correctness or data-integrity bugs. Then select net-new work from the lowest
incomplete milestone, breaking ties by user impact, especially current usability
and data correctness.

Create future work as scoped, deliverable-sized issues with checkbox acceptance
criteria. Keep issues synchronized with the codebase by closing completed work
and updating changed scope. The product specifications remain authoritative:
when an issue is not covered by a specification, request a spec update in an
issue comment rather than implementing it; when a specification has no issue,
create one before implementation.

If the issue board is inaccessible, do not guess at priorities. Stop and tell
the user that board access is required before picking or closing work.

## Taskmaster recommendation tie-off

Recommendations go stale when the work they propose lands through a PR that
never ties back to the recommendation — agents later pick the stale
recommendation and rediscover work that is already merged. When your change
implements an existing Taskmaster recommendation (one whose
`proposedImprovement` your diff addresses, whether or not the task started
from it), complete that recommendation through the sidecar API before the PR
reaches a terminal state — not after the merge.

The completion endpoint is session-bound: it accepts a recommendation only
when `targetSessionId` equals the session derived from
`ORKWORKS_REPORT_TOKEN` and the recommendation is `Accepted`. For work that
did not start from the recommendation, the sequence is:

1. Find the matching active record: `GET /taskmaster/recommendations` (propose
   only the one whose `proposedImprovement` your verified diff demonstrably
   addresses; if no active recommendation matches, there is nothing to tie
   off).
2. Accept it from this session: `POST
   /taskmaster/recommendations/${RECOMMENDATION_ID}/accept` with
   `{"session_id":"${ORKWORKS_SESSION_ID}"}`.
3. Complete it: `POST /taskmaster/recommendations/${RECOMMENDATION_ID}/complete`
   with `Authorization: Bearer ${ORKWORKS_REPORT_TOKEN}` and
   `{"summary":"Verified disposition and checks run."}`.

```bash
curl --fail-with-body -X POST \
  "http://127.0.0.1:${ORKWORKS_PORT}/taskmaster/recommendations/${RECOMMENDATION_ID}/complete" \
  -H "Authorization: Bearer ${ORKWORKS_REPORT_TOKEN}" \
  -H "Content-Type: application/json" \
  --data '{"summary":"Verified disposition and checks run."}'
```

- Only post completion after the change is verified; the summary must state
  the verified disposition (implemented change, or noise/not-a-defect with
  evidence).
- If the recommendation carries a `completionPacket`, the accept and complete
  requests must also include a `packetMutation` with the packet's current
  `revision`, `evidenceFingerprint`, and the `approval.idempotencyKey` from
  the same `GET /taskmaster/recommendations/${RECOMMENDATION_ID}` payload —
  the sidecar rejects the request otherwise.
- Reference the recommendation ID in the PR body as well, so the linkage is
  visible in review.
- Never edit recommendation files under `~/.orkworks/` directly; the API is
  the only write path.
- When work starts *from* a recommendation, the
  [`working-on-recommendation`](https://github.com/Rambolarsen/orkworks/blob/main/skills/working-on-recommendation/SKILL.md)
  skill's own step 5 covers the same completion call; the rule here extends
  it to work that lands a recommendation's improvement from any starting
  point.

## Assumption discipline

Before acting, make every assumption that could change scope, target,
permissions, or expected behavior explicit. Investigate first, then decide:
read the authoritative sources before committing to an interpretation or an
action — never decide first and search for supporting evidence afterward.
Treat missing context as unknown, not as permission to guess. Validate
material assumptions against the task, live OrkWorks recommendation or API
when applicable, authoritative specs, and scoped repository instructions.

Separate facts, inferences, and open questions in the working update or plan.
If authoritative evidence is missing or conflicts, stop and ask rather than
silently choosing an interpretation. A source session, stale metadata, or an
inferred status never authorizes resuming, reopening, or modifying that
session; follow the task's explicit scope instead.

When a Taskmaster recommendation or rollup reports a repeated obstacle or
workflow recommendation, treat its `proposedImprovement` as unverified until
the raw grounding is reproduced from the session's own artifacts and the
described gap is verified against it — a grounded `missing_context` or
`assumption` recommendation may target repository context that does not
exist yet, so the gate is a verified gap, not a pre-existing named rule.
Evidence made only of page or plan titles, spinner frame labels, or
user-configured model instructions — with no concrete artifact matching
the described problem area — counts as over-detection noise to report
through the recommendation-completion summary, not as confirmation.
A "trial" recommendation that would document an experiment in
`AGENTS.md` is implemented only when it names the specific behavior, its
measured recurrence source, and a defined success measure; see the
[Workflow-improvement trials from Taskmaster] section of the root
`AGENTS.md` guide.

A PR reference is a required assumption. When a task names or implies a pull
request ("the PR", "continue the PR", a bare number), state which concrete PR
you are acting on and resolve it before acting — never assume the current
branch's PR. The resolution procedure lives in
[Resolving a PR reference](#resolving-a-pr-reference) below.

## Planning uncertainty and blind-spot checkpoint

After reviewing the relevant context and authoritative sources, but before
decomposing a plan into implementation tasks, answer both questions:

> **What am I least confident about right now?**

> **What's the biggest thing I'm missing about the situation right now? What
> am I not realizing?**

Use the first question to identify uninvestigated files or paths, unsupported
assumptions, unverified edge cases, dependencies taken on faith, and tests that
may not pin the behavior they claim. Use the second to challenge the user's or
project's framing: look for omitted constraints, alternative explanations,
likely failure modes, and gaps between the request, specs, ADRs, code, and issue
board.

Investigate every answer that could change scope, architecture, acceptance
criteria, or research conclusions before finalizing the plan. Record the
evidence used and distinguish resolved questions from risks that remain. Carry
real unresolved risks into the plan with a concrete validation or mitigation;
when the finding is a spec gap, stop implementation planning and route it
through the issue/spec workflow. Do not turn an uninvestigated doubt into a
finding or add a separate issue for a concern already fixed or tracked.

## Plan complexity and quality

Use the repo-owned
[`reviewing-plans`](https://github.com/Rambolarsen/orkworks/blob/main/skills/reviewing-plans/SKILL.md)
skill before drafting or substantially revising an implementation plan and
before executing or handing off any new, revised, or received plan. The root
`AGENTS.md` makes this required alongside external `writing-plans`; apply the
local scope and detail constraints when using an external template.

After the uncertainty checkpoint above, assess dependencies, blast radius,
state changes, reversibility, and uncertainty with concrete evidence. Keep
unknowns explicit. For received plans, establish or validate the requirements,
evidence, and initial complexity ratings. Review the actual draft for scope,
simplicity, clarity, and verification. Map each mandatory requirement to a
delivery step and verification check; resolve uncovered requirements before
marking the plan Ready. Fix findings before dependent implementation. Preserve
approved requirements when proposing smaller delivery steps. Scores neither
grant approval nor impose automatic architecture-review thresholds. Keep the
assessment in the plan; bounded changes do not need a written plan just to
run the check. The rubric and rating anchors live in the skill.

## Pull request babysitting

Opening a pull request starts its review lifecycle; it is not the end of the
coding session. After creating or adopting an open PR, use the repo-owned
[`babysitting-pull-requests`](https://github.com/Rambolarsen/orkworks/blob/main/skills/babysitting-pull-requests/SKILL.md) skill.
It requires checking PR conversation comments, inline review comments and
threads, review summaries, and status checks. Vet every comment against the
codebase and requirements, fix it or push back with evidence, and keep the
human partner informed when uncertain. After a substantial feedback-driven
change, trigger a fresh automated review before completing the cycle.

When `finishing-a-development-branch` chooses the push-and-create-PR path,
that choice hands the open PR to this babysitting workflow; creating the PR or
passing its initial checks is not a terminal state.

The PR is not ready for completion or merge while an actionable comment lacks
a disposition. If the bounded babysit budget expires, an explicit unresolved-
work handoff is allowed only after reporting the outstanding comments and
checks; `gh pr view` alone is not a complete inline-comment inventory.

## Architecture decision records

Architecture decisions are recorded in `docs/adr/`, using the
`docs/adr/template.md` template and listed in `docs/adr/README.md`.
Create an ADR before implementation for a decision that shapes architecture,
stack, or protocol. If that decision becomes clear during implementation,
pause, record it, then continue.

When a decision changes, write a new ADR and supersede the earlier one rather
than deleting history. If implementation has diverged from an ADR, write the
replacement ADR first, then mark the earlier ADR `superseded` with a reference
to its replacement. Add every ADR to the index.

The root guide's Architecture bullets are curated, not exhaustive. An inline
ADR remains only while accepted, constrains implementation, and has no
independent prose summary elsewhere. Remove its inline bullet when it is
superseded; when another document gains prose that covers it, collapse the
bullet to a pointer. Specifications define what to build; ADRs record why.

### ADR change sequence

Use this sequence when a change touches an existing ADR, especially when a
review questions whether the implementation matches an older decision:

1. Compare the implementation with both the ADR's `Decision` and
   `Consequences`. A consequence that deliberately qualifies the ordering is
   part of the decision record; do not treat a terse ordering summary as the
   whole contract.
2. Choose the history operation:
   - **Amend** the existing ADR in a dated `## Amendment` section when the
     decision still stands and the change clarifies, operationalizes, or
     records behavior already intended by its consequences. Keep the ADR's
     status and number unchanged.
   - **Supersede** only when the product decision is reversed or the intended
     implementation deliberately diverges. Create the next numbered ADR,
     link it from the old record, mark the old record `superseded`, and update
     the index.
3. Synchronize the documentation surfaces in the same change: update
   `docs/adr/README.md` for every new ADR, remove or collapse an eligible
   inline ADR bullet in `AGENTS.md`, and update the relevant concept document
   when it carries the detailed prose.
4. Verify the sequence before handoff with `git diff --check` and
   `bash scripts/doc-check.sh`. A review disagreement alone is not evidence
   that an ADR should be superseded; resolve it against the complete record.

## Resolving a PR reference

When the user refers to a PR without giving its full identifying detail (e.g.
"the PR", "continue the PR", "PR #500" with no repo/URL), do not guess,
assume the current branch's PR, or proceed without one. Resolve it to a
concrete PR first:

```bash
bash scripts/resolve-pr.sh [--search SEARCH_PHRASE] [PR_REFERENCE]
```

The helper accepts no argument (resolves the current checkout's branch PR),
a bare number or `#number`, a full PR URL, or a branch name as a direct
reference, or a phrase through explicit `--search`; it prints the PR's number,
URL, head/base branches, and state. References never fall back from one
operand kind to another (a failed number, URL, or branch lookup stops and
asks), and search matches run across all PR states; when nothing unambiguous
matches or `gh` itself fails, the helper exits nonzero with an ask-the-user
message instead of guessing.

If you need to do it manually, or the helper is unavailable:

- Run `gh pr view --json number,url,headRefName,baseRefName,state` for the
  current checkout's branch.
- Run `gh pr view <number> --json ...` for a bare number.
- Run `gh pr list --state all --search "..."` when neither of the above pins
  it down.

If the checkout, branch, and any number given still don't converge on exactly
one PR, stop and ask the user for the PR number or URL rather than acting on
an assumed target.

After a concrete PR merges, run the repository cleanup helper from the
repository root:

```bash
bash scripts/finish-pr.sh <PR_NUMBER_OR_URL>
```

It rechecks the PR's merged-into-`main` state, finds the matching local branch
and worktree, and refuses current, ambiguous, or dirty worktrees before
removing the clean worktree and local branch.

## Documentation and worktree maintenance

Before ending a session, run:

```bash
bash scripts/doc-check.sh
```

Address every flagged file before closing. The committed check is
harness-neutral; Claude Code runs it from a Stop hook, other harnesses run it
as part of completion verification, and `pr-ci.yml` runs the same checks as a
non-blocking `doc-drift` backstop after a pull request opens. That CI signal
does not replace the local check.

Also run:

```bash
bash .claude/hooks/worktree-check.sh
```

This examines every worktree and branch, flagging merged branches and branches
stale for more than seven days without an open pull request. It exists because
parallel sessions cannot individually see the full worktree fleet. Only act on
branches you own; report another owner's flags rather than changing their
worktree.

Keep `AGENTS.md` and `README.md` current when runtime dependencies, planned
architecture directories, `apm.yml` agent targets, documented conventions or
workflows, or ADRs change. Treat stale documentation as a bug. Keep
[`domain-entities.md`](domain-entities.md) current when `SessionMetadata`
fields, status or lifecycle vocabulary, or terminology boundaries change in
`crates/orkworksd/src/metadata.rs` or related session/API mapping code.

## Related context

- [Product boundaries](product-boundaries.md) defines product non-goals and UX
  constraints that workflow changes must not expand.
- [Project context](project-context.md) covers CI routing and environments.
- [APM and agent plugins](apm.md) covers generated agent configuration.
