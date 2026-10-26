---
type: Design Proposal
title: Hierarchy projection payload and read boundary
description: Proposed exact overview DTOs, finite read budgets, authorization and synthetic contract fixtures for hierarchy consumers.
tags: [orkworks, hierarchy, projection, transport, review]
status: draft
---

# Hierarchy projection payload and read boundary

Tracking: [#746](https://github.com/Rambolarsen/orkworks/issues/746), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Status: proposed specification for written owner/producer review. This supplies
the `projection` member of the [transport envelope](2026-10-17-hierarchy-projection-transport-design.md).
It neither accepts the upstream drafts nor authorizes runtime implementation.
#740's capped evidence work remains deferred until November; no profile is eligible.

## Scope and source ownership

Choose a complete overview DTO with bounded references for later inspection.
Embedding canonical configuration/evaluation objects would expose instructions
and require JavaScript to parse large source integers; event deltas would add
another recovery protocol. This overview instead uses validated display data,
exact references and complete collection membership. Detail retrieval and
mutation schemas remain separate contracts, never guessed renderer endpoints.

[#741](2026-10-04-agent-role-configuration-design.md) owns immutable root/child
configuration. [#742](2026-10-04-orchestrator-preparation-design.md) owns declared
plans, allocation, run state and authority. [#743](2026-10-04-skill-usage-evidence-design.md)
owns child evidence. [#744](2026-10-04-assignment-evaluation-design.md) owns results,
candidates and authoritative evaluation. [The mapping](2026-10-15-hierarchy-projection-mapping.md)
defines exact joins. Canonical ordinary session data owns lifecycle/attention;
this DTO copies only a validated display subset, not a second session store.

The source baseline is merged transport PR #912. Current ordinary session enums
were checked in `apps/desktop/src/api.ts`; legacy `status`/lifecycle-phase aliases
are not wire inputs here. The proposed sidecar must resolve authoritative
canonical fields or return unavailable, not apply renderer migration heuristics.

## Common types and strict validation

Every record below has exactly the listed properties. Fields are required;
`null` is permitted only where stated. Reject unknown properties/tags, duplicate
JSON keys, duplicate set members, invalid references, non-finite numbers and
invalid encodings before serving or adopting a complete projection. Values
are plain UTF-8 text, never HTML, Markdown execution, paths or terminal input.

| Type | Exact meaning |
| --- | --- |
| `ID`, `Digest`, `Generation` | #741 ASCII ID (1..128 bytes), lowercase 64-character SHA-256 hex, #742 opaque lowercase 64-character hex respectively |
| `Label`, `Reason` | Nonempty text, at most 512 / 2,048 UTF-8 bytes |
| `Counter` | Plain JSON decimal integer `1..2147483647`; no fraction, exponent or coercion |
| `U64`, `PositiveU64` | Canonical decimal strings `"0".."18446744073709551615"` / same excluding zero; compare losslessly |
| `Time` | Valid UTC RFC3339 with `Z` or `+00:00`, optional 1..9 fractional digits, at most 35 ASCII bytes; preserve validated source precision/spelling; absent time is null, never now |
| `AssignmentIdentity`, `AuthorityGeneration`, `ResultPredecessor`, `EvaluationReference` | Exact tagged #744 records; their bounded numeric revisions stay JSON integers; no added metadata |
| `ResultRef` | `assignmentIdentity`, `resultRevision` (1..32), `resultDigest: Digest` |
| `RoleRef` | `role` from `orchestrator/research/implementation/review/verification/remediation`, `templateId: ID`, `templateVersion: Counter`, `templateDigest: Digest` |
| `CodingToolRef` | `harnessId: ID`, `definitionDigest: Digest`, `label: Label`; immutable #741 harness binding plus validated catalog label/fallback |
| `NodeRef` | `{kind: "root"}`; `{kind: "stage", planId: ID, planRevision: 1..64}`; or `{kind: "task", planId: ID, planRevision: 1..64, taskId: ID}`; workspace/run come only from the envelope |
| `Signals` | `decisionRequired`, `blocked`, `reworkRequired`: booleans supplied by the sidecar for current unresolved work, never inferred from historical text |

A `NodeRef` resolves exactly one node in this projection. Configuration references
and identities must agree with that node, envelope and immutable source. Same
labels/paths never join records. Duplicate actual session ownership, multiple
parents or impossible allocation joins make the run `invalid-structure`.
An explicitly missing ordinary session for a valid owned reference is allowed.

## Run, stage and task records

`RunProjection` has exactly `root`, `coordination`, `capacity`, `stages`, and
`actions`. Arrays retain the declared orders below; no timestamp sorting.

| Record | Exact properties |
| --- | --- |
| `RootNode` | `assignmentIdentity` (root variant), `role: RoleRef`, `codingTool: CodingToolRef`, `label: Label`, `goal` (nonempty exact UTF-8 text, at most 16 KiB), `goalDigest: Digest`, `repositoryId: ID`, `runtime: RuntimeBinding`, `session: SessionDisplay`, `skills: SkillDisplay[]`, `evaluation: EvaluationDisplay`, `signals: Signals` |
| `RunCoordination` | `runVersion: Counter`, `runSequence` (#742 positive decimal string up to `9223372036854775807`), `state` (enum below), `currentStage` (stage NodeRef or null), `clarification: ClarificationDisplay`, `pauseReason` (null or #742 pause reason), `stageContinueRequired: boolean`, `terminal: boolean`, `blockers: Blocker[]` |
| `StageProjection` | `planId: ID`, `planRevision` (1..64), `planDefinitionDigest: Digest`, `preparationRevision` (1..64), `inputDigest: Digest`, `kind` (`research/execution`), `childLimit` (1..16), `disposition` (`proposed/approved/active/paused/completed/cancelled/rejected/superseded`), `terminal: boolean`, `signals: Signals`, `tasks: TaskProjection[]` |
| `TaskProjection` | `taskId: ID`, `configurationId: ID`, `configurationDigest: Digest`, `role: RoleRef`, `codingTool: CodingToolRef`, `label: Label`, `batchIndex` (0..127), `declarationIndex` (0..127), `phase` (`planned/starting/session/session-unavailable`), `allocation: AllocationDisplay`, `allocationRecovery: AllocationRecovery`, `session: SessionDisplay`, `coordination: TaskCoordination`, `dependencies: NodeRef[]`, `skills: SkillDisplay[]`, `evaluation: EvaluationDisplay`, `signals: Signals` |
| `TaskCoordination` | `label: Label`, `terminal: boolean`, `waits: WaitReason[]`, `blockers: Blocker[]` |
| `Blocker` | `code` (`clarification/approval/capacity/dependency/allocation/recovery/evidence/other`), `label: Reason`, `current: boolean`; at most one entry per code per record, preserving all source references in separate details |
| `WaitReason` | `{kind: "task", target: <task NodeRef>}`; `{kind: "batch", batchIndex: 0..127}`; or `{kind: "approval"}`, `{kind: "capacity"}`, `{kind: "quiescence"}`, `{kind: "recovery"}` |

`coordination` is `RunCoordination`. Its state is exactly one of #742's
`assessing`, `awaiting_clarification`, `awaiting_research_approval`, `researching`,
`synthesizing`, `awaiting_execution_approval`, `executing`, `blocked`, `paused`,
`awaiting_resume_approval`, `complete`, `cancelled`. Only the last two are terminal.
Pause reason is null except in `paused`, where it is `stage_cancelled`,
`parent_ended`, `workspace_changed` or `sidecar_replaced`. The independent
stage-continue gate and all lower-priority blockers remain visible under #742.
Current stage is the exact #742 current plan/revision when present, resolves
one stage, and is null only when the source has no current plan.
ClarificationDisplay is exactly `{kind: "none"}` or `{kind: "pending",
questionId: ID, requestVersion: Counter, preparationRevision: 1..64}` from the
current unanswered #742 request. Preserve it while paused/blocked; explicit
answer/correction changes it under #742, never elapsed time or a UI default.
Each stage childLimit copies its exact immutable plan ceiling and is no greater
than the run ceiling. Root runtime
copies #742's current parent runtime binding, with authoritative absence distinct
from unknown. Root label/repository/identity come from the immutable bootstrap.
Root goal is the exact UI-authorized goal retained in #742's immutable run
definition; SHA-256 of its original UTF-8 bytes must equal goalDigest and the
bootstrap goal binding. Preserve whitespace/Unicode bytes without normalization,
truncation or substitution from assignment.label. The source run-definition byte
cap remains a lower limit. RoleRef copies role-template id/version/contentDigest.
Task labels/roles/configuration identities come from that exact #741 assignment.
Root/task CodingToolRef copies `harness.harnessId` and `harness.definitionDigest`
from its immutable bootstrap/assignment AdapterBinding, including before launch.
Label is validated catalog display text or the harness ID if unavailable. Glyphs
use the existing local ID-to-artwork mapping with generic fallback; no arbitrary
URL/path, renderer session lookup or role portrait substitutes for this binding.
The configured identity alone proves neither eligibility nor delivery.
Skill identity/version/digest/requirement copy its exact SkillSnapshot; display
label uses the approved catalog label when available, otherwise the snapshot ID,
never an instruction excerpt. These copies establish no content delivery.
Stage/task terminal flags are authoritative coordination dispositions, not PTY
exit or independent quality. The task label describes that disposition without
asking the renderer to interpret an upstream private state enum.

Stages contain every retained plan revision in source creation order, including
superseded unlaunched work and previously launched paths. At most 16 logical plan
IDs, 64 revisions per plan and 128 tasks per exact revision are allowed. Thus
1,024 stage records / 131,072 task records are structural maxima, **not** a
promise they fit the byte budget. Rejected proposals remain identified history;
no history is copied into another revision. Tasks sort by batch membership then
original declaration order. Indices are declaration coordinates, not contiguous
array offsets. Declaration indices are unique within an exact stage; batch
indices may repeat for tasks in the same batch. Dependencies are
unique task references in that exact approved plan revision, must resolve, and
must form the source-declared acyclic graph. They are not parentage edges.
Wait targets must be declared dependencies. Root role is orchestrator; child
roles are non-orchestrator. No coding-tool-native subagent becomes a node.

### Allocation, session and capacity

`AllocationRecovery` is exactly `{kind: "settled"}`, or `{kind: "unsettled",
reservationId: ID | null, reason: Reason}`, or `{kind: "interrupted",
reservationId: ID | null, reason: Reason}`. These are display metadata keyed by
the enclosing exact stage/task/configuration, not a new source authority record.
One attempt per declared task means at most one unresolved allocation for that
task. A null reservation is allowed only for a durable pre-reservation intent
with AllocationDisplay none; otherwise the reservation matches reserved/bound
allocation exactly. Bind each source intent to that declared task before exposing
it; missing/ambiguous mappings make the run invalid-structure, never anonymous.

Each unsettled/interrupted task has a current allocation blocker, a recovery wait
and blocked signal so Show next action can reveal it. Preserve those flags even
in cancelled history while ownership remains unresolved. Stage completion and
run complete/research-finish cannot coexist with unresolved allocation; cancelled
coordination may retain it and still cannot auto-collapse or silently clean up.
Resolution must be explicit and source-validated under #742, not a count decrement
or text edit. No automatic retry/new reservation is granted by inspection.

`AllocationDisplay` is exactly one of:

- `{kind: "none"}`: no reservation or launched assignment.
- `{kind: "reserved", reservationId: ID, taskVersion: Counter}`: no bound child yet.
- `{kind: "bound", assignmentIdentity: <child AssignmentIdentity>, launchGeneration: Generation, runtime: <RuntimeBinding>}`.

`RuntimeBinding` is `{kind: "current", generation: Generation}` or
`{kind: "absent"}`. Absent means authoritative absence; unknown/corrupt runtime
state instead makes the projection unavailable. It never permits replacement.
A bound child identity matches the stage/task/configuration/root parent exactly.
`phase` is planned for none, starting for reserved, and session or
session-unavailable for bound depending on its ordinary session reference.
Missing started session records remain bound, never revert to planned.
Bound `launchGeneration` is the latest admitted child launch, retained after it
ends; a current runtime generation must equal it. Retained evidence may bind
that terminal launch until a later resume replaces the binding. No write grant
is implied by retaining a generation.

`SessionDisplay` is exactly `{kind: "none"}` (unallocated task only),
`{kind: "unavailable", sessionId: ID}`, or:

`{kind: "available", sessionId: ID, lifecycle, attention, memoryState,
connectivity, terminalOutcome, lastActivityAt}`.

Lifecycle is `creating/alive/stopping/dead`; attention is null or
`working/idle/needs_you/blocked/failed/capped`, and must be null unless alive.
Memory state is `live/remembered/resumable/unsupported`; connectivity is
`online/offline`; terminal outcome is null or `ended/killed/error` and must be
null unless dead. `lastActivityAt` is Time or null. Copy canonical values;
missing optional source fields become null only where allowed, never invented
lifecycle/connectivity. Session IDs match the root/child identity. Root session
may be unavailable but never none. None/reserved allocations have no evaluation
identity or ordinary session. Read-only selection/history navigation is separate
from resume options, which remain in the ordinary session contract.

Capacity is `{kind: "unavailable"}` or exactly
`{kind: "known", runLimit: 1..16, liveChildren: 0..16,
unattachedReservations: 0..16, unresolvedAllocations: 0..16, currentPlan}`.
CurrentPlan is exactly `{kind: "none"}` when coordination.currentStage is null,
or `{kind: "known", stage: <stage NodeRef>, limit: 1..16,
liveChildren: 0..16, unattachedReservations: 0..16, availableSlots: 0..16}`.
Its stage equals coordination.currentStage and its limit equals that stage's
childLimit; every plan limit is no greater than runLimit. All count equalities
below are validated by the sidecar under the coherent read boundary.

Known run liveChildren equals the distinct bound task sessions whose validated
canonical lifecycle is nonterminal across every retained stage. Known
unattachedReservations equals the distinct visible reserved allocations.
Known unresolvedAllocations equals the distinct task keys whose AllocationRecovery
is unsettled/interrupted, including pre-reservation intents. Reconcile every
ledger entry to a visible exact task before claiming complete: an unmapped child,
reservation or intent is invalid-structure for the run, never an anonymous count.
An explicitly owned but unavailable child session keeps its task visible and makes
capacity unavailable until lifecycle is reconciled. Count mismatches cannot be
served as known. Run liveChildren plus unattachedReservations cannot exceed
runLimit; an ambiguous/excess count yields unavailable capacity, never zero.

CurrentPlan child/reservation counts equal the same validated sets restricted
to its exact plan/revision, not all previous plans. Their sum cannot exceed its
limit. AvailableSlots is the checked nonnegative minimum of remaining run slots
and remaining current-plan slots. Prior research children consume run slots but
do not become execution-plan children. Example: run limit 4 with two prior-plan
children and an empty current execution plan capped at 2 has two slots; two
current-plan children instead exhaust that plan even though the run ceiling is 4.
Expose both ceilings/counts; do not compare all run children only to the smaller
plan ceiling. Capacity availability is not approval/readiness: unresolved
allocation, dependency, grant and capability gates remain independent.

Root is not a child slot. A complete run payload may show unavailable capacity,
but cannot claim readiness or auto-collapse from it. Count all prior-plan
nonterminal children and reservations even outside active coordination.

## Evaluation summary and references

`EvaluationDisplay` is `{kind: "not-started"}` only for none/reserved allocation,
`{kind: "unavailable", reason: <Label>}`, or exactly:

`{kind: "known", assignmentIdentity, accepted, candidates, summary, evaluations}`.

`accepted` is `{kind: "none"}` or `{kind: "result", reference: ResultRef}`.
`summary` is exactly `{kind: "unavailable"}` or
`{kind: "known", summaryRevision: <#744 decimal string "1".."2147483647">,
overall: "meets-requirements" | "needs-rework" | "unassessed"}`.
The sidecar supplies both current material summary and durable revision under
#744. Missing/corrupt/exhausted lineage yields unavailable summary, not a token
or old passing summary. Accepted result/review references may remain inspectable;
summary unavailability prevents viewed identity and automatic collapse.
`evaluations` is a unique array of at most 16 exact `EvaluationReference` values
for the accepted head, sorted by canonical reference bytes. Each reference binds
this assignment and exact result, including independent reviewer identity.
With no accepted result, that array is empty and a known summary cannot say meets-requirements.
Known required failure may still produce needs-rework through #744; the renderer
never recomputes this from exit, absence, candidate rejection or self-report.

`candidates` contains at most 32 exact records with `candidateDigest: Digest`,
`authorityGeneration: AuthorityGeneration`, `predecessor: ResultPredecessor`,
`state` (`staged/rejected/unsupported/superseded/abandoned/accepted`). Sort by candidate digest;
include every retained candidate's disposition, not a guessed latest candidate.
Candidate identity is the enclosing assignment plus its exact digest and generation.
Candidate success cannot transfer a previous result's review to staged bytes.
Explicit Electron-authorized abandonment is a distinct retained terminal
candidate disposition under #744; it is not rejection, supersession, accepted
result, automatic quality failure or permission to relaunch.
Required failures win over missing evidence; optional-only stale details do not
downgrade a pass. Source freshness and disputes are validated by #744 before
projection. Root evaluation uses user review; child references must resolve
eligible independent reviewers or the user. Reviewer performance is separate.
Completeness/quality scores, outcomes, findings, evidence and permissions are
inspection details, resolved from these exact references by separately reviewed
read operations. This overview intentionally supplies no write/correction route.

## Skills and lossless evidence metadata

Every selected skill is present in immutable configuration order (0..16):
`SkillDisplay` has exactly `skillSnapshotId: ID`, `skillVersion` (nonempty UTF-8,
128 bytes), `skillContentDigest: Digest`, `label: Label`, `requirement`
(`mandatory/optional`), and `evidence`.

Evidence is `{kind: "root-deferred"}` for roots only;
`{kind: "not-started"}` for none/reserved allocations only;
`{kind: "unavailable", reason: <Label>}` for a bound child whose optional
telemetry cannot be validated; or exactly
`{kind: "current", binding: SkillBinding, delivery, reported, observed, events}`.
Unavailable evidence renders selected/unconfirmed/unknown, not numeric zero.
Invalid immutable skill identity fails the run; unavailable optional telemetry
does not remove an otherwise valid task. Root delivery/usage remains deferred;
mandatory parent delivery eligibility is still required before launch/resume.

`SkillBinding` copies exactly #743's `runId`, `planId`, `planRevision`, `taskId`,
`taskVersion`, `reservationId`, `parentSessionId`, `configurationId`,
`configurationDigest`, `childSessionId`, `sidecarGeneration`, `launchGeneration`,
`adapterGeneration`, `evidenceStoreGeneration`. Numeric bounds and generation
namespaces remain #743's. Shared run/plan/revision/task/reservation/parent/child/
configuration members and the latest admitted launch must match, including
retained terminal-launch evidence validated by the sidecar. The same-named
`taskVersion` values have different owners: #744's child identity copies frozen
#742 `assignmentTaskVersion`; #743's binding keeps its task-state/launch-binding
version. Validate each against its own authoritative source. Their equality is
not a join condition, and neither value repairs an absent or invalid other
binding. Do not rewrite a retained #744 identity/result digest when task state
or a skill binding changes.
Current binding sidecarGeneration also equals the envelope lifetime; retained
old-lifetime receipts stay historical in details. It cannot decorate a current
badge after resume with old-launch evidence.

Delivery is `{state: "selected_unconfirmed"}` or
`{state: "loaded", eventId: <EventID>, mechanism: Label}`. Loaded requires an
accepted current verified-adapter delivery receipt for the exact skill closure.
Reported/observed usage cannot establish delivery. EventID is nonempty UTF-8 text
up to 128 bytes, validated under #743's event identity contract, not #741's ID regex.

Reported and observed are independent `UsageDisplay` records, each exactly:

- `{state: "unknown", coverage}`; or
- `{state: "recorded", count: U64, countKind: "recorded" | "lower_bound", lastAt: Time | null, coverage}`.

Coverage is `complete/partial/interrupted/unsupported/unknown`. The field name
fixes the source: reported means assigned-session self-report (no verified process
origin), observed means a verified adapter-native recognized invocation with its
declared coverage. Unsupported has unknown state. Positive counts require a
server receipt time; zero requires null lastAt and an explicitly counted stream,
not an absent producer. Partial/interrupted counts are lower bounds. Unknown
usage never means unused. Full coverage windows/source declarations remain in
authenticated evidence details, never generalized beyond their source intervals.

`events` is an array of unique current accepted reported/observed usage references,
each exactly `eventId: EventID`, `workspaceSequence: PositiveU64`, `source`
(`reported/observed`). Order by durable workspace sequence; at most 256 events
across all skills of one assignment. It is retained usage references, not complete
history or delivery receipts. Sidecar alias/correction logic deduplicates logical
invocations before projecting references/counts. Cross-skill event identities
include the skill snapshot; the same event is not counted twice for one skill.

The proposed #743 refinement makes workspace sequence a checked positive u64
source integer and aggregate counts checked nonnegative u64 source integers.
Display values are decimal strings **before** JavaScript parsing, as in #912;
producer stream sequences remain bounded JSON integers. Neither source nor
projection counter wraps. Source exhaustion fences new evidence/aggregation,
retains authoritative readable history and exposes unavailable current telemetry
where reconciliation is needed. It cannot reset counts or establish non-use.
This numeric contract needs #743 owner acceptance; it is not capability evidence.

Initial snapshot/return after a gap sets the local per-binding sequence watermark
without replaying historical highlights. Later strictly new accepted usage
references may highlight once; polling/duplicate references and correction-only
count changes do not. View-state keys include exact assignment, skill and evidence/
launch generation. No timestamp sorting or count-delta fabrication is permitted.
A count can remain available after raw event trimming; an empty reference array
never claims an empty stream or complete coverage.

## Actions, rollups and completeness

`actions` is a unique array of `ActionEntry`, exactly `node: NodeRef` and
`navigation: <#912 NavigationDescriptor>`. Its descriptor is the exact `kind`,
`target`, `label` shape in the transport proposal, not an opaque command or URL.
Run/clarification/plan/task subjects resolve root/root/stage/task respectively and agree with envelope,
current run version and all source bindings. Plan targets copy that stage's
revision/definition/input/preparation values; task targets copy its complete
bound identity and latest launch; run targets copy the root parent/runtime.
An answer-clarification action always targets NodeRef root and copies the exact
pending questionId/requestVersion/preparationRevision from coordination.clarification.
No pending request means no clarification action. Root decisionRequired remains
true for a currently required answer, even while a higher-priority blocker is
present; a missing applicable clarification descriptor makes the run unavailable.
The source decides applicability;
missing decision bytes yield unavailable action state, never an empty array.
Actions sort root first, then source stage/task order, then descriptor kind bytes.
They inspect decisions only; commands need their own fresh authorized validation.

No aggregate rollup numbers are sent: derive distinct live needs-you session IDs,
blocked node references and decision tuples from the complete validated payload;
these categories overlap and are never summed. `signals` identify only the node
its record belongs to (not inherited descendant rollups) and current work,
so superseded historical failures do not become current blockers. All current
required decisions must have descriptors; if decisionRequired is true but no
matching action exists, the entire run is unavailable. Relevant live attention
remains discoverable even without an orchestration descriptor: Show next action
may reveal its exact node without selecting/acknowledging the session.
Blocked/required-rework signals and local unviewed summaries also reveal their exact
nodes; they do not manufacture an upstream decision descriptor or command. Local
unviewed-summary counts use exact #744 summary tokens, independently of unread.
A complete projection is not approval, readiness, capability eligibility, workspace-
wide completeness or user acceptance. No actions is only a current run claim.

Auto-collapse consumes coordination terminal flags, all current summary tokens,
actions/signals, authoritative capacity and canonical nonterminal sessions;
unavailable evidence needed for these checks prevents collapse. Focus/inspection
and viewed/reopening suppression remain bounded UI state under #746. The sidecar
never acknowledges unread by serving a snapshot. A complete payload may contain
visible unavailable session/evidence/summary fields explicitly allowed above;
unknown ownership, missing required decisions or partial collection scans cannot.

## Read authorization and finite budgets

Renderer-to-preload request is exactly `{schemaVersion: 1, runId: ID}`.
Electron main validates the request, its owned application window and main-frame
sender against the existing app URL/origin boundary, then derives the currently
selected workspace and owned sidecar lifetime, and sends exactly
`{schemaVersion: 1, workspaceId: ID, runId: ID, sidecarGeneration: Generation}`
to the logical `ReadHierarchyProjection` operation. Concrete route/IPC names
remain implementation-plan decisions. The sidecar requires the existing
Electron-only UI authority (`ORKWORKS_OPEN_PLAN_TOKEN`) before parsing source
references, admission lookup or allocating snapshot slots. Parent/child bearers,
unauthenticated ordinary routes and renderer input cannot authorize this read.
No new token is introduced, returned, persisted or logged.

Revalidate selected workspace/lease and generation at read publication and in
Electron before forwarding the result. The renderer additionally checks current
selection/request context under #912. Denied auth/scope reads expose no run
existence or partial projection; they are request failures, not not-found
snapshots. Only an authorized current-workspace read can get not-found/null.
The existing same-user bearer replay limitation remains; this is not OS
process-origin proof. Background instances and arbitrary workspace paths cannot
be selected by this request. Ordinary terminal data/selection remain independent.

| Resource | Proposed version-1 bound |
| --- | --- |
| Renderer/main or main/sidecar request | 1 KiB serialized UTF-8; reject duplicate keys/unknown properties |
| Complete envelope, including projection | 8 MiB serialized UTF-8, maximum nesting depth 32 |
| Unavailable envelope | 1 KiB serialized UTF-8, fixed reason enum only |
| Stage/task/skill collections | 16 plan IDs × 64 revisions × 128 tasks, 16 skills per node; byte cap wins |
| Per-node candidate/evaluation/usage references | 32 / 16 / 256 across skills, with upstream lower limits winning |
| Dependencies/waits/blockers | 128 unique task dependencies / 133 unique waits / 8 blocker codes per record |
| Navigation descriptors | At most 7 per node; unique complete kind/target tuples; envelope byte cap wins |
| In-flight sidecar overview reads | At most 2 per selected workspace; coalesce identical reads in Electron |
| Attempts | 60 per rolling minute per selected workspace under valid UI authority, including malformed/replayed/failed reads |
| Assembly | At most 2 coherent assembly attempts, 5-second deadline, byte/depth/count checks while accumulating |
| Snapshot counter metadata | #912's 1,024 slots / 512 KiB per workspace/lifetime; unknown IDs allocate none |

Reserve a read slot before source assembly and release on completion, cancellation
or deadline. Check auth, then charge attempt budget before JSON decoding/lookup;
transport enforces the 1 KiB byte ceiling while reading. Rate/concurrency refusal
is a request failure and must disable stale decisions. Oversized source structure
returns versioned limit-exceeded for a tracked run; never truncate, paginate a
complete claim, drop conflicts/actions, or mutate history to fit. On assembly
races/deadline return reconciling (or request failure if no valid scope remains).
Do not return complete once the owned lifetime/workspace lease is invalidated.
A legitimate maximum source run may exceed 8 MiB and be visibly unavailable;
future paginated history needs separate completeness/selection semantics.

The response builder counts bytes incrementally; it cannot first allocate an
unbounded object and check size afterwards. Read size/depth/count budgets also
apply to Electron/renderer decoding before adoption. The exact UI-authorized goal
is permitted local display content, distinct from harness startup prompts. Do not
read transcript, raw harness prompt/instruction, credential, arbitrary file
content, full evidence or repository history to build this overview. Permitted labels/receipt metadata
remain local authorized display data. Log bounded reason codes/counts, not body
bytes or secrets. Details remain behind an authenticated exact-reference read
contract; a DTO reference is never a filesystem capability.

## Concrete synthetic fixtures

The JSON below is a shape-valid complete envelope for a synthetic admitted run
with an unavailable parent session and no plans yet. Hex values are explicit
fixture identities, not claimed production hashes or capability observations.
Its synthetic source manifest declares the matching immutable bootstrap/root,
role template, repository ID, no accepted result/candidate/evaluation, durable
summary revision 1 and paused/parent-ended coordination. No missing source identity is
inferred by the fixture. A future executable fixture must materialize that source
manifest and verify all joins/digests separately from shape validation.

```json
{
  "schemaVersion": 1,
  "workspaceId": "workspace-fixture",
  "runId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1",
  "sidecarGeneration": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
  "snapshotVersion": 1,
  "state": {
    "kind": "complete",
    "projection": {
      "root": {
        "assignmentIdentity": {
          "kind": "root",
          "workspaceId": "workspace-fixture",
          "runId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1",
          "rootAssignmentId": "root-fixture",
          "parentSessionId": "parent-fixture",
          "bootstrapId": "bootstrap-fixture",
          "bootstrapConfigurationDigest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        },
        "role": {
          "role": "orchestrator",
          "templateId": "role-fixture",
          "templateVersion": 1,
          "templateDigest": "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
        },
        "label": "Synthetic workflow",
        "goalDigest": "4e020b046d87dcbcfa44777687f13c1ccc17a51d8c256ad778e4071cc5660bd8",
        "repositoryId": "repo-fixture",
        "runtime": {
          "kind": "absent"
        },
        "session": {
          "kind": "unavailable",
          "sessionId": "parent-fixture"
        },
        "skills": [
          {
            "skillSnapshotId": "skill-fixture",
            "skillVersion": "v1",
            "skillContentDigest": "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "label": "Graph planning",
            "requirement": "mandatory",
            "evidence": {
              "kind": "root-deferred"
            }
          }
        ],
        "evaluation": {
          "kind": "known",
          "assignmentIdentity": {
            "kind": "root",
            "workspaceId": "workspace-fixture",
            "runId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa-1",
            "rootAssignmentId": "root-fixture",
            "parentSessionId": "parent-fixture",
            "bootstrapId": "bootstrap-fixture",
            "bootstrapConfigurationDigest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
          },
          "accepted": {
            "kind": "none"
          },
          "candidates": [],
          "summary": {
            "kind": "known",
            "summaryRevision": "1",
            "overall": "unassessed"
          },
          "evaluations": []
        },
        "signals": {
          "decisionRequired": false,
          "blocked": false,
          "reworkRequired": false
        },
        "goal": "Plan a synthetic workflow.",
        "codingTool": {
          "harnessId": "fixture-tool",
          "definitionDigest": "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff",
          "label": "Fixture coding tool"
        }
      },
      "coordination": {
        "runVersion": 1,
        "runSequence": "1",
        "state": "paused",
        "currentStage": null,
        "pauseReason": "parent_ended",
        "stageContinueRequired": false,
        "terminal": false,
        "blockers": [],
        "clarification": {
          "kind": "none"
        }
      },
      "capacity": {
        "kind": "known",
        "liveChildren": 0,
        "unattachedReservations": 0,
        "unresolvedAllocations": 0,
        "runLimit": 2,
        "currentPlan": {
          "kind": "none"
        }
      },
      "stages": [],
      "actions": []
    }
  }
}
```

Apply each row independently to a fresh copy of the base fixture. Paths are
JSON Pointers; changing a leaf replaces it, and removing a property means it is
absent rather than null. Source-validation cases require the stated synthetic
source change; transport-shape acceptance alone never establishes a valid join.

| Mutation / additional fixture | Required disposition |
| --- | --- |
| Base fixture unchanged | Complete paused run; unavailable parent; root skill selected/unconfirmed/unknown; no replacement or automatic resume |
| `/state/projection/root/evaluation/summary` → `{ "kind": "unavailable" }` | Valid unavailable summary; preserve exact retained references, prevent auto-collapse |
| `/state/projection/root/skills/0/evidence` → `{ "kind": "current" }` | Reject: root telemetry is deferred and current evidence is missing required fields |
| `/state/projection/root/assignmentIdentity/runId` → `"another-run"` | Invalid-structure: envelope/root source identity mismatch |
| `/state/projection/root/evaluation/summary/summaryRevision` → `1` | Reject numeric summary token; decimal string required |
| `/state/projection/root/evaluation/summary/overall` → `"meets-requirements"` | Reject: no accepted result or current required review supports pass |
| `/state/projection/root/signals/decisionRequired` → `true` with actions still empty | Run action state unavailable; cannot claim No actions |
| `/state/projection/capacity/liveChildren` → `3` while runLimit is 2 | Reject known capacity; source builder must instead emit `{ "kind": "unavailable" }` |
| `/state/projection/coordination/runSequence` → `"01"` | Reject noncanonical decimal sequence |
| Add `token` or raw `instructions` to root | Reject unknown properties; never forward secrets/instruction bodies |
| Remove `/state/projection/stages` or use null | Reject; empty complete collection must be explicit array |
| Root goal removed, replaced with label, normalized, or hash mismatches goalDigest | Reject: exact immutable authorized goal/binding required |
| Root goal contains multiple lines/Unicode and is within source/16 KiB caps | Preserve exact UTF-8 display bytes and matching SHA-256; no normalization/truncation |
| Session time is `2026-10-17T08:00:00+00:00` or `2026-10-17T08:00:00.123456789+00:00` | Accept valid canonical UTC source spelling/precision, at most 35 ASCII bytes |
| Time has an invalid calendar value, unknown/nonzero UTC offset, or more than nine fractional digits | Reject display Time; narrower producer timestamp rules still apply |
| Retained pending candidate is explicitly user-abandoned | Include state abandoned; preserve accepted head/review; no relaunch or inferred quality failure |
| Ledger live count is 1 but complete payload has no bound nonterminal child | Reconcile missing ownership; run unavailable if child has no mapped task; never known anonymous slot |
| Known count differs from visible bound sessions/reserved allocations, or child lifecycle is unavailable | Reject known capacity; show unavailable while reconciling, preserving valid owned rows |
| Root session becomes available/alive/needs_you in a coherently advanced snapshot | Valid source-backed attention; root needs-you count changes without selection/unread acknowledgement |
| Exact retained plan has two tasks in batch 0 with declaration indices 0 and 1 | Valid ordering; same batch is allowed, duplicate declaration index/task ID is not |
| Two retained revisions reuse a task label/ID under distinct planRevision values | Separate nodes and assignment/result/skill/viewed state; never replace the earlier path |
| Valid bound child has accepted result and staged candidate, but canonical lifecycle is alive | Both result and pending candidate remain distinct; child still consumes capacity and prevents group collapse |
| Child identity keeps admission taskVersion 1; independently valid #743 evidence binding has taskVersion 2 with the same shared identity and current launch | Valid join when each version matches its own source; no forced equality, identity rewrite or transferred delivery proof |
| Evidence taskVersion matches the #744 identity but differs from its authoritative #743 binding | Reject current evidence; matching the frozen result identity is insufficient |
| Valid bound child's latest launch changes while same session ID remains | Old binding cannot confirm current skills or supply a task navigation target; fresh sidecar snapshot required |
| Root/stage/task has the same actual session ID as another owner | Invalid-structure, rather than duplicated canonical session or guessed parentage |
| Child usage count `"9007199254740993"` / sequence `"18446744073709551615"` in valid current binding | Exact strings survive DTO/IPC; narrower source count limits still win; no Number conversion |
| Same usage fields become numbers, overflow strings or leading-zero strings | Reject before adoption; do not round/coerce |
| Child has counted zero with null lastAt and explicitly known producer coverage | Display zero for that counted interval only; never infer unused |
| Partial/interrupted usage supplies countKind recorded | Reject; the known count must be labeled lower_bound |
| Latest event payload was trimmed, aggregate count remains valid, events is empty | Show authoritative source-labeled count, no manufactured highlight or absence claim |
| Current action descriptor references another workspace/run/version/node | Invalid-structure; cannot offer stale/foreign decision navigation |
| Source contains 65 revisions for one plan or duplicate stage keys | Limit-exceeded/invalid-structure respectively; never truncate to 64 |
| Source-valid complete run exceeds 8 MiB after encoding | Versioned limit-exceeded; preserve history and ordinary session access |
| Auth fails, stale workspace lease, wrong generation, or parent bearer attempts read | Request failure before run existence disclosure or snapshot-slot allocation |
| Unresolved intent has no mapped task, or its count differs from per-task recovery records | Run invalid-structure for unmapped ownership / reject known count; never hide behind anonymous capacity |
| Interrupted intent precedes reservation | Visible exact task with recovery kind interrupted, null reservation, current blocker/wait; no invented session/reservation or automatic retry |
| Cancelled stage retains an interrupted allocation | Preserve recovery node/signal/count; no auto-collapse or cleanup; completed stage/run cannot claim settlement |
| Run ceiling 4, current plan ceiling 2 with two current-plan live children | Display run 2/4 and plan 2/2, zero availableSlots; plan limit still gates launches |
| Run ceiling 4 with two prior research children; current plan ceiling 2 has none | Display run 2/4 and current plan 0/2, availableSlots 2; no reassignment of prior-plan children |
| Pending clarification under paused/blocked run | Exact root action with matching pending request identity/version/preparation; preserve other blockers |
| Clarification action targets a stage/task or stale/nonpending question | Invalid-structure; never arbitrary placement or stale answer navigation |
| Planned/reserved child has no session | Coding-tool glyph/label derives from immutable harness binding; unknown artwork gets generic fallback |
| CodingToolRef differs from that node's exact configuration binding | Invalid-structure; no renderer inference from role/label/another session |
| Third concurrent read / 61st attempt in rolling minute | Bounded refusal; release/cancel completed slots; no body scan or stale decision success |
| Source changes during both allowed assembly attempts | Reconciling or request failure under invalidated scope; no mixed complete result |

These include shape-valid unavailable cases as well as invalid mutations.
Existing #912 fixtures still govern ordering/exhaustion/failure fences. Existing
#746 fixtures govern keyboard, return navigation, summaries/collapse and reduced
motion. Concrete launched-child/evaluation codec fixtures must expand all source
references in the executable plan; the logical rows above are not production
observations or a runtime validation claim.

## Review and implementation handoff

This proposal supplies the overview shape, selected-source mapping, finite read
budgets and authority boundary. Before treating it as accepted, review the exact
stage/revision membership and allocation/session joins with #741/#742; child
evidence counters/intervals with #743; accepted/candidate/summary references with
#744; and rollups/navigation with #746. Source proofs, current-action completeness
and canonical digest validation are prerequisites to serving complete, not renderer
heuristics. Root telemetry remains deferred.

Remaining execution decisions are concrete routes and independent Electron-main/
preload/renderer definitions, coherent-read/invalidation transactions, native
capability evidence, durable summary material bytes/storage/recovery, and exact
inspection-detail codecs. Workspace run discovery/paginated-history completeness
and cross-workspace custody remain separately scoped. This does not create a
runtime launch plan while #740 remains no-go. Record written owner/producer
acceptance before reconciling canonical drafts and seeking separately approved
runtime plans. Keep #746 open; passing documentation checks is not that acceptance.

## Proposed read and recovery follow-up

The [coherent-read, summary-recovery and detail-codec proposal](2026-10-20-hierarchy-read-recovery-design.md)
supplies a proposed joint publication boundary, explicit summary material and
storage budgets, crash/purge cases, and exact-byte result/evaluation inspection.
The owner [accepted four recovery-contract choices on 2026-10-23](2026-10-20-hierarchy-read-recovery-design.md#owner-acceptance--2026-10-23).
Producer agreement remains pending for all four choices, including material
fields/classifications, summary budgets, publication/recovery semantics and
narrow detail scope. Broader projection acceptance, runtime storage
qualification, capability evidence and executable-plan gates remain open.

## Contract review — 2026-10-26

The [production projection review](../../validation/hierarchy-projection-contract-review.md)
separates frozen evaluation identity from skill task-state versions. It also
records that the closed overview summary currently returns only overall result
and revision, while the accepted narrow record read supplies only whole-record
freshness. Neither supplies authoritative current per-outcome/finding details or
aggregate completeness/quality. Do not label raw reviewer claims as those current
aggregates or mark a full material summary viewed from record inspection alone.
A scoped read-projection proposal or explicit first-slice deferral is required
before that UI/detail handoff is executable. This note adds no wire fields, read
route, quota change or runtime authority.

## Current-summary detail proposal — 2026-10-26

The [#1089 current-summary detail proposal](2026-10-26-hierarchy-summary-detail-design.md)
addresses the missing authoritative outcome/finding and completeness/quality read
identified by the production projection review. It proposes a separately bound
read and explicit complete material display before local viewed marking, within
the existing detail budgets. Source/material codecs, summary revision semantics
and the narrow immutable-record read are unchanged. The new response/display/
viewing choices await scoped owner/producer review; no runtime handoff or broader
projection acceptance follows from merging the proposal.
