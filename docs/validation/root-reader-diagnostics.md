---
type: Verification Record
title: Isolated root reader diagnostics on owned fixtures
description: Records v3 sparse checkpoint verification without an installed-input or qualification allowance.
tags: [orkworks, qualification, windows, diagnostics, validation]
status: draft
workflow_status: implementation-validation-in-progress
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

Final host verifier, current-head Windows suite, fixed read-only reviews and
hosted review/CI disposition are pending in this record. Native installed-input
capacity, CPU/I/O distribution, throughput, RSS and the historical timeout cause
remain UNVERIFIED. The next possible installed-input step requires a separate
concrete finite proposal and owner decision after tooling verification; this
record grants none. #1176 and all qualification/activation gates remain open.
