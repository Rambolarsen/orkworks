---
type: Design Proposal
title: Initial root preparation membership and transaction effects
description: Proposed closed initial run, reserved parent and capability-artifact rows with exact draft/admission effects, both summary-capacity branches and shared ledger rules.
tags: [orkworks, hierarchy, preparation, storage, transactions, accounting]
status: draft
---

# Initial root preparation membership and transaction effects

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746); initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `4d62d773` after PR #1132.
**Proposed for scoped owner/producer review. No dependent fixture/checker, migration,
DDL, native probe, executable plan or runtime implementation is delivered.**

## Outcome and short plan

The [accepted root producer](2026-10-25-root-preparation-producer-design.md#owner-acceptance-and-handoff--2026-10-25)
and [source joins](../../validation/root-preparation-joins.md) establish exact
draft/approval/source shapes. The [accepted initial codecs](2026-10-26-root-preparation-source-codecs-design.md#initial-admitted-run-and-reserved-parent)
do not yet select storage keys, artifact retention or the complete changed-row
set. Close that initial logical slice here, review its new choices, then build
independent before/after witnesses only after acceptance. All native, complete
production-capacity and runtime-plan gates remain independent.

This is an initial-admission profile, not the full mutable run lifecycle. Reuse
bootstrap v2, existing source/digest/receipt fields, frozen-byte equality,
non-adoptable reservations, current-authority-before-retry and burned sequences.
Preserve the accepted exclusion of preparation from #744 claim-first authority.
Do not turn the unverified bootstrap in PR #1128/#1132 into an eligible draft or
admitted source merely because its bytes join. Future logical witnesses remain
explicitly synthetic while capability/native/source evidence is missing.

## Scope and rejected alternatives

Cover new-store resource initialization, root-draft-prepare, exact root-draft-cancel
and first subject-preparation for one fresh run/root. Input values may be absent
or inline, but no reference input is supported in this first effect profile.
The output contract must have no declarations and both aggregate caps zero;
the reviewed assignment must require no materialized deliverable. Reports, artifacts,
plan/task consumers, external custody references and accepted results are absent.
No output wrapper or input-reference custody codec is inferred from empty lists.

Retain every selected capability reference's bytes under this exact run, including
settings/model/configuration projections. Copy per reference; prohibit cross-run
sharing and digest-only reuse in this profile. This closes the otherwise omitted
source bytes without inventing a general object store or shared reference counter.
Instruction/rule/resource snapshots remain complete inline bootstrap values;
provenance is not a live-file dependency or fetch grant. Applicable source/catalog
retirement and native revalidation still require their own evidence.

Rejected alternatives: persisting an initial state in an unspecified JSON bag;
calling an attachment record session-producer proof; omitting evidence bytes from
command/accounting totals; inferring absent shared ledgers as zero; creating an
unavailable SummaryRow when its capacity was denied; and borrowing the historical
empty-output witness's synthetic source or quota ceilings as production approval.
The selected cost is three new typed row families and explicit workspace resource
initialization, with no sharing optimization or silent old-store adoption.

## Closed initial registry additions

Use the existing format-3 logical `{table,key,body}` row commitment and canonical
integer/string/ID/Digest/CommitRef rules. Unknown fields/tags, duplicate keys,
noncanonical stored bytes and owner/hash drift reject the whole command. The
following are new proposed registry entries; their presence here does not make
an existing format-3 database accept them. No wire/storage migration is selected.

| Table / exact key | Exact logical value and initial-only rule |
| --- | --- |
| run_states / `{workspaceId,runId}` | `{schemaVersion:1,initialInput:InitialInputV1,state:InitialRunStateV1,lastCommit:CommitRef}`; only the accepted empty-decision, assessing, null-runtime variant |
| parent_attachments / `parentSessionId` | `{schemaVersion:1,attachment:ParentAttachmentV1,lastCommit:CommitRef}`; only the accepted reserved variant, with no runtime generation property |
| preparation_artifacts / `{workspaceId,runId,referenceId}` | `{schemaVersion:1,contentBase64,sizeBytes,contentDigest,lastCommit:CommitRef}`; one immutable exact-byte occurrence per capability reference |

run_states and parent_attachments values each have a proposed inclusive 4,096-byte
ceiling. With inherited maximum ASCII IDs, an 84-byte epoch/sequence run ID,
64-byte digests and a 19-digit CommitRef sequence, their initial closed values
are at most 1,694 and 925 canonical bytes respectively. These are author sizing
calculations for these exact fields, not native measurements or executed vectors.
The proposed ceiling includes the complete wrapper/link, not just the nested state.
Later lifecycle/input/attachment variants require separately reviewed codecs,
effects and bounds; no generic updater or startup action is supplied here.

initialInput and state use the accepted codecs unchanged. Recompute inputDigest
from exact InitialInputV1 bytes under `orkworks.preparation-input.v1`; retain the
complete initialInput instead of recovering it from that hash. All source,
workspace/run/parent/root/bootstrap identities join the same retained preparation.
The run-state key equals the embedded state; the attachment key equals its reserved
parent ID. A parent ID has at most one row in this store and exactly one matching
run/root. Sidecar-owned fresh non-adoptable reservation and cross-workspace/session
uniqueness remain source-owner obligations; a local row key proves neither.
An ordinary/foreign session cannot supply this reservation.

sizeBytes is a canonical nonnegative Counter equal to the decoded byte length.
contentBase64 uses the standard alphabet and required padding; decoding/re-encoding
must preserve exactly the stored text. contentDigest is raw SHA-256 of those bytes
and equals that reference's digest in the frozen capability snapshot. Reference
IDs and kinds/locators are resolved through that one complete snapshot; rows cannot
add unlisted references. Keep the existing at most 16 references and 1 MiB sum of
raw artifact occurrences per snapshot. Each raw artifact therefore fits 1 MiB;
selected settings/model/configuration records retain their smaller 64 KiB limits
and exact canonical payload rules. Duplicate content under different reference IDs
is separately retained and charged; no physical or logical sharing discount.

A preparation_artifacts value has a proposed inclusive 2 MiB ceiling. Base64 for
a maximum raw occurrence uses 1,398,104 ASCII bytes, plus its bounded metadata/link.
The larger encoded-row ceiling does not raise the existing 1 MiB raw aggregate
or any source/command budget. Raw bytes may be binary; do not normalize Unicode,
line endings or JSON before their existing raw digest. Known secret-bearing,
hidden-reasoning or forbidden source capture remains rejected by the owning
privacy/evidence contract. Byte retention does not establish semantic evidence.

For all three new tables exclude only lastCommit at its named outer path from
the row commitment. Keep keys, version, complete initial state/input, attachment,
Base64 bytes, size and contentDigest in the committed body. Do not adopt the
payloads table's omitted-bytes projection for this new artifact family. Preserve
all existing root_drafts/root_preparations exclusions, including only the latter's
receipt.commit. Every excluded link still resolves to its exact retained proof;
exclusion prevents a digest cycle, not integrity checking. No link-only rewrite.

## Ownership, retention and bounded reads

A run's artifact rows are owned by exactly one pending draft or retained source
with the same workspace/run and exact snapshot references. Draft preparation
inserts them with the draft. Admission keeps the same rows and atomically replaces
the draft owner with the retained source; unchanged artifact rows keep original
lastCommit links/proofs. Exact cancel deletes the pending draft's rows only when
no admitted source exists. No arbitrary artifact deletion, global digest lookup,
extra runtime read action or automatic eviction is introduced. Root/source purge,
shared custody and later consumers remain separate closed protocols.

The command owner derives the read/affected set, never the caller. Read the exact
workspace draft, run/source/state/parent/subject keys and typed resource keys.
Enumerate this run's artifact prefix with a 17th-row overflow sentinel: it must
match the selected reference-ID set exactly, at most 16, with no duplicates or
unlisted rows. Rehash every referenced occurrence. Validate exact retained source
and frozen review bytes as required by the accepted producer, not display labels.
This is a logical bounded lookup recipe; physical index/query and native I/O
limits remain unqualified and still consume the inherited visited-row budget.

Fresh namespace/reservation proof, absence of prior source/state/subject and exact
parent non-adoption are checked before preparing the initial effect. Missing
historical ownership, header/proof or required shared ledger fences the operation;
absence is not an empty store. Current source/UI/fence/eligibility checks remain
mandatory at their original boundary, including authenticated retry lookup.

## Resource keys and zero initialization

Reuse the existing [candidate resource/owner codecs](2026-10-25-hierarchy-candidate-command-manifests-design.md#closed-resource-ledger-subset).
Propose exactly these additions, outside all source digests:

| Resource / owners | used and reserved |
| --- | --- |
| preparation-retained-bytes / run, workspace | used is the sum of canonical key bytes plus committed-body projection bytes for root_drafts, root_preparations, run_states, parent_attachments and preparation_artifacts occurrences owned by that scope; reserved is zero in committed rows |
| preparation-run-count / workspace | used is the count of distinct run IDs with a pending draft or retained source, including closed runs and old epochs; reserved is zero |

The run/workspace owner mapping is exact and independent of which table supplies
the physical primary key:

| Row family | Workspace/run supplying both charged owner scopes |
| --- | --- |
| root_drafts | value.draft.workspaceId and value.draft.runId; the workspace-only table key must equal the former |
| root_preparations | value.bootstrap.workspaceId and value.bootstrap.runId, equal its full source/run/key bindings |
| run_states | value.state.workspaceId and value.state.runId, equal initialInput and the complete key |
| parent_attachments | value.attachment.workspaceId and value.attachment.runId; its parent key joins that exact run/root |
| preparation_artifacts | key.workspaceId and key.runId, joined to the exact owning draft/source reference set |

Charge every occurrence to `{kind:"run",workspaceId,runId}` and also to
`{kind:"workspace",workspaceId}` from this mapping, including a reserved run
before admission. No current selected-run pointer, row table key alone, display
label or latest source chooses the owner. A draft-to-source transfer retains the
same run/workspace owner scopes: delete the draft occurrence, add the newly
retained source/state/attachment occurrences, and keep artifact occurrence charges
unchanged. It does not reassign an artifact or draft byte to another run.

The same row occurrence may be bounded by a source, preparation, command and
physical quota independently; that is not duplicate physical storage accounting.
The preparation projection omits only the named commit links just listed. Their
full bytes remain in command/main/physical accounting. Count repeated inline
bootstrap/run/instruction bytes wherever actually retained in different row
bodies. Artifact encoding overhead is included. Do not count a source receipt as
a #744 reporting slot or candidate/result/evaluation/disposition object.

For this proposed profile only, explicit new-store initialize-store creates all
nine workspace quota rows at zero along with header/new proof: the existing three
reporting/record/payload resources, four summary resources, and these two new
preparation resources. This extends the historical header-only fixture profile;
it neither edits that fixture nor permits an existing store to invent missing
rows. Adoption/migration/backfill of an existing store is out of scope and no-go
until separately designed. A matching header alone cannot establish newness.

Draft preparation may initialize the fresh run's preparation-retained-bytes row
with its exact initial charge. At admission, initialize only the new assignment's
three reporting/record/payload rows and the still-unadmitted run's corresponding
three rows, all at zero. Their prior absence is justified by the fresh reserved
run and absence of any admitted source/state or reporting ownership; an existing
or ambiguous resource row is not reset. Workspace resources always already exist
and must reconcile with retained ownership. Existing unrelated run/subject rows
and old epochs are preserved. Ordinary evaluation/candidate writers continue to
use the same existing workspace/run/assignment pools; this creates no extra pool.

For a changed row r, let B(r) be canonical key length plus canonical committed-body
projection length for the preparation resource. Sum insert/delete/replacement
deltas per owner/resource, then replace each changed quota row once. No change
means no quota/link rewrite. Draft-to-source transfer changes preparation bytes
by the actual projection difference; its run count stays one. Exact cancel may
remove that unadmitted run's preparation ledger and decrement its workspace
count/bytes only after proving no source or other retained ownership exists.
The admission sequence remains burned; resource release never permits identity reuse.
Closed admitted sources remain charged until separately authorized complete purge.

**No production numerical run/workspace preparation-byte or run-count caps are
selected here.** Finite caps and source-history/aggregate policy require scoped
owner review before any executable profile. Absence of those limits is a no-go,
not unlimited capacity or permission to reuse receipt-pool/fixture ceilings.
Existing reporting and summary limits retain their own status and values.
Never-initialized summary markers still consume complete command/main/physical
storage and bounded retained-run inventory, although they are not SummaryRowV1
payloads in the summary byte/count ledgers.

## Exact command effects

Let C be the next checked CommitRef from the exact current header, A the root
AssignmentIdentity copied from the validated bootstrap, Q the final changed quota
rows derived above, and K exactly header replacement plus new proof insertion.
Only actually changed (table,key) rows appear; compute one original before and
one final after value. All commands preserve unrelated rows and retain existing
needed proofs. This profile prunes no old proofs. Manifest changes are the whole
changed set minus K, uniquely sorted under the unchanged format-3 domains.
Header/new proof and every changed outer link must join C; unchanged artifacts
and other rows keep their original proof links. Commit Counter overflow denies.

| Command | Core effects, in addition to Q and K |
| --- | --- |
| initialize-store for this profile | No source or subject; exactly the nine zero workspace quota rows and initial header/proof, null predecessor |
| root-draft-prepare | Insert one immutable root_drafts row and exactly the selected artifact occurrences. No admitted source, run state, parent attachment, summary, reporting slot or process |
| root-draft-cancel | Delete exactly that pending draft and its artifact occurrences. No retained source may exist; no other run or admitted history is removed |
| subject-preparation | Delete the exact draft; insert original root_preparations receipt/source, initial run_states, reserved parent_attachments and summary_state marker. Keep artifact occurrences unchanged and owned by the new source. No result/head/candidate/work/receipt-slot/claim-proof/mutation-lookup/event/plan/runtime rows |

Draft prepare requires no pending workspace draft, no existing keys/ownership for
the fresh run and no parent adoption. Durable namespace reservation occurs first
under the existing separate fence protocol; a later store denial cannot unburn
it. Store initialization and draft preparation are separate commands. An orphaned
artifact prefix or partial old draft cannot be treated as a clean new reservation.

Cancel requires the exact current draft ID/digest and valid UI scope/context under
the existing ordering. Revocation makes the context unusable independently of
whether a corrupt/over-budget store can finish deletion; no denied cancel can
resurrect authority. A cancel racing admitted source publication has one serialized
winner. An old cancel cannot delete a replacement or admitted run.

Subject preparation requires the exact frozen eligible source/draft/approval and
current reservation proof, absent initial membership/subject rows and a complete
matching artifact prefix. The new initial state and input use the unchanged
accepted values, state assessing with no runtime/plan/blocker/decision, revision 1.
The attachment is reserved only. Copy the accepted RootPreparationReceiptV1 into
the source row with commit C, equal source.lastCommit. It is not a start receipt.
Receipt retry is an authorized read of that exact retained tuple/bytes under the
existing cutoff: identical authorized retry returns the original receipt with no
Q, K, source, sequence, proof or process change. Altered tuple/bytes conflict;
missing source never permits reconstruction from an old approval/high-water.

The six new zero reporting-resource rows in subject preparation are mandatory
logical effects even though used/reserved are zero. root preparation creates no
reporting receipts or reserved slots. Workspace reporting resources do not change.
The run/workspace preparation byte rows reflect the draft deletion and all newly
retained source/membership bytes. Workspace preparation-run-count is unchanged
and must not be rewritten merely to stamp the new commit.

## Initial summary: both capacity branches

Derive criteria/rubric from the exact accepted source, using #744's existing sorted
criterion snapshot and rubric snapshot domains. Never use the old witness's seed
or a caller-supplied empty snapshot. Initial material M0 uses exact A, accepted
`{kind:"none"}`, those snapshot digests, overall unassessed, every required
criterion, rubric dimension and rating unassessed in canonical target order, and
no required findings. This fresh initial profile has no accepted result/evaluation
or independent failure source. It asserts no task success.

If row-count, row-byte and checkpoint-lineage capacities all fit, insert exactly:

```text
summary_state[A] =
 {subjectRevision:"1",admission:"initialized",
  row:{schemaVersion:1,assignmentIdentity:A,summaryRevision:"1",
       state:{kind:"current",material:M0}},
  checkpoint:{material:M0,summaryRevision:"1",applied:C},lastCommit:C}
```

Increment only the existing workspace summary-row-count by one,
summary-row-bytes by the exact complete SummaryRowV1 bytes and
summary-lineage-bytes by exact checkpoint material bytes. summary-pending-count
is unchanged; no pending transition or predecessor exists for a new subject.
The duplicate material occurrence in row and checkpoint counts in its respective
existing ledger and both copies count in complete-command/physical bytes.

If any of those summary capacities is unavailable, admission may instead insert:

```text
summary_state[A] =
 {subjectRevision:"1",admission:"never-initialized",row:null,
  checkpoint:null,lastCommit:C}
```

All four workspace summary ledgers remain unchanged. No SummaryRowV1,
summaryRevision, material/checkpoint or pending row is fabricated. Surface the
existing unavailable status through the owner projection. A later read or newly
free capacity cannot initialize this already admitted subject. Never replace an
existing initialized/exhausted lineage with this marker; this branch applies only
to a proven new subject. This reuses the accepted capacity policy, not a retry or
new reconciliation behavior.

Both branches still require capacity for their entire durable source/membership,
quota/control/proof set and the marker itself. If those cannot fit, deny the whole
admission and retain the exact pending draft/source artifacts; do not commit source
without its marker or split membership/summary into a later command. No runtime
or reporting capability is delivered by either branch.

## Logical size and effect-set checks

Count canonical key plus full new value for every inserted/replaced row, including
all outer links, Base64 occurrence bytes, zero ledgers, header and new proof.
For a deletion charge its key plus its actual manifest/control effects; give no
negative command-byte credit. The new proof value already contains manifest bytes;
do not add the same physical logical occurrence twice. Enforce the existing
proposed 16 MiB/4,096 changed-row ceilings and every smaller source/row limit.
Physical BLOB/page/index/journal/scratch and memory copies remain separate gates.

For n selected capability references, at most 16 with all mandatory source
references still required by the bootstrap, the declared initial-only effect sets have:

| Command | Changed rows when all required rows are as specified |
| --- | ---: |
| New-store initialize-store | 11: nine zero resource rows plus K |
| Draft prepare | n + 6: draft, n artifacts, run byte ledger, two workspace preparation ledgers, K |
| Exact draft cancel | n + 6: draft/artifact/run-ledger deletions, two workspace preparation replacements, K |
| Subject preparation with initialized summary | 18: five core changes, six new zero reporting ledgers, two preparation-byte replacements, three summary ledger replacements, K |
| Subject preparation with never-initialized summary | 15: same set without the three summary ledger changes |

These counts are a symbolic recipe check, not byte fixtures, maximum payload
eligibility or a native measurement. An extra required dependency, event, session,
index or accounting row must have a reviewed closed codec and be included before
claiming completeness; it cannot be hidden behind these counts. This profile has
no dependency-table effects: instruction snapshots are inline, artifacts are
copied into the same run owner, reference inputs/materialized outputs/consumed
reports and plans are excluded. A producer needing cross-owner custody or retained
consumer edges does not qualify this profile. No same-owner join creates a fake
cross-owner dependency merely to fill an accounting placeholder.

## Written rejection and recovery oracles

| Case | Required result |
| --- | --- |
| Missing initialInput bytes but matching inputDigest | Reject membership; no reconstruction from hash |
| Reserved attachment contains a runtime generation or ordinary session adoption | Reject initial profile; no startup or credential |
| Capability reference has no exact retained occurrence, a digest-only pointer, unknown prefix member or more than 16 rows | Reject complete source; no extra object-store path |
| Two reference IDs share bytes | Retain/charge both occurrences; no per-digest discount |
| Reference input or required materialized output appears | Reject this profile pending that producer's custody/effect contract |
| Existing workspace quota row absent or zeroed despite other owners | Fence writes; missing history is not zero |
| Existing run/assignment reporting keys conflict with allegedly fresh reservation | Reject initialization; never reset them |
| Draft transfers to source | Keep artifact rows/proofs, replace owner atomically, count one retained run |
| Exact authorized receipt retry | Return original bytes; no new commit, slot or charge |
| Matching stored bytes after context revocation | Deny mutation/retry under existing authority cutoff |
| Summary count, bytes or lineage cannot fit | Commit never-initialized marker only if the whole remaining command fits |
| Never-initialized subject later has free row capacity | Stay unavailable; no lazy initialization |
| Marker/proof/shared accounting cannot fit | Deny complete admission; preserve draft and burned reservation |
| Crash after fence reservation, before a full store commit | Sequence remains burned; no fabricated active run |
| Crash during subject-preparation | Qualified recovery must establish complete pre-state or complete source/membership/marker/accounting state; hashes alone do not prove SQL atomicity |
| Artifact owner changes but old artifact links still name draft commit | Preserve that proof while referenced; do not rewrite links solely to hide history |
| Same-user rollback, missing header/proof or ambiguous native outcome | No authenticity claim or latest-counter repair; fence as required by owning contracts |

## Review choices and handoff

Owner/producer review should accept or revise: the initial-only run_states and
parent_attachments storage keys/wrappers and 4 KiB bounds; per-reference run-owned
preparation_artifacts with Base64/raw-byte commitments and 2 MiB encoded-row bound;
the two preparation resource meanings and new-store nine-zero-row baseline; the
three exact effect recipes and inherited summary-capacity mapping; and the narrow
no-reference-input/no-materialized-output/no-sharing profile. These are author
proposals, not choices already accepted by permission to draft or merge this text.

After acceptance, a bounded independent byte packet should cover new-store/draft/
cancel and both admission branches with full before/after maps, exact manifests,
source/run/artifact/parent/summary joins, every changed shared ledger, retained
proofs and deliberately nonzero unrelated ownership. If fixtures compare usage with
ceilings, publish a finite fixture-only comparison schedule and distinguish it
from unresolved production defaults; a passing witness cannot approve a cap.
Preserve all prior catalogs
and baseline source bodies. Do not reuse the old synthetic store seed as source
qualification. Test over-budget, missing-ledger, duplicate/missing artifact,
self-consistent substituted-source, retry/cancel race and crash-gap cases at the
appropriate logical/native layer; a source fixture cannot execute native races.

Before any production qualification/executable plan, separately close finite
production preparation/source/history/run-count and overall shared-custody caps,
all existing storage numerical/physical/resource/query gates, source/catalog
retirement and native artifact/session evidence, excluded-action revocation/commit
ordering and later lifecycle/startup producers. #740 remains deferred/no-go until
November and all six role profiles remain UNVERIFIED. No database, migration,
model/native probe, runtime implementation or launch eligibility follows here.
Keep #741/#742/#744/#746 open.


## Bounded review disposition

One read-only correctness pass found no material contradiction in the selected
logical effects, source/accounting scope or authority boundaries and reconciled
the symbolic changed-row counts. A separate fresh builder check asked which run
owns workspace-keyed pending draft bytes. The author made the source-field owner
mapping explicit for every row family: both scopes come from that exact reserved
run even before admission; transfer preserves scopes and changes occurrence sums.
That clarification follows the existing frozen draft/run identities, without a
new assignment or selectable owner. Fixture-only cap comparisons are also
explicitly separated from production-cap approval. No post-edit independent
rereview, source/native qualification or owner acceptance is claimed.
