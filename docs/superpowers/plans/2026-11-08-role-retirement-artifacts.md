---
type: Implementation Plan
title: Validate supplied role retirement artifacts
description: Plan one dormant snapshot codec using the accepted role and skill retirement digest contract.
tags: [plans, roles, retirement, validation]
status: stable
workflow_status: implemented
---

# Validate supplied role retirement artifacts

Outcome: one pure Rust entry validates a supplied canonical retirement-artifact
payload, retains its exact bytes, computes the existing snapshot digest and derives
its typed lookup key. Pass condition: independent golden bytes/digests/keys and
bounded denial tests pass with no production caller or side effect.
Tracking: [#1548](https://github.com/Rambolarsen/orkworks/issues/1548), under
[#741](https://github.com/Rambolarsen/orkworks/issues/741) / [#738](https://github.com/Rambolarsen/orkworks/issues/738).
Owner approved this exact reviewed unit for inline execution on 2026-11-08.

## Basis and nonduplicate scope

Reuse the accepted [core handoff](../specs/2026-11-02-role-configuration-component-review.md),
[canonical record/digest rules](../specs/2026-10-04-agent-role-configuration-design.md#canonicalization),
[resource closure](../specs/2026-10-04-agent-role-configuration-design.md#referenced-skill-instruction-resources)
and [retirement key/snapshot contract](../specs/2026-10-04-agent-role-configuration-design.md#bounds-compatibility-and-persistence).
The local/nonlocal provenance records use the already accepted selected source
shape in the [root source schemas](../specs/2026-10-27-root-bootstrap-source-schemas-design.md).

At inspected main `692821e67a6f82f6058e8550cffee946f19f8d8a`, instruction
composition (#1303 / #1305), retained root-v2 validation (#1307 / #1315) and
selected worker/reviewer source validation (#1345 / #1349) already exist.
`role_instructions`, `root_bootstrap` and `child_sources` remain dormant. No Rust
retirement-artifact codec or matching implementation issue was found. Do not
reimplement those shipped validators or reopen #740 research.

This unit validates supplied bytes only. It does not parse/write a tombstone,
read a catalog, decide retirement or selection, compare a retry against stored
state, invalidate proposals, approve work, expose an endpoint, resolve a path,
fetch content, qualify an adapter or launch anything. Storage transactions,
ledger exhaustion, Electron authority and retained-history rules stay with their
separately approved consumers. Hash validity proves neither provenance authenticity
nor current authority. No full configuration validation is claimed.

## Supported input and result

Use one crate-private function in `taskmaster/role_retirement.rs`:
`validate_retirement_artifact_v1(bytes: &[u8]) -> Result<ValidatedRetirementArtifact, RetirementArtifactError>`.
The output has private fields and read-only accessors for canonical bytes,
snapshot digest and a derived `RetirementArtifactKey`. Its version is typed:
role-template revision versus exact skill version text. It carries no
eligible/approved/retired flag, store reference or authority witness.

Input is exactly the existing canonical payload with keys `artifactKind`,
`snapshot`, `resources`. It is not a VersionRetirement wire record. Reject unknown
or duplicate fields, invalid UTF-8, floats/noncanonical numbers, unsupported tags
and noncanonical JSON bytes. Reuse existing coordinator canonical serialization
and taskmaster byte-only helpers where suitable; no serializer/domain changes or
shared-helper refactor is needed. Keep errors bounded and free of supplied text.

- `role-template`: snapshot has exactly the seven catalog fields named by the
  retirement contract; resources is empty. Accept exactly orchestrator, research, implementation, review, verification and remediation.
  Revision is integer 1 through 2^31−1. Verify instructionsDigest over exact UTF-8
  and contentDigest over the canonical role descriptor excluding contentDigest.
- `skill`: snapshot has exactly id, version, content, contentDigest, resourceIds,
  provenance. Version is nonempty UTF-8 text within 128 bytes. Assignment-only
  requirement, selectionReason and requirementSources are rejected here: future
  consumers must project the catalog-owned fields before calling this codec.
- Each resource has exactly the six existing catalog resource fields. Verify
  exact content digest, unique ID, matching owner and complete resourceIds closure.
  Require resourceIds and resources in canonical ID order; reject duplicates,
  missing/extra/cross-owner resources rather than silently repairing source bytes.
- Supported provenance is the existing schemaVersion 1 local/nonlocal closed
  record: sourceKind, sourceIdentity, sourceRevision and local sourcePath, with
  the existing variant constraints and lexical byte bounds. It is supplied
  evidence, never a filesystem observation. Other provenance shapes are unsupported.
- Derive sourceIdentity from snapshot.provenance, artifactId from snapshot.id,
  artifactKind from the validated variant and version from its typed value.
  snapshotDigest is not part of that lookup key. Preserve sourceRevision/path in
  the payload so changing provenance changes the digest without changing the key.

IDs/digests retain the existing 128-byte ASCII / lowercase 64-hex rules. Reuse
512-byte source identity/revision and 2 KiB source locator bounds; resource count
and resourceIds count are at most 32. Check total supplied content at 256 KiB and
input/output payload at 1 MiB before unbounded copying/output allocation. These
are this dormant projection's admission caps drawn from the accepted configuration
bounds, not a whole ledger packing or native resource qualification. Exact and
one-byte-over multibyte cases must be covered.

Snapshot SHA-256 is the UTF-8 domain `orkworks.retirement-artifact-snapshot.v1`
followed by one literal LF and the canonical payload bytes. Include every catalog
field/provenance/resource; preserve content whitespace, Unicode and line endings.
Never substitute a configuration, root source or raw content digest domain.

## Delivery and verification

One writer implements inline; validation, canonical bytes and key derivation are
one coupled unit. A focused fresh reviewer checks the complete code diff before
merge. Planning review uses one read-only Luna-tier reviewer, one pass; the owner
retains all edits. No implementation worker or coding harness is launched.

| File | Responsibility |
| --- | --- |
| `crates/orkworksd/src/taskmaster/role_retirement.rs` | Dormant entry, strict supported shape, private validated output and key/digest derivation. |
| `crates/orkworksd/src/taskmaster/role_retirement_tests.rs` | Golden, denial, boundary and cross-domain cases. |
| `crates/orkworksd/tests/fixtures/role-retirement/` | Synthetic canonical role/skill/resource payloads with independent expected digests and typed keys. |
| `crates/orkworksd/src/taskmaster/mod.rs` | Module declarations only; no production caller. |

- [x] Pin independent role-template, plain skill and resource-bearing skill
  goldens using an independently written Python/Node calculation from literal
  catalog data. Include non-ASCII content, JSON escaping, LF/CRLF and provenance.
- [x] Observe focused tests fail before implementation. Cover strict/unknown
  fields, wrong version types, bad nested digests, duplicate keys/IDs, closure
  ownership, unsorted sets, invalid provenance and wrong hash domains.
- [x] Implement the supported codec without I/O or consumers. Assert derived
  keys: same key/different provenance payload yields different snapshot digest;
  a different catalog source yields a different key. Do not implement the later
  stored retry/retirement decision.
- [x] Verify exact/over-limit byte and numeric bounds, empty role resources and
  exclusion of assignment annotations. Re-run existing role instruction, root
  bootstrap and child-source goldens unchanged.
- [x] Run Rust fmt/build/tests and `bash scripts/verify-repo.sh` under pinned
  Node 22 with frozen desktop/docs dependencies and host fs-ext ABI before Rust.
  Run `bash scripts/doc-check.sh` and `bash .claude/hooks/worktree-check.sh`.
  Exact-code manual review passed; required CI remains the PR merge gate.

## Investigated uncertainty and review

The initial suggested validator/renderer was stale: #1305 already delivers it,
and #1315/#1349 cover later selected source-byte slices. Searching actual code,
all-state issue titles and merged PRs resolved that duplication before editing
product code. A broad general configuration codec would add unrelated permission,
provenance and multi-version choices; it is excluded from this bounded unit.

Retirement source identity versus snapshot identity is the main risk. The accepted
contract explicitly separates the key from snapshotDigest and includes catalog
provenance/resources. Private output, typed versions, strict supported provenance,
independent goldens and no stateful caller prevent this codec from silently
claiming the later authority/transaction behavior.

| Dimension | Rating | Reason |
| --- | --- | --- |
| Dependencies | 2 | Existing accepted core and byte helpers; no independent service/release. |
| Blast radius | 2 | Dormant module; accidental hash drift would affect a future consumer. |
| State changes | 1 | Pure supplied-byte validation; no persisted state. |
| Reversibility | 1 | No caller/data migration; revert the isolated module. |
| Uncertainty | 2 | Exact digest/key defined; supported provenance/caps need explicit plan approval. |

Total: 8/25. Rollback is local removal/revert of the dormant module, with existing
consumers and wire formats unchanged. Plan disposition: **Ready for owner review**, following one focused read-only
Luna review with no material findings. The optional role-tag spelling clarification
was applied. Owner approval was recorded on #1548 before implementation. That approval does not
supply storage, retirement authority, an eligible role or a new probe budget.

## Implementation evidence — 2026-11-08

Owner approval preceded code and is recorded on #1548. The dormant codec and
three independent synthetic Python goldens are implemented; the first valid-input
test failed against a rejecting stub before implementation, then all nine focused
tests passed. Existing instruction/root/child suites passed unchanged (18/18/12).
The full repository verifier passed under Node 22 with frozen dependencies and
host fs-ext ABI: 3,265 Rust tests (18 ignored), all integration tests, 31 reporter
tests, 1,709 desktop tests (3 skipped), 1,202 docs tests, formatting, builds and
currency checks. Doc-check's broad user-doc/architecture warnings need no edit:
there is no user behavior, endpoint, dependency or runtime flow change.

Fresh `/code-review medium` on the implementation tree
`f1c02b297548c2964550099b507f916139228407` found no findings or declined judgments.
Medium effort follows the 500-line threshold. The review inspected tests but did
not run them; the executor supplied the test evidence above. Required CI and PR
feedback disposition remain merge gates, tracked by #1548's implementation PR.
Encoded native path records and nonempty content reuse the selected source
validator contract; other path shapes and empty content remain unsupported.
No consumer, retirement authority or new model/native probe was introduced.
