---
type: Validation Record
title: Live progress reader verification
description: Separates owned-fixture verification of the isolated live reader from the sole gated candidate attempt.
tags: [orkworks, qualification, windows, diagnostics]
status: draft
workflow_status: verification-in-progress
---

# Live progress reader verification

Tracking [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
The owner accepted the [combined package](../superpowers/specs/2026-11-27-live-reader-integration-design.md)
after #2222. [ADR amendment](../adr/0123-synthetic-reader-diagnostics.md#amendment-2026-11-28-isolated-live-profile)
and [plan](../superpowers/plans/2026-11-28-live-reader-progress.md) precede code.

## Fork inventory and current evidence

New pair-progress-windows-build files fork the complete replacement reader and
candidate stage logic. Candidate inputs, tool selectors, helper closure, SDK
selector, macro/startup flags and actual-argv/map/archive predicates stay exact.
Only v2 request/output domains, fixed derived roots, bounded streaming retention,
one leased payload scratch array, expanded-read precharges, monotonic scheduling,
source/output-before-settlement, protected parent admission and receipt retention
change. The SDK wrapper changes only its own environment marker domain. Every
old replacement and synthetic source/workflow remains unchanged. The sole old
CI exception is one five-minute direct Node fixture step; LF rules name only
new own files. No dependency is added.

Local Node tests cover v1/v2 separation, canonical framing, malformed/late/extra
bytes, strict output parsing, unknown/known settlement, decreasing grants,
parent-admission reserve, source/output refusal, inherited tool/manifest/SDK
predicates, transport doubles and deadline crossing. Local Python tests execute
six fake-registry cases and explicitly skip the one Windows-only HKCU fixture.
One fixed read-only review round covered correctness and missing coverage. Its
findings led to a shared strict request decoder/manifest pipeline/output writer
exercised by the native fixture, explicit conservative row/output encoding
precharges, and an explicit unavailable-publication outcome on reserve expiry.
Windows native fixture and full repository verification remain pending. Passing model tests do not establish physical native work,
capacity, historical timeout cause, runtime qualification or production pins.

The normal-CI live-reader harness reaches only owned synthetic files, archive
refusals and process doubles, at most one PowerShell fixture child and one final
zero-hash Node timeout child. Its PS tests report cumulative hash work, exclusive
allocation metrics and false qualification flags; parent applies the cumulative
64 MiB work and 1 MiB streams ceilings. Installed selectors/live main cannot be
called from these fixtures. Native evidence is unavailable until that exact
hosted job passes. Encoding work is deliberately overcharged: each primary row
reserves 512 plus six times the root/path/identity string lengths before JSON
conversion; archive member rows likewise use their names/digests. The final
serializer charges its whole allowed output ceiling before allocating JSON/UTF-8.
These conservative logical debits are not physical bytes or throughput. TAR
request precharges update the same budget immediately, so serialization cannot
spend pending decompression credit. No precharge is refunded on EOF or error.

No qualification dispatch has occurred. After verified tooling merges, an
immutable source freeze and fresh 0/1 history check must precede the one newly
accepted attempt. All earlier allowances remain spent; target execution is zero.
