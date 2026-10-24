---
type: "Design"
title: "Clarification and research-to-execution lifecycle"
description: "Design history: Clarification and research-to-execution lifecycle."
tags: ["orkworks", "design"]
---

# Clarification and research-to-execution lifecycle

- Status: proposed contract; continuity direction approved, written contract review pending
- Date: 2026-10-04
- Tracking: [#742](https://github.com/Rambolarsen/orkworks/issues/742), parent [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Product direction: [Agent hierarchy and configuration learning](2026-10-04-agent-hierarchy-and-configuration-learning-design.md)
- Launch baseline: [Taskmaster orchestrated child sessions](2026-09-26-taskmaster-orchestrated-child-sessions-design.md)
- Configuration: [Role configuration](2026-10-04-agent-role-configuration-design.md)
- Eligibility: [Capability evidence register](../../validation/agent-role-capabilities.md), [Claude Code 2.1.287 static assessment](../../validation/claude-code-role-capability-evidence.md)

## Scope and status

Keep the same orchestrator running from goal assessment through clarification,
approved research, preparation of execution, and approved execution. The user
chose continuity and a high degree of automation. The parent automatically
collects available reports, synthesizes the research summary, proposes the
execution plan, and requests ready declared tasks after approval. There is no
mandatory pause/resume action between successful research and plan preparation.

Automation does not remove exact-plan approval. Research and every new execution plan require separate exact approval;
research completion never transfers approval into an execution plan. The parent delegates investigation, implementation, verification, and
review. It never integrates child code. Plan revision approval, dependent
worktree quiescence acknowledgement, and manual integration remain user actions.

This contract proposes a specific amendment to the baseline's one-plan parent
capability lifetime: run planning authority survives research-plan completion;
that plan's execution authorization does not. It does not silently reuse the
baseline's expired plan bearer. The [#610 scope alignment](2026-10-04-taskmaster-orchestration-scope-design.md)
and [ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md) now
carry the accepted lifecycle scope into authoritative routing. Scope acceptance
is recorded on PR #747; detailed contract and adapter evidence gates remain open.
This document does not authorize runtime implementation or claim adapter support.

### Established source facts and unresolved eligibility

| Source checked at `7d7e3d0b87b6388202da9a568cfaf6d4791d0525` | Established fact | Consequence for this proposal |
| --- | --- | --- |
| `crates/orkworksd/src/taskmaster/coordinator.rs:1` | Data-only coordinator records, no runtime authority | New run/grant records need a reviewed runtime plan |
| `crates/orkworksd/src/taskmaster/coordinator_store.rs:1` | Durable records, no runtime authority | Persisting a plan does not authorize a child |
| `crates/orkworksd/src/session_application.rs:5114`, `:5353`, `:5370` | Resume workflow retains the OrkWorks session ID and creates a new runtime | Same visible parent can resume; new runtime needs fresh authority |
| Historical September launch baseline | Completion revoked one plan-bound parent bearer | Replaced by the run/grant scope accepted in PR #747; detailed lifecycle contract review remains open |
| Capability evidence register | Exact role/delivery combinations remain unverified | No eligible automated parent is claimed yet |

A live PTY is not evidence that a parent can receive reports and act on them.
The selected adapter must verify a machine-readable coordination/event channel
that continues the same parent agent without typing into a terminal or starting
another coding tool. This is an additional #740 eligibility question. If the
adapter cannot provide it, automated continuity is unavailable for that adapter;
OrkWorks shows the missing capability instead of pretending the process is working.

## Assessment and skip decisions

The initial assessment uses the goal, user-supplied requirements, applicable
instruction/configuration snapshots, and declared coordination metadata. Reading
reported statuses and child reports is coordination. Opening the codebase to
investigate implementation, probing a tool, or searching external sources is
research and belongs to a child.

Record a `PreparationDecision` with its input revision, `clarificationNeeded`,
`researchNeeded`, reasons, and identified unknowns. Skip clarification only when
no unknown material requirement could change the scope, desired result, access,
or approach. Skip research when supplied evidence is sufficient to propose
assignments and their acceptance criteria without investigating an unresolved
technical question. Task size alone does not make missing requirements harmless.
A small, clear task goes directly to an execution proposal; loading a graph skill
does not force additional workers or research.

Before decomposing work, record what the orchestrator is least confident about
and what the project may be missing. Resolve user-owned requirements with
clarification; investigate technical uncertainty through approved research.
Carry unresolved risks into the next proposal rather than inventing answers.

## Material clarification lifecycle

A `ClarificationRequest` identifies the run, preparation revision, question ID,
one material question, why its answer matters, and up to four suggested answers.
Free-text answers are always allowed. Show the question in the parent hierarchy
and attention details. Prefer one concise question at a time; unrelated work
may continue only when already approved and independent of the answer.

Only an explicit user answer submitted through the UI resolves the request.
Record a `ClarificationAnswer` with request identity/version, answer ID, answer
text, and user provenance. Terminal text, model guesses, elapsed time and a
preselected option do not answer it. A stale answer is rejected with the current
request; duplicate identical answer IDs return the stored result and conflicting
replays fail. Changed answers create a new preparation revision.

Submit the answer through an Electron-main-authorized sidecar operation using
the existing UI authority. The sidecar derives user provenance from that
authorized operation; it never trusts a renderer-supplied provenance field or
the parent's run bearer as proof of a user answer. Validate the current run,
preparation revision, and clarification request version, then record the answer
and its new input revision atomically. This adds no renderer-held token or new
authority credential.

The parent can incorporate answers and prepare drafts automatically. A material
unanswered question blocks submission for approval of any plan that depends on
it. If it affects an approved plan, pause new launches and propose a revised
exact plan; already-running children are not silently reconfigured or killed.
Input revisions bind the request/answer records used by each proposal. Changes
invalidate a pending proposal; the UI cannot approve its stale digest.

Clarification refines the UI-authorized run goal; it cannot broaden the parent
bootstrap's permissions or immutable goal boundary. A genuinely new goal or a
required change to parent instructions/skills/profile requires an explicit UI
creation/relaunch decision under a newly approved bootstrap. The parent cannot
make that change through its planning capability.

## Research question and task contracts

Each question has an ID, question text, declared source/file scope and allowed
external research, known inputs, expected evidence, stopping conditions, and
explicit uncertainty to resolve. Map questions to named tasks with one artifact
owner, role/configuration digest, expected report, acceptance criteria,
dependencies, ordered batch and worktree group. Researchers answer assigned
questions; they do not redesign the run, edit product files, or spawn children.

Require the reviewed `orchestrating-task-graphs` skill in the bootstrap for
parallel planning. Split only questions that do not consume one another's
results. A task consuming another report has a real dependency and later batch;
parentage is not a dependency. Include distinct correctness and completeness
review questions for parallel reports in the proposed plan. Reviewers consume
the reconciled research output; the parent synthesizes reports without doing the
independent review itself. One writer owns each artifact.

Use the baseline's explicit ordered batches, one attempt per task and capacity
checks. A research plan lists every researcher/reviewer, declared result or
report artifact, exact configuration, source scope, worktree path/branch and
concurrency limit before approval. It creates no child or plan-owned worktree
until Electron-authorized approval of that exact revision.

A version-1 `ResearchReport` contains:

| Field | Meaning |
| --- | --- |
| `reportId`, `reportVersion` | Stable report identity and positive immutable version |
| `runId`, `planId`, `planRevision`, `taskId` | Exact approved research assignment |
| `reservationId`, `childSessionId`, `configurationDigest` | Exact launched assignment, derived/validated by the sidecar |
| `sidecarGeneration`, `launchGeneration` | Sidecar-derived producer runtime identity; a new child runtime/resume gets a new launch generation even when its OrkWorks session ID is unchanged |
| `questionIds` | Questions assigned to this task, without duplicates |
| `findings` | Answer, evidence reference IDs, and `established` / `inferred` / `unverified` classification |
| `uncertainties`, `blockers` | What was not established and why |
| `evidenceReferences` | Declared file/revision/location or source URL/retrieval time and bounded excerpt digest |
| `recommendations`, `limitations` | Proposed consequences and coverage limitations |
| `author` | Sidecar-derived `{"kind":"child"}` or `{"kind":"user"}` provenance; never caller-selected |
| `contentDigest` | Digest of the immutable validated report descriptor, including `author` |

Report authentication derives the child, reservation, configuration, sidecar
and active producer launch generation from the session capability and launch
records. Client fields must match those values. Check generation before accepting
or deduplicating a receipt: a pre-resume runtime cannot submit/replay evidence as
the current producer merely because the stable session ID matches. A user-authorized
child resume atomically revalidates the role/configuration, current worktree-group
ownership and live-child capacity before recording a fresh active runtime
binding. Successor reservation durably transfers the group and blocks predecessor
resume thereafter, even after the successor ends. Resume before handoff invalidates
earlier terminal/quiescence evidence; a fresh acknowledgement is required after
the resumed child ends. Resume never retries a task or restores a revoked grant; a new report needs a
new immutable version with that binding. Existing
report versions retain their original generation as historical evidence, never
rewritten or silently promoted to the resumed runtime's current delivery.

### Proposed evaluation admission-version snapshot

For #744's immutable child identity, propose a durable reservation metadata field
`assignmentTaskVersion`: a plain decimal JSON integer from 1 through 2^31−1.
It is not a configuration field, a runtime generation, a reporting capability or
a new mutable counter. The [evaluation reconciliation](2026-10-04-assignment-evaluation-design.md#configuration-and-reporting-reconciliation--2026-10-23)
uses it only as the frozen source of `AssignmentIdentity.taskVersion`.
This amendment remains proposed for written owner acceptance; it authorizes no
runtime implementation or eligible adapter.

1. At the baseline's successful child reservation transition, capture the
   **resulting** task-state `taskVersion` after that transition's normal version
   update. Commit the reservation, exact workspace/run/plan/revision/task/
   configuration binding, `launching` state and `assignmentTaskVersion` together,
   before child metadata attachment or PTY creation. Capturing the value causes
   no additional counter increment. Overflow, invalid version or failed durable
   commit rejects reservation; no child may start from a partial record.
2. Resolve an authenticated exact launch retry using its existing durable
   reservation and original snapshot, even if the live task counter has since
   advanced. Never recapture the current counter or make another reservation.
   Conflicting requests and revoked/stale authority follow the existing launch
   protocol; this metadata supplies no retry or launch permission.
3. Child metadata attachment and recovery must prove the exact reservation and
   configuration binding. Only its uniquely attached child session may complete
   the #744 child identity. A reservation without a verified attached child is
   not an evaluable assignment identity; never guess a child from a worktree or
   timestamp. Interrupted allocation retains the original snapshot; a separately
   approved replacement plan revision has its own reservation and snapshot.
4. Later task results, reports, pause/completion transitions, runtime resumes
   and sidecar restarts never rewrite `assignmentTaskVersion`. Exact authorized
   resume preserves the reservation/assignment identity but establishes fresh
   runtime authority. Current `expectedTaskVersion` checks still compare live
   task state, not this snapshot. #743's task-state evidence binding is unchanged.
5. Recover the snapshot only from the proven committed reservation history.
   Missing, corrupt, mismatched or ambiguous snapshot evidence blocks #744
   identity resolution; do not backfill from the latest counter, an old report
   or a child claim. Legacy records without this snapshot remain unsupported for
   #744, without changing ordinary-session behavior or treating valid #742
   reports as nonexistent. Retain the snapshot and exact attachment proof while
   any retained #744 identity/reference depends on them. Revocation/namespace
   rotation does not rewrite historical identity or restore old write authority.

The root bootstrap has no child reservation/task version and uses no synthetic
snapshot. The field is bounded reservation metadata, retained with that record
and included in its storage accounting; it is never hashed into the approved
#741 configuration, #742 plan definition or a `ResearchReport` descriptor.
Its value is hashed through #744's already-defined child identity. This proposal
introduces no alternate identity encoding or migration of previously hashed
#744 records. Such records must match the proven admission snapshot or remain
unsupported; do not relabel them in place.

Required synthetic verification cases, before any implementation qualification:

| Case | Required outcome |
| --- | --- |
| Crash before reservation commit | No admitted snapshot or child; no inferred identity |
| Crash after reservation commit, before child attachment | Recover one reservation and frozen version; no complete #744 child identity or automatic spawn |
| Concurrent duplicate launch requests | One reservation/snapshot; exact retry returns it without recapturing the live counter |
| Task advances then child resumes | #744 identity retains the snapshot; live CAS and runtime authority use their current independent values |
| Attachment points to another reservation/configuration | Reject identity resolution; no child/session-name fallback |
| Task-version transition exhausts its range | Reject reservation without partial allocation or wrapping |
| Recovery has only a current task counter or report | #744 identity unavailable; no historical backfill |

### Authenticated report submission

The proposed `submitResearchReport` child tool uses
`POST /sessions/{childSessionId}/orchestration/research-reports` on the existing
local sidecar channel. The request carries `schemaVersion` (1),
`expectedTaskVersion`, `expectedReportHead` and the immutable report descriptor, at most 32 KiB
including the envelope. It uses `Authorization: Bearer` with a distinct
OS-random `ResearchReportCapability`, injected through the verified child's
startup environment/tool integration, never model prompts or renderer state.
The parent's run bearer and ordinary workflow-report token are not accepted as
child report authentication. No second UI token is introduced.

Before child start the sidecar binds this volatile capability to workspace,
admission epoch/run, parent, exact approved plan/revision/task/reservation,
child session, configuration, sidecar and producer launch generations. It
allows only submission/reading of that assignment's report receipts and bounded
current report/task-version metadata. It grants no launch, approval, terminal
control, external-source or unrelated-session action. The adapter exposes only
this scoped tool; read-only researchers do not gain shell execution to report.
These credentials are same-user workflow associations, not OS process proof.

Validate authentication and current generation before schema/version/idempotency
checks and persistence. IDs/digests/source scope must match the stored assignment;
producer identity is sidecar-derived, never trusted from body fields. Responses:
`201` stores a new receipt; `200` returns an identical stored receipt; `400` invalid
schema; `401` absent/revoked capability; `403` cross-assignment/scope access;
`409` stale generation/task version/head or conflicting report ID/version; `413`
oversize; `429` rate limited. Bounded responses provide current authorized
metadata where needed; retries do not alter task outcomes or version counts.

Revoke this capability when that child runtime ends/resumes, on sidecar/workspace
change, run cancellation/final completion or namespace rotation. A live child
may report while its stage plan is paused/complete and the run remains active;
a report still cannot restore a launch grant. On an explicit verified child
resume, issue a fresh generation-bound capability, retain historical reports,
and require new versions for current delivery. After final run completion,
corrections require the separate Electron-authorized user-provenance operation
below; they cannot impersonate a child or reopen the run. Report acceptance emits a
bounded run event for automatic parent collection. Apply the stated report/rate/
aggregate limits before persistence and serialize with runtime/run revocation.

A report is an agent claim with provenance, not proof that its recommendations
are correct. Native tool observations and independent review evidence remain
separate. No full prompts, credentials, hidden reasoning, or complete transcripts
are stored in research reports. Evidence references point to approved sources;
a URL or quoted path does not authorize following it or widening access.

### Current report head and corrections

This is a proposed extension to the report protocol, for written review with
[#744's evidence-source contract](2026-10-04-assignment-evaluation-design.md#criteria-completeness-and-quality).
It specifies reporting only; it grants no additional investigation, child
resume, plan approval or launch authority. Withdrawal without replacement
remains unsupported. A replacement may explicitly record uncertainty or a
retracted finding rather than silently erase earlier evidence.

**Stream and head.** One exact approved research task/reservation has one report
stream. Its key object has exactly `workspaceId`, `runId`, `planId`,
`planRevision`, `taskId`, `reservationId`, `childSessionId` and
`configurationDigest`, using existing encodings and sidecar-derived bindings. A successor assignment
has a different stream. Runtime generations are immutable version provenance,
not stream keys: an authorized resume can correct the same assignment without
making its old runtime authoritative. The first accepted report fixes the
stream's `reportId`; another ID cannot bypass its correction history or limit.

The durable `ResearchReportHead` has exactly `reportId`, `reportVersion` and
`contentDigest`, with the existing ID/counter/digest encodings. It resolves one
immutable report with that stream identity and exact original generation
fields. No report means no head. First publication requires
`expectedReportHead: null` and `reportVersion: 1`. Each correction supplies the
complete prior head as `expectedReportHead`, retains `reportId`, and uses exactly
prior `reportVersion + 1`, capped at eight. Reject gaps, branch versions and
reusing a version with changed bytes. Neither latest arrival, largest numeric
version nor a resumed runtime selects a head; only the committed transition
does. Historical versions remain readable under existing scope/reference rules.

**Author and authority.** A child publication has `author: {"kind":"child"}`;
the report's sidecar/launch generations are its authenticated current runtime.
The parent cannot submit corrections for a researcher. The existing scoped
capability permits corrections while the run is active, including a paused or
completed stage; it still cannot survive runtime end, final run completion,
cancellation or namespace rotation.

An explicit user correction uses a separate Electron-main-authorized logical
`correctResearchReport` operation over existing UI authority, not the child
endpoint or a new renderer token. It requires an existing head, the exact stream,
`expectedRunVersion`, `expectedReportHead` and a replacement descriptor within
the same 32 KiB request limit. The sidecar sets `author: {"kind":"user"}`;
it copies assignment identity and generation fields from the immutable
predecessor named by `expectedReportHead` as
source lineage, not proof the child authored this version. User content cannot
change question IDs, approved source scope, assignment or configuration. The
content digest includes user provenance. #744's report identity maps these
lineage fields normally; reading details must retain the user author label.

The user may correct an existing retained stream in an active, completed or
cancelled run in the current admission epoch. This neither revives credentials
nor changes terminal lifecycle/coordination outcomes. Missing/forgotten runs and
old-epoch records remain unavailable/read-only under the admission fence. A user
cannot create an initial child report, impersonate a runtime, correct a foreign
workspace, or clear a blocker by submitting a replacement. Publication and any
later continue/reapproval remain separate explicit operations. A resumed child
may correct a user-authored head only under its fresh assignment-bound capability
and exact head CAS; each version keeps its own author, without promoting user
content to child evidence or vice versa.

**Acceptance order and retry.** Serialize against runtime revocation, namespace
rotation, run/task mutations and all publishers for the stream:

1. Authenticate and validate the admitted workspace/run/assignment and current
   authority generation. For the user operation, verify existing UI authority
   and retained current-epoch run identity. Revoked child requests fail before
   receipt lookup, including retries of an earlier successful publication.
   Then consume/check the authenticated child/per-run user request-rate budget
   before receipt lookup or any early return. Duplicates consume that budget;
   an exhausted budget returns `429` without changing stored records.
2. Validate the bounded schema, identities, source scope and sidecar-derived
   author. User normalization resolves the request’s immutable expected
   predecessor, not the live head; a later head must not change normalized retry
   bytes. Derive a receipt key from the stream plus `reportId`/`reportVersion`,
   and compare the exact canonical operation kind, expected task/run version,
   expected head and replacement descriptor against any stored request.
3. An exact accepted retry returns the saved receipt **before** task/run version
   or head CAS, even if a later correction advanced the head. A different
   request for that key conflicts. The receipt says which immutable version
   was accepted; it does not assert that version is still current. Currentness
   requires an authoritative head read. Never republish or re-invalidate on retry.
4. For a new request, compare the expected task version (child) or run version
   (user) and expected head against stored state. A stale comparison returns
   `409` with bounded authorized current metadata; it stores nothing. Validate
   exact next version and storage/counter quotas before committing. A changed same-version
   request is not a retry, even when its replacement content happens to match.
5. Durably publish the immutable descriptor, receipt/request binding, new head
   and dependent freshness fences as one recoverable transaction. Increment
   `runVersion` with checked arithmetic; reporting alone does not advance the
   task’s coordination version or outcome. For affected active preparation,
   create the new input revision in this transaction; terminal runs gain no
   preparation/proposal transition. New publication
   returns `201`; exact retry `200`. No successful receipt is returned until the
   transaction is durable. User operations use the same schema/conflict/size/rate
   response classes as child submission without accepting a child bearer.

The stored receipt binds stream, author, accepted head and exact request; its
request fingerprint is SHA-256 over `orkworks.research-report-request.v1\n`
(one literal LF) plus role-contract canonical JSON of the operation kind,
sidecar-resolved stream, expected versions/head and replacement descriptor.
The fingerprint payload has exactly `schemaVersion: 1`, `operation`
(`submitResearchReport` or `correctResearchReport`), `stream` (the stream-key
fields above), `expectedTaskVersion` (integer for child, otherwise null),
`expectedRunVersion` (integer for user, otherwise null), `expectedReportHead`
(head object or null), and `report` (complete accepted descriptor). All members
are required, including nulls. It excludes credentials, transport metadata and
the fingerprint itself. The
stored accepted descriptor includes `contentDigest`; its own content hash uses
the existing report domain and excludes only that hash field. A fingerprint
collision with a different canonical request fails closed. Retain the request
binding and receipt with the immutable version; reuse that descriptor rather
than duplicate its bytes in receipt metadata. No separate unbounded retry log.

**Dependent reads and recovery.** Head advancement makes the old exact report
version stale for every citing preparation summary/proposal and every #744
outcome that cites its source key. It never rewrites an accepted evaluation or
substitutes the replacement as reviewed evidence. Unrelated outcomes remain
eligible; #744 still computes overall failure/conflict precedence. A correction
invalidates affected pending proposals; affected approved plans pause new
launches and show the existing revision/reapproval blocker. Already-launched
assignments retain their original bindings and receive a visible blocker.
Historical plan/coordination records remain intact. Terminal runs stay terminal.

Consumers validate exact version/digest against the authoritative head in the
same serialization boundary as evidence admission, evaluation projection,
proposal approval and dependent launch. Dependent caches/projections must be
fenced before the new head is visible. If #745 repository-wide learning cannot
join that boundary, exclude/fence affected contributions before serving its
next summary; never serve an old cached passing outcome as current. Its joint
custody/repository transaction gate remains separate and closed until reviewed.

Recovery exposes either the complete old transaction or complete new
transaction, never a new head paired with current old dependent evidence.
An implementation must choose and review the journal/transaction and durable
fence mechanism before code. On missing descriptors, digest mismatch, ambiguous
head, or incomplete dependency reconciliation, block current evidence reads,
approvals and affected launches; do not elect the largest version or reconstruct
from event arrival. Reconcile durable receipts before permitting retries; no
recovery path recreates volatile credentials. Run events notify collection after
commit, carry bounded references only, and are not the authority for freshness.
An event-ring gap requires an authoritative snapshot of heads and fences.

For example, a child corrects head `(report-a, 1, digest-a)` to version 2
with `digest-b`. A user then corrects that exact head to version 3 with
`digest-c`. The still-authorized child’s exact version-2 retry returns its
original version-2 receipt; the head remains version 3. Another new request
expecting version 1 conflicts. An outcome citing version 1 or 2 is stale; only
a new explicit evaluation citing the user-authored version 3 can assess that
replacement. These symbolic digests illustrate transitions, not wire fixtures.

**Bounds and retention.** The eight-version budget is shared by child and user
publications, including the first report; competing writers share one head.
Each request/descriptor, receipt and head counts toward the 32 MiB run metadata
limit. User correction requests are limited to ten per rolling minute per run;
duplicates count against request rate but not stored versions. Receipt metadata
is capped at 2 KiB per accepted version, head metadata at 512 bytes per stream;
all fields use existing finite encodings. Quota or counter exhaustion rejects the
whole mutation, preserves the prior head, and shows a visible blocker; it cannot
silently publish a partial correction. Quota rejection does not advance the
head or freshness state; the visible rejection must distinguish retained prior
evidence from an accepted correction. No consumer can claim the rejected
replacement was incorporated. Exhaustion pauses affected automation under the
existing counter/limit policy, without rewriting stored evaluations. Existing run/input
counter exhaustion rules also apply when a correction invalidates preparation.
Deletion serializes with head publication and protects descriptors, receipts,
heads and dependency fences while referenced. Forget removes the whole eligible
stream only after references retire, preserving the admission high-water fence.

## Research approval and result collection

Only UI approval of the exact proposal creates a plan execution grant. Before
launch, revalidate every configuration and baseline launch condition. The parent
requests eligible declared tasks without additional per-child approvals, within
the approved batches and cap. Failed/blocked/interrupted tasks pause launches;
the parent may automatically collect reports and draft a recovery proposal,
but a retry or additional task still needs a new approved revision.

A report receipt and a task-turn receipt are different records. Reports do not
mark tasks successful, free capacity or unlock dependencies. Preserve the
baseline's authenticated turn-completion or explicit UI-action gate, then an
explicit parent coordination outcome bound to the exact revision/reservation/
child/task version. The parent can report completion after checking the report
contract and declared review results; it does not independently grade its work.
Missing required research, unresolved reviewer conflicts, or material uncertainty
prevents a ready execution proposal. No process exit or silence means success.

Once all task outcomes and reservations satisfy the baseline completion check,
mark the research plan complete and invalidate its execution grant atomically.
The run transitions to synthesis with the same live parent and planning
capability. Research children may remain live; their sessions, worktrees and
reports remain visible. Completing a plan does not terminate those processes.

## Same-parent research-to-execution transition

For a research-only goal or no-go conclusion, synthesize the bounded summary
without inventing execution assignments and show the version-bound UI finish
choice. The parent planning bearer cannot finish on the user's behalf.

The parent assembles a bounded `ResearchSummary`: findings affecting the
approach, remaining uncertainty, assignment choices and their reasons, plus
exact report versions/digests and review references. Display it with the
execution proposal; supporting sources remain in research-agent details.
The parent prepares this summary and proposal automatically after successful
research. It waits only for missing material information, blockers or user
approval, with the reason visible on the hierarchy.

A new execution plan has a new `planId`, the same `runId` and `parentSessionId`,
its own revision/digest and configurations, and references to the completed
research plan and input manifest. Research plans remain immutable historical
records. An execution proposal cannot mutate a completed research plan.
Planning is not launching: no execution child, worktree, retry or tool-profile
change is authorized by a summary or research-plan completion.

Only one plan revision may hold a launch grant for the run at a time. Subsequent
plans are proposed only after the preceding plan is complete or cancelled and
all interrupted allocations are explicitly resolved. A failed/blocked research
plan uses a recovery revision of that plan, not an overlapping execution plan.
Cancellation of a stage pauses the run for an explicit UI continue decision;
there is no automatic route around cancelled work.

The UI-created run has an immutable maximum live-child limit. Count every
nonterminal child from every plan in the run and every unattached reservation,
including research children whose tasks are reported complete. A plan may lower
this limit but cannot increase it. Launch checks enforce both the run limit and
the current plan's limit. Execution approval may occur with old children live;
launch waits until capacity is available. The parent does not terminate them to
make room. Show which sessions hold slots and the ordinary user controls.

Each new plan receives its own baseline-derived worktree groups. No cross-plan
reuse is introduced. The predecessor's changes are not automatically copied
or merged into an execution worktree. If needed inputs require user integration
or a changed base SHA, show that blocker and approve a proposal with the resulting
exact clean base. Within one plan, dependent reuse still requires an ended
predecessor and the user's quiescence acknowledgement.

## Planning authority and capability lifecycle

### Logical records and immutable parent configuration

These are proposed logical interfaces, not implemented types or final HTTP routes.
Version 1 uses the configuration contract's ID/digest/canonicalization rules.

| Record | Required binding |
| --- | --- |
| `OrchestrationRunDefinition` | `schemaVersion`, `runId`, workspace/parent identity, `repositoryBindingDigest`, `sourceWorktreeBindingDigest`, UI-authorized goal and `goalDigest`, sidecar-assigned `admissionEpoch`/`runSequence`, `bootstrapConfigurationDigest`, `maxParallelChildren`, `runDefinitionDigest` |
| `OrchestrationRunState` | Run version, closed lifecycle state, input revision/digest, ordered plan references, current plan, current parent runtime generation, durable blockers, independent `stageContinueRequired` gate, and pending clarification identity/version; mutable evidence separate from the definition |
| `PreparationRevision` | Run ID, strictly increasing revision, decision, exact question/answer/report/review references and digests, input manifest digest; immutable once referenced by a plan |
| `ParentPlanBinding` | Run/parent/plan/revision identities, run definition, bootstrap configuration, repository binding and source-worktree binding digests, preparation revision/input digest; included in the exact approved plan definition |
| `PlanExecutionGrant` | Server-held grant ID, run/parent/runtime/sidecar generation, exact approved plan revision and definition digest, UI approval ID, grant state (`active`, `paused`, `revoked`); never a child-held credential |

For `runDefinitionDigest`, canonicalize the immutable run definition without
that field and hash `orkworks.orchestration-run.v1\n` plus the bytes (one literal
LF after `v1`). `goalDigest` hashes the exact UTF-8 user goal bytes. Report
`contentDigest` excludes itself and uses `orkworks.research-report.v1\n`;
input manifests exclude `inputDigest` and use `orkworks.preparation-input.v1\n`.
Manifests include the exact decision and every consumed record's ID/version/
digest in deterministic namespace/ID/version order. Other meaningful arrays
retain their declared order. Never hash a bearer, grant state or mutable status
into an approved definition. Report corrections get new versions/digests.

The parent's startup instructions, role, mandatory skills, model policy and
permissions are reviewed at UI-authorized creation and snapshotted in a separate
`OrchestratorBootstrapConfiguration`. It uses the role contract's snapshot,
composition, digest and adapter rules; it is run-bound rather than future-plan-
bound. The complete repository and source-worktree binding descriptors are
immutable bootstrap inputs; the run definition and each `ParentPlanBinding`
retain their exact digests. Preparation input manifests cannot replace them.
`ParentPlanBinding` links each later plan to those unchanged bytes. The child
`AssignmentConfiguration` remains plan/task-bound and carries the same exact
repository and source-worktree bindings.

A plan's tasks and incoming child reports are coordination data. Reading a new
approved plan does not replace the parent's system/startup instructions or loaded
skill set. If those must change, fail drift validation and require an explicit
UI-authorized new run with a newly reviewed bootstrap. A current run keeps its
immutable bootstrap; exact resume of that run requires the same verified bytes
and settings. Changed bootstrap bytes cancel/fence the old run before new-run
creation; they do not migrate old approvals or child assignments. Never advertise a
skill selected for a later plan as loaded in an already-running parent.

### Transport and grants

1. Electron main creates the run using the existing `ORKWORKS_OPEN_PLAN_TOKEN`
   authority, withheld from renderer and coding-tool environments. It authorizes
   the goal, bootstrap and run ceiling. No second UI token is introduced.
2. The sidecar creates a fresh OS-random run planning bearer, bound in memory to
   the run, parent, parent runtime generation, workspace and sidecar generation.
   Inject it only through the adapter's verified parent startup environment.
   It is distinct from workflow-report credentials and never persisted/logged.
3. The bearer permits bounded clarification requests, proposals, reading run
   coordination state, waiting for run events, and explicit result reports for
   already-launched exact assignments. It cannot approve a plan, change bootstrap
   scope, create ordinary sessions, launch grandchildren or control other runs.
   While `stageContinueRequired` is set, it permits only reads, event
   waits and result reports for already-launched exact assignments. The sidecar
   rejects clarification or proposal mutations until the version-bound,
   Electron-main-authorized continue transition clears that gate. Runtime
   resume alone cannot clear it or restore proposal/clarification mutations.
4. Electron-authorized approval of an exact plan creates a fresh server-held
   `PlanExecutionGrant`. A launch presents the run bearer plus exact run/plan/
   revision/task identities. The sidecar checks the matching current grant and
   all baseline conditions atomically before reservation/allocation/spawn.
   The run bearer alone never grants a launch. No execution secret needs to be
   sent into the running model, terminal, prompt or renderer.
5. A paused plan has a paused grant: result collection remains allowed by the
   run bearer, while launches are denied. Exact recovery approval revokes the
   old grant and creates a fresh grant for the approved revision without resetting
   prior launch attempts. Research-plan completion invalidates its grant before
   moving to synthesis.
   The run bearer remains valid for preparing the linked execution plan. Its
   eventual approval creates a different grant; an expired plan bearer/grant is
   never reactivated. A rejected execution proposal supplies no launch grant.
6. Final execution completion ends the run and revokes the run bearer and all
   launch grants. After completed research, an exact version-bound UI finish
   action can instead complete a research-only run or decline execution, including
   a no-go conclusion. It atomically invalidates pending proposals and revokes
   all authority before any execution approval, with no execution allocation;
   unsettled reservations/interrupted allocations block finish. It does not infer
   result quality/user acceptance. Explicit run cancellation, parent ending,
   workspace change or
   sidecar-generation change likewise revokes both. Already-launched children
   retain ordinary session lifecycle and manual-integration records.

Grant validation and run/plan mutations serialize together, including capacity
reservations and cancellation. Requests racing completion/revocation cannot
reserve a child after the fencing transition. Idempotent requests return the
same durable reservation/result; they never produce a second launch.

This intentionally replaces the baseline's single bearer that both proposes and
executes one plan. It preserves exact-plan approval and generation fencing.
It is a same-user workflow boundary, not OS process-origin proof: another
same-user process may inspect/replay the environment bearer. Ordinary session
routes retain their existing boundary; coding-tool restrictions and skill
instructions do not turn this into OS confinement.

### Automatic event delivery

The proposed adapter exposes `waitForRunEvent` as a declared coordination tool.
A request uses the run capability and an exact run version/event cursor; it waits
at most 30 seconds and returns bounded changed-state references or an explicit
no-change result. The parent reads authoritative state before acting. Event
notifications are hints, never authority or proof of task success. Duplicate
notifications do not relaunch tasks or add report/usage counts.

Every parent-relevant UI mutation must advance the run version and publish a
cursor event: a clarification answer or other input revision, exact-plan
approval, proposal rejection, explicit stage continuation, report receipt,
task outcome, blocker change, or capacity release. Events contain only bounded
type and record references, not answer text, report contents, credentials, or
launch grants; the parent reads the authorized current record before acting.
This lets an answer resume clarification without a mandatory pause while
preventing a notification from becoming authority.

Final run finish/cancellation is terminal for the current bearer. The live
sidecar atomically revokes authority and wakes outstanding waits with
`run_closed`; the ended parent does not need another authorized read. Parent
runtime end or sidecar/workspace generation change also invalidates the wait
and capability; if the sidecar itself is gone, the request fails closed at the
transport boundary. UI resume starts a new runtime for the exact parent
identity and bound workspace, gives it a fresh capability, and provides
authoritative current state as startup context; it is not an event delivered to
the old runtime.

The adapter must demonstrate delivery of all nonterminal parent-relevant events
to the same parent agent when it is waiting, or return `refresh_required` with
the current run version when the bounded ring has overwritten the requested
cursor. In that case, the parent reads an authorized current-state snapshot and
continues from its returned cursor; it must not infer missed transitions from
partial event history. A tool continuation may return an event; terminal text
inference, scheduled new sessions and injecting keystrokes are not fallback
delivery mechanisms. If the parent's model loop has stopped, show automation
unavailable/needs continuation; do not equate a live PTY with successful
waiting. No reconnect after a parent end, sidecar restart or workspace change
is automatic. These event and continuation behaviors remain unverified under
#740 and do not qualify an adapter or role.

## State and transition table

Run states are distinct from baseline plan/task states. Every transition is
version-checked and maintains run/plan fencing in one serialized mutation.
`complete` and `cancelled` are absorbing: parent exit, restart or UI resume
never returns a terminal run to `paused` or restores its authority.

| Run state / event | Next state | Automatic work | Required user authority |
| --- | --- | --- | --- |
| UI-authorized creation | `assessing` | Assess supplied inputs | Reviewed bootstrap/goal/run cap |
| Assessment finds material unknown | `awaiting_clarification` | Present question, retain independent approved work | Explicit answer |
| Current material answers received | `assessing` when the runtime is active and no retained blocker or stage-continue gate remains; otherwise retain the paused/gated state | Record the answer; incorporate it and decide research need only when planning is allowed | None beyond those answers; recovery/continue authority remains separate |
| Research needed | `awaiting_research_approval` | Prepare bounded research proposal | Exact research revision approval |
| Clear task skips research | `awaiting_execution_approval` | Prepare execution proposal | Exact execution revision approval |
| Research approval accepted | `researching` | Request eligible declared researchers/reviewers | Existing approved revision |
| Research report / turn / review arrives | `researching` | Collect report, explicit coordination outcome, advance declared batch when gates pass | Quiescence acknowledgement when reusing a chain worktree |
| Research plan complete | `synthesizing` | Revoke that grant; keep parent running; summarize approved outputs | None to prepare next plan |
| Summary and execution proposal ready | `awaiting_execution_approval` | Display plan and summary | Exact execution revision approval |
| Execution approved | `executing` | Request eligible declared tasks; collect results | Existing approved revision and any reuse acknowledgement |
| Final execution plan complete | `complete` | Revoke run bearer/grants; retain visible results | Manual integration remains separate |
| Completed research, no execution desired (research-only/no-go/decline) | `complete` | Revoke bearer/grants, invalidate pending proposals, retain summary/children/artifacts; before execution approval, settled reservations and no interrupted allocation required | Exact version-bound UI finish decision |
| Failure/blocker/conflict/interrupted allocation | `blocked` | Collect outstanding results; explain the blocker and draft recovery when safe | Resolve any allocation; approve an exact recovery revision before launches |
| Proposal rejected | Same approval-wait state | Show rejection; stop repeated unchanged proposals | Changed proposal needs fresh exact approval |
| Recovery inputs/allocation resolved and recovery proposal prepared | `awaiting_research_approval` or `awaiting_execution_approval` | Bind the proposal to a new input revision; keep launches closed | Exact recovery revision approval |
| Exact recovery revision approved | `researching` or `executing`, matching its declared stage | Create a fresh grant; launch only not-yet-reserved declared tasks | Exact approval and any worktree-quiescence acknowledgement |
| Stage plan cancelled | `paused` with `pauseReason=stage_cancelled` and `stageContinueRequired=true` | Revoke the stage grant; retain the live parent's run bearer for reads, event waits and existing result reports; reject clarification/proposal mutations | Electron-main-authorized, version-bound continue decision before preparing the next proposal |
| Paused stage continued | `blocked` if a retained blocker remains, otherwise `awaiting_clarification` if a current request is unanswered, otherwise `assessing` | Clear only `stageContinueRequired`; preserve blockers and clarification records; reassess and prepare a new proposal only when those gates permit | Electron-main-authorized, version-bound continue decision; no launch until its exact plan revision is approved |
| Parent ends / workspace or sidecar changes | `paused` with `pauseReason=parent_ended`, `workspace_changed`, or `sidecar_replaced` | Revoke bearer/grants; reconcile durable records; preserve run and child state | Exact-identity UI resume in the bound workspace |
| Resumed parent still requires stage continuation | `paused` with `pauseReason=stage_cancelled` | Issue fresh runtime-bound authority for reads, event waits and existing result reports only; retain all blockers and pending clarification | Separate Electron-main-authorized, version-bound continue decision after exact-identity resume; no cancelled plan reapproval |
| Resumed parent has a retained failure/blocker/conflict or unresolved/interrupted allocation, with no stage-continue gate | `blocked` | Restore bounded recovery planning and result collection; preserve blocker and reservation evidence; do not treat interrupted allocation as settled | Resolve the retained blocker/allocation and approve an exact recovery revision before launches |
| Resumed parent has a current unanswered clarification, with no stage-continue gate or retained blocker | `awaiting_clarification` | Restore the same pending request identity/version and preparation binding; no duplicate question or replacement version on resume | Explicit UI answer to that current request; dependent plan approval remains blocked |
| Resumed parent has a previously approved plan with unreserved tasks and its input digest is current | `awaiting_resume_approval` | Restore planning only; retain the stage as the resume target; do not launch | Fresh approval of the exact current plan revision |
| Resumed parent has a previously approved plan with unreserved tasks but its input digest is stale | `assessing` | Keep unused tasks paused; prepare a revised definition against current inputs; do not launch | Exact approval of the revised plan before launches |
| Resumed parent has a live plan but no unreserved task to launch | Its retained `researching` or `executing` coordination state | Restore report/outcome collection with a fresh run bearer and no plan grant | No new approval for collection; any future plan needs its own approval |
| Resumed parent has a never-approved proposal with a current input digest | Its existing `awaiting_research_approval` or `awaiting_execution_approval` state | Restore planning only with a fresh run bearer; retain the proposal and create no grant | Approval of that exact proposal; changed inputs require a new revision |
| Resumed parent is between plans with no pending proposal | `synthesizing` or `assessing`, matching retained inputs/results | Restore planning only with a fresh run bearer | Every future plan needs its own exact approval |
| Exact current plan reapproved after resume | Its retained `researching` or `executing` stage | Create a fresh grant for the same immutable plan revision; do not repeat any reserved task | Exact approval and any worktree-quiescence acknowledgement |
| Run cancelled | `cancelled` | Revoke all authority; retain children/artifacts | New UI-authorized run for new work |

Every recovery, continue, and resume transition compares the current run version
and relevant plan/input digest, then commits the state change and authority
fencing atomically. Recovery revisions preserve prior task attempts and
reservations; a retry is a newly declared task identity in the newly approved
revision, never a second launch of a reserved task. Stage continuation does not
restore the cancelled plan grant. Parent/runtime resume never revives an old
bearer or grant. Choose resume paths in this order: retain a pending stage-continue
gate; otherwise retain any failure/blocker/conflict or unresolved/interrupted
allocation; otherwise restore a current unanswered clarification; only then
classify approved plans, pending proposals or between-plan preparation. All
lower-priority evidence remains durable and visible while a higher-priority gate
is active. A current plan input digest is not evidence that a blocker was resolved.
Resume and same-plan reapproval cannot bypass the exact recovery revision required
for a retained failure/blocker/conflict. `awaiting_resume_approval` has no launch
authority until the current input digest is revalidated and fresh approval creates a new
server-held grant. If it is stale, unused tasks remain paused until a revised
definition is approved. A pending, never-approved proposal
returns to its existing approval-wait state only if its input digest is still
current; otherwise the parent must prepare a new revision. Terminal `complete`
and `cancelled` runs remain absorbing.

Persist `pauseReason` as exactly `stage_cancelled`, `parent_ended`,
`workspace_changed`, or `sidecar_replaced`, plus the prior coordination state
and current plan/input digests needed to choose the resume path. Persist
`stageContinueRequired` independently of `pauseReason`: it starts false, stage
cancellation sets it, and only a successful version-bound UI continue after any
required exact-identity runtime resume clears it. Parent exit,
workspace change, sidecar replacement and repeated interruption may replace the
runtime pause reason but cannot erase that gate, retained blockers, or a pending
clarification's identity/version and preparation binding. Preserve the logical
resume target across repeated pauses rather than replacing it with `paused`.

A `stage_cancelled` pause retains the live parent's run bearer but has no plan
grant. Other pause reasons revoke the bearer and require exact-identity UI resume
before issuing a fresh one. If `stageContinueRequired` remains set, resume restores
the stage-cancelled pause with reads, event waits and existing result reports only;
proposal and clarification mutations stay fenced until a separate explicit
continue. Continue never restores the cancelled plan grant and cannot clear a
blocker or answer a question. Recovery still requires its exact revised approval.

A resumed pending clarification is the same current request, not a new request
or preparation revision. An explicit authorized answer received while paused is
retained and revalidated normally; do not restore an already answered request or
let its answer clear a blocker or stage-continue gate. A resume target records
whether the parent returns to clarification, research/execution coordination,
a pending approval proposal, or between-plan preparation; it never itself grants
a launch.

`awaiting_execution_approval` can display capacity held by older children.
Approval does not remove that wait. `complete` does not mean all PTYs ended,
all worktrees removed, or the user's goal accepted. Research finish records
research-only/declined-execution disposition without labeling the run cancelled.
A terminal run cannot regain planning authority; new work requires a new UI-created
run. Evaluation and integration
remain separate contracts.

## Revision, cancellation and recovery

A changed report version, user answer, review finding, Git base or configuration
creates a new preparation input revision. Proposed plans bind one immutable input
manifest. Revalidation before approval and every launch detects drift; a stale
proposal cannot authorize current inputs. Approved definitions are never patched
in place. Unused tasks on affected plans pause until the user approves a revised
definition; already-launched tasks remain bound to their original assignment.

Late reports for complete/cancelled plans never reopen tasks or restore a grant.
Identical report replays return the stored receipt. A new correction follows [head CAS and atomic publication](#current-report-head-and-corrections),
retains the original, invalidates dependent summaries
and pending proposals, and raises a visible blocker if execution already launched.
It cannot retroactively rewrite a coordination outcome or silently reconfigure
live children. Reports from the wrong child/reservation/generation fail validation.

A rejected proposal leaves the parent live, with planning rights only. It can
incorporate user feedback and propose materially changed work automatically;
it cannot repeatedly resubmit the identical rejected digest or treat elapsed
time as approval. Required new research is another exact approved bounded plan.
There is no automatic retry/review loop or new undeclared remediation task.

On parent death, revoke the parent run bearer/grants; existing live children
may still submit reports through their unchanged scoped capabilities while the
run is paused. Sidecar/workspace change discards every volatile capability and
grant. Startup
reconciliation attaches exact recorded children to reservations or records
`launch_interrupted`; it never relaunches children. The UI may resume the same
OrkWorks parent ID with verified exact native identity, a new runtime generation
and fresh run bearer. Keep the run paused and require approval of the exact
current plan revision before further child launches. If between stages, resume
restores planning only; the future execution plan still needs its own approval.
Unsupported exact resume blocks recovery without a replacement-parent shortcut.

Stopping a run does not automatically kill children, delete worktrees or forget
protected ownership records. Cleanup retains the baseline's clean/quiescent/
plan-owned gates and remains outside the parent's coordination authority.

## Bounds and record behavior

Proposed version-1 admission limits below are UTF-8/serialized bytes. Any lower
baseline, configuration or verified adapter limit wins. Reject excess before
persisting or creating any process. Do not silently truncate requirements or
replace old approval evidence to make room.

| Item | Limit |
| --- | --- |
| IDs / digests | Role contract: 128-byte ASCII IDs / lowercase 64-character SHA-256 |
| Wire counters / generations | Exact encodings below; validate before digesting or comparing |
| Run definition | 16 KiB excluding the separately retained bootstrap, at most 16 plans per run |
| Bootstrap | Role contract's 1 MiB inclusive snapshot/content limit (256 KiB rendered context) |
| Run child ceiling | UI-selected integer 1–16; each plan's ceiling is no greater |
| Plan revisions | At most 64 per plan; baseline 128 tasks / 2 MiB total definition limit |
| Preparation revisions / clarifications | At most 64 revisions and 32 question/answer pairs per run |
| Question / answer / reasons | 2 KiB each; at most four suggestions, 512 bytes each |
| Research questions | At most 32 per research plan; each question at most 8 KiB inclusive scope/criteria references |
| Report | 32 KiB; 32 findings, 64 evidence references, 16 uncertainty/blocker entries, 16 recommendation/limitation entries |
| Evidence reference / finding | 1 KiB / 2 KiB; no inline full artifact |
| Report versions | One stream per exact task/reservation; at most eight versions shared by child/user, including initial publication |
| Report head / receipt metadata | 512 bytes per stream / 2 KiB per version, included in the run aggregate |
| User report corrections | Ten requests per rolling minute per run; duplicates count against rate, not stored versions |
| Research summary | 8 KiB; at most 128 report/review references |
| Input revision manifest | 32 KiB; at most 256 unique record references |
| Event cursor stream | Ring of 256 metadata-only notifications per run, 1 KiB each; report content fetched separately |
| Parent mutations | Ten new proposals/clarifications/result mutations per rolling minute per run; duplicates count against request rate, not stored history |
| Report submission | Ten requests per rolling minute per child; duplicate requests do not increase stored version count |
| Event waits | One outstanding wait per run bearer; maximum 30 seconds, at most 60 requests per rolling minute |
| Run preparation/report metadata | 32 MiB aggregate per run, excluding separately bounded baseline plans/configurations and ordinary terminal history |

### Exact wire counters and generation identities

These encodings apply to every request, response, durable record, capability
binding and event in this proposed preparation protocol. `schemaVersion` is the
JSON integer 1. Numeric counters use plain decimal JSON integer tokens, without
fraction/exponent or leading zeros: `reportVersion` is 1–8; `planRevision` and
`preparationRevision` and `inputRevision` are 1–64; `runVersion`, `taskVersion`, `expectedRunVersion`, `expectedTaskVersion`
and clarification answer/request version counters are 1 through 2^31−1.
`eventCursor` is 0 through 2^31−1; zero means before the first event. Counts,
batch indices and capacity values are nonnegative integers within their stated
collection/capacity limits. Reject negative, out-of-range, fractional, string
or otherwise invalid counter values before persistence/digest/idempotency checks.
A consumer must not coerce or round a value into the valid range.

`sidecarGeneration` and `launchGeneration` are opaque sidecar-issued 256-bit
random identities encoded as lowercase 64-character hexadecimal strings, never
JSON numbers. A new sidecar lifetime gets a fresh sidecar identity; every new or
resumed runtime gets a fresh launch identity. Parent runtime generation uses
that same launch encoding. Compare exact strings, including in grants and child
report bindings. Adapter capability generation is the role contract's bounded
ASCII identity and is not a runtime counter.

The admission fence's `highWaterSequence` and each run's `runSequence` use
canonical decimal strings, not JSON numbers. High-water accepts `"0"` through
`"9223372036854775807"`; run sequence accepts `"1"` through that maximum. No
sign, whitespace, exponent or leading zero is allowed except the single `"0"`.
Validate decimal length/value losslessly (BigInt or checked integer parsing),
never via JavaScript Number. Keep this exact string in canonical digest bytes
and `<admissionEpoch>-<runSequence>` IDs. Admission epoch remains a 64-character
hex identity. This replaces the formerly ambiguous numeric fence representation.

Every counter increments with checked arithmetic. At its maximum, reject the
next mutation and pause affected launches/event automation with a visible
exhaustion blocker; never wrap, reset or reuse a version/cursor. Retained results
and authoritative reads remain available. Recovery requires explicit UI-owned
new-run/revision decisions within existing gates; sequence exhaustion uses the
quiescent namespace rotation below. The 256-event ring's retention window does
not reset its monotonic cursor. Cross-language implementation fixtures must
cover minima/maxima, 2^53+1 input, decimal-string admission sequence maxima,
opaque generations and overflow without truncation. No runtime fixture is run
by writing this specification.

Persist preparation identities, reports and run/plan references; never bearers or
active grants. Event cursor gaps return `refresh_required` and the current run
version; read authoritative state instead of inventing missed events. A duplicate
record ID with identical bytes is idempotent; different bytes require an explicit
new version or return conflict. Persisted limits are revalidated on load;
malformed/oversized ownership state fails closed and pauses launches.

History is repository/workspace-local and protected while children, allocations,
worktrees or referenced plan evidence remain. Deletion/forget cannot remove a
report used by a retained proposal or plan; explicit cancellation/rejection and
reference retirement are required first. Forget revokes the eligible run's
capabilities before removing its unreferenced records.

Use one bounded workspace admission fence instead of accumulating per-run
tombstones: `schemaVersion`, `workspaceId`, `admissionEpoch` (sidecar-generated
256-bit random namespace, lowercase 64-character hex), and `highWaterSequence`
(canonical decimal string `"0"` through `"9223372036854775807"`). Persist this record, at most 4 KiB, under
`~/.orkworks/taskmaster/admission/<workspace-hash>.json`, outside workspace
metadata GC. Run IDs are sidecar-assigned `<admissionEpoch>-<runSequence>`;
the positive sequence is decimal without leading zeros. The run definition and
all producer bindings retain these values. A parent/child cannot select a run ID
or recreate a missing record.

Serialize admission with workspace fencing: durably reserve the next sequence
before creating the run record; a crash may burn a sequence but never reuse it.
Accept operations only for an existing admitted run with matching capability,
identity/generation and plan binding. A missing run at or below the high-water
mark is retired/unavailable, never recreatable through report replay. A future
sequence without a recorded admission is also invalid. Forget leaves the fence
unchanged, so forgetting more than 1,024 runs does not permanently block new work.

If the sequence is exhausted or the fence needs recovery, provide an explicit
Electron-authorized namespace rotation using existing UI authority. It requires
all runs quiescent (no active plans, nonterminal children, reservations or
unreconciled allocations), revokes all run/report capabilities and grants, and
atomically persists a fresh random epoch with sequence `"0"` before admitting
new runs. Retained old-epoch records remain read-only historical artifacts;
old-epoch mutations/replays are rejected without storing an unbounded epoch
list. Worktree/artifact ownership protection is unchanged. Missing or corrupt
fencing/ownership data pauses admission and exposes this recovery or restoration
path; OrkWorks does not guess quiescence or rotate automatically. A crash during
rotation fails closed until durable fence identity is reconciled. No new UI
token, automatic deletion or same-user OS security guarantee is introduced.

Broader retained-history limits/export policy belong to #743/#745 and must
preserve references and this admission fence. Numeric limits are reviewable
defaults, not claims about current storage or tool behavior.

## Example workflows

**Small direct task.** The user supplies a clear correction and acceptance
criteria. The parent records why research/clarification can be skipped and
proposes implementation plus the required verification/review tasks. Approval
of the exact execution plan creates a grant. Declared ready tasks run without
additional launch approvals; final outcomes end the run. The parent never edits
the correction or performs its own review.

**Research followed by execution.** The user asks for a new integration with
unknown delivery and permission support. The parent proposes independent
capability questions and subsequent review tasks. After exact approval, children
research and reviewers inspect the reconciled report. Successful research closes
that plan's grant. The same parent automatically displays a summary and prepares
a new execution proposal bound to the reviewed report digests. No resume button
is required. If support is unverified, it displays the no-go/blocker rather than
proposing an unsupported launch. Execution requires a new exact approval.
Old research PTYs still consume the run ceiling; the next launch waits if full.

**Material clarification.** A requested data import omits whether existing
records may be replaced. The parent asks that question and waits for an explicit
answer before proposing dependent implementation. Elapsed time does not choose
replacement behavior. Independent already-approved research can continue. A
changed answer later invalidates the affected proposal and pauses unused tasks
until the new exact plan is approved.

## Contract verification cases

These are requirements for a later implementation plan; no runtime tests or
live coding-tool probes were run while drafting this document.

| Case | Required result |
| --- | --- |
| Counter outside range / numeric admission sequence / malformed generation | Reject before digesting or stale-write comparison; never round or wrap |
| Clear small task | Direct execution proposal; no ceremonial research |
| Material answer missing/stale/terminal-only | Dependent proposal cannot be approved |
| Parallel question consumes another's result | Explicit dependency/later batch; no concurrent launch |
| Research approved, execution unspecified | Research only; no execution allocation |
| Research-only/no-go summary finished through UI | Exact run-version finish becomes `complete`, revokes bearer/grants and invalidates proposals; no execution allocation or result-acceptance claim |
| Finish races proposal/launch or has unresolved allocation | Serialized current-version fencing; stale finish or unsettled/interrupted allocation denied |
| Research completes with live children | Revoke research grant; same parent synthesizes; retain full run capacity accounting |
| Old research grant / old plan bearer presented | No launch or renewal; never revive an expired credential |
| Run bearer without current exact grant | Planning allowed while active, child launch denied |
| Approve execution with old child filling run ceiling | Grant exists, launch waits; no implicit termination or slot release |
| Unauthenticated/cross-child report / parent run bearer used for ingestion | Reject; only the exact generation-bound child report capability can submit |
| Report event without task-turn receipt | Store report, do not infer task result or advance dependency |
| Required review conflict / unknown capability | Block ready execution proposal; explain missing evidence |
| Initial report / replacement | Null head admits version 1; exact prior version/digest admits only the next version under the same stream/report ID |
| Concurrent child/user corrections from one head | One durable winner; loser conflicts without consuming a version or invalidating twice |
| Successful correction retry after another version wins | Return original receipt before expected task/run/head CAS; do not claim it is still current |
| Same version with different expected head or content / revoked child retry | Conflict / reject before receipt lookup; no publication |
| Accepted receipt replay exceeds the request-rate budget | `429` before receipt lookup; no new version, head change or reinvalidation |
| User correction after final completion/cancellation | Preserve user provenance and original assignment lineage; stale citing evidence, keep run terminal and grants revoked |
| Child resume followed by correction | Same stream and exact head, new child generation/provenance; predecessor runtime cannot replay |
| Corrected research source used by evaluation | Only citing outcomes stale; replacement needs explicit new review, not automatic source substitution |
| Correction racing evaluation/proposal approval/launch | Serialize head validation and dependent fence; cannot expose old evidence as current after publication |
| Crash before/after correction commit or event loss | Complete old/new transaction or fail closed; retry resolves durable receipt, snapshot resolves head |
| Different report ID, ninth shared version or metadata quota | Reject before publication; retain head/history and show exhaustion where applicable |
| Correction racing deletion or admission-epoch rotation | Serialize; preserve referenced history, reject retired identity, never recreate forgotten run |
| Pending proposal report corrected | New input revision invalidates old proposal; approval fails stale |
| Correction after execution launched | Visible blocker; keep original assignments, no automatic rewrite/retry |
| Stable child session ID resumes into a new runtime | Revalidate group ownership/capacity before spawn; reject old-generation report requests before dedupe; retain historical reports; new versions bind current runtime |
| Predecessor resume races/follows successor reservation | Serialized ownership transfer; only one current group owner, predecessor resume denied after transfer even if successor ends |
| Child resumes before acknowledged handoff | Invalidate old terminal/quiescence evidence; next successor needs fresh terminal evidence and user acknowledgement |
| Forget more than 1,024 runs | Admission continues with constant-size epoch/high-water fencing; retired IDs remain non-recreatable |
| Exhausted/corrupt admission fence | UI-only quiescent namespace recovery; reject all old-epoch mutations and preserve artifact ownership |
| Parent receives duplicate events / launch requests | Read current state; one task attempt/reservation |
| Completion/cancellation races launch | Serialized fencing prevents a post-revocation reservation |
| Parent live but model loop stopped | Show continuation unavailable; no terminal typing or new session |
| Clarification answer, proposal rejection, approval or explicit stage continuation while parent waits | Emit a bounded cursor event; parent reads the authorized current record before acting |
| Finish/cancel while parent waits | Revoke authority atomically and return `run_closed`; no follow-up read or launch is accepted |
| Stale clarification answer or stage-continue decision | Reject against the current run/request version; do not mutate state or emit a success event |
| Event wait cursor has fallen behind the bounded ring | Return `refresh_required` and the current run version; parent reads an authorized snapshot and resumes from its cursor |
| Recovery revision approved after a task failure | Return to the declared stage with a fresh grant; preserve prior reservations and never relaunch the failed task identity |
| Stage cancelled, then explicitly continued | Clear only the continue gate; retain `blocked` or `awaiting_clarification` when applicable, otherwise return to `assessing`; no cancelled grant is restored |
| Stage cancelled, then parent ends / sidecar restarts / workspace changes, including repeated interruptions | Revoke runtime authority but preserve `stageContinueRequired`; exact-identity resume restores a gated pause, and a separate version-bound continue is still required |
| Failed task/conflict, then parent ends with current plan inputs and unreserved tasks | Resume stays `blocked`; a current digest and same-plan reapproval cannot replace blocker resolution and exact recovery revision approval |
| Parent ends while awaiting a material answer, with no higher-priority gate | Resume restores `awaiting_clarification` with the same current request identity/version; no duplicate question or preparation revision |
| Clarification pending alongside a blocker or stage cancellation | Preserve all gates across interruptions; answer/continue/resume clears only its own condition; dependent proposals and launches remain fenced |
| Explicit clarification answer received while runtime-paused | Retain the authorized answer and input revision; resume does not resurrect the answered request or clear other gates |
| Parent ends with unreserved tasks in an approved plan | If inputs are current, exact-identity resume enters `awaiting_resume_approval`; if stale, it enters `assessing` to prepare a revised definition; neither path launches before approval |
| Parent ends after a task reservation but before allocation is settled | Exact-identity resume enters `blocked` for explicit allocation resolution before any recovery approval or further launch |
| Parent ends after all tasks are reserved and allocations are settled | Exact-identity resume may collect outcomes with planning authority but no grant; already-reserved tasks are never relaunched |
| Parent end / sidecar restart / workspace change | Revoke credentials, pause, exact identity resume and reapproval |
| New parent bootstrap bytes/skills/settings | Fence old run, UI-authorized new run with reviewed bootstrap; never active configuration mutation or approval transfer |
| Final execution complete with child PTYs live | End run authority; preserve normal child controls/artifacts |
| Oversized record / exhausted history / forgotten-run replay | Reject before persistence/launch; preserve existing records and fences |

## Review handoff — 2026-10-14

**Disposition: ready for written owner review; runtime planning remains blocked.**
This handoff reconciles the specification deliverable with [#742](https://github.com/Rambolarsen/orkworks/issues/742).
It does not record owner acceptance, adapter eligibility, or implementation-plan
approval. The accepted scope/ADR alignment in PR #747 is already delivered;
it is not a remaining prerequisite to redo.

### Acceptance coverage

The following entries identify written requirements, not implemented behavior
or exercised adapter evidence. Owner acceptance of the detailed contract remains
pending even where the requirement is specified.

| #742 requirement | Contract and acceptance evidence |
| --- | --- |
| Assessment inputs, skip criteria and material unknowns | [Assessment and skip decisions](#assessment-and-skip-decisions); clear-task and missing-answer cases in [Example workflows](#example-workflows) |
| Bounded questions, independent decomposition and assignment ownership | [Research question and task contracts](#research-question-and-task-contracts), [Bounds](#bounds-and-record-behavior); result-consuming questions require a dependency/later batch |
| Research approval, separate execution approval and report bindings | [Research approval and result collection](#research-approval-and-result-collection), [Same-parent transition](#same-parent-research-to-execution-transition); a report/summary cannot create launch authority |
| Multi-plan identity, revocation/renewal and sidecar-generation fencing | [Planning authority and capability lifecycle](#planning-authority-and-capability-lifecycle), [Exact encodings](#exact-wire-counters-and-generation-identities); fresh runtime authority never revives an old grant |
| Batches, parent-only delegation, one attempt, quiescent reuse, manual integration and run-wide capacity | [Research contracts](#research-question-and-task-contracts), [Same-parent transition](#same-parent-research-to-execution-transition); completed research PTYs still consume capacity, no cross-plan code/worktree transfer |
| Cancellation, missing/conflicting research, corrections, rejection, scope change and restart/resume | [State and transition table](#state-and-transition-table), [Revision, cancellation and recovery](#revision-cancellation-and-recovery); retained blockers, stage continuation and clarification survive interruption |
| Explicit lifecycle table and three requested examples | [State and transition table](#state-and-transition-table), [Example workflows](#example-workflows) |
| Short summary beside the execution proposal, supporting evidence in child details | [Same-parent transition](#same-parent-research-to-execution-transition); bounded summary binds exact report/review references and leaves sources in research-agent details |
| Version/content-bound graph-planning skill, independent correctness/completeness review and one artifact owner | [Research contracts](#research-question-and-task-contracts), [Role composition](2026-10-04-agent-role-configuration-design.md#role-templates-and-mandatory-skills); missing/conflicting mandatory content blocks eligibility |
| Automatic collection/synthesis/proposal without an inter-stage resume action | [Same-parent transition](#same-parent-research-to-execution-transition), [Automatic event delivery](#automatic-event-delivery); stopped model loops remain unavailable rather than inferred live |
| Run planning bearer survives research; each exact plan has a separate launch grant | [Transport and grants](#transport-and-grants); research completion revokes its grant, final completion/cancellation revokes all authority |
| Immutable parent/plan binding, live-child accounting and exact continuation/resume | [Logical records](#logical-records-and-immutable-parent-configuration), [Same-parent transition](#same-parent-research-to-execution-transition), [State table](#state-and-transition-table); the written contract is specified, while adapter continuation/resume evidence remains unverified under #740 |

### Review evidence and current source seams

PR [#768](https://github.com/Rambolarsen/orkworks/pull/768) merged the clarification,
event-delivery and recovery/resume refinements. Its final independent consistency
review and [Codex review of `bce040e376`](https://github.com/Rambolarsen/orkworks/pull/768#issuecomment-6169348761)
reported no remaining actionable finding after the three lifecycle corrections.
That evidence covers the reviewed changes; it is not owner acceptance of the
whole proposed contract. This handoff reuses it rather than claiming a fresh
review of unchanged requirements.

Source inspection at `7d7e3d0b87b6388202da9a568cfaf6d4791d0525` confirms the
[coordinator records](https://github.com/Rambolarsen/orkworks/blob/7d7e3d0b87b6388202da9a568cfaf6d4791d0525/crates/orkworksd/src/taskmaster/coordinator.rs)
and [coordinator store](https://github.com/Rambolarsen/orkworks/blob/7d7e3d0b87b6388202da9a568cfaf6d4791d0525/crates/orkworksd/src/taskmaster/coordinator_store.rs)
remain explicitly data-only/persistence-only. They do not provide run authority.
The [session application](https://github.com/Rambolarsen/orkworks/blob/7d7e3d0b87b6388202da9a568cfaf6d4791d0525/crates/orkworksd/src/session_application.rs)
retains the session ID through `resume_session_workflow` and calls
`start_session_runtime` for the resumed runtime; `create_session_workflow` remains
the ordinary creation seam. These are existing integration seams, not verified
orchestration, role-restriction or model-loop continuation implementations.

### Remaining gates and exact next actions

| Gate / owner | Current disposition | Evidence required to advance |
| --- | --- | --- |
| Detailed contract acceptance / project owner, #742 with #741 | Written requirements mapped; owner acceptance pending | Record acceptance or concrete changes to this lifecycle and #741's immutable bootstrap/plan/input bindings, including identity, capacity, correction, recovery and numeric bounds |
| Role and continuation eligibility / #740 | No eligible launch slice; all six Copilot profiles remain no-go, as do the six statically assessed Claude Code profiles on the sampled platform | Reviewed version/platform/profile-specific instruction/skill delivery and effective restriction evidence; separately establish same-parent event continuation, gap handling and exact resume, or retain explicit no-go reasons |
| Scoped runtime handoff / #610 consumers | Not executable; no runtime launch plan is authorized | After contract acceptance and an eligible initial slice, prepare PR-sized plans with exact records/routes/migrations, Electron-main/preload interfaces, renderer projections and adapter fixtures; obtain approval before code |

The [#741 handoff](2026-10-04-agent-role-configuration-design.md)
explicitly says not to create a runtime launch plan while the reviewed evidence
yields no eligible profile. The [#740 register](../../validation/agent-role-capabilities.md#decision)
records that no-go; draft probe work and static source/help inspection do not
promote it. This specification task does not authorize live model probes or
remove their separate evidence/resource gates.

The existing [#610 unit boundaries](../plans/2026-09-26-taskmaster-orchestrated-child-sessions.md#runtime-planning-boundaries)
identify the future consumers. #742 supplies the preparation records and bounds
for persistence, the run/grant/resume contract for authority, the clarification/
report/event contracts for continuation, and the summary/blocker/approval
projections for desktop. Those boundaries are not an executable implementation
plan and do not authorize work on an otherwise gated unit.

Keep #742 open for owner acceptance and the remaining scoped handoff. Updating
its definition checkboxes means the written requirements have evidence here;
it must not mark capability verification, runtime implementation or execution
approval complete. If #740 retains no-go, record that blocker explicitly and
leave runtime planning closed rather than inventing a fallback or unsupported
launch plan.

### Proposed hierarchy read transport

The [hierarchy transport proposal](2026-10-17-hierarchy-projection-transport-design.md)
supplies run-scoped snapshot ordering and exact navigation targets around these
records. Its target/source mapping and invalidation coverage remain pending
producer and owner review; no new decision authority or runtime route is
established by that draft.

### Proposed hierarchy producer reconciliation

The [producer responsibility and field mapping](2026-10-20-hierarchy-read-recovery-design.md#producer-reconciliation--2026-10-23)
connects this contract's run/plan/allocation/runtime sources and exact report
stream/head to the shared hierarchy publication boundary. A report correction
must durably publish its existing descriptor/head, receipt, preparation effects
and dependent freshness fences before an affected summary can be current.
Exact retries preserve this contract's receipt-before-head-CAS semantics and
cannot republish a summary transition. No hierarchy read creates a run grant,
changes a report source identity or substitutes a newer version as reviewed
content. Writer/storage integration and producer agreement on the full
material, budget, recovery and detail scope remain pending; the recorded owner
acceptance of four recovery choices does not accept this whole lifecycle or
approve runtime work.

## Execution plan and implementation gate

Keep #742 open: this is a concrete proposed contract, not a reviewed executable
handoff. The [#610 alignment](2026-10-04-taskmaster-orchestration-scope-design.md)
and authoritative spec/ADR scope routing were accepted in PR #747. Before code
planning, finalize and review #741's run bootstrap/plan bindings;
verify #740's parent role, skill delivery, event continuation and exact resume
capabilities or retain explicit no-go combinations. Review the run/grant,
identity, capacity, correction, recovery and numeric-bound contracts together.

The later scoped plan must name exact Rust types/routes/store migrations,
Electron-main approval/preload interfaces, renderer projections, adapter
fixtures and verification cases. Preserve ordinary-session compatibility,
manual integration and both Electron/renderer and runtime ownership boundaries.
No general DAG scheduler, grandchildren, automatic code integration, automatic
retry loop, native confinement or additional UI authority is added here.

## Proposed exact-result reviewer timing — 2026-10-24

The [#744 timing proposal](2026-10-04-assignment-evaluation-design.md#proposed-exact-result-review-approval)
records the owner's 2026-10-24 acceptance of fresh exact-result approval for
the initial child-evaluation flow.
The example workflows above do not establish support for a reviewer of future
#744 output: until a deferred selector is separately specified and accepted,
a worker/reviewer intention cannot become an executable reviewer configuration
without an already accepted target. Research-report review is not automatically
#744 result evaluation and receives no artifact conversion from this proposal.

For future output, the proposed follow-up is a user-created review run with its
own bootstrap and exact review-plan approval after result finalization. Final
execution still revokes the worker run; no planning authority survives solely
to create the review run, and no execution-to-execution transition, cross-plan
worktree reuse or automatic retry is introduced. If the exact result already
exists before a plan is proposed, its reviewer may be declared in that plan.

Before an executable handoff, agree #741's typed immutable target and read
scope, same-workspace retained-result custody/authorization, and stale-input
checks at approval/admission. The linked proposal supplies lifecycle/race cases;
the timing/user-flow choice is accepted while technical producer agreement
remains pending. It does not change accepted ADR 0077 or qualify any #740 profile. Keep #742 and its broader review/runtime gates open.

## Proposed retained review-input admission

This is the proposed #742 consumer of #741's
[typed reviewer configuration](2026-10-04-agent-role-configuration-design.md#proposed-typed-reviewer-input--2026-10-24)
and #744's [retained-input dependency boundary](2026-10-04-assignment-evaluation-design.md#proposed-review-input-custody).
It implements no route or credential today. The accepted timing/user flow is
unchanged; this schema/authority/custody amendment requires owner acceptance.

At proposal publication, derive workspace ownership from authenticated run state,
resolve the exact target and all selected immutable bytes, and validate the v2
configuration/digest, selected repository identity, independence, rubric,
output declaration and byte bounds. A same-workspace source run may be terminal;
that run's old bearer is neither required nor restored. The new parent receives
only descriptor/coordination data authorized by its own bootstrap and UI-selected
review goal, not an arbitrary old-run artifact reader. An unrepresentable
bootstrap input/read scope blocks the review run; the parent cannot investigate
source bytes or expand its startup instructions to prepare the proposal.

Publish the proposal and its #744 retained-reference edges atomically under the
workspace dependency boundary. No plan-owned worktree, branch or child is
created. At exact-plan approval and again before the unique durable child
reservation, validate the immutable target against the current accepted head,
source freshness, custody, configuration/permissions, repository/worktree
bindings, runtime generations, grant, capacity and all existing launch fences.
Serialize these checks and reservation with conflicting source publication,
purge, cancellation and revocation. Recheck before content delivery/spawn;
if a source changes after reservation, preserve the reservation/one-attempt
record, block launch and use existing recovery rules rather than erase it or
retry automatically. Missing/stale input cannot obtain a review-read grant.

`AssignmentReviewReadCapability` is a distinct proposed volatile bearer bound
to the exact reviewer assignment/configuration digest, sidecar/child-launch
generation and the complete immutable `reviewInput`. The server resolves every
read under that binding; request paths/IDs cannot widen it. It permits only the
manifest, criteria/rubric and enumerated snapshot/evidence reads. It cannot list
artifacts, control the worker/source run, submit results/evaluations or launch
children. The parent planning bearer, research-report capability and evaluation
submission capability cannot substitute for this read grant. Exact adapter
action IDs, delivery/envelope schemas, response/rate budgets and safe startup
transport remain separate implementation prerequisites, not implied support.

Issue/deliver this grant only after a successful exact approved reservation and
fresh source/authority checks. Revalidate scope and retained-object integrity
on each read. A newer target head after admission never retargets the grant:
existing freshness rules make its evaluation historical or Unassessed. Reads
of still-retained original immutable bytes remain scoped to that historical
subject; integrity/custody failure denies reads.

**Read/revocation cutoff.** Request acceptance alone authorizes no response.
Prepare a complete bounded response and verify its immutable bytes outside the
authority/dependency boundary. Immediately before committing that response to
the bounded transport handoff, recheck the current capability, generation, exact
scope and custody under the same serialization as revocation. That handoff is
the read's authorization linearization point: revocation first discards the
buffer and denies delivery; response commitment first allows those committed
bytes to arrive even if revocation precedes the client's receipt. Do no blocking
I/O or network write inside this final boundary. Revocation does not wait for
uncommitted reads and prevents all later response commitments. No partial or
streaming response bypasses this check; unsupported response buffering/delivery
bounds keep the adapter no-go. This cannot retract already committed/delivered
bytes from an ordinary coding-tool process or transport.

Follow existing child-runtime/end, run cancellation/final completion,
sidecar/workspace and namespace fences. Exact resume retains the same approved
input, requires current eligibility/freshness checks and fresh generation-bound
credentials, and never reuses an old grant. Retention of the worker result or
proposal does not restore read/launch authority. Resumed or interrupted
allocations keep the existing ambiguity, single-reservation and reapproval gates.

## Proposed output-contract admission

The [#741 output-bearing child/bootstrap versions](2026-10-04-agent-role-configuration-design.md#proposed-structured-output-contract--2026-10-24)
and [#744 byte/rehash accounting](2026-10-04-assignment-evaluation-design.md#proposed-output-byte-and-rehash-accounting)
are proposed consumers of this lifecycle, not runtime support. Version-aware
admission must bind the whole child output contract in its exact approved plan
and the whole root contract at UI-authorized run creation. Derived plan capacity
sums include every child/task without deduplication. Validate declaration/source/
role scope, independent metadata/payload caps and effective adapter limits before
approval/reservation/content delivery; unknown versions or unsupported producer
contracts block admission. A reporting token cannot add declarations or increase
caps after approval. New output-bearing schema versions do not auto-upgrade v1
or v2 records or qualify a coding-tool profile.

Root output scope/limits are immutable across research/execution plans and exact
resume. Changed root declarations need the existing new-bootstrap/new-run path;
child changes require a new exact approved configuration/plan. The candidate
work ledger belongs to #744 and survives this contract's parent/child runtime
end, run completion, cancellation, interruption and generation fences. It grants
no launch/report authority and never resets on resume. A finalization rejection
remains an explicit result-snapshot outcome, not a successful coordination result
or permission to retry/advance undeclared work.

Exact result/evaluation capability envelopes, artifact publication wrappers,
rate/response/candidate-work admission and aggregate storage/worker budgets still
need their scoped contracts and implementation approval. The proposed numeric
limits and ledger introduce no automatic retries, runtime routes, worker
qualification, cross-plan worktree reuse or relaxation of #740's no-go state.
