---
type: Design Handoff
title: Hierarchy projection mapping and review fixtures
description: Proposed source mappings, freshness boundaries, and synthetic consumer fixtures for the agent hierarchy.
tags: [orkworks, hierarchy, projection, review]
status: draft
---

# Hierarchy projection mapping and review fixtures

- Tracking: [#746](https://github.com/Rambolarsen/orkworks/issues/746), initiative [#738](https://github.com/Rambolarsen/orkworks/issues/738).
- Source baseline: `2e85038c1d33476df436b12cf1e2083d61db569f` (2026-10-15); producer reconciliation drafted against `1dc68e025038dfe7a6d96c4229d9852a7c9d2996` (2026-10-17).
- Status: proposed consumer mapping for written review; upstream agreement and owner acceptance remain pending.

This supplements the [hierarchy interaction contract](2026-10-04-agent-hierarchy-ui-design.md#renderer-projection-and-contract-fixtures).
It maps existing proposed records rather than defining an HTTP/IPC schema.
The accepted shell and ordinary-child scope remain binding. No adapter profile,
root evidence producer, runtime implementation, approval, or release validation
is established by these fixtures. #740's capped capability work remains deferred
until November; all profiles remain unverified/no-go.

## Source ownership and exact joins

[Role configuration #741](2026-10-04-agent-role-configuration-design.md#approved-logical-records)
owns immutable assignment/bootstrap content. [Preparation #742](2026-10-04-orchestrator-preparation-design.md#logical-records-and-immutable-parent-configuration)
owns run/plan authority and mutable coordination. [Skill evidence #743](2026-10-04-skill-usage-evidence-design.md#ui-projections)
owns delivered/used evidence. [Evaluation #744](2026-10-04-assignment-evaluation-design.md#canonical-records)
owns accepted results, reviewer references and authoritative outcomes. The
sidecar validates those joins before exposing a read projection. The renderer
never fills missing identity members from labels, paths, timestamps or another
session's records.

| Consumer value | Source and required join | Unavailable/mismatch behavior |
| --- | --- | --- |
| Snapshot scope/completeness | Selected workspace plus current #742 `sidecarGeneration`; a separate projection version orders snapshots within that lifetime; explicit complete/unavailable status covers validated ownership, collection bounds and session joins | Reject late/older responses; partial, corrupt or oversized structure is visibly unavailable, never an empty tree/No actions; do not compare snapshot versions across sidecar lifetimes |
| Run root | #742 `OrchestrationRunDefinition.runId`, parent/workspace identity, `goalDigest`, `runSequence`, `bootstrapConfigurationDigest`; #741 bootstrap has the same run/parent/workspace and digest | No guessed root or cross-workspace membership; missing session is distinct from invalid run identity |
| Repository reference | #741 bootstrap/child `repositoryId`, `repositoryBinding`, `sourceWorktreeBinding`; their digests must equal the run and `ParentPlanBinding` values | No path/name inference; optional learning unavailability does not remove otherwise valid ordinary sessions |
| Stage/task navigation key | `(runId, planId, planRevision, taskId)` from the exact declared plan; `ParentPlanBinding` matches the run/bootstrap and preparation input | New plan revision means new keys; stage is a non-agent heading; declaration/batch order is retained |
| Child assignment identity | Copy #744 child `AssignmentIdentity`: `workspaceId`, `runId`, `planId`, `planRevision`, `taskId`, `taskVersion`, `reservationId`, `parentSessionId`, `childSessionId`, `configurationId`, `configurationDigest`, with `kind: child` | Navigation key alone cannot join an evaluation; no child identity before a server-bound allocation/session exists |
| Child configuration | #741 `AssignmentConfiguration` supplies configuration ID/digest, role template, assignment label/criteria/dependencies, ordered skills and tool/model/permission bindings; #742 allocation supplies task version/reservation/session | Copy only validated values; a configuration does not itself prove launch, delivery, effective model observation or review |
| Root bootstrap details | #741 `OrchestratorBootstrapConfiguration.roleTemplate`, immutable `assignment` label/criteria, ordered `skills`, `rubric`, tool/model/permission bindings and user goal bound by `goalDigest`; run/root identity and bootstrap digest match | Project bounded display details, not raw instruction content; later plan/task configuration cannot replace the root role, criteria or selected skills |
| Root assignment identity | Copy #744 root `AssignmentIdentity` from #741 bootstrap: `workspaceId`, `runId`, `rootAssignmentId`, `parentSessionId`, `bootstrapId`, `bootstrapConfigurationDigest`, with `kind: root` | No invented plan/task/reservation/child fields; a later plan cannot replace the root's bootstrap |
| Ordinary session status | Exact validated parent/child session ID joins current canonical `SessionInfo.id`; lifecycle, attention, memory state, terminal outcome and resume options remain ordinary session data | Valid reference with absent session gets Unavailable; conflicting ownership gets Workflow structure unavailable; ordinary sessions remain usable |
| Stage/run coordination and capacity | #742 run state/version, exact plan/task state/version, declared dependencies/batches, backend live-child/reservation counts and unresolved allocation state | No capacity release from result acceptance or process-text inference; no UI-derived readiness or approval |
| Accepted result | #744 complete assignment identity plus `resultRevision` and `resultDigest`; resolve the immutable `AssignmentResult` | No accepted result means Unassessed unless authoritative #744 outcomes establish a known failure; candidate bytes never acquire the prior result's review |
| Candidate state | #744 candidate identity, predecessor, authority generation and sidecar finalization disposition, separate from the accepted head | Label awaiting finalization/rejection separately; rejection alone is not a quality failure |
| Review details | Exact #744 `EvaluationReference`: assignment identity, result revision/digest, reviewer identity, evaluation revision/digest; criteria/rubric snapshot digests resolve immutable approved snapshots | Preserve historical/conflicting sources; no averaging or worker self-report promoted to review |
| Overall result/freshness | Sidecar projection of #744 current required outcomes, quality, findings and disputes; source validation/invalidation precedes projection | Required current uncontested failure wins over missing evidence; optional-only stale details do not downgrade a pass; renderer does not recompute |
| Child skill identity | #743 `skillSnapshotId`, `skillVersion`, `skillContentDigest` resolves exactly one #741 selected skill; its full `binding` matches the allocation/configuration and evidence generations | Same label/version with different content is not the same skill; missing bindings cannot decorate a current badge |
| Skill sources and ordering | #743 independent `delivery`, `usage.reported`, `usage.observed`, `currentEvidenceRefs`; order by durable `workspaceSequence`; display `lastAt` from server `receivedAt` | Missing use is Unknown; lower-bound counts retain coverage; producer `occurredAt` cannot order UI updates or claim complete coverage |
| Action navigation | Authoritative current #610/#742 decision identity/version and exact run/plan/task target, projected as an opaque display/navigation descriptor | Reveal/inspect only; stale/failed refresh disables decisions; Electron main separately validates any eventual command |

The last row does not establish an upstream `ActionTarget` wire record. Likewise,
#742 has no finalized hierarchy snapshot version. #744 now proposes separate
[summary revision metadata](2026-10-04-assignment-evaluation-design.md#hierarchy-summary-revision-ownership),
without adding a field to canonical result/evaluation records. The remaining
encodings/retrieval and allocation-field
mapping need consumer agreement before executable planning. Keep transport
metadata separate from canonical result/configuration objects; do not add
unknown fields to strict upstream records to make the UI convenient.

### Generation and root-evidence boundaries

#742 sidecar/launch generations are opaque 64-character lowercase hex values;
adapter generation is #741's bounded ASCII capability identity. #743's evidence
store generation is a separate positive bounded integer. Compare each in its
own namespace. #744 child `AssignmentIdentity.taskVersion` is the frozen
#742 reservation `assignmentTaskVersion`, not the current coordination counter.
#743 skill-evidence `binding.taskVersion` retains its independently validated
task-state/launch-binding semantics. Match the shared run/plan/revision/task,
reservation/parent/child/configuration members, but validate these two task-version
values against their respective source records rather than requiring equality.
Never replace either with the other or backfill missing admission evidence from
today's task state. Reservation identity and each source-specific version remain
part of their own joins, not substitutes for the stable navigation key. Preserve #742's canonical decimal
string `runSequence` losslessly; never coerce it through JavaScript Number.
#744's u64 artifact versions/metadata similarly need a reviewed lossless
cross-language encoding before runtime projection; this logical mapping does
not resolve their transport by rounding or changing strict source records.

An old in-flight response is rejected after restart. A fresh authoritative
snapshot may still expose accepted historical results/evaluations whose original
producer generation is no longer active. It must revalidate them under #744;
revoked reporting authority neither erases retained review nor permits writes.
Resume needs newly bound current skill evidence; it cannot promote receipts
from an old launch into confirmation for a new one.

#743's current producer binding and UI example require a launched child,
reservation and plan/task. They do not define a root reporter. The root may show
its immutable bootstrap's selected skills with Delivery unconfirmed and Usage
unknown. Do not relabel child evidence as parent evidence or show a later plan's
skills as loaded in the parent. Confirmed root delivery/usage requires a separately
reviewed root-binding/projection extension. The proposed initial slice explicitly
defers root telemetry, as recorded in
[#741's bootstrap contract](2026-10-04-agent-role-configuration-design.md#run-bound-orchestrator-bootstrap)
and [#743's root boundary](2026-10-04-skill-usage-evidence-design.md#root-evidence-boundary).
Mandatory verified parent content delivery remains a launch/resume prerequisite
even though this projection has no root badge receipt. Root evaluation instead uses
#744's root identity and explicit user review.

## Summary revision and action freshness

A task's stable navigation key, complete assignment identity, accepted
`resultRevision`/`resultDigest`, #744 purge/CAS `subjectRevision`, and UI
`summaryRevision` have different lifetimes. The first locates a row; the next
three bind assignment/result/storage operations. Only the proposed summary
revision keys explicit viewing and automatic-collapse suppression.

Proposed producer rule for owner review, owned by
[#744](2026-10-04-assignment-evaluation-design.md#hierarchy-summary-revision-ownership):
the sidecar issues a stable summary revision for the exact assignment/result
and its material displayed summary. It changes when accepted result identity, overall result, required
findings, or required outcome freshness/conflict disposition changes. A change
back to an earlier disposition is still a new material revision. Optional-only
detail changes, polling, navigation, and a runtime restart with unchanged
revalidated content do not change it. The token is a canonical decimal string
`"1".."2147483647"`, scoped to the complete assignment identity and treated as
opaque by the renderer. A durable row advances once per committed material
transition; restart preserves it. Source freshness, material state and revision
publish atomically or affected reads are fenced. Missing/corrupt lineage, purged
subjects and exhausted counters produce summary unavailable; they never reset
to 1 or serve an old pass as current. Source corrections remain possible at
exhaustion. Exact material bytes, storage bounds and transaction/deletion
recovery remain runtime-plan gates. Do not substitute a
content digest that reuses an earlier revision after a correction is reversed.

A group is auto-collapse eligible only after every current summary was explicitly
opened, coordination is terminal, no current decision/blocker/failure requiring
action/nonterminal child/reservation remains, and focus/inspection are outside.
A new summary revision clears only that summary's viewed/reopen-suppression state;
it does not acknowledge a session or create approval. Missing summary identity
prevents automatic collapse. Mere restoration/mounting never marks it viewed.

Decision freshness is independent: a current action descriptor can change without
a new result summary. Rollups and Show next action use complete current action
state, canonical live attention and unviewed summaries as separate categories.
An unavailable refresh is not an empty action set. Do not make a stale button
valid just because its node or summary revision still matches.

## Synthetic fixture vocabulary

These are logical fixtures, not accepted JSON or retained runtime observations.
`D(x)` denotes a distinct synthetic 64-character lowercase digest for each
named value; `G(x)` denotes an opaque generation in the same encoding. `R` is
an admitted fixture run with its exact epoch/decimal sequence; these aliases
must be expanded and validated in any future executable fixture. No real
prompts, paths, transcripts, credentials or capability evidence are included.

| Alias | Exact logical content |
| --- | --- |
| `K1` | Navigation tuple `(R, execution-a, 2, I1)` |
| `A1` | Child identity: `kind=child`, `workspaceId=workspace-a`, `runId=R`, `planId=execution-a`, `planRevision=2`, `taskId=I1`, `taskVersion=1`, `reservationId=reservation-a`, `parentSessionId=parent-a`, `childSessionId=child-a`, `configurationId=config-a`, `configurationDigest=D(config-a)` |
| `B1` | Root identity: `kind=root`, `workspaceId=workspace-a`, `runId=R`, `rootAssignmentId=root-a`, `parentSessionId=parent-a`, `bootstrapId=bootstrap-a`, `bootstrapConfigurationDigest=D(bootstrap-a)` |
| `E1` | Evaluation reference: `assignmentIdentity=A1`, `resultRevision=1`, `resultDigest=D(result-a)`, `reviewerIdentity={kind:user,userReviewId:user-review-a}`, `evaluationRevision=1`, `evaluationDigest=D(evaluation-a)` |
| `S1` | #743 child skill projection: `skillSnapshotId=skill-a`, `skillVersion=v1`, `skillContentDigest=D(skill-a)`; binding matches A1's run/plan/revision/task/reservation/parent/child/configuration members, has independently source-validated `taskVersion=2` (A1 retains admission version 1), plus `sidecarGeneration=G(sidecar-a)`, `launchGeneration=G(launch-a)`, `adapterGeneration=fixture-adapter-a`, `evidenceStoreGeneration=1` |
| `U1` | Stable reported-use event `fixture-stream-a:1`, `workspaceSequence=1`, source `assigned_session_self_report`, `processOriginVerified=false`; delivery remains `selected_unconfirmed`, observed use Unknown/unsupported |

All outcomes in the following table are supplied by an authoritative synthetic
sidecar projection, not calculated by the renderer. Each fixture starts from
its explicitly stated inputs; table rows do not mutate one shared scenario.

| Fixture | Inputs/change | Expected display and interaction |
| --- | --- | --- |
| F01 planned then allocated | K1 has config-a but no A1; later the server supplies valid A1 and canonical child-a with `lifecycle=alive` | Preserve row/order; no terminal command before binding; afterwards Open terminal selects only child-a |
| F02 incomplete assignment join | E1 arrives for K1 but `reservationId` is missing or differs | Do not show E1 as K1's current review; visibly unavailable evaluation; no inferred identity |
| F03 revision reuse | New navigation tuple `(R, execution-a, 3, I1)` repeats I1's label | New row identity; no copied evaluation, loaded badge, viewed summary or approval |
| F04 root with missing session | Valid B1/bootstrap and run reference remain; parent-a is absent; child-a survives | Parent session unavailable, selected bootstrap skills unconfirmed/unknown; child-a remains ordinarily selectable; no replacement/resume |
| F05 usage without delivery | S1 accepts U1, then receives the same event again | Outlined badge, Reported use, details say reported by assigned session; at most one visible highlight; no confirmed loading/process claim |
| F05a independent task versions | A1 retains frozen admission taskVersion 1; S1 has independently valid evidence-binding taskVersion 2 for the same reservation/configuration/child/latest launch | Validate each version against its owner; do not reject current skill evidence solely because 1 differs from 2; never mutate A1 or rehash its result |
| F05b invalid source-specific version | S1 claims taskVersion 2 but its authoritative #743 source binding proves 3; other identities match | Reject current evidence; equality with A1 would not repair a mismatched #743 binding |
| F06 stale skill binding | A1 resumes with `launchGeneration=G(launch-b)`; S1 still names launch-a | No current Loaded/use promotion; retain source-labeled history only; adapter generation alone cannot bridge launches |
| F07 finalization pending | Accepted A1 result 1/E1 pass remains; new candidate is staged, then rejected | Show exact accepted revision separately from awaiting finalization/rejection; candidate never inherits Meets requirements |
| F08 partial/conflicting review | Current projection says required failure plus stale other outcomes; alternative input has threshold conflict and no established failure | Needs rework in first case; Unassessed in second; preserve unknown/conflicting details; no averaging |
| F09 optional-only correction | Accepted result remains pass; only optional evidence becomes stale | Mark detail stale; overall pass and summary revision stay unchanged; no new required-result action |
| F10 material correction round trip | Result 1 digest unchanged; summary q1 pass becomes q2 Unassessed, then q3 pass after corrected required evidence | q2 and q3 are new/unviewed, even when q3 resembles q1; opening q1 cannot satisfy q3; no ordinary unread acknowledgement |
| F11 restart fencing | Old response under G(sidecar-a) arrives after G(sidecar-b); then fresh server snapshot revalidates result 1/E1 | Drop old response; show retained review only from fresh snapshot; unchanged material summary keeps q1; no restored write grant |
| F12 independent action version | Same K1/result/summary; plan decision advances from version 4 to 5; version-4 command is attempted | Disable/reject stale target, refresh exact proposal; inspection grants nothing; no inherited approval or silent retry |
| F13 capacity and missing summary | Task reports complete but child-a is alive, or reservation remains; alternative has terminal coordination but missing summary ID | Keep authoritative capacity; no automatic collapse in either case, no assumed-read result |
| F14 incomplete action refresh | Last complete snapshot had zero decisions; refresh fails, is partial, or exceeds bounds | Action state unavailable; no No actions claim or enabled stale decision; canonical switcher remains usable |
| F15 inspection and workspace race | Inspect K1 while another terminal is retained; switch workspace before K1 details arrive | Inspection changes no active session/unread; late details never paint; explicit terminal opening remains exact-ID ordinary selection |
| F16 root review versus child evidence | B1 result has explicit user review; S1 is otherwise valid child evidence | Root result may display authoritative review; S1 cannot fill a root badge; never manufacture child-binding members for B1 |
| F17 hierarchy unavailable | Snapshot for R is partial, corrupt or oversized, although the canonical session list is valid | Workflow structure unavailable, no guessed edges/empty-tree success or No actions; stale workflow decisions disabled; ordinary switcher usable |
| F18 immutable root details | B1 bootstrap selects orchestrator role/skill-a and root criterion C1; a later exact plan selects child skill-b and criterion C2 | Root keeps bootstrap role/skill-a/C1 and goal; child shows its own skill-b/C2; no parent reload or transferred delivery claim |
| F19 delivery without use | S1 has a current verified delivery receipt; reported/observed usage both Unknown with no countable stream | Filled Loaded badge with check/text; usage Unknown, counts unavailable, no usage highlight or claim of adherence |
| F20 root receipt sent to child route | A caller supplies B1 plus invented plan/task/reservation members | Reject evidence; root stays selected/unconfirmed/unknown with no count or highlight; parent launch delivery gate remains mandatory |
| F21 lost summary lineage | Retained A1/result 1/E1 pass exists but its durable summary row is missing/corrupt | Summary unavailable; do not recreate revision `"1"`, mark viewed, or auto-collapse; source review is not silently rewritten |
| F22 summary exhaustion | A1 reaches `"2147483647"`; an authorized required-evidence correction changes its disposition | Accept/fence the source correction under its own rules; summary unavailable, no wrap or old current pass, no automatic collapse |
| F23 correction publication crash | #742 report-head correction commits while a dependent summary is read; crash before/after the transaction boundary | Complete old head/old summary or complete new head/new revision; otherwise affected reads unavailable; never new head with current old pass; event replay cannot increment twice |
| F24 subject purge and retained row | A1's purge succeeds while its navigation key remains in retained plan history | No result summary/viewed token reused; show summary unavailable and prevent automatic collapse; another assignment's revision `"1"` cannot join A1 |

For F10/F11, q1/q2/q3 denote the concrete canonical revision strings
`"1"`/`"2"`/`"3"` within A1, not globally unique tokens. In F10 both committed
transitions advance the durable row, including the return to pass; a restart
between them cannot reset its counter. F23's complete-old case applies only
before the correction commits; after commit, old currentness is forbidden.

Expand these into exact byte/record fixtures only after the upstream agreement
below. The broader hierarchy contract retains keyboard, responsive/zoom,
summary disclosure, invalid lineage, large-collection and return-navigation
cases; this handoff does not replace native accessibility/usability validation.

## Proposed transport follow-up

The [snapshot and navigation transport proposal](2026-10-17-hierarchy-projection-transport-design.md)
defines a separate run-scoped envelope, volatile snapshot ordering, exact
inspection targets and lossless display DTO encodings. It leaves canonical
record bytes/digests intact. Producer mapping/owner acceptance, full payload
schema, read authorization and runtime plans remain pending; this link does
not finalize the upstream records or establish workspace-wide completeness.

## Review disposition and next gates

| Owner/surface | Concrete agreement requested | Current disposition |
| --- | --- | --- |
| #741/#742 | Confirm bootstrap/run/plan joins and allocation sources for taskVersion/reservation/session; declare snapshot ordering and exact decision targets separately from immutable records | Proposed mapping; source types/routes are not final |
| #743 | Confirm complete child evidence join, source/count/coverage and stable-event behavior; review explicit initial root-telemetry deferral | Deferral recorded in #741/#743; child mapping and written acceptance remain pending |
| #744 | Confirm complete root/child result/reviewer references, outcome freshness, candidate separation and producer-owned summary revision transitions | Durable summary rule recorded in #744; report-based outcomes consume #742's written correction-head contract, with acceptance/joint transaction review still required |
| #745 | Confirm optional opaque repository/history references; no shared custody or learning authority inferred | Independent history/custody gates remain |
| #746/owner | Accept exact mapping/fixtures and material summary rule, or request changes; this session's drafting authorization is not acceptance of the written contract | Written acceptance pending |

After agreement, produce separately scoped executable plans for validated
sidecar projection/session joins and pure renderer hierarchy state, retaining
Electron-main authority and independent preload/renderer definitions. Exact
wire schemas, numerical projection-version/summary bounds and recovery behavior
must be reviewed before code; #740 eligibility and #610 approvals remain runtime
gates. Keep #746 open. Merging this handoff records a proposal, not owner or
component-owner acceptance, new runtime authority, or completion of the initiative.

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
corrects the cross-contract task-version equality implied by the earlier mapping.
The accepted #742/#744 admission-version boundary and #743's own evidence binding
remain unchanged; no source record or digest bytes are rewritten. The review
also identifies a missing authoritative current-summary/outcome detail projection.
Keep the broader consumer handoff pending that explicit scope/codec decision and
producer agreement; a fixture-backed UI preview is not production source proof.
