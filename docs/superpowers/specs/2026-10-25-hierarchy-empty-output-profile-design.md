---
type: Design Proposal
title: Hierarchy empty-output logical source-profile witness
description: Exact synthetic format-3 preparation, admission and acceptance bytes with summary checkpoints, shared ledger rows and bounded owner-cap witnesses.
tags: [orkworks, hierarchy, storage, fixtures, accounting]
status: draft
---

# Hierarchy empty-output logical source-profile witness

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `a10f11f1`.
**Two scoped representation/preparation choices accepted for further design/qualification;
no executable or runtime-eligible profile is established.** See the
[owner acceptance](#owner-acceptance-and-handoff--2026-10-25). This continues the
[accepted three command choices](2026-10-25-hierarchy-candidate-command-manifests-design.md#owner-acceptance-and-handoff--2026-10-25),
[format-3 codecs](2026-10-25-hierarchy-compact-bindings-design.md#owner-acceptance-and-handoff--2026-10-25)
and [slot metadata](2026-10-25-hierarchy-slot-finalization-design.md#owner-acceptance-and-handoff--2026-10-25).
Source/request/wire objects, accepted quotas and claim/lease policy are unchanged.
#740 remains deferred/no-go until November.

## Outcome and preparation plan

The preceding seven cases proved symbolic effect-set accounting. This deliverable
pins one concrete sequence's complete logical row snapshots, exact canonical
key/value projections, manifests, commitments, command-byte charges and owner
ledger/cap comparisons. Specify a deliberately small witness, generate and
independently review its joins/accounting, then review any new representation or
preparation choices before dependent implementation. It starts no database,
worker, coding-tool session, model or filesystem content pass.

The [catalog](../../validation/hierarchy-empty-output-profile.json) and
[generator/checker](../../validation/check-hierarchy-empty-output-profile.mjs)
are reproducible synthetic evidence. They reuse the exact root-candidate-initial
source, request, receipt, ClaimProofV2 and retry key from both pinned existing
catalogs, whose SHA-256 values are retained. Neither seed supplies a verified
bootstrap, permission profile, native authority history or deployed store.

## Profile and external assumptions

Choose the existing root assignment, one candidate with no accepted predecessor,
no output declarations, and one accepted result revision. There are no path,
artifact, evidence, evaluation, incoming dependency or pending-material rows.
There is no elapsed/cost source; timeCostEvidence is exactly empty. Accepted
snapshot/result bytes do not establish task success: the required criterion,
one rubric dimension and rating all remain Unassessed.

The catalog supplies one required criterion and one versioned review-role rubric
dimension, their exact approved-snapshot-shaped bytes and recomputed #744 snapshot
digests. These are synthetic context witnesses. **Their binding to the seed's
bootstrapConfigurationDigest is UNVERIFIED**: no complete #741 bootstrap is
fabricated or asserted approved. No real run/source membership, user approval,
claim ordering, lease, permission enforcement or parent liveness is established.
The store snapshots are complete for the listed logical slice, not complete
producer/bootstrap/allocation state or proof of an admitted launch history.

The witness has no present outputs and therefore no payload rows, content reads,
rehash passes or candidate-work reservation commands. Its candidate work counters
stay zero. This avoids importing missing artifact/custody codecs or native path
exclusion into byte evidence. It qualifies neither those source combinations nor
their maximum sizes. Existing positive-pass accounting remains covered by the
separate symbolic command and source contracts.

## Proposed logical row byte projection

All keys and values in the catalog retain exact compact canonical UTF-8 JSON
strings. Integer source tokens stay integers; Counter columns stay canonical
u64 decimal strings. The catalog's pretty JSON envelope is evidence packaging,
not a new store table or a raw request log in production metadata.

For the objects rows used here, propose the same decoded-body rule already fixed
for claim_proofs: the row commitment's body.body is the complete strict decoded
canonical source object, not a Base64 or JSON-quoted BLOB. Physical objects.body
bytes must equal C(that object), verified against its unchanged source-domain
digest and exact ObjectKey. This closes that row-projection choice for these two
source variants only; unknown source/table variants remain unsupported.

Every stored row value in this fixture is its logical column projection, including
outer links. ClaimProofV2 stays the accepted decoded proof object. Responses stay
original canonical receipt bytes encoded as standard padded Base64. The accepted
exclusions remove only lastCommit, claim_proofs.commit, receipt.commit and
summary_state.checkpoint.applied at their named paths. All other fields and full
keys remain in the v3 row envelope. mutation_lookup uses its accepted raw 64-byte
ASCII physical key for charges and its JSON digest string in the manifest.

For changed logical bytes, count the complete canonical key/value projection
once per changed row, using the special 64-byte lookup key rule. Nested source
JSON contributes its canonical bytes once. Every changed source/receipt/ledger/
summary/link/header/new-proof row counts. The manifest entries are already bytes
inside the new commit_proof value and are not added a second time. This is exact
for the **declared logical projection**, not a proposed SQLite record encoding
or measurement of physical BLOB, page, index, journal, memory or I/O costs.
Any additional required logical metadata needs a reviewed closed representation
and remeasurement; physical overhead remains a separate qualification gate.

## Explicit preparation rather than invented history

The fixture sequence has four complete commands:

| Sequence | Command | Complete state effect |
| --- | --- | --- |
| 1 | initialize-store | Header and its v3 initialization proof only, null predecessor |
| 2 | subject-preparation (new proposed kind) | Initialize this one synthetic subject's current no-result summary/checkpoint and all thirteen required quota rows, including zeros; header/new proof |
| 3 | candidate-admission | Ten slots, candidate, work row, claim, lookup, six quota replacements, subjectRevision update; header/new proof |
| 4 | result-finalization accepted | Result/head/work, nine slot replacements, six source/receipt quota replacements, two summary-resource replacements, summary/checkpoint; header/new proof |

subject-preparation is a **new proposed closed manifest kind**, not an existing
runtime action, route, allocation grant or sidecar API. For this witness only,
it requires the initialized store and absent subject/ledger keys, no retained
subjects/consumers, and explicit synthetic source context. It creates exactly one
summary_state and thirteen quota rows. It cannot reset an existing subject,
initialize on read, repair a lost store or infer approved producer context.
A real preparation producer and its approval/capacity/ownership atomicity remain
undefined/ineligible until their source-owned contract is reviewed.

The prepared subjectRevision is "1" and summaryRevision is "1". Admission moves
subjectRevision to "2" without changing material/checkpoint/summaryRevision.
Acceptance moves subjectRevision to "3", installs the exact new accepted ResultRef
and advances summaryRevision to "2" with its new checkpoint. No pending row is
needed because the witness's summary capacity fits. Both old material/row bytes
are retained in prior snapshots as fixture evidence, not as uncharged live rows.

The authoritative manifest list is precisely the difference between adjacent
logical row maps, excluding header/new-proof controls. Every changed row's outer
link joins its manifest. Admission leaves checkpoint.applied on sequence 2;
acceptance binds the new checkpoint to sequence 4. No source bytes, current head,
quota row, receipt or material transition is reconstructed from a maximum counter.
No proof is deleted by these commands. A separate check permits pruning the now
unreferenced initialization proof, while deleting a proof still needed by retained
rows fails. Retaining a predecessor reference alone does not require an unbounded
chain of old proofs, consistent with the existing retention rule.

## Exact ledger and owner-cap witness rows

All thirteen quota_usage rows use the accepted shared owner/resource keys.
Assignment/run/workspace receipt counters each move 0/0 → 16,384/147,456 →
32,768/131,072 (used/reserved bytes). The pool total stays **163,840 bytes**
after admission. Record counters at each scope move 0 → 618 → 1,362 bytes.
Payload counters stay zero; summary count stays one; pending count stays zero.
Summary row and checkpoint-material counters are recomputed from their complete
canonical bodies. They are not inferred from receipt reservation or source size.

The catalog also contains thirteen ledgerCapWitness entries keyed by the exact
owner/resource. These are **fixture-only comparison ceilings**, not quota_usage
columns, a new durable cap table or proposed global production defaults:

- Receipt ceilings remain accepted 17/64/256 MiB by assignment/run/workspace.
- Record ceiling is 1 MiB at each scope for this isolated one-candidate/one-result
  witness: two possible 512 KiB source records. This does not authorize another
  record/assignment or settle finite aggregate production source quotas.
- Payload ceiling is zero because this witness forbids present outputs.
- Summary ceilings reuse the existing proposed 256 MiB row bytes, 4,096 rows,
  64 MiB lineage bytes and 4,096 pending entries. Source limits stay independent.

These ceilings permit reproducible checked comparisons for the concrete state.
They confer no allocation authority or claim that the main file can fit them.
Production source/payload caps remain unresolved: do not load these witnesses
as runtime configuration or treat passing comparisons as owner approval of new
numerical defaults. Broader runs/epochs and shared custody need their own ledgers.

## Measured exact logical bytes

| Quantity | Measured value |
| --- | ---: |
| Candidate canonical source | 618 bytes |
| Accepted result canonical source | 744 bytes |
| Initial / accepted summary material | 832 / 1,287 bytes |
| Admission complete changed logical set | 23 rows / 28,371 bytes |
| Acceptance complete changed logical set | 23 rows / 30,151 bytes |
| Charged receipt pool after either command | 163,840 bytes at each scope |

These command totals use actual projected row bytes, not conservative full-slot
allowances. Slot **capacity** remains 16,384 bytes per retained/reserved slot.
Receipt-row fit still needs the accepted per-slot combined metadata plus physical
overhead qualification; command fit and reserved pool capacity are distinct checks.
The 23 acceptance rows include no payloads, but include the current summary and
its two resource-ledger changes. They are a complete witness-specific set, not a
maximum-profile estimate or a claim every accepted result uses 23 rows.

Both commands fit the proposed logical 16 MiB/4,096-row ceilings in this projection.
The physical feasibility of this state is UNVERIFIED. Nothing here qualifies the
one-worker lease lifetime, SQL deadlines, journal/scratch limits, native flush/
recovery or finalization exclusion against writable ownership. No custom VFS,
external payload store, split publication or numerical limit increase is selected.

## Reproduction and negative checks

From the repository root:

```bash
node docs/validation/check-hierarchy-empty-output-profile.mjs
```

The default is read-only: rebuild in memory from pinned seeds, compare the entire
catalog byte-for-byte, verify row/manifests/outer links, exact source/receipt/head/
lookup joins and ledger reconciliation/caps, and run sixteen deliberate corruption
cases. --write regenerates this evidence file only; never use it on a store.
The cases reject wrong outer link, changed source bytes, missing/wrong ledger,
missing required commit proof, duplicate row, missing head/slot, missing/extra
manifest changes, wrong beforeDigest, wrong slot purpose/reservation, mismatched
claim/candidate references and an out-of-range slot key. The checker separately
compares each manifest to the full adjacent row-map difference, including both
before and after commitments, and recomputes its exact control/command charges.
Slot checks enforce the closed per-ordinal variant and combined logical metadata
fit, including the admission proof and lookup.
Allowed pruning of an unreferenced old proof is separately checked.

This helper is not a production strict parser, authenticated claim verifier,
SQL implementation, exhaustive schema validator, cross-language serializer test,
fuzz campaign or crash/lease experiment. Synthetic byte validation cannot prove
that approvals, source capabilities or sidecar ordering occurred. It neither
opens a database nor starts native/model probes. No bootstrap evidence is silently
upgraded by the successful byte comparison.

## Bounded review disposition

Two read-only reviews found no contradiction in the concrete accounting or declared
projection. They identified missing independent before/after manifest-diff checks
and incomplete slot/receipt semantic assertions in the first helper. Both are
addressed: every command now compares its full actual row-map difference and
both commitments against the manifest, and slot checks enforce exact purposes,
reservations, references, responses, commit joins and logical size limits.
Sixteen deliberate corruption cases exercise those checks. The source, stored
row bytes and measured command totals remain unchanged; only validation/evidence
coverage expanded. No post-edit independent technical or native review is claimed.

## Review handoff and remaining gates

The owner accepted the narrow decoded objects.body commitment projection and
explicit subject-preparation manifest kind/zero-ledger initialization for further
design/qualification, as recorded below. This acceptance covers those two choices,
not every broader component contract or a runtime-ready source profile. The
fixture-only cap schedule establishes no production source defaults. Technical
definitions, source/summary/receipt/proof/link joins, logical measurement rules
and all catalog/checker bytes remain unchanged.

The next evidence gap is a source-owned preparation/bootstrap producer contract
with exact approved snapshot and membership bindings, finite production aggregate
source/payload defaults and required custody/affected-set mappings. Then expand
canonical witnesses to the supported payload/unavailable/pending/rejection paths
and every native qualification requirement under a separately approved bounded
plan. #740, finalization/quiescence/recovery versus writable ownership, physical/
resource/query/durability feasibility and shared custody remain gates. Keep
#744/#746 open. No migration, database write, runtime implementation or launch
eligibility is claimed.

## Owner acceptance and handoff — 2026-10-25

After PR #1061 merged and the two proposed choices were presented for owner
acceptance, the owner answered “go ahead”. Accepted for further design and
qualification against reviewed source
[PR #1061](https://github.com/Rambolarsen/orkworks/pull/1061), merged as
`1a50c4f05efe29bc85992d38d2241e64f992b6cb` (reviewed head
`387dff2dd7b99637b99cc71d7a3cf96de12f7f42`):

- For the two objects source variants used by this witness, commitment body.body
  contains the strict decoded canonical candidate/result object. Physical source
  BLOB bytes must equal that object's canonical serialization and validate its
  unchanged source-domain digest and complete key. No Base64/quoted-object
  alternative, new source domain or general unknown-variant fallback is accepted.
- Explicit subject-preparation is a closed manifest kind for this synthetic
  witness's initialized-store/absent-subject case. It initializes one no-result
  summary/checkpoint and all thirteen required quota rows, including zeros.
  It cannot reset an existing subject, initialize on read, infer approvals or
  repair missing history. A production preparation actor/API and its exact
  authority, producer/context and capacity/ownership atomicity remain unqualified.

The accepted cost is exact byte/codec checking at each source-row commitment and
explicit preparation proof before subject ledgers may be used. This is scoped
choice acceptance, not blanket acceptance of the broader store/component design,
production source caps or any admitted maximum profile. The thirteen cap witnesses
stay fixture-only comparisons; the synthetic criteria/rubric binding to the seed
bootstrapConfigurationDigest remains UNVERIFIED. No approval/permission/lease/
claim history or physical feasibility is established by these hashes.

This record changes no technical definition, stored witness bytes, measured
command total, negative case, source/request/wire field, quota value or claim/lease
policy. Admission remains 23 changed logical rows / 28,371 bytes; acceptance is
23 / 30,151. Logical fit supplies no native writability or execution authority.

Source-owned bootstrap/preparation membership and snapshot bindings, finite
production aggregate source/payload defaults, custody/affected-set codecs,
payload/pending/unavailable/rejection witnesses, native query/physical/resource/
scratch/durability feasibility, finalization/quiescence/recovery ordering against
writable ownership, shared custody and separately approved executable plans remain
gates. No database, migration, native/model probe, runtime implementation or
launch eligibility is approved. #740 stays deferred/no-go until November; keep
#744/#746 open.

Next bounded design task: specify the source-owned bootstrap/preparation producer
contract and its exact approved snapshot/membership bindings. Treat any new
production quota defaults as separate proposed choices for owner review before
dependent implementation.
