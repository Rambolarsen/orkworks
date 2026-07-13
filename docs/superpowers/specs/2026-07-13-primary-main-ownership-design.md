# Primary `main` Ownership Design

## Context

The primary checkout was left detached at `origin/main` while the local
`main` branch was checked out in a linked worktree. That made routine
operations ambiguous and violated the intended ownership rule: linked
worktrees isolate agent-owned work, rather than host the trunk branch.

## Decision

The local `main` branch may be checked out only in the primary checkout.
Linked worktrees may check out only explicitly agent-owned feature or fix
branches; they must never check out `main` or remain detached. The primary
checkout may still temporarily check out an agent-owned branch under the
existing workflow.

If the primary checkout is detached, agents must not use a linked worktree as
a substitute for `main`, nor check out `origin/main` as a workaround. They
must inspect `git worktree list --porcelain` and restore `main` to the primary
checkout only after no other worktree holds it. If another worktree holds
`main`, agents must ask its owner to restore the owner branch or remove that
worktree; if it is clean and the agent is explicitly authorized, they may do
so themselves. They must not detach it or use force operations. If recovery
would disrupt a non-clean worktree or an active owner, they must stop and
obtain direction.

## Documentation Changes

- Add the ownership rule and recovery requirement to the Branch and PR
  workflow in `AGENTS.md`.
- Add a preflight check and the same recovery rule to
  `skills/starting-work/SKILL.md`, where agents select a branch or worktree.

## Verification

- Confirm that any checkout of local `main` is the primary checkout.
- Confirm every linked worktree is clean and attached to its explicitly
  agent-owned feature or fix branch; none is on `main` or detached.
- Treat synchronization with `origin/main` as a task-specific, non-destructive
  check rather than an ownership prerequisite.
- Review the documentation diff for consistent wording and run the repository
  doc currency check.
