# Pull Request Babysitting Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a repo-owned skill that keeps an agent responsible for an open pull request through review comments, CI changes, and a terminal state.

**Architecture:** Keep APM-installed skills untouched. Add the lifecycle protocol as a committed skill under `skills/`, then route the post-PR transition to it from `AGENTS.md` and the repo skill catalogs. Extend the existing blind-spot skill with an explicit planning mode so its trigger and output agree with the planning checkpoint already added.

**Tech Stack:** Markdown, Agent Skills frontmatter, GitHub CLI/API documentation.

**Spec:** `AGENTS.md`, `docs/agents/development-workflow.md`, and the Agent Skills standard.

## Global Constraints

- Committed repo skills under `skills/` are the source of truth; do not edit `.agents/skills/` or other APM-generated artifacts.
- All changes remain on the existing `planning-blind-spot-checkpoint` PR branch.
- Review feedback must be technically evaluated and answered in its inline thread; suggestions are not accepted blindly.
- The PR babysitting loop must not claim “no comments” without inspecting inline review comments.

---

### Task 1: Define the repo-owned PR babysitting skill

**Files:**
- Create: `skills/babysitting-pull-requests/SKILL.md`

**Interfaces:**
- Consumes: an open PR number or URL, the current branch/commit, GitHub PR state, CI state, and all available comment channels.
- Produces: a complete comment disposition, fixes or reasoned replies, refreshed CI state, and either a terminal PR state or an explicit handoff point.

- [ ] **Step 1: Capture the RED baseline**

Record the demonstrated failure from PR #538: `gh pr view --json comments,reviews` showed the review summary but omitted the actionable inline Codex comment. The skill must therefore require the dedicated review-comments endpoint in addition to PR-level fields.

- [ ] **Step 2: Write the skill**

Document the trigger as “after opening an open PR or when checking an open PR for review/CI changes.” Require a complete inventory of PR conversation comments, inline review comments/threads, review summaries, requested changes, and status checks. Require every comment to receive a disposition: fix, reasoned pushback with an inline reply, informational acknowledgment, or already-handled evidence. Require re-scanning after pushes and CI completion. Define a substantial-change threshold for triggering fresh Codex/Copilot review after feedback; do not trigger new review for trivial edits, and ask the human partner when uncertain. Define terminal-state, user-stop, permission-blocked, and bounded-budget stopping conditions.

- [ ] **Step 3: Validate the skill structure**

Run `git diff --check` and inspect the frontmatter, trigger wording, API coverage, disposition rules, and stopping conditions manually. Confirm the skill contains no APM-generated path or instruction to edit generated files.

### Task 2: Make planning and catalog routing unambiguous

**Files:**
- Modify: `skills/surfacing-blind-spots/SKILL.md`
- Modify: `AGENTS.md`
- Modify: `README.md`
- Modify: `docs/agents/apm.md`

**Interfaces:**
- Consumes: the existing planning checkpoint and repo skill catalog entries.
- Produces: discoverable, consistent routing for planning, PR babysitting, and audit/close-out modes.

- [ ] **Step 1: Add explicit planning mode**

Add planning to the surfacing skill description and mode table. State that planning records evidence, resolved assumptions, unresolved risks/spec gaps, and mitigations in the plan; it does not file issues unless an audit or explicit issue-generation request is active.

- [ ] **Step 2: Add babysitting triggers**

Add the committed skill to the repo skill catalogs and state in `AGENTS.md` that opening a PR transitions the session into the dedicated babysitting skill. Keep `starting-work` focused on ownership and initial checkout setup.

- [ ] **Step 3: Check catalog consistency**

Search for every `surfacing-blind-spots` and `babysitting-pull-requests` catalog entry and ensure each description identifies its actual trigger and ownership.

### Task 3: Verify the workflow documentation

**Files:**
- Modify: `docs/agents/development-workflow.md`

**Interfaces:**
- Consumes: the branch/PR lifecycle rules and the new repo-owned skill.
- Produces: durable workflow guidance that places babysitting after PR creation and before completion/handoff.

- [ ] **Step 1: Document the lifecycle handoff**

Add a concise pointer that `finishing-a-development-branch` hands an opened PR to `skills/babysitting-pull-requests/`, and that an open PR is not complete merely because it was created or its initial checks passed.

- [ ] **Step 2: Run repository verification**

Run `git diff --check`, `bash scripts/doc-check.sh`, and `bash scripts/verify-repo.sh`. Confirm the unrelated `.github/workflows/pr-review.yml` edit, if present in the primary checkout, is not staged or changed by this worktree.

- [ ] **Step 3: Commit the review fix**

```bash
git add AGENTS.md README.md docs/agents/apm.md docs/agents/development-workflow.md skills/surfacing-blind-spots/SKILL.md skills/babysitting-pull-requests/SKILL.md docs/superpowers/plans/2026-09-13-pr-babysitting-skill.md
git commit -m "docs: add pull request babysitting workflow"
```

- [ ] **Step 4: Push and re-check PR #538**

```bash
git push origin planning-blind-spot-checkpoint
gh pr checks 538
```

Then inventory all PR comment channels and reply to the Codex inline thread with the implemented disposition.
