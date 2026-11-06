---
type: Implementation Plan
title: Bounded recommendation cleanup preflight
description: Qualify a read-only original-directory recommendation inventory before SQL deletion integration.
tags: [orkworks, taskmaster, sqlite, storage]
status: stable
workflow_status: implementation-complete
---

# Bounded recommendation cleanup preflight

Prerequisite under [#1243](https://github.com/Rambolarsen/orkworks/issues/1243),
after #1471; implements the ownership/preservation boundary of the accepted
[storage contract](../specs/2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion).

## Outcome and decisions

Provide a source-only, read-only cleanup preflight owned by RecommendationStore.
Memory-only capture copies the store root and shared operation owner. Binding
runs off application guards, opens the original workspace/recommendation
directories without following aliases, validates and retains the exact lease.
Inspection nonblockingly claims the existing operation owner before all file I/O;
a returned inventory retains that permit and binding until dropped. It contains
validated recommendations, never an arbitrary-path or deletion capability.

Use descriptor-relative enumeration/reads on qualified macOS/Linux only.
Conservative internal preflight limits are 1,024 directory entries, one MiB per
record and 32 MiB aggregate encoded record bytes. Exceeding a limit is explicit
failure, not truncation or a new production storage limit. Refuse unknown entries,
transaction directories (even empty), legacy/mismatched filenames, ambiguous JSON,
invalid graphs and nonregular/aliased files before returning an inventory. Pending
transactions require existing owner recovery before retry; preflight performs no
recovery or file writes. Reject a recommendation-directory replacement/appearance;
retain original workspace anchoring if its path moves. Unsupported platforms
return Unsupported before file effects.

This inventory is not a tombstone/source-freshness check or evidence protection
decision. No app wiring, recommendation removal, new transaction/recovery format,
SQL progress bit, final row/job completion or production activation. Graph
publication can recreate records from stale snapshots between operations: exact
SQL source revalidation/revocation remains required before deletion activation.

## Delivery and checks

- [x] TDD memory capture/binding, shared contention/owned lifetime, original root
  and recommendation identity, retained lease, repeat inspection and absent root.
- [x] Implement the private preflight module and bounded strict record reader;
  reuse existing recommendation/graph validators and encoded filenames.
- [x] Test malformed/duplicate/trailing JSON, unsafe entries (symlink, hardlink,
  directory, FIFO), mismatched filenames/IDs, pending transactions, count/record/
  aggregate limits, graph failure, no mutation and independent roots. The API is blocking-only and unwired; application admission and independent
  terminal-delivery proof remain required when integrating the captured stage.
- [x] Update architecture/validation with exact limits and exclusions; run focused
  tests, full repository verifier and current-product manual medium review,
  then prepare the reviewed PR.
- [ ] Record PR review/CI and owned cleanup at terminal state.

## Plan review

Ready under the accepted design, scoped to read-only qualification. Inspection
found legacy read_all/read_all_stored use unbounded path reads and cleanup first
replays transactions; calling that path on captured original files would not
establish finite work or alias safety. Reusing the existing mutation/recovery
algorithms now is therefore rejected; this task builds only the bounded read
boundary. The existing image cleanup module supplies the descriptor enumeration
pattern; opening an independent directory stream avoids sharing seek offsets.

Dependencies3(store/lease/validators), blast radius1(unwired private module),
state changes1(read-only), reversibility1(revert unused source), uncertainty2
(descriptor races/limits), total8. Tests pin root rename/replacement, directory
appearance/replacement and shared owner lifetime. Retained lease/permit protects
in-process ownership only; this is not protection against arbitrary same-user
filesystem mutation, power loss, a moved-path independent owner, or qualified
cross-platform deletion. These limits stay explicit before dependent work.

Manual medium descriptor/ownership review found one interface issue: a public
Vec permitted accidental record extraction while releasing inventory ownership.
The records are now private and exposed by borrowed slice; explicit clones remain
stale snapshots without authority. Consumer compilation failed for the missing
accessor before implementation, then all 68 store tests passed including 11
preflight tests. Aggregate fixtures now accept exactly 32 records of exactly one
MiB each before rejecting the next record. Both independent final delta reviews
are clear on product SHA256
`7cee03a412c66ab6050f79aabe8a0cd9b7b1c8ef44e1bab0fab95a4336cbe58e`.
Filesystem enumeration order is intentional: this inventory promises completeness,
not ordering. Final-code full verification passed: 3,104 Rust unit tests (18 existing ignored),
seven integration, 31 reporter, one Electron ABI proof, desktop type-check/build
and 1,694 tests (three existing skipped), docs 1,202 checks/build and
diff/doc/worktree checks. The public-doc reminder needs no installer/user-flow
claim update for this unwired read-only prerequisite. PR lifecycle evidence
will be recorded on #1243.
