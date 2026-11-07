---
type: Implementation Plan
title: Session startup diagnostics implementation
description: Deliver the owner-approved safe launch and resume failure contract for issue 689.
tags: [sessions, diagnostics, implementation]
status: draft
workflow_status: active
---

# Session startup diagnostics implementation

Implement [the approved contract](../specs/2026-11-07-startup-diagnostics-design.md)
for [#689](https://github.com/Rambolarsen/orkworks/issues/689). Owner approval:
2026-11-07, “go ahead” after the concrete #1503 contract approval request.

## Boundaries and routing

Root owns Rust, canonical/user documentation and integration. One frontend
implementer owns desktop files in a separate sibling worktree; root integrates
its verified commit. Maximum one implementer plus one final reviewer, one review
round, no hosted coding-tool calls. Use lower-cost implementation tier first.
The desktop consumes the fixed code/operation/occurredAt object independently
of Rust implementation. No runtime or capability gates change.

## Delivery

- [x] Add safe typed codes and tolerant optional metadata deserialization; carry
  the field through all complete projections and preserve unrelated writes.
  Test malformed, unknown, oversized and legacy records and bounded catalog.
- [x] Carry classified startup errors separately from internal context; clear
  on admitted creation/resume and publish with generation-guarded error status.
  Preserve rollback, cancellation, finalization and storage-failure semantics.
  Test help failure/timeout/oversize, PTY faults, stale generations and reload.
- [x] Add specific HTTP 500 body and empty 409 cancellation; test bounded fixed
  messages, successful exact resume identity and asynchronous launch admission.
- [x] Frontend: validate/bound response parsing, fixed catalog, Terminal inline
  diagnostic, tracked-create toast and resume refresh without repeated toasts.
  Test hostile/unknown responses, workspace races and remembered presentation.
- [x] Update approved-status record, MVP/domain/API/user docs. Run Rust build,
  full tests/fmt, desktop tests/typecheck, verify-repo, docs build and close-out
  hooks. Review current code diff before PR handoff, then CI/PR close-out.

## Review and recovery

Reviewed once against each contract section: Ready. Dependencies 3 (known
Rust/desktop DTO); blast radius 3 (startup shared owners); state change 2
(additive optional JSON); reversibility 2 (old readers ignore the field; rollback
loses presentation only); uncertainty 2 (source paths inspected). Total 12/25.
No migration or destructive state changes. Important review focus: cancelled
startup after spawn, failure during metadata I/O, uncommitted resume rollback,
stale workspace response, legacy JSON field forgery. Tests exercise each;
forgery can select fixed text but confers no authority.

## Evidence ledger

- Preparation: existing design approved; isolated worktree reported; source base
  418cacba. No scope or permission changes.

- Regression evidence: resume diagnostic test failed with Null instead of
  process_spawn_failed, then passed after typed publication. Eight focused
  safe-record/HTTP/cancellation/persistence tests passed. Desktop affected
  checks passed 134/134 before review.
- Final review (medium, justified by lifecycle/API and diff size) requested three
  fixes: cancellation-only finalization, malformed JSON fallback, and lost
  intermediate-ending toast. Full initial Rust run: 3,147 passed, 3 failed,
  18 ignored; one was the deliberately changed 409 contract expectation, two
  reproduced cancellation regressions. Fix wave restores killed finalization,
  moves running after terminal I/O acquisition, adds PTY/I/O fault injection,
  and protects malformed JSON and pending-create reconciliation. Desktop fix
  tests reproduced failures before fixes; affected suite now 136/136.
- Review exclusions: native qualification and hosted execution remain outside
  authorization; no such calls ran. CI/build completion is checked by root.

- Scoped final fix review: APPROVE, medium effort; all three findings addressed,
  no new critical/required findings. Reviewed complete code diff SHA-256
  `54bd29f3eadb6e72f96261ac6493d7fc1966c97fe20a24aa2629235a716cf274`.
  Cancellation regressions passed 2/2; actual PTY/reader/writer fault test passed.
  No deferred minor findings or contract deviations remain. Full verifier completed successfully; see final evidence below.

- Final verification: `scripts/verify-repo.sh` passed using pinned Node 22:
  Rust formatting/build, 3,151 unit tests (18 ignored), seven integration tests,
  31 reporter tests, desktop typecheck, native ABI check, 1,709 desktop tests
  (three skipped), desktop build, 1,202 docs checks/build, whitespace and both
  close-out hooks. Existing Rust dead-code/unused-result, Vite chunk and Node
  color warnings are unrelated to this change. README currency suggestion was
  reviewed: the architecture reference adds the API contract but the product
  overview, milestones and ADRs are unchanged, so README needs no edit.
- All implementation and local verification steps complete. PR CI and terminal
  close-out remain tracked in the PR; native/storage activation is unchanged.

- Hosted documentation routing tests caught the new Rust test's embedded desktop
  catalog as an unrouted external input. Reproduced both routing failures locally;
  added that exact path to Rust CI and excluded it from CodeQL's unrelated-UI
  skip. Product-code diff/review remains unchanged; routing tests verify the fix.
