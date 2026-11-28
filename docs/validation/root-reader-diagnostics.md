---
type: Verification Record
title: Isolated root reader diagnostics on owned fixtures
description: Records v3 sparse checkpoint verification without an installed-input or qualification allowance.
tags: [orkworks, qualification, windows, diagnostics, validation]
status: stable
workflow_status: verified-owned-fixtures-only
---

# Isolated root reader diagnostics on owned fixtures

Tracking [#2274](https://github.com/Rambolarsen/orkworks/issues/2274) under #1176.
The owner accepted [the concrete contract](../superpowers/specs/2026-11-28-within-root-reader-diagnostics-design.md)
through “keep going” after #2267. The
[acceptance receipt](https://github.com/Rambolarsen/orkworks/issues/1176#issuecomment-7236442647)
covers isolated tooling and bounded owned fixtures only. All earlier candidate
allowances remain spent. Node, PowerShell and SDK CLI entry deny live use;
there is no new qualification workflow, installed-input attempt or activation.

## Implemented boundaries

The new numeric v3 domain retains all 21 structural meanings and permits at most
11 fixed MSVC samples: seven early completed boundaries, then completed file
counts 16, 256, 4,096 and 65,536. No sample is an active-operation timer, file
name, inventory proof or admission. Silence cannot identify a runtime hot path.
Legacy tooling is unchanged. The policy is the exact 43,200-byte frozen copy,
SHA-256 1f5c006379597223a48939f2451ddaf4a530a9e31c83d52c7a8b676d95fecfa6;
the four repository source pins matched before implementation.

Strict framing/order/counter and complete-manifest predicates enforce the
conditional sample prefix. A counter-proven omitted sample denies known
settlement, including an error before MSVC opens. Lines stay within 128 bytes,
32 records/4 KiB per reader, 144-byte immutable arrival rows, 2 KiB aggregate
closed status metadata, 18 KiB traces and 64 KiB result. Typed receipts reject
unknown status fields and reused invocation nonces. Held complete source/output
admission precedes any known-work refund; unknown work spends its whole grant.
All repeated ancestry, handle identity, metadata, hashing, archive/member/helper
and row checks remain, with one exclusive 1 MiB payload buffer.

## Native red-to-green evidence

The draft intentionally established the native red case first:
[run46124777850/job140916868155](https://github.com/Rambolarsen/orkworks/actions/runs/46124777850/job/140916868155)
at ba9a409bbf70c68b949b728a1ab18b742f9b0a54 failed the independent owned-file
SHA-256 golden against a denied-entry reader stub. The complete custody reader
was then ported into the new namespace and the stub removed.

The first native green milestone was
[run46125523365/job140919146744](https://github.com/Rambolarsen/orkworks/actions/runs/46125523365/job/140919146744)
at addb9fdd. Its three bounded PowerShell workers checked preserved literal
custody/refusal cases, compared old/new complete normalized manifests and held
identities on the same owned files, and stopped after each of the seven early
samples. The actual pipe delivered 29 records before close; parent admission
read the held complete output after verifying its source seal. Seven controlled
failed subjects remained incomplete while their expected-refusal assertions
passed. The sole final zero-hash Node timeout child was drained with one owned
termination and scratch removal. It reported 35 preserved assertions,
35,174,013 conservatively charged hash bytes and 6,938 stream bytes, within the
64 MiB/1 MiB fixture envelope. This intermediate milestone predates the final
shared-pool/typed-receipt refinements; final-head evidence is recorded below.

## Verification and remaining limits

Local native work is explicitly skipped off Windows. Pure literal vectors cover
all higher milestones rather than creating a large installed tree. Transport
doubles exercise deadline crossing during spawn/close, unexpected stderr and
late data after refusal. Source/output tests use actual owned files and held
reads; reserve arithmetic is modeled on one decreasing ledger. The fixture
suite shares one cumulative work/stream pool, including conservative admission
and native worker precharges; it does not reset the pool per scenario.

An independent unchanged-code Windows Job fixture failed on the first two CI
heads with an empty retained-descendant snapshot/NotRunning. It is tracked in
[#2211](https://github.com/Rambolarsen/orkworks/issues/2211#issuecomment-7236539195).
No ownership code, assertion or timeout is changed, and a later passing run
would not explain those occurrences.

Final code [4f0fbe78217374c2d1848ab02ecfec1a33ce584a](https://github.com/Rambolarsen/orkworks/commit/4f0fbe78217374c2d1848ab02ecfec1a33ce584a)
passed [the final owned Windows job](https://github.com/Rambolarsen/orkworks/actions/runs/46127277161/job/140924629783):
Node v22.23.5, 64 total wire/storage-model vectors, 85,427 injected bytes,
41 native assertions including empty root/first-empty-file, 29 live records and
seven controlled refusals. It charged 47,840,847 conservative hash bytes and
76,468 streams, with exactly three PowerShell workers, one final zero-hash Node
timeout, one drained termination and owned scratch removal. Runtime/production
flags stayed false and target executions zero.

Pre-effect storage partitions cover the whole suite: parent 32 files/16 dirs/
1 MiB inputs/2 MiB scratch; each worker 512 files/64 dirs/2 MiB scratch, with
2 MiB inputs for checks and 512 KiB for each other worker. Their combined maxima
are 1,568 files/208 dirs/4 MiB inputs/8 MiB scratch. Four exact memory-only
storage overflow controls do not create a large filesystem. The pure suite also
checks the maximum-field-width literal, exact tail capacity and overflow,
69-row aggregate and over-cardinality refusal, lower-threshold bounded encoding,
and shared-drain expiry/latching. Unreachable outer byte boundaries are not
fabricated as valid domain records: stronger closed types/cardinality already
fit below them. Future domain expansion must update this compositional proof;
the actual hard limits remain independently enforced.

The fixed read-only correctness review found no material defect. Coverage
review gaps in storage/boundary tests were corrected in one pass. Additional
self-review regressions closed early-error omitted-sample refunds, typed receipt
metadata/nonce uniqueness and launched-fixture/test source sealing, with observed
red-to-green tests. The source seal covers all fifteen own/imported inputs below
1 MiB per pass. Hosted Codex completed the final-code review with no findings at
2026-11-28T08:55:47.738775Z
([summary](https://github.com/Rambolarsen/orkworks/pull/2278#issuecomment-7236891757)).
Copilot quota was already exhausted; no blind request was repeated. No review
finding remains undispositioned and no minor finding was deferred.

Full host verification passed: 3,758 Rust unit tests (25 ignored) plus integration
suites, 59 reporter cases, desktop type-check/native setup, 1,827 desktop tests
(5 skipped), desktop build, 2,583 docs tests and rendered build, doc/diff/worktree
checks. Changes made during that run were confined to the new qualification
profile/tests/evidence; dedicated local and exact-head Windows tests cover them.
Rust/Desktop source remained unchanged. The informational foreign-worktree
warning was reviewed and left to its owner; existing build/unused-code warnings
do not describe this diff. Required CI remains a merge gate; the
[PR check inventory](https://github.com/Rambolarsen/orkworks/pull/2278/checks)
records final status. The unchanged ownership job passed on the final code head;
this does not classify its earlier #2211 failures.

Native installed-input
capacity, CPU/I/O distribution, throughput, RSS and the historical timeout cause
remain UNVERIFIED. The next possible installed-input step requires a separate
concrete finite proposal and owner decision after tooling verification; this
record grants none. #1176 and all qualification/activation gates remain open.
