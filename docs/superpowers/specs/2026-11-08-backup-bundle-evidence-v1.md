---
type: Contract Proposal
title: Backup bundle and owner evidence v1
description: Exact logical bundle bytes, acyclic seals and purpose-bound owner records for conditional complete backup qualification.
tags: [orkworks, sessions, backup, codec, ownership]
status: draft
workflow_status: representation-proposed
---

# Backup bundle and owner evidence v1

Tracking: [#1525](https://github.com/Rambolarsen/orkworks/issues/1525),
[#1250](https://github.com/Rambolarsen/orkworks/issues/1250).
Consumes the accepted [backup/restore contract](2026-11-07-session-backup-restore-contract.md),
[storage requirements](../../../specs/session-storage.md) and unchanged
[adoption inventory v1](2026-11-07-legacy-adoption-inventory-approval.md).
This **proposed logical representation** does not qualify a native profile,
backup producer, validator, resource fit, restore protocol or product feature.
No serializer/parser, SQL/native/model execution or live capture is supplied.

## 1. Exact bytes and fixed layout

Reuse adoption v1's exact `C`, `H`, `U`, `X`, `D`, `N`, `Attempt`, `Profile`,
`Native`, `Path` and `Session` definitions by reference. `C` permits only closed
arrays, unescaped ASCII strings and null, with no whitespace/BOM/trailing bytes;
`U` is a checked canonical u64 decimal string. `H(x)=SHA256(C(x))`.
`SHA(bytes)` hashes the exact specified bytes, including stream line feeds.
This new `owb.*.v1` namespace does not extend adoption's closed Receipt-kind enum.
Every array below has exactly the declared fields. Unknown tags/types/lengths,
alternate encodings, overflow and conflicting repeated identities reject.

The maintenance owner generates Attempt. The native owner exclusively creates
`backup-<attemptN>` under the captured disjoint backup parent, with no overwrite.
Its contents are exactly:

```text
intent.json
entries.ndjson
relations.ndjson
exclusions.ndjson
issues.ndjson
objects/<entryOrdinalU>.bin
evidence/<bodyDigestD>.json
manifest.json
seal.json
```

`objects/` and `evidence/` exist even when empty. Each JSON file is exactly C of
its record, without LF. Streams use exactly one LF byte after every C record,
including the end record. Ordinals/hex basenames use the exact atom spelling;
no leading zeros, alternative suffixes or platform aliases. Only included regular
files have objects. Directories/access metadata are represented in entries, not
materialized using source paths inside the bundle. There is no content dedup,
link sharing, compression, path traversal or archive extraction.

The generated bundle root is its final namespace from exclusive creation. It is
incomplete until qualified publication of seal.json last; consumers must never
infer completion from its name or directory presence. No bundle-root rename is
required by this representation. Exact exclusive/control publication and flush
semantics still require native qualification. Extra/missing entries, links or
foreign replacements in the bundle reject; partial attempts stay unavailable.

## 2. Intent, purpose and complete streams

```text
Intent = ["owb.intent.v1",Attempt,purpose,workspaceD,sourceRootNative,
          bundleRootNative,applicationProfile,schemaProfile,nativeProfile,Registry]
Registry = [[role,Profile],...]
```

`purpose` is exactly `recoverable-workspace`, `retained-evidence` or
`preservation-only`, with exactly the accepted consuming rights. Purpose is selected explicitly by
the owning operation before root construction/Intent and is immutable for that
Attempt. A semantically valid source may support more than one purpose, but the
operation selects one; there is no automatic strongest-class promotion, fallback
or downgrade after an error. Changing purpose requires a new Attempt, root, Q
and applicable confirmation. Retained-evidence's format eligibility means its
source/root/cutover eligibility for adoption, not recoverable-workspace semantic
validity; exclude-only malformed/missing cases below remain permitted.
Profiles come
from the actual closed maintenance registry, never a caller-selected plug-in.
Registry is sorted by role then C(Profile) byte order, duplicate-free, and lists
every actual producer needed for included/excluded namespaces. `role` is exactly
`maintenance`, `native`, `scope`, `sql`, `metadata`, `payload`, `related`,
`resource` or `backup`. One role can have multiple owner profiles; this does not
permit an owner to claim another owner's namespace. Registry has exactly one
maintenance, native, scope, resource and backup slot; metadata/payload/related
slots cover the actual durable namespaces, even when empty. There is at most one
SQL slot, required for recoverable SQL and absent otherwise. Profiles are unique
across slots. The maintenance/scope owners select the complete registered set
from actual source-owner contracts before capture; callers cannot supply a
subset. An unexpected format or namespace requiring another owner invalidates
Intent and requires a fresh Attempt/registry, never an appended owner.
All are unqualified until
their exact versions and guarantees are registered. Native locator/access proof
encodings remain profile-specific qualification inputs, not arbitrary trusted X.

`workspaceD` is the existing qualified logical workspace binding, not a hash of
a displayed pathname. Application/schema profiles bind the exact same-version
restore compatibility contract. The root constructor supplies bundleRootNative
before Intent is written. Its admission binds Attempt, roots, planned purpose
and profiles without H(Intent); it cannot depend on a not-yet-written Intent.
Live constructor evidence remains required and cannot be reconstructed from disk.

For each kind `entries`, `relations`, `exclusions`, `issues`, stream bytes are:

```text
C(["owb.stream.v1",kind,H(Intent)]) LF
C(row0) LF ... C(rowN-1) LF
C(["owb.end.v1",kind,rowCountU]) LF
Descriptor = ["owb.descriptor.v1",kind,rowCountU,totalBytesU,SHA(allStreamBytes)]
```

rowCountU is the actual row count; row ordinals start at zero and are contiguous. Digest
and total bytes include header and end. Empty streams retain both. Truncation,
extra rows, duplicate end/header, absent LF, wrong count/digest or trailing bytes
reject. Stream/hash processing is incremental; no all-history buffer is implied.
Descriptors occur exactly once in the fixed kind order above.

```text
Entry = ["owb.entry.v1",ordinalU,Path,kind,ownerProfile,sourceNative,
         copiedNativeOrNull,contentBytesU,contentDigestOrNull,Access]
Access = ["owb.access.v1",nativeProfile,modeX,ownershipX,aclX,attributesX]
Relation = ["owb.relation.v1",ordinalU,ownerProfile,Subject,targetPath,
            requirement,state,targetOrdinalOrNull,reason]
Exclusion = ["owb.exclusion.v1",ordinalU,Path,ownerProfile,reason]
Issue = ["owb.issue.v1",ordinalU,ownerProfile,Subject,code,disposition]
Subject = ["workspace"] | ["entry",entryOrdinalU] | ["session",Session]
          | ["module",Profile]
```

Entry kind is `file` or `directory`. Included files have exact byte count, D and
a qualified independent copied Native for objects/ordinal.bin. Both source and
copied identities bind actual different occurrences; identity does not transfer
on restore. Directories use exactly zero bytes and null content/copy fields;
this means no copied content by kind, never unknown access/identity. Access fields
are exact profile-defined canonical native bytes; empty means no value only where
the registered profile explicitly permits it. Unsupported ACL/identity/name or
link/reparse/special object yields Unknown and blocks, not an invented encoding.

Entries and exclusions are sorted separately by Path component-byte order,
shorter prefix first. Entry paths must be unique under the native profile's actual
equivalence; directory/file and case/alias collisions reject. Exclusion paths
are disjoint minimal prefixes: no included path is at/below one, and exclusions
cannot overlap. Source-root classification must cover all actual entries with
included entries or qualified excluded namespaces; no catch-all unknown filter.
Excluded namespace contents are not read or hashed. Owners must prove that whole
namespace is excluded before traversal; no post-copy secret redaction.

Exclusion reason is exactly `lock`, `runtime-credential`, `transient-relay`,
`process-handle`, `global-setting`, `native-store` or `repository-content`.
External stores are scope exclusions, not invitations to traverse outside the
source root; an exclusion Path names a namespace in the captured root only.
The scope body's separate external-exclusion list names those external classes.
Unknown/unreadable included entries block every purpose. No null digest or zero
bytes can stand in for a failed read.

Relation requirement is `required` or `optional`; state is `present` or `missing`.
Present requires targetOrdinal pointing to the exact Entry Path and valid owner
association; missing uses null ordinal and owner-proven absence at Q. Reason is
`recorded-reference`, `protection`, `cleanup`, `organization`, `orphan` or
`historical-availability`. Relations sort by C([ownerProfile,Subject,targetPath,requirement,reason]), with unique tuples and no contradictory
state/target. Every relevant source record/reference must be accounted for, not
just selected UI rows. A malformed raw record uses entry Subject, never a
fabricated Session. Whole-module relationships use module Subject.

Issue code is `malformed`, `missing-required`, `missing-optional`, `orphan`,
`unsupported`, `unresolved-work`, `corrupt` or `unknown`. Disposition is
`exclude-only`, `reported-absence`, `retained-evidence` or `block`. Issues sort by
C([ownerProfile,Subject,code,disposition]), unique. They are diagnostic facts,
not owner decisions or a bypass for incomplete physical capture. Disposition
retained-evidence means retaining raw facts within the fixed bundle purpose; it
does not change a preservation-only bundle into an adoption input. Exact supported
relationships/classifications must be generated and validated by their real
owners; byte-valid rows alone prove none of these semantics.

## 3. Closed evidence bodies and acyclic subjects

```text
Evidence = ["owb.evidence.v1",role,Profile,idX,SubjectBinding,Facts]
Ref = ["owb.ref.v1",role,Profile,idX,H(Evidence)]
SubjectBinding = ["intent",H(Intent)] | ["content",H(Content)]
Unknown = ["unknown",code]
```

`idX` is a nonempty exact owner-generated ID. Full Evidence is retained as
`evidence/H(Evidence).json`. Ref role/Profile/id must equal its body. Same
role/Profile/id with different bodies rejects (including across stages); an
owner generates distinct IDs for admission and validation. Unknown code is
`unsupported`, `unqualified`, `unavailable`, `stale`, `cancelled`, `malformed`,
`ownership-lost`, `incomplete`, `resource-unavailable` or `overflow`.
Unknown cannot replace a required Ref in an eligible manifest. Stored Evidence
is data, not a live lease, signature, confirmation or effect permission.

Intent-bound admission bodies are exactly these four roles:

```text
maintenance: ["owb.maintenance-admission.v1",qN,claimIdX,serialU,Modules]
Modules = [[Profile,revisionX,"drained"],...]
native: ["owb.native-admission.v1",sourceRootNative,bundleRootNative,
         constructorIdX,namespaceProofX]
scope: ["owb.scope-admission.v1",scopeContractD,externalExclusions]
resource: ["owb.resource-admission.v1",reservationIdX,Bounds]
Bounds = [[metric,U],...]
```

Modules are sorted by C(Profile), unique, complete under the maintenance registry
and include every writer/cleanup/relay owner. The actual live claim proves paused
scheduling, closure/drain, external quiescence and serialized cancellation; a
caller cannot create it by writing Modules. Proof/ID/revision X fields are
nonempty canonical bytes under that issuing profile. ExternalExclusions is a
sorted unique array drawn from `global-setting`, `native-store`,
`repository-content`; the complete accepted exclusions must be declared.

Bounds contains each metric exactly once in literal byte order: `callbacks`,
`control-bytes`, `entries`, `evidence`, `exclusions`, `issues`, `objects`,
`physical-bytes`, `relations`, `transient-bytes`. Count metrics bound the number of entries/objects/rows/evidence files and
callbacks (simultaneously admitted callback count); byte metrics bound complete
control, physical or transient bytes. Per-callback byte bounds belong to the
closed resource profile and are charged to transient bytes. Values are finite
checked bounds, not measurements of free space. The real resource owner must account for every
simultaneous source, backup, preservation, staging, object, auxiliary, partial
and scratch occurrence through settlement. Scope-dependent byte/path/per-record
and validator limits must also be fixed by the exact owner profiles. No numbers
are chosen here and missing limits/profile qualification block before allocation.

After files/streams are copied and revalidated, construct:

```text
Content = ["owb.content.v1",H(Intent),qN,sourceRootAccess,AdmissionRefs,
           Format,Descriptors,Totals]
AdmissionRefs = [maintenanceRef,nativeRef,scopeRef,resourceRef]
Format = ["legacy"] | ["sql",workspaceD,epochX,schemaU,importD,generationU,
                      markerEntryU,databaseEntryU]
         | ["unresolved",issueOrdinals]
Totals = ["owb.totals.v1",entryCountU,fileCountU,directoryCountU,
          relationCountU,exclusionCountU,issueCountU,objectContentBytesU]
```

sourceRootAccess is exactly Access for the captured source directory; native
validation must revalidate it along with every included entry. Bundle directories
retain private capture permissions, not copied live-source authority.
Every admission Ref binds H(Intent), same roots/purpose and qN where applicable.
Totals are recomputed from the actual streams; checked file+directory=entries,
object count=file count and byte sum equals actual object lengths. They do not
claim total physical fit. Format is actual source authority state. SQL fields
must match the qualified header/marker and copied objects, including the actual
generation. Recoverable legacy requires no cutover evidence, recoverable SQL
requires valid marker and complete closed image/auxiliary validation. Unresolved
is permitted only for preservation-only; issueOrdinals is nonempty, sorted,
unique and in range, never empty success. Retained-evidence must satisfy the
adoption owner's actual format eligibility without claiming semantic recovery.

Content-bound validation bodies are closed by role:

```text
maintenance: ["owb.maintenance-validation.v1",qN,claimIdX,serialU,Modules]
native: ["owb.native-validation.v1",sourceRootNative,bundleRootNative,
         treeProofX,accessProofX,copyReadbackProofX]
scope: ["owb.scope-validation.v1",scopeContractD,"complete",externalExclusions]
sql: ["owb.sql-validation.v1",Format,auxiliaryEntryOrdinals,
      sessionCountU,organizationCountU,deletionJobCountU,
      relationOrdinals,issueOrdinals,"closed","valid"]
metadata: ["owb.metadata-validation.v1",formatKind,recordCountU,
           entryOrdinals,relationOrdinals,issueOrdinals]
payload: ["owb.payload-validation.v1",entryOrdinals,relationOrdinals,issueOrdinals]
related: ["owb.related-validation.v1",entryOrdinals,relationOrdinals,issueOrdinals]
resource: ["owb.resource-validation.v1",reservationIdX,Bounds,"retained"]
backup: ["owb.backup-validation.v1",purpose,requiredRefs,"complete"]
```

Every ordinal array is sorted, unique and in range of its named stream. recordCount counts all source metadata records belonging to that profile,
including readable malformed records in retained-evidence. SQL sessionCount
includes tombstoned rows; organization/deletion-job counts include every stored
record, not a UI projection. Metadata
formatKind is `legacy`, `sql` or `unresolved` (the last only for preservation-only);
SQL validation is mandatory for recoverable SQL,
forbidden otherwise (damaged SQL can be preservation evidence, not a valid seal
from its SQL owner). Sql Format must equal Content Format. Auxiliary ordinals
must include every actual required SQL auxiliary, or the qualified SQL owner
must prove a self-contained closed image with empty auxiliary list. No DB-only
copy, quick_check or absence guess substitutes for this validation.

All required registered metadata/payload/related namespace owners validate their
complete records, relationships, revisions/protection, pending work and issues
under their closed profile. Recoverable-workspace cannot carry malformed/corrupt/
unknown required state or unresolved jobs without an already-qualified restorable
owner contract. Retained-evidence permits readable malformed independent records
only as exclude-only; required missing relationships require excluding affected
source-only metadata when consumed by adoption. Preservation-only can retain
semantic damage/unresolved work, never unreadable or incomplete physical state.
Any block disposition rejects eligibility. No ordinal list may hide unlisted
relevant records; owner completeness must cover its entire registered namespace.

Required evidence is mechanically derived: all four admission Refs, exactly one
Content-bound validation body for every Registry slot except backup, and exactly
one final backup body. Empty namespaces still require their registered owner's
validation with empty ordinal lists; an absent payload is not permission to omit
its owner. No additional profile can appear in a Ref. Every included file ordinal
must occur exactly once in its matching owner's metadata/payload/related entry
list, or SQL's database/marker/auxiliary union. Every Relation and Issue ordinal
must occur exactly once in its row's owning profile validation list. Native owns
all directory entries/access and the complete tree; scope owns all exclusions.
Every row ownerProfile must be registered. Exclusion owners remain their actual
namespace owners; the scope validator checks every excluded namespace.
Scope validates those total coverage partitions against actual owner namespace
contracts, not only against caller-authored rows. Cross-owner references are
relations, not duplicate file ownership. A gap, overlapping coverage, wrong role,
unknown namespace or an unqualified owner mapping rejects the bundle.

Maintenance validation must re-establish the same live Q/claim, module revisions
and current serial ordering; late mutation/owner loss rejects. Native validation
binds the complete tree/access and actual independent copied objects. These
profile-defined proof bytes are not a self-asserted portable native algorithm.
Resource validation must match the original reservation/bounds and actual held
resources. Backup requiredRefs is sorted by role/C(Profile)/idX and contains
all admission and content-validation Refs except backup's own Ref, with no
missing/extra owner, duplicate or equivocation. Backup validates every full body,
class-specific semantics and complete graph/byte accounting before issuing it.
No backup body binds Manifest or Seal; doing so would make a digest cycle.

Manifest and seal are:

```text
Manifest = ["owb.manifest.v1","orkworks.workspace-backup.v1",Intent,Content,
            EvidenceRefs]
Seal = ["owb.seal.v1",H(Intent),H(Content),H(Manifest),manifestByteCountU]
```

EvidenceRefs is the sorted unique exact union of admission/validation/backup
Refs; each listed full body must exist, and evidence/ contains no unlisted file.
Required owners are derived from Registry, scope and source format, never caller
selection. Exactly one final backup validation Ref is required. No evidence may
bind H(Manifest) or H(Seal) from inside this graph. `intent.json` must equal the
Manifest Intent bytes, each stream its Content descriptor, and Seal hashes must
match actual bodies. ManifestByteCount is exactly the length of C(Manifest).
Manifest/seal files do not list their own native identity or digest in themselves.

Seal publication follows durable objects, streams, evidence and manifest plus
qualified parent/directory flush. That publication's acknowledgement is outside
Seal; the seal cannot certify its own successful flush. A consumer needs current
qualified native validation of the complete bundle plus actual owner semantics,
not merely syntactically valid Seal bytes. Data may survive response loss, but
no stored record revives lost capture/restore authority. Resource admission must
include manifest/seal full bytes, evidence/stream headers/ends, all directory
entries and partial objects as well as payload bytes before their allocations.
Count actual complete control bytes externally; no self-referential total field
is used to infer fit. Failure/cancel/unknown settlement never fabricates Seal.

## 4. Inert consumption and rejection oracles

Validation never calls normal recovering store opens, performs journal repair,
schedules work or invokes native tools. Validate exact control/layout bytes,
complete streams, object read-back/access, all body references/profiles and
class-specific semantics. A valid data bundle grants no new effect admission.
Only recoverable-workspace can enter a separately approved workspace restore.
Retained-evidence remains per-candidate source evidence; preservation-only remains
raw current-state preservation. The accepted restore fence/protocol is a separate
qualification, not implemented by manifest parsing or by the Seal.

For adoption, the actual backup owner binds the retained-evidence source and
recoverable legacy global bundle to the exact unchanged v1 Snapshot inventories,
roots and current publication Intent/Plan. It must validate both complete sets
and current restore/evidence-capture validity. The Ref above is not a substitute
for an original v1 backup Receipt or publication backup-current body. No extra
Receipt kind or reconstructed approval handle is introduced.

| Input/change | Required result |
| --- | --- |
| Alternate JSON, leading-zero ordinal, wrong field count/tag/version | Reject exact encoding before semantic success. |
| Header/end LF removed, reordered rows, truncation, mismatched count or descriptor | Reject whole stream/set, no partial inventory. |
| Extra or missing control/evidence/object file, alias, link or changed native identity | Reject; matching content hash grants no ownership. |
| Intent purpose changed or retained-evidence/preservation offered to workspace restore | Hash/subject or purpose rejection; no class substitution. |
| Malformed independent local bytes, complete capture, exclude-only issue | Retained-evidence may validate capture; never selected metadata or recovery-workspace eligibility. |
| Required local content missing; source-only metadata excluded | Retained-evidence can preserve/report absence; adopting affected metadata rejects. |
| Global corruption, required protection missing, incomplete or unreadable capture | No recoverable global backup and no adopted batch; exclude cannot bypass it. |
| Unknown profile/proof, stale Q, missing module/evidence, conflicting ID/body | Stop/unavailable, never known by digest. |
| SQL marker/header/generation mismatch or unknown auxiliary closure | Reject recoverable SQL, no marker/DB-only fallback. |
| Resource overflow/missing bound or released reservation | Reject admission/completion; report cap/free space is not fit. |
| Partial seal/publication, response loss, restart, cancellation | Truthful stopped/unknown until full actual data/native validation; never reconstruct claim or auto-delete. |
| Backup validation references itself or Manifest/Seal; admission references Content | Reject cyclic/wrong-stage subject; no repair by dropping evidence. |

## 5. Written component evidence and remaining gates

The following examples are codec components only. All-zero digest bytes are
literal test input, not qualified roots/evidence. Even a correctly encoded seal
with these fields is ineligible without its full valid bundle and real producers.
They establish no known receipt, executable validator or physical result.

In these literal examples, replace `Z` with 64 `0` characters, `O` with 64 `1`
characters and `T` with 64 `2` characters **inside the quoted field**, without
adding escapes or whitespace. Code-block display newlines are LF. The seal has
no final LF. These are component vectors, not a valid manifest or source snapshot.

```text
["owb.stream.v1","entries","Z"]
["owb.end.v1","entries","0"]
```

```text
["owb.stream.v1","issues","Z"]
["owb.issue.v1","0",["66","1","Z"],["entry","0"],"malformed","exclude-only"]
["owb.end.v1","issues","1"]
```

```text
["owb.seal.v1","Z","O","T","0"]
```

| Expanded exact bytes | Byte count | SHA(bytes) |
| --- | --- | --- |
| Empty entries stream, two trailing line feeds total | 124 | `7b2b77da36aa5e010c364f15ba028d6b61f9320850cee4ad20f219b00ff007b1` |
| One malformed issue stream, three trailing line feeds total | 262 | `bf6af5aabe6c2adb89d69fd8bfbc671d8c04a9e1cfcc51add9a15cf088c45ce3` |
| Seal component, no final LF | 220 | `719447b995c6deb605c259859fa239dfb2af7f92add02c5e58fac7c4d98b5d3c` |

Python standard-library literal-byte hashing and an independent Node JSON
parse/re-encode plus SHA check agreed on every byte count/digest. No product codec or native/SQL operation was executed. The issue's fixture
profile is unqualified and its referenced entry is absent, so it cannot be
consumed as a complete valid bundle. The seal's manifest byte count of zero is
also semantically impossible for a valid Manifest; exact syntax/hash does not
make it eligible. Deleting the final stream LF changes both byte count and
digest and must reject even if a line-oriented parser still reads its rows.

Before dependent code, obtain owner acceptance of this representation, qualified
native/access proof encodings and constructor/current validators, complete scope/
association/history/protection validators, inert SQL closure/integrity validation,
maintenance/application confirmation and resource enforcement with finite limits.
Actual native execution and copied/synthetic recovery tests require their own
bounded authorization. The accepted policy/status does not supply these missing
inputs; #1250/#1244 and existing producer/native/hierarchy/rollout gates remain.

## Review disposition

Fresh authority review found no material conflicts with accepted purpose,
marker/restore or inert-authority rules. Fresh completeness review identified
ambiguity in purpose selection and deriving required owner records. Clarified
pre-capture immutable purpose with no automatic promotion/fallback, fixed registry
role cardinalities, all-owner validation including empty namespaces, and total
file/relation/issue coverage partitions checked by the actual scope owner. No
physical profile or runtime producer is qualified by these written corrections.
