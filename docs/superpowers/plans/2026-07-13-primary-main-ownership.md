# Primary `main` Ownership Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prevent linked worktrees from holding or becoming a detached substitute for the local `main` branch.

**Architecture:** Put the policy in `AGENTS.md` and the operational preflight/recovery procedure in `skills/starting-work/SKILL.md`. The rule is about local branch placement, not remote synchronization.

**Tech Stack:** Markdown, Git worktrees, repository documentation hooks.

## Global Constraints

- Local `main` may be checked out only in the primary checkout.
- Linked worktrees may use only explicitly agent-owned or owner-authorized feature/fix branches; they may not be detached or on `main`.
- Before recovering a worktree that holds `main`, verify that specific worktree is clean and obtain explicit authorization; never detach it or use force operations.
- Synchronization with `origin/main` is task-specific and non-destructive, not an ownership precondition.
- Preserve the existing explicit-owner-authorization exception.

---

### Task 1: State the branch-ownership invariant in the repository policy

**Files:**
- Modify: `AGENTS.md:147-169`
- Test: `bash .claude/hooks/doc-check.sh`

**Interfaces:**
- Consumes: `docs/superpowers/specs/2026-07-13-primary-main-ownership-design.md`
- Produces: Policy language used by `skills/starting-work/SKILL.md`.

- [ ] **Step 1: Insert the policy after the trunk statement**

Add this paragraph immediately after the trunk statement:

```markdown
**`main` checkout ownership:** The local `main` branch may be checked out only in the primary checkout. Linked worktrees must be attached to an explicitly agent-owned or owner-authorized feature or fix branch; they must never check out `main` or remain detached. The primary checkout may temporarily use an agent-owned or owner-authorized branch under the rules below.
```

- [ ] **Step 2: Insert recovery safeguards before the parallel-work rule**

Add this paragraph before **Parallel work**:

```markdown
**Recovering `main`:** If the primary checkout is detached, do not check out `origin/main` or use a linked worktree as a substitute. Inspect `git worktree list --porcelain`. If another worktree holds `main`, ask its owner to restore its owner branch or remove it. Only when that specific worktree is clean and you are explicitly authorized may you perform that recovery yourself. Never detach the worktree or use force operations. If an active owner or uncommitted changes would be affected, stop and obtain direction.
```

- [ ] **Step 3: Review policy consistency**

Run: `rg -n -C 3 "main.*checkout|Recovering.*main|owner-authorized|Parallel work" AGENTS.md`

Expected: New language preserves the existing foreign-branch ownership and parallel-work rules without requiring all linked worktrees to be clean.

### Task 2: Add preflight and recovery procedure to the starting-work skill

**Files:**
- Modify: `skills/starting-work/SKILL.md:14-57`
- Test: `rg -n -C 3 "Preflight|owner-authorized|origin/main|worktree list --porcelain" skills/starting-work/SKILL.md`

**Interfaces:**
- Consumes: The ownership language from `AGENTS.md` and the approved design.
- Produces: A procedure agents follow before choosing primary checkout or a linked worktree.

- [ ] **Step 1: Add a `## Preflight: establish checkout ownership` section before `## Decide where the work lives`**

Document these checks and decisions:

```markdown
Run `git worktree list --porcelain` before selecting a checkout. The local `main` branch may be checked out only in the primary checkout; linked worktrees must be attached to an explicitly agent-owned or owner-authorized feature or fix branch.

If the primary checkout is detached, do not check out `origin/main`. If another worktree holds `main`, ask its owner to restore the owner branch or remove the worktree. Only if that worktree is clean and you are explicitly authorized may you perform the recovery; never detach it or use force operations. Stop for direction if recovery would affect an active owner or uncommitted changes.
```

- [ ] **Step 2: Update ownership terminology in the decision table and prose**

Replace exclusive references to a branch being “yours” with “yours or explicitly authorized by its owner,” while retaining the rule against unapproved commits on someone else’s branch.

- [ ] **Step 3: Correct the existing-branch worktree example**

Replace the existing-branch note with:

```markdown
- If the branch already exists and its owner has explicitly authorized your work (for example, review fixes), drop `-b <branch-slug>` and use `git worktree add ../orkworks-<branch-slug> <branch-slug>`.
```

- [ ] **Step 4: Verify the procedural wording**

Run: `rg -n -C 3 "Preflight|owner-authorized|origin/main|worktree list --porcelain|already exists" skills/starting-work/SKILL.md`

Expected: The skill prevents checkout of `origin/main`, does not permit a detached linked worktree, and preserves explicit owner authorization.

### Task 3: Verify and commit the documentation update

**Files:**
- Modify: `AGENTS.md`, `skills/starting-work/SKILL.md`
- Test: `bash .claude/hooks/doc-check.sh`

**Interfaces:**
- Consumes: The completed policy and procedure edits from Tasks 1–2.
- Produces: A clean, documented workflow update on `main`.

- [ ] **Step 1: Inspect the final diff**

Run: `git diff --check && git diff -- AGENTS.md skills/starting-work/SKILL.md`

Expected: No whitespace errors; both documents use `agent-owned or owner-authorized` and prohibit detached linked worktrees.

- [ ] **Step 2: Run the repository doc currency check**

Run: `bash .claude/hooks/doc-check.sh`

Expected: No unaddressed documentation triggers beyond the files intentionally changed by this task.

- [ ] **Step 3: Commit the policy and procedure update**

Run: `git add AGENTS.md skills/starting-work/SKILL.md` then `git commit -m "docs: guard primary main checkout"`.

Expected: One docs-only commit containing the two operational documentation files.

## Plan Self-Review

- Spec coverage: Task 1 states invariant/recovery policy; Task 2 makes it actionable and preserves owner authorization; Task 3 verifies the final documentation and doc-currency check.
- Placeholder scan: no incomplete tasks, unspecified files, or deferred work remain.
- Consistency: every occurrence uses `agent-owned or owner-authorized`; only the recovery target requires a cleanliness check; no task treats `origin/main` synchronization as an ownership prerequisite.
