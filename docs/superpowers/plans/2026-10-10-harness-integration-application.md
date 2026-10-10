---
type: Implementation Plan
title: Harness integration application extraction
description: Extract existing Rust integration orchestration behind a typed application interface and preserve every existing transport and runtime behavior.
tags: [harness, integration, refactor]
workflow_status: accepted
---

# Harness integration application extraction

> **For agentic workers:** Use `subagent-driven-development` for the single cohesive implementation task below, with task review and final branch review.

**Goal:** Integration callers use a typed application interface without knowing
HTTP implementation, lock/probe sequencing or revocation bookkeeping. The
pass condition is unchanged configuration, attention and transport outcomes.

**Architecture:** One concrete `HarnessIntegrationApplication` coordinates the
existing AppState and existing adapters. Preserve separate private legacy and
grouped revalidation paths. HTTP adapters parse and serialize; session callers
use the new application module directly.

**Tech stack:** Existing Rust, Tokio, Axum, serde, git2 and tempfile; no new dependencies.

**Spec:** [Approved design](../specs/2026-10-10-harness-integration-application-design.md).
**Issue:** [#816](https://github.com/Rambolarsen/orkworks/issues/816).
**Authority:** [MVP integration contract](../../../specs/orkworks-mvp.md#resolved-harness-capabilities-and-integrations),
[ADR 0030](../../adr/0030-integration-lock-check-await-helper.md) and
[architecture reference](../../agents/architecture.md).

## Global constraints

- “Existing observable behavior is preserved.”
- “No synchronous guard may cross an await.”
- “Do not add a new asynchronous gap between configuration mutation and required revocation finalization.”
- “Group mutation requests retain required document and active-selection revisions; legacy harness requests retain their existing contract.”
- “Grouped adapter failures currently become a grouped status with registration `error`, often returned with HTTP 200. Legacy adapter failures become HTTP errors.”
- “Launch readiness is only a conservative boolean query and performs no revocation.”
- “Cleanup retains its existing selection, ownership checks, diagnostics and complete/cleanup-needed outcomes; extraction must not add new mutation or revocation policy to that path.”
- No Electron/renderer changes, new tool bindings, reporter edits, public schema changes, migrations or probe process-tree changes.
- Preserve authorization in the existing router middleware and native confirmation in Electron.

## Complexity and uncertainty review

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 3/5 | Known AppState, session application, adapter, probe and HTTP callers; reuse them |
| Blast radius | 5/5 | Shared inspection/mutation and attention flows; retain full existing tests and direct interface coverage |
| State changes | 3/5 | Existing disk writes/revocation exercised; no new state or storage format |
| Reversibility | 2/5 | Source/ADR revert without migration; do not invent rollback of an already-published config |
| Uncertainty | 3/5 | Sequencing inspected; verify race and partial-failure outcomes before claiming equivalence |

Total: **16/25**, carried from the independent design review. The review's
inspection-side-effect finding is resolved in the approved design.

Least confidence: cancellation/partial failures between external writes and
revocation. Mitigation: retain the synchronous sequence, existing snapshots,
error precedence and list collection order. Project blind spot: inspection is
not pure. Mitigation: document its demotion side effect on the caller methods,
assert it through that interface, and keep readiness queries free of revocation.

Scope/simplicity: one module extraction, no generic new traits, no tool-adapter
cleanup. Clarity: request/result types and failure categories below are explicit.
Verification: baseline, direct-interface tests, transport tests and full Rust
checks. Plan quality: **Ready for execution** after independent review and resolution
of ADR sequencing and failure-test coverage findings (2026-10-10).

## Mandatory requirement coverage

All rows are binding acceptance criteria; Task 1 delivers the Rust changes and
the coordinator delivers the coupled documentation and PR workflow.

| Requirement | Delivery | Verification |
| --- | --- | --- |
| Typed application seam owns integration orchestration, without HTTP dependencies or duplicated state | Task 1 interface, private helpers, direct caller migration | Direct-interface tests; searches for Axum/HTTP types and caller dependencies |
| Preserve legacy/grouped outcomes, messages, serialization and strict revision/body contract | Task 1 typed errors and thin handlers | Retained HTTP status/body, malformed request, stale revision and adapter-error tests |
| Keep locks outside probe awaits, revalidate workspace/definition/revisions and preserve cache behavior | Task 1 moves existing private sequences | Existing slow-probe, switch/edit, revision and cache tests migrated to application seam |
| Preserve inspection demotion, tuple/source handling, list collection order and revocation-error precedence | Task 1 readiness/snapshot/finalization and list implementation | Direct config/authority assertions; list/failure precedence tests using deterministic private test hooks at the actual revocation invocation |
| Preserve mutation revocation timing and partial-write semantics, with no new await before finalization | Task 1 mutation helpers and finalization | Required-revocation tests plus diff review of synchronous write/snapshot/finalization sequence |
| Cleanup keeps ownership/selection/diagnostics and adds no new revocation policy | Task 1 cleanup and caller migration | Shared-consumer, foreign/ambiguous ownership and cleanup-result tests |
| Launch readiness stays synchronous, conservative and without revocation | Task 1 launch caller and method | Direct readiness/authority test and unchanged launch contract review |
| Keep adapter/reporters, auth, confirmation, public schema and unrelated subsystems unchanged | Task 1 limits file ownership and reuses adapters | Scope diff; router auth tests/review; existing adapter/reporter suites |
| Preserve architecture decision and document application ownership before code | Coordinator ADR 0030 amendment and architecture/README updates | ADR review, doc-check and edited-link validation |
| Demonstrate baseline, seam failure, green focused/full Rust checks, formatting and clean diff | Task 1 test-first seam, migrated tests and report | Recorded command outputs and counts; build/test/fmt/diff checks |
| Independent review covers current code and findings are resolved | Task review and final medium code review | Review artifacts identify base/head and disposition; fresh review after code fixes |
| Issue, recommendation and PR reach verified disposition through authorized paths | Coordinator #816/Taskmaster tie-off/PR workflow | PR links issue/recommendation; required checks; terminal PR status; guarded owned-worktree cleanup |

## Ownership

The implementation worker owns all Rust source/test changes listed in Task 1.
The coordinating agent owns the design/plan, ADR 0030 amendment, architecture
reference and README pointer. They do not edit one another's owned files or
commit concurrently. Other sessions are active; leave their work untouched.

### Task 1: Extract and verify the complete integration operation

**Pre-implementation gate:** The coordinator completes and reviews the dated
ADR 0030 ownership amendment and architecture/README updates before the worker
writes Rust implementation code. Baseline tests and read-only preparation may
precede this gate.

**Worker files:**
- Create: `crates/orkworksd/src/harness_integration_application.rs`.
- Modify: `crates/orkworksd/src/main.rs` (module registration only).
- Modify: `crates/orkworksd/src/http/integration_handlers.rs` (thin transport and focused transport tests).
- Modify: `crates/orkworksd/src/http/harness_handlers.rs` and `crates/orkworksd/src/http/session_handlers.rs` (cleanup calls and result type imports).
- Modify: `crates/orkworksd/src/session_application.rs` (launch-readiness call only).
- Add a sibling application test file if moving orchestration tests there makes the module easier to read.

**Consumes:** Existing `Arc<AppState>`, `IntegrationKey`, `IntegrationStatus`,
`IntegrationError`, `HarnessDocumentRevision`, `ResolvedHarness`, integration
probe cache, adapters and `SessionApplication` revocation methods.

**Produces:** A concrete application interface with these operations:

```rust
HarnessIntegrationApplication::new(state: Arc<AppState>) -> Self
async fn inspect(&self, target: IntegrationTarget)
    -> Result<IntegrationInspection, IntegrationApplicationError>
async fn mutate(&self, request: IntegrationMutationRequest)
    -> Result<IntegrationInspection, IntegrationApplicationError>
async fn list_workspace(&self)
    -> Result<Vec<GroupedIntegrationStatus>, IntegrationApplicationError>
async fn reconcile_unreferenced(&self, keys: BTreeSet<IntegrationKey>,
    expected_workspace_path: Option<PathBuf>) -> IntegrationCleanupResponse
fn prompt_attention_launch_ready(&self, harness_id: &str, executable: &str) -> bool
```

`IntegrationTarget` selects `Harness(String)` or `Group(IntegrationKey)`.
`IntegrationInspection` holds `Harness(IntegrationStatus)` or
`Group(GroupedIntegrationStatus)`. `IntegrationMutation` selects Install,
Repair or Uninstall. `IntegrationMutationRequest` has a legacy Harness variant
carrying ID/operation and a Group variant carrying key/operation plus a required
`IntegrationRevisionExpectation`. That expectation retains current document,
active-selection and optional workspace-path fields. Existing grouped/cleanup
serialization is unchanged. Errors carry domain facts/current revisions;
no Axum types or HTTP codes occur in the application implementation.

- [x] Establish baseline with `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::integration_handlers::tests -- --test-threads=1`. Expected: existing tests pass. Report any failure before classifying its cause; do not silently change unrelated behavior.
- [x] Before implementation, add a direct-interface test demonstrating the new seam. Reuse the existing fresh-workspace fixture and literal expected state:

```rust
let app = HarnessIntegrationApplication::new(state);
let result = app.inspect(IntegrationTarget::Harness("claude-code".into()))
    .await.unwrap();
let IntegrationInspection::Harness(status) = result else {
    panic!("expected harness status");
};
assert_eq!(status.registration, IntegrationRegistration::Absent);
assert!(!status.enabled);
```

  The fixture initializes a temporary Git workspace with Claude settings ignored,
  uses the existing FakeHome guard and `test_app_state_with_workspace`, and does
  not invoke an installed coding agent. The break this test catches is failure
  to expose an operation returning the real observed state through the new seam.
  Record the expected pre-extraction failure, then implement. Preserve existing
  behavior tests while refactoring rather than inventing intentional behavior
  failures for this structural change.
- [x] Move existing probe/revalidation/context construction, readiness/revocation,
  grouped projection, list and cleanup behavior behind the interface. Keep
  private operation callbacks private. Replace HTTP responses in the moved
  implementation with typed errors, keeping current distinctions/messages.
  Move reporter resolution into the application implementation. Retain existing
  callbacks/locks internally where necessary; avoid a generic replacement framework.
- [x] Rewrite handler entrypoints to parse requests, construct typed requests,
  call the application and map results to the existing protocol. Keep strict JSON
  parsing and mutation authorization routing. Rewire cleanup and launch-readiness
  callers directly, without adding forwarding application functions in `http`.
- [x] Migrate orchestration scenarios to direct application tests: fresh/installed/
  drifted/ambiguous state, shared consumers, cleanup, trust/version thresholds,
  cache reuse/invalidation, stale revisions and workspace/definition changes
  during a slow probe, unrelated-hook preservation, slow-probe responsiveness
  and current-generation fencing. Keep focused HTTP tests for malformed requests,
  authorization, exact status/body mapping, and legacy/grouped error distinction.
  Keep existing adapter/reporter tests intact.
- [x] Add or migrate direct tests for inspection demotion and prompt tuple/source
  handling, no configuration mutation during inspection, readiness without
  revocation, required revocation after mutation, and list/error precedence.
  Reuse deterministic local fixtures or private test seams at actual fallible
  dependencies. For deterministic failure coverage, use private, per-instance,
  test-only hooks at the actual snapshot-revocation invocation: record attempts,
  call the real dependency, then force its returned failure where needed. A
  private test-only hook before a later grouped revalidation may change the
  actual workspace/definition to trigger the existing conflict path. No global
  mutable hooks or production dependency framework. Require separate tests for inspect
  revocation failure overriding successful status and overriding adapter failure,
  list failure overriding later revalidation failure, all
  captured list revocations attempted in order despite an earlier failure, and
  later revalidation failure when finalization succeeds. Assert real resulting
  config/status/authority; avoid a mock application implementation. Missing
  required coverage blocks completion rather than becoming a review concern.
- [x] Run the new application tests and retained HTTP tests. Run full Rust build,
  full Rust tests, formatting check and `git diff --check`. Expected: all pass;
  no new production warnings. Record exact commands, counts and output.
- [x] Self-review for scope and compatibility. Search for application callers
  reaching into `http::integration_handlers`; expect no orchestration dependencies.
  Search the new application module for Axum/HTTP types; expect none. Check
  router authorization is unchanged. Verify helper-test moves do not duplicate
  an entire suite at both seams.
- [x] Commit only worker-owned Rust files after verification; write the task report
  with baseline/red/green evidence, full checks, changed files and concerns.

Coordinator pre-implementation work for Task 1: amend ADR 0030's ownership in a dated section,
retaining its accepted decision; update the architecture module list and README
pointer. Review user-facing docs: no change needed if behavior equivalence holds.
Run `rtk proxy bash scripts/doc-check.sh` and link validation on edited docs.

## Review and completion

The independent plan review must resolve findings before Task 1 starts. After
Task 1, run a spec/quality review over its full diff and fix verified findings.
Run a final `/code-review medium` at the current code head because the extraction
coordinates concurrency and lifecycle. Record SHA, effort and finding disposition
in the PR. Verify any matching live Taskmaster recommendation and tie it off
through the authenticated sidecar protocol after verification and before merge.

Open one PR for #816, follow `babysitting-pull-requests`, vet comments and required
checks, and use the repository's maintainer merge path only after all gates pass.
Clean up only this owned worktree after merge via `scripts/finish-pr.sh`. A bounded
external gate must be reported explicitly if the PR cannot reach a terminal state.

## Execution evidence (2026-10-10)

Task 1 is implemented and independently approved after adding successful
inspection demotion/source-tuple checks and a grouped post-probe revision race
test. The application suite has 31 tests; five focused HTTP tests retain the
transport contract. Adapter tests remain unchanged.

The branch was rebased cleanly onto current main before consolidated
verification. `rtk proxy bash scripts/verify-repo.sh` passed Rust formatting,
build and tests, reporter tests, desktop type checking/tests/build, docs build,
diff/doc currency checks and worktree reporting. The excluded implementation
plan link uses a repository URL so the published design does not produce a dead
link. Four existing Rust dead-code warnings and existing bundle-size warnings
remain; this extraction adds none. Other owners' stale worktrees were reported
and left untouched.

Final current-head code review, PR checks and merge disposition are recorded
in the pull request for #816.
