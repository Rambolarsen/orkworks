---
type: Validation Record
title: Synthetic reader diagnostic fixtures
description: Records bounded parser and owned timeout evidence separately from native Windows fixture and live qualification claims.
tags: [orkworks, windows, diagnostics, validation]
status: draft
workflow_status: implementation-verification-in-progress
---

# Synthetic reader diagnostic fixtures

Tracking [#1176](https://github.com/Rambolarsen/orkworks/issues/1176), accepted
[contract](../superpowers/specs/2026-11-27-reader-diagnostic-contract-design.md),
[ADR 0123](../adr/0123-synthetic-reader-diagnostics.md) and
[implementation ledger](../superpowers/plans/2026-11-27-reader-diagnostic-implementation.md).
The owner accepted synthetic-only implementation with “keep going” after #2193.
No live job, installed input, compiler, SDK read, target execution or renewed
qualification allowance is part of this change.

## Executable checks

Run directly with the repository's Node version:
node scripts/qualification/pair-reader-diagnostic.test.mjs. No Node test-runner
worker is added. Sixty-three pure/model/event-transport vectors share one ledger and
clock, followed by at most three fixed PowerShell fixture workers on Windows
and the one final zero-hash Node timeout child. Other hosts explicitly skip the
native portion. Failed subjects remain incomplete even when their expected
refusal assertions pass. No result is imported into candidate admission.

Local macOS arm64 / Node 22.23.5 verification passed the 63 vectors and real
owned timeout: one retained start record, failed/incomplete transport subject,
zero transport hash grant, owned termination/drain and scratch removal. Modeled
cap-boundary counters are arithmetic-only fake transport state, not newly
received bytes or physical hash work. The shared ledger separately reports
fixture_accounting_bytes and fixture_unknown_grant_bytes; those are not measured
hash throughput. The source/control seals, golden generation and final bounded
result commitment remain charged. Native Windows outcomes are pending CI and
must not be inferred from this local result.

## Independent review dispositions

One fixed round used two read-only Luna reviewers; root remained sole writer.
Correctness review found missing parent volume and directory-ID verification;
root added complete file/directory/archive identity binding and a wrong-volume
regression. Completeness review found premature reservation settlement, weak
guard-summary matching, allocation arithmetic and missing boundary/process tests.
Root retained grants until output admission, added a failed-admission regression,
compared the exact 32 guard name/reason dispositions, pinned 4,123 reference
payload work-unit allocations (3 archive hashes + 1,027 member hashes + 1,027
extracted reads + twice 1,033 manifest reads), and added modeled cap boundaries
plus event-driven fake spawn/close/stderr/overflow/timeout/drain and finalization
failure checks. Superseded line references in reviewer reports refer to the
pre-fix draft. Neither reviewer executed native code.

## Limits of the evidence

The reusable worker must allocate exactly one 1 MiB payload array, with maximum
one lease, and preserve the complete independently checked manifest and h from
reference. Catalog/protocol/framework buffers are excluded from this metric.
The fixture proves allocation behavior on fixed owned inputs; it is no claim
about RSS, Windows live capacity, speedup or the cause of run46020662577's timeout.
All qualification flags remain false and target_executions remains zero.
Old collectors, pins, source policies, allowances and ADRs 0118/0119 are unchanged.
