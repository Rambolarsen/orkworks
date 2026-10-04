# Clarification and research-to-execution lifecycle

- Status: proposed contract; continuity direction approved, written contract review pending
- Date: 2026-10-04
- Tracking: [#742](https://github.com/Rambolarsen/orkworks/issues/742), parent [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Product direction: [Agent hierarchy and configuration learning](2026-10-04-agent-hierarchy-and-configuration-learning-design.md)
- Launch baseline: [Taskmaster orchestrated child sessions](2026-09-26-taskmaster-orchestrated-child-sessions-design.md)
- Configuration: [Role configuration](2026-10-04-agent-role-configuration-design.md)
- Eligibility: [Capability evidence register](../../validation/agent-role-capabilities.md)

## Scope and status

Keep the same orchestrator running from goal assessment through clarification,
approved research, preparation of execution, and approved execution. The user
chose continuity and a high degree of automation. The parent automatically
collects available reports, synthesizes the research summary, proposes the
execution plan, and requests ready declared tasks after approval. There is no
mandatory pause/resume action between successful research and plan preparation.

Automation does not remove exact-plan approval. Research and execution remain
separately approved when execution assignments were not fixed in the research
plan. The parent delegates investigation, implementation, verification, and
review. It never integrates child code. Plan revision approval, dependent
worktree quiescence acknowledgement, and manual integration remain user actions.

This contract proposes a specific amendment to the baseline's one-plan parent
capability lifetime: run planning authority survives research-plan completion;
that plan's execution authorization does not. It does not silently reuse the
baseline's expired plan bearer. Both contracts remain proposed until their
authoritative specs/ADRs and #610 agree and their written review gates pass.
This document does not authorize runtime implementation or claim adapter support.

### Established source facts and unresolved eligibility

| Source at branch head `7edbe56e` | Established fact | Consequence for this proposal |
| --- | --- | --- |
| `crates/orkworksd/src/taskmaster/coordinator.rs:1` | Data-only coordinator records, no runtime authority | New run/grant records need a reviewed runtime plan |
| `crates/orkworksd/src/taskmaster/coordinator_store.rs:1` | Durable records, no runtime authority | Persisting a plan does not authorize a child |
| `crates/orkworksd/src/session_application.rs:4856`, `:4966`, `:5029` | Resume workflow retains the OrkWorks session ID and creates a new runtime | Same visible parent can resume; new runtime needs fresh authority |
| Launch baseline, Approved plan / Session creation and ownership | Completion revokes a parent capability bound to one plan | Continuity requires the explicit amendment below |
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
| `contentDigest` | Digest of the immutable validated report descriptor |

Report authentication derives the child, reservation, configuration, sidecar
and active producer launch generation from the session capability and launch
records. Client fields must match those values. Check generation before accepting
or deduplicating a receipt: a pre-resume runtime cannot submit/replay evidence as
the current producer merely because the stable session ID matches. A user-authorized
child resume revalidates the role/configuration and records a fresh active runtime
binding; a new report needs a new immutable version with that binding. Existing
report versions retain their original generation as historical evidence, never
rewritten or silently promoted to the resumed runtime's current delivery.

### Authenticated report submission

The proposed `submitResearchReport` child tool uses
`POST /sessions/{childSessionId}/orchestration/research-reports` on the existing
local sidecar channel. The request carries `schemaVersion` (1),
`expectedTaskVersion` and the immutable report descriptor, at most 32 KiB
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
`409` stale generation/task version or conflicting report ID/version; `413`
oversize; `429` rate limited. Bounded responses provide current authorized
metadata where needed; retries do not alter task outcomes or version counts.

Revoke this capability when that child runtime ends/resumes, on sidecar/workspace
change, run cancellation/final completion or namespace rotation. A live child
may report while its stage plan is paused/complete and the run remains active;
a report still cannot restore a launch grant. On an explicit verified child
resume, issue a fresh generation-bound capability, retain historical reports,
and require new versions for current delivery. After final run completion,
corrections require a separate Electron-authorized user-provenance record;
they cannot impersonate a child or reopen the run. Report acceptance emits a
bounded run event for automatic parent collection. Apply the stated report/rate/
aggregate limits before persistence and serialize with runtime/run revocation.

A report is an agent claim with provenance, not proof that its recommendations
are correct. Native tool observations and independent review evidence remain
separate. No full prompts, credentials, hidden reasoning, or complete transcripts
are stored in research reports. Evidence references point to approved sources;
a URL or quoted path does not authorize following it or widening access.

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
| `OrchestrationRunDefinition` | `schemaVersion`, `runId`, workspace/repository/parent identity, UI-authorized goal and `goalDigest`, sidecar-assigned `admissionEpoch`/`runSequence`, `bootstrapConfigurationDigest`, `maxParallelChildren`, `runDefinitionDigest` |
| `OrchestrationRunState` | Run version, closed lifecycle state, input revision/digest, ordered plan references, current plan, current parent runtime generation; mutable evidence separate from the definition |
| `PreparationRevision` | Run ID, strictly increasing revision, decision, exact question/answer/report/review references and digests, input manifest digest; immutable once referenced by a plan |
| `ParentPlanBinding` | Run/parent/plan/revision identities, run definition and bootstrap configuration digests, preparation revision/input digest; included in the exact approved plan definition |
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
bound. `ParentPlanBinding` links each later plan to those unchanged bytes.
The child `AssignmentConfiguration` remains plan/task-bound.

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
   launch grants. Explicit run cancellation, parent ending, workspace change or
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

The adapter must demonstrate that this channel can deliver approval, report,
blocker and capacity-release changes to the same parent agent when it is waiting.
A tool continuation may return an event; terminal text inference, scheduled new
sessions and injecting keystrokes are not fallback delivery mechanisms. If the
parent's model loop has stopped, show automation unavailable/needs continuation;
do not equate a live PTY with successful waiting. No reconnect after a parent
end, sidecar restart or workspace change is automatic.

## State and transition table

Run states are distinct from baseline plan/task states. Every transition is
version-checked and maintains run/plan fencing in one serialized mutation.

| Run state / event | Next state | Automatic work | Required user authority |
| --- | --- | --- | --- |
| UI-authorized creation | `assessing` | Assess supplied inputs | Reviewed bootstrap/goal/run cap |
| Assessment finds material unknown | `awaiting_clarification` | Present question, retain independent approved work | Explicit answer |
| Current material answers received | `assessing` | Incorporate answers; decide research need | None beyond those answers |
| Research needed | `awaiting_research_approval` | Prepare bounded research proposal | Exact research revision approval |
| Clear task skips research | `awaiting_execution_approval` | Prepare execution proposal | Exact execution revision approval |
| Research approval accepted | `researching` | Request eligible declared researchers/reviewers | Existing approved revision |
| Research report / turn / review arrives | `researching` | Collect report, explicit coordination outcome, advance declared batch when gates pass | Quiescence acknowledgement when reusing a chain worktree |
| Research plan complete | `synthesizing` | Revoke that grant; keep parent running; summarize approved outputs | None to prepare next plan |
| Summary and execution proposal ready | `awaiting_execution_approval` | Display plan and summary | Exact execution revision approval |
| Execution approved | `executing` | Request eligible declared tasks; collect results | Existing approved revision and any reuse acknowledgement |
| Final execution plan complete | `complete` | Revoke run bearer/grants; retain visible results | Manual integration remains separate |
| Failure/blocker/conflict/interrupted allocation | `blocked` | Collect outstanding results; explain and draft recovery | Exact recovery revision and any allocation resolution |
| Proposal rejected | Same approval-wait state | Show rejection; stop repeated unchanged proposals | Changed proposal needs fresh exact approval |
| Stage plan cancelled | `paused` | Revoke grant; retain reports/children | Explicit continue decision before proposing next stage |
| Parent ends / workspace or sidecar changes | `paused` | Revoke bearer/grants; reconcile durable records | UI resume and exact-plan reapproval before launches |
| Run cancelled | `cancelled` | Revoke all authority; retain children/artifacts | New UI-authorized run for new work |

`awaiting_execution_approval` can display capacity held by older children.
Approval does not remove that wait. `complete` does not mean all PTYs ended,
all worktrees removed, or the user's goal accepted. Evaluation and integration
remain separate contracts.

## Revision, cancellation and recovery

A changed report version, user answer, review finding, Git base or configuration
creates a new preparation input revision. Proposed plans bind one immutable input
manifest. Revalidation before approval and every launch detects drift; a stale
proposal cannot authorize current inputs. Approved definitions are never patched
in place. Unused tasks on affected plans pause until the user approves a revised
definition; already-launched tasks remain bound to their original assignment.

Late reports for complete/cancelled plans never reopen tasks or restore a grant.
Identical report replays return the stored receipt. A new correction is a separate
versioned evidence record, retains the original, invalidates dependent summaries
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
| Run definition | 16 KiB excluding the separately retained bootstrap, at most 16 plans per run |
| Bootstrap | Role contract's 1 MiB inclusive snapshot/content limit (256 KiB rendered context) |
| Run child ceiling | UI-selected integer 1–16; each plan's ceiling is no greater |
| Plan revisions | At most 64 per plan; baseline 128 tasks / 2 MiB total definition limit |
| Preparation revisions / clarifications | At most 64 revisions and 32 question/answer pairs per run |
| Question / answer / reasons | 2 KiB each; at most four suggestions, 512 bytes each |
| Research questions | At most 32 per research plan; each question at most 8 KiB inclusive scope/criteria references |
| Report | 32 KiB; 32 findings, 64 evidence references, 16 uncertainty/blocker entries, 16 recommendation/limitation entries |
| Evidence reference / finding | 1 KiB / 2 KiB; no inline full artifact |
| Report versions | At most eight per task; immutable digest-bound versions |
| Research summary | 8 KiB; at most 128 report/review references |
| Input revision manifest | 32 KiB; at most 256 unique record references |
| Event cursor stream | Ring of 256 metadata-only notifications per run, 1 KiB each; report content fetched separately |
| Parent mutations | Ten new proposals/clarifications/result mutations per rolling minute per run; duplicates count against request rate, not stored history |
| Report submission | Ten requests per rolling minute per child; duplicate requests do not increase stored version count |
| Event waits | One outstanding wait per run bearer; maximum 30 seconds, at most 60 requests per rolling minute |
| Run preparation/report metadata | 32 MiB aggregate per run, excluding separately bounded baseline plans/configurations and ordinary terminal history |

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
(integer 0 through 2^63−1). Persist this record, at most 4 KiB, under
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
atomically persists a fresh random epoch with sequence zero before admitting
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
| Clear small task | Direct execution proposal; no ceremonial research |
| Material answer missing/stale/terminal-only | Dependent proposal cannot be approved |
| Parallel question consumes another's result | Explicit dependency/later batch; no concurrent launch |
| Research approved, execution unspecified | Research only; no execution allocation |
| Research completes with live children | Revoke research grant; same parent synthesizes; retain full run capacity accounting |
| Old research grant / old plan bearer presented | No launch or renewal; never revive an expired credential |
| Run bearer without current exact grant | Planning allowed while active, child launch denied |
| Approve execution with old child filling run ceiling | Grant exists, launch waits; no implicit termination or slot release |
| Unauthenticated/cross-child report / parent run bearer used for ingestion | Reject; only the exact generation-bound child report capability can submit |
| Report event without task-turn receipt | Store report, do not infer task result or advance dependency |
| Required review conflict / unknown capability | Block ready execution proposal; explain missing evidence |
| Pending proposal report corrected | New input revision invalidates old proposal; approval fails stale |
| Correction after execution launched | Visible blocker; keep original assignments, no automatic rewrite/retry |
| Stable child session ID resumes into a new runtime | Reject old-generation report requests before dedupe; retain old reports with historical generation; new versions bind the authorized current runtime |
| Forget more than 1,024 runs | Admission continues with constant-size epoch/high-water fencing; retired IDs remain non-recreatable |
| Exhausted/corrupt admission fence | UI-only quiescent namespace recovery; reject all old-epoch mutations and preserve artifact ownership |
| Parent receives duplicate events / launch requests | Read current state; one task attempt/reservation |
| Completion/cancellation races launch | Serialized fencing prevents a post-revocation reservation |
| Parent live but model loop stopped | Show continuation unavailable; no terminal typing or new session |
| Parent end / sidecar restart / workspace change | Revoke credentials, pause, exact identity resume and reapproval |
| New parent bootstrap bytes/skills/settings | Fence old run, UI-authorized new run with reviewed bootstrap; never active configuration mutation or approval transfer |
| Final execution complete with child PTYs live | End run authority; preserve normal child controls/artifacts |
| Oversized record / exhausted history / forgotten-run replay | Reject before persistence/launch; preserve existing records and fences |

## Execution plan and implementation gate

Keep #742 open: this is a concrete proposed contract, not a reviewed executable
handoff. Before code planning, reconcile the proposed capability amendment with
#610 and authoritative specs/ADRs; finalize #741's run bootstrap/plan bindings;
verify #740's parent role, skill delivery, event continuation and exact resume
capabilities or retain explicit no-go combinations. Review the run/grant,
identity, capacity, correction, recovery and numeric-bound contracts together.

The later scoped plan must name exact Rust types/routes/store migrations,
Electron-main approval/preload interfaces, renderer projections, adapter
fixtures and verification cases. Preserve ordinary-session compatibility,
manual integration and both Electron/renderer and runtime ownership boundaries.
No general DAG scheduler, grandchildren, automatic code integration, automatic
retry loop, native confinement or additional UI authority is added here.
