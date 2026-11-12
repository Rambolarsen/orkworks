---
type: Design
title: Brain-guided next improvement
description: Proposed manual investigation that returns a useful next agentic-development improvement and an assessment from shared evidence.
tags: [orkworks, taskmaster, brain, readiness, recommendations]
status: draft
workflow_status: approved
---

# Brain-guided next improvement

Tracking: [#1686](https://github.com/Rambolarsen/orkworks/issues/1686).
Decision: [ADR 0114](../../adr/0114-brain-guided-improvement-investigation.md).

## Outcome and approval scope

The primary action answers the owner's actual request: **Use Brain to suggest
the next improvement towards agentic development.** It investigates the selected
repository, ranks supported opportunities and explains one preferred change.
An assessment explains that choice. A numeric score is secondary and is not a
prerequisite for receiving useful advice.

The owner approved the direction and then this concrete design in the
2026-11-12 conversation ("go ahead" after review of draft PR #1690). The bounded
investigation, result/persistence changes and usage limits below are approved for
implementation. Runtime activation still requires the signed playbook dependency,
compatibility checks and usefulness qualification. Approval itself changes no
running app or installed knowledge. Implementation is tracked in
[#1694](https://github.com/Rambolarsen/orkworks/issues/1694).

## Evidence and current mismatch

The owner reports a score of 48 and zero suggestions, while a direct Brain-guided
agent produces many suggestions. The exact 48-point run has not been traced;
there is no claim that every direct-agent suggestion is correct.

Current source deliberately separates scoring from suggestion generation:

- [Readiness collection](../../../specs/repository-agent-readiness.md#collection-calls-and-authority)
  permits two calls and one follow-up file selection. The combined flow permits
  four calls, with a saved score followed by an independent assessment.
- [Assessment context](../../../crates/orkworksd/src/taskmaster/assessment_context.rs)
  reads bounded prefixes and exposes excerpts of at most 2,000 characters.
- [Assessment prompts](../../../crates/orkworksd/src/taskmaster/assessment.rs)
  require strict citations and positive opportunity evidence, with an at-most-one
  proposal response and extensive prose restrictions. Low or unknown ratings do
  not themselves establish an opportunity.
- [ADR 0099](../../adr/0099-readiness-evidence-before-candidates.md) already records
  failed usefulness samples and the candidate-before-evidence problem. Its
  evidence-first correction retains the same one-read/four-call limits.

In this session, read-only inspection found live Peon inference succeeding with
zero recorded observations, and a saved November 10 ordinary analysis with zero
created proposals. These are separate paths, not a reconstruction of the reported
score run. Observation quality remains [#1077](https://github.com/Rambolarsen/orkworks/issues/1077).

Brain's locally inspected `playbooks/propose-next-step.md`,
`playbooks/continuous-agent-readiness-improvement.md` and
`playbooks/assess-repository.md` instead describe repository inspection,
progressive guidance retrieval, bottleneck discovery, ranking and a preferred
next step. Direct agents can investigate links and implementation before choosing.
The restricted app path is not equivalent to that workflow. Whether those
restrictions explain this particular missing suggestion remains a hypothesis
until paired comparison.

## Product behavior

Keep **Find next improvement** as the primary manual action. One request owns
one investigation and one final result. No preceding score request is necessary.
Existing standalone scoring/history and background discovery stay available;
this design replaces only the primary combined manual action after qualification.

The final result presents:

- **Preferred next step:** the concrete change, affected repository surface,
  observed mechanism/opportunity, practical benefit, effort/risk/reversibility,
  supporting facts and Brain references, counterevidence, and a validation and
  rollback description. One preferred next step may become one recommendation.
- **Why this first:** up to five ranked opportunities, with qualitative ranking
  and brief reasons for deferring the alternatives. Alternatives are report
  findings, not active cards or implementation authorization.
- **Assessment:** the fourteen existing capabilities, each assessed only as far
  as the same evidence supports. Unknown remains unknown. Preserve existing
  deterministic score calculation: an overall score only with all fourteen
  rated, with coverage and limitations visible. No predicted point gain.
- **Investigation limits:** unavailable evidence, restricted paths, remaining
  uncertainty and whether the run exhausted a limit. Distinguish insufficient
  evidence, no worthwhile supported opportunity, existing/dismissed work,
  configuration failure, interruption and invalid model output.

A current repository opportunity need not be a demonstrated failure or a
repeated observation. It must explain a specific mechanism and plausible benefit
using inspected evidence. Unknown behavior cannot be presented as a missing
capability. When the bottleneck is uncertainty, a bounded evidence-gathering next
step may be suggested as investigation, not misrepresented as a verified fix.
Never manufacture an improvement to explain a low score.

Workflow observations are optional supporting evidence. Neither Peon setup nor
an observation count gates this manual repository investigation. Existing
observation-based cards and recurrence rules remain unchanged.

## One investigation module

A sidecar-owned manual investigation module owns admission, progressive reads,
model exchanges, final validation and publication. Its caller starts, inspects
or cancels one workspace-bound run. It uses the existing Taskmaster selection,
transport preparation/trust and installation-wide analysis lease. There is no
new coding session, child harness, provider fallback or native tool permission.

Reuse existing inference-only transports for a sequence of bounded JSON calls.
Intermediate responses request repository catalog IDs plus byte ranges and
signed Brain page IDs. The sidecar validates requests and returns immutable facts;
only it performs reads. Native model tools remain disabled under existing adapter
profiles. Unsupported providers remain unavailable; custom executable trust and
managed-policy disclosures remain unchanged.

The workflow follows the reviewed Brain entry point: detect configured harnesses,
inspect repository mechanisms, inspect relevant concepts/practices progressively,
check apparent gaps against existing implementation, rank opportunities and
produce the preferred next step with the assessment. The model may request
additional evidence without first proving a candidate defect.

Use complete returned text ranges, not a second 2,000-character excerpt of an
already bounded read. Every fact identifies its path, byte interval, captured
file size, digest and whether the interval covers the entire file. Partial reads
cannot establish file-wide absence. Ranges must end on UTF-8 boundaries. Excluded,
ignored, credential, non-text, symlink and out-of-workspace files remain denied.
Governing instruction closure is required before related source is supplied;
instructions are applicability context, never new execution authority.

## Explicit finite limits

These are approved defaults for this manual purpose, not changes to background
analysis or existing standalone scoring:

| Resource | Proposed ceiling |
| --- | --- |
| Provider calls | Six total, with the final call reserved for the result |
| Evidence rounds | At most five, up to eight requests per response |
| Repository catalog | Existing 2,000-path/depth-five traversal; at most 512 model-facing entries/16 KiB, with incompleteness reported |
| Repository evidence | 32 distinct files, 32 returned ranges, 64 KiB aggregate returned text |
| One returned range | At most 8 KiB; later ranges may inspect beyond the original prefix |
| Signed guidance | 16 distinct pages, 32 KiB total text supplied |
| Request and response | 128 KiB request, 64 KiB response, including schemas and all context |
| Whole run | Eight minutes, including collection and model calls |
| Persisted final report | 128 KiB, including cited snapshots and publication marker |

Initial instruction/context reads count toward aggregate evidence bounds. The
sidecar clips neither requested read sets nor final results silently. Refused
reads return fixed reasons; a model can choose another permitted request within
remaining rounds. Exhausted evidence budgets still allow the reserved final call
if the time/context budget permits, with explicit limitations. A final model may
return a supported proposal despite partial coverage; it may not claim missing
omitted implementation. If finalization cannot run, show incomplete investigation
rather than a successful empty assessment. No automatic retry or replacement
sample follows invalid output or provider failure.

These ceilings bound work, not billing. Manual runs remain outside background
allowance/cooldown. Settings disclose up to six calls before first use; the action
starts them without per-read approvals. Every adapter keeps its existing shorter
invocation timeout, capped by the remaining absolute run deadline. Cancellation
or workspace/configuration change stops new calls and reads, requests existing
process cancellation and suppresses late results. Retain ownership/lease until
actual process cleanup settles; a deadline does not prove a process has exited.

## Brain delivery and applicability

Use only the verified signed application bundle, pinned for the whole run.
Never fetch private Brain files or follow remote links during analysis. Package
a reviewed adaptation of the actual next-step playbook plus its concept/rubric
index and relevant practices, explicitly describing which steps OrkWorks cannot
perform: command execution, writing an assessment into Brain and central learning.
Checks needing execution remain unverified and can be proposed for later approval.

A new signed capability identifies that complete investigation entry point and
its compatible rubric, with explicit page-ID relationships. Missing guidance is
unavailable, not an invitation to synthesize substitute playbook text. Guidance
selection happens inside the pinned eligible bundle, using its index and links.
A knowledge-publication follow-up in the Brain repository must deliver/review the
starter/export before runtime activation; this design does not authorize edits to
Brain or claim that the current starter already supplies this capability.

## Evidence, output and authority

Both recommendation and capability ratings come from one final validated response
against the same accumulated immutable facts and pinned guidance. Do not create a
second independent proposal call after saving a score. Final structural validation
checks supported enums, bounds, known citations and consistent preferred-candidate
identity. Keep rejection of executable command payloads, unknown citations and
claims of unperformed validation. Reuse the current command/prose validator in
the first implementation, with positive ordinary-explanation fixtures and
negative executable-instruction/false-verification fixtures. This design does
not authorize weakening that validator. If it rejects an independently valid
proposal, retain the sample and resolve the precise validation change in a
separately reviewed contract before activation; do not bypass validation or
report that sample as a successful suggestion.

Separate evidence strength (observed problem, supported improvement hypothesis,
or investigation needed) from action authority. All suggestions remain advisory.
Only the preferred implementation opportunity can use existing explicit **Fix with
AI**. An investigation-needed result offers an explicitly labeled investigation
handoff; it must not claim a verified defect or silently become an implementation
prompt. Both handoffs require the user's click and use the existing active-session
handoff machinery. Nothing edits files, runs commands or launches/resumes sessions
during analysis. Ranking and assessment do not grant permission to change policy,
merge code or publish lessons.

## State, compatibility and recovery

Use an additive, versioned workspace-local `improvement-assessment.json` record;
do not reinterpret old `assessment.json`, readiness history or combined-run files.
The new report is the authoritative join between the preferred recommendation
and assessment rows. Existing score history receives a linked snapshot only when
it fits its unchanged format/cap through a new explicitly versioned purpose; its
numeric calculation stays unchanged. A history-publication failure is reported
separately and does not hide a committed recommendation/report. Do not display an
unrelated old score as the score belonging to the new result.

The report stores only cited snapshots and bounded final findings, not full
prompts, model transcripts or uncited file text. Its manifest retains identities
for every supplied fact, page, observation, catalog and settings snapshot,
including uncited input. Purpose/collector/prompt/schema versions, selected
model/effort/trust identity and knowledge identity enter freshness/cache checks.
Ranged facts require range-aware revalidation, not legacy prefix restoration.

Reuse existing recommendation deduplication/dismissal and pending-publication
ownership. Publish the report and preferred recommendation with a stable run ID,
pending marker and idempotent reconciliation before either is actionable.
Progress and incomplete states never mint a recommendation. Reconcile pending
publication on restart; interrupted investigation never replays inference.
Changed evidence/context/workspace/provider/trust/guidance suppresses stale
publication and handoff under existing currentness rules. Access narrowing
redacts cited content and derived prose from the new report, its linked history
and recommendations across all affected workspaces before exposure. Workspace
metadata deletion includes the new record. No analysis report goes to Brain.

The new purpose and storage version must fail closed in old readers; deployment
is sidecar/desktop together. Rollback disables new admission and forward-fixes
publication/read problems while retaining existing privacy reconciliation for
saved new reports. Do not downgrade binaries or hand-edit local records.
The design requires a concrete wire/storage/privacy compatibility check in the
implementation plan before code, not a guessed reuse of old strict codecs.
That check must define the exact history link and publication order, include the
new record in global/workspace access-narrowing and deletion owners, and prove
that every persisted representation fits its own cap. No existing codec is
assumed to accept a new link or purpose. Admission stays disabled if these checks
remain unresolved; a new report reader cannot be shipped without its privacy
reconciliation path.

## Alternatives and chosen tradeoff

- More wording changes inside the existing two-call assessment are cheapest but
  do not let the model inspect a newly discovered dependency or a later file range.
- Launching an unrestricted coding agent most closely resembles direct usage but
  changes authority, privacy and execution semantics. It is outside this scope.
- The proposed bounded sidecar-mediated investigation reuses inference transports
  and collection policy while adding progressive evidence. It costs up to six
  calls and requires range-aware identity, privacy and recovery tests. Usefulness
  remains to be demonstrated rather than inferred from this architecture.

## Delivery scope and acceptance

After approval of this concrete design, write one concise implementation plan:

1. Close the new strict request/result and stored-record shapes against current
   evidence, recommendation/history and privacy owners. Deliver the reviewed
   signed Brain capability as a separate publication dependency.
2. Implement the manual investigation behind disabled admission, with deterministic
   provider fixtures exercising iterative reads, shared final scoring/proposal,
   cancellation/budgets, denied reads, malformed output and all freshness/redaction/
   restart/publication paths. Existing actions and old records must remain intact.
3. Wire the primary action and result explanation to that one run. Preserve
   standalone score/history, background analysis and explicit handoffs. Verify
   partial coverage and no-result/error states through the real desktop flow.
4. Run paired usefulness qualification before enabling the new action. Update
   authoritative contracts and public behavior claims only for delivered scope.

Qualification compares the exact owner request against a direct Brain-guided
agent at a pinned repository revision, equivalent permitted files/knowledge and
the same provider/model/effort. Record differences in tools, context, time and
knowledge; an unrestricted direct-agent run is a diagnostic baseline, not a
controlled equivalent. Independent adjudication checks both outputs against
actual artifacts; more suggestions alone is not success.

Use a fixed corpus: three known worthwhile opportunities, one already-satisfied
repository and one evidence-restricted case. Credit a positive only for a
specific valid next step with evidence, benefit and validation scope; require at
least two of three positives and no invented gap in either negative. Retain all
raw outputs and failures in owner-approved, credential-free qualification artifacts.
Count invalid/rejected/empty positive results as failures. Every direct-agent
candidate absent from OrkWorks must have a recorded explanation (invalid,
unsupported, duplicate, lower priority, or missed by the app). In controlled
same-evidence pairs, a valid baseline preferred opportunity missed by the app
blocks activation unless the app supplies an independently adjudicated equally
useful or better supported next step. A difference in phrasing or choice between
comparable improvements is acceptable; an unexplained loss of useful advice is
not. The two-of-three floor alone cannot override that paired comparison gate.
For unrestricted diagnostic baselines, context/tool differences are reported and
must not be counted as a controlled pass or failure. At most one
six-call app run and one bounded direct-agent run per case, no silent retries or
model switching. Live provider trials require a separate explicit call/cost budget
in the implementation plan; none are run by this design task.

Transport/schema tests cannot establish usefulness. Observation capture/model
quality, shared provider setup, automatic discovery redesign, arbitrary tool use
and orchestration remain outside this deliverable.

## Design review

Two independent read-only `gpt-6-luna` reviewers checked builder completeness
and current-contract correctness on 2026-11-12. The final author revision makes
paired usefulness loss an explicit activation blocker, preserves the existing
command validator and names the required history/privacy compatibility proof.
The latter remains an implementation-plan prerequisite, not an assertion that
current strict codecs already support the proposed record. No runtime or live
provider qualification was performed.
