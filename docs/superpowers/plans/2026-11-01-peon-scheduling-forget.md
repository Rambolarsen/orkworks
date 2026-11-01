---
type: Implementation Plan
title: Fence Peon scheduling against forgetting
description: Proposed implementation and verification of the Peon scheduling deletion boundary.
tags: [peon, sessions, concurrency, testing]
status: draft
workflow_status: approved
---

# Fence Peon scheduling against forgetting

Prevent earlier scheduler selections from recreating tracking after forgetting,
while preserving ordinary end, replacement runtimes and independent provider
execution. Implement the [proposed design](../specs/2026-11-01-peon-scheduling-forget-design.md)
for [#403](https://github.com/Rambolarsen/orkworks/issues/403). The owner approved
the concrete design with “go ahead” on 2026-11-01.
Continue inline in this owned task; no implementation fan-out is needed.

## One coupled deliverable

- [ ] Record the approved synchronization decision in the next available
  `docs/adr/` record and indexes, linking the existing runtime/input/title
  contracts. Capture fresh source/configuration-matched Sonar and relevant
  Peon/retention regression baselines before source changes.
- [ ] Add deterministic failing regressions around the actual synchronous
  scheduling boundary in `crates/orkworksd/src/runtime/peon_runtime.rs`. Exercise
  candidate admission, label requeue and timer repair on each side of real
  manual/retention deletion; include pre-metadata and post-metadata failures.
  A controlled barrier must demonstrate the interleaving, not a sleep or a
  source-text assertion. Extend shared fixture support only where necessary.
- [ ] Give synchronous selection/admission a private, projection-fenced owner
  returning owned prepared attempts. Launch/reassemble/run providers outside
  that owner. Fence timer-origin repair separately, releasing it before the
  existing idle-transition application. Remove escaping unadmitted ID writes
  without changing diagnostics/provider outcomes or introducing tombstones.
- [ ] Preserve current lock edges explicitly: release Peon map snapshots before
  sessions, and sessions before lease/in-flight mutation. Do not hold the fence
  across await, provider/input waits, or filesystem/Git scans. Keep current
  runtime/title/output guards and deletion eligibility authoritative.
- [ ] Verify late success/failure/timeout/drop cannot repopulate forgotten
  tracking; replacement and unrelated sessions survive; provider blocking and
  shutdown do not hold the fence. Re-run existing end-label, title epoch,
  output debounce/generation, admission and retention failure regressions.
- [ ] Run `bash scripts/verify-repo.sh` under Node 22, then a same-configuration
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

Review disposition: **Ready for owner design approval**. A fresh read-only
Luna builder/design review on 2026-11-01 checked the proposed boundary against
the scheduler, deletion owners, idle transitions and affected lock edges; it
found no material unanswered questions. Inline plan review found scope,
requirement coverage and recovery checks sufficient. This is readiness for the
approval gate, not authorization to implement or evidence that future code
passes its checks. Owner approval was supplied on 2026-11-01; implementation is in progress. Reuse review of unchanged artifacts and revise only
material findings.
