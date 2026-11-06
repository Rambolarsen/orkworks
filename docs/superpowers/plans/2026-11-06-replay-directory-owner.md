---
type: Implementation Plan
title: Retained replay cleanup directory ownership
description: Bind replay cleanup to original directory handles outside application guards.
tags: [orkworks, sessions, sqlite, storage]
status: stable
workflow_status: implementation-complete
---

# Retained replay cleanup directory ownership

Fix [#1442](https://github.com/Rambolarsen/orkworks/issues/1442) under the accepted
[storage contract](../../../specs/session-storage.md), continuing #1427 and #1444.
No production activation or new deletion scope is authorized.

## Outcome and decisions

A captured replay cleanup may remove only its original root/events directory
contents after either path is renamed/replaced, before execution or after
admission. Preserve exact ID, checkpoint cache, sync ordering and pending-job
partial receipts. Current code opens root/events from a copied path at execution;
no-follow does not distinguish a replacement ordinary directory.

- Keep `MetadataStore::capture_replay_cleanup` memory-only. Split its path/cache
  intent from an anchored owner with private types. Only the anchored owner can
  remove files; no implicit path reopen at removal.
- During `SessionWrites::capture_deletion`, validate the exact closed tombstone,
  reserve the existing file-job budget and bind the original directory handles
  in a retained blocking job outside application guards. Revalidate original
  memory after binding. Validate the opened root against the captured lease
  lock inode using a no-follow relative stat; a root replaced even before binding
  fails closed. Lease acquisition and legacy behavior remain unchanged.
  A two-second capture wait may expire without effects;
  the bounded job retains its lease/capacity until it drains.
- A successful captured view owns its budget slot until dropped or its stage's
  definitive completion; the stage transfers that slot rather than reserving
  twice. This bounds retained capture jobs/FDs to32 views/jobs with the existing
  byte limits. Workflow stage keeps its existing owner and semantics.
- On Unix retain no-follow root and optional events FDs. Absent events at capture
  is explicit; do not adopt a subsequently created directory. Removal must
  reject unexpected later presence without unlinking it, retain the job and
  permit explicit fresh capture/retry. Existing files, max IDs, unlink-only leaf
  symlinks and repeated directory synchronization keep their current behavior.
- Binding errors remain file outcomes for the stage, never a successful progress
  acknowledgement; workflow cleanup can proceed independently. Unsupported
  platforms retain their explicit Unsupported file receipt. No generic file
  service, schema, retention, producer or legacy JSON changes.

## Delivery and checks

- [x] Reproduce root/events replacement through actual SQL cleanup before
  execution and after admission; assert replacement bytes survive.
- [x] Implement retained capture/slot transfer. Test absent-directory replacement,
  original sync-failure retry, capture cancellation/expiry, exact32-view capacity
  and reuse, plus shared replay/workflow suites and one-blocking-worker behavior.
- [x] Update architecture/validation; run full repository verification and
  current-diff medium ownership/concurrency review.
- [ ] Record PR checks and owned cleanup on the PR and tracking issues.

## Plan review

Ready within existing approval. Main uncertainty was capture I/O under guards:
inspection confirms MetadataStore construction/capture must remain memory-only;
handle binding moves outside guards and keeps the existing shared budget and
lease. Dependencies3(existing application/file/SQL owners), blast radius2(private
cleanup capture), state changes2(fixture file removal, no live routing),
reversibility1, uncertainty2(controlled rename/capture failures), total10.
Power-loss/platform and numeric performance qualification remain activation gates.

Local implementation complete: both actual SQL regressions were observed red
before their fixes. All31 focused replay tests and full repository verification
passed (3,029 Rust unit tests, seven integration, 31 OpenCode reporter, Electron
ABI proof, desktop type-check/build and1,691 passing tests, documentation1,202
checks/build). Two independent medium reviews report no actionable findings.
Generic public-documentation reminders need no user-facing claim change for this
private source-only fix; architecture and validation describe its real boundary.
