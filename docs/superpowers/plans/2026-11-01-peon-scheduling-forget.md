---
type: Implementation Plan
title: Fence Peon scheduling against forgetting
description: Implementation and verification of the Peon scheduling deletion boundary.
tags: [peon, sessions, concurrency, testing]
status: draft
workflow_status: implemented
---

# Fence Peon scheduling against forgetting

Prevent earlier scheduler selections from recreating tracking after forgetting,
while preserving ordinary end, replacement runtimes and independent provider
execution. Implement the [proposed design](../specs/2026-11-01-peon-scheduling-forget-design.md)
for [#403](https://github.com/Rambolarsen/orkworks/issues/403). The owner approved
the concrete design with “go ahead” on 2026-11-01.
Continue inline in this owned task; no implementation fan-out is needed.

## One coupled deliverable

- [x] Record the approved synchronization decision in the next available
  `docs/adr/` record and indexes, linking the existing runtime/input/title
  contracts. Capture fresh source/configuration-matched Sonar and relevant
  Peon/retention regression baselines before source changes.
- [x] Add deterministic failing regressions around the actual synchronous
  scheduling boundary in `crates/orkworksd/src/runtime/peon_runtime.rs`. Exercise
  candidate admission, label requeue and timer repair on each side of real
  manual/retention deletion; include pre-metadata and post-metadata failures.
  A controlled barrier must demonstrate the interleaving, not a sleep or a
  source-text assertion. Extend shared fixture support only where necessary.
- [x] Give synchronous selection/admission a private, projection-fenced owner
  returning owned prepared attempts. Launch/reassemble/run providers outside
  that owner. Fence timer-origin repair separately, releasing it before the
  existing idle-transition application. Remove escaping unadmitted ID writes
  without changing diagnostics/provider outcomes or introducing tombstones.
- [x] Preserve current lock edges explicitly: release Peon map snapshots before
  sessions, and sessions before lease/in-flight mutation. Do not hold the fence
  across await, provider/input waits, or filesystem/Git scans. Keep current
  runtime/title/output guards and deletion eligibility authoritative.
- [x] Verify late success/failure/timeout/drop cannot repopulate forgotten
  tracking; replacement and unrelated sessions survive; provider blocking and
  shutdown do not hold the fence. Re-run existing end-label, title epoch,
  output debounce/generation, admission and retention failure regressions.
- [x] Run `bash scripts/verify-repo.sh` under Node 22, then a same-configuration
  Sonar after-scan. Report whole-call-path production/test deltas honestly;
  extraction alone is not a measured simplification. Update runtime/architecture
  prose to the implemented boundary, obtain current-diff review at the effort
  appropriate to concurrency changes, and follow the normal PR/CI/merge cleanup
  workflow. Close #403 only after reconciling every retained acceptance criterion.

## Review and risk assessment

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 2 | Existing scheduler, SessionApplication and shared projection fence; no external API/release dependency |
| Blast radius | 3 | Shared scheduler and input/deletion fence; isolate synchronous preparation from provider work |
| State changes | 2 | Process-local ordering only; no schema or stored-data migration |
| Reversibility | 1 | Local revert restores previous behavior; no data conversion |
| Uncertainty | 3 | Callback ordering reproduced; complete scheduler barrier tests and affected lock-order audit remain implementation acceptance checks |

Total: 11/25. Scores do not grant approval. This plan is proportional to the
explicit concurrency gate, not a requirement for routine cleanup.

Review disposition: **Ready**. A fresh read-only Luna builder/design review
found no material unanswered questions; the owner approved the concrete design
on 2026-11-01. Current-diff `/code-review medium` of
`cd16f48e6184bdcaa39c8edc3007afb11baedf01` found no findings. Medium effort was
selected for concurrency and the large preparation extraction. Final CI/merge
status is tracked in [PR #1264](https://github.com/Rambolarsen/orkworks/pull/1264).

## Execution evidence

The controlled boundary regression failed on the unfenced extraction, then
passed after fencing. The three added tests exercise 18 selection/deletion/error
combinations, deletion before admission, late settlement and forgetting during
blocked provider execution. Existing replacement-runtime, output, label and
shutdown checks remain green.

The full `bash scripts/verify-repo.sh` retry passed: 2,412 Rust tests (16 ignored)
plus integration tests; 1,651 desktop tests (2 skipped), types/native ABI/build;
776 docs tests/build; and repository close-out checks. The first run had one
unrelated initiating harness test failure (`codex_owned_install_preserves_needs_trust_with_compatible_tool`,
Unknown versus NeedsTrust), followed by 106 HOME-lock poison failures. Its
isolated run and the full retry passed; no unrelated source was changed. Logs
remain under `/tmp/orkworks-peon-scheduling-verify*.log`.

Matched Sonar scans of the exact rebased baseline and implementation reported
no analyzer errors/warnings. Whole-source totals, including inline Rust tests,
increased by 384 nonblank lines, 36 cyclomatic complexity, 38 cognitive
complexity and 7 functions; duplication was unchanged. Extraction is not claimed
as an overall complexity reduction. Coverage remains unverified.

A temporary timing probe exercised 10 preparations each with 1 and 32 live
sessions, 200 lines/session and 512 bytes/line. Preparation including channel
coordination peaked at 1.555 ms and 2.830 ms; contending projection-gate
acquisition peaked at 0.801 ms and 2.750 ms. Timer repair peaked at 0.061 ms and
0.029 ms. No provider work was included; its independent liveness is established
by the blocked-runner regression. This is synthetic gate-contention evidence,
not a terminal input latency bound or an end-to-end interactive benchmark.
The probe was removed and source restored exactly; raw samples remain at
`/tmp/orkworks-peon-scheduling-timing.log`.
