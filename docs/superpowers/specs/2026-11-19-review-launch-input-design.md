---
type: Design
title: Ordinary plan-review proposal and prepared input
description: Candidate typed proposal, narrow reviewer configuration and complete logical launch-input commitment for ordinary review.
tags: [orkworks, review, proposals, admission]
status: draft
---

# Ordinary plan-review proposal and prepared input

**Proposed for owner acceptance, with the coupled offline plan.** The owner
requested the next prerequisite after [#1919](https://github.com/Rambolarsen/orkworks/pull/1919).
That accepts investigation/design preparation, not these new source/profile/byte
choices or runtime implementation. Track [#68](https://github.com/Rambolarsen/orkworks/issues/68),
[#66](https://github.com/Rambolarsen/orkworks/issues/66) and
[#1899](https://github.com/Rambolarsen/orkworks/issues/1899).

Reuse the accepted [preview binding](2026-11-19-review-handoff-binding-design.md)
and [source/admission contract](2026-11-19-review-source-qualification-design.md).
This closes a finite **logical value and commitment** design. Actual producer
custody, configuration/native eligibility, revision publication, storage and
Start integration remain qualification gates. No production capability is added.

## Source findings and selected approach

Audit base: `37bfcbaaaa820c91cdbd88b89e0d0cc8388659c0`.

| Source under `crates/orkworksd/src/` | Finding and design consequence |
| --- | --- |
| `taskmaster/mod.rs::RecommendationType`, `Recommendation` | Only improve_workflow/cleanup exist. Suggested strings and updated_at are not typed review intent or immutable proposal revision. Add neither launch interpretation nor authority to them. |
| `session_application.rs::CreateSessionCommand`, `resolve_session_launch` | Inputs include images, optional effort/model/tool/prompt; missing/unknown tool can fall back to generic-shell and absent model to its default. The new consumer must use a separately qualified exact prepared seam. |
| `harness/registry.rs::ResolvedHarness::build_launch`, `harness.rs::CommandSpec` | Program/ordered args/cwd are concrete values. Capture them rather than only a harness name; current CommandSpec has no environment field. |
| `harness/store.rs::HarnessDocumentRevision` | Hash of persisted override bytes, not an effective built-in/catalog/profile revision or mutation history. Bind a live configuration owner epoch as well. |
| `branch_documents.rs::DocumentKind`, `Document::selection_id` | Selected slot is design or implementation; selection_id omits kind. Bind the exact slot as a separate field, not the synthetic fixture label plan. |
| `runtime/session_runtime.rs::RuntimeIdentity` | Runtime instance and run generation already exist. Keep both with exact source session; title/branch are not identity. |

The [role configuration core](2026-10-04-agent-role-configuration-design.md)
is logically accepted but has no qualified ordinary-review launch producer.
Do not reuse an orchestrated root/child snapshot or its grant as ordinary Start
authority. Initial profile below is intentionally narrower; expansion requires
its own source/profile qualification, not a wider free-form JSON field.

Alternatives considered: hashing suggested strings misses source/configuration;
using the ordinary optional command permits fallback; importing a hierarchy
configuration brings independent owners/grants and unsupported runtime assumptions.
Choose a closed ordinary proposal and a complete narrow logical launch value.

## First proposal source and revision

Future internal recommendation discriminant maps the spec's
`start_review_session` to **ordinary_plan_review**. Existing variants retain
their schemas, accept actions, completion semantics and storage behavior.
This is a candidate value contract, not a migration or serde change.

First modeled source is a live, owned, active ordinary session. First modeled
evidence is one **explicit user request for plan review**, owned
by the authenticated recommendation producer. It is not inferred from Peon
completion text, a missing reviewer, process exit, or an instruction's wording.
Automatic review-ready and other #66 intents need separately qualified producers.
The future producer must bind the user's request to the selected exact source;
no new request UI or endpoint is implemented or authorized here.

Closed proposal fields, in canonical key order:

- `schemaVersion`: 1; `intent`: ordinary_plan_review;
- `recommendationId`: opaque ASCII `[A-Za-z0-9_-]{1,128}`;
- `proposalRevision`: positive canonical u64 decimal string;
- `sourceSessionId`: the same opaque ID domain;
- `sourceRuntime`: `{runtimeInstanceId, runGeneration}` (UUIDv4, positive u64);
- `request`: `{requestId, requestRevision, text}` (UUIDv4, positive u64,
  nonempty UTF-8 text at most 2 KiB);
- `chain`: `{chainId, depth}` (opaque ID, integer 0..2);
- `proposedDraft`: `{codingToolId, modelId, userInstruction}` in that order.

Draft tool ID uses the opaque domain; model ID is nonempty UTF-8 at most 256
bytes; full userInstruction is nonempty UTF-8 at most the accepted 16 KiB.
Proposal encoding is at most 24 KiB. No document content, images, provider
output, arbitrary cwd, runtime capability claim or child/run/grant ID is allowed.
A proposed draft is advice; it is never itself prepared launch configuration.

The proposal owner stores immutable revisions. A material request, source,
change/evidence, chain or proposed-draft change publishes a strictly newer
revision, including observed A→B→A. Unchanged reads do not increment it; wall
clock time/content hashes do not stand in for that owner history. Source changes
invalidate the old proposal for new inspection immediately; it remains unavailable
until the writer can publish its successor. Status reads never repair/publish it.
Revision exhaustion denies further use until a separately reviewed new identity;
no wrap, reset, or silent replacement. The actual publication/retention/compatibility
protocol is not established by a passing value codec.

Current eligible status is separately read from the recommendation owner and
must be proposed, with no target. It is not caller-controlled or a second mutable
status in this immutable proposal. Start lifecycle publication across owners
remains a runtime/storage prerequisite; neither inspection nor this digest
accepts/completes the recommendation. Retained-attempt replay uses #1919 before
preview/proposal expiry checks and never reconstructs a new launch from this value.

## Narrow effective configuration

Propose first profile **ordinary-plan-review-instruction-v1**:

- exact non-retired non-generic coding tool and explicit supported model;
- role reviewer, assurance instruction_only, selected skills empty;
- no initial images, custom effort, resume, provider override, custom environment
  override, generated post-inspection prompt suffix or fallback;
- instruction-only access is disclosed as ordinary native-user permissions,
  not filesystem confinement or a semantic guarantee that free text is obeyed.

An independently qualified adapter/configuration owner must establish that this
profile is supported for the actual platform/executable/version and selected
model. Registry presence, a native capability bit, an opaque hash or a well-shaped
fixture cannot qualify it. The supported production set remains empty here.
Existing native role/profile gates are preserved.

Closed configuration fields, ordered:

`{profile, ownerInstanceId, ownerRevision, codingToolId, modelId, platform,
executable, argv, catalogDigest, integrationDigest, capabilityEvidenceId,
capabilityEvidenceRevision, role, assurance, skills, reasoningEffort, images}`

Profile/role/assurance are the exact literals above. ownerInstanceId is a
sidecar-issued UUIDv4 correlation label backed by the exact live owner reference;
ownerRevision is a checked positive u64 epoch advancing for every observed
material configuration/adapter/integration/eligibility change, including failed
publication and ABA. IDs cannot substitute for that live custody.
Platform is macos_arm64, linux_x64 or windows_x64; these are value tags, not a
supported-profile announcement. executable is an owner-validated canonical
absolute UTF-8 path at most 4 KiB; argv is an ordered array of at most 64 UTF-8
strings, each at most 1 KiB (empty entries preserved). Full configuration is at
most 12 KiB. catalogDigest/integrationDigest are lowercase SHA-256 of the owner's
complete effective catalog and integration inputs, not just the override file;
their leaf production/coverage remains unqualified. capabilityEvidenceId is an
opaque ID, its revision a positive u64. skills/images must be empty arrays;
reasoningEffort must be null. Unsupported/extra values are unavailable, not
normalized to this profile. No secret or credential bytes may enter this value.

The exact owner must also retain nonserialized executable/native/integration
custody and revalidate it at consumption. Ambient native configuration,
credentials, repository instructions, executable replacement and external ABA
are not made immutable by this logical record. If the profile cannot bind a
controlled input that affects the resolved OrkWorks launch, deny rather than
claim a complete actual launch commitment. This task does not define a codec for
the complete effective catalog/integration inputs or qualify native delivery.

## Frozen prepared input and digest

Closed prepared value in key order:

`{schemaVersion, previewId, workspace, proposal, source, artifact, configuration,
userInstruction}`

schemaVersion is 1; previewId is the existing server-issued UUIDv4. The remaining
closed nested values are:

- workspace: `{workspaceId, sidecarInstanceId, workspaceGeneration, canonicalRoot}`;
  owner-derived 64 lowercase hex ID, UUIDv4, positive u64 and absolute UTF-8 path
  at most 4 KiB. The actual canonical owner, not a caller path, is authoritative.
- proposal: the **complete** proposal above, not only ID/revision or its hash.
- source: `{sessionId, runtimeInstanceId, runGeneration, eligibilityRevision,
  lifecycle, organization, effectiveWorktree, git}`; opaque ID, UUIDv4, positive
  u64, positive u64, literal live, literal active, absolute UTF-8 path at most
  4 KiB and the Git binding below. Source matches proposal's exact runtime/session.
  eligibilityRevision is a future source-owner epoch, not inferred lifecycle
  text. Its capture/fencing remains unqualified. This initial profile excludes
  ended/detached-owner/archived sources; broader source eligibility is separate.
- artifact: `{ownerCaptureId, generation, branchIdentity, kind, anchor, reference,
  pinned, selectionId}`; UUIDv4, canonical u64 allowing zero but below MAX,
  nonempty UTF-8 branch identity at most 1 KiB, exact design or implementation
  kind from the artifact owner, nonempty UTF-8
  anchor/reference each at most 2 KiB, boolean, nonempty ASCII selection ID at
  most 128 bytes. The capture label maps to the exact live Store ref in the
  owning entry; it is neither a serialized pointer nor transferable custody.
- configuration: the complete resolved narrow configuration above.
- userInstruction: the complete final inspected instruction, at most 16 KiB.

Source git is `{commonDirectory, head}`: an owner-validated canonical absolute
UTF-8 common-directory path at most 4 KiB, plus a closed tagged union. Head is
`{kind:branch, reference}` (owner-canonical branch identity, nonempty UTF-8 at
most 1 KiB) or `{kind:detached, commit}` (40 or 64 lowercase hex). A detached
HEAD needs a qualified shared artifact owner; a detached/session-local document
without that owner remains unavailable. Artifact branch identity and source Git
identity must join through the exact captured owner, not a title or same-name
branch in a different common directory. These are logical domains, not filesystem
canonicality proof.

The source effectiveWorktree, workspace canonicalRoot and prepared command cwd
must be the same owner-validated location. Cwd is canonicalRoot; it is not a
second caller-selectable configuration field. Current-root scope is preserved.
Draft tool/model selection must equal resolved configuration tool/model; a user
edit may differ from proposedDraft only after a new preview. Complete final
userInstruction equals the exact draft value used for this preview: no hidden
prefix, suffix, normalization, trimming or post-inspection regeneration. The
initial proposal path uses proposedDraft; an edited path supplies the accepted
strict three-field draft. The entry privately retains that exact selected draft.
The byte codec alone cannot distinguish a legitimate edited draft from arbitrary
bytes: the future preview producer must prove this join from its owned entry.

Nonserialized live workspace/proposal/runtime/configuration/Store references and
status/expiry are retained alongside this value. Labels and included snapshots
are logical commitments only; actual final admission must compare those exact
owners, epochs, eligibility, location and artifact selection under qualified
fences. Matching bytes or a digest never substitutes for currentness or authority.

Let C be the canonical UTF-8 bytes defined below. The precise commitment is:

```text
preparedInputDigest = lowercase_hex(SHA256(
  UTF8("orkworks.ordinary-plan-review.prepared-input.v1\n") || C(prepared)
))
```

Every prepared field, including full proposal/configuration and final instruction,
is included. previewId makes a separately inspected entry distinct; actual
attemptId/allocatedSessionId do not exist yet and are excluded. Exact retry
retains its original #1919 digest/session ID; it never hashes a regenerated view.
The digest is internal admission metadata, not a signature, capability, secret
container, native delivery receipt, process identity or verified result.

## Canonical encoding and byte bounds

Use compact closed JSON in the listed key order; arrays retain their stated
order. Fixed literals/numbers are exactly encoded; u64 domains are decimal
strings with no leading zero. No floats, negative zero, unknown/missing keys,
duplicates, sparse arrays or null except the explicit effort field. The Git
head union uses its variant-specific key order stated above. UUIDs are
canonical lowercase v4. Full-string validation rejects alternative representations.

All text must be Unicode scalar UTF-8: reject malformed UTF-8, lone surrogates,
NUL, C0 other than tab/LF/CR, and DEL. IDs/path fields additionally reject all
controls. No Unicode normalization, CR/LF conversion or trimming. Encode quote
and backslash as escaped characters; tab/LF/CR use JSON short escapes; all
other allowed characters are literal UTF-8. Do not escape slash, printable Unicode,
U+2028 or U+2029. No BOM, indentation or trailing newline. This is a fixed-shape
writer using standard JSON string escaping, not a new general-purpose parser.
Decode bounded fatal UTF-8 → parse → validate/join → fixed-order encode → exact
byte equality. Reject duplicate keys/alternate escaping/reordering via that equality.

Prepared bytes are at most 48 KiB. Before retaining/returning, charge the entire
accepted 64 KiB preview entry and 64 KiB response **separately**, including
this value wherever retained, live-entry encoded metadata, full display view and
any duplicated instruction bytes. This is not a sum of the two envelopes. The
actual entry/view/response codecs remain future work; the proposed component
tests qualify the closed proposal/configuration/prepared and draft-request bytes
only, not a complete cache/response budget witness. No arbitrary padding envelope
will be presented as a valid maximum. The stricter applicable limit wins; individual maxima do not promise combined fit. Retain the existing
24 KiB raw request and 16 KiB instruction caps, 8 entries/workspace and 30-second
nonrenewing TTL. Oversize means unavailable without truncation or partial hashing.
Never persist/log/export prepared bytes or prompt-bearing cache content. Only the
accepted bounded digest goes in an admitted #1919 record. Paths not representable
as scalar UTF-8 are unavailable for this profile; native-byte path support is
separate future work, not lossy conversion.

## Coupled qualification and remaining gates

The [offline plan](../plans/2026-11-19-review-launch-input-components.md) is
proposed for the same owner decision. It will add only Node 22 standard-library
logical codecs/vectors and joins. A passing golden digest establishes that exact
fixture bytes were committed, not that a source produced them or a process used
them. Existing #1919 fixtures and their registry digest field remain unchanged.

Investigated blind spots: persisted override hash is not effective configuration;
command defaults can change the selected launch; runtime identity has two parts;
role/profile capability remains unqualified; strings/JSON can change Unicode or
escaping; fixed leaf maxima can exceed total cache budget. Mitigations are the
closed profile, exact live-owner joins, full byte accounting and explicit production
refusal until those owners and physical boundaries are qualified.

Review correction: builder completeness review exposed the missing complete
entry/response codec, so that budget-qualification claim is deferred explicitly;
correctness review found the synthetic plan label would lose the actual selected
slot, now corrected to design/implementation with a substitution-denial test.
Bounded correction checks from both reviewers found no material remaining issue.
The coupled plan was reviewed once under the local plan policy.

Production still needs the #66 authenticated request/proposal producer and
revision/history publication, complete effective catalog/integration leaf codecs,
actual platform/executable/model eligibility, source/Store capture and lock
ordering, Start/recommendation lifecycle joins, durable admission initialization/
retention/export/compatibility and the private exact prepared creation seam.
No new native probe, permission, storage activation or production import is
authorized. Disposition: **Ready for scoped owner review; production Investigate.**
