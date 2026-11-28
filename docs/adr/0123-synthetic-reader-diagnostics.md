---
type: Architecture Decision Record
title: Synthetic reader diagnostics stay separate from qualification
description: Records the accepted bounded diagnostic protocol and exclusive payload-buffer fixture comparison.
tags: [orkworks, qualification, diagnostics]
status: stable
---

# Synthetic reader diagnostics stay separate from qualification

- Status: accepted
- Deciders: owner, through “keep going” after merged proposal #2193
- Date: 2026-11-27

## Context

The source-only diagnosis in #2187 found that timeout loses the buffered reader
report. It did not establish the Windows hot path. The owner accepted the
[complete synthetic contract](../superpowers/specs/2026-11-27-reader-diagnostic-contract-design.md)
for implementation after #2193. Tracking remains [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).

## Decision

Implement reader-diagnostic-v1 only in new pair-reader-diagnostic files: fixed
canonical nonce-bound phase tuples, incremental bounded parent retention,
cooperative-close accounting and full unknown-grant charges on failure. Compare
fresh and exclusively leased 1 MiB payload buffers over the same owned synthetic
identities, with independent catalog goldens and unchanged custody checks.
The accepted contract supplies all byte, work, time, input, process and result
limits. One five-minute step in the existing SDK fixture CI job is the sole
existing code/configuration exception. No dependency or discovery is added.

## Consequences

Tests can show the latest validated phase and allocation reduction on a fixed
fixture; they cannot establish live throughput, timeout cause or source admission.
Native execution is Windows-only; other hosts explicitly skip it. A failed
subject remains incomplete even when its expected-refusal test passes. Old
collectors, grants and ADRs 0118/0119 are unchanged. No live integration,
qualification job or renewed allowance follows. Removal of the new files and
single step reverses the change without runtime state migration.

## Amendment — 2026-11-28: isolated live profile

The owner accepted the [live integration package](../superpowers/specs/2026-11-27-live-reader-integration-design.md)
through “keep going” after merged #2222. Add a separate reader-live-progress-v2
codec/controller/reader and byte-identical selected policy, preserving the
synthetic v1 profile and all old spent tooling. The accepted contract fixes
retained prefixes, one leased 1 MiB payload buffer, output/source admission
before known settlement and the 132 MiB per-outstanding-reader parent share of
the same 32 GiB ledger. It also accepts exactly one new 45-minute Windows job
after verification, review, merge, immutable source freeze and fresh-history
checks. No retry or runtime/production qualification follows. All ADRs 0118/0119
constraints remain. Reverting the new isolated tooling/workflow/fixture step
reverses integration; an attempted job's allowance remains spent.

## Amendment — 2026-11-28: isolated root diagnostics, owned fixtures only

The owner accepted the [within-root contract](../superpowers/specs/2026-11-28-within-root-reader-diagnostics-design.md)
through “keep going” after #2267. Implement an isolated v3 request/record/output
domain in new pair-root-diagnostic-windows-build files, with 21 structural
boundaries and at most 11 sparse MSVC samples. Its 32 × 128-byte inner format
retains the same 4 KiB report, 18 KiB traces, 64 KiB result and conservative
source/output/debit settlement. Reuse guarded pure validators and seal their
transitive repository inputs; retain every reader custody pass and exclusive
payload lease. Legacy tooling and its domains stay byte-identical.

Acceptance covers new tooling and the contract's bounded owned fixtures plus
one five-minute normal SDK-fixture CI step. Node, reader and SDK CLI entry
refuse live use before scratch, installed probes or children. No qualification
workflow or installed-input attempt is included. A later concrete finite
proposal/owner decision must enable any live entry. Removing the new files,
LF entries and fixture step reverses this additive change, with no runtime
state migration and no renewal of a spent allowance. Native capacity, runtime
cost and the historical timeout cause remain unverified.
