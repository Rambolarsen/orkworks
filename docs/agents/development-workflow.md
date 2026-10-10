---
type: Process Guide
title: Development workflow reference
description: Detailed issue prioritization, assumption discipline, decision tracking, and maintenance checks.
tags: [orkworks, workflow, agents, documentation]
status: stable
---

# Development workflow reference

The repository root guide remains authoritative for universal obligations.
This concept holds the detailed procedures behind those rules. Load the section
for the current phase; the root guide governs preparation size and approval reuse.

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

## Branch and PR workflow

`main` is the trunk, not the workspace. All changes — code and docs alike — land through branches and PRs.

**Resolving a PR reference:** Never guess or assume the current branch's PR when the user names one without full detail. Resolve it to a concrete PR before acting; see [Resolving a PR reference](development-workflow.md#resolving-a-pr-reference).

**`main` checkout ownership:** The local `main` branch may be checked out only in the primary checkout. Linked worktrees must be attached to an explicitly agent-owned or owner-authorized feature or fix branch; they must never check out `main` or remain detached. The primary checkout may temporarily use an agent-owned or owner-authorized branch under the rules below.

When starting any task that will produce changes (code or docs), invoke the `starting-work` skill (in `skills/starting-work/`) before editing. It walks through the branch-vs-worktree decision, naming convention, and per-checkout setup that operationalize the rules in this section.

**Default to a worktree.** Agents may enter the primary checkout between your checks — their edits, branch switches, and ref updates collide with yours silently (real incident, 2026-09-13: two sessions in the primary checkout swapped each other's branch names mid-flight). Unless the user explicitly confirms the checkout is unshared for the task's duration, do your work in a sibling worktree on your own branch, even for docs-only or trivial fixes. A branch in the primary checkout is the exception: it requires a clean residue check and an explicit user confirmation, and the exception expires the moment another session shows any sign of activity. The `starting-work` skill defines the residue check, the exception's preconditions, and the cleanup commands.

Each coding session owns one task through its PR reaching a terminal state (merged or closed), not just through opening it — a session may keep checking on its own PR (CI, comments, reviews) within a bounded check-in budget instead of handing off immediately. Do not launch another coding harness; hand off work beyond that budget, or any follow-up unrelated to the PR being watched, by stopping and having the user start a separate session from the appropriate repository root or sibling worktree. The detailed session-scope, self-babysit budget, and supported-handoff rule lives in `skills/starting-work/`.

**Don't stack commits on branches you don't own.** This rule exists to prevent two writers on one branch: an agent silently adding commits to a branch another agent or person is actively working on causes lost work, confusing history, and clobbered checkouts. It is not a ban on landing legitimate changes — if the branch owner explicitly asks you to push to their branch (e.g. applying review fixes to their PR), do so. Absent that permission: if the primary checkout is on a branch someone else created, do not add commits to it — open a worktree on your own branch instead. If you find yourself on a foreign branch in a worktree, stop and create a new one.

**Every change requires a branch + PR.** This includes docs-only changes (`docs/`, `specs/`, ADRs, `README.md`, `AGENTS.md`, `CLAUDE.md`, and other `*.md` outside `apps/`/`crates/`) and trivial code fixes under ~20 lines (typos, comment edits, single-line config tweaks). There are no direct-to-`main` pushes. Branch protection requires one approving review plus the required status checks for all PRs; GitHub blocks self-approval, and this repo has a single maintainer, so `enforce_admins` is disabled and the maintainer lands PRs via explicit admin override (`gh pr merge --admin`) — a deliberate, per-merge act, not a general exemption (policy decision, 2026-08-30). Required status checks still gate non-admin actors, and `main-ci.yml` re-validates `main` itself after every merge. The `/code-review` gate below still applies only to PRs touching code, not to docs-only PRs.

**Single-maintainer review and merge handoff:** GitHub rejects approval from the PR author; do not spend time retrying `gh pr review --approve` from the author account. If you are not the repository maintainer, request an approving review and stop at that external gate. If you are the maintainer of this single-maintainer repository, wait for every required status check to pass and complete the applicable `/code-review low` gate, then use the explicit, per-PR admin path:

```bash
gh pr merge <PR_NUMBER> --squash --admin
```

The admin override is the documented recovery for the impossible self-approval case; it is not permission to merge failing or unreviewed work, and it does not make OrkWorks or an agent the approver. The contract is checked by `scripts/branch-protection-policy-check.sh` in PR CI.

**One PR per logical unit of work.** A burst of 5–10 small commits in a few minutes that share a feature name is one PR, not ten commits on main. Squash or rebase locally before opening it.

**Review gate:** PRs that touch code under `apps/desktop/` or `crates/orkworksd/` must receive a completed `/code-review <effort>` before merge. The review may run in the current session or a separate remote Codex session. Always use an explicit effort argument — **`/code-review low` is the default and the expected case**: a diff-scoped pass over the changed lines only, no repo-wide exploration, reporting only findings that would change the diff. Escalate to medium effort or higher only for bigger or riskier changes: cross-cutting architecture/runtime work, concurrency or lifecycle changes, protocol/schema/migration changes, security-sensitive work, or unusually large diffs (roughly more than 8 code files or 500 lines); if a PR grows into that territory, prefer splitting it over escalating the review. For a remote-session review to satisfy the gate, record the reviewed code head SHA, effort, and findings disposition in the PR description or a linked review artifact. The review must cover the current code diff. A later code change requires a fresh review; docs-only commits do not invalidate the review when the code diff is unchanged. Address findings or note why each is intentional in the PR description. `pr-review.yml` (see "CI routing" above) posts an automated first-pass comment on the same escalation-sized PRs; treat it as a convenience heads-up, not a substitute for `/code-review`. GitHub's native Copilot code review (also configured under "CI routing") may post a second, independent review comment on PRs into `main`; it is likewise a heads-up, not a substitute for `/code-review`.

**Squash-merge by default.** Preserve multiple commits only when the history tells a story worth keeping (e.g. a refactor followed by a focused fix on top).

**Stranded branches:** branches that go >7 days without merging must either be rebased and progressed, or closed with a one-line reason in the PR. No long-lived dev branches. The same rule applies to stranded worktrees.

**Recovering `main`:** If the primary checkout is detached, do not check out `origin/main` or use a linked worktree as a substitute. Inspect `git worktree list --porcelain`. If another worktree holds `main`, ask its owner to restore its owner branch or remove it. Only when that specific worktree is clean and you are explicitly authorized may you perform that recovery yourself. Never detach the worktree or use force operations. If an active owner or uncommitted changes would be affected, stop and obtain direction.

**Parallel work:** when more than one branch is in flight at once (multiple agents running concurrently, a hotfix on top of an in-progress feature), use `git worktree` so each branch has its own filesystem checkout — branch-switching in the main checkout will collide with other agents' uncommitted edits and build output. Also use a worktree whenever the active branch in the primary checkout is one you did not create, even if no other agent is running. See "Default to a worktree" above for when a worktree is the default rather than the exception. Invoke the `starting-work` skill before opening a worktree for the path convention, per-worktree setup, and cleanup steps.

**Clean up your worktrees when done.** Remove the worktree and prune it as soon as the branch merges (or the task is abandoned). Leaving stale worktrees behind wastes disk space and confuses subsequent `git worktree list` output. The `starting-work` skill includes the exact cleanup commands.

For a merged PR with a local branch/worktree, use the guarded repository
helper with the concrete PR number or URL:

```bash
bash scripts/finish-pr.sh <PR_NUMBER_OR_URL>
```

It verifies that the PR is merged into `main`, refuses current, ambiguous, or
dirty worktrees, and then removes the clean matching worktree and local branch.

Because parallel agents each see only their own worktree, none of them individually notices the fleet-wide sprawl this creates — see the [worktree currency check](#worktree-currency-check) below, which runs at the end of every session and reports on all worktrees, not just the current one.

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
[Workflow-improvement trials from Taskmaster](peon-model-detection-troubleshooting.md#workflow-improvement-trials-from-taskmaster)
section of the troubleshooting guide.

A PR reference is a required assumption. When a task names or implies a pull
request ("the PR", "continue the PR", a bare number), state which concrete PR
you are acting on and resolve it before acting — never assume the current
branch's PR. The resolution procedure lives in
[Resolving a PR reference](#resolving-a-pr-reference) below.

## Planning uncertainty and blind-spot checkpoint

For routine work, inspect the affected flow and resolve material uncertainty
directly. No separate blind-spot exercise, scoring table, or planning artifact
is required. For a feature, discuss the unresolved choices before coding.

For architecture, protocol/schema migration, security-sensitive, or explicitly
gated work, use `surfacing-blind-spots` after relevant context review and before
decomposition: what are you least confident about, and what might the project
be missing? Investigate outcome-changing uncertainties. Record the resulting
evidence, recovery/compatibility constraints, and remaining risks in the same
design or plan. Do not create a separate assessment document. An unresolved
product spec gap still follows the issue/spec workflow.

## Plan complexity and quality

The root guide's preparation tiers govern all plan skills, including external
`writing-plans`. Multiple steps, files, or tests do not by themselves require a
written plan. A clear user request or existing approval authorizes work within
that scope; ask again only for a material scope/behavior decision, missing
authority, or a specific product approval gate. A new or materially changed
architecture, migration, or security design needs user approval before
dependent implementation; reuse approval of an existing concrete design rather
than requesting separate chat and written-spec approvals.

For features with meaningful choices, a short in-chat plan normally suffices.
Save a written plan when it supports coordination, handoff, high-risk work, or
an explicit user request. Reuse approved designs and existing acceptance
criteria instead of restarting discovery. Investigate uncertainty before
choosing a heavier path; downgrade preparation when the evidence warrants it.

Written plans should contain:

- User outcome, scope boundaries, and links to the issue and applicable specs.
- Decisions and independently meaningful deliverables, with affected modules
  or interfaces where useful. Group coupled work around its acceptance check.
- Acceptance criteria and the smallest meaningful checks with expected results.
- Material risks, dependencies, compatibility, and recovery requirements.

Use concise OKF metadata when saving a plan. Link requirements and constraints
instead of copying them. Include a small code example only when needed to
disambiguate a contract; full implementation/test code, repeated snippets,
minute-by-minute TDD steps, and file skeletons are not required. Implementation
and test discovery happen in the codebase. Preserve exact safety and
compatibility constraints wherever their omission could change behavior.

Use [reviewing-plans](https://github.com/Rambolarsen/orkworks/blob/main/skills/reviewing-plans/SKILL.md) once on the written
plan before execution or handoff: scope, simplicity, clarity, requirement
coverage, and verification. Perform the check inline in the same artifact.
For high-risk work, also assess dependencies, blast radius, state changes,
reversibility, and uncertainty with evidence; keep unknowns explicit. The
rubric lives in the skill. Scores do not grant approval. Fix material findings
before dependent implementation. Review a revision only for changed scope,
risks, interfaces, or acceptance criteria; unchanged evidence remains valid.

An execution handoff is needed when the work actually changes owners or
sessions. Continue inline by default when the current session owns the work;
use `executing-plans` for a saved plan and `subagent-driven-development` only
when delegation is requested or justified under `orchestrating-task-graphs`.
Do not pause just to offer an execution-mode menu for already-authorized work.

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

The [runtime contract list](runtime-contracts.md#architecture-constraints) is
curated, not exhaustive. Retain only accepted constraints without independent
prose coverage, and remove superseded bullets or collapse them to the canonical
reference when that reference gains the explanation. New ADRs belong in the
ADR index and relevant concept, without a mandatory root-guide bullet.
Specifications define what to build; ADRs record why.

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
   ADR constraint in `runtime-contracts.md`, and update the relevant concept document
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

Review every flagged surface against the diff. Update documentation when its
claims change, or record why the path-based warning needs no edit. A warning
does not require a cosmetic edit merely to mark a file as touched. The committed check is
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

Keep each canonical document current when its claims change. Update the root
`AGENTS.md` for universal rules or routing changes, `README.md` for onboarding
or project-overview changes, and the relevant concepts for dependencies,
architecture, integrations, and agent tooling. New ADRs and dependency bumps do
not automatically require root-guide additions. Treat stale claims as bugs.
Keep [`domain-entities.md`](domain-entities.md) current when `SessionMetadata`,
status/lifecycle vocabulary, or session/API terminology changes.

## Related context

- [Product boundaries](product-boundaries.md) defines product non-goals and UX
  constraints that workflow changes must not expand.
- [Project context](project-context.md) covers CI routing and environments.
- [APM and agent plugins](apm.md) covers generated agent configuration.
