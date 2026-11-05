---
type: Implementation Plan
title: SQL final-scan runtime composition
description: Connect original captured SQL provider, workflow evidence and history owners without activating production SQL.
tags: [orkworks, sqlite, sessions, runtime]
status: stable
workflow_status: implementation-complete
---

# SQL final-scan runtime composition

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Reuses the accepted [storage contract](../../../specs/session-storage.md) and
[ending evidence design](../specs/2026-11-04-ending-workflow-evidence.md).
The owner authorized this next integration after #1392 with “go ahead”.

## Outcome and decisions

Use the existing original capture, provider receipt, evidence receipt and history
completion as one explicit source-only runtime path. Capture before inference;
never construct replacement ownership after provider waits. Preserve per-stage
partial/unknown results and errors. Provider/evidence failure cannot impersonate
successful persistence, but compatible independent stages may still finalize.
A missing/unavailable/stale SQL preflight rejects the scan without JSON fallback.

A focused `terminal_runtime/final_scan.rs` owns snapshot/fallback selection and
stage composition. A small shared runner owns output capture and inference waits;
production still constructs JSON, with a separate explicit SQL constructor for
qualification. Controlled inference futures exercise the same runtime runner
without introducing a second provider framework or launching a coding tool.

Capture bounded output under original memory fences on blocking threads; perform
reassembly, SQL/worker/file waits and provider work outside application guards and
Tokio workers. Preserve JSON's existing session-generation/phase check and complete
its synchronous scan decision/effects together on one blocking job. Keep legacy
JSON scheduling behavior. SQL evidence's qualified owner alone schedules its
scoped deterministic evaluation; no additional model-analysis dispatch is added.

Retain inference-over-fallback precedence, captured ending observation fallback,
null recovery and terminal-outcome rules. Read SQL fallback from the authoritative
original identity; do not refresh its authority snapshot. Preserve existing
history-only APIs and physical-cleanup/close-first gates. No schema, migration,
production selector, native eligibility or measured responsiveness change.

## Delivery and checks

- [x] Add a failing complete-pipeline test demonstrating that the runtime
  history-only owner cannot yet compose provider/evidence/final-snapshot effects.
- [x] Add typed runtime scan owners/results and original-owner output/metadata
  access; connect JSON default and explicit SQL through the same runner.
- [x] Test exact provider/evidence/history order and durable fields, fallback/no
  inference/empty output, duplicate keys, already-ended/stale owners across waits,
  missing/unavailable SQL with valid legacy JSON, partial file/provider failures,
  cancellation and original-resource retention. Verify default JSON compatibility.
- [ ] Test a stalled stage on one Tokio worker with an independent OS watchdog;
  check application guards and another runtime input receiver. Run affected tests,
  full repository verification, current-diff medium review, CI and owned cleanup.

## Plan review

Ready for inline execution within the already approved ownership contracts.
Dependencies 3 (qualified captured owners), blast radius 2 (explicit SQL path and
bounded JSON composition refactor), state changes 2 (fixture files/rows),
reversibility 1 (source-only revert), uncertainty 2 (cross-stage partial outcomes),
total 10. Stage results remain separate; no cross-file atomicity or automatic retry
is claimed. Existing architecture decisions stand; this composes their owners.
