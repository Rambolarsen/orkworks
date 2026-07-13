# Primary `main` Ownership Design

## Context

The primary checkout was left detached at `origin/main` while the local
`main` branch was checked out in a linked worktree. That made routine
operations ambiguous and violated the intended ownership rule: linked
worktrees isolate agent-owned work, rather than host the trunk branch.

## Decision

The primary checkout exclusively owns the local `main` branch. Linked
worktrees may check out only explicitly agent-owned feature or fix branches;
they must never check out `main`.

If the primary checkout is detached, agents must not use a linked worktree as
a substitute for `main`, nor check out `origin/main` as a workaround. They
must first restore `main` to the primary checkout. If doing so would disrupt a
non-clean worktree or an active owner, they must stop and obtain direction.

## Documentation Changes

- Add the ownership rule and recovery requirement to the Branch and PR
  workflow in `AGENTS.md`.
- Add a preflight check and the same recovery rule to
  `skills/starting-work/SKILL.md`, where agents select a branch or worktree.

## Verification

- Confirm the primary checkout is on `main` and aligned with `origin/main`.
- Confirm the former linked checkout is detached and clean.
- Review the documentation diff for consistent wording and run the repository
  doc currency check.
