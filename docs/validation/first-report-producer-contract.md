---
type: Contract Investigation
title: First research-report producer and publication contract
description: Source-traced first-report admission, history and effect obligations, with explicit research-profile and storage-catalog prerequisites instead of synthetic producer adoption.
tags: [orkworks, reports, preparation, producers, history, qualification]
status: draft
workflow_status: gate-2-investigate-research-source-closure
---

# First research-report producer and publication contract

Tracking: [#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Source-owned specification gap: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689).
Inspection base: `ae5e04cdbbd70c368c704c5086932d54dae2d57a`.

**Disposition: Gate 2 remains Investigate.** This derives the first-report
producer contract from accepted sources and identifies the concrete missing
research source/control profile. It chooses no new codec, default, command tag,
authority cutoff, migration or storage format. It supplies no dependent fixture,
runtime/native execution, model probe, adapter or executable plan. Existing
protocols, caps, source bytes and validation artifacts remain unchanged.

## What the earlier acceptance supplies

| Accepted input | Usable contract | Qualification limit |
| --- | --- | --- |
| [#742 report submission and correction](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#authenticated-report-submission) | Assignment-bound child capability, original runtime generation, stream/head/request rules, rate-before-lookup and atomic publication | A logical operation is not a closed research assignment/control producer or a native commit fence |
| [ResearchReport source codec](../superpowers/specs/2026-10-28-research-report-source-codecs-design.md#closed-descriptor-and-inherited-scalar-types) | Exact descriptor/findings/reference leaves and existing source digest, question/scope and excerpt joins | Valid report bytes cannot supply missing approved research source/custody |
| [Terminal storage registry](../superpowers/specs/2026-10-29-report-publication-profile-design.md#report-head-and-receipt-registry) | Compact head and v1 historical child request/receipt mapping, exact closed response | New writes in that profile are terminal user corrections only; it does not authorize initial child publication |
| [Fresh report-ledger acceptance](../superpowers/specs/2026-11-11-report-retention-initialization-design.md#scoped-owner-acceptance--2026-11-12) and [offline witnesses](report-ledger-initialization.md) | Explicit fresh workspace/run report zero-ledger creation and original proof joins | Every witnessed report balance is zero; no legal report-producing history or full child-profile initializer |
| [Terminal UI authority protocol](../superpowers/specs/2026-11-10-terminal-ui-authority-protocol-design.md#scoped-protocol-acceptance-and-component-handoff--2026-11-10) | Exact terminal/evaluation-only user-correction admission and receipt-v2 metadata | Cannot create an initial report, authenticate a child or extend child persistence past revocation |

## Source/control boundary found by inspection

The accepted [retained child source slice](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md#outcome-and-selected-slice)
supports a verification worker or review/verification reviewer, not a research
producer. Its ability to consume existing report evidence is not the ability to
produce research reports. ChildSourcePlanV1 carries no research-question/task
grammar. The accepted [selected controls](../superpowers/specs/2026-11-02-child-launch-control-design.md#decision-and-selected-scope)
are explicitly direct execution: DirectDecisionV1 has researchNeeded false;
material research blocks that variant. ChildRunControlV1 has no researching or
synthesizing state. Do not relabel these records, set a new role/tag or substitute
an empty reviewInput to manufacture research eligibility.

The initial run_states snapshot is historical under that selected child profile;
current runVersion belongs to child_run_controls. The terminal report profile
instead defines its own terminal run_states variant as sole mutable version owner.
These placements are independently scoped contracts, not two counters that can
be updated interchangeably. A real research/terminal path must close its current
owner and full transition effects before either placement can be reused.

| Current source seam | Observed fact | First-report consequence |
| --- | --- | --- |
| [hierarchy_storage/mod.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mod.rs), mechanics module | Generic hierarchy mechanics are cfg(test); module states they cannot admit/publish an accepted production profile | A dormant helper is no real first-report publisher |
| [schema.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/schema.rs), CATALOG | Partial 17-family test fixture, explicitly not the accepted 33-family profile; no report_heads/report_receipts families | Empty tables or generic heads cannot establish this compact report registry |
| [reload.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/reload.rs), CLOSED_COMMAND_KINDS | Recognizes the string report-publication | Recognizing a tag supplies neither a producer nor its complete effects |
| [publication.rs](../../crates/orkworksd/src/taskmaster/hierarchy_storage/mechanics/publication.rs), journal_object / journal_child_metadata / journal_dependency | Separate generic object/metadata/edge journal operations, as bounded by the [accepted seam audit](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md#merged-kernel-seam-check-and-handoff) | Sequential calls cannot stand in for one atomic report/head/receipt/run/preparation/resource outcome |

This is bounded source inspection, not proof that every differently named or
external writer has been inventoried. No Rust, database, native process or
application runtime was executed.

## Producer-owned admission and immutable history joins

The missing research profile must supply the following original records and
current controls. These are required joins, not newly defined record bodies.

| Producer-owned input | Exact obligation before first acceptance |
| --- | --- |
| Run/root | Original UI-admitted run, immutable bootstrap/configuration and run definition; workspace, current admissionEpoch/runSequence and retained current run/version owner agree |
| Research plan/task | Exact approved plan revision/digest and task, assigned sorted question IDs, question/source/stopping scope, report contract, configuration and approval history; retain full definitions rather than a report-shaped context |
| Reservation/allocation/attachment | One original attempt, successful reservation and frozen assignmentTaskVersion, original childSessionId/configuration/worktree allocation and unique lineage attachment; neither an ordinary session nor matching IDs reconstructs them |
| Current child runtime/report authority | Actual active sidecar/launch generation and assignment-bound ResearchReportCapability, checked before receipt lookup. This is separate from parent run/grant, ordinary workflow report and #744 evaluation authority |
| Live task versus frozen identity | expectedTaskVersion compares the actual live task control. Frozen assignmentTaskVersion remains the successful reservation snapshot; do not recapture/reset it to fit a report request |
| Source custody and complete membership | Approved question/scope/reference bindings, exact retained excerpt/revision or retrieval facts and source-owner inventory; neither a URL/path nor digest authorizes a fetch or proves bytes retained |
| Report resources and originals | Existing workspace/run report ledgers and all applicable preparation/control/source resources reconcile complete retained ownership, with legal original initialization proofs |

### Retained R2 requests do not close the live task-version join

Inspection of `r2-report-vectors.json` finds c1-v1, c2-v1 and o-v1 each use
expectedTaskVersion **1** with reportVersion 1. The component model's
[emptyCapturedState](r2-report-components.mjs) supplies task version 1, rather
than deriving it from actual reservation/start history. By contrast, the accepted
[selected reservation contract](../superpowers/specs/2026-11-02-child-launch-control-design.md#task-launch-and-ordinary-session-controls)
requires expectedTaskVersion+1 = frozen assignmentTaskVersion; its task versions
are positive. A successful reservation in that selected chain has version at
least 2, before any later live-control advances.

Those retained requests therefore cannot be adopted as evidence of that real
selected reservation chain. This does not invalidate their documented synthetic
source/CAS/correction tests or choose a new research task-version rule. The
research owner must close its actual current-version trace under the unchanged
reservation/one-attempt contract. A future producer witness needs its own exact
normalized request and fingerprint for that trace; never edit/reset counters or
silently rewrite the original R2 vectors to advertise admitted history. Report
descriptor bytes have separate source/lineage joins and are not automatically
qualified by changing the request's expectedTaskVersion.

## Exact first-report request and acceptance order

Use the existing child submitResearchReport operation, not terminal UI correction.
The normalized fingerprint remains schemaVersion 1, operation submitResearchReport,
the complete eight-field stream S, expectedTaskVersion, expectedRunVersion null,
expectedReportHead null and the complete accepted ResearchReport. reportVersion
is exactly 1; child author and runtime generations are sidecar-derived. The
descriptor includes contentDigest; request and descriptor independently fit 32 KiB.
No new route, transport field, token or digest domain is selected here.

1. Validate current assignment/child/runtime authority, current epoch, original
   owner/lease/health and authenticated child request rate before lookup. Ten
   requests per rolling minute per child still includes duplicates. Completed/
   cancelled final runs and ended/replaced child runtimes cannot replay a child
   receipt through a revoked capability. Stage completion alone retains the
   existing active-run reporting rules; a missing stage codec grants nothing.
2. Validate complete bounded schema, approved identity/question/scope/custody and
   sidecar-derived provenance. Reconstruct canonical request bytes and compare
   any accepted binding exactly, not just its digest. For initial publication,
   derive identity from original research assignment/runtime, not a predecessor
   report because none exists.
3. A proven authorized identical accepted request returns its original receipt
   before live task/head CAS, even after later correction. It changes no stored
   row, run/task counter, charge, material, snapshot or event. Changed bytes for
   the same accepted key conflict. Missing head/history/proof is not permission
   to manufacture an accepted retry or recreate version 1.
4. For a genuinely new request, prove complete stream/history absence, compare
   the actual live task version, then enforce exact initial version, all caps,
   full read-set/current source/consumer fences and the owning child cutoff.
   Any source/control drift denies this attempt; do not retarget the request.
5. Commit the complete source-owned effect below before success. New acceptance
   is 201 and exact accepted retry is 200 with identical closed response bytes
   `{schemaVersion:1,acceptedHead:H}`. Receipt inspection is not current-head
   proof, task success, capacity release, grant restoration or viewed marking.

This is the accepted logical order, not proof of its physical enforcement.
#744 claim-first remains excluded. The narrow terminal UI exception gives no
initial-child write permission after child revocation; transport abortion or
suppressed response cannot prove persistence denial. The actual research producer
must qualify its original cutoff without holding input-critical locks through
SQL/fsync or making ordinary stop wait for it.

## Complete publication responsibilities

These groups describe the required union, not an executable generic row list or
an approved complete command with a fabricated total. The report side has closed
shapes; active research/control/preparation/custody effects still require #1689.

| Effect group | Required source-owned change / preserved invariant |
| --- | --- |
| Immutable report | Insert exactly one research-report object for the validated descriptor and full #744 ReportIdentity. First acceptance fixes reportId for this exact stream; no overwrite or alternate-ID bypass |
| Authoritative head | Insert the compact report_heads[S] pointing to that version/digest; no duplicate generic head or maximum-version selection |
| Original receipt | Insert the original v1 submit request/response binding for `{stream:S,reportVersion:1}`. expectedTaskVersion is the original live CAS value, expectedRunVersion/expectedReportHead null. Do not add terminal v2 authorization or #744 slots/claim/lookup |
| Current run owner | Checked runVersion+1 once in the actual research current-state owner; no second mutable counter beside an initial seed or terminal cache. Terminal state is not created by report publication |
| Active preparation/collection | For every source-owner-identified affected active preparation, create its exact immutable input revision and current pointer/fences atomically as #742 requires. Report receipt itself creates no successful coordination outcome, task-version advance, plan completion, launch or capacity refund |
| Consumers/custody/dependencies | Include all actual source/custody and dependency effects identified by complete owning inventories. Preserve exact retained old references; do not invent report credibility/evaluation edges merely to populate a graph |
| Resources | Charge full new report object/head/receipt occurrences to existing run and workspace report ledgers; coalesce actual preparation/control/source/summary/custody owner deltas once per key. Do not initialize/reset a missing resource in this command |
| Publication controls | One header/new proof outcome with every logical before/after effect, required original proof links and physical lookup maintenance; no later journal call or post-commit registration to complete the command |

No new #744 evaluation, result, disposition, artifact, receipt credit or material
revision follows merely because a report exists. A research finding is not a
required-rework finding. Derive any genuine affected aggregate/lineage protection
from its exact source owners; unchanged material creates no revision. If required
unavailable/reconciling protection cannot commit, reject source publication rather
than expose a partial head. Source-run and dependent tracked-run detail snapshots
invalidate for visible changes even without a material change, before another
current read. Commit notifications are bounded hints after publication, not
history reconstruction or authority for automatic execution approval.

### Why first-report absence does not settle consumers

In a coherent store, an exact retained evidence reference cannot already resolve
an unaccepted/missing version-1 object. Such a dangling reference is corruption;
first publication cannot repair it by creating the target. However, active
research can already have task/report expectations, collection selections or
preparation relationships that are **not** references to an existing immutable
report. Absence of a head or incoming evaluation edge therefore proves neither
an empty active affected set nor that preparation/input updates are unnecessary.

The research owner must define those expectation/selection owners and complete
membership under the workspace boundary. Exact immutable evidence retention and
prospective stream/collection state are distinct responsibilities. Do not invent
a new dependency purpose, infer an empty set from a caller list or apply the
terminal evaluation-only exclusion to an active researching run. Read-only peers,
historical consumers, concurrent insertion/removal and purge must participate.
Count full discovery, repeated lookups and final validation under 100,000 visits;
overflow denies without truncation or split publication.

## Accounting, format and recovery boundaries

Preserve the independent 32 KiB descriptor/request ceilings, 512-byte complete
head-value ceiling, 2 KiB complete receipt key/value ceiling and eight total stream
versions; head keys still count in aggregate/command storage. Charge report
resource occurrences with only their named outer links
excluded; descriptor/request/response bytes and hashes retain accepted domains.
Run preparation/report still share 32 MiB, workspace report usage stays within
256 MiB and each smaller applicable source/control/preparation limit also wins.
Excerpt custody is additional real ownership/accounting, not made free by a
descriptor's digest or charged to #744 protected receipt pools by analogy.
Keep complete 16 MiB/4,096 changed-row and all retained logical/proof limits.
No complete byte/row total is qualified until all effect groups are enumerated.

The accepted terminal and fresh-ledger witnesses use **format-3** storage domains.
The accepted selected child/control catalog uses **format-4** and one complete
mandatory catalog, not alternative profiles chosen from present rows. A source
digest staying equal does not make differently committed rows, receipts or
manifests interchangeable. The fresh-ledger acceptance does not initialize the
full child/source/control/artifact resource set. #1689 must reconcile the chosen
research catalog, version placement, resource creation and command effects with
their owners before dependent complete-state fixtures. This document chooses
neither conversion nor a new/partial format-4 catalog.

Recovery must yield complete old/new state or unavailable reconciliation, preserving
original report/request/receipt/source/custody/proof/resource history. Never elect
the largest version, synthesize receipts from objects, backfill zero ledgers,
retoken old proofs, reset counters or restore volatile credentials. A proof tag,
receipt-only manifest, seed subject-preparation command or model-supplied producer
boolean cannot establish a legal creation/acceptance history. Denied operations
preserve retained bytes and do not opportunistically repair corrupt state.

## Required next source closure and review disposition

[#1689](https://github.com/Rambolarsen/orkworks/issues/1689) requests one concrete
source-owned research profile: question/task/configuration/plan grammar; original
approval/reservation/attachment/runtime joins; current research control/version
ownership; active preparation/collection/custody membership; and a complete
catalog/initialization/first-report effect and recovery map. Preserve accepted
report protocols and child cutoffs. Any new codec/default or semantic choice
needs scoped owner acceptance before dependent fixtures. This is a missing source
contract, not a request to reopen #740 research or run a native probe.

That closure must identify exact durable record families/keys, their source-owner
modules, complete indexes and transaction read/write sets. Repeating the abstract
owner obligations here is insufficient; it must make each affected set and
resource occurrence derivable without a caller-supplied completeness flag.

Real finish/cancel effects and complete terminal consumer/index membership remain
separate Gate 2 requirements. Existing R2 C1-required versus O-optional corrections
and current C2 failure retain their exact accepted bytes/material/snapshot oracles;
this investigation does not rewrite them or add production provenance. Keep
#1089/#742/#744/#746 open. This document is ready as a written contract investigation,
not a closed producer profile, Gate 2 completion or executable handoff.

Two bounded fresh read-only Luna reviews found no material correctness or builder
handoff defect. The correctness pass checked the captured-version inference,
authority/effect distinctions and accepted source citations. The builder pass
identified exact record-family/owner/index/read/write mapping as the next source
contract's central obligation; the author made that requirement explicit above.
No fixture, native process or runtime was executed by either reviewer. Review
qualifies this written investigation only, not the missing research producer.

## Immutable research-source proposal — 2026-11-12

The [research source-codec proposal](../superpowers/specs/2026-11-12-research-source-codecs-design.md)
under #1689 supplies concrete new immutable source grammar, distinct from the
accepted direct-execution variants. Captures/questions/excerpts, scope/configuration/
plan approval and original reservation/allocation/attachment identities are proposed
for scoped review. No source component fixture precedes acceptance. Prepared-input/
current-control/catalog/consumer/atomic publisher closures above remain open;
this is no complete producer or Gate 2 qualification.

## Accepted capture/scope component evidence — 2026-11-12

The owner has accepted the four immutable research-source choices for offline
source-component fixtures only. The [first bounded packet](research-capture-scope-components.md)
checks captures, questions, excerpts and immutable task scope against supplied
synthetic bindings. It does not close configuration/startup, original approval/
launch sources, prepared input, current controls, report consumers, accounting,
catalog or atomic first-report publication. #1689 remains open and Gate 2 remains
Investigate. All earlier source/control/report vectors retain their original bytes.

## Research configuration/startup component continuation

The [synthetic configuration/startup packet](research-startup-components.md) adds
complete v3 research source-shape, common evidence and startup composition checks
under the existing scoped acceptance. Actual capture/admission/model delivery and
tool mapping remain unqualified, as do the original source chain and all current
control/catalog/consumer/atomic-publication obligations above. Gate 2 remains
Investigate and #1689 remains open.

## Original research-source chain component continuation

The [source-chain packet](research-chain-components.md) supplies independent
synthetic original plan/approval/reservation/allocation/attachment/assignment
bytes, exact expected/frozen identity joins and research source-domain denials.
It embeds the startup packet unchanged. Prepared input, symbolic commit links,
native events and legal command history remain unqualified. Next written closure
is prepared input/current controls with exact original-versus-live version and
source-owner effect ownership; complete catalog/accounting/consumer/atomic
publication closure still follows. #1689 remains open and Gate 2 Investigate.
