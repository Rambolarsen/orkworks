---
name: grooming-the-board
description: Use when asked to groom or tidy the issue board, on a scheduled housekeeping run, or when the board and the codebase have visibly drifted (duplicate issues, done-but-open issues, stranded branches, stale ADR index).
---

# Grooming the Board

## Overview

**Core question: where do the board, the code, and the specs disagree?**

`AGENTS.md` mandates keeping issues in sync with the codebase, closing them when done, and progressing or closing branches older than 7 days — but nothing runs those rules. This audit walks the three sources of truth (issue board, code, specs/ADRs), finds disagreements, and repairs the cheap ones directly. Unlike the other audit skills, its primary output is *actions on the board* (close, comment, file), not new implementation issues.

**Evidence before action.** Every close or comment cites the commit, file, or duplicate issue that justifies it. When the evidence is ambiguous, comment and leave open — never close on a hunch.

## Checks

Run each; skip none silently.

1. **Duplicates** — same title/body or same acceptance criteria across open issues. Action: comment linking the survivor, close the newer one as duplicate.
2. **Done but open** — open issues whose checkbox acceptance criteria are demonstrably met by merged code. Verify each checkbox against the code (not the PR title). Action: comment with per-checkbox evidence (`file:line`, commit); close only when *all* boxes are proven; otherwise comment listing what remains.
3. **Open but out of scope** — issues describing work not covered by the specs, or barred by non-goals (`specs/orkworks-mvp.md` "Explicit MVP Non-Goals", `AGENTS.md` constraints). Action: comment noting the gap per the `AGENTS.md` rule; do not close — the owner decides.
4. **Spec work with no issue** — spec sections describing undone work with no tracking issue. Action: file one, following the issue format in `skills/surfacing-blind-spots/` Phase 4.
5. **Stranded branches and worktrees** — `git fetch --prune origin` first (explicitly `origin`: the audit reads cached remote-tracking refs, and a bare fetch only refreshes the current branch's tracked remote), then run `bash scripts/stranded-branch-audit.sh --state all`. It compares against the fetched `origin/<default>` tree and flags remote branches >7 days old whose merge result differs from it (the tip contributes content or conflicts; already-content-merged tips, e.g. squash-merge leftovers, are skipped) and which no OPEN same-repo PR covers. Classify each flagged branch by evidence before acting, comparing against the fetched `origin/main`, not a possibly-stale local `main`:
   - **Deletion-safe** — its content demonstrably already exists on `origin/main` (verify, e.g. byte-diff the file against the copy on `origin/main`). A MERGED PR record alone is not sufficient: the branch may have post-merge commits, or `main` may have evolved past the merged lines so both the audit and the byte-diff flag it — verify the tip directly (e.g. diff against the merged PR's merge commit, or byte-diff the files) before trusting that the PR covered it. Action: comment on the branch's PR, or bundle them into a single housekeeping-deletion issue naming the 7-day rule (precedent: #210).
   - **Real, unmerged content** — commits distinct from `origin/main` with no OPEN PR covering them: no PR ever opened, a PR closed unmerged, or newer commits beyond an already-merged PR. This is neither generic cleanup (the #210 shape) nor a bug/feature issue: it needs an owner decision to land, extract (e.g. recover a design doc into a PR), or abandon. Before filing, verify whether the content landed or was superseded through other PRs/ADRs, and check a closed PR's closing comment for an already-recorded close-with-a-reason disposition; file one owner-decision issue listing the branches (precedent: #211) only when no such decision exists, recording that evidence per branch. Deletion is the owner's call, never the audit's.
   - **Stalled open PR** — the audit reports a branch whose only open PR has been inactive past the 7-day window (`#n open, stalled Nd`) instead of treating the OPEN record as coverage; that PR is itself subject to the rebase-or-close rule. Action: comment on the PR naming the rule; the owner decides.
   - In every case comment on the branch's PR or file the appropriate issue naming the rule; do not delete branches.
6. **ADR and doc drift** — `docs/adr/README.md` index missing ADR files or listing wrong statuses; superseded ADRs not marked; skills listed in `AGENTS.md`/`docs/agents/apm.md` that don't match `skills/`. Action: these are docs-only fixes — land them via branch + PR per the branch policy in `AGENTS.md` (docs no longer go direct to `main`). If the run has no repository write access (e.g. the scheduled CI audit runs with `contents: read`), file a single docs-drift issue listing the exact fixes instead, using the issue format in `skills/surfacing-blind-spots/` Phase 4.

## Output

End with a summary: actions taken (closed/commented/filed/fixed, with links), disagreements found but left for the owner (with reasons), and checks that came back clean. A clean check is a result — report it.

## Red flags — stop and reconsider

- Closing an issue where any checkbox is unverified — "the feature basically works" is not evidence
- Closing an out-of-scope issue instead of commenting — scope calls belong to the owner
- Filing a "clean up the board" meta-issue — this skill *is* the cleanup; do the work
- Deleting anything (branches, comments, files outside docs-drift fixes)

## Common mistakes

| Mistake | Fix |
| ------- | --- |
| Checking PR titles instead of code | Squash-merges hide partial work; verify acceptance criteria against `main` |
| Treating an old issue as stale by age alone | Age is a prompt to check, not evidence; unstarted valid work stays open |
| Fixing code bugs found along the way | File them (or hand to `surfacing-blind-spots` conventions); grooming stays docs/board-only |
