---
type: Implementation Plan
title: Brain-guided manual improvement investigation
description: Implement progressive permitted evidence collection and one shared assessment and preferred next step without expanding execution authority.
tags: [orkworks, taskmaster, brain, readiness, implementation]
status: draft
workflow_status: active
---

# Brain-guided manual improvement implementation

Outcome: **Find next improvement** returns a useful preferred agentic-development
change and the assessment explaining it from one investigation. Track
[#1694](https://github.com/Rambolarsen/orkworks/issues/1694); implement the
[approved design](../specs/2026-11-12-brain-guided-improvement-design.md) and
[ADR 0114](../../adr/0114-brain-guided-improvement-investigation.md).
The owner approved the concrete design and continuation on 2026-11-12. Execute
using TDD, with isolated ownership for backend and desktop integration and fresh
review of each integration plus the whole diff before PR handoff. No repeated
design approval is needed for decisions within that contract.

## Boundaries and contract decisions

- New purpose `brain-improvement-v1`, separate from old readiness/assessment.
  Keep six total provider calls, five evidence rounds, a reserved final call,
  eight-minute absolute deadline, current catalog and approved evidence caps.
- `taskmaster/improvement/` owns the investigation protocol, evidence owner,
  reducer, report store and runtime. Reuse existing safe file opening and
  traversal/instruction closure from `assessment_context`, transport preparation
  and trust from Taskmaster runtime, strict JSON/prose validation, and readiness
  rubric/row validation. Do not copy filesystem security into another walker.
- Intermediate strict version-1 responses contain `repositoryReads` (catalog
  file ID, byte offset, byte length), `knowledgePageIds` and `finish`. Unknown,
  duplicate, excess, arbitrary-path/command fields fail. Read requests can be
  refused individually with fixed reasons within the remaining finite budget;
  admission never grants native model tools.
- Final version-1 response has summary, fourteen existing readiness rows, up to
  five ranked opportunities, preferred opportunity ID or null, explicit no-result
  reason or null, and limitations. Opportunity IDs are run-local; each candidate
  names evidence strength, mechanism/change/benefit, target surface, qualitative
  effort/risk/reversibility, validation/rollback and supplied fact/page citations.
  Reuse current prose/command validation. The final preferred candidate, rows and
  totals are validated together against exactly the same supplied facts/rubric.
- Facts identify catalog path, exact returned byte range, captured file size,
  digest and completeness. Governing instruction closure precedes source reads.
  Catalog incompleteness and denied/partial ranges never prove absence. All reads
  count toward the 32-file/32-range/64-KiB bound; one range is at most 8 KiB.
  Repeated identical reads reuse immutable facts; changed bytes invalidate the run.
- Knowledge remains pinned verified signed data. Require new capability
  `taskmaster-improvement-v1` with reviewed entry-point/rubric relationships;
  at most sixteen pages/32 KiB. Missing dependency keeps production admission
  unavailable. No private Brain access, live fetch or substitute playbook.
- Store the new report separately as `improvement-assessment.json`, format 1,
  maximum 128 KiB. Keep cited snapshots only and identities for every input.
  Explicit new-purpose provenance joins recommendations to this owner rather
  than causing legacy `assessment.json` to validate them. Unknown provenance
  cannot acquire freshness through a legacy report.
- Reuse recommendation pending-publication ownership, dedupe/dismissal and
  exact run ID. New report + preferred recommendation publish before exposure;
  restart reconciles pending writes only, never model calls. Linked readiness
  history is optional secondary publication with its own failure state/cap.
  Define a new supported history purpose rather than inserting unsupported fields
  into existing strict codecs. Same-response rows are the authoritative assessment.
- Privacy reconciliation includes the new report, all derived prose/snapshots,
  recommendation and linked history on global/workspace access narrowing,
  including unopened workspaces. Missing workspace origin with a new report
  fails closed. Workspace deletion removes it. Currentness validates all supplied
  ranges/catalog/pages/settings/provider/trust, not only cited output.
- Keep old manual actions while replacement is disabled/unqualified. New explicit
  start/status/cancel calls use Electron-only authority; cancellation denies new
  reads/calls and late publication, retaining lease/process ownership until settled.
  Existing Fix with AI applies only to implementation candidates; investigation
  candidates get an explicitly labeled investigation handoff after a user click.

## Task 1: finite protocol and safe progressive evidence

- [x] Add `taskmaster/improvement/mod.rs`, `model.rs`, `evidence.rs` and focused
  tests; extend the existing evidence owner only with the narrow helpers needed
  to reuse its canonical catalog, safe open and instruction closure.
- [x] First prove failing cases: a needed implementation beyond the first excerpt
  can be requested without a candidate; invalid/duplicate IDs and schemas,
  denied/symlink/ignored/credential paths, UTF-8 boundaries, changed facts,
  instruction closure and each exact aggregate limit are enforced.
- [x] Implement typed step/result decoding and a finite call/evidence budget;
  finalization is reserved and already-satisfied/unknown cases remain honest.
  Validate shared scoring rows and preferred-candidate citations through the
  public module interface, not test-only bypasses.
- [x] Check: focused Rust improvement tests pass, ordinary assessment/readiness
  collector tests still pass and no production caller/admission is enabled yet.

## Task 2: runtime, report publication and privacy

- [ ] Add private runtime and store modules using captured evaluation identity
  and existing analysis lease. Register code-owned native response shapes;
  custom/Claude/Ollama output still goes through strict application decoding.
- [ ] Before wiring a route, implement new report/provenance/history codecs,
  range restoration, complete input identity and every access/deletion owner.
  Prove serialized maximum bounds and old-reader fail-closed behavior.
- [ ] Test with deterministic provider fixtures: multiple evidence rounds without
  Peon, final ratings and proposal from shared facts, early finish and all six
  calls, deadline/cancel/changed inputs, denied reads, malformed output, provider
  failure, no useful opportunity and missing signed capability. No fallback.
- [ ] Test pending publication/live reconciliation/restart, duplicate dismissal,
  settings reduction across unopened workspaces, stale list/get/accept/handoff,
  unknown versions and optional history-publication failure. No inference replay.
- [ ] Check: focused runtime/store/privacy and existing assessment/readiness
  integration suites pass with production admission still disabled.

## Task 3: explicit desktop workflow and qualification gate

- [ ] Add narrow authenticated start/status/cancel transport and independent
  main/preload/renderer contract copies; keep the electron/src import boundary.
- [ ] Present the preferred next step, brief ranked alternatives, assessment and
  limits in Recommendations. Wire one run to the primary action only when its
  supported signed capability and qualification are available. Preserve existing
  standalone readiness/history and background analysis. Show useful unavailable,
  partial, no-result, interrupted and failure states without mixing old scores.
- [ ] Verify real routing and desktop fixtures: no background inference from a
  status/history read; stale workspace responses ignored; explicit handoff labels;
  cancellation and disabled/unavailable knowledge retain a usable old flow.
- [ ] Run `bash scripts/verify-repo.sh`, scoped Rust build/test/fmt and desktop
  checks; fresh `/code-review low` of current code diff before merge. Update public
  behavior claims only for delivered source behavior, never infer installer state.

## Task 4: signed guidance and usefulness before activation

- [ ] Track the required reviewed signed entry point/export/starter with Brain's
  publication owner. This repository task does not edit Brain or publish keys.
- [ ] Preserve current production routing until the compatible signed capability
  is delivered and paired usefulness qualification passes.
- [ ] Before live trials, obtain an explicit finite call/cost budget. Use the
  design's fixed three-positive/two-negative corpus, same model/effort/knowledge
  and comparable access. Retain raw outputs, independently adjudicate practical
  next-step value and explain every valid baseline suggestion lost by the app.
  Do not launch another coding harness from this active session.
- [ ] Require two of three positives, no invented negative gaps, and no loss of a
  valid baseline preferred step without an equally useful supported alternative.
  Rejected/empty positive samples fail. No silent retries/model switches.
- [ ] Enable only after all gates; otherwise report the exact remaining dependency
  and leave the old primary flow intact. Transport tests are not usefulness proof.

## Review focus and disposition

Concrete hazards covered above: later-range evidence with changing files;
instruction/exclusion changes between rounds; unknown versus absent behavior;
partial publication/redaction in unopened workspaces; cancellation versus actual
provider process cleanup. Tests must cross the same investigation interface as
callers and retain existing provider/approval ownership.

Risk review: dependencies 4 (signed Brain publication and existing owners), blast
radius 3 (manual action, report/privacy integration), state changes 3 (additive
record/provenance with old readers), reversibility 3 (disable admission and forward
fix), uncertainty 4 (usefulness unqualified). Total 17/25. No score grants approval.

Disposition: **Ready** for disabled implementation of Tasks 1–3, following the
approved design. Task 2 codec/privacy checks precede production wiring. Task 4
activation remains blocked on signed delivery and explicit live-trial budget.
One author plan review on 2026-11-12 checked scope, dependencies, compatibility,
recovery and checks; no speculative runtime authority or extra design gate added.

## Integration interface and ownership

Task 1 is implemented in the owning worktree; nineteen focused tests pass. The
separate store currently has no production caller. Whole-repository verification
precedes its integration checkpoint commit. Do not confuse this foundation with
an enabled feature or useful live-provider qualification.

Task 2 owns Rust backend integration (including provider controls, new typed
recommendation provenance, access/history/recovery and HTTP handlers). Task 3
owns `apps/desktop/` only. Each writer uses an owned sibling worktree from the
verified checkpoint. The parent integrates them; neither rewrites the other’s
files. At most two implementers run concurrently. Review has one initial scoped
pass per integration, with author adjudication of findings and one final whole
branch review. Use the current Luna default first.

Freeze the transport seam before parallel implementation:

- `GET /taskmaster/improvement`: Electron-authorized, read-only status, no body.
- `POST /taskmaster/improvement`: Electron-authorized, empty body, explicit manual
  start. Admission returns `{outcome,runId,reasonCode}` with the same three outcome
  values as readiness admission. No dispatch when production qualification or
  verified signed improvement guidance is missing. No request flag enables it.
- `POST /taskmaster/improvement/cancel`: Electron-authorized, strict bounded
  `{runId}` body. It can cancel only that run in the selected workspace.
- Status is `{workspacePath,available,reasonCode,run}`. Run is null or
  `{id,state,phase,startedAt,completedAt,errorCode,report}`; state uses the new store
  states, phase is collecting_evidence/reading_evidence/assessing/null. Report is
  null unless committed, current and allowed. A public report has `summary`,
  `concepts` (existing displayed readiness-row shape with repositoryFactHashes),
  `opportunities` (the final opportunity fields), `preferredOpportunityId`,
  `noProposalReason`, `limitations`, `coverage`, `overallScore`,
  `recommendationId`, `historySnapshotId`, `historyError`, `repositoryEvidence`
  (cited range facts plus complete flag) and `knowledgeEvidence`. A reader never
  treats the persisted report alone as current execution authority.
- Main/preload/renderer independently expose `getImprovementStatus()`,
  `requestImprovement()` and `cancelImprovement(runId)` through narrow IPC.
  No arbitrary URLs/headers/credentials cross into the renderer. Main validates
  the cancel ID and enforces a bounded response before parsing; renderer validates
  the public response and ignores stale workspace generations.
- New recommendation provenance adds optional `improvementReference` with
  `{version:1,intent:implementation|investigation}`; the existing provenance ID
  is the exact report/run join. Legacy absence retains old behavior. Readers
  validate the new purpose through its own report/currentness owner rather than
  accepting an old assessment report. Unknown versions/intents fail closed.
  Investigation intent changes card and handoff wording to **Investigate with AI**
  and prohibits an implementation instruction. Existing acceptance still needs
  an explicit click and the active target session.
- The new primary button is selected only when status `available` is true;
  otherwise retain the current primary action. Show the new report independently
  when it is current. Its own score is secondary to the preferred next step;
  never substitute an old history score into that result. Status/history reads
  never invoke inference. UI copy discloses at most six calls.
- Production qualification stays false until signed playbook delivery and paired
  usefulness evidence; deterministic fixtures may exercise qualified behavior
  through private test state only. There is no user/env/HTTP bypass. The existing
  starter remains valid for legacy actions and is not relabeled as compatible.

The remaining deadline must reach actual process/HTTP execution, not just a
pre-call timer. Cancellation stops new dispatch and requests termination of the
current owned invocation; ownership and the analysis lease persist through actual
cleanup. If existing transport APIs cannot provide that guarantee, extend their
control seam with focused regression coverage before making admission eligible.

## Bounded handoff — 2026-11-12

Core checkpoint `7d95dc6e` is implemented and source-reviewed at medium effort
because the strict-protocol/evidence diff exceeds 500 source lines. Its nineteen
focused tests pass; the earlier full verifier passed 3,496 Rust tests (19 ignored),
Rust integrations, 1,722 desktop tests (5 skipped), build and 1,297 docs tests.
A required store aggregate-overflow fix and regression are included in the next
checkpoint. The review must be refreshed for the integrated code before merge.

Backend and desktop handoffs have been copied into the owning worktree. They
remain **uncommitted pending integrated verification**. Backend adds typed signed
capability capture, report privacy/origin reconciliation, strict provenance that
fails closed against legacy reports and unavailable authenticated route shells.
Desktop adds bounded IPC, strict report parsing, legacy fallback and investigation
wording. Parent regression fixes retain legacy Fix with AI, count UTF-8 bytes,
validate score/coverage consistency and combine workspace abort with IPC timeout.
Type-check and all 64 focused desktop tests passed after those fixes. Backend
worker focused tests/format passed. The integrated full verifier is running at
`/private/tmp/orkworks-brain-improvement-integrated-verify.log`; do not claim its
success until the actual final result is read. Tool session: 82221 in this thread.

The replacement is still unavailable. Outstanding Task 2 requirements: live run
and analysis-lease ownership, actual provider deadline/cancellation cleanup,
range-aware current report projection/fences, recommendation pending publication
and restart recovery, and linked history. No provider controls were implemented;
do not infer a runtime from the current unavailable HTTP shell. Do not enable or
merge as a complete feature. Task 4 needs signed publication [#1703](https://github.com/Rambolarsen/orkworks/issues/1703)
and a separately authorized finite paired-usefulness trial budget. Actual packaged
brain-3 has only assessment/readiness capabilities and lacks the new entry.

The root worktree is `/Users/froomiebot/workspace/orkworks-brain-improvement-design`.
Child backend/desktop worktrees retain the handoff originals; their writers have
stopped. Preserve them until current parent changes are committed/verified, then
remove only these explicitly owned child worktrees. No live provider calls,
publication, migration, merge or production activation occurred. The original
PR #1690 two-hour lifecycle budget expires at 06:47 UTC; hand off rather than
silently reset it. Continue with the remaining real runtime/transport seam and
current-tree verification/review in a fresh session, following #1694.
