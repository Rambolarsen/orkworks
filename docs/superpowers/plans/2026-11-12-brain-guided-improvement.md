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

- [x] Add private runtime and store modules using captured evaluation identity
  and existing analysis lease. Register code-owned native response shapes;
  custom/Claude/Ollama output still goes through strict application decoding.
- [x] Before wiring a route, implement new report/provenance/history codecs,
  range restoration, complete input identity and every access/deletion owner.
  Prove serialized maximum bounds and old-reader fail-closed behavior.
- [x] Test with deterministic provider fixtures: multiple evidence rounds without
  Peon, final ratings and proposal from shared facts, early finish and all six
  calls, deadline/cancel/changed inputs, denied reads, malformed output, provider
  failure, no useful opportunity and missing signed capability. No fallback.
- [x] Test pending publication/live reconciliation/restart, duplicate dismissal,
  settings reduction across unopened workspaces, stale list/get/accept/handoff,
  unknown versions and optional history-publication failure. No inference replay.
- [x] Check: focused runtime/store/privacy and existing assessment/readiness
  integration suites pass with production admission still disabled.

## Task 3: explicit desktop workflow and qualification gate

- [x] Add narrow authenticated start/status/cancel transport and independent
  main/preload/renderer contract copies; keep the electron/src import boundary.
- [x] Present the preferred next step, brief ranked alternatives, assessment and
  limits in Recommendations. Wire one run to the primary action only when its
  supported signed capability and qualification are available. Preserve existing
  standalone readiness/history and background analysis. Show useful unavailable,
  partial, no-result, interrupted and failure states without mixing old scores.
- [x] Verify real routing and desktop fixtures: no background inference from a
  status/history read; stale workspace responses ignored; explicit handoff labels;
  cancellation and disabled/unavailable knowledge retain a usable old flow.
- [x] Run `bash scripts/verify-repo.sh`, scoped Rust build/test/fmt and desktop
  checks; fresh `/code-review low` of current code diff before merge. Update public
  behavior claims only for delivered source behavior, never infer installer state.

## Task 4: signed guidance and usefulness before activation

- [ ] Track the required reviewed signed entry point/export/starter with Brain's
  publication owner. This repository task does not edit Brain or publish keys.
- [x] Preserve current production routing until the compatible signed capability
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

### Integration review findings retained for continuation

The read-only follow-up review requested two changes before integrating/merging
this working-tree delta: add explicit small `DefaultBodyLimit` layers to the new
POST start/cancel routes and test oversized bodies; render cited repository and
Brain evidence excerpts/identities in the preferred-result UI. These remain
unresolved at handoff. The reviewer checked the signed-proof, privacy/provenance,
legacy fallback, typed intent and store-bound correction without further findings.
Do not mark the integrated working tree review-complete or merge it yet.

## Explicit continuation — 2026-11-12 12:00 UTC

The owner explicitly instructed "keep going" after the bounded handoff. Continue
the same approved task within a fresh bounded two-hour window, through 14:00 UTC;
this is owner-directed continuation, not an unattended budget reset. The integrated
verifier completed successfully. Both retained review findings now have passing
regressions: a one-KiB route body limit and visible cited repository/Brain evidence.

Use two isolated backend writers. Runtime owner edits Taskmaster improvement
runtime/access/store/history, response schemas and HTTP. Provider-control owner
edits providers.rs, providers/custom_inference.rs, providers/inference.rs and
taskmaster/runtime/inference.rs. Frozen control seam:
`providers::InferenceControl::new(deadline, Arc<AtomicBool>)`, cloneable, with
`remaining()` and `is_cancelled()`; native
`invoke_native_taskmaster_prompt_controlled` takes the existing response-shape
invocation arguments plus `&InferenceControl` last; custom
`invoke_custom_inference_controlled` extends the existing custom dispatch-gate
invocation likewise. Existing callers retain old behavior. Controlled calls
return after actual owned child/request and pipe cleanup, retaining the caller’s
analysis lease through delayed physical cleanup. Signed publication and live-trial
budget gates remain. The parent owns desktop/review fixes and integration.

Bulk syncing was rejected by automatic approval review to protect uncommitted
work. Preserve the retained handoff and apply only explicitly compared changes;
never overwrite an entire existing source tree to update a baseline.


## Continuation verification, 12:39 UTC

The completed controlled-provider task passed a separate medium code review with
no required changes. Seven controlled-call tests and the existing provider suite
passed in the provider worktree; Windows job-object execution remains a CI check.
The four owned provider files were integrated only after comparing each target
against its known base; no bulk source-tree replacement was performed.

The desktop task now includes explicit pending-publication busy state, individual
assessment rows and their citations, preferred-opportunity repository/Brain
excerpts, linked-recommendation refresh, UTF-8 range arithmetic and a standalone
read-only investigation handoff. The backend independently enforces investigation
scope rather than trusting arbitrary prompt overrides. Desktop review identified
a synchronous workspace-switch rendering race; the boundary fix passed re-review.
The affected 114 desktop tests and TypeScript checks pass. No final integrated
repository-verification claim is made for the still-active runtime work.

Concrete local, unsigned prerequisites are prepared outside the repository:
`/private/tmp/orkworks-brain-improvement-publication-candidate/` contains a public
playbook page proposal and publication-owner instructions. All related IDs resolve
against the packaged public starter; entry plus mandatory rubric content is
15,991 UTF-8 bytes. No Brain content or signed starter was modified or published.
`/private/tmp/orkworks-brain-improvement-qualification-corpus/` contains the fixed
three-positive/two-negative inert fixtures and separate adjudication criteria.
No live provider call, independent coding harness or usefulness trial occurred.
These artifacts do not satisfy the publication or qualification activation gate.

The runtime owner is completing recommendation-consumer currentness, pending
recovery, global/unopened-workspace privacy, later-range identity and history
codec tests. Production admission remains disabled while those tasks and the
external activation prerequisites are incomplete.


Documentation checks for the current continuation pass: OKF metadata/indexes and
1,297 documentation validation tests, documentation currency and worktree
currency. The currency script recommends public onboarding and architecture
review because code/API surfaces changed. Public onboarding is intentionally
unchanged while the replacement remains disabled; the architecture's accepted
design section states that existing manual behavior remains until qualification.
The domain reference now defines the new attempt states and explicit read-only
investigation intent. No published-installer behavior is claimed.


## Final integration and review, 12:57 UTC

The frozen backend handoff is integrated through an explicit 21-file manifest,
with unchanged-parent hash checks and per-file backups. The owning branch was
rebased onto `origin/main` at `6fc39980`; upstream provider-test stabilization
was preserved. Parent-owned core tests and route limits were retained.

Backend focused evidence: 29 improvement tests, 16 assessment-access tests, two
readiness-store tests, one authenticated HTTP test and one investigation-handoff
test passed. The core now supplies every distinct required rubric page before
the first model call; 22 core regressions pass, including no dispatch for missing
or oversized mandatory guidance. Final backend coverage includes a real controlled
HTTP transport against a local mock Ollama endpoint, a barrier-controlled custom
authority/cancel race, exact pending recovery, changed inputs, global/unopened
redaction, no-proposal rubric citations, history failure and dismissal dedupe.

An audit caught and corrected a custom-dispatch lock inversion before handoff.
The intended order is runtime authority, per-run dispatch mutex, then workspace;
registry access never spans those guards. Durable pending-marker persistence is
the cancellation cutoff after provider cleanup. This boundary is included in the
fresh final review rather than inferred from passing transport tests.

The complete repository verifier is running against the integrated current code.
Two fresh medium-effort code reviews cover runtime/privacy/history and
provider/API/desktop integration, respectively. Medium is justified by the
cross-component concurrency and protocol change and more than 500 changed lines.
The frozen code identities are retained locally for matching the reviewed code
to the eventual commit. Source remains disabled pending external activation gates.


### Final-review corrections

Both fresh medium-effort reviews are complete. Integration review required a
bounded streaming response reader in Electron rather than applying the byte cap
after `response.text()`. The transport now stops and cancels before EOF on
oversize, and checks sidecar generation during reads. A pre-existing security
fixture was updated from a text-only response double to an actual stream; its
fetch/body generation-change assertions still pass. The affected 130 desktop
regressions and Electron type check pass. Re-review found no remaining integration
finding.

Runtime review required current-report joins for terminal recommendations as
well as active ones, and cancellation/deadline checks during initial collection.
Terminal projection now fails closed without the current report; normal dismissal
watermarks remain compatible and duplicate advice stays suppressed. An ephemeral
read guard covers canonical seed preparation, range opens/reads and revalidation.
Runtime passes its original absolute deadline and shared cancellation flag. The
failure-settlement cutoff also shares the per-run dispatch mutex with cancel.
Focused regressions passed, including every collection guard boundary and later
reads; re-review found both findings resolved with no concrete residual finding.

The first complete run passed 3,530 Rust tests (19 ignored), then caught the old
text-only sidecar-generation fixture during desktop tests. That fixture is now
corrected and passing. The definitive complete verifier is running after all
source fixes; the earlier partial run is not completion evidence for the final
code. No live usefulness qualification or activation occurred.


## Verified source completion, 13:18 UTC

The definitive `bash scripts/verify-repo.sh` completed successfully after every
review correction: Rust build/format and 3,532 tests passed (19 ignored), Rust
integration tests passed, 31 OpenCode reporter tests passed, desktop type check,
native ABI verification, 1,743 desktop tests (five skipped) and desktop build
passed, and 1,438 documentation tests plus docs build passed. Git diff,
documentation currency and worktree currency checks passed. Existing unrelated
compiler warnings were reviewed; no changed claim requires an installer claim.

Both current-code medium reviews are complete with all required findings resolved.
The reviewed code/test identities match the final verifier input. Tasks 1–3 are
complete as disabled source implementation; this does not satisfy Task 4 or fix
the currently installed primary flow. The external signed-publication and
separately budgeted paired-usefulness gates remain open in #1703 and #1694.
No private Brain reads, signed publication, live model call or activation occurred.


## Signed publication and first usefulness attempt, 18:47 UTC

The owner explicitly approved Brain PR #147's exact public candidate and signed
publication, then authorized five paired trials capped at sixty model calls with
no silent retries or model switches. Brain PR #147 merged as
`bc0d46a599be626885ac39d2c2a016ae963287a1`. The successful publication workflow
[43276768094](https://github.com/Rambolarsen/brain/actions/runs/43276768094)
deployed immutable `brain-4` with the unchanged production key.
[Publication operations](../../agents/taskmaster-knowledge-publication.md#improvement-guidance-publication)
records the verified identities and recovery. The actual signed entry and rubric
fit the initial guidance budget. Source now packages the generated activation;
focused offline starter verification passes. This establishes signed guidance
availability, not recommendation quality or an installed release.

The owner-authorized trial ran once for each fixed case using Codex
`gpt-6-luna` at medium reasoning effort through the existing inference-only
native adapter. Both arms used the actual signed guidance and production
collector over the same pinned Git fixtures. The app used the production
investigation engine; the controlled baseline used a data-only broker and
natural final advice rather than the app's fourteen-row result schema. Neither
arm could execute commands, edit files or use native tools. Each arm retained
the six-call/eight-minute ceilings and the same evidence/guidance limits.

The compiled trial harness SHA-256 was
`2436a55f93b6fceb43cce5ce50b0c0b4b2be59aa4f9465106c46d1189a8911c3`;
`run.json` separately records the base source commit, runtime-module hash, signed
identity and exact model selection. Full fixture trees, tracked paths and file
hashes were verified before dispatch. All requests and raw responses, including
rejected results, remain in the owner-local qualification artifacts. The ledger
finished **incomplete**, with **27 reserved and 27 completed calls of 60**, no
retries, switches or fixture changes. Unused calls do not authorize a second
run of a case.

| Fixed case | App / baseline calls | App outcome | Controlled baseline |
| --- | --- | --- | --- |
| Behavioral assertions | 4 / 2 | Valid preferred change: assert the arithmetic return value rather than a constant | Invalid evidence response |
| Entry command | 3 / 2 | Valid preferred change: align the absent verification script named in instructions with the actual script | Invalid evidence response |
| CI failure status | 3 / 2 | Rejected final: useful failure-masking diagnosis with citations to undelivered pages | Invalid evidence response |
| Already satisfied | 3 / 2 | Valid result with no preferred change | Invalid evidence response |
| Restricted tests | 4 / 2 | Rejected final: undelivered-page citations; raw setup-documentation proposal is a weak but plausible supported hypothesis | Invalid evidence response |

Actual offline decoding of each rejected app response against its saved prompt
returned `Improvement opportunity has invalid identity or citations`. The raw CI
response recognized the workflow's unconditional-success fallback, but cannot
count as a valid positive after rejection. The restricted response correctly
recognized that excluded tests are unknown; it did not claim they were absent.
Independent review treats its local-version alignment idea as a weak but plausible
supported hypothesis with modest payoff for this small dependency-free package.
An observed incident is not required by the design. Citation rejection still
prevents counting that result as a validated negative pass.

All five baseline second responses set `finish: true` while requesting knowledge
pages. The decoder correctly rejected that combination. The baseline prompt
omitted the explicit empty-request completion rule present in the app prompt,
so these failures do not show that direct Brain advice is poor. PR review also
found that the baseline omitted the full signed guidance catalog available to
the app, further preventing an equivalent discovery comparison. Two rejected
app finals retained transport-completed per-call ledger rows because semantic
validation errors were recognized through a narrow message-prefix check; the
arm result files correctly recorded failure. Source corrections supply the same
catalog and classify production decoding failures explicitly. The original
27-call ledger and outputs remain unchanged. There are no
valid baseline preferred opportunities to compare; the paired-comparison gate
is **unevaluable**, not passed by default. Independent adjudication accepts the
two valid positives and the satisfied negative, but does **not** qualify
activation. The two-of-three positive floor alone cannot override that gate.

Post-trial corrections clarify the baseline completion contract and explicitly
separate final citation sources from catalog-only guidance in the production
prompt. Citation validation remains unchanged. These corrections have not had
a new live trial. Production qualification stays false, the existing primary
action remains, and #1694 stays open. A new paired run requires an explicit
owner authorization because the approved plan permits one attempt per case.

### Qualification continuation verification

The full repository verifier completed with `RUST_TEST_THREADS=1`: 3,541 Rust
tests passed (21 ignored), integration tests passed, 31 reporter tests passed,
1,743 desktop tests passed (five skipped), native ABI/type checks and desktop
build passed, and 1,505 documentation tests plus docs build passed. Two earlier
parallel runs encountered existing timing-sensitive SQL queue/expiry and
readiness scheduling assertions, followed by shared-lock poisoning; the first
failing tests passed independently and the serial full run passed. Those
failures remain in the local verification logs; no unrelated production code
was changed to suppress them.

After the final prompt-only corrections, all 23 improvement-engine tests and
all ten deterministic qualification tests passed. Both new prompt contracts
were observed failing before their corrections and passing afterward. The
post-trial runner hash is
`d6f2af43e79b484d3a97d4a0804f92967af695b1ac95eb19415b2a113659250f`,
which differs from the preserved live-run hash above. A fresh manual medium
review approved the current code/assets diff without findings; reviewed file
hashes were independently matched before commit. Medium effort is justified
by the test harness's size and provider-authorization boundaries.

Documentation and worktree currency checks passed. The documentation currency
warning recommends public onboarding review because a release input changed;
public user instructions remain accurate while the new flow is disabled. The
canonical architecture and publication guide now distinguish signed starter
availability from the unpassed usefulness gate and published installers.

### PR review cycle 1

PR #1733 review identified the missing baseline discovery catalog and the
semantic-rejection ledger label described above. The corrected baseline exposes
the same signed page metadata as the app while retaining content only for
selected pages. Both app response phases use the production decoders for audit
classification; the callback stages a decoder error and updates the ledger only
after the engine settles with that exact error. Cancellation, deadline and
evidence errors retain precedence. This avoids rewriting a transport-completed
response as invalid when an intervening input or lifecycle check wins.

Twelve deterministic qualification tests pass, including raw-output preservation,
malformed evidence/final responses, semantic citation/preferred/readiness errors,
catalog-only discovery and post-invocation settlement overrides. New regressions
failed before their fixes and passed afterward. The revised qualification-module
SHA-256 is `1f0494d3ccffca2ef970b6725ea0cdbc66be0cb0643d4c352aa84048719ac308`.
The original live trial ledger is byte-identical at 27 reserved/completed calls.
No new live inference or activation occurred.


## Corrected comparison and prose rejection diagnosis, 20:00 UTC

The owner answered “keep going” to the explicit request for a fresh five-pair
comparison capped at sixty calls. The second cohort used a separate corpus and
ledger, preserved the first attempt, and kept Codex `gpt-6-luna` / medium, the
actual signed `brain-4` proof, identical pinned fixture revisions/file hashes,
and the same per-arm limits. Source was `04de3bb57610466eadf546395be1b0bc54b3d14c`;
the compiled runner hash was
`1f0494d3ccffca2ef970b6725ea0cdbc66be0cb0643d4c352aa84048719ac308`.
The twelve deterministic runner tests and zero-call signed-input preflight
passed before dispatch. This run finished incomplete at **35 reserved and 35
completed calls of 60**, with no retries or switches.

| Fixed case | App / baseline calls | App result | Baseline adjudication |
| --- | --- | --- | --- |
| Behavioral assertions | 3 / 4 | Valid useful result-sensitive test assertion | Same useful preferred improvement; no loss |
| Entry command | 3 / 4 | Rejected by prose validator despite valid citations | Final contained evidence-protocol JSON, not advice |
| CI failure status | 3 / 4 | Rejected by prose validator despite valid citations | Final contained evidence-protocol JSON, not advice |
| Already satisfied | 3 / 3 | `provider_or_policy_failure`, no final text | Final requested more evidence, not advice |
| Restricted tests | 4 / 4 | Valid bounded investigation of tracked-test availability and CI discovery | Validation-map expansion is redundant or lower value; app investigation is better supported |

All five baseline transports returned nonempty final text, but independent
adjudication found three final responses were still evidence requests/finish
objects. Transport completion does not establish valid advice. The restricted
app result explicitly kept excluded test contents unknown and proposed an
investigation before a fix; it did not infer absent tests or bypass access.
The satisfied-negative failure emitted no invented gap, but cannot certify a
negative pass. No valid baseline-preferred loss was found; the one-of-three
positive result and incomplete comparison still block activation.

Offline replay through the production decoder identified the precise rejection
fields. The entry-command response's readiness limit said “No outcomes log,
issue history, or correction records are supplied.” Comma splitting treated
`issue history` as an imperative. The CI response's benefit said that preserved
status “would make automated test feedback more actionable”; the tool-token
check interpreted the causative verb as a Make recipe. These are descriptive
statements, not executable instructions or performed-verification claims. Both
responses' fact hashes and page IDs were present in their saved final prompts.
The satisfied case has no captured model text; its provider/policy category is
retained without claiming a more precise cause.

The bounded fix extends the existing negated-inventory grammar to `issue
history` and recognizes a bounded feedback noun phrase followed immediately by
its adjective complement in a modal causative clause. Known recipe targets, literal executable syntax, addressed-reader
instructions, shell operators and completed-verification claims remain denied.
Regressions include the actual phrases, a complete grounded opportunity,
unknown/known recipes, cross-sentence false exemptions and reader directives
across punctuation. The final baseline prompt no longer carries evidence-step
schemas/examples, and a typed evidence response is explicitly rejected as final
advice after its raw text is retained. No new live trial has tested these fixes.

The second ledger SHA-256 is
`e72059051fd50e3935de85b64735a66865b9789331bca9ba9ec767491f819054`;
the original first-run ledger remains byte-identical at 27 calls. Neither
replay nor post-trial corrections reclassify the historical attempts as passes.
Production qualification remains false and #1694 remains open.

Exact offline replay after the grammar correction accepts both immutable saved
final responses and retains their original preferred opportunities. This proves
the decoder false-positive fix without new model calls; it does not turn the
historical live failures into qualification passes. The temporary diagnostic
was removed. All twenty assessment-validator tests and 51 improvement tests
passed (two live entries ignored), and a fresh security-focused manual medium
review approved the source diff without findings. Reviewed source hashes match
the restored tree; full repository verification follows.

### Verified prose-fix source, 20:31 UTC

The complete repository verifier passed on the frozen corrected source: 3,547
Rust tests (21 ignored), integration tests, 31 reporter tests, 1,743 desktop
tests (five skipped), native/type checks and desktop build, and 1,564 docs tests
plus docs build. Git diff, documentation currency and worktree currency checks
passed. The public-onboarding currency suggestion was reviewed: these bounded
text/phase fixes add no setup steps, the new flow stays disabled, and source
claims remain distinct from installed releases. Both trial ledgers are unchanged.

The security-focused manual medium review approved all three changed source
files without findings, and their hashes matched the verification/commit input.
This is verified bug-fix source, not a passed live usefulness comparison or
authorization to activate the new flow.

A final docs-only recheck subsequently failed in the unchanged synthetic test
`unapproved native continuation packet passes only synthetic clients and process
checks`: Python's owned-fake-process cleanup raised `PermissionError` while
signaling the process group after its raw-file-cap canary. That named test
passed in isolation, then the complete documentation check passed all 1,564
tests without skips. The failure and rerun logs are retained; no process
supervisor or test expectations were changed in this task.

### Prose-fix PR review cycle 1

PR #1740 review found that the initial nearby-adjective search could exempt an
unknown Make target followed by a purpose phrase or conjunction. The reported
examples reproduced as failing rejection tests. The revised grammar recognizes
only a short plain-ASCII feedback noun phrase with known modifiers and an
immediately following `more`/`less` plus `actionable`/`reliable` complement.
Identifiers, purpose phrases and conjunctions cannot borrow a later benefit.
Known recipes, literal syntax, addressed-reader wording and verification claims
retain their rejection paths. Twenty validator tests and 51 improvement tests
pass; the fresh manual medium security re-review approved the tightened source.
No live inference, historical result or activation state changed.

The exact saved entry-command and CI finals also decode successfully under this
tightened grammar without new inference. The temporary replay was removed and
all reviewed source hashes matched. The final full-verification and current-head
review results are retained in [PR #1740](https://github.com/Rambolarsen/orkworks/pull/1740)
before merge.

The tightened-source verifier rerun passed Rust, integration, reporter, desktop
and native/type checks, then encountered the same unchanged macOS synthetic
cleanup failure tracked in [#1741](https://github.com/Rambolarsen/orkworks/issues/1741).
All 1,564 documentation tests subsequently passed with concurrency one and no
skips; the site build is checked separately. No unrelated supervisor source,
assertion or limit was changed, and the failed full-run log remains retained.
