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
[GitHub issue board](https://github.com/Rambolarsen/orkworks/issues). Prioritize
actionable GitHub Copilot harness work first, including resume, model
selection, capacity signals, native voice, session-ID capture, and
attention/integration coverage (issues #323–#327). Once none is actionable,
prioritize stabilization work: user-visible bugs, regressions, failing tests,
and correctness or data-integrity bugs. Then select net-new work from the
lowest incomplete milestone, breaking ties by user impact, especially current
usability and data correctness.

Create future work as scoped, deliverable-sized issues with checkbox acceptance
criteria. Keep issues synchronized with the codebase by closing completed work
and updating changed scope. The product specifications remain authoritative:
when an issue is not covered by a specification, request a spec update in an
issue comment rather than implementing it; when a specification has no issue,
create one before implementation.

If the issue board is inaccessible, do not guess at priorities. Stop and tell
the user that board access is required before picking or closing work.

## Assumption discipline

Before acting, make every assumption that could change scope, target,
permissions, or expected behavior explicit. Treat missing context as unknown,
not as permission to guess. Validate material assumptions against the task,
live OrkWorks recommendation or API when applicable, authoritative specs, and
scoped repository instructions.

Separate facts, inferences, and open questions in the working update or plan.
If authoritative evidence is missing or conflicts, stop and ask rather than
silently choosing an interpretation. A source session, stale metadata, or an
inferred status never authorizes resuming, reopening, or modifying that
session; follow the task's explicit scope instead.

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

- Run `gh pr view --json number,url,headRefName,baseRefName,state` for the
  current checkout's branch.
- Run `gh pr view <number> --json ...` for a bare number.
- Run `gh pr list --search "..."` when neither of the above pins it down.

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
