---
name: starting-work
description: Use when starting a new piece of work in OrkWorks — picks the right branching strategy (main vs branch vs worktree), creates the working environment, and primes the per-checkout setup so parallel agents do not collide.
---

# Starting Work

## Overview

OrkWorks is built to coordinate parallel AI sessions, and the dev workflow is parallel AI sessions. This skill is the procedural counterpart to the **Branch and PR workflow** section in `AGENTS.md` — that section defines the rules, this skill walks through executing them when you sit down to start a task.

Use this skill at the start of any task that will produce changes (code or docs). Skip it for pure conversation, exploration, or read-only investigation.

## Session scope and supported handoff

One coding session owns one task from its prompt through its PR reaching a
terminal state (merged or closed) — a task is not complete just because it
opened a PR. A session may keep checking on its own PR (CI, comments,
reviews) rather than handing off immediately once the PR is open.

Bound that self-babysitting so it never runs unattended waiting on human
review: use `ScheduleWakeup` on a spaced cadence (20-30 minutes) up to a
2-hour wall-clock budget measured from when the PR opened. If the PR has not
reached a terminal state when that budget expires, stop, report the handoff
point, and let the user start a fresh session to continue.

When a fresh session explicitly adopts an already-open PR, start a new bounded
2-hour babysit budget at adoption rather than inheriting the prior session's
elapsed time. Do not reset the budget repeatedly within the same session, and
still require ownership or explicit authorization before modifying the PR
branch.

Do not launch another coding harness from an active session. If a separate
task or follow-up needs its own context — not just watching the PR this
session opened, or the check-in budget above has expired — finish or pause
the current task, report the handoff point, and stop and let the user start
a separate session from the appropriate repository root or sibling worktree.
For code changes that need parallel isolation, use the sibling-worktree path
below and let the new session start there; never launch the second harness
inside the current session. Use the repository's explicit `/code-review low`
gate in the current session when review is required.

## Preflight: establish checkout ownership

Run `git worktree list --porcelain` before selecting a checkout. The local `main` branch may be checked out only in the primary checkout; linked worktrees must be attached to an explicitly agent-owned or owner-authorized feature or fix branch.

If the primary checkout is detached, do not check out `origin/main`. If another worktree holds `main`, ask its owner to restore the owner branch or remove the worktree. Only if that worktree is clean and you are explicitly authorized may you perform the recovery; never detach it or use force operations. Stop for direction if recovery would affect an active owner or uncommitted changes.

### Detect a shared checkout

The primary checkout can look idle while another session is mid-task — agents enter it between your checks, and their edits, branch switches, and ref updates collide with yours silently (real incident, 2026-09-13: two sessions working in the primary checkout swapped each other's branch names mid-flight). `git worktree list` only shows live worktrees; it says nothing about who used this checkout. Before trusting that the primary checkout is yours alone, check for residue another session leaves behind:

```bash
git status --short                          # untracked files or edits you didn't make
git reflog --date=iso | head -20            # checkouts/commits you didn't perform
git branch -v --sort=-committerdate | head  # foreign branches, very recent activity
```

**Default to a worktree.** Unless the user has explicitly said no other agent will use this checkout, do your work in a sibling worktree on your own branch — even for docs-only or trivial fixes. A worktree is cheap (seconds to create); a mid-session branch swap, clobbered index, or interleaved uncommitted edits are not. Working on a branch in the primary checkout is the exception, not the default.

**Residue rule:** any reflog entry, branch, or untracked file not created by this session counts as residue — there is no recency or relevance exemption, and you cannot cheaply prove the session that left it is done. Residue found means worktree, no exceptions. Do not delete, clean up, or "adopt" residue; it may be another live session's uncommitted work — flag it to the user and leave it in place.

**The exception's full preconditions** (all required, checked in order):
1. The residue check above comes back clean — no residue of any kind, per the residue rule.
2. The user affirmatively confirms — in response to a question that names this checkout and this moment ("is anything else running against `~/workspace/orkworks` right now?") — that no other agent or human is working in this checkout and none is expected while the task runs. A vague "should be fine" or an unsolicited assumption does not count; ask.
3. The confirmation covers the task's whole duration. If another agent appears mid-task (new worktree, foreign branch updates, edits you didn't make), the exception expires: stop, commit or stash nothing further in the primary checkout, and move to a worktree.

Even with all three met, a mid-task entry by another session stays possible — that residual risk is why the worktree is the default and the exception should be rare.

## Decide where the work lives

Pick the lowest-overhead option that satisfies the rules in `AGENTS.md`.

| Change shape | Where to work |
| ------------ | ------------- |
| Any change (code or docs), default | Worktree on your own branch (see "Detect a shared checkout" above) |
| Docs-only or trivial code fix <~20 lines, all exception preconditions met | Branch in the primary checkout |
| Code change in `apps/desktop/` or `crates/orkworksd/`, all exception preconditions met, branch is **yours or explicitly authorized by its owner** | Branch in the primary checkout |
| Code change while the active branch in the primary checkout is **not yours and not explicitly authorized by its owner** | Worktree (do not add commits to someone else's branch) |
| Any residue of another session found (foreign branches, unfamiliar reflog entries, untracked files you didn't create) | Worktree — treat the checkout as possibly-shared; flag the residue, touch nothing |
| Parallel agents on independent tasks | One worktree per agent, always |

If the work itself is a multi-agent effort being planned or dispatched (not just this skill's one-checkout-per-agent isolation), structure it with the `orchestrating-task-graphs` skill first — it decides whether to fan out at all and how to verify and merge the results.

The triggers for a worktree are **concurrency**, **shared-checkout risk**, and **foreign-branch ownership**. If the primary checkout is on a branch you didn't create and its owner has not explicitly authorized your work, use a worktree — don't stack commits on branches you don't own. The point of the ownership rule is preventing two writers on one branch, not blocking legitimate changes: with the branch owner's explicit permission (e.g. they ask you to land review fixes on their PR branch), pushing to their branch is fine. See "Branch and PR workflow" in `AGENTS.md`.

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
# from the repository root, after resolving the concrete merged PR
bash scripts/finish-pr.sh <PR_NUMBER_OR_URL>
```

The helper verifies the PR is merged into `main`, refuses the current,
ambiguous, or dirty worktree, and performs the matching worktree removal,
prune, and local branch cleanup as one guarded operation.

Also clean up worktrees for abandoned tasks — if you decide not to pursue a task, remove the worktree immediately. Do not leave it "just in case."

Stranded worktrees follow the 7-day stranded-branch rule. If a worktree has gone >7 days without progress, either rebase and continue or remove it and close its PR with a one-line reason.

## Red flags — stop and use a worktree

- "It's just a small docs/trivial fix, the primary checkout is fine" — small fixes are exactly where the ownership check gets skipped under pressure; the fix's size says nothing about who else is in the checkout.
- "The checkout was clean when I started" — clean at session start does not mean unshared mid-session; other agents enter between your checks, which is why the exception expires on any mid-task sign of another session.
- "The reflog commits are from an old session, it's abandoned" — the residue rule has no recency exemption; you cannot cheaply prove the session that left it is done.
- "A worktree means a full pnpm install, too slow" — the install is minutes once, not per-commit; a clobbered index or swapped branch costs far more.
- "There's no evidence anyone else is running" — absence of evidence at one instant is not evidence of absence for the session's duration; that is why the default is the worktree, not more checking.
- "The user said hurry, that's permission" — urgency satisfies none of the exception's preconditions; the speed-preserving move is the seconds-cheap worktree.
- "I'll just clean up the stray files first" — residue may be another live session's uncommitted work; flag it, never delete or adopt it.

## Quick reference

```bash
# list active worktrees
git worktree list

# create a new worktree on a new branch
git worktree add ../orkworks-my-feature -b my-feature

# create a worktree on an existing branch
git worktree add ../orkworks-my-feature my-feature

# remove a finished worktree after its PR merges
bash scripts/finish-pr.sh <PR_NUMBER_OR_URL>
```
