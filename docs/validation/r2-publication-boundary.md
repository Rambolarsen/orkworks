---
type: Validation Evidence
title: R2 report publication boundary audit
description: Maps the retained R2 corrections to atomic dependency and snapshot obligations and identifies the closed-schema prerequisites that prevent complete transaction qualification.
tags: [orkworks, reports, hierarchy, storage, dependencies, validation]
status: draft
---

# R2 report publication boundary audit

Tracking: [#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Missing specification profile: [#1136](https://github.com/Rambolarsen/orkworks/issues/1136).
Inspection base: `11ca93890be36b2d46b8f17673b3f1c3196c39c0` (PR #1135).

## Disposition

The [R2 packet](r2-report-components.md) establishes selected canonical source
bytes and two logical correction histories. It does **not** establish an atomic
publication. The existing contracts determine the required externally observable
outcomes, but explicitly leave research-report receipt/ledger rows and mutable
preparation/plan consumers outside the closed storage registry. Complete changed-row
or crash witnesses cannot be qualified without those source-owned definitions.
This audit requests that spec update through #1136; it introduces no new codec,
receipt, storage default, effect command, capability or implementation approval.

Do not reinterpret `applyCaptured`'s accepted/retry objects as production receipts,
its copied maps as an authoritative dependency index, or its sequence of logical
states as a recoverable transaction. Passing the component tests cannot close this
boundary. The supplied run/configuration/UI context remains explicitly synthetic.

## Owning contracts and exact gaps

| Authority | Existing obligation | Missing qualification input |
| --- | --- | --- |
| [#742 report publication](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#current-report-head-and-corrections) | Descriptor, exact request/receipt, head, run version and dependent freshness/preparation effects publish together; current authority and rate admission precede receipt lookup | Closed research-report receipt and request-binding storage/response shapes, retention and complete ledger mapping |
| [Storage registry](../superpowers/specs/2026-10-24-hierarchy-storage-contract-design.md#source-registry-and-remaining-schema-work) | `research-report` objects and heads reuse source identity; configuration/plan/proposal consumers require strict additional registry entries | Exact consumer variants, keys, edges and bounds; a named `report-publication` command does not define its complete effects |
| [ReceiptBinding](../superpowers/specs/2026-10-24-hierarchy-storage-contract-design.md) | Closed union contains #744 mutation and candidate-finalization bindings; research-report receipts explicitly need their own registry/ledger shapes | Reports cannot be coerced into #744 receipt slots or borrow their protected credits |
| [Initial preparation effects](../superpowers/specs/2026-10-28-root-preparation-effects-design.md#scope-and-rejected-alternatives) | Initial-only proposal excludes reports, plan/task consumers and mutable lifecycle variants | No accepted initial-run row can stand in for report corrections to an active or terminal run |
| [Dependency boundary](../superpowers/specs/2026-10-04-assignment-evaluation-design.md#local-dependency-boundary-and-subject-purge) | Incoming/outgoing retained membership changes atomically with its owner; final deletion scan serializes with insertion | Exact selected consumer mapping, bounded complete affected-set query and index recovery evidence |
| [Summary detail coherence](../superpowers/specs/2026-10-26-hierarchy-summary-detail-design.md#coherence-failures-and-budgets) | Every detail-visible change invalidates the run snapshot under the publication boundary, including unchanged material | Writer coverage and capture/final-validation evidence for these report-derived fields |

These gaps were found by inspecting the owning sections and the existing candidate
command manifests and authority-ordering scope. The latter's #744 claim-first
exception [excludes research reports](../superpowers/specs/2026-10-24-hierarchy-authority-ordering-design.md).
Neither the report codec acceptance nor this audit expands that exception.
No claim of an exhaustive runtime caller/direct-write inventory is made here.

## Required effects of the retained R2 histories

Use the packet's original version-1 C1/C2/O sources and one immutable evaluation.
The catalog contains alternative versions; only replayed accepted head transitions
select a history. The current C2 failure must survive both corrections.

| Required observation | A: correct required C1 | B: correct optional O |
| --- | --- | --- |
| Published source/head | Exact C1 version 2 descriptor, digest and authoritative head | Exact O version 2 descriptor, digest and authoritative head |
| Historical source/evaluation | Original v1 descriptors and cited source keys remain retained; evaluation bytes/digest unchanged | Same retention and immutable evaluation rule |
| Current outcome evidence | C1 stale; C2 remains current; O remains current | O stale; C1 and C2 remain current |
| Current required findings | f-c2; stale f-c1 excluded from current material | f-c1 and f-c2 unchanged |
| Required completeness | Unassessed percentage: one unsatisfied, one unassessed | Assessed: zero of two required satisfied |
| Overall | Needs rework from current C2 failure | Needs rework from current C1/C2 failures |
| Durable material lineage | Evaluated revision 3 advances to 4 exactly once | Retain revision 3; optional-only change creates no material transition |
| Detail-visible invalidation | Snapshot invalidates before any new current detail is served | Snapshot also invalidates despite unchanged material revision and overall |
| Immutable record inspection | Whole evaluation is historical under the narrower record-read proof | Whole evaluation is historical under the narrower record-read proof |

The 3→4 and 3→3 values describe this particular packet, not a revision allocator.
A current summary cannot be inferred from a report classification, receipt, event,
maximum report version or the final equality of material bytes. The response
sizes and conditional general packing proof remain in the
[source/packing audit](summary-source-packing.md); they do not bound a complete
publication command with keys, receipts, dependencies, ledgers and proofs.

### Retention membership and current projection are separate

The immutable evaluation continues to cite the old exact source keys after either
correction. Its retained dependencies therefore continue protecting the original
reports. Correction does not redirect those edges to v2 or delete them because an
outcome became stale. The new report's existence alone grants no evaluation
credibility and does not admit a new consumer edge. The current projection uses
maintained head/currentness proofs while retention protects exact cited objects.

The affected set is source-owned, derived under the same dependency boundary as
edge insertion/removal and source publication. The single evaluation in this
packet is only one selected consumer, not proof that no preparation summary,
proposal, approved plan, launched assignment or learning contribution also cites
the report. A caller list or scan of current evaluation heads is insufficient:
retained records and the other admitted consumer kinds matter. Missing/ambiguous
membership cannot be interpreted as an empty affected set.

For affected active preparation, #742 requires a new input revision. Pending
proposals invalidate; approved plans pause new launches with the existing revision/
reapproval blocker; already-launched assignments retain their original bindings
and gain a visible blocker. Terminal runs remain terminal with no new preparation/
proposal transition. These are required logical effects, **not** invented rows.
If repository learning cannot join this boundary, its affected contributions must
be excluded/fenced before its next summary; the joint #745 gate remains separate.

## Publication and read acceptance oracles

The following cases are contract-derived obligations for future qualification,
not executed concurrency, SQLite, crash or presentation tests.

| Case | Required oracle |
| --- | --- |
| New correction admitted | All descriptor/request/receipt/head, checked run version, affected preparation/dependency and summary/fence effects are durable before success; no reporting-only task outcome/version advancement |
| Exact accepted retry after later correction | Authority and rate checked first; return original saved receipt before live CAS, with no republish, material increment or reinvalidation; receipt is not current-head proof |
| Changed request for accepted key or stale new CAS | Conflict; preserve all accepted state and dependent currentness |
| Authority revoked or request rate exhausted | Reject before receipt lookup; prior success does not revive report authority; rate exhaustion stores no new version |
| Correction versus new citing evaluation/edge | Serialize both: edge commits first and is included in affected protection, or follows and validates current head; no old source admitted as current after correction |
| Correction versus purge/retention | Final membership scan and action serialize; retained old evaluation evidence remains protected; no dangling reference or partial deletion |
| A correction with summary capacity exhausted | Authorized source correction needs durable unavailable protection and intact ordered lineage as required by the owning contract; if protection cannot persist, reject source publication |
| B correction with identical material | No material increment; changed outcome/source freshness still invalidates snapshot before current reads |
| Unknown affected set or corrupt index/descriptor/head | Fence current evidence reads, approvals and affected launches pending reconciliation; never elect the largest surviving version |
| Read assembled before publication, validated after it | Reject stale reference on changed snapshot/selection/head/health; require fresh overview and explicit reselection; never silently retarget |
| Fresh read during missing proof or recovery | Unavailable with no partial current detail, fake zero metrics or viewed identity; narrower immutable inspection may remain usable on its own proofs |
| Commit durable before acknowledgement | Recover complete committed effects before reads/retry; exact retry returns original outcome without duplicate lineage or invalidation |
| Crash during staging or installation | Complete old state, complete new state, or fenced reconciliation; never visible new head with current old dependent evidence |
| Event delayed, duplicated or ring gap | Events remain hints; authoritative head/fence snapshot controls freshness; no reconstruction from arrival order |

A live detail request captured before a successful correction cannot become a
new current success merely because B retains summaryRevision 3. Its old snapshot
is stale. If currentness/lineage is unprovable, use unavailable instead of claiming
a proven snapshot mismatch. Actual rendering and viewed marking require the
accepted explicit-opening/complete-material presentation proof separately.

## Bounded next deliverable and acceptance gate

[#1136](https://github.com/Rambolarsen/orkworks/issues/1136) requests a closed
source-owned report-publication profile: receipt/request storage and quotas;
selected mutable run/preparation/consumer variants; exact dependency discovery;
complete changed-row, summary/fence and ledger effects; command/proof bounds;
and snapshot invalidation coverage. An excluded state must fail unavailable or
be otherwise fenced by its existing owner, never silently disappear from an
asserted complete affected set. New codec/default choices need scoped owner/
producer acceptance before dependent golden transaction fixtures.

After that acceptance, independent before/after/denial witnesses can cover both
R2 branches, exact retries, missing/extra affected consumers and unavailable
protection using the unchanged source bytes. Those witnesses will still not be
native durability or complete admitted-run evidence. Native storage, custody,
authority/lifecycle, #740 capability and separately approved executable runtime
plans remain independent. No live route, database/migration, model/native probe
or runtime implementation is delivered by this audit. Keep #742/#744/#746/#1089 open.

## Concrete profile proposal — 2026-10-29

The [terminal-run correction proposal](../superpowers/specs/2026-10-29-report-publication-profile-design.md)
selects a bounded first storage/effects profile for #1136: user corrections in
terminal runs with retained evaluation consumers only. It supplies proposed closed
receipt/head/terminal rows, edges, effects and quotas while failing closed on
unsupported consumers. New choices await scoped owner/producer acceptance; no
dependent transaction fixtures or native/runtime qualification are claimed.
