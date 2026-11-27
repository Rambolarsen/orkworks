---
type: Validation Record
title: Synthetic reader diagnostic fixtures
description: Records bounded parser and owned timeout evidence separately from native Windows fixture and live qualification claims.
tags: [orkworks, windows, diagnostics, validation]
status: stable
workflow_status: synthetic-fixtures-passed
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
worker is added. Sixty-four pure/model/event-transport vectors share one ledger and
clock, followed by at most three fixed PowerShell fixture workers on Windows
and the one final zero-hash Node timeout child. Other hosts explicitly skip the
native portion. Failed subjects remain incomplete even when their expected
refusal assertions pass. No result is imported into candidate admission.

Local macOS arm64 / Node 22.23.5 verification passed the 64 vectors and real
owned timeout: one retained start record, failed/incomplete transport subject,
zero transport hash grant, owned termination/drain and scratch removal. Modeled
cap-boundary counters are arithmetic-only fake transport state, not newly
received bytes or physical hash work. The shared ledger separately reports
fixture_accounting_bytes and fixture_unknown_grant_bytes; those are not measured
hash throughput. The source/control seals, golden generation and final bounded
result commitment remain charged. Native Windows evidence is separate below.

## Windows fixture evidence

The [normal CI fixture job](https://github.com/Rambolarsen/orkworks/actions/runs/46041265231/job/140649409977)
passed at code head 690ee1999b43ee8fb86f9f59965945f4276103d8 on windows-2022,
Node 22.23.5 and PowerShell 7.6.10. Checkout used PR merge ref
2f926fd00e00204e27ca6255e6adc65c066338f9 (that head merged into main
8fc2f8d9208e9820411fdac756879d59fe2ef3a4); this records the tested code, not
a candidate source-admission claim. The step ran from 17:23:00Z to 17:23:23Z
on 2026-11-27; the driver reported 21,786 ms, with no time threshold claim.

- All 64 pure/model/event vectors and all exact 32 native guard dispositions passed.
- Complete reference/reuse manifests, IDs, catalog/member goldens and worker
  hash debits matched. Each positive worker reported h=6,533,035 bytes.
- Reference made 4,123 payload-scratch allocations; reuse made one. Both had
  at most one simultaneous lease. Cumulative allocation capacity is not RSS.
- Three native children ran. Trace counts were 21, 21, 3 and one final Node
  prefix; total child streams were 3,643 bytes and injected bytes 16,259.
- Total charged work was 21,719,692 bytes, including 2,112 fixture-accounting
  bytes (2,048 unknown modeled grants). Actual worker unknown-work was false;
  reservations ended at zero. No full manifests or raw invalid output were retained.
- The final zero-hash transport subject timed out as expected, remained
  incomplete, retained exactly one start record and drained with one termination.
  Scratch was removed. All qualification flags stayed false; target executions zero.

Earlier normal-CI heads refused before start, charged the full unknown grant,
stopped later children and removed drained scratch. Fixed numeric status 65
located the startup binding problem. Explicit owned WorkingDirectory and
normalized TEMP/TMP comparisons resolved that refusal. The independent root-row
normalization review finding and explicit byte-array request overload were also
corrected before this successful head. This is fixture evidence, not a retry of
the spent candidate job or evidence of its Windows timeout cause.

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

## Repository verification and independent CI limitation

Current code head 690ee1999b43ee8fb86f9f59965945f4276103d8 received a completed
[Codex review](https://github.com/Rambolarsen/orkworks/pull/2203#issuecomment-7224169723)
with no findings at 17:25:49Z on 2026-11-27 (substantial cycle 2/3). The initial
root-normalization and CodeQL newline-fixture threads have explicit fix replies.

The separate Windows workspace-reporting job failed in unchanged
scripts/antigravity-reporter-windows.test.mjs with spawnSync powershell.exe
ETIMEDOUT at its 30-second deadline. It is independently tracked in open
[#2209](https://github.com/Rambolarsen/orkworks/issues/2209), with recurrence on
other unchanged/docs-only heads; do not call it a transient flake or a live
Antigravity session result. The [observed job](https://github.com/Rambolarsen/orkworks/actions/runs/46041265231/job/140649410502)
is separate from the passing synthetic reader job. It is not one of main's
required contexts (Desktop, Rust, No relevant code changes). This tooling change
neither modifies that fixture nor weakens its test or timeout. Its failure must
remain disclosed even if the required contexts pass.

Full local bash scripts/verify-repo.sh passed: pinned Node prerequisites, Rust
format/build/tests, OpenCode reporter tests, Desktop type checking, native
Electron ABI repair, Desktop tests/build, docs build, diff check and doc/worktree
currency. Rust unit tests reported 3,745 passed and 25 ignored; all executed
integration tests also passed. The initial PATH-only Cargo failure was corrected
using the existing toolchain. No dependency pin changed. Rust warnings and the
site bundling warning refer to unchanged code; the README currency nudge was
assessed against this test-only ADR and needs no product claim change.
