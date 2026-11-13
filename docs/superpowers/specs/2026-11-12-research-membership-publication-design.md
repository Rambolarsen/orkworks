---
type: Design Proposal
title: Research membership and first-report publication closure
description: Proposed research reference recipes, bounded consumer discovery and original-to-final first-report effect ownership, preserving accepted protocols and separate physical and authority gates.
tags: [orkworks, research, consumers, publication, history, qualification]
status: draft
workflow_status: proposed-for-scoped-membership-review
---

# Research membership and first-report publication closure

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `04de3bb57610466eadf546395be1b0bc54b3d14c`.

**Written proposal for scoped owner review.** The accepted research sources and
input/control choices permit the [offline prefix packet](../../validation/research-prefix-components.md).
They do not accept the new membership/discovery choices here. This document
closes a proposed logical reference and first-report effect boundary; it does
not claim complete physical catalog, initializer, producer history, currentness,
authority, cutoff, ordinary bridge or storage qualification. Gate 2 remains
Investigate. No dependent fixture or executable product plan accompanies it.

## Purpose and preserved decisions

The prefix packet's input-2/input-3 and four independent control views give
concrete source joins. They do not prove which other owners retain those sources
or which effects an actual first report must publish. A complete incoming-edge
query cannot answer that question unless all relevant source owners participate.
This proposal names those owners and derives their logical reference recipes.

Preserve the [research source registry](2026-11-12-research-source-codecs-design.md#proposed-source-registry-accounting-and-ownership),
[research input/control contract](2026-11-12-research-input-control-prefix-design.md),
#742 descriptor/request/head/receipt and child revocation, #744 source/evaluation/
material rules, and #746 full-current-summary viewing. No changed source digest,
wire field, report identity, dependency purpose, version default, cap increase,
volatile credential serialization or terminal UI exception is proposed.

Reuse the [accepted child membership discipline](2026-11-03-child-consumer-membership-design.md#scoped-owner-acceptance-and-component-handoff--2026-11-03):
full typed identities; source-derived forward/reverse lookup; historical direct
proof retention; coherent complete discovery; no read-time repair; and separate
retention, freshness, read authority and accounting. Extending that discipline
to the research families is a **new author choice**, not implied acceptance.
Rejected alternatives are a caller's affected list, an independent graph writer,
report citation crawling, a second run-version owner and first-head absence as
proof of zero consumers.

## Inspected seams and limits

| Source / accepted contract | Observed fact | Consequence |
| --- | --- | --- |
| [hierarchy_storage/mod.rs](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/mod.rs) | Generic mechanics are cfg(test), explicitly unable to admit/publish production profiles | No existing research membership or publication owner is qualified |
| [connection.rs](../../../crates/orkworksd/src/taskmaster/hierarchy_storage/connection.rs), Connection::open | Returns unqualified profile before using the supplied path | No eligible open/lookup/initializer is supplied by a declaration of families |
| [format-6 reconciliation](../../validation/child-preparation-catalog-reconciliation.md#full-logical-inventory-and-original-attach-applicability) | Audits 35 logical families for the selected child/ordinary preparation path | That audit does not add research object/control/report variants to its catalog |
| [paired membership design](2026-11-05-paired-membership-index-design.md#scoped-physical-design-acceptance--2026-11-05) | Its physical layout is accepted for further qualification; actual owners, RegistryRoot and native/resource proof remain pending | Preserve that acceptance; do not cast research recipes into format6 or infer complete absence from an index stamp |
| [first-report investigation](../../validation/first-report-producer-contract.md) | Separates original research assignment, live CAS, report history and complete atomic effects | A syntactically valid source or receipt-only proof cannot supply the missing publisher |

These are bounded source facts. They neither inventory all differently named or
external writers nor prove installation behavior. No database, Rust/Electron
runtime, native harness or model was executed for this investigation.

## Proposed logical lookup extension

Keep the existing logical location/target shapes from the membership contract:
object locations contain the complete ObjectRef; metadata locations contain the
fixed family and complete existing canonical key. No hash-only key, field-path
locator, generic caller table or separately authoritative graph is admitted.

For this research slice, add exactly the six research object kinds and four
source metadata families in the accepted source registry, the nine metadata
families in the accepted input/control registry, and the two compact report_heads/
report_receipts families to the relevant logical lookup unions. The latter use
the unchanged [compact registry keys/values](2026-10-29-report-publication-profile-design.md#report-head-and-receipt-registry),
not arbitrary metadata locations or generic heads aliases. Use their existing
exact identities/keys, bodies and source domains.
This selects no storage row domain, physical name, column, schema or format.
Research-specific references resolve research sources; they are not aliases for
child-source objects with similar fields. Existing root/report/#744 locations
retain their original typed variants and codecs.

For SubjectSource of a research child, resolve the original
research-assignment-source and its research reservation/attachment/configuration
group. Preserve the complete frozen ChildAssignmentIdentity. A complete future
catalog must select one original source owner for that identity at admission;
matching IDs, current role or whichever family has a row cannot elect it.
Conflicting ownership denies. This is not an extra field on AssignmentIdentity
or a profile migration, and does not change the child path's source resolver.

Do not extend control-origin to research recovery snapshots: no such closed
recovery codec has been selected. Direct proof targets use existing CommitRefs
and the eventual owning catalog's exact committed projection. An unselected row
domain is unavailable, not a license to borrow format-3/4/6 commitments.

Each logical occurrence is the existing complete consumer/target tuple.
Duplicate tuples may collapse only for reachability. Repeated fields, embedded
source copies, retained bodies and physical index occurrences remain charged.
The complete old/new owner recipes determine membership, including owners with
zero outgoing references; unknown/missing/extra owners or occurrences deny.

### Research object reference recipes

All arrows below are **typed source-group membership**, not new dependencies
rows or purpose strings. Reconstruct each target's full reference from the
original source group and validate complete bytes. Existing #744 dependency
recipes remain distinct and unchanged.

| Consumer kind / existing identity | Exact required membership |
| --- | --- |
| research-source-capture / ResearchCaptureRef | Original root/run repository and source-worktree bindings, exact TaskKey source group and its retained capture bytes. A path/revision/content hash supplies no live read or native origin proof |
| research-source-scope / ResearchScopeRef | Every listed ResearchCaptureRef; each question sourceId and excerpt resolves that same capture set, with its accepted byte-range/excerpt joins. Never fetch an unretained source |
| research-source-configuration / ConfigurationRef | Its original run/plan/task and bootstrap; every exactly cited research-source-evidence occurrence by full task/referenceId/contentDigest and research whole-body fingerprint; exact scope used by startup settings. Embedded instruction snapshots remain under their original configuration owner |
| research-source-plan / PlanRef | Exact original configuration, scope and declared group; original root and prepared input at its frozen preparationRevision/inputDigest, resolving the paired input revision. Never today's currentInput |
| research-source-evidence / task,referenceId,contentDigest | Original task/configuration capability reference and retained raw occurrence. Equal bytes under another referenceId remain another occurrence; payload text creates no inferred graph edge |
| research-assignment-source / full ChildAssignmentIdentity | Exact original plan/configuration/scope/approval/reservation/allocation/attachment and root/source group; frozen taskVersion and original child/parent IDs. Never latestApproval, live taskVersion or a replacement worktree |
| research-report / full accepted ReportIdentity | Original research assignment/reservation/configuration/scope and reporting runtime-history source. Every file evidence reference resolves an exact approved excerpt/capture through that scope. Empty findings/references do not discard the original assignment/scope group |

The last row is a recipe for the existing report object kind, not a seventh new
research-source kind. A report's historical generations resolve its original
runtime record; they need not equal the process currently running. Current
reporting authority remains a separate live check. Question text, uncertainty,
URLs and source prose never create transitive retained references or fetch grants.

### Research metadata reference recipes

The existing exact key for every row is mandatory; the field recipes below add
no codec members. Each row retains its own original lastCommit proof and every
explicit direct receipt/origin proof under the future closed catalog.
Resolve nullable references **only when present**; null creates no target and is
preserved exactly. This includes pending plans/tasks and expectations before
reservation or first report. A recipe is not a requirement to populate a null.

| Family / existing key | Complete required typed references |
| --- | --- |
| research_plan_approvals / plan,approvalId | Exact original plan, request/source fingerprint, review context and original receipt/commit. Historical approval does not recreate a grant |
| research_reservations / TaskKey | Exact task/plan/configuration/scope and original ApprovalRef; unique original childSessionId and expected/frozen pair. Do not follow plan.latestApproval instead |
| research_allocations / ReservationRef | Original reservation, plan/group, repository and original allocated descriptors; allocation phase cannot rewrite retained source worktree identity |
| research_attachments / childSessionId | Original reservation/allocation, attachmentId, full frozen identity and research assignment/prepared subject group; unique child attachment remains required |
| research_control_inputs / workspaceId,runId,preparationRevision,inputRevision | Original root goal/bootstrap; every configuration, scope and capture record; each inline report source's exact report; producer RuntimeRef. A stale decision additionally retains the unique original run/k/k assessment row, its exact decision/digest/parent producer and runtime/proof origin |
| research_runtime_attachments / RuntimeKey | Parent: original root/run/bootstrap and attached parent identity. Child: original research attachment/reservation/assignment source and actual successful runtime-generation origin. Spawn intent creates no history row |
| research_run_controls / RunKey | Original root/run/bootstrap/repository/source bindings; currentInput; every ordered PlanRef, including current/pending when present; parentRuntime when present and blocker TaskKeys. The initial pointer resolves original root input-1, not a fake research input row |
| research_plan_controls / PlanRef | Exact original plan/task; latestApproval and replaces only when present, otherwise retain null. This prefix has no positive replaced-plan variant; do not invent a prior-plan input record to admit one |
| research_task_controls / TaskKey | Original task/plan/configuration; original reservation only when present, otherwise retain null. Positive coordination-result codecs remain unclosed here; encountered unsupported nonnull results deny this slice rather than inventing a receipt |
| research_launch_controls / TaskKey | Original reservation, frozen admission approval/parentRuntime, established allocation/attachment and successful spawn history. Prepared intent is not an established runtime source |
| research_session_states / TaskKey | Original reservation/attachment/assignment source/session and current RuntimeRef only when present; ordinary producerRevision remains independent of taskVersion and metadata revisions |
| research_collections / RunKey | Exact owning run and singleton TaskKey, including retained inactive membership; no second current-input pointer |
| research_expectations / TaskKey | Collection RunKey and original configuration/scope; original reservation and selected report source only when present, otherwise retain their nulls. Collecting and retired rows remain discoverable, including pre-reservation/pre-report expectations. Resolve Stream only when the original reservation is bound, through it and the original configuration |
| report_heads / exact Stream | Exact report ObjectRef through complete stream plus head reportId/version/digest; original assignment/source group and accepted contiguous history/receipt prefix. A generic head or maximum version cannot substitute |
| report_receipts / stream,reportVersion | Exact original immutable report and complete original normalized request/closed response binding, original source group and direct commit proof. Historical expectedTaskVersion is not recaptured from today's live task; receipt lookup never grants current reporting authority |

An inline ResearchReportEvidenceSource in an input/expectation is an exact
accepted value, not evidence that a standalone assignment-evidence-source object
was inserted. Derive its sourceKey and full ReportIdentity/size/digest join and
retain the report through the **containing** consumer. Do not synthesize an
evaluation, evaluation-evidence edge or separately charged source object. If a
real standalone #744 evidence-source object exists, its unchanged source→report
dependency and every retained evaluation→source dependency apply separately.

The initial run_states seed and root input remain historical. Research current
runVersion has only its accepted research_run_controls owner. Current attached
parent_attachments retains its root-owned charge and joins the research runtime
history for this admitted research profile; it does not create a second version
owner or move the original root preparation into a research source pool.

## Complete discovery and the positive first-report slice

The following is a new stricter **research eligibility choice**: support first
publication only with the producer run's one active collection and its exact
collecting expectation as the affected active preparation. Any additional
prospective consumer whose semantic effect is not closed by this slice makes
it unavailable. Preserve its rows and references. This does not remove #742's
broader obligation to invalidate every affected active preparation; the bounded
publisher must refuse histories requiring unsupported effects.

Discovery must cover both the prospective owner relationships and exact retained
references. Resolve these complete relations under one qualified workspace/store
boundary before declaring this narrow history eligible:

| Owning relation | Required complete result / treatment |
| --- | --- |
| Run→collections and collection→all expectations | Owning collection plus every retained expectation, including inactive/retired rows; check exact singleton collecting task when active. Active/collecting are source state, never caller completeness flags |
| Exact TaskKey / resolved Stream→expectations | Derive from original reservation/configuration, reject alias/duplicate/conflicting ownership. The one active local expectation binds this stream, with selectedReport null before first acceptance |
| Exact source→current and historical input consumption | Include every immutable input record and stale-assessment-origin join. No current-pointer-only scan; historical inputs remain protected |
| Research/root/child source groups | Full original source/configuration/plan/approval/attachment/runtime and direct proofs, with source-owner/index correspondence. A complete returned list of matches does not establish all owners exist |
| #744 evidence, results/evaluations/artifacts and source groups | Existing full retained membership, including uncited evidence and historical/ineligible evaluations; subject/receipt/debit/proof families keep their accepted recipes. An exact reference to the not-yet-accepted report is corruption, not an effect first publication can repair |
| #741/#742 configurations, review inputs, summaries, proposals, plans and launched bindings | Complete owner inventory proves absence of unsupported prospective consumers. A retained PlanRef alone is not evidence of report consumption; inspect its exact source. Unsupported or unknown codecs deny rather than count as zero |
| #745 learning/shared custody | Existing local exclusion and joint custody fence remain closed. Workspace lookup cannot prove repository-wide absence; ambiguous/shared protection makes this slice unavailable |
| #746 tracked owner runs | Derive producer run and any actual dependent owner runs from authoritative ownership, including historical retention. Classify visible-change/snapshot effects separately from retention; unknown tracking denies |

Use full source-derived forward and reverse membership, collection/task/Stream
lookups and original identity bindings, not a scan of heads or dependency rows
alone. The storage owner must prove complete owner-family coverage and both lookup
directions. A zero-reference owner or a matching stamp proves neither complete
inventory nor currentness. Complete source/index correspondence, not only counts
or hashes, is required before current reads, writes or purge.

Discovery and final validation share one cumulative research bound: propose the
accepted child profile's **4,096 logical observations/probes and 16 MiB canonical
discovery bytes**, with exactly its observation units. Each returned occurrence,
separately read owner/projection, repeated lookup, failed exact-key seek, page and
separate end/empty-range probe counts. Successful one-record lookup counts once;
joined records count separately. Repeated owned bytes/probe keys count again.
In-memory reachability deduplication cannot discount actual I/O. Completion and
overflow detection must themselves fit; reaching a bound without completion
denies. This is stricter than, and does not replace, the existing 100,000 visited-row
ceiling and cooperative SQL deadline. All smaller worker, content, scratch,
command, store, index and physical limits apply independently. No body is relabelled
metadata or fetched outside custody to make discovery fit.

Physical research lookup layout, complete catalog/version and initializer remain
separate written decisions. The accepted paired layout is a constraint/prior art,
not an automatically extended eligible research format. If a paired ordinary view
is required, independent snapshots do not suffice: its actual broker/connection,
both headers, source permits and ordinary owner must be qualified together.
No RegistryRoot acceptance, new global counter or header-only absence proof is
inferred. No discovery/repair scan, physical index or adapter is implemented here.

## First-report read set and original-to-final effects

Preserve the [first-report admission order](../../validation/first-report-producer-contract.md#exact-first-report-request-and-acceptance-order).
Current child capability/generation/epoch/owner/lease/health and rate precede
receipt lookup. Exact authorized retry returns its original binding before live
CAS with no mutation. Final run/child revocation still denies before lookup;
#744 claim-first and the terminal UI exception supply no child cutoff.

For a genuinely new first request, capture original source/control/input/runtime
and proof joins; actual live taskVersion; exact absent stream/head/receipt/history;
complete membership above; actual run/collection/expectation versions; every
resource origin; and all affected source/health/tracking/freshness facts. Revalidate
the entire set and membership ranges at the separately qualified commit boundary.
Insertion/removal, source or header drift denies that captured attempt. Do not
patch the request, retarget its stream or silently retry under updated facts.

The publication owner receives typed source intent and complete internally
derived effects, never arbitrary row commands or caller-supplied read sets. The
logical before→final union is:

| Owner | Required change / unchanged facts |
| --- | --- |
| Report | Insert the original report object, compact authoritative report_heads entry and v1 original request/closed response receipt. Full identities, domains, body/request/head/receipt caps and shared eight-version budget stay unchanged |
| Input/collection | Create next immutable paired input revision with the reporting child's RuntimeRef, exact selected report record and unchanged old decision; retain its original assessment row. Change expectation selectedReport/version and collectionVersion once, with source-derived lookup maintenance |
| Current run | Change currentInput and checked runVersion once. Preserve state, parent pointer, stageContinueRequired, blockers, pause/clarification and plan references unless an independently prescribed effect changes them. No second run counter |
| Task/plan/launch/session/runtime/source | Reads/fences only for this report event. No taskVersion/outcome, plan completion, launch grant, slot refund, attachment or ordinary bridge write follows from report arrival |
| Custody/membership | Preserve originals and every historical reference; derive new report source-group and input/selection references from final bodies. Existing dependencies change only where their unchanged owner recipe actually requires them; no invented report credibility/evaluation edge |
| Summary/tracking | Source-run detail snapshots invalidate for visible report/input/selection change before another current read. Any actual dependent visible change receives its required owner-run fence. No new evaluation/result or material revision merely because a report exists; source facts decide whether subject effects are truly empty |
| Resources | Coalesce each exact owner/resource key's old→final delta once; preserve all retained copies and original ledger proofs. Missing/mismatched ledger denies, never create/reset it during publication |
| Storage | Commit all logical changes, required direct proof links, derived physical lookup maintenance, one header advance and one new complete proof outcome together. No post-commit registration or partial success |

For the normal illustrative prefix only, runVersion 10→11, input 2→3 and
collection/expectation versions 2→3; live taskVersion stays 7 and frozen identity
stays 2. These are samples, not admission defaults. A valid active paused run may
receive reports without a live parent or launch grant; collection cannot clear
its gates. Input-64 exhaustion denies a new acceptance requiring input-65; an
otherwise authorized exact accepted retry remains read-only.

An effect union with any unsupported consumer, unclosed source-health/tracking
protection, unknown native/ordinary outcome or unqualified cutoff is unavailable.
No whole-command byte/row count follows from the finite fixture's 24 examples.
Notifications occur after durable publication and remain hints; a notification,
receipt, view token or event-ring arrival cannot prove current head or viewing.

## Accounting, recovery and compatibility

For every actual changed row r in its existing pool, compute charge(final r) minus
charge(original r), coalesced from the original state directly to final state.
Never charge through intermediate versions or discount repeated occurrences.

| Occurrence | Existing owning resource / rule |
| --- | --- |
| Report object, report_heads and report_receipts | Run/workspace report-retained bytes; descriptor/request/response retain accepted hashes and exact original request binding. Exclude only their named outer storage links |
| New research input and changed run/collection/expectation controls | Shared existing child-control-retained-bytes pools, 8 MiB/run and 64 MiB/workspace; old immutable inputs/runtime sources remain charged, mutable replacements use original→final deltas |
| Six research source kinds and four source metadata families | Existing shared child-source-retained-bytes pools, 8 MiB/run and 64 MiB/workspace. First report reads rather than rewrites original source rows; every newly retained real source occurrence remains separately charged |
| Root preparation and parent_attachments | Existing preparation ownership; no transfer or free copy because research runtime history also exists |
| Genuine subject/summary/custody effects | Their unchanged exact owner resources and limits, derived only if actual effects exist. No #744 receipt credit, evaluation slot or new resource is borrowed for reports |
| Physical memberships, indexes and proof/header/store growth | Existing independent physical/command/store budgets. Derived lookup rows are not extra logical manifest changes or an additional quota allowance |

Preserve 32 MiB run preparation/report aggregate, 256 MiB workspace report pool,
all smaller source/control/preparation limits, row caps and complete-command
4,096 changed-row/16 MiB limits. Discovery's identical-looking numbers are
independent bounds, not another allowance. A future whole-state witness must
enumerate every actual occurrence, proof/index cost and ledger delta; sums of
the four alternative views cannot supply that accounting.

Fresh research creation must initialize its complete reviewed catalog and full
resource inventory with original proof links. The zero-report-ledger packet is
not that initializer. No partial format-3/4/6 store, differently committed row or
receipt is adopted, upgraded, retokened or selected from table presence. This
document intentionally selects no format number or source/row commitment change.

Recovery exposes complete old/new state or unavailable reconciliation. Reconstruct
source-to-membership equality, exact head/history/request/receipt, retained input
origins, resource balances and required direct proofs before dependent current
use. Preserve original attempts/generations/custody. No index rebuild, missing-zero
insertion, maximum-version election, receipt synthesis, volatile credential
restoration or recursive replay of all unrelated predecessor proofs is permitted.
Opening/recovery scans need their own resource/content authority qualification;
an ordinary read or artifact debit cannot trigger an uncharged scan. Ordinary
input/stop/end never waits for hierarchy SQL/fsync or recovery locks. Unknown
publication/cutoff disposition holds hierarchy ownership/capacity conservatively.

## Owner decisions and bounded handoff

The new author choices for scoped written review are:

1. Extend the existing typed logical lookup discipline to the exact research
   source/control and compact report-registry families with the recipes above, using direct source-group
   membership rather than a new dependency purpose or graph owner. Inline report
   sources retain through their containing input/expectation, without a fabricated
   standalone #744 object. Research SubjectSource has one original owner.
2. Support only the one local prospective collection for positive first-report
   effects; complete other-owner discovery must prove unsupported consumers absent,
   otherwise preserve them and deny the slice. Retention/currentness/authority/
   freshness/tracking effects stay distinct.
3. Apply the explicit 4,096-observation/16 MiB cumulative discovery discipline to
   research, including misses/pages/revalidation, independently of existing visited
   row, command and physical limits. Overflow never truncates or splits publication.

The original-to-final effect/accounting tables reconcile accepted obligations;
they do not qualify a real command. Acceptance of these three choices would permit
only a bounded **offline membership/effect component packet** under
`docs/validation/`, using the already accepted prefix examples:

- An independent Python generator and retained JSON derive finite reference sets
  from those source/input/control/report bytes; no existing artifact is rewritten.
- A data-only JavaScript checker/tests compare forward/reverse tuples, original
  assessment/runtime references, inline versus standalone report-source ownership,
  prospective collection membership and original-to-final sample effects. It is
  neither a store/publisher nor a reducer for application events.
- Denials cover missing/extra/foreign/unknown membership, dropped historical
  origins, unsupported prospective consumers, repeated observation/byte budget
  overflow and report-induced task/frozen-version changes. Conditional supplied
  complete-set/range/proof assumptions must remain explicit; finite tuple equality
  cannot certify global source/index coverage or a legal event history.
- Evidence prose, index/check routing and LF attributes record scope, independent
  measurements and all remaining physical/producer/authority limits. No whole-
  transaction count or accounting success is inferred from alternative examples.

Complete physical research catalog/version/initializer/proofs, actual source-owner
currentness and indexes, feasible resource/native behavior, child cutoff and
ordinary bridge, broader active/terminal/recovery consumers and separate runtime
design/plan approvals remain blocking. No new Rust/Electron files, reducers,
factories, handlers, codecs, storage operations, migrations, adapters, native/model
probes or executable product plan are authorized by this written handoff. #1689,
#1089 and their broader component issues remain open; #740 stays closed research.

## Written review disposition

One fresh read-only Luna builder check found no material unanswered question for
the bounded offline packet. The independent correctness check flagged ambiguous
nullability wording for plan/task controls and expectations. The author checked
the accepted nullable codecs and clarified that only present references resolve
targets: pending/pre-reservation/pre-report nulls remain intact. The clarification
also makes optional current parent/session-runtime references explicit. No source
codec, producer behavior or accepted lifecycle rule changed. This was one review
round; the author applied the clarification afterward. Neither reviewer executed
code, a fixture, database or native/runtime process. Physical/global completeness,
actual source currentness, authority and publication remain unverified.

## Scoped owner acceptance and offline component handoff

The owner answered “keep going” to the explicit request to accept the three
choices for **offline fixtures only**. This accepts the research reference recipes,
single local prospective collection boundary and stricter discovery limits at
that scope. It does not accept a physical catalog/initializer, real source/index/
currentness proof, cutoff/ordinary bridge or product implementation.

The bounded packet derives research object/source/history membership and separate
membership sets for the four unchanged prefix views. It adds a synthetic compact
head/receipt/request join over the supplied first report and compares the four
local input/control changes between the running and selected-source samples.
Conditional discovery traces retain supplied coverage/header assumptions. It
creates no store, reducer, publisher, legal event sequence or whole accounting
witness. Earlier artifact bytes remain unchanged; broader gates above stay open.

The [offline membership/effect packet](../../validation/research-membership-components.md)
records bounded component evidence for this acceptance. Its supplied owner/header
assumptions do not prove complete source/index coverage or legal publication.

The [fresh initialization/catalog follow-up](2026-11-13-research-initialization-catalog-design.md)
proposes the explicit creation resource union and initial current-run owner at
root admission. It identifies the direct-execution-only preparation authorization
as a research catalog blocker and does not assign a format from table counts.
Its two new initializer choices require their own scoped review; earlier
membership acceptance supplies no complete-store or runtime authorization.
