---
name: starting-work
description: Use when starting a new piece of work in OrkWorks — picks the right branching strategy (main vs branch vs worktree), creates the working environment, and primes the per-checkout setup so parallel agents do not collide.
---

# Starting Work

## Overview

OrkWorks is built to coordinate parallel AI sessions, and the dev workflow is parallel AI sessions. This skill is the procedural counterpart to the **Branch and PR workflow** section in `AGENTS.md` — that section defines the rules, this skill walks through executing them when you sit down to start a task.

Use this skill at the start of any task that will produce code changes. Skip it for pure conversation, exploration, or read-only investigation.

## Preflight: establish checkout ownership

Run `git worktree list --porcelain` before selecting a checkout. The local `main` branch may be checked out only in the primary checkout; linked worktrees must be attached to an explicitly agent-owned or owner-authorized feature or fix branch.

If the primary checkout is detached, do not check out `origin/main`. If another worktree holds `main`, ask its owner to restore the owner branch or remove the worktree. Only if that worktree is clean and you are explicitly authorized may you perform the recovery; never detach it or use force operations. Stop for direction if recovery would affect an active owner or uncommitted changes.

## Decide where the work lives

Pick the lowest-overhead option that satisfies the rules in `AGENTS.md`.

| Change shape | Where to work |
| ------------ | ------------- |
| Docs-only (`docs/`, `specs/`, ADRs, `*.md` outside `apps/`/`crates/`) or trivial code fix <~20 lines | Directly on `main` in the primary checkout |
| Code change in `apps/desktop/` or `crates/orkworksd/`, no other agent active, branch is **yours or explicitly authorized by its owner** | Branch in the primary checkout |
| Code change while the active branch in the primary checkout is **not yours and not explicitly authorized by its owner** | Worktree (do not add commits to someone else's branch) |
| Code change while another branch is already in flight in the primary checkout, or another agent is running | Worktree |
| Parallel agents on independent tasks | One worktree per agent, always |

The triggers for a worktree are **concurrency** and **foreign-branch ownership**. If the primary checkout is on a branch you didn't create and its owner has not explicitly authorized your work, use a worktree — don't stack commits on branches you don't own. The point of the ownership rule is preventing two writers on one branch, not blocking legitimate changes: with the branch owner's explicit permission (e.g. they ask you to land review fixes on their PR branch), pushing to their branch is fine. See "Branch and PR workflow" in `AGENTS.md`.

## Path and naming convention

- Branches: short kebab-case, scoped to the unit of work. Examples: `app-settings-hotkeys`, `peon-status-lifecycle`, `taskmaster-spec`.
- Worktrees: sibling directory next to the primary checkout, named `../orkworks-<branch-slug>`.
  - Keeps `ls` legible.
  - Prevents Vite, Electron, and `pnpm` from following symlinks into nested worktrees.
  - Cleanup tooling and agent prompts can assume this path.

## Create a branch (no worktree)

```bash
git switch -c <branch-slug>
```

That is it. The primary checkout's `node_modules` and Cargo `target/` are reused.

## Create a worktree

```bash
git worktree add ../orkworks-<branch-slug> -b <branch-slug>
cd ../orkworks-<branch-slug>
cd apps/desktop && pnpm install
```

Notes:

- `pnpm install` is per-worktree — `node_modules` is not shared across worktrees.
- Cargo manages its own `target/` per worktree automatically; no extra step.
- If the branch already exists and its owner has explicitly authorized your work (for example, review fixes), drop `-b <branch-slug>` and use `git worktree add ../orkworks-<branch-slug> <branch-slug>`.
- Run any agent (Claude Code, Codex, OpenCode, Aider) from inside the worktree directory, not the primary checkout. Treat the worktree as the project root for that task.

## While the work is in flight

- Commit frequently inside the worktree or branch; rebase onto `main` rather than merging `main` in.
- Do not edit the same files from the primary checkout and a worktree at the same time — git will let you, the build tools will not.
- If you start a second concurrent task, open a second worktree. Do not branch-switch inside an existing worktree mid-task.

## Wrapping up

**Always clean up your worktrees when done.** Leaving stale worktrees behind wastes disk space and creates confusion for future sessions.

When the branch merges (squash-merge by default per `AGENTS.md`):

```bash
# from anywhere
git worktree remove ../orkworks-<branch-slug>
git worktree prune
git branch -d <branch-slug>          # local cleanup
```

Also clean up worktrees for abandoned tasks — if you decide not to pursue a task, remove the worktree immediately. Do not leave it "just in case."

Stranded worktrees follow the 7-day stranded-branch rule. If a worktree has gone >7 days without progress, either rebase and continue or remove it and close its PR with a one-line reason.

## Quick reference

```bash
# list active worktrees
git worktree list

# create a new worktree on a new branch
git worktree add ../orkworks-my-feature -b my-feature

# create a worktree on an existing branch
git worktree add ../orkworks-my-feature my-feature

# remove a finished worktree
git worktree remove ../orkworks-my-feature
git worktree prune
```
