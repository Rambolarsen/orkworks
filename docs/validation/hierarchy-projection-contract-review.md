---
type: Contract Review
title: Hierarchy production projection contract review
description: Source-backed review of hierarchy identity joins and the missing current-summary detail read.
tags: [orkworks, hierarchy, projection, review]
status: draft
---

# Hierarchy production projection contract review

Reviewed on 2026-10-26 against main
`288bcede6a0d8bcd9474ab9decae5215b55102d6`, including the merged synthetic preview
in [PR #1078](https://github.com/Rambolarsen/orkworks/pull/1078).
Tracking: [#746](https://github.com/Rambolarsen/orkworks/issues/746), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).

**Disposition: Revise before the full production UI/detail handoff.** One
source-join correction is applied to the mapping and payload drafts. One
read-projection gap remains a scoped design decision. This is an authored
specification review, not independent producer agreement, owner acceptance of
a new codec, runtime implementation or native qualification. The other session's
root-preparation fixture work is outside this review and was left untouched.

## Scope and sources

Review the [mapping](../superpowers/specs/2026-10-15-hierarchy-projection-mapping.md),
[transport](../superpowers/specs/2026-10-17-hierarchy-projection-transport-design.md),
[payload](../superpowers/specs/2026-10-17-hierarchy-projection-payload-design.md), and
[read/recovery contract](../superpowers/specs/2026-10-20-hierarchy-read-recovery-design.md)
against their source contracts and the
[UI's result-detail requirements](../superpowers/specs/2026-10-04-agent-hierarchy-ui-design.md#result-identity-freshness-and-review-details).
The preview demonstrates presentation interactions only; its local types are not
an approved production DTO or proof of a valid source join.

| Area checked | Review result / evidence boundary |
| --- | --- |
| Workspace/run/lifetime and read authority | Transport binds exact scope and current request; payload derives workspace/lifetime in Electron and uses the existing UI authority; no renderer bearer or path capability |
| Ordering and stale response handling | Tracked versus stateless responses, latest-request failure fences, terminal counter exhaustion and coherent publication are specified; runtime writer coverage and race proof remain separate |
| Frozen result identity versus skill binding | Incorrect blanket equality corrected below; source-specific version checks remain mandatory |
| Root and child separation | Bootstrap identity and deferred root skill telemetry stay distinct from child evidence; no fake root reservation or profile eligibility |
| Capacity and allocation recovery | Exact task membership, separate run/current-plan ceilings and visible unresolved ownership are specified; unavailable child lifecycle cannot imply zero capacity |
| Evaluation and material summaries | Sidecar owns current aggregation and durable material lineage; the UI's needed current detail values have no complete read shape, as below |
| Encoding and bounds | Source digests remain over canonical source bytes; display u64 values are strings; byte caps win over structural maxima; no schema/native qualification inferred |
| Ordinary session display | Lifecycle, attention, connectivity and terminal-outcome names match the current renderer `api.ts` declarations; actual producer/serializer integration still needs implementation evidence |

## R1 — Do not equate independent task-version meanings

**Impact:** The old payload rule required all child assignment members to match
the #743 skill binding. Both contain `taskVersion`, but they no longer have the
same meaning. A builder could reject valid current skill evidence or rewrite
an immutable result identity in an attempt to make the versions agree.

The accepted
[#742 admission-version amendment](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#proposed-evaluation-admission-version-snapshot)
freezes `assignmentTaskVersion` at reservation. It explicitly keeps current task
CAS and #743's evidence binding separate. The
[#744 reconciliation](../superpowers/specs/2026-10-04-assignment-evaluation-design.md#configuration-and-reporting-reconciliation--2026-10-23)
copies that frozen value into every child `AssignmentIdentity.taskVersion` and
preserves it across task changes and resumes.
[#743](../superpowers/specs/2026-10-04-skill-usage-evidence-design.md#assignment-and-reporter-binding)
requires its own server-resolved version and generation checks, with exact
replays verified against the stored launch binding.

**Correction applied:** Match the shared run/plan/revision/task, reservation,
parent/child and configuration fields; validate each task-version value against
its own owner. Do not require cross-contract equality or substitute one version
for the other. The mapping's S1 alias and both drafts' synthetic cases now make
this distinction explicit. No new #743 capture/advance rule is selected and no
source schema, hash domain, generation or reporting authority changes.

Review counterexample: frozen evaluation version 1 and independently source-valid
skill-binding version 2 may name the same child/reservation/configuration/latest
launch. The old blanket equality rejects that join; the corrected rule checks
each source separately. If the skill binding itself proves version 3, a claimed
version 2 still fails. Merely matching the frozen evaluation version cannot make
invalid skill evidence current. These are logical review cases, not executed
source-valid fixtures or proof of an eligible adapter.

## R2 — Current aggregate detail has no read representation

**Impact:** A production UI cannot fulfill the proposed current completeness,
quality, finding and outcome-detail requirements using the two read shapes.
It would need to invent freshness/aggregation in the renderer or show reviewer
claims as if they were current authoritative findings.

The [closed overview summary](../superpowers/specs/2026-10-17-hierarchy-projection-payload-design.md#evaluation-summary-and-references)
contains only `summaryRevision` and `overall`; it explicitly puts completeness,
quality, outcomes and findings into later inspection details. The accepted
[narrow record read](../superpowers/specs/2026-10-20-hierarchy-read-recovery-design.md#exact-result-and-evaluation-detail-reads)
returns exact immutable result/evaluation bytes and a **whole-record** current or
historical annotation. It forbids deriving aggregate pass or per-outcome
freshness from that annotation. Neither shape exposes #744's current aggregate
completeness/quality or its current required finding/outcome classifications.
The durable `SummaryMaterialV1` has some of these values internally, but it is
not a response member or supported detail selector.

Concrete indistinguishable-detail case:

1. One immutable evaluation cites separate evidence for required failures C1,
   C2 and an optional outcome O. Accepted result and evaluation bytes are retained.
2. In history A, only C1's source becomes stale; C2 remains a current failure.
3. In history B, only O's source becomes stale; both C1 and C2 remain failures.
4. Both reads return the same immutable evaluation bytes and whole-record
   `historical`; both overviews still say Needs rework. Snapshot/summary tokens
   can differ but are opaque identities, not a per-outcome explanation.
5. The UI cannot tell whether C1 is a current required action. It must neither
   mark all findings stale nor promote all historical claims to current findings.

The summary-viewing contract has the same boundary: opening one immutable record
is not proof that the current material summary was displayed. A changed summary
token alone supplies no missing current detail content. No auto-collapse/viewed
claim follows merely from decoding an evaluation record.

**Requested next specification:** Tracked by
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089). Define a bounded, source-owned current-summary/
aggregate detail read (or explicitly defer those UI details and dependent
viewed-summary behavior in the first production slice). A proposed read must
name exact assignment/result/summary/snapshot binding, authoritative fields and
freshness, unavailable states, byte/work budgets and stale-selection behavior.
Reuse #744 aggregation and accepted material lineage; do not modify the immutable
record codec, increase existing caps, or add raw evidence/file access implicitly.
New read fields/authority/defaults need their scoped design review before code.
This review does not choose a new endpoint or accept a DTO by implication.

## Remaining gates and next action

The correction in R1 reconciles an already accepted source boundary; it does not
establish broad owner/producer acceptance of #746. Keep #746 open. Next, write
and review the bounded R2 detail-projection proposal or an explicit first-slice
deferral, then obtain the relevant producer agreement before an executable UI/
projection plan. Complete source-valid fixtures, writer/storage and native
qualification remain separately tracked prerequisites. #740 stays deferred/no-go
until November; this review ran no model, capability, native or database probe.

Validation for this review is documentation metadata/index/link/build checks and
source comparison. No new runtime test or production behavior is claimed.

## R2 proposal handoff — 2026-10-26

The [current-summary detail proposal](../superpowers/specs/2026-10-26-hierarchy-summary-detail-design.md)
addresses the missing read shape with an exact current selector, existing material,
source-labelled per-outcome annotations, current metric displays and explicit
viewed-summary presentation. This is a proposed resolution under #1089, not owner/
producer acceptance or a replacement for the immutable-record codec. R1's version
correction and all source/capability/runtime gates remain unchanged.

## R2 scoped design acceptance — 2026-10-28

The owner [accepted the concrete current-summary read, metric display and viewed-summary choices](../superpowers/specs/2026-10-26-hierarchy-summary-detail-design.md#scoped-owner-acceptance-and-review-handoff--2026-10-28)
for documentation fixtures and further qualification after an authored comparison
with #744 aggregation/currentness, read/recovery and #746 viewing contracts found
no blocking mismatch. This resolves the pending owner decision for R2's proposed
read representation; it is not independent producer agreement, executed fixture
evidence or a production UI/detail handoff. The stale-C1 versus stale-optional-O
counterexample remains in the logical case inventory. #1089 remains open for the
bounded fixture and producer/qualification handoff; source/packing, writer/storage,
native, #740 and executable runtime-plan gates remain independent. R1 and all
existing technical definitions and fixture bytes are unchanged.
