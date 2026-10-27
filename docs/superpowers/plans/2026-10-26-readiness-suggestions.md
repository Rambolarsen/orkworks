---
type: Implementation Plan
title: Readiness assessment with grounded next-step suggestions
description: Proposed combined assessment flow reusing independent score history and workflow recommendation publication.
tags: [orkworks, taskmaster, readiness, recommendations]
status: draft
workflow_status: approved-implementation-in-progress
---

# Readiness assessment with grounded next-step suggestions

Tracking: [#1091](https://github.com/Rambolarsen/orkworks/issues/1091).
Status: owner approved the concrete design and implementation on 2026-10-27.
The owner requested the fix on 2026-10-26. Implementation is in progress.

## Outcome and decisions

Add **Assess and suggest** as the primary readiness action. One explicit request
saves a readiness assessment, then seeks at most one current, evidence-backed
workflow recommendation. Retain **Assess readiness** as a secondary scoring-only
action. Show a concrete no-proposal reason or safe stage failure when no
recommendation can be produced. A low score never obliges a fabricated fix.

Reuse the existing workflow assessment for recommendation publication and
**Fix with AI**. Readiness snapshots stay independent historical records. Do not
convert per-area model prose into recommendation records, create a second
recommendation lifecycle, or silently switch the user's Taskmaster model.

The [readiness spec](../../../specs/repository-agent-readiness.md) and
[ADR 0095](../../adr/0095-repository-agent-readiness-history.md) deliberately
exclude this combined behavior today. This proposal changes that product choice,
while retaining score storage, signed guidance and privacy boundaries.

## Reproduced evidence

The local run completed at 2026-10-26T17:05:07Z using Codex Luna medium.
Its 26/56 raw points correctly round to 46/100. It supplied twelve repository
files without the canonical architecture or domain documents. Several
low-confidence transitions requested evidence, rather than identifying a
repository defect. Separate manual analysis completed with zero proposals.

The retained [model qualification](../../validation/repository-readiness-runtime.md)
records Luna medium failing semantic acceptance and GPT-6.1-Sol low passing the
synthetic fixture criteria. Neither that qualification nor a score establishes
capabilities across arbitrary repositories.

## User-facing behavior

Before invocation, explain that the combined action may make four model calls
using the configured Taskmaster selection. Show **Scoring readiness** then
**Finding a next improvement**. Persist/display a complete or partial score
before the suggestion stage; its failure cannot erase that score.

Show the exact existing recommendation card, no-proposal reason, active-card
outcome or interrupted/failure state produced by this operation. Never present
an older unrelated workflow report as its result. An existing active Brain
recommendation allows scoring but blocks another suggestion; surface that card
and its existing explicit Fix with AI action instead.

Expose area transitions and confidence/evidence limits beside the score under
**What to improve next**, rather than burying all of them in nested details.
Evidence-seeking transitions describe investigation needs, not confirmed defects.
The later workflow assessment owns the grounded proposal decision; no heuristic
classifier converts wording into a defect or executable fix.

For snapshots using Luna medium, display a fixed warning that the profile failed
retained readiness qualification. Identify GPT-6.1-Sol low's qualification as
limited to the retained fixture. Other profiles have no retained qualification.
Derive this message from the snapshot model/effort, not the current selection;
historical scores and live settings remain intact. Changes to inference prompts
require new retained qualification before claiming the changed profile qualified.

## API, scheduling and authority

Add authenticated empty-body `POST /taskmaster/assess-and-suggest`, with
`{outcome, runId, reasonCode}` using existing readiness admission vocabulary.
Electron owns the fixed route; renderer input cannot choose a workspace, model,
command or arbitrary request body. Existing standalone routes retain behavior.

A sidecar-owned combined operation acquires the existing installation-wide
analysis lease once. Execute readiness first, then transfer the already-held
lease into the existing workflow-assessment owner. Refactor an internal seam;
never acquire recursively or release between stages. Keep all existing active
recommendation, deduplication, dismissal and explicit Fix with AI gates.

The combined ceiling is four provider calls: at most two scoring calls and two
workflow-assessment calls. Each stage retains its independent collection,
prompt, response and timeout bounds. No retries, repair loops, fallback models,
commands, terminal replay, external requests, file edits, session launches or
expanded access. Manual work does not consume background cooldown/daily allowance.

The suggestion stage uses freshly captured repository facts, observations,
settings, provider/trust and verified signed Brain guidance. Scores and generated
transitions are not proposal evidence or prompt inputs. A recommendation is a
current improvement for the same workspace, not an explanation of its exact
numeric score. Force fresh workflow evaluation rather than reusing an earlier
cached no-proposal; keep all existing deduplication/dismissal rules.

Before starting that stage, verify canonical workspace path, live instance,
effective settings, provider/trust and guidance still match combined admission,
and revalidate the saved readiness inputs. Changes interrupt continuation;
the request cannot follow the user into another workspace or selected model.
Workflow inference then retains its own complete dispatch/publication fences.

## State and recovery

Keep readiness-history format 1 and workflow-assessment format 2 unchanged,
including existing recommendation provenance and crash-consistent publication.
Add one workspace-local `readiness-suggestion-run.json`, atomically replaced,
strict format 1 and at most 8 KiB. Exact fields: `formatVersion`, `id`,
`startedAt`, `completedAt`, `state`, `phase`, `readinessRunId`, `assessmentId`, `recommendationId`,
`reasonCode`. Child IDs and unfinished completion time may be null.
State is `queued|running|succeeded|interrupted|failed`; phase is
`scoring|suggesting|finished`. IDs use existing server-generated identities.
Reasons use fixed local codes: `active_recommendation`, `no_proposal`,
`recommendation_created`, `inputs_changed`, `guidance_unavailable`,
`metadata_unavailable`, or existing sanitized stage failures.
The record contains no repository text, scores, paths, hashes, provider settings,
credentials or prompts. Unknown/corrupt/oversized records fail unavailable.

Expose an additive `suggestionRun` field in readiness status and resolve its
exact child report through existing access/current-input status projection.
Active-recommendation outcomes bind the exact `recommendationId` and use the
existing active-card projection, never a fabricated assessment ID. A created
recommendation ID must match the linked assessment report before exposure. Standalone scores do not inherit older combined results.

Persist the parent before inference and each child ID before its dispatch.
Guard parent updates with the same workspace/settings identities as the children.
Startup recovers children through existing owners, then reconciles the parent
against exact child IDs: committed child results determine completed stages;
queued/running work without a committed result becomes interrupted. Never replay
inference after restart. Parent write failure cannot expose a pending workflow
recommendation or erase a committed score. Existing workflow publication recovery
must complete before a linked recommendation can appear or accept Fix with AI.

Access narrowing still redacts affected histories, reports and recommendations
before responses. Parent audit IDs/codes/times can remain; linked status must not
expose stale/redacted source content. Workspace metadata deletion removes the
parent. Older binaries ignore this source-free added file; existing retained
readiness evidence still imposes its documented downgrade restriction.

## Evidence selection and qualification

Reproduce document starvation with a synthetic repo containing linked architecture
and domain guidance plus distracting source files. Give permitted documentation
and instruction links fair catalog/discovery priority within existing traversal,
byte, exclusion and no-symlink bounds. Do not hardcode OrkWorks paths or infer a
gap from unread files. Insufficient evidence remains unknown or partial.

Version changed collector/prompt identities so old cached results cannot masquerade
as new captures. Preserve display of old immutable history versions; currentness
can become changed/unknown without corrupting history. Rerun isolated retained
provider trials for changed prompts and record exact semantic outcomes, including
failures. Do not claim general score accuracy from fixture qualification.

## Implementation and verification

Implement inline in this owning session/worktree under the 2026-10-27 approval. No delegation
or second coding harness is required.

- [ ] Add failing combined-operation integration tests first, then extract the
  orchestration/store beside `taskmaster/readiness.rs`. Refactor
  `taskmaster/assessment.rs` for lease transfer and fresh evaluation without
  changing standalone admission. Add fixed HTTP, Electron/preload and independent
  renderer/IPC types.
- [ ] Test real scoring followed by one grounded recommendation, no proposal,
  active recommendation, cached no-proposal bypass, partial score, scoring
  failure, suggestion failure with retained score, and lease exclusion throughout.
- [ ] Test changes of workspace/instance, evidence, settings, provider/trust and
  guidance between stages: no continuation or cross-workspace publication.
  Test corruption/size bounds and every parent/child crash boundary, including
  pending workflow publication. Recovery must not invoke a provider.
  Verify access narrowing during both stages, after restart and for unopened
  workspaces, preserving the existing read/accept/Fix with AI gates.
- [ ] Reproduce linked-document selection failure before adjusting
  `assessment_context.rs`/`context.rs` or discovery. Pin final evidence inclusion,
  exclusions, unsafe links, catalog/traversal/byte limits and governing instruction
  closure. If shared collector ordering changes, test ordinary workflow behavior
  and previous history version readability/currentness.
- [ ] Update `ReadinessPanel.tsx`, `ReadinessSummary.tsx` and
  `RecommendationsPanel.tsx` for combined actions/status, exact linked outcomes,
  prominent transitions/limits and model qualification. Render-level checks cover
  all outcomes, historical models, older unrelated reports and workspace switches.
- [ ] Run changed-profile live semantic qualification in isolated synthetic
  metadata, retaining all raw inputs/outputs. Supported gaps yield a bounded
  proposal; healthy/evidence-limited inputs never invent a fix from a low score.
- [ ] After approval synchronize the accepted spec/ADR and architecture/user docs.
  Run `bash scripts/verify-repo.sh`, docs checks/build, `bash scripts/doc-check.sh`,
  `bash .claude/hooks/worktree-check.sh` and the required current-diff code review
  before merge. The current docs-only proposal is not evidence the runtime works.

## Alternatives, uncertainty and review

Converting low scores directly into cards would bypass grounding and conflate
uncertainty with defects. A joint score/proposal model schema would change the
qualified scoring protocol and couple publication to score history. A renderer
chain loses exact linkage/recovery on reload. The chosen flow reuses the existing
owners and adds one small source-free operation record.

Investigation resolved two important seams: workflow assessment currently takes
the shared lease itself, so transfer needs an internal entry point; it also reuses
matching no-proposal reports, so the combined action needs fresh evaluation.
A low score may still honestly yield no proposal. That is an outcome to explain,
not a reason to weaken grounding or promise recommendations for every run.

Review disposition: **Approved for implementation on 2026-10-27**. Scope,
simplicity, clarity and requirement coverage reviewed inline. Lease transfer,
cached outcomes, exact linkage, partial failure, privacy and recovery checks are
included. Implementation and changed-profile semantic acceptance remain unproven.

Risk assessment: dependencies **3** (existing score/assessment/IPC owners);
blast radius **3** (shared scheduler); state changes **3** (additive source-free
record); reversibility **3** (revert combined action while preserving existing
history/privacy/downgrade rules); uncertainty **3** (inspected seams, model behavior
needs trials). Total **15/25**, a sizing aid rather than approval.
