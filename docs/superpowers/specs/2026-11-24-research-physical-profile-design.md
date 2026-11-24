---
type: Design Proposal
title: Research profile physical catalog and boundary
description: Proposed written physical format, additive profile registration, family/index/proof mapping and limit accounting for the retained-file research profile, with every unavailable authority/native/physical qualification retained as an explicit no-go boundary.
tags: [orkworks, research, storage, physical, qualification]
status: draft
workflow_status: proposed-for-scoped-physical-profile-review
---

# Research profile physical catalog and boundary

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738); consumer
[#610](https://github.com/Rambolarsen/orkworks/issues/610).
Inspection base: `65ae7351f4e54ca40e24ebdc73fca2c471b2794e` (origin/main),
including the command/proof acceptance merged in
[#2026](https://github.com/Rambolarsen/orkworks/pull/2026).

**Proposed for scoped owner review, for written physical-profile boundary work
only.** This is [source-coverage audit item
5](../../validation/research-source-coverage-audit.md#catalog-indexes-initialization-and-proofs-still-required):
it selects, as a written proposal, the full format/commitment/proof/index/
recovery contract for the recognized `retained-file-research-v1` profile over
the accepted [catalog enumeration](2026-11-23-research-catalog-closure-design.md#enumerated-logical-catalog),
[discovery/invalidation ownership](2026-11-23-research-discovery-invalidation-design.md#complete-discovery-inventory),
[three initializer outcomes](2026-11-23-research-initializer-manifest-design.md#the-complete-creation-manifest)
and [command/proof recipes](2026-11-23-research-command-proof-design.md#per-operation-effect-recipes) —
**without retokening format-3/4/6 artifacts**. It supplies written
representations and arithmetic only: no executable command codec, database,
migration, runtime/native/model execution, adapter or implementation plan.
Gate 2 remains **Investigate**.

## Outcome and preserved decisions

Make the physical profile enumerable the same way the logical chain made the
catalog, initializer and command unions enumerable. A reader of this document
can derive: which physical store and format version hold the research catalog,
how every enumerated family and object kind maps to physical rows, how the
discovery inventory's both-direction obligation is realized physically, which
commitment/proof/recovery representation each outcome carries, and which
accepted limits the written arithmetic satisfies versus which terms remain
physically unqualified.

Preserve unchanged: every accepted logical key, body, digest domain, wrapper,
exclusion, receipt, resource name, pool placement, version ceiling and
permission rule from the merged contracts; the accepted formats 3/4/6 codecs
and their exact exclusions as the authoritative representation of their own
rows; the accepted [authority-ordering policy](2026-10-24-hierarchy-authority-ordering-design.md#owner-acceptance-and-handoff--2026-10-24)
and every excluded action's own cutoff; and the [writer/crash oracles](../../validation/hierarchy-writer-storage-qualification.md#storage-and-crash-evidence-packet),
which this document restates as obligations, never as executed evidence.

Rejected alternatives: one physical store per profile (the accepted initializer
outcome 2 spans research and inherited core-admission families in **one**
outcome; a second store would need exactly the cross-owner transaction this
chain refuses to invent); retokening or re-encoding existing format-3/4/6 rows
into a new domain; per-family mixed codecs inside one manifest; a
research-only subset reader; and raising any limit to make a profile fit.

## Source trace and obligations

| Source | Constraint carried into this proposal |
| --- | --- |
| [Audit item 5](../../validation/research-source-coverage-audit.md#catalog-indexes-initialization-and-proofs-still-required) | Select the full format/commitment/proof/index/recovery contract without retokening format-3/4/6 artifacts; satisfy every smaller limit in writing; independently qualify authority/cutoff, custody, ordinary storage activation and native bridge participation — which finite written facts cannot do |
| [Compact bindings / format 3](2026-10-25-hierarchy-compact-bindings-design.md#proposed-storage-format-3-and-compatibility) | Closed registry extension, change/row domains with LF, manifest change coverage/order, excluded-link validation, strict no-mixed-codec/no-implicit-migration compatibility |
| [Format-4 source registry](2026-10-31-child-review-source-preparation-design.md#closed-format-4-source-registry-and-commitment) | Whole-catalog reader obligation; canonical BLOB keys/values; per-table exclusion envelopes; 2 MiB row ceiling with smaller per-family limits |
| [Format 6](2026-11-05-child-preparation-authorization-design.md#atomic-commitments-and-compatibility) | New-store-only version bump precedent; union inventory counting (33-family inventory, ordinary_child_lineage 34th, authorization 35th); inherited exclusions for unchanged logical rows; older readers reject newer formats |
| [Initializer acceptance](2026-11-23-research-initializer-manifest-design.md#scoped-owner-acceptance--2026-11-23) | Three distinct original outcomes; registered empty families; additive profile registration with peers preserved byte-identical and no row-family mixing |
| [Command/proof acceptance](2026-11-23-research-command-proof-design.md#scoped-owner-acceptance--2026-11-24) | Additive admission to an existing store preserves the actual H, cannot reset the epoch or create a second null-predecessor outcome; physical index/proof bytes count in complete-command/store accounting |
| [Physical prerequisite evidence](../../validation/pair-native-envelope-input-closure.md#why-this-does-not-close-the-first-gate) | Pre-operation whole-peak growth and scratch qualification remain unavailable; the accepted Windows candidate is unavailable at its first gate; both execution batches stay consumed; no new probe budget exists |
| [Writer/crash oracles](../../validation/hierarchy-writer-storage-qualification.md#transaction-coverage-checklist) | Durable unit, recovery and race acceptance oracles are future qualification requirements, not executed tests |

## The proposed physical profile

### One store, one header sequence

The workspace has **one hierarchy store** with one header, storage epoch and
manifest sequence. Every profile admitted to the workspace lives in that store;
profiles never mix row families, but their outcomes advance the same header.
This is forced by the accepted initializer: outcome 2's manifest includes the
accepted six core admission changes — exact draft deletion, `root_preparations`
insert, `run_states` insert, reserved `parent_attachments` insert,
`summary_state` insert and the initial `research_run_controls` insert
([initialization recipe](2026-11-13-research-initialization-catalog-design.md#logical-initializer-and-admission-effects)) —
alongside the research run owner, selector and ledgers; one original outcome,
one C. (The initializer acceptance's loose "inherited six core source effects"
label covers this same set; the run-controls insert is research-own.) Two
stores would make that outcome a cross-store transaction, which the accepted
command/proof contract explicitly leaves unqualified rather than inventing.

Consequences (inherited except where the format section below marks a
mechanism as this proposal's explicit new choice):

- Fresh workspace admission under this profile initializes the complete union
  catalog through the accepted three distinct initializer outcomes — the first
  (initialize-store) alone uses the accepted null predecessor/initial sequence;
  root admission and attachment preparation advance from the prior header in
  the same store/epoch, per the accepted command/proof contract.
- Additive research admission to an existing store (ordinary/direct-execution,
  child-source or preparation-authorization profiles present) is **one
  original outcome whose predecessor is the store's actual current header**.
  It preserves H's continuity, never resets the epoch, never creates a second
  null-predecessor outcome, and leaves every existing row byte-identical.
- Ordinary/native producers keep their own stores and identities; the ordinary
  bridge, not the hierarchy header, owns their disposition. Nothing here
  qualifies their participation.

### Store format 7 and the closed union registry

Propose **format 7 as the research profile's store format, reached two ways**:
fresh initialization of a new store (null-predecessor outcome), or the one
original additive-admission outcome above, whose predecessor is the store's
actual current header and whose commit advances the store's header and manifest
schemaVersion to 7. No prior contract defines an in-place header/version
advance across profiles or a superset reader; the version-advance mechanism
and reader obligation below are therefore **new choices of this proposal**, not
inherited rules, and are part of what the owner acceptance question decides.

Header and manifest schemaVersion are 7; change and row domains are
`orkworks.hierarchy-storage-change.v7` and `orkworks.hierarchy-storage-row.v7`,
each followed by LF, following the format-6 domain precedent.

The closed registry is the **complete union inventory**: the accepted 35-family
format-6 inventory (the named 33-family mandatory inventory,
`ordinary_child_lineage` as the 34th, `child_preparation_authorizations` as the
35th) **plus** the research profile's 24 metadata families, the shared report
registry (`report_heads`, `report_receipts`) and the research objects table.
Resource charges are rows of the inherited `quota_usage` family already inside
that inventory — the fourteen workspace, nine run and five assignment resource
**names** are values there, not additional registry entries. Every name is
mandatory; a format-7 reader that cannot address any registered family —
including mandatory empty ones — reports unavailable. A reader supporting only
a subset is not a format-7 reader.

Compatibility, stated as this proposal's explicit mechanism:

- Historical rows keep their **own original format's domains, codecs and
  exclusions** forever. Format-3/4/6 bytes are never retokened, re-encoded,
  rewritten, upgraded or re-committed under v7 domains; their proofs validate
  under their original domains exactly as accepted. A v7 manifest lists only
  the rows its own outcome changes, under v7 envelopes.
- Each manifest validates under its own schemaVersion's domain; the header
  sequence joins them. No mixed-codec interpretation, implicit migration,
  downgrade path or copied proof is created. A format-7 store's historical
  manifests may therefore span schemaVersions 3–7; this multi-version
  manifest/reader rule is new here and explicitly replaces the
  one-store-one-version reading for stores that advance by admission.
- A format-7 reader is a **superset reader**: it validates the complete union
  catalog and every historical manifest under that manifest's own original
  domain, so the coexisting profiles' codecs and reader obligations remain
  unchanged for their rows. Readers of formats 3/4/6 reject a format-7 store
  before interpreting rows; missing research registration is never an older
  format's subset.

### Profile registration

The header-adjacent registration carries one row per admitted profile:
`{profile:"retained-file-research-v1", admittedBy:<original outcome C>,
families:<the complete enumerated name list>}`. The committed registration
body is exactly the profile and the complete enumerated family list; the
`admittedBy` CommitRef is an **excluded outer link** — attached after C is
computed, validated separately through the owning manifest, never inside the
body whose digest contributes to C. No registration field contains its own new
C, so no hash fixed point or extra exclusion beyond this named link is needed,
matching the accepted excluded-CommitRef construction. The research profile
registers only through its own original admission/initialization outcome; the
registration row is a real effect, counted in that outcome's manifest and
limits. Profile identity is selected by the accepted admission rules (per
RunKey at fresh root admission, per ChildAssignmentIdentity at the unique
attachment), never by row presence or registration lookup alone.

### Physical name and key mapping

Canonical JSON BLOB keys/values throughout, per family key exactly as the
[enumeration](2026-11-23-research-catalog-closure-design.md#metadata-families-24)
fixes them. Every complete key/value pair obeys the inherited 2 MiB row
ceiling; smaller per-family limits (64 KiB wrapper, 32 KiB input body, 16
references/64 KiB/1 MiB evidence aggregate, and each owning receipt/report
bound) apply independently and deny rather than raise.

| Physical group | Contents | Envelope exclusions |
| --- | --- | --- |
| `research_objects` / canonical ObjectRef | The 8 research object kinds under the accepted shared objects envelope; full retained bodies with exact source digests | Outer lastCommit only |
| Immutable source families | `research_plan_approvals`, `research_reservations`, `research_allocations`, `research_attachments`, `research_control_inputs`, `research_runtime_attachments`, `research_turn_completions`, `research_coordination_receipts`, `research_recovery_decisions`, `research_summaries`, `research_lifecycle_records`, `research_reviews`, `research_clarification_requests`, `research_clarification_answers`, `research_execution_plan_approvals`, `research_preparation_authorizations` | Outer lastCommit, plus each owning codec's named exclusions (e.g. receipt.commit on approval rows) |
| Mutable control families | `research_run_controls`, `research_plan_controls`, `research_task_controls`, `research_launch_controls`, `research_session_states`, `research_collections`, `research_expectations`, `research_lifecycle_state` | Outer lastCommit per the owning codecs; unchanged rows keep original links |
| Shared report registry | `report_heads`, `report_receipts` | Report publication profile's exclusions, unchanged |
| Resource ledgers | The fourteen workspace, nine run and five assignment resource names at their accepted scopes | Ledger envelope per the accepted resource recipe |

The research objects table is research-profile-own physical infrastructure; it
is not a ninth catalog name and never holds another profile's objects. No
research row is created inside another profile's family or vice versa.

### Index and lookup representation

The discovery inventory's both-direction obligation is realized physically as:

- a unique exact-key index per family table over its full canonical key;
- derived exact-key indexes where an owning contract prescribes one (the
  format-6 authorization uniqueness precedent), preventing two P values or
  two owners for one identity;
- lookup maintenance rows only where an owning contract already names them —
  this profile adds no new global dictionary, alias table or hidden index.

Key/digest/body disagreement, duplicate keys, changed same-key bytes for
**immutable** families, or a missing/owning-proof-less index is unavailable,
never overwrite or collision chaining. Mutable control families and report
heads/ledgers replace same-key values only through their owning contract's
captured original-to-final CAS/coalescing rules within one admitted outcome —
the index reflects the replaced value under that same outcome, never a silent
or out-of-band overwrite. **Every index creation and maintenance cost counts**
in the owning outcome's complete-command/store accounting and in this
document's limit table; no index escapes accounting as infrastructure.
Read-side discovery work is bounded by the accepted cumulative discovery
bounds, unchanged.

### Proof and recovery representation

Each original outcome keeps the accepted proof shape under its own format
version: complete before/final-after row commitments, named exclusions,
excluded outer commit links, separate header and new complete-proof controls,
and each changed row's `lastCommit` naming its own C. Additive admission binds
the registration and created rows to its own C; existing rows' original direct
proofs are retained and resolved under their original domains.

Recovery resolves each required retained row, snapshot, receipt and excluded
link to its **own original complete proof** in the same workspace/epoch, per
the accepted recovery boundary: direct proofs only, no recursive replay of all
predecessors, no election by maxima, no reconstruction from current rows, no
repair of a missing ledger, registration or index. The
[crash/race oracles](../../validation/hierarchy-writer-storage-qualification.md#storage-and-crash-evidence-packet)
apply to this physical profile unchanged and remain unwritten obligations,
not evidence.

## Written limit accounting

This table is the audit item 5 obligation "satisfy every smaller row, snapshot,
coupled-job, command, discovery and physical limit", honestly split: written
arithmetic closes the **logical** charges from already accepted codecs; every
physical/native term remains explicitly unresolved.

| Accepted limit | Written status under this profile |
| --- | --- |
| Per-family smaller limits (64 KiB wrapper, 32 KiB input body, evidence 16×64 KiB/1 MiB, per-report/receipt bounds) | Closed by the owning contracts' arithmetic; this profile adds no allowance and changes no value |
| 2 MiB complete row ceiling; 16 KiB #744 claim slot (child profile) | Inherited; research rows never enter claim slots, and maximal bodies deny rather than raise either bound |
| Shared child source pools 8 MiB/run, 64 MiB/workspace; control/report/preparation pools; five per-assignment pools | Placement confirmed unchanged by the accepted accounting section; no new resource name, no sharing discount across profiles |
| 32 MiB run aggregate; 64/128/256 MiB record/payload pools | Bind independently of profile registration; registration rows are real charges against their outcome |
| Complete command: 4,096 changed rows / 16 MiB | Applies to every format-7 outcome including initializer and additive admission; index/registration/proof costs count; no truncation or split |
| Discovery/final validation: 4,096 observations / 16 MiB; 100,000 visited rows; cooperative SQL deadline | Shared bounds apply unchanged across the union catalog; overflow denies whole |
| Native auxiliary 2 GiB / scratch 8 MiB ceilings; pre-operation whole-peak growth (F/P/R/L/S envelope) | **Unresolved — no-go.** The candidate is unavailable at its first gate; both execution batches are consumed; written arithmetic cannot substitute ([evidence](../../validation/pair-native-envelope-input-closure.md#why-this-does-not-close-the-first-gate)) |
| SQLite page/index/journal/WAL/allocator overhead; simultaneous-maximum coexistence | **Unresolved.** No numeric claim, guessed copy count or sample substitutes for the missing envelope; no limit is redefined as logical-only |
| Authority/cutoff, custody, ordinary storage activation, native bridge participation | **Unresolved — no-go.** Each keeps its own accepted boundary and evidence pointer; written acceptance cannot supply them |

No simultaneous maxima, complete byte total, physical peak, eligible adapter or
supported platform is established by the closed rows above. A locally legal
manifest can still exceed a containing bound and must deny.

## What stays gated

Executable command tags/codecs; the execution launch chain and recovery
revisions; component packets under this profile (Gate 2 remains Investigate);
writer/producer coverage; crash/power-loss and per-platform durability
evidence; authority/cutoff and custody qualification; ordinary storage
activation; native bridge participation; profile/adapter eligibility; and all
separately approved implementation plans. The dormant kernel's existing
mechanics remain test-only; the production opener's `unqualified profile` fence
is unchanged by this document.

## Owner choices and bounded next steps

Accept or revise these proposed choices for **written physical-profile
boundary work only**:

1. One hierarchy store per workspace with one header/epoch/manifest sequence;
   additive research admission preserves the actual H, never resets the epoch
   or creates a second null-predecessor outcome.
2. Format 7 as the research profile's store format — fresh-initialized stores
   (null-predecessor outcome) and, as an explicit new mechanism of this
   proposal, existing stores advancing to schemaVersion 7 through the one
   additive-admission outcome — with v7 change/row domains for new manifests,
   the closed union registry (accepted 35-family inventory + research names +
   report registry), per-manifest own-domain validation across schemaVersions
   3–7, a superset format-7 reader obligation, never-retokened format-3/4/6
   bytes, and unchanged older-reader rejection.
3. The physical name/key/index mapping above — canonical BLOB keys, research
   objects table, unique exact-key indexes, derived-key indexes only where
   owning contracts prescribe them, no hidden index, all index/registration
   costs counted.
4. Proof/recovery representation per owning codecs with unchanged crash oracles
   and direct-proof recovery; registration binds to its own original C.
5. The written limit accounting table with every physical/native/authority term
   explicitly unresolved — no numerical default, profile eligibility, probe
   budget, migration or activation is selected by this document.

Acceptance would close only this **written physical-profile boundary**. It
would not qualify a complete command, real producer/cutoff, actual
ordinary/native bridge, legal full-state history, adapter or activation. The
next bounded work after acceptance is the audit's remaining physical
qualification — which requires identified missing bounds within the accepted
gates and its own separately approved plans, not this document.

## Review and verification handoff

One writer: the owning session. One bounded round with two fresh read-only
reviewers: representation correctness (format/registry/coexistence/exclusions
against the accepted formats and this chain) and accounting honesty (every
limit row sourced; every unresolved term explicit; no silent default).
Reviewer cap two; no reviewer edits, fixtures, runtime, native or model probes.
Verify docs metadata/index, build, rendered links, preservation of prior
validation artifacts, append-only audit changes and diff/doc/worktree
close-out. Publication/merge does not accept the five choices above.

## Remaining gates

Physical qualification itself (native envelope, crash evidence, per-platform
durability), authority/cutoff and custody qualification, ordinary storage
activation and native bridge participation, executable command codecs, the
execution launch chain, component packets, writer coverage and separately
approved implementation plans remain open.
#1689/#1089/#610 stay open; #740 stays closed research. Gate 2 remains
**Investigate**.
