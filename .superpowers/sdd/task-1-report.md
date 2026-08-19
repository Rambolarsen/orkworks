# Task 1 report — Make the feature authoritative and tracked

## Status: DONE_WITH_CONCERNS

One concern (ADR numbering collision, see below) required a judgment call rather than following the brief literally. Everything else completed as specified.

## What I implemented

1. **`specs/orkworks-mvp.md`** — added a "Current-summary snapshot" subsection (summary/summarySource/summaryConfidence/summaryObservedAt, update/clear rules) under Peon, and a new "## Workflow observations" section (record shape, recording module interface, explicit-report route, storage/limits). Extended the Peon `may infer`/`may update`/`must not` lists and the Session Metadata Protocol / Peon MVP-scope bullet lists to reference the new directory and observation candidates.
2. **`specs/taskmaster.md`** — added a "Workflow observations" input subsection, an `improve_workflow` recommendation type, a full "## Workflow-improvement recommendations" section (eligibility, kind-to-target mapping table, recommendation shape/`workflowImprovement` object, dedupe family, dismissal watermark, presentation), an API-section note on which routes apply to this passive variant, and three new acceptance-criteria checkboxes.
3. **`docs/adr/0042-workflow-observations-replace-summary-checkpoints.md`** (new) — `Status: accepted`, `Date: 2026-08-14`. Supersedes ADR 0024 and ADR 0029, restates their surviving decisions (bounded terminal replay; stable one-shot label), and records the five decisions requested: current-summary snapshot, workflow observations as durable evidence, exact deterministic Taskmaster correlation, embedded recommendation evidence, removal of summary checkpoints.
4. **ADR 0024 / ADR 0029** — status line changed to `superseded by [ADR 0042](...)`. Bodies left untouched (historically accurate as written).
5. **`docs/adr/README.md`** — 0024 and 0029 rows updated to "superseded by 0042"; new 0042 row added.
6. **`docs/agents/architecture.md`** — removed `GET /sessions/:id/summary-log` from the Key endpoints list and its explanatory paragraph; updated the `metadata.rs` bullet to stop claiming durable NDJSON checkpoints; added a new "## Workflow observations and the current-summary snapshot (design)" section documenting the target contract (current-summary fields, `WorkflowObservation` shape, recording-module interface, storage paths/bounds, and the three routes: `POST /sessions/:id/workflow-observations`, `GET /taskmaster/recommendations`, `POST /taskmaster/recommendations/:id/dismiss`), explicitly marked as design/not-yet-implemented and pointing at issue #313.
7. **`docs/agents/domain-entities.md`** — added a "## Workflow observation feedback loop (design)" section (also marked not-yet-implemented) documenting the forthcoming `SessionMetadata` snapshot fields and the `WorkflowObservation` type, without touching the existing "Notable fields" list (which documents actual current code).
8. **`AGENTS.md`** — curated ADR bullet list: removed 0024/0029 from the "prose lives in architecture.md" pointer (now superseded), added 0042 to that same pointer (it has independent prose coverage in architecture.md, so per the section's own eligibility rule it doesn't get a dedicated inline bullet). Metadata-protocol bullets updated: `events/<id>.ndjson` description no longer claims checkpoints; added `workflow-observations/` path bullet; replaced the summary-log/"Task history" bullet with current-summary-snapshot wording and an `improve_workflow` clarification.
9. **`README.md`** — same class of updates: replaced checkpoint/Task-history claims in the Architecture and Metadata-protocol bullet lists with current-summary-snapshot + workflow-observation wording, all pointing at ADR 0042.

## ADR numbering collision (the concern)

The design doc and task brief both assume ADR 0041 is free. It is not: `docs/adr/0041-session-runtime-generation-ownership.md` was accepted on 2026-08-15 (one day after the design doc was authored) for an unrelated decision (PTY runtime generation ownership), and is already referenced by name from `README.md`. Renumbering or overwriting an already-accepted, already-referenced ADR was not a safe option.

I filed the new ADR as **0042** instead, keeping `Date: 2026-08-14` (the date the underlying design was approved) and adding an explicit numbering note in the ADR's Context section. All cross-references (specs, README, AGENTS.md, architecture.md, domain-entities.md, the ADR index) consistently point to 0042. Tasks 2–11 should reference ADR 0042, not 0041.

## Other judgment calls

- **specs/orkworks-mvp.md and specs/taskmaster.md had no prior summary-checkpoint/"Task history" language** — that content lived only in ADR 0024/0029, `docs/agents/architecture.md`, `AGENTS.md`, and `README.md`. So the "remove statements that define durable summary checkpoints" instruction for the two specs was a no-op there; I only needed to *add* the new contract to them.
- **"the three Taskmaster API routes" (Step 4)** — Taskmaster has zero implemented routes in code today (grepped `crates/orkworksd/src` — no `/taskmaster` handlers exist; it's spec-only). I interpreted this as the three routes this design actually exercises for the passive `improve_workflow` variant: `POST /sessions/:id/workflow-observations`, `GET /taskmaster/recommendations`, and `POST /taskmaster/recommendations/:id/dismiss` (accept/refresh don't apply — no accept/execute action, no manual refresh needed given the 5s correlation debounce). I did **not** add `/taskmaster/*` routes to architecture.md's "Key endpoints" list, since that list documents actually-implemented routes and none exist yet; instead they're documented in a clearly-marked "(design)" section.
- **architecture.md / domain-entities.md forward-looking content** — both files otherwise document *current* Rust code. Since none of this design is implemented yet (Tasks 2–4's job), I added clearly-labeled "(design)" sections rather than editing the "current state" prose as if the code already existed, and pointed them at issue #313 with an instruction to fold the content into the normal current-state prose once implemented. This keeps both docs honest today while still satisfying the brief's synchronization requirement.
- **Pre-existing worktree issue, fixed transiently, not committed**: `docs/.vitepress/config.mts`'s `srcExclude` doesn't list `apm_modules/` (a gitignored, `apm install`-generated directory). CI never hits this (fresh checkouts never have `apm_modules` present), but this worktree had it populated, and one third-party file (`apm_modules/leonardomso/rust-skills/CLAUDE.md`) has a malformed HTML tag that breaks the Vue/Vite Markdown compiler. I moved `apm_modules/` aside (to the scratchpad dir) for the duration of the build check, confirmed a clean build, then moved it back — no committed file was touched for this. Worth a separate follow-up issue to add `apm_modules/**` to `srcExclude` so any agent that runs `apm install` before `pnpm docs:build` locally doesn't hit this.
- **Pre-existing dirty files in the worktree**: `.claude/settings.json` and `.codex/hooks.json` were already modified (apparently from an `apm install` run during worktree setup) before I started. They are unrelated to this task, not in the brief's file list, and I left them untouched and unstaged — confirmed via `git status` before and after the commit.

## What I tested

- `pnpm docs:build` (correct command per `.github/workflows/docs.yml`; the brief's literal `pnpm --dir docs build` doesn't match any script — the actual script is `docs:build`, run with cwd `docs/`) — **exit 0**, no dead-link errors, after installing `docs/node_modules` via `pnpm install --frozen-lockfile` (not yet present in this worktree) and temporarily moving aside the unrelated `apm_modules/` blocker described above.
- `bash .claude/hooks/doc-check.sh` (both via `rtk` and directly) — **exit 0**, no drift output.
- `rtk gh issue list --state open --search 'workflow observation feedback loop in:title'` — confirmed no existing match before creating the issue.

## Files changed (all absolute paths under the worktree)

- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/specs/orkworks-mvp.md`
- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/specs/taskmaster.md`
- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/docs/adr/0042-workflow-observations-replace-summary-checkpoints.md` (new)
- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/docs/adr/0024-bounded-terminal-replay-durable-summary-checkpoints.md`
- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/docs/adr/0029-session-label-topic-vs-activity-summary.md`
- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/docs/adr/README.md`
- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/docs/agents/architecture.md`
- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/docs/agents/domain-entities.md`
- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/AGENTS.md`
- `/Users/froomiebot/workspace/orkworks-workflow-observation-feedback-loop/README.md`

## GitHub issue

Created: **https://github.com/Rambolarsen/orkworks/issues/313** — "Implement workflow observation feedback loop" (exact checklist body from the brief's Step 6 command).

## Self-review findings

- Confirmed via grep that no edited doc still advertises `summary-log` or "Task history" as current/required behavior — all remaining mentions explicitly say "removed."
- Confirmed all new/changed relative markdown links resolve (`../docs/adr/...` from `specs/`, `./architecture.md` from `docs/agents/`) and that the VitePress build's dead-link checker passed.
- Confirmed the commit contains exactly the 10 files named in Step 7 (plus the new ADR file under `docs/adr/`) and no code files.
- Verified `git status` before and after the commit to confirm the pre-existing unrelated `.claude/settings.json`/`.codex/hooks.json` changes were never staged or touched.

## Concerns for the plan owner

1. **Please confirm the ADR-0042 numbering decision.** If a different resolution is preferred (e.g., reserving 0041 differently, renaming the session-runtime-generation ADR), that would need to happen before Tasks 2–11 start citing "ADR 0042" in code/tests.
2. Consider filing a follow-up issue for the `docs/.vitepress/config.mts` `srcExclude` gap (`apm_modules/` isn't excluded, breaking local `pnpm docs:build` runs whenever `apm_modules` is populated) — out of scope for this docs-only task but a real, reproducible local-dev papercut. (Update: this gap is now tracked as issue #314; not fixed here per the fix-round-1 brief.)

## Fix round 1

A reviewer found one Critical issue in commit `8364222`: several edits stated Tasks 2–11's not-yet-built functionality as present-tense current fact, and in a few places falsely claimed still-working current functionality had already been removed. No Rust or desktop code has changed at any point in this task — none of the workflow-observation feedback loop is implemented yet.

### What changed and where

**`AGENTS.md`** (Metadata protocol section):
- `sessions/<id>.json` bullet: restored "session state" as the current-fact lead, moved the `summary`/`summarySource`/`summaryConfidence`/`summaryObservedAt` fields into an explicit `(design, not yet implemented — see issue #313)` clause.
- `events/<id>.ndjson` bullet: restored the original "append-only event log with durable, exact consecutive-deduplicated summary checkpoints and accepted provenance" wording — checkpoints are still appended today; the "no new checkpoint is appended" claim was false.
- `workflow-observations/` bullet: prefixed with `(design, not yet implemented — see issue #313)` and reworded "recorded through" to "planned to hold ... recorded through" — the directory, module, and route do not exist yet.
- Restored the `GET /sessions/:id/summary-log` bullet (verbatim, still-true today) that the prior commit had deleted.
- Taskmaster bullet: restored the original "requires explicit user approval for every action" as the current-fact lead, moved the `improve_workflow` passive-recommendation claim into a `(design, not yet implemented — see issue #313)` clause with future-tense wording ("is planned as," "would require," "could not," "could only").

**`README.md`** (Architecture bullets and Metadata protocol section — same pattern applied in four places):
- Terminal-replay/current-summary bullet: restored the original "accepted session summaries are retained as durable checkpoints (ADR 0024)" as current fact; moved the current-summary-snapshot claim into a `(design, not yet implemented — see issue #313)` clause.
- Session-label bullet: restored "decoupled from the turn-by-turn summary/checkpoint log (ADR 0029)" — the current-summary snapshot doesn't exist to decouple from yet.
- WorkflowObservation-recording bullet: prefixed with `(design, not yet implemented — see issue #313)` and switched "can record"/"correlates" to future tense ("will be able to record"/"will correlate").
- `events/<id>.ndjson` bullet (Metadata protocol section): restored the original wording, same as AGENTS.md.
- `workflow-observations/` bullet (Metadata protocol section): prefixed with `(design, not yet implemented — see issue #313)`, "bounded ... records" reworded to "planned to hold bounded ... records."
- Restored the original `GET /sessions/:id/summary-log` bullet and the original "Taskmaster proposes cross-session transitions; every v1 transition requires explicit user approval" bullet, then appended the current-summary-snapshot and `improve_workflow` claims as a trailing `(design, not yet implemented — see issue #313)` sentence on the Taskmaster bullet.

**`docs/agents/architecture.md`**:
- Restored `GET /sessions/:id/summary-log` to the "Key endpoints" list (it is still a live, registered route).
- Restored the full explanatory paragraph describing the route's response shape, `apps/desktop/src/api.ts`'s `getSummaryLog`, and `SessionDetailPanel`'s "Task history" section rendering — all still live and unmodified in this diff — then appended a `(design, not yet implemented — see issue #313)` sentence noting ADR 0042's plan to remove them, pointing at the standalone design section below.
- `metadata.rs` module bullet: restored the original sentence describing `metadata.rs` preserving exact summaries as durable NDJSON checkpoints with accepted provenance (ADR 0024) — this is what the code does today — then appended a `(design, not yet implemented — see issue #313)` sentence describing ADR 0042's planned replacement.
- Did **not** touch the standalone "## Workflow observations and the current-summary snapshot (design)" section — the reviewer confirmed it was already correctly labeled as design/not-yet-implemented, and Step 3 of the fix brief said to leave it alone.

**Untouched per the fix brief**: `specs/orkworks-mvp.md`, `specs/taskmaster.md`, all ADR files (including 0042, 0024, 0029, and `docs/adr/README.md`), and `docs/agents/domain-entities.md`'s "## Workflow observation feedback loop (design)" section — the reviewer found no issues there and the brief said not to touch them.

### Verification

- `pnpm docs:build` (run from `docs/`, matching the script name in `package.json`, same as the first round) — **exit 0**, build completed with no dead-link or Vue-compiler errors, after temporarily working around the same pre-existing, unrelated `apm_modules/` `srcExclude` gap from round 1 (moved `apm_modules/` outside the repo tree for the duration of the build, then restored it byte-for-byte afterward — confirmed via directory listing). That gap is tracked separately as issue #314 and was not touched here.
- `bash .claude/hooks/doc-check.sh` (via `rtk`) — **exit 0**, no drift output.
- `git status --porcelain` before committing showed only the six intended-or-pre-existing dirty files: `AGENTS.md`, `README.md`, `docs/agents/architecture.md`, `.superpowers/sdd/task-1-report.md` (this report), plus the pre-existing unrelated `.claude/settings.json`/`.codex/hooks.json` (left untouched and unstaged, same as round 1).

### Remaining concerns

None. The fix is surgical: every location the reviewer cited now states the current-vs-design split explicitly, ADR 0042's numbering and cross-references are untouched, and both verification commands pass clean.
