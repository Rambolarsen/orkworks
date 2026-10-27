---
type: Design Proposal
title: Root preparation source codecs and initial membership
description: Proposed exact root assignment, executable and provenance records plus a bounded initial run and parent attachment slice for source-byte qualification.
tags: [orkworks, hierarchy, preparation, codecs, approval]
status: draft
---

# Root preparation source codecs and initial membership

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742); initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Inspection base: `273a02f184076c6b607c8eede03f9b8cb9ea281f`.
**Scoped source codecs accepted for documentation component fixtures on 2026-10-27.
No runtime implementation, source-valid full bootstrap, migration or eligible
coding-tool profile is claimed.**

## Intent and boundary

Close the five source gaps identified by the
[component-byte audit](../../validation/root-preparation-components.md#source-closure-findings)
without filling them with arbitrary objects. This extends the accepted
[identity-first draft/source flow](2026-10-25-root-preparation-producer-design.md#owner-acceptance-and-handoff--2026-10-25).
The author chooses the field names, initial shapes and limits below for review;
the owner's instruction to take this task does not approve those new choices.

Deliver one documentation proposal with exact variants, joins and rejection
oracles. After acceptance, the next unit is documentation-only component byte
fixtures/checkers. Complete bootstrap/source/transition fixtures still require
closure of every selected nested permission/evidence record and every affected
storage row. This document deliberately closes only the initial membership
slice, not the entire mutable orchestration lifecycle or command manifest.
Ordinary sessions and child configuration codecs stay unchanged.

Alternatives considered: an extensible JSON object leaves canonical bytes to
individual writers; reusing a child assignment invents future plan/task identity;
a general lifecycle migration expands this bounded source task. Choose separate
root-specific records and fail closed outside the stated slice. No new endpoint,
bearer, database table or native probe is supplied here.

## Investigated uncertainty

The least certain boundary was whether admission already implied a running
parent. The accepted producer's atomic-effects section requires initial run
membership before runtime/grant delivery, while #742's creation table names
assessing. Represent that combination explicitly with a reserved attachment
and null runtime; do not invent an already-started parent. Another likely
failure is calling closed component bytes complete source validity: the audit
also identifies unclosed selected permission/evidence entries and complete
changed-row effects. Keep those qualifications separate and visible rather
than quietly filling them with empty values. Existing recovery rules already
cover burned reservations and stale review contexts; preserve them here.

## Common encoding and compatibility

Use [#741 canonicalization](2026-10-04-agent-role-configuration-design.md#canonicalization):
strict UTF-8, no unpaired surrogates, duplicate/unknown keys or unsupported tags,
recursively UTF-8-key-sorted compact JSON, exact integer encodings, preserved
string bytes and declared array order. Reject rather than normalize input into
an accepted stored object. IDs follow #741's 1–128 byte ASCII alphabet; Digest
means exactly 64 lowercase hexadecimal SHA-256 characters. All object properties
listed here are required; null is legal only where explicitly stated. Counts
and byte ceilings are inclusive and checked before persistence. Existing smaller
adapter/source/containing-object bounds still win.

Reuse the accepted [PathIdentity](2026-10-04-configuration-learning-design.md#logical-records-and-digest)
exactly: `{encoding,base64}`, with canonical absolute native path units,
standard padded base64 and platform-consistent encoding. Native canonicality and
object validity are resolver/evidence obligations; a synthetic string cannot
qualify them. Decoded path storage is at most the existing 2 KiB path ceiling;
base64 and JSON overhead also count against containing descriptor limits.
This does not redefine existing repository/worktree descriptors.

These records are a proposed codec amendment to previously unclosed nested
fields of output bootstrap v2. Keep its existing field set, prefix and digest
rule; do not reinterpret any previously hashed seed as conforming. Legacy v1
and synthetic v2 examples remain historical/unverified. If a retained producer
already used an incompatible nested shape, reject it for this slice; any actual
migration or broader wire-version decision needs its own reviewed plan.

## Root assignment

For bootstrap v2 only, `assignment` is exactly RootAssignmentV1:

```text
{schemaVersion:1,label,description,inputs,expectedOutput,criteria,
 dependencies,worktreeScope}
```

label is nonempty UTF-8 through 512 bytes; description is nonempty through
4 KiB; expectedOutput is nonempty explanatory UTF-8 through 2 KiB. Preserve it
as distinct from bootstrap.outputContract, which retains its accepted structured
output declarations and caps. Neither text can supply an undeclared output or
permission. criteria is the existing ordered array of 1–32 CriterionSnapshot
objects with unique IDs and at least one required criterion; descriptions are
nonempty through 2 KiB. Preserve input and criterion order for rendering/digests.

inputs has 0–32 entries with unique input IDs, each exactly one of:

```text
{kind:"inline",id,content,contentDigest}
{kind:"reference",id,sourceIdentity,sourceRevision,reference,contentDigest}
```

Inline content is exact nonempty UTF-8 through 4 KiB; contentDigest is its raw
SHA-256. A reference retains an already reviewed source locator through 2 KiB,
nonempty sourceIdentity/sourceRevision through 512 bytes each, and a Digest of
the exact independently retained content. It is neither an instruction resource
nor a URL/path fetch request. Before approval, the producer must resolve that
exact locator/revision to retained bytes and verify the digest; missing bytes
make the complete source unavailable. The snapshot's owning existing custody
record and its bounds must be closed before nonempty reference fixtures can be
called source-valid. No new uncounted blob store is introduced. Reference text
never grants filesystem/network access or overrides binding rules.

For this root slice dependencies is exactly `[]`: the immutable run assignment
cannot depend on future plan/task identities. Later child dependencies remain
plan coordination records. No nullable child plan/task/worktree-group/base fields
are smuggled into this root object.

worktreeScope is exactly:

```text
{kind:"source-worktree",repositoryBindingDigest,sourceWorktreeBindingDigest,
 readPaths,writePaths}
```

Both digests must match the bootstrap's complete accepted descriptors. Paths
are source-root-relative nonempty UTF-8 through 2 KiB, slash separated, with no
empty, `.` or `..` component, backslash, NUL or absolute/drive prefix. Each list
has at most 64 distinct paths, sorted by UTF-8 bytes; no Unicode/case folding.
These lists express intended scope, not enforcement. They must fit the approved
requested/effective profile. An empty list means no path access, not all paths.
The source worktree is existing input identity, never a reserved child worktree.
Parent implementation writes remain forbidden by the role contract; declared
coordination outputs still require their separate exact allowed actions.

## Snapshot provenance

All selected snapshot `provenance` fields in this slice are exactly
SnapshotProvenanceV1, one of:

```text
{schemaVersion:1,kind:"local",sourceKind,sourceIdentity,sourceRevision,sourcePath}
{schemaVersion:1,kind:"nonlocal",sourceKind,sourceIdentity,sourceRevision}
```

sourceKind remains `repository`, `bundled` or `user-approved`. sourceIdentity
is the nonempty stable owning-catalog identity through 512 UTF-8 bytes;
sourceRevision is a nonempty exact revision string through 512 bytes. Neither
is a credential or a display label used as proof of identity. local sourcePath
is the accepted PathIdentity. nonlocal has no sourcePath property, including no
null placeholder. Repository file snapshots must be local; bundled and explicitly
approved catalog sources may use either variant if their source owner can prove
the identity/revision/content binding. Nonlocal does not authorize retrieval.

Keep RuleSnapshot.sourcePath's existing relative locator distinct from
provenance.sourcePath's resolved native absolute path. Producer revalidation must
prove they identify the same reviewed source under the named catalog/root;
matching display text is insufficient. Content and catalog snapshot hashes retain
their existing ownership; this provenance object is included in every containing
canonical descriptor that currently covers provenance. No standalone provenance
digest or mutable timestamp is added.

## Executable identity

The shared executableIdentity value in AdapterBinding and capability evidence
must be byte-identical and exactly one ExecutableIdentityV1 variant:

```text
{schemaVersion:1,kind:"direct",executable:ExecutableFileV1}
{schemaVersion:1,kind:"wrapped",executable:ExecutableFileV1,
 components:[ExecutableComponentV1,...]}
```

ExecutableFileV1 is exactly `{path,contentDigest}`; path is PathIdentity and
contentDigest is raw SHA-256 of the complete file bytes. executable is the
actual OS process image, not a command name discovered through PATH. A direct
variant is eligible only when that image is the complete tool binding and no
wrapper/interpreter/runtime package requires separate pinning.

ExecutableComponentV1 is exactly `{kind,path,contentDigest}`, kind being
`wrapper`, `interpreter`, `package-manifest` or `package-file`. Every component
pins complete file bytes. components has 1–64 entries in invocation/dependency
order, not sorted as a set; duplicate `(kind,path)` pairs are rejected. The
adapter evidence must establish this complete ordered chain, including all
resolved wrappers/interpreters and the manifest plus executable package content
when a package participates. A manifest digest alone cannot establish package
content identity. No directory digest, glob or unspecified package hash is
accepted in this file-only slice. A larger/unknown package closure is unsupported,
not truncated to 64 entries or treated as direct.

Each complete executable identity is at most 64 KiB within existing containing
limits; this proposed new nested ceiling does not increase the capability
snapshot's existing 64 KiB or bootstrap's 1 MiB limit. Path and content equality
are both required at launch/resume. The evidence must establish safe measurement
and revalidation of what actually executes; exact file hashes do not prove
permission enforcement, prevent same-user replacement races or qualify a native
platform. Arguments/settings remain in their existing reviewed binding, not
implicit additional fields here. No credential value is hashed or retained.

## Initial admitted run and reserved parent

The producer owns two complete logical values for the source-preparation commit.
Their storage table/keys, registry entries, row hashes, complete command effects
and quotas need separate closure before a storage command is executable. They
are not arbitrary new columns on accepted source rows.

InitialRunStateV1 is exactly:

```text
{schemaVersion:1,workspaceId,runId,parentSessionId,runDefinitionDigest,
 bootstrapConfigurationDigest,runVersion:1,state:"assessing",
 preparationRevision:1,inputRevision:1,inputDigest,planReferences:[],
 currentPlan:null,parentRuntime:null,blockers:[],stageContinueRequired:false,
 pendingClarification:null,pauseReason:null,resumeTarget:null}
```

All repeated identities/digests equal the one validated embedded run/bootstrap.
This is a closed initial variant, not permission to serialize later clarification,
blocker, plan or resume values using undefined nested objects. Their full codecs
remain open. Assessing is the existing creation state, but automatic assessment
requires a separately verified current parent runtime; this row starts none.

For this initial slice only, inputDigest is the existing
`orkworks.preparation-input.v1` + one LF domain over canonical InitialInputV1:

```text
{schemaVersion:1,workspaceId,runId,preparationRevision:1,inputRevision:1,
 goalDigest,bootstrapConfigurationDigest,decision:null,records:[]}
```

goalDigest equals the retained runDefinition.goalDigest field, the raw hash of
the exact reviewed goal text. No assessment decision, question,
report or plan exists before the parent acts. This proposed initial-manifest
variant closes the initial empty-input case only; it does not change the later
manifest's requirement to retain exact consumed decision/record identities.
A missing initial manifest cannot be reconstructed from a hash during recovery.
Retain its exact bytes with the source-owned initial state under the eventual
closed storage representation and charge all copies to its accounting.

ParentAttachmentV1 is exactly one of:

```text
{schemaVersion:1,kind:"reserved",workspaceId,runId,parentSessionId,
 rootAssignmentId,bootstrapConfigurationDigest}
{schemaVersion:1,kind:"attached",workspaceId,runId,parentSessionId,
 rootAssignmentId,bootstrapConfigurationDigest,sidecarGeneration,
 launchGeneration}
```

The reserved record is created with initial run/source admission; no generation
is present or null. An attached record has both generations as the existing
#742 opaque 64-lowercase-hex strings, proven by the session producer. Identity
comes from the exact frozen bootstrap and fresh reserved parent, never a session
search or inferred worktree match. Attachment is unique for the run/parent; the
same session cannot be attached to another root/run. No bearer is stored here.

A later attach operation requires current explicit UI startup authority, unchanged
eligible bootstrap/source/worktree evidence, and the existing exact native
session producer proof. It atomically replaces this run's reserved attachment
with attached, changes parentRuntime from null to exactly
`{sidecarGeneration,launchGeneration}`, and increments runVersion from 1 to 2
for this initial-only branch. It preserves input/definition/summary identities
and creates no plan grant. The session-creation record, publication ordering,
startup failure/recovery rows and the full changed-row set must be specified
before implementing this transition. An attachment record alone never starts a
PTY or proves delivery. Resume and repeated attachment use the existing lifecycle
requirements and remain outside this initial-only codec; do not overwrite an
attached predecessor as a shortcut.

## Recovery and written rejection oracles

| Case | Required outcome |
| --- | --- |
| Future child dependency or plan/task key in root assignment | Reject this root codec |
| Expected output text conflicts with structured output declaration | Block approval; neither silently wins |
| Duplicate input/criterion/path, unsorted path set or unknown property | Reject before hashing/persistence |
| Reference digest without retained source bytes | Component shape may be valid; complete source remains unavailable |
| Nonlocal provenance includes sourcePath, including null | Reject variant |
| Rule relative source locator disagrees with resolved provenance | Reject source binding |
| Wrapped CLI pins only a version/manifest, omits interpreter or required package files | Ineligible; no direct fallback |
| Hash-equivalent display paths with different native units | Compare exact PathIdentity bytes; no display round trip |
| Executable identity differs between adapter and capability snapshot | Reject exact binding even if toolVersion agrees |
| Initial state has a plan, runtime, blocker or answered question | Reject initial variant; no generic mutable-state fallback |
| Initial input has a decision or source digests differ | Reject; no invented assessment or hash repair |
| Only reserved attachment is committed | Source membership exists; runtime/planning/launch remain unavailable |
| Existing ordinary session offered as the reserved parent | Reject adoption |
| Crash after reservation but before committed source/run/membership | Preserve burned sequence; no source reconstruction |
| Attached record lacks matching session proof or complete atomic run effect | Unavailable; no credential or PTY delivery |
| Restart/revoked UI context with matching stored approval bytes | Existing current-authority-before-retry rule still denies |

The accepted original receipt identity, frozen-byte equality, review-context
revocation, durable fence/high-water, burned sequences and shared-ledger rules
remain binding. These written cases are not executed transition/native evidence.

## Review and handoff

Owner review should explicitly accept or revise: root-only empty dependencies;
input forms and bounds; tagged provenance using PathIdentity; the file-only
executable chain and its 64-entry/64 KiB bounds; and the initial null-decision
manifest, reserved/attached shapes and first-attachment version effect.

After acceptance, make byte/digest fixtures for these components with independent
expected values and rejection cases. Reconcile the owning #741/#742 sections and
audit the remaining selected nested permission/capability records before calling
any full bootstrap source-valid. Close the complete mutable lifecycle, session
producer proof and storage command membership/effects separately. No synthetic
source byte can upgrade #740 eligibility: all role profiles stay unverified/no-go
and capped capability work remains deferred until November. Native authority
ordering, resource/shared custody qualification, complete production quotas and
a separately approved executable plan remain independent gates. Keep #741/#742
and the broader component issues open.

## Owner acceptance and component handoff — 2026-10-27

The owner answered “yes, go ahead” after the concrete draft PR #1093 and its
schema-approval question were presented. This accepts the five source-codec
choices and the initial null-decision manifest/first-attachment slice for
component fixture/checker work. It does not approve a production wire migration,
complete mutable lifecycle/storage command, native execution or adapter eligibility.
The earlier proposed-status wording records the pre-acceptance design history.

The [component evidence](../../validation/root-preparation-source-components.md)
retains independent Python-derived bytes and structural rejection tests. These
are synthetic components only; the source-valid bootstrap, retained source rows,
atomic effects and capability/native gates named above remain open. Preserve the
original 2026-10-26 run/approval catalog and the older unverified bootstrap seed.
