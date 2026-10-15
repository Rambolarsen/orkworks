---
type: Implementation Plan
title: Revisioned session metadata transactions
description: Storage prerequisite for coordinated Codex approval attention writes.
tags: [codex, metadata, concurrency]
status: draft
---

# Revisioned session metadata transactions

## Outcome and boundary

Implement [#881](https://github.com/Rambolarsen/orkworks/issues/881), the storage slice of [#788](https://github.com/Rambolarsen/orkworks/issues/788), using the [approved metadata design](../specs/2026-10-08-codex-native-attention-layer-design.md). A versioned cooperating writer must never replace a record after another accepted writer has advanced its revision. Legacy reads report revision `0` without rewriting the record; successful replacements persist checked, opaque revisions.

This slice adds no HTTP endpoint, producer marker, source split, native eligibility, or native-clear rollout. Existing callers that read before entering the replacement lock retain their legacy behavior; they still need migration under #788. Arbitrary direct filesystem writes remain outside the coordination guarantee.

## Decisions and evidence

- Keep one canonical session JSON record and the existing per-session lock in `metadata.rs`. A focused `metadata/session_transactions.rs` module owns revision parsing, fallible reads, and checked read/check/mutate/replace operations.
- Serialize the revision into the canonical JSON record at replacement, deriving it from the latest durable record rather than trusting a caller-supplied snapshot. This avoids adding a bookkeeping field to every legacy metadata constructor or promoting legacy writes into optimistic API writes.
- A checked transaction accepts an opaque expected revision, reads and validates the record under the same lock used by owned native clears, invokes a field mutation, and replaces the complete record before releasing the lock. Reject identity changes, stale revisions, malformed records, overflow, and failed persistence.
- All existing sidecar replacements, including identical and owned writes, advance the durable revision. Existing conservative ownership invalidation remains unchanged; attention-only ownership and provenance migration belong to #788.
- Investigation: the current lock protects final replacement only; existing native clear checks ownership under that lock. Its `ready` callback runs immediately before rename and must remain there. The transaction must not reenter store operations for the same session.
- Recovery: legacy records remain readable and acquire a revision only on their next accepted write. Do not quarantine malformed data in the fallible transaction reader or interpret it as a missing record. No production eligibility gate changes; reverting the implementation leaves the additive JSON field ignorable by older readers.

## Delivery and checks

- [x] Add failing storage tests for durable revisions through ordinary and owned replacements, stable legacy revision zero, overflow and malformed-data rejection.
- [x] Implement opaque revision and fallible versioned read; centralize revision advancement for ordinary and owned replacement.
- [x] Add failing revision-checked transaction tests, then implement the locked operation. Reject stale input and failed/invalid mutations without changes to metadata, revision, or ownership.
- [x] Exercise both owned-clear/writer orders and identical competing writes with real disk storage and deterministic channels. Hold the lock through the owned clear final check and replacement; a stale transaction must conflict after clear commits. Writer-first must invalidate the clear token.
- [ ] Update metadata domain/architecture guidance, run focused tests followed by required repository verification, review the current code diff, and open a PR linked to #881. Keep #788 and #690 open.

## Risk review

| Dimension | Rating | Evidence / treatment |
| --- | --- | --- |
| Dependencies | 1 | Existing metadata store and serialization; no new dependency or external runtime |
| Blast radius | 4 | Every session replacement passes this code; run full sidecar suite and repository checks |
| State changes | 3 | Additive string revision in existing JSON; legacy zero and older-reader compatibility |
| Reversibility | 1 | Older readers ignore the additive field; no rollout or migration marker |
| Uncertainty | 2 | Existing locked write/ownership code inspected; tests must pin both commit orders and I/O failure |

Total: 11. Remaining risk: these primitives do not serialize arbitrary legacy read/modify/write callers or external replacements. Those are explicitly excluded from this slice and cannot justify native-clear eligibility.

Review disposition: Ready for implementation within the approved storage design and the user instruction to work on the fix. Scope and checks cover #881; dependent API, provenance, producer, and native rollout work remains tracked separately.

## Review routing

One read-only review round, at most two lower-cost reviewers, after implementation.
The correctness reviewer inspects the changed storage code and tests for defects,
including lock and replacement semantics. The completeness reviewer compares the
same diff against #881 and this plan, looking for uncovered failure paths and
claims that exceed the slice. Both review the actual working diff against
`001972da90b5bafe0d2a979333ae1eb526d1b121`, cite file/line evidence, and flag
unverified assumptions. Neither edits, runs another coding harness, or delegates.
The owning session applies verified findings and remains the sole writer.
These reviews supplement the repository's explicit `/code-review` merge gate.

Supplemental review outcome: the correctness and completeness reviewers found no
actionable in-scope defects. Both identified the retained-store assumption, which
is documented above and in the domain/architecture guides. The completeness
review noted that the clear-first test proves the clear owns the lane until
replacement and the stale write conflicts afterward; its attempt signal alone
does not prove that the writer had already entered the mutex wait queue. No
stronger scheduling claim is made. Neither reviewer reran tests or invoked the
repository slash-command review gate.

## Verification evidence

`bash scripts/verify-repo.sh` passed on 2026-10-15 with Node 22.23.3:
Rust formatting/build, 1,910 unit tests (5 ignored), 4 reporter integration tests,
OpenCode reporter tests, desktop type-check/tests/build, documentation metadata
and indexes/build, diff checks, and documentation/worktree currency hooks.
The initial replacement regression failed because metadataRevision was missing;
after implementation the metadata suite and full suite passed. The initial
transaction tests also demonstrated the absent transaction operation before
implementation.

Existing Rust unused-code/test warnings, Node module/color warnings, and
frontend/documentation chunk-size warnings are outside this storage diff.
The documentation currency suggestion for user guides needs no edit because
this slice exposes no new user behavior or release availability. Worktree
currency initially labels this uncommitted new branch as merged because its
HEAD is still the main base; the pending commit resolves that false positive.
Other flagged worktrees are foreign and were left untouched.

The mandatory slash-command code review remains pending; supplemental reviews
do not replace it. Concurrency and revision semantics justify medium effort.
