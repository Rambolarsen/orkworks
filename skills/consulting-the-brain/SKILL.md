---
name: consulting-the-brain
description: Use when asked to analyze, verify, assess, or improve OrkWorks' own agentic workflow / agent-readiness using "the brain" (Rambolarsen/brain, viewer at rambolarsen.github.io/brain), or to run a brain assessment, consult the brain, or update the brain's OrkWorks project page.
---

# Consulting the Brain

## Overview

**The brain is a repository, not just a viewer.** `https://rambolarsen.github.io/brain/` renders one artifact of `Rambolarsen/brain` — a private, git-based [OKF](https://github.com/GoogleCloudPlatform/knowledge-catalog/blob/main/okf/SPEC.md) knowledge system the owner maintains about agent-readiness. It already tracks OrkWorks specifically: `projects/orkworks.md` and dated pages under `assessments/orkworks/` and `experiments/orkworks/`. Treat the URL as a pointer to that repo, not as the whole tool — a page that only describes the graph viewer is missing the actual content.

**Core question this skill routes to:** does brain already have context on this, and what does its own playbook say to do next — not "what would a generic readiness audit look like."

## Steps

> **Quick path — scoped page maintenance:** If the owner asks for a direct edit to one specific existing brain page (e.g. correcting a fact on `projects/orkworks.md`, appending a link under `experiments/orkworks/`), make that edit, run `python3 scripts/validate-brain.py` and `python3 scripts/check-okf-declaration.py` in brain, commit and push to brain's `main`, and report back. Skip the rest of these steps — the workflow below is for readiness analysis/improvement work, not page maintenance.

1. **Get repo access.** If `Rambolarsen/brain` isn't already available in this session/workspace, clone or attach it (owner `Rambolarsen`, repo `brain`).
2. **Read what brain already knows before doing anything else.** Open `index.md`, then `projects/orkworks.md` and the newest page(s) under `assessments/orkworks/` and `experiments/orkworks/`. A fresh assessment extends or supersedes the latest one — it does not restart from zero, and it should not re-litigate a bottleneck the newest assessment already recorded as closed or partially closed.
3. **Follow the playbook exactly.** Read `playbooks/continuous-agent-readiness-improvement.md` in brain and follow it as written — it composes `assess-repository.md`, `run-experiment.md`, and `propose-playbook-improvement.md`. Don't duplicate its steps here; that playbook is the file the owner keeps current, and this skill would drift from it otherwise.
4. **Respect brain's proposal-only default.** The playbook stops at producing an implementation proposal (its step 8) and only implements when the owner explicitly authorizes that specific proposal (step 9) — authorization for one improvement doesn't carry to the next. This matches OrkWorks' own explicit-approval conventions (Taskmaster, branch/PR rules) — being in auto mode here is not standing authorization to skip it.
5. **If a change to OrkWorks is authorized, apply it under OrkWorks' own rules, not brain's.** Brain's playbook doesn't know OrkWorks' branch/PR/ADR/`/code-review` conventions — root `AGENTS.md` does. Use it for how the change actually lands (branch + PR + review for everything — direct-to-`main` is blocked by branch protection, including for admins).
6. **Record the outcome back in brain — only if an authorized change was implemented.** If step 5 ran (the owner authorized a specific OrkWorks change and it landed), follow `run-experiment.md` to write the experiment page, then run `python3 scripts/validate-brain.py` and `python3 scripts/check-okf-declaration.py` in brain before treating the change as complete. Brain is a personal knowledge repo, not a code repo — commit and push directly to its `main`, same as the ingest playbook does. If the owner stopped at the proposal-only stage or declined the proposal, do **not** write a speculative experiment page — the assessment page produced by step 3's `assess-repository.md` already records the bottleneck ranking and the proposal, and that is the only durable artifact this round produces.
7. **Report back:** which brain page(s) were created or updated (with links), and, if applicable, what OrkWorks branch/PR/issue resulted.

## Red flags — stop and reconsider

- Treating the URL as "just a graph viewer with no API" and stopping there — the actual content is the git repo behind it.
- Running a full assessment from scratch without reading `projects/orkworks.md` and the latest `assessments/orkworks/*.md` first.
- Implementing a ranked improvement without the owner explicitly authorizing that specific proposal.
- Writing and pushing a brain experiment page when no authorized OrkWorks change was actually implemented — that records an experiment that never ran and contradicts the proposal-only default.
- Applying an authorized change to OrkWorks without going through OrkWorks' own branch/PR/ADR/review rules, just because brain's playbook doesn't mention them.
- Committing a brain page without running `validate-brain.py` and `check-okf-declaration.py` first.

## Common mistakes

| Mistake | Fix |
| ------- | --- |
| Re-deriving OrkWorks' agent-readiness state from scratch | Brain already has it — read the latest assessment and extend it |
| Copying playbook steps into task planning | Read and follow the playbook file directly; it's the source of truth, this skill isn't |
| Skipping straight to code changes because a bottleneck looks obvious | Bottlenecks still route through propose → authorize → implement, per the playbook |
