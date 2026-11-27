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
