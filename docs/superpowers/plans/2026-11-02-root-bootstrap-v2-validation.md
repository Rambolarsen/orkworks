---
type: Implementation Plan
title: Retained root-bootstrap v2 source validation
description: A dormant Rust validator for the existing complete unverified root source slice, preserving canonical bytes and explicit version limits.
tags: [plans, roles, bootstrap, codecs, validation]
status: stable
workflow_status: implemented
---

# Retained root-bootstrap v2 source validation

Outcome: one pure, dormant Rust interface validates a complete retained
root-bootstrap v2 against its run, exact goal and retained artifact bytes.
Pass condition: independent existing golden bytes and all in-scope rejection
cases pass; success remains `unverified-source-bytes-only`.
Tracking: [#1307](https://github.com/Rambolarsen/orkworks/issues/1307), under #738.
The owner approved inline implementation of reviewed source
`c163bbbd3bde45c451f74a77c173e2569063ca38` with “go ahead”.

## Accepted basis and selected version

Use the accepted [#741 core](../specs/2026-11-02-role-configuration-component-review.md),
[root source codecs](../specs/2026-10-26-root-preparation-source-codecs-design.md),
[root-only schemas and rendering](../specs/2026-10-27-root-bootstrap-source-schemas-design.md),
and [repository/worktree identities](../specs/2026-10-04-configuration-learning-design.md#logical-records-and-digest).
The [root byte packet](../../validation/root-bootstrap-components.md) and
`docs/validation/root-bootstrap-vectors.json` supply independent Python-derived
positive expectations. The existing JavaScript checkers are reference consumers,
not authoritative when their checks are more permissive than accepted bounds.

The October source-codec approval covered documentation fixtures only. This
plan requests the named extension to dormant Rust source-byte validation, not
promotion to source authenticity, native support, approval or runtime authority.
#740 and #741 stay closed; their later dispositions supersede historical pending/
deferred wording only within their accepted scopes.

Support exactly the root v2 envelope and its named v1 nested source records.
Reject root v1, unknown versions and child-family records, including child v2
which shares a numeric tag but has a different closed shape. No generic latest
configuration type, v1 downgrade, migration or fallback is added. Child v1/v2/v3
source closure remains [#1183](https://github.com/Rambolarsen/orkworks/issues/1183).

The source slice retains the six own-run coordination actions; empty read/write/
command/external-source/connector permissions; no reference assignment inputs;
and root server-artifact output declarations with the existing tool pairing and
byte limits. Empty and nonempty output structures can be checked, but no producer
wrapper is qualified. A verified/limited/unsupported evidence decision is outside
this validator: accept only the existing explicitly unverified source slice.
No live discovery, file reads, native identity check, settings edit, storage,
endpoint, model call, probe, session launch or authority token is included.

## Interface and ownership

One implementer owns the coupled codec; inline execution is recommended. Use
existing serde_json, sha2/hex, base64 and chrono dependencies, with no new package.
A small interface hides the closed nested schemas and joins:

`validate_unverified_root_bootstrap_v2(bytes: &[u8], context: &RootBootstrapContext) -> Result<ValidatedUnverifiedRootBootstrap, RootBootstrapError>`.

`RootBootstrapContext` borrows canonical run bytes, exact UTF-8 goal text and an
array of retained artifact entries (raw bytes and their claimed content digest).
For this slice artifacts are well-formed UTF-8, matching the existing retained
text context; only settings/model/projection artifacts are parsed as canonical
JSON. Other retained fixture text is hashed literally. It contains no parsed/
bootstrap duplicate, callback, source resolver or authority.
The result owns the unchanged canonical bootstrap bytes and recomputed digest;
its constructor/fields are private, with read-only accessors and the literal
classification above. No serde implementation exposes a way to fabricate that
validated state. Errors use bounded static invariants/field locations, not source
text, secret-like values or a formatted parser error that echoes input.

| File | Responsibility |
| --- | --- |
| `crates/orkworksd/src/taskmaster/root_bootstrap.rs` | Single interface, byte/context preflight and complete root-to-run/artifact joins. |
| `crates/orkworksd/src/taskmaster/root_bootstrap/wire.rs` | Private strict canonical decoding, domain hashing and the existing root renderer. |
| `crates/orkworksd/src/taskmaster/root_bootstrap/source.rs` | Private closed nested source/profile/settings/model/evidence validators. |
| `crates/orkworksd/src/taskmaster/root_bootstrap_tests.rs` | Test through the same interface as callers; independent positives and adversarial cases. |
| `crates/orkworksd/tests/fixtures/root-bootstrap/` | Independently extracted canonical run/context byte expectations and provenance notes, only where the existing catalog lacks byte strings. |
| `crates/orkworksd/src/taskmaster/mod.rs` | Dormant module/test declarations only; no production caller. |
| `docs/validation/root-bootstrap-components.mjs` and its test | Narrow corrections of inherited revision/artifact bound checks; preserve existing positive catalog bytes. |

Helpers stay private to this module. Reuse coordinator canonical JSON/hash
functions behind the root-specific integer/shape guards; leave the coordinator's
accepted behavior and bytes unchanged. Do not extract a new generic framework.
Do not call JavaScript or spawn a process from Rust validation. JS is a test-time
comparison, never a runtime dependency.

## Exact byte and validation contract

- Preflight raw bootstrap at 1,048,576 bytes, run at 16,384 bytes and the existing
  artifact count/individual/aggregate ceilings before deserialization or copying.
  Goal bytes must equal the goal retained in the bounded run. Retained artifacts
  are a closed digest-addressed input set, not a filesystem lookup request.
- Accept canonical stored bytes only: strict UTF-8, well-formed scalar strings,
  recursively UTF-8-byte-sorted object keys, compact deterministic JSON, exact
  integer spelling, and declared sorted unique sets. Parse a bounded value, check
  allowed numeric forms/ranges and re-encode for exact byte equality. Duplicate
  keys, whitespace, BOM, alternate escapes and noncanonical numeric spelling
  cannot survive that comparison. Retain serde_json's recursion protection.
- Validate every closed record/enum/field and its owning bound from the linked
  schemas, not just the top-level tag. Include complete repository/worktree
  descriptors, RootAssignmentV1, provenance and direct/wrapped executable identity;
  role/rules/skills/resources, exact manifest/contributor projections and rubric;
  root requested/effective profiles; settings/configuration/model artifacts;
  all ten evidence surfaces/references and output declarations. Validate owner
  joins and complete declared resource/artifact closure in both directions.
- Recompute every owning digest, including snapshot content/role descriptor,
  permission profiles, retained artifacts, settings/model policy, evidence,
  repository/worktree, bootstrap v2 and run v1. Use the existing domain plus one
  literal LF exactly where specified; raw text/artifact hashes have no prefix.
  Supplied paths/object identities are syntax only, never native proof.
- Render exactly the accepted `ORKWORKS ROOT INSTRUCTIONS v1` length-framed
  grammar, fixed boundary text and canonical `{assignment,outputContract,rubric}`
  block. Compare full rendered bytes and raw text digest; count all framing in
  262,144 bytes. The #1303 renderer has different framing: do not substitute it,
  rewrite it or relabel/reseal old root bytes to make the two formats match.
- Preserve 2^31−1 maximum role-template/rubric revisions, 64 KiB inline evidence,
  64 KiB per retained evidence artifact, 1 MiB aggregate evidence artifacts and
  every smaller nested cap. Measure exact canonical descriptor bytes, including
  keys/escaping/inline values, rather than only summing selected text fields.
  Enforce the separate 4,096-byte repository/worktree native-path and 2,048-byte
  provenance/executable-path limits. No new retention or pruning policy is added.

The serialized raw bootstrap/run and retained-artifact limits bound parsing work;
all collection limits are checked before building indexes or rendered buffers.
Reference aliases do not create extra retained artifacts or hide bytes: validate
unique retained digest closure and apply the owning per-artifact/aggregate rules.
Natural-language applicability, source relevance, graph-skill applicability,
secret recognition, adapter setting/model semantics and actual delivery remain
review/qualification responsibilities. No success value can act as an admission
or launch grant.

## Delivery tasks and checks

### Task 1: Strict bytes and inherited reference bounds

Write failing wire cases for duplicate/nested duplicate keys, invalid UTF-8 or
surrogates, trailing data/whitespace, BOM, exponent/fraction/negative-zero numeric
spellings, wrong family/version and unsupported tags. Implement the private byte
and integer guards plus strict version-specific dispatch. Add JS regressions and
minimal guard corrections for role/rubric version 2,147,483,648 and a 65,537-byte
retained evidence artifact; both currently pass after consistent resealing.
Cover the inline evidence size cap too. Accepted maxima remain inclusive.
No positive catalog is regenerated or altered to accommodate the Rust serializer.

### Task 2: Complete root source validation and canonical rendering

Add the existing complete-root golden as a failing interface test, then implement
closed leaves and exact joins using the linked contracts. Compare all eight
existing vector byte lengths/fingerprints where relevant and the exact root/run/
rendered digests. Add an independently derived canonical run byte fixture using
Python standard-library serialization and the existing runDefinitionDigest;
never derive expected bytes using the Rust code under test.

Port the existing JS adversarial cases as observable Rust interface cases, with
consistently resealed controls so tests reach the targeted check. Include wrong
run/goal/repository/worktree identity; stale source/content hashes; malformed
manifest/resource/contributor links; nonempty forbidden root access or reference
inputs; output/tool mismatches; retained closure gaps/extras/duplicates; settings/
model/executable/platform mismatch; fake verified evidence; invalid timestamps;
and changed rendering. Preserve lossless decimal-string run counters through
2^63−1, including values above 2^53; reject a numeric replacement. Test native
Windows paths as exact UTF-16LE units in base64 (JSON scalar-string validation
does not normalize those native units), plus Unix byte paths and their distinct
NUL/absolute-path checks. Add maximum/one-over cases for every owning field and
collection limit, combined rule/resource count, canonical evidence/configuration
bytes and UTF-8/render framing. Reject overflow without allocating huge buffers.

### Task 3: Cross-language regression and bounded handoff

Run the unchanged Python catalog generator and compare its output byte-for-byte.
Run `node --test docs/validation/root-bootstrap-components.test.mjs` and the
complete docs checks. Test root v2 rejection of legacy/child families explicitly;
no public generic configuration parser exists. Compare Rust positive canonical
bytes/digests with the independent catalog, not only with the JS implementation.
Record that success is source-byte structure only; inspect the diff/call graph
for no production caller, runtime I/O or old renderer/serializer changes.

Use TDD for each code task and observe meaningful red assertions before the
minimal implementation. Focused command:
`rtk proxy cargo test --manifest-path crates/orkworksd/Cargo.toml root_bootstrap`.
Expected: intended cases execute and pass, not zero filtered tests. Then complete
`rtk proxy bash scripts/verify-repo.sh`, the sidecar build/test/fmt requirements,
current-code-diff manual review and PR CI. Select Node from `.nvmrc` and keep
fs-ext on the host ABI until Rust tests finish. Report any parallel-suite failure;
serialized verification cannot erase the failure history or waive required CI.
No native/model probe is needed. Docs-only preparation of this plan uses docs
checks/build, not the product test suite.

## Investigation and review disposition

At inspected base `a713bc5c`, the JS root checker provides the complete structural
slice and the Rust coordinator has reusable canonical JSON/hash helpers. #1303
is a text projection utility with intentionally different framing, not a root
codec. #1183 remains open for child source schemas. Offline synthetic resealing
confirmed that current JS accepts role/rubric revisions of 2,147,483,648 and a
65,537-byte evidence artifact; accepted canonical bounds require rejection.
These are reference-consumer corrections, not new schema/default choices.

| Dimension | Rating and evidence |
| --- | --- |
| Dependencies | 1 — One owner, existing accepted schemas and independent fixtures; no external producer needed. |
| Blast radius | 1 — Dormant Rust interface plus narrow documentation-checker rejection fixes. |
| State changes | 1 — In-memory validation only; no stored record or migration. |
| Reversibility | 1 — Revert module/checker changes; original catalog bytes remain intact. |
| Uncertainty | 2 — Cross-language canonical parity and old checker permissiveness need explicit differential/boundary tests. |

Total: 6/25; judgment aid, not approval threshold. Self-review: **Ready for owner
review**. All in-scope invariants have delivery/check coverage; no missing child
codec or native/source-custody qualification is guessed. This plan’s approval
would authorize only the named dormant validator and reference-check corrections.


## Implementation record

The owner-approved dormant validator and narrow reference-checker corrections are
implemented. Eighteen focused Rust interface test groups pass, including the
unchanged complete-root golden; native byte encodings; lossless counters;
settings/model/artifact joins; exact 1 MiB canonical bootstrap, 16 KiB run,
256 KiB rendering and 64 KiB evidence/artifact limits; and declared output bounds.
All 78 root JavaScript cases and 780 documentation component tests pass. The
unchanged Python generator reproduces the original catalog byte-for-byte.
The initial strict-wire/source and four bound regressions were observed failing
before their corresponding implementation. Full repository verification,
independent review and PR CI evidence are recorded with #1307's implementation PR.
No production caller or native/source-custody/authority qualification is added.
