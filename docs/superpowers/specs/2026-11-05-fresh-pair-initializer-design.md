---
type: Design Proposal
title: Fresh linked pair initializer and original-attach preparation path
description: Proposed complete fresh-pair initialization for the ordinary schema-2 plus hierarchy format-6 pair and the exact prestate chain, effect union and dispositions of the supported original-attach command; scoped review only.
tags: [orkworks, hierarchy, storage, preparation, initialization, qualification]
status: draft
---

# Fresh linked pair initializer and original-attach preparation path

Tracking: [#1440](https://github.com/Rambolarsen/orkworks/issues/1440),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738). Inspection base:
`51ab79fe82f3f695ac3b97e9f59fcd896dd820e5` (origin/main at session start; the
[readiness checklist](2026-11-05-source-publisher-execution-design.md#finite-implementation-readiness-checklist)
base plus the merged #1435/#1436 records).

## Outcome and scope

The [accepted readiness record](2026-11-05-source-publisher-execution-design.md#finite-implementation-readiness-checklist)
names the next finite contract work: the closed **fresh-pair initializer** and
the **supported original-attach command/preparation path**, with exact inputs,
prestate-producing events, all original-to-final effects and unknown
disposition. This document supplies both. It must not silently reinterpret any
accepted contract; every genuinely new choice is flagged as a proposal for one
scoped owner review.

Deliverables:

1. **Fresh-pair initializer.** One separately closed command that creates the
   linked ordinary schema-2 database plus hierarchy format-6 peer with the
   complete 35-family logical catalog, the accepted derived physical structures
   empty, the complete workspace-scope zero-resource row set, and both store
   headers; with fresh-namespace prestate checks, retry/replay behavior, bounded
   fresh-only recovery disposition and unknown-outcome handling.
2. **Original-attach preparation path.** The ordered chain of earlier required
   events that establish the original-attach command's admitted prestate in a
   fresh pair, each owner-resolved (the accepted five-command tags plus
   explicitly named missing publishers); the attach command's complete
   original-to-final effect union in the fresh-pair direction, including the
   accepted derived physical deltas; and the exact disposition contract
   (commit, retry, lost response, unknown, cancellation) under the accepted
   original-preparation policy.

No SQL/DDL, database, file creation, code, migration, fixture, native/model
probe, live registration or activation is delivered. Existing implementations
stay dormant; the production hierarchy opener remains refused. The ordinary
schema-2 DDL/creation and hierarchy-creation wiring remain separately approved
maintenance items; this document fixes the contract, not the code.

## Accepted basis

Raised earlier and reused without reapproval:

| Accepted contract | Reused substance |
| --- | --- |
| [Joint preparation architecture](2026-11-05-joint-child-preparation-design.md#scoped-architecture-acceptance--2026-11-05) / [ADR 0106](../../adr/0106-joint-child-preparation.md) | Ordinary-worker attached rollback transaction broker; schema-2 lineage member/projection direction and the fresh profile direction; retained linked tombstones; shared-worker availability cost |
| [Authorization representation](2026-11-05-child-preparation-authorization-design.md#scoped-representation-acceptance--2026-11-05) / [ADR 0107](../../adr/0107-original-child-preparation-permit.md#amendment-authorization-evidence-accepted--2026-11-05) | OriginalPreparationIntentV1/AuthorizationV1; one P-keyed authorization row; permit compare-and-consume; format-6 domains and link exclusions; the scoped original-preparation late-save/no-spawn policy |
| [Physical paired membership design](2026-11-05-paired-membership-index-design.md#scoped-physical-design-acceptance--2026-11-05) / [ADR 0108](../../adr/0108-paired-membership-indexes.md) with the [registry exception](2026-11-03-child-consumer-membership-design.md#scoped-derived-index-exception--2026-11-05) | membership_owners/membership_edges (with its mandatory reverse index)/identity_bindings/identity_binding_contributors/artifact_read_metadata; ConsumerLocationV2/ReferenceTargetV2; owning-write maintenance; complete lookup/absence rules |
| [RegistryRoot ordering](2026-11-05-source-publisher-execution-design.md#scoped-registryroot-acceptance-and-readiness--2026-11-05) / [ADR 0109](../../adr/0109-retained-source-registry-ordering.md) | Retained root compare-and-close ordering accepted for further qualification; not a durable membership or commit-cutoff proof |
| [35-family audit](../../validation/child-preparation-catalog-reconciliation.md) | Complete prestate/effect obligation map for original attach; "no insertion" rows; separate header/new-proof controls; required absence/uniqueness proof |
| [Five-command effect map](2026-10-31-child-review-source-preparation-design.md#durable-command-effect-map--2026-11-03) | propose/approve/reserve/allocate (intent plus outcome)/attach recipes; common read/commit/retry contract; frozen assignmentTaskVersion |
| [Root preparation effects](2026-10-28-root-preparation-effects-design.md#exact-command-effects) | initialize-store/root-draft-prepare/root-draft-cancel/subject-preparation recipes; namespace reservation first; both summary branches; the nine-row workspace packet |
| [Aggregate budget acceptance](2026-10-29-assignment-aggregate-budgets-design.md#owner-acceptance-and-component-handoff--2026-10-30) and the [fresh-attachment closure](2026-10-31-child-review-source-preparation-design.md#fresh-attachment-and-preparation-closure) | Shared ledger keys/meanings and owner scopes; "the ten corresponding run/workspace keys must already exist"; missing historical peer rows are unavailable, never a fresh-zero opportunity |
| [Control catalog and snapshots](2026-11-02-child-launch-control-design.md#closed-storage-catalog-and-finite-accounting) | Nine control rows; first-parent-attach runVersion 1→2 in the new current control; snapshot capture semantics |
| [Ordinary SQL store](2026-11-01-sqlite-session-storage-design.md#storage-ownership-and-schema) | Ordinary header/epoch/history-generation shape; the dormantly implemented schema-1 catalog; schema-2 as the accepted direction with its separate maintenance approval |
| [Debit representation](2026-10-31-artifact-read-debit-proof-design.md#one-retained-row-and-exact-six-row-effect) | evaluation-artifact-read-bytes owner scopes and the prepared-ledgers requirement |

## Decision boundary

- The initializer is a **storage/maintenance command**, not one of the five
  source commands and not an evaluated-use permit member: it creates no source,
  runtime, grant, capability or producer eligibility. Its only "event" is the
  fresh pair's own header/catalog/zero-row creation.
- No missing historical row, peer or ledger is initialized on demand by any
  other command; only the initializer prepares fresh workspace scopes. The
  older three-resource root packet "cannot be silently treated as all five" —
  the fresh pair's complete preparation is settled here explicitly.
- The accepted broker's no-ATTACH-as-side-effect rule stays: the initializer
  creates both files explicitly under its own typed command; command
  attachment never creates a missing peer.
- Every prestate event for attach must have a durable, owner-resolved original
  proof; a row whose creating event has no closed publisher is a **named gap**
  in this document, never an assumed empty state, synthesized proof or skipped
  chain element.
- Actual runtime producer/ordinary activation qualification
  ([#1242](https://github.com/Rambolarsen/orkworks/issues/1242) and the
  ordinary cutover chain), source/revocation native ordering, executable plans,
  fixtures, content/recovery maintenance and RegistryRoot's implementation all
  remain separate gates. #740 stays closed research.

Rejected alternatives: extending an existing workspace store in place
(adoption is separately gated); creating files as a side effect of any other
command; fabricating "assumed fresh" prestate proofs; arriving at eligible
attach with missing run-scope rows zero-initialized on demand; and treating
the ordinary schema-2 DDL as authorized by this document.

## Part 1 — the fresh-pair initializer

### Command identity and inputs

`commandKind` stays `initialize-store` in the format-6 manifest union. The
command is executed once per fresh workspace by the qualified linked broker
owner; its inputs are exactly:

- `workspaceId` — the retained workspace's exact canonical identity, resolvable
  only through an actually owned workspace lease; not caller-supplied text;
- both owner-resolved fixed paths — the ordinary
  `<workspace-metadata>/session-store/v1.sqlite` and the hierarchy peer per the
  accepted storage naming — owner constants, never caller paths, URIs or
  config values;
- ordinary schema-2 selection (the accepted schema-2 direction with its
  separate maintenance approval) and hierarchy format 6;
- one fresh ordinary `databaseEpoch` — a fresh lowercase UUID, never reused
  across attempts, generations or sidecar restarts;
- one fresh hierarchy `storageEpoch` — a fresh random 32-byte Digest; and
- a fresh sidecar `PreparationPermitIdentityV1`/`DebitIdentityV1` sequence
  scope (runtime state; a genuinely new sidecar generation establishes a new
  scope; no old permit/credit is restored).

No existing pair, store, JSON or other file is adopted, imported, repaired or
reset. Activation/UI ownership of "create linked pair" remains an
implementation gate; this document defers any such decision and seeks scoped
representation approval only. For a fresh non-import creation the ordinary
header's import manifest digest is null — a non-import use of that accepted
field, part of handoff item 1's scoped decision rather than an assumed default.

### Prestate checks and producing events

Before any effect, the qualified owner checks all of:

| Prestate | Requirement and provenance |
| --- | --- |
| Pair absence | Both fixed paths (and any ordinary activation marker) are absent, or both belong to this exact attempt's pending state under [recovery](#fresh-only-recovery-and-unknown-disposition); any other combination — including a published activation marker over anything incomplete — is unavailable |
| Linked eligibility | The accepted implementation readiness for the ordinary SQL cutover and the new hierarchy profile is actually enabled at this owner per the [activation gates](2026-11-01-sqlite-session-storage-design.md#migration-and-single-authority-activation); never inferred from the absence of an error |
| Workspace lease/identity | The actual retained lease/identity for `workspaceId` is current at the exact command boundary; sidecar generation, fresh sequences and both candidate epochs are already the attempt's prestate (fresh store, no run/subject) |
| Separate new-store authorization | The explicit [new-store initialization authorization](2026-10-24-hierarchy-storage-contract-design.md#canonical-byte-and-transition-fixtures) under the current Electron-main-authorized workspace flow |
| Bounded joint job | The complete scheme fits the ordinary 1 MiB serialized-intent bound and every accepted queue/command limit; excess denies rather than splitting transactions |

### Effects (both stores, one joint commit)

The initializer's hierarchy-side joint manifest contains only hierarchy-side
logical changes (the thirteen insertions plus the two controls); the ordinary
side carries no ChangeManifest — it is witnessed by its own committed header
and created schema-2 catalog inside the same engine transaction. The ordinary
`format.json` activation record is published after that commit as the accepted
activation point ([accepted activation contract](2026-11-01-sqlite-session-storage-design.md#migration-and-single-authority-activation));
pre-activation the pair is not openable, and a published marker with anything
incomplete is a fenced state, never a silently completed one.

| Store | Effects |
| --- | --- |
| Ordinary | Create the owned schema-2 file and catalog: the accepted schema-1 tables/indexes (sessions/history columns/organization/FTS5/deletion jobs per the accepted ordinary contract) plus the reserved `orkworksHierarchyPreparation` lineage member inside the canonical metadata and its exactly-pinned UNIQUE full-assignment-key projection/index derived transactionally from the member ([accepted direction](2026-11-05-joint-child-preparation-design.md#original-ordinary-lineage-and-versioned-commitments)). Header: schemaVersion 2, `workspaceId`, fresh `databaseEpoch`, history generation 0, import manifest digest null; no session rows; no FTS content |
| Hierarchy peer | Create the hierarchy file with the complete 35-family logical catalog and the accepted [derived physical catalog](2026-11-05-paired-membership-index-design.md#scoped-physical-design-acceptance--2026-11-05): all four derived structures (membership_owners, membership_edges with its reverse index, identity_bindings/contributors, artifact_read_metadata) present and empty, declared indexes exactly as accepted. Header singleton: schemaVersion 6, `workspaceId`, `storageEpoch`, `latest` CommitRef; `commit_proof` sequence 1 with null predecessor, explicit new-store initialization |
| Workspace zero-resource rows | Exactly thirteen `quota_usage` rows at `used="0"`/`reserved="0"`, workspace owner scope, each `lastCommit` equal to the initializer's own C: the accepted nine (reporting-receipt-bytes, assignment-record-bytes, result-payload-bytes, summary-row-bytes, summary-row-count, summary-lineage-bytes, summary-pending-count, preparation-retained-bytes, preparation-run-count) plus child-source-retained-bytes, child-control-retained-bytes, evaluation-artifact-bytes, evaluation-artifact-read-bytes |
| Per-run rows | None. No run is admitted by the initializer; run-scope rows are prepared by admitted run preparation events ([the chain](#the-original-attach-preparation-chain)) |
| Other rows | No source, subject, attachment, plan/task/launch/run/session control, receipt, claim, lookup, candidate, evaluation, artifact, lineage or artifact-read row; no fabricated owner/edge/binding rows; nothing extra logical or physical |

The manifest changes list the thirteen quota-row insertions; header and initial
proof remain the two separately validated controls. Ordinary bytes count under
ordinary physical limits; hierarchy bytes under the hierarchy's existing store
and command limits; no fresh allowance exists.

### Exact retry and replay

- The initializer is not idempotent against existing paths: any attempt that
  sees an openable pair, a non-pending file at either fixed path, or a
  previously completed pair is denied — it never upgrades, re-imports or
  re-initializes.
- A re-executed initializer command attempt with both candidate epochs already
  consumed by another attempt denies on epoch/namespace freshness; no second
  sequence scope mints itself.
- Exact authorized re-entry after a proven complete commit resolves through
  the supported open/recovery path (return the pair identity; no new rows,
  proofs, headers or files).

### Fresh-only recovery and unknown disposition

Pending-state classes for the fresh pair (none of which is user-visible as an
eligible store): (a) uncommitted (neither header committed), (b) committed but
pre-activation (both headers committed, `format.json` activation record not yet
published), (c) incomplete after commit (missing rows or one-sided catalog), and
(d) activated-but-incomplete (marker published; fenced, never self-settled).

| Outcome | Disposition |
| --- | --- |
| Interruption before either header commits | Both files exist as uncommitted fresh state; they are fenced (unopenable) everywhere. The initializer's one bounded fresh-only recovery disposition — the **single authorized destruction class** (new scoped choice) — may settle it: destroy exactly both fresh files (plus any staging marker) and release the attempt's sequence scope, with the "neither header committed" proof coming from bounded private recovery, never from counting or digests alone |
| Interruption mid-joint-commit | Engine transaction atomicity is a candidate mechanism per the accepted broker; qualification remains gated. Recovery establishes complete committed old or complete committed new state at both stores through header/proof/catalog verification; no two-sided reconstruction |
| Committed but pre-activation pair (marker not published) | The extended fresh-only recovery disposition (new scoped choice) applies instead of fencing: after proving the pair complete (both headers, full catalog, thirteen zero rows, empty derived physical structures), recovery completes the accepted activation step by publishing `format.json`; if the pair proves incomplete, this class instead destroys both files under the same bounded destruction proof. In either branch no source, attach or run event was ever admitted, so the destroyed bytes are the initializer's own |
| One header committed, the other not | Unavailable (class c); the incomplete pair stays a fenced documented state, never auto-paired, auto-completed, silently deleted or presented as eligible; a supported rescue needs its own concrete proposal |
| `format.json` published but the pair proves incomplete | Fenced (class d); never self-settled; explicit supported rescue only |
| Complete new pair, response lost | The pair is re-established via open/recovery at its original identity (same epochs, same headers); report as initialized; no second pair, headers or sequence scope |
| Runtime crash after commit and activation, before reporting | Activation already gates normal open/recovery; ordinary accepted open/recovery rules apply; nothing reinitialized |

## Part 2 — the supported original-attach preparation chain

### The original-attach preparation chain

The ordered chain of earlier required events, each supplying part of the
original attach command's admitted prestate in a fresh pair:

| # | Event | Owning command/producer | Prestate facts it establishes for attach |
| --- | --- | --- | --- |
| 1 | Fresh-pair initializer | `initialize-store` (Part 1) | Both headers (hierarchy H/epoch; ordinary schema-2/epoch and G=0 baseline), the complete thirteen-row workspace scope (all zero), the empty catalogs and empty derived physical structures |
| 2 | Durable namespace reservation proof | the accepted durable admission fence protocol (the #742 sidecar's fresh-[namespace admission](2026-10-04-orchestrator-preparation-design.md#exact-wire-counters-and-generation-identities)), recurring before root-draft prepare | Provably fresh, non-adoptable namespace/attempt identity; a burned sequence never revives |
| 3 | root-draft-prepare | root-draft-prepare recipe (accepted, conditional) | The pending `root_drafts` row and the selected artifact occurrences; per the accepted recipe the run's `preparation-retained-bytes` run row may initialize with its exact initial charge (the accepted symbolic count includes it), and both workspace preparation-ledger rows update if changed. Chain 4 re-resolves the actual state rather than assuming this row exists |
| 4 | root subject-preparation | subject-preparation recipe (accepted, **extended** per the [run-scope proposal](#run-scope-preparation-dependency-proposal)) | `root_preparations` original receipt/source row, immutable `run_states`, reserved `parent_attachments`, the root `summary_state` branch (initialized row/checkpoint or the never-initialized marker), the root assignment's three reporting rows plus the run's corresponding rows at zero, and — in the fresh-pair direction — the four extended run-scope zero rows |
| 5 | child-source-propose | child-source-propose recipe (accepted) | Plan/configuration/non-null evidence/(optional review-input) objects with exact provenance edges; the exact prepared-input row; new plan/task controls at version 1; ordered planReferences/pendingPlan; currentInput where actually changed |
| 6 | child-source-plan-approve | child-source-plan-approve recipe (accepted) | The immutable approval source/digest/receipt and `child_plan_approvals` row; plan-control `latestApproval`/status transition (this event's approval is the one whose ApprovalRef attach cites, not any later latestApproval) |
| 7 | First parent attach cluster | **Named gap:** the parent-attach/run-control events have no complete durable publisher in the [control-event inventory](2026-10-31-child-review-source-preparation-design.md#control-events-not-given-a-publisher-by-these-five-tags) | The durable runVersion-1 `child_run_controls` row (its own creation event being unnamed in accepted contracts), then the first parent attach performing the accepted runVersion 1→2 transition in the new current control ([accepted control rule](2026-11-02-child-launch-control-design.md#transition-and-recovery-rules)); `parent_attachments` attached variant with immutable runtime history and current run pointer/version; the parent-side native attach event that publishes the actual `RuntimeRef` history whose digest reserve freezes |
| 8 | child-source-reserve | child-source-reserve recipe (accepted) | The original reservation with frozen `assignmentTaskVersion`; `child_launch_controls` at phase reserved (admission approval + the actual current parent runtime, null allocation/attachment/spawnRuntime); task control launching with the single once-advanced live taskVersion |
| 9 | child-source-allocate (intent then outcome) | allocate producer with distinct intent/outcome phases | The durable allocatory intent precedes any native mutation; on verified outcome the original `child_allocations` row, launch phase allocated, exact observed directory objects; on failure/timeout the retained interrupted/uncertain disposition (neither an allocation nor a retry permission) |
| 10 | child-source-attach | the original attach command under the accepted original-preparation permit and broker ([Part 2 disposition](#dispositions-retry-lost-response-unknown-cancellation)) | The complete original preparation union per the [audit](../../validation/child-preparation-catalog-reconciliation.md#complete-paired-effect-obligations) plus the accepted derived physical deltas; the paired ordinary commit; the one new `child_preparation_authorizations` row |

Every chain step's producer participation remains independently gated; chain 7
plus the other list-clean gaps below are just the enumerated "no assumed empty
state" set. A row whose creating event has no closed publisher still blocks
attach — never silently.

Named remaining gaps the chain exposes (explicitly incomplete, not proposals
here): the first parent attach cluster (parent-attach/run-pointer/run-control
initialization command, including the parent-side native runtime publish event);
the complete allocate intent/outcome command representation (its source-owned
native disposition); and any other covered-control event from the inventory
that the chain requires but has no closed codec. Each becomes its own scoped
deliverable before a complete transaction fixture.

### Attach's admission read set, resolved against the chain

Attach resolves, at one captured pair (H, G), through the accepted
[authorization read set](2026-11-05-child-preparation-authorization-design.md#required-joins-and-actual-authority)
with one fresh-pair elaboration for the immutable originals: plan, input,
approval/reservation/allocation/attachment source and object rows resolve to
their chain event's original manifest commit, never "the latest writer".
Mutable control rows keep the accepted read-set semantics instead: they are
captured at H with their accepted snapshot semantics and revalidated at the
transaction boundary; a legitimate intervening semantic advance denies via
the stale-H rule rather than pinning back to a chain commit. In addition to
the accepted set:

| Additional fresh-pair requirement | Enumerated prestate |
| --- | --- |
| Complete workspace scope reconciliation | the thirteen initializer rows reconcile against actual committed deltas; a missing row is not a fresh-zero opportunity ([accepted rule](2026-10-31-child-review-source-preparation-design.md#fresh-attachment-and-preparation-closure)) |
| The complete run-scope row set | all fresh run-scope rows exist because the run was admitted through events 3–4; attach does not create any of them |
| Complete bounded absence | the accepted [absence recipe](2026-11-05-paired-membership-index-design.md#qualification-sparse-recovery-and-corruption) at the fresh-pair state; including tombstones and retained history |
| Paired identity checks | the schema-2 lineage projection and the pairing session/attachment bindings including tombstones, with the paired-capture join governed by the accepted broker |
| Four control snapshots | the run/plan/task/launch captures resolve through events 5–9 to actual original rows with the accepted [snapshot semantics](2026-11-02-child-launch-control-design.md#closed-storage-catalog-and-finite-accounting); a snapshot about a still-gap-publisher row stays unavailable, not an assumed-empty state |
| Permit/sequence candidate | the candidate permit lives in the fresh sidecar generation with its distinct checked sequence under the accepted candidate compare-and-consume rule; no reused sequence |

### Complete original-to-final attach union

Reusing the accepted audit's paired-effect obligations with the concrete fresh-
pair deltas:

| Effect row | Concrete fresh-pair delta |
| --- | --- |
| Admission/read set | per [the read set table](#attachs-admission-read-set-resolved-against-the-chain); every required row resolves to its chain-event commit |
| Original permit | the accepted compare-and-consume permit; no authority amendment elsewhere |
| Ordinary member | insert the complete normalized creating metadata at its creation identity/revision, initial history/search projections, and the schema-2 lineage member/UNIQUE full-P projection; advance G once |
| Hierarchy source | insert exact attachment and child-assignment-source plus every prescribed provenance/object edge; retain original immutable source history; unique reserved child/attachment across runs and tombstones |
| Controls | prepared launch control (original attachment, spawnRuntime null) and prepared session state (producerRevision "1", runtime null); only prescribed semantic control transitions actually change their row, each version advanced once; never rewrite immutable frozen assignmentTaskVersion |
| Fresh subject | insert exactly the assignment-owned five zero rows (reporting-receipt-bytes, assignment-record-bytes, result-payload-bytes, evaluation-artifact-bytes, evaluation-artifact-read-bytes) at used="0"/reserved="0"; the ten run/workspace peers already exist and reconcile, and a zero new usage neither initializes nor stamps them |
| Summary | the exact child's authoritative criteria/rubric branch: subjectRevision "1" with the initialized Unassessed row/checkpoint from this child's own bytes, or the accepted never-initialized marker under capacity denial; charge the actual summary bytes/counts/lineage branch |
| Additional format6 evidence | both the `child_preparation_authorizations` and `ordinary_child_lineage` rows commit in the same joint outcome under their accepted representation and link exclusions; the full identity bindings/contributors (session-owner, child-attachment, preparation-permit, ordinary-lineage) derive from these actual originals, sourced at chain commits |
| Charges | coalesce actual source/control/summary owner deltas; source copies include the full authorization snapshots/origin links and the lineage projection under the 8 MiB run / 64 MiB workspace source pools; controls charge their own pool; the accepted 32 MiB run metadata, 16 MiB complete-command and all lower row/discovery/ordinary limits apply unchanged; no sharing discount, ledger borrowing, new allowance or cancellation refund |
| Commit/publication | include every real changed logical key once in sorted before/final-after form; header/latest and new commit_proof obey their separate equations; all derived physical deltas (owner rows, membership edges with reverse-index entries, binding/contributor rows, artifact metadata) are applied atomically inside the owning transaction; the joint paired commit publishes ordered references/invalidations before releasing the broker position; no SessionHandle/startup future/grant is published by preparation |

The exact changed set is the union of actually changed rows only; a fixed
row-count template is not a manifest witness. New zero rows are real changes;
unchanged peers keep their original links. The audit's "no insertion" rows
still resolve through their owning recipes and never implicitly skip passive
source/reader obligations.

### Run-scope preparation dependency proposal

For a provably new run, the fresh pair must prepare run-scope zero rows before
any child command. The accepted root packet already establishes the run's three
reporting rows at subject-preparation and the run's `preparation-retained-bytes`
row at root-draft prepare. **New scoped choice**: in the fresh-pair direction
only, extend the root subject-preparation prepared row set to additionally
create four run-scope zero rows — `child-source-retained-bytes`,
`child-control-retained-bytes`, `evaluation-artifact-bytes` and
`evaluation-artifact-read-bytes` — each with the run's new owning link (the
same original commit C). This is a real zero-row change counted in the
command/manifest/complete-command accounting, per the accepted run/workspace
scope-owner rules. It does not extend or retroactively change the historical
format-4 root-packet profile and gets its own scoped owner decision before any
dependent fixture.

### Dispositions: retry, lost response, unknown, cancellation

The accepted attach-recovery rules
([fresh attachment closure](2026-10-31-child-review-source-preparation-design.md#fresh-attachment-and-preparation-closure))
govern without change, as do the authorization oracles
([authorization design](2026-11-05-child-preparation-authorization-design.md#denialrecovery-oracles-and-handoff)).
For the chained original-attach command specifically:

1. Prestate capture: proven chain commits plus the accepted candidate
   prebuild rule; a stale-by-H/G or source-drift denial before any effect.
2. The accepted narrow original-preparation permit cutoff and late-save
   policy apply per [ADR 0106](../../adr/0106-joint-child-preparation.md)/[ADR 0107](../../adr/0107-original-child-preparation-permit.md);
   no other source command shares those.
3. After a proven complete joint commit, exact authorized retry resolves the
   original attachment/source/subject/ledger/proof bytes and returns the
   original outcome identity — no second row, zero replay, no revision
   advance, no slot allocation, no capability restoration, and never a claimed
   second child attempt. Proven rollback leaves complete old paired state,
   with the consumed volatile permit sequence not reused. Unknown outcomes
   retain the actual private plan/worker/position until the owner proves a
   complete old/new state; neither task Drop nor ordinary end proves rollback.
4. Cancellation before the permit results in no attachment/hierarchy effects
   and a normal prestate; cancellation after the accepted permit completes
   only the exact paired save (history/charges retained, no spawn). A changed
   H, G, read set or lease loss denies/reverts exactly as the accepted
   broker's transaction primitive governs.

## Verification oracles

| Case | Disposition |
| --- | --- |
| Initializer attempt with missing/foreign workspace lease identity | deny before any file creation; nothing partial remains |
| Initializer attempt colliding with a leftover fenced pending pair | denied until the bounded fresh-only recovery disposition (or a separately approved owner rescue) settles it |
| A second fresh-pair initializer attempt while pending state remains | deny on existing paths; no second sequence scope |
| One-sided commit across the pair under an unqualified engine | availability/qualification stays gated; the written disposition covers expressed recovery only, not a bypass |
| Restore/re-run of the initializer on an exported/backup copy of an existing pair | deny; the fresh-only initializer cannot re-initialize a restored or partial pair |
| Attach before the chain-9 (first parent attach) snapshot leg resolves | that leg is unavailable; not an assumed-empty state; no fabricated status byte |
| Attach with any chain event still pending/interrupted (allocation uncertain, draft unplanned, etc.) | affected prestate unavailable; attach rejects; the accepted attempt-prebuild rules remain unchanged |
| Attach with missing/foreign reservation, wrong digest binding, stale H/G or forged lineage | deny all prestate facts at their original cutoffs; no second save; no patched intent |
| Attach commit with lost response / unknown | per the accepted dispositions: original C/lineage/authorization resolution; no rebuild; no second publication |
| Two concurrent attach attempts racing the same candidate PreparationPermitIdentityV1 sequence | the compare-and-consume yields exactly one winner; the loser denies at its cutoff with no second save, no patched prebuilt candidate grant, and no reuse of the consumed sequence |
| Attach colliding with a retained/tombstoned prior child or attachment identity | attached unavailable; tombstones participate in the uniqueness/absence checks and are never silently accepted or de-tombstoned |
| Fresh-only recovery invoked against a one-header-committed, partially committed, or already activated pair | the bounded destruction proof fails; recovery refuses to delete and leaves the pair fenced; the deletion class never runs against any state but the exact never-committed (or pre-activation incomplete) fresh pair |
| Post-recovery re-initialization after a proven fresh-only destruction | the next attempt proceeds with genuinely fresh epochs/sequence scope (the released scope is never reused); no aliasing between the destroyed attempt and the new one |
| Unrelated concurrent ordinary/hierarchy mutation outside the accepted broker | attach prestate denies via changed H/G and membership; not silently merged into the accepted broker |

## Handoff and scoped decision

This document requests one scoped decision accepting, for further
qualification, the new pieces proposed here:

1. the exact fresh-pair `initialize-store` command recipe with the
   thirteen-row workspace zero-resource set, the empty derived physical
   catalog, the non-import null import-manifest-digest use, and the
   fresh-namespace/authorization rules;
2. the fresh-only recovery disposition (the bounded destruction class for
   interrupted never-committed states, plus the bounded activation-completion
   branch for a committed-but-pre-activation pair) and the incomplete-pair
   availability dispositions;
3. the extended root-subject-preparation run-scope row set (four additional
   child-resource run rows) as the named run-scope preparation dependency;
4. the chain gap acknowledgements (chain 9 and its control-snapshot leg, plus
   the allocate intent/outcome command representation) as the explicit
   do-not-assume-empty set from
   [the readiness checklist](2026-11-05-source-publisher-execution-design.md#finite-implementation-readiness-checklist); and
5. the fresh-pair-direction reuse of `initialize-store` under format 6.

It approves no DDL, no code, no activation, no producer eligibility, no
native/resource result and no fixture. Once these scoped choices land, the next
work per that checklist is to close the named chain gaps as their own scoped
deliverables, then the complete coupled-publisher executable/recovery/producer
qualification plan behind the actual-owner gates. #1176 stays open; #740 stays
closed research.


## Bounded review disposition

One bounded read-only lower-cost round used two independent reviewers — a
correctness reviewer against every cited accepted contract, and a
builder/coverage reviewer against tracking issue #1440's acceptance criteria,
the readiness checklist and the control-event inventory. Neither edited files
or ran builds; they independently verified the thirteen-row workspace
enumeration, the four-row run-scope extension arithmetic (eight pre-attach run
rows total), and the chain-event mapping against the accepted recipes.

Applied dispositions (author corrections from one round; no second post-edit
review is claimed):

- **Material:** the ordinary `format.json` activation record was missing from
  the initializer's contract. **Applied:** the activation record's publish
  ordering after the joint commit, the pending-state classes
  (uncommitted/committed-but-pre-activation/incomplete/activated-but-incomplete),
  and the extended fresh-only recovery with both bounded branches
  (activation-completion for a proven complete pre-activation pair;
  bounded destruction otherwise).
- **Material:** the chain ordered reserve (which freezes the parent runtime)
  before the first parent attach that publishes that runtime history.
  **Applied:** chain reordered — the first parent attach cluster (7) now
  precedes reserve (8) and allocate (9), with the runVersion-1 creation event
  and the parent-side native runtime publish event named inside the same
  cluster gap.
- The chain-commit elaboration was scoped to immutable originals (mutable
  controls keep the accepted captured-at-H semantics); chain 3's "may
  initialize" retained the accepted conditional wording; the null
  import-manifest digest use was moved into handoff item 1's scoped decision;
  the "defers any such decision" wording replaces the ambiguous request
  phrasing; the namespace-reservation owner is now named by its accepted
  protocol pointer; both broken anchors fixed; three new oracles added
  (permit race, tombstone collision, destruction-class refusal) plus the
  post-recovery re-initialization oracle; the gates inventory mirrors the
  checklist's "actual source/ordinary admission" row.

Remaining reviewer notes accepted as recorded: the ordinary-side manifest
witness statement (now explicit in the effects paragraph) and the
activation-ownership gate (deferred, not requested). The corrections are
clarifications of proposed content; no acceptance, qualification or owner
decision is claimed by them, and all proposed choices in
[the handoff](#handoff-and-scoped-decision) remain owner-gated.
