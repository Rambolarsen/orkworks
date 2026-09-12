# Custom Inference Approval Flow Implementation Plan

> **For agentic workers:** Use executing-plans inline. Root is sole writer; continue the owner-approved design without another design gate.

**Goal:** Expose explicit inspect/approve/revoke in Recommendations settings without activating custom inference.

**Architecture:** A sidecar application service holds the harness store mutation lock while resolving the current adapter and writing trust. Dedicated authenticated routes carry strict revision-bound requests. Electron owns fixed endpoints and a native warning confirmation; the renderer receives independently declared narrow types and never receives authority.

**Tech Stack:** Rust/serde/Axum; Electron TypeScript; React; existing tests.

**Spec:** [Approved adapter design](../specs/2026-09-10-custom-inference-adapters-design.md), ADR 0055, issue #503.

## Constraints

- Existing CLI login/configuration remains untouched. No command probes or custom model discovery.
- Inspect includes command, canonical path, argument templates, input mode and timeout. Approval warning discloses workspace context, credential access, hooks/plugins, no sandbox certification, installed-tool updates, administrator policy, and timeout-limited revocation.
- Imported JSON, general settings saves, and session tokens cannot grant trust.
- Requests bind harness-document revision, identity digest and durable trust generation (decimal string on the JS boundary). Changed definition, path, generation, or sidecar generation rejects approval.
- Custom adapters remain inactive, distinct from approved. List capability-bearing tools independently of Peon, including unavailable executables so users can revoke.
- One root writer; maximum two read-only reviewers, one correctness and one coverage question, one round. No external cross-model review, commit, push, or PR.

## Task 1: Sidecar approval service and authenticated routes

Files: new `taskmaster/inference_approval.rs`, new `http/inference_trust_handlers.rs`; register modules/routes; extend `harness/store.rs` with a locked-snapshot callback and `inference_trust.rs` with generation inspection; reuse existing Taskmaster request authorization.

Interfaces: `inspect_adapters(&HarnessStore, &InferenceTrustStore) -> Result<Vec<AdapterTrustView>, ApprovalError>`; `change_approval(&HarnessStore, &InferenceTrustStore, ApprovalRequest) -> Result<(), ApprovalError>`. Request `{harnessId, action, expectedRevision:{documentRevision,generation,digest}}`; action approve/revoke; digest nullable only for unavailable executable revocation. View includes id/name/definition/resolvedPath/state/revision. POST returns success; GET refreshes views.

- [x] Test first: persisted custom no-Peon definition lists approval-required; approve and reload becomes approved; same request conflicts after reuse; editing definition makes the old approval request conflict; missing executable can be revoked. Example assertion: `assert_eq!(inspect_adapters(&harnesses, &trust)?.len(), 1)`.
- [x] Run `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml inference_approval -- --test-threads=4` to observe missing behavior, then implement service using `HarnessStore::with_locked_snapshot` and fresh `AdapterIdentity::resolve`.
- [x] Add authenticated route tests: a session bearer token without privileged authority cannot mutate; duplicate/unknown fields rejected by strict bounded parsing; stale revision returns 409, invalid request 422, unavailable persistence 503.
- [x] Run focused Rust tests/build/fmt.

## Task 2: Privileged Electron flow and Settings UI

Files: new `electron/inferenceTrust.ts`, new `src/inferenceTrust.ts`, new `src/components/InferenceTrustSettings.tsx`; modify main/preload/window declarations and mount component in Taskmaster settings; new desktop tests.

Interfaces: `getInferenceTrust()`, `approveInferenceAdapter(request)`, `revokeInferenceAdapter(request)` narrow preload methods. Electron validates all runtime input, reloads the exact inspected identity before a native confirmation, checks sidecar generation again after confirmation, then POSTs the revision-bound request. Renderer never provides URLs, tokens, executable paths, or warning text.

- [x] Write tests for fixed loopback endpoint, invalid request rejection before fetch, cancellation without POST, and sidecar generation change during confirmation without POST. Example assertion: `assert.equal(postCount, 0)` after declining.
- [x] Observe failures, implement independent wire types and transport/controller, then add a Settings list with inspect details, explicit approve/revoke buttons, busy/error handling, refresh and inactive-state disclosure.
- [x] Run desktop tests/typecheck, full repository verification, then read-only reviews. Patch actionable findings and verify again. Record remaining execution/cache integration under #503.

## Execution and review record

- Implemented the application service, authenticated routes, narrow Electron/preload boundary, native cancel-default confirmation, and renderer controls. Custom execution remains inactive; no provider was invoked or real user trust granted.
- One read-only review round: correctness reviewer found no actionable defects; coverage reviewer identified malformed digest error classification, missing digest checks on available revocation, and endpoint coverage gaps.
- Reproduced both behavioral findings with a regression test (first malformed digest returned Conflict instead of Invalid; then stale revocation returned success instead of Conflict). Patched them and added HTTP success/conflict/invalid/corrupt-persistence/unavailable-revocation coverage. Focused inference suite: 67 passed, 2 ignored.
- Close-out uncertainty check: inspected the native confirmation wiring and inactive-state disclosures. Existing CLI access, hooks/plugins, same-path updates and absence of sandbox certification are disclosed; runtime trust enforcement is deliberately pending under #503, not a new uncovered scope item. Native GUI and Windows runtime smoke tests have not been performed.
- The owned feature worktree is retained because the broader feature is still in progress; neither main nor the separate Windows installer worktree was changed. No commit, push, PR, external review, or native `/code-review low` gate was performed in this slice.
- Final verification: `rtk env RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` exited 0: 1,109 Rust unit tests and 3 integration tests passed (2 ignored), 692 desktop tests passed, typecheck/build/docs/format/diff/currency checks passed. The preceding full run had two Rust failures, including a one-second broken-pipe test reporting timeout; all seven process-runner tests passed in isolation and the unchanged full rerun passed. Scheduling sensitivity is suspected, not established; no unrelated runner change was made.
