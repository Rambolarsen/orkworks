---
type: Design Proposal
title: Hierarchy storage caller inventory and strict publication contract
description: Source-traced writer routes, proposed storage rows and byte fixtures, complete-command accounting and the unresolved authority-commit race.
tags: [orkworks, hierarchy, storage, sqlite, recovery]
status: draft
---

# Hierarchy storage caller inventory and strict publication contract

Tracking: [#746](https://github.com/Rambolarsen/orkworks/issues/746),
[#744](https://github.com/Rambolarsen/orkworks/issues/744), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `0ed5de99` (merged PR #1029), 2026-10-24.
**Proposed detailed design; not an executable handoff.** The owner accepted only
one selected-workspace SQLite database/private worker with DELETE/EXTRA for
further design/qualification. This document supplies that next preparation;
new row shapes, accounting choices and fixtures below require review. No source
schema, permission, quota or native profile is accepted or changed here.

Read with the [storage design](2026-10-23-hierarchy-transaction-storage-design.md),
[accepted recovery behavior](2026-10-20-hierarchy-read-recovery-design.md),
[#744 technical acceptance](2026-10-04-assignment-evaluation-design.md#technical-contract-acceptance--2026-10-24)
and [qualification register](../../validation/hierarchy-writer-storage-qualification.md).
Those domain contracts win on authority, canonical bytes, replay and retention.
This is static source inspection and synthetic byte/arithmetic work only: no
application/database invocation, native fault experiment, model probe or code
implementation. #740 stays no-go/deferred until November.

## What this pass establishes

The current writers do not participate in a hierarchy publication module.
The inventory below traces concrete callers and bypasses, distinguishing existing
ordinary writes from proposed hierarchy producers. It is a bounded baseline for
an implementation-base re-audit, not certification of all external installations.
A missing route or unknown external producer prevents complete hierarchy reads.

The proposed storage rows can preserve exact source records without adding
storage metadata to their digests. Their complete-command accounting reveals
that several independently valid source maxima cannot be published together.
There is also no demonstrated ordering that meets immediate revocation denial,
SQL-commit linearization and nonblocking ordinary stop simultaneously. That
race remains a specific no-go, rather than an implied mutex implementation.

## Caller and direct-write inventory

Paths and line anchors below refer to the inspection base. `O` means announce
pending ordinary copies before mutation, then publish immutable copies and a
checked generation after the existing write lane is released. `H` means announce
an immediate health/authority fence. `D` means a typed durable command containing
all source/receipt/dependency/summary effects. `R` means capture and final token
validation, with no repair or initialization on a hierarchy read. These are
required future routes, not installed wrappers.

| Existing caller chain / direct writer | Required route and verification case |
| --- | --- |
| HTTP create/resume/status/delete → `SessionApplication::{create_session,resume_session,transition_session_status,delete_session}`; launch/resume helpers `try_install_claimed_resume_handle`, `ResumeGuard::drop`, and initial metadata publication in [session_application.rs](../../../crates/orkworksd/src/session_application.rs) (2196, 3923, 5178, 5394, 5472, 6326) | O for membership/lifecycle copies, H for runtime replacement/revocation. Failed launch/rollback must publish the actual restored/removed handle, never leave a copied live child or recreate authority |
| PTY/startup/exit events → [session_runtime.rs](../../../crates/orkworksd/src/runtime/session_runtime.rs) `handle_runtime_exit`, startup running writes (1200), output-recency write (2589), generation-bound status calls (1911, 1925, 2117) → [terminal_runtime.rs](../../../crates/orkworksd/src/runtime/terminal_runtime.rs) `set_session_status_for_generation`, `finalize_session_ending` → `complete_session_ending` | O/H. An obsolete generation cannot publish replacement runtime values; output draining/stop cannot wait for hierarchy SQL. Attach/detach alone is not runtime end |
| Peon scan → [peon_runtime.rs](../../../crates/orkworksd/src/runtime/peon_runtime.rs) label assignment (599), `persist_input_label` (902), `persist_peon_observation` (964), idle timeout (1142); runtime recency/process events → `persist_output_recency`, `persist_process_transition` | O. Direct live-label/recency writes must announce too; wrapping only metadata persistence misses them. A scan racing capture forces retry without creating a summary transition |
| Terminal input → [terminal_runtime.rs](../../../crates/orkworksd/src/runtime/terminal_runtime.rs) `record_terminal_input`, process-attention changes (1009, 1098), prompt/reset paths → `SessionApplication::{record_user_input_topic,reset_session_topic,commit_prompt_identity_reset_under_transition}` | O/H. Prompt, identity and attention copies change coherently without becoming result evidence; preserve native/user precedence |
| Hook/agent/debug/report HTTP → `SessionApplication::{apply_attention_signal,report_attention,report_prompt_attention,apply_debug_attention,report_harness_session_with_codex_context,report_plan_path}`; authenticated metadata PATCH → [session_metadata_handlers.rs](../../../crates/orkworksd/src/http/session_metadata_handlers.rs) `apply_agent_patch` then direct handle copies (145 onward) | O/H. API success, validation-only requests, rejected patches and direct handle copies have distinct outcomes. A rejected/validation-only write cannot invent a generation/material change |
| Native approval → [codex_approval_application.rs](../../../crates/orkworksd/src/runtime/codex_approval_application.rs) metadata lane and live updates (360, 388, 463, 483, 810, 834, 879, 886); dormant completion → [codex_completion_application.rs](../../../crates/orkworksd/src/runtime/codex_completion_application.rs) live update (166); diagnostic → [codex_native_diagnostic.rs](../../../crates/orkworksd/src/runtime/codex_native_diagnostic.rs) writes (474, 715, 719) | O/H if the path is ever eligible. Native/dormant/diagnostic code is not a qualified production producer. Preserve its existing gates; missing coverage keeps hierarchy unavailable rather than activating it |
| Application labels/model/native identity/topic/plan selection → `SessionApplication::{persist_codex_label_for_runtime,persist_input_label,report_harness_session_with_codex_context,persist_printed_plan_fallback,select_plan,set_active_harnesses_at}`; [observed_status.rs](../../../crates/orkworksd/src/runtime/observed_status.rs) live helpers | O/H where identity affects authority. Audit both helper and direct `handle.info` assignments; changing mutable model/label display cannot rewrite immutable approved configuration |
| Per-session persistence → [metadata.rs](../../../crates/orkworksd/src/metadata.rs) `try_write_session_owned`, `try_mutate_session_owned`, `try_write_session_if_owned_when`, `write_session`; revisioned writes → [session_transactions.rs](../../../crates/orkworksd/src/metadata/session_transactions.rs) `update_session_locked`, `commit_session_record_with_revision_before_replace` | O around all entry variants, not just PATCH. Each `MetadataStore` instance owns its own lane map. File rename and the before-replace callback are not a joint SQLite commit or evidence of cross-instance serialization |
| Workspace open/startup → `SessionApplication::open_workspace_with_identity` (3688): lease acquisition, migration, catalog replacement (3758), remembered metadata reconciliation (3840); [migration.rs](../../../crates/orkworksd/src/migration.rs); [main.rs](../../../crates/orkworksd/src/main.rs) catalog construction | H before workspace/lease/copy-scope change; O on changed copies. No complete hierarchy response until all included ordinary inputs are recaptured. Migration or startup normalization cannot initialize missing hierarchy lineage |
| Harness create/patch/retire/profile handlers → [harness_handlers.rs](../../../crates/orkworksd/src/http/harness_handlers.rs) `mutate_at` callers (137, 210, 342, 464) → [HarnessStore](../../../crates/orkworksd/src/harness/store.rs) `mutate_at` (255), catalog replacement (296); `snapshot`/workspace reload | O for catalog copies; D only for explicit new immutable approved configurations. Global document changes from another process are not serialized by a selected-workspace latch; unknown freshness fences complete reads |
| Session-list read → [SessionProjection](../../../crates/orkworksd/src/session_projection.rs) `snapshot`, `list`, `project_capacity` (332–557) applies capacity observations and live pending flags; `read_remembered_sessions` reads revisioned metadata | O for read-triggered capacity writes; R for the new hierarchy read. Existing list's outer projection lock and separate catalog/live/disk acquisitions are not a coherent hierarchy snapshot. A list call racing capture must invalidate copied capacity |
| Forget/archive/delete → [session_actions.rs](../../../crates/orkworksd/src/session_actions.rs), `SessionApplication::forget_session`; background [retention.rs](../../../crates/orkworksd/src/runtime/retention.rs) `retention_cleanup_once` → `delete_session_evidence`, session-map removals; [workspace_gc.rs](../../../crates/orkworksd/src/runtime/workspace_gc.rs) | H before destructive effects; D only for approved hierarchy subject purge under exact revision/incoming dependencies. Existing multi-store deletion can fail partway and cannot prove shared custody or atomic hierarchy purge |
| Read-triggered corrupt metadata quarantine → [metadata.rs](../../../crates/orkworksd/src/metadata.rs) `load_session_file` (1366); missing/digest-invalid retained source, failed directory/record read, recovery | H immediately; D for any authoritative source-health/material change. Quarantine/read failure cannot look like an empty successful hierarchy or reset an initialized summary |
| Checked-in reporters: [session-metadata.py](https://github.com/Rambolarsen/orkworks/blob/0ed5de99b1fd34b7922018ae2ff6a51a59c60b87/crates/orkworksd/scripts/session-metadata.py), `report-harness-event.sh/.ps1`, `opencode-session-reporter.js`, `report-opencode-session.sh` | The Python helper uses the metadata API and forbids file fallback; hooks use attention/identity/plan routes. This scan proves checked-in route choices only. #789 migration, installed instruction copies and unsupported direct-file agents remain unverified; do not infer mediation from helper availability |

Inspection used symbol/call searches plus direct `handle.info`, session-map,
catalog replacement, metadata rename and deletion searches in sidecar production
source; checked-in scripts and Electron call sites were searched for session-file
writers. Test fixtures are not producers. Git/output history, recommendation
stores, image context and ordinary summary logs retain their own persistence:
they are not new #743 events or #744 results. Include their health/dependency
route if a future approved hierarchy input actually references them. This pass
does not approve copying every ordinary field into hierarchy material.

### Proposed producers without production callers

| Contract owner | Durable commands/effects that must remain inseparable |
| --- | --- |
| #741 | Admit exact immutable configuration/bootstrap/snapshots; retire references without mutating approved bytes. Catalog labels use O, not source replacement |
| #742 | Run/plan/task/reservation/grant/clarification/allocation changes, approval/revocation, reports/heads/receipts, affected preparation revision/blockers. Existing `CoordinatorStore` is data-only and stays independent |
| #743 | Stream provisioning, event, replay/alias/tombstone, high-water/gaps, aggregates, correction/coverage/clear/trim/finalization. No generic #744 receipt substitute or quality inference |
| #744 | Candidate/rehash reservation and terminal outcome, final result/output snapshots, evaluation/disposition/head/slot changes, dependencies, subject revision, summaries/checkpoints/lineage and protected receipt credits |

There is no qualified hierarchy implementation for these routes. A later plan
must map every command variant to its exact caller and denial test, including
internal finalization/recovery/retention. No route may accept caller SQL or a
caller-supplied affected-subject list as authoritative.

## Proposed strict storage representation

This is a logical relational contract for review, not executable DDL. Columns
listed are exhaustive; absent values require the explicit nullable type below.
Unknown tables/columns/versions/tags, duplicate keys, malformed canonical bytes,
unknown source kinds and broken joins make the store unavailable. Never migrate
or normalize an unknown store on read. SQLite physical layout, indexes and query
plans need a separately reviewed adapter; no trigger/function-generated domain
state or arbitrary JSON extension column is permitted.

`Id`, `Digest`, tagged assignment/result/evaluation/report identities and source
bounds are exactly the owning contracts. `Counter` is canonical ASCII decimal
TEXT for u64, checked numerically in Rust: `0` or a nonzero digit followed by up
to 19 digits, value ≤18446744073709551615. A positive counter excludes zero.
BLOB values carry exact bytes; keys use canonical JSON BLOBs with byte equality,
not locale collation, hash-only identity or a concatenated delimiter key.
Source JSON numbers retain their source numeric encodings; storing a separate
counter as TEXT does not rewrite a source record into string-valued integers.

`ObjectKey` is exactly `{kind, identity}`. The closed registry below defines
`identity`; no caller-defined kind/identity is admitted. `ObjectRef` is exactly
`{key: ObjectKey, digest: Digest}`. A source digest uses its existing owner domain;
storage-row commitments use the envelope defined below. A `CommitRef` is exactly `{storageEpoch: Digest,
commitSequence: positive Counter, changeDigest: Digest}`.

| Logical table / primary key | Exact remaining columns and checks |
| --- | --- |
| `store_header` / singleton `1` | `schemaVersion: 1`, `workspaceId: Id`, `storageEpoch: Digest`, `latest: CommitRef`; epoch/workspace agree with every row/ref. Header/latest is control metadata, excluded from its own change manifest |
| `objects` / canonical `ObjectKey` | `body: BLOB`, `digest: Digest`, `lastCommit: CommitRef`; body strictly validates against the registry, reconstructs its exact key and passes its domain digest. Immutable objects cannot be overwritten with different bytes |
| `payloads` / exact `(AssignmentIdentity, resultRevision, declarationId)` canonical key | `bytes: BLOB`, `sizeBytes: Counter`, `contentDigest: Digest`, `lastCommit: CommitRef`; size equals byte length and approved declaration; digest is existing raw-content SHA-256. Result publication and these new bytes are one transaction; no external path pointer substitutes for retained bytes |
| `heads` / exact tagged stream key | `target: ObjectRef`, `lastCommit: CommitRef`; the closed HeadKey variants below; target belongs to that exact stream and accepted predecessor transition. No largest-revision head election |
| `dependencies` / `(consumer: ObjectRef, target: ObjectRef, purpose)` | `lastCommit: CommitRef`; purpose is `review-input`, `evaluation-evidence` or `proposal-input`. Exact incoming/outgoing membership is owned by storage; deletion checks retained consumers, not just current heads. Additional dependency kinds require explicit registry review |
| `summary_state` / full AssignmentIdentity | `subjectRevision: Counter`, `row: SummaryRowV1 or null`, `checkpoint: Checkpoint or null`, `admission: never-initialized or initialized`, `lastCommit: CommitRef`; null row is allowed only for never-initialized. Checkpoint is required for initialized non-exhausted lineage; absence/corruption cannot initialize from latest sources |
| `pending_material` / `(AssignmentIdentity, assignedRevision: positive Counter)` | `before: SummaryMaterialV1`, `after: SummaryMaterialV1`, `origin: CommitRef`; canonical bytes differ, identity agrees, assigned revisions/predecessor material form one intact ordered chain from checkpoint. Both bytes count toward lineage capacity |
| `receipt_slots` / `(AssignmentIdentity, slotId)` | `state: reserved or consumed`, `purpose`, `result: exact ResultRef or null`, `stream: exact ReviewerIdentity or null`, `receipt: ReceiptBinding or null`, `lastCommit: CommitRef`; purpose/owner rules below; consumed iff receipt is present |
| `candidate_work` / exact candidate ObjectRef | `reservedRehashBytes: Counter`, `consumedPasses: Counter`, `state: open or accepted or rejected or superseded or abandoned`, `outcomeReceipt: slotId or null`, `lastCommit: CommitRef`; preserve conservative #744 reservations/no refunds and terminal outcome rules, including nonzero charge for empty passes |
| `quota_usage` / `(owner, resource)` | `used: Counter`, `reserved: Counter`, `lastCommit: CommitRef`; owner is exact assignment, owner-run or workspace/epoch scope. Resource is a reviewed source-specific byte/count/receipt/rehash category, never an arbitrary caller string. Recompute and compare on reload; missing/ambiguous counts fence writes |
| `commit_proof` / positive `commitSequence` | `body: ChangeManifest`, `changeDigest: Digest`; retain latest and every proof named by a retained receipt/checkpoint/transition/row; removal only with proven unreferenced state under the same transaction. This bounded proof set is charged to command/main-file capacity, not an unbounded audit log |

`Checkpoint` is exactly `{material: SummaryMaterialV1, summaryRevision: positive
Counter, applied: CommitRef}`. The existing `SummaryRowV1` and material are
unchanged. Exhausted rows remain terminal and need no impossible future material
increment; never-initialized must not acquire a checkpoint through read/recovery.

`HeadKey` is exactly `{kind, identity}`: `research-report` uses #742's exact
stream key (workspace/run/plan/revision/task/reservation/child/configuration);
`assignment-result` uses AssignmentIdentity; `assignment-evaluation` uses exactly
`{assignmentIdentity, resultRevision, resultDigest, reviewerIdentity}`. Head targets
must resolve the corresponding variant; a resumed runtime never changes a key.

`ReceiptBinding` is a closed union of storage metadata, not a new wire receipt:

- Mutation: exactly `{kind: "mutation", protocol: "assignment-report-v1",
  scopedKey, requestDigest, record: ObjectRef, responseBytes, commit: CommitRef}`.
  #744 defines the scoped key, request digest and exact ReportingMutationReceipt
  response. Candidate/evaluation/disposition objects must match that response.
- Candidate finalization: exactly `{kind: "candidate-finalization",
  candidate: ObjectRef, responseBytes, commit: CommitRef}`. The candidate is an
  exact retained candidate; responseBytes are #744's accepted/rejected finalization
  receipt. Accepted result must resolve exactly; rejected result is null with
  its bounded reason. This internal outcome uses the candidate's one reserved
  outcome slot, not an invented reporter action/idempotency key/request digest.

Rate/authentication before lookup and exact mutation receipt before mutable CAS
are unchanged. A child and workspace-UI key can never alias. Research-report
receipts and #743 replay/alias rows need their own closed registry/ledger shapes;
they cannot consume these #744 slots or be coerced into this union.

Receipt purpose is `candidate-admission`, `candidate-outcome`, `user-evaluation`,
`user-invalidation`, `child-evaluation` or `child-invalidation`. Propose the
following fixed slot mapping, outside all source digests. Ordinals never reset
across restart, token change or namespace rotation; this is a proposed storage
mapping of accepted quotas, not an additional allocation allowance.

| Ordinal / range | Purpose, exact owner and transition |
| --- | --- |
| Candidate ordinal c=0..31: slot 10c | Admission; reserve and consume with that candidate's mutation receipt in one transaction. Ordinal is monotonic assignment admission order, never recycled while subject history remains |
| Slot 10c+1 | Candidate outcome; reserve at admission, consume only with its one accepted/rejected finalization receipt. Superseded/abandoned without outcome retains this reserved slot with admission history; no manufactured finalization receipt |
| Slots 10c+2+2u and 10c+3+2u, u=0..3 | Four user writes and four paired invalidations. Initially reserved to candidate with null result/stream; accepted finalization transfers all eight to the exact result without recharging. A write binds its stream; initial stream creation earmarks its paired invalidation slot. A correction consumes another write but its extra invalidation slot remains reserved/unspendable; the stream uses its first-write paired invalidation once |
| Child ordinal q=12(resultRevision−1)+j, j=0..11: slots 320+2q and 321+2q | Twelve child evaluation revisions per result across all streams, numbered in admission order. Reserve the mutation/invalidation pair atomically in the exact target result; consume write with that revision's receipt. Bind both to its exact reviewer stream. Invalidation consumes only the stream's first-revision pair; correction pairs stay protected unused |

All 1,088 ordinals are 0..1087; no aliases or a second binding per slot. A slot
is `consumed` iff its receipt is nonnull; it is `reserved` otherwise. Candidate-admission slots always have null result/stream. Candidate-outcome
slots always have null stream, and have the exact result only for an accepted
outcome; otherwise result is null. Evaluation/invalidation slots use the exact
result and stream as described above. Issued streams cannot be rebound. Reserved child rows are
created with the evaluation transaction, not by an uncommitted request.

A terminal rejected/superseded/abandoned candidate that can no longer publish
releases its eight unused pre-result user credits by removing those rows and
reducing all three owner ledgers atomically. Keep its admission/outcome slots
and candidate ordinal; removed credits cannot fund a new ordinal beyond the
accepted 32-candidate cap. Accepted-result user credits never release merely
because the run ends or a stream is invalidated. Safe subject deletion removes
all remaining slots atomically with source/dependency/proof deletion, following
the existing retention gate. No other release/reuse path is admitted.

The entire slot/key/index/response must fit 16 KiB. The assignment/owner-run/
workspace ledgers charge the full slot for reserved and consumed rows; finalization
transfer or consumption does not charge again. This mapping provides no guarantee
of physical writability or permission to execute any candidate work.

### Source registry and remaining schema work

| Kind / exact identity | Body and digest authority |
| --- | --- |
| `assignment-result-candidate` / `(AssignmentIdentity, candidateDigest)` | Complete #744 `AssignmentResultCandidate`; candidateDigest is its source-domain digest |
| `assignment-result` / `(AssignmentIdentity, resultRevision, resultDigest)` | Complete #744 `AssignmentResult` |
| `assignment-evaluation` / full #744 EvaluationReference | Complete #744 `AssignmentEvaluation`; body reconstructs every reference field |
| `assignment-disposition` / `(EvaluationReference, dispositionRevision)` | Complete #744 disposition; resultDigest comes from the exact joined result, not an invented source field |
| `assignment-evidence-source` / complete #744 EvidenceSource identity | Complete source, with its existing source-key domain; referenced retained bytes must match |
| `research-report` / full #744 ReportIdentity | Exact #742 immutable descriptor, author/content digest rules and head mapping; no local replacement report format |

The dependency table also needs configuration/plan/proposal consumers, and #741,
#742 coordination and #743 evidence need strict additional registry entries.
Their current logical/illustrative records do not by themselves supply all
storage row/receipt/aggregate schemas. **These kinds remain inadmissible until
their owners provide closed fields, keys, domain fixtures and complete quotas.**
Do not use generic objects to smuggle an unreviewed plan/event through storage.
In particular, #743 aggregate/correction-target/tombstone/alias/high-water shapes
and #742 allocation/grant/recovery records remain explicit schema prerequisites.
This document does not claim that the whole registry or executable DDL is ready.

### Commit commitments without a digest cycle

Propose `ChangeManifest` with exactly `schemaVersion: 1`, `workspaceId`,
`storageEpoch`, `commitSequence`, `predecessor: CommitRef or null`,
`commandKind`, and `changes`. `commandKind` is `initialize-store`,
`candidate-admission`, `candidate-work`, `candidate-supersession`,
`candidate-abandonment`, `result-finalization`, `evaluation-write`,
`evaluation-invalidation`, `report-publication`, `source-health`,
`summary-reconciliation` or
`subject-purge`; future producer commands require closed reviewed additions.
Each change is exactly `{table, key, beforeDigest: Digest or null,
afterDigest: Digest or null}`; at least one digest is nonnull. Sort uniquely by
UTF-8 table name then canonical key bytes. A deleted row has null afterDigest;
an inserted row has null beforeDigest. Hash the exact manifest with the existing
`orkworks.hierarchy-storage-change.v1` + LF domain.

The row commitment is SHA-256 of
`orkworks.hierarchy-storage-row.v1` + LF + canonical `{table, key, body}`.
Here body is the exact listed remaining columns after excluding only these
storage-link paths: `objects.lastCommit`, `payloads.lastCommit`, `heads.lastCommit`,
`dependencies.lastCommit`, `summary_state.lastCommit`,
`summary_state.checkpoint.applied`, `pending_material.origin`,
`receipt_slots.lastCommit`, `receipt_slots.receipt.commit`,
`candidate_work.lastCommit` and `quota_usage.lastCommit`. Omit an excluded path,
never insert null in its place. Keep keys and every other field, including all
source-body properties, intact. For payloads only, omit the bytes column from the commitment body and retain
sizeBytes/contentDigest in the same row envelope; the actual bytes must verify
that raw digest. Payload lastCommit is excluded by the rule above. Thus changing metadata/ownership changes the row
commitment even when content bytes are reused.

Exclude header/latest and the new manifest's own commit_proof row from changes;
validate them by the equations below. Deleting an older unreferenced proof is a
listed change whose body is that older proof's full manifest/digest. This avoids
receipt→commit→receipt recursion without omitting source or ownership fields.
A row's link is assigned only on a body/key change, never through link-only
churn; exact retries leave links/proofs unchanged. Listed replacements must
change the body commitment; insert/delete uses the explicit null counterpart.
A non-initialization command has a nonempty changes list, covering every changed
logical row except its header/new-proof control effects. Missing/extra/duplicate
changes or an unexplained row-link rewrite fails validation.

Initialization creates header.latest and proof sequence 1 together, with null
predecessor, under explicit new-store preparation only. Every subsequent commit
requires its predecessor to equal the pre-transaction header.latest exactly and
its checked sequence to be predecessor+1; insert the new proof (key must be
absent) and replace header.latest atomically. Header.latest must resolve to one
proof with identical workspace/epoch/sequence/digest and a valid manifest hash.
No retained proof may have a future sequence greater than header.latest. Every
retained row/receipt/checkpoint link must resolve to its exact retained proof;
newly changed rows link to this command. A purged subject cannot retain orphaned
links, and a retained historical row cannot lose its proof.

On ambiguous recovery, use the recovered header and exact receipts/proofs to
establish complete old/new state; never elect a header from maximum sequence or
recreate it from remaining rows. Missing header/proof, wrong epoch/digest,
invalid transition or broken required link means unavailable. Legitimately
removed unreferenced historical proofs do not imply a corrupt sequence gap;
only the predecessor at admission must be present as the current latest proof.
This is integrity checking, not authenticity, an independent application redo
journal or protection from same-user whole-store rollback. SQL recovery must
still establish complete old/new state; a valid hash alone is insufficient.

All proof rows, links and lookup-index bytes count in budgeting. Protect proofs
needed by retained history; no receipt or lineage loses its proof to make room.
A proof-retention scheme requiring unbounded chain replay is not eligible.
Only the latest predecessor reference is required by a new manifest; this does
not require retaining every historical predecessor proof after its last owner
is safely deleted. No recovery reconstructs forgotten acceptance from hashes.

## Canonical byte and transition fixtures

These are proposed **written fixtures**, not executed serializer/SQL tests.
Local Python SHA-256 calculation checked the listed ASCII bytes; future Rust
and Electron implementations must reproduce them with their actual serializer.
No example creates a real grant, workspace or store.

Counter fixture: raw ASCII `18446744073709551615` is 20 bytes, SHA-256
`2cdb26265b4dc65e3b44d694f121fd6de99b9e4b8ae7f08d84bfa9537635ae43`.
`0` is valid only where zero is permitted; `00`, `+1`, `1.0`, `1e0`, surrounding
whitespace and `18446744073709551616` are invalid counter TEXT. A JSON source
u64 remains a lossless numeric token; the same quoted string is invalid there.

An initialization-only change manifest has these exact 205 UTF-8 bytes, without
a trailing newline (initialization has no domain source changes):

```json
{"changes":[],"commandKind":"initialize-store","commitSequence":"1","predecessor":null,"schemaVersion":1,"storageEpoch":"0000000000000000000000000000000000000000000000000000000000000000","workspaceId":"w"}
```

SHA-256 over `orkworks.hierarchy-storage-change.v1` + one LF + those bytes is
`3402eea84d0dd2f70ee9c709735d44027595580bc70d72a7807ad84e5d2d35dc`.
The all-zero epoch is synthetic only; real explicit UI creation requires a new
random epoch and qualified durable initialization. A second null predecessor,
changed epoch, unknown field, duplicate key or noncanonical stored body fails.

| Fixture | Required oracle |
| --- | --- |
| Reorder valid input object properties | Same canonical bytes/digest after strict parsing; stored noncanonical bytes fail reload rather than silently changing commitment |
| Same key/digest, different canonical bytes | Reject/fence; no hash-only identity or material comparison |
| Receipt with its own CommitRef attached | Manifest commitment is unchanged by attachment, but changing a source property changes its digest; wrong proof link or link-only rewrite fails validation |
| Report correction and evaluation still citing old report | New report/head/receipt and all stale dependencies/summary transitions commit together; retained evaluation bytes stay unchanged |
| Material A→B→A while unavailable | Two ordered pending rows/revisions; same final bytes cannot collapse them. Missing middle proof keeps unavailable |
| Reserved user credit, candidate finalizes | Transfer existing eight credits to that exact result without another charge; child writes cannot spend them |
| Commit response lost | Resolve original source receipt under current authentication before CAS; no new proof, candidate, revision or material transition |
| Corrupt ledger, dangling payload/head/dependency or missing proof | Unavailable, not inferred quota/head/acceptance from surviving rows |

Complete variant/nesting/null/order/max-integer byte fixtures for the inherited
#741/#742/#743/#744 source records remain required before executable handoff.
The two small vectors above do not qualify those serializers or native storage.

## Complete-command accounting and payload placement

Propose inline retained payload BLOBs in this selected SQLite store for this
slice. Copy/hash work remains bounded outside core locks; newly retained bytes
cannot be written after acceptance or split into another unprotected transaction.
No external payload store or custom VFS is implied. Candidate inspection still
requires the accepted durable conservative rehash ledger; this document supplies
no worker/scratch enforcement mechanism and starts no copy/hash operation.

Before SQL, build one bounded immutable command description from authoritative
source state; then revalidate under the eventual qualified ordering. Charge:

`commandBytes = newPayloadBytes + canonicalObjectBytes + headAndIndexBytes +
receiptAndLookupBytes + dependencyBytes + summaryRowBytes + checkpointAndPendingBytes
+ quotaAndCandidateLedgerBytes + commitProofAndControlBytes`.

Count every newly written/replaced logical value and key/link once per physical
logical row, including duplicate canonical bytes deliberately retained in another
row. Count both before/after material retained in pending proof, old checkpoint
bytes newly copied, all retained request-key bindings and new manifest metadata.
Existing unchanged payloads contribute new reference metadata only; source
occurrence quotas still apply without a physical-reuse discount. Deletion counts
its manifest/tombstone/counter effects, not negative credit against the command.
Conservative full-slot charging is permitted and must not undercount receipt
metadata. This is a logical admission byte bound, separate from SQLite page,
index, journal, auxiliary-file, allocator and OS-I/O qualification.

Recompute the complete affected set and changed logical rows before commit.
Changed-row accounting includes source/head/payload/dependency/summary/lineage/
slot/quota/proof/header rows, including deletions. No command may claim all 4,096
summary rows plus additional source rows fit a 4,096 changed-row cap. A command
with too many affected rows or insufficient durable unavailable proof is denied;
it cannot publish a partial correction, queue split transactions, or drop old
passing protection. All numbers below use proposed storage defaults, not newly
accepted capacities. MiB means 1,048,576 bytes.

| Synthetic publication budget | Consequence under 16 MiB complete-command cap |
| --- | --- |
| Two newly retained 8 MiB path snapshots | Exactly 16,777,216 payload bytes; any nonempty object/receipt/metadata makes the command ineligible |
| 32 new 512 KiB artifacts | Same 16 MiB payload-only total; source legality does not establish publication eligibility |
| One 8 MiB payload + 512 KiB object + 16 KiB receipt allowance + 2 MiB summary + 4 MiB pending before/after material | 15,220,736 bytes before all remaining heads/keys/edges/ledgers/proof/control; only 1,556,480 bytes remain. Admit only if exact remaining effects fit and all independent quotas pass; this is not a certified profile |
| Eight changed 2 MiB summary rows | 16 MiB before source/receipt/proof; ineligible. Seven such rows leave 2 MiB for every remaining effect, without guaranteeing fit |
| Full logical receipt pool, owed finalization/user mutation | Reuse protected slots; no new reservation check. Physical space/command/proof failure still fails closed, preserving the accepted distinction between capacity and writability |
| Unavailable summary needs old 2 MiB checkpoint plus new 2 MiB before and 2 MiB after material | At least 6 MiB lineage if all three are retained; count the retained row separately where it duplicates bytes. The 64 MiB workspace lineage cap is shared, not per subject |

No eligible maximum profile is established here. A future profile must supply
exact maximum lengths and counts for **every** term, domain quota and changed row,
including fan-out, canonical key escaping and proof retention. Missing #741/#742/
#743 row shapes prevent that proof today. Lowering source output limits or
raising storage limits requires a separate explicit reviewed choice; this pass
does neither and does not promise all independently maximal inputs can coexist.

## Authority and commit ordering: explicit no-go

The [accepted reporting cutoff](2026-10-04-assignment-evaluation-design.md#authentication-exact-retry-and-atomic-writes)
requires revocation first to deny persistence; commit first leaves the complete
record/receipt durable. The storage proposal places linearization at database
commit and forbids ordinary stop/end waiting for SQL/fsync or holding ordinary
core locks across it. A check before commit does not jointly establish these.

Concrete counterexample, including a last-moment volatile check:

1. Worker validates the still-current runtime/lease/namespace and starts or is
   about to finish SQL commit; its last authority read sees active.
2. Ordinary end/revocation changes that authority and returns without waiting
   for the worker. An immediate health fence blocks hierarchy delivery.
3. The already active SQL path durably commits the new record/receipt afterward.

The fence hides the result but does not undo persistence. A post-commit flag
check, timeout, interrupt request, replacement worker, compensating deletion or
receipt suppression cannot prove the accepted revocation-before-write denial.
A final callback still has a gap unless the actual durable commit and revoke
share an enforceable ordering. No such native ordering is demonstrated here.
This is a rejection of the unqualified precheck design, not a proof that every
possible future adapter is impossible.

Keep queued and active child writes no-go until a concrete design closes the
race. Do not silently reinterpret stop as delayed revocation, queue revocation
behind stuck I/O, grant already-admitted commands irrevocable authority, or move
acceptance to a later volatile publication CAS. Each changes an accepted or
proposed contract and needs its own explicit design review/owner decision.
Read response commitment remains the existing short in-memory cutoff; already
committed response bytes may arrive later, which grants no persistence exemption.

Required ordering packet: exact authority owner and linearization point for run
end, runtime end, lease loss, namespace rotation and sidecar replacement; every
lock/atomic/SQL/flush acquisition; failure and crash outcomes; and executable
races placing revocation immediately before/after each irreversible boundary.
Prove ordinary control latency is independent of a stuck storage worker and
that each committed new source was authorized at the accepted cutoff. Current
UI authentication and internal finalization authority also need their own exact
mapping; user authority cannot bypass namespace/ownership or source checks.
No implementation plan may claim this packet is supplied by the inventory.

## Handoff and review boundary

Delivered: a source-traced baseline of ordinary callers/bypasses and proposed
producer routes; proposed storage row envelopes and closed #744 receipt/commit metadata; two byte/hash
vectors and transition oracles; inline-payload and whole-command arithmetic;
and a concrete authority/commit counterexample. These clarify the detailed
proposal but do not make it executable or qualify its limits.

Still required before an executable plan: complete domain registry/record/ledger
schemas and cross-language vectors; implementation-base writer re-audit plus
installed/external producer eligibility; exact worst-case profile/row accounting;
a qualified revocation/commit ordering; effective SQLite feature/API/query and
allocator/VFS/auxiliary-file enforcement; native clock/durability evidence and
separately approved qualification scope. Native/model execution and runtime
code remain unauthorized. #745 shared custody, #744/#746 producer/projection
agreement and broader component choices retain their independent gates.

The next design decision is the authority/commit ordering, before selecting an
executable storage adapter or pretending the proposed numerical limits are
proven. Keep #744/#746 open. Accepting these written rows or merging this proposal
cannot waive any no-go or authorize runtime work.
