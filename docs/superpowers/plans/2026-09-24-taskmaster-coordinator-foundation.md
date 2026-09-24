# Taskmaster Coordinator Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first safe coordinator slice: immutable root-plan domain types, canonical digests, server-owned approval records, hard capability denials, and crash-safe persistence/lifecycle primitives without launching child sessions.

**Architecture:** Add a focused coordinator domain module under Taskmaster. Keep plan revisions and approvals as immutable durable records, while capability bearer values remain process-local and are never persisted. Expose no launch or child-session API in this slice; later graph/budget/launcher work will consume the validated types through separate plans and PRs.

**Tech Stack:** Rust 2021, serde/serde_json, sha2/hex, existing atomic file-replacement and Taskmaster persistence patterns, Rust unit tests.

**Spec:** `docs/superpowers/specs/2026-09-24-taskmaster-bounded-coordinator-design.md`

## Global Constraints

- This slice does not authorize coordinator code that launches children, child-session APIs, recursive orchestration, or autonomous runtime work.
- Design-gate approval and root-plan approval are distinct; only a user-approved exact plan revision may become active.
- Plan and evidence digests are server-generated from versioned canonical bytes; caller/model-supplied digests are rejected.
- Git mutation, credentials, permission changes, destructive commands, merge approval, scope expansion, provider substitution, and implicit delegation are hard-denied for every role and grant path.
- Existing Taskmaster v1 recommendation lifecycle and single-active-context rules remain unchanged.
- Bearer capability material is process-local or channel-bound, never persisted, logged, serialized into prompts, or returned in reports.
- Malformed, unknown-field, stale-revision, cross-workspace, digest-mismatched, and illegal-transition inputs fail closed.
- Use TDD for Rust behavior: write a focused failing test, run it, implement the smallest change, rerun the focused test, then run crate formatting/tests before each task commit.
- Use the existing Rust-sidecar validation commands: `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`, `cargo test --manifest-path crates/orkworksd/Cargo.toml`.

---

## Scope decomposition

The full design covers independent subsystems and is intentionally split:

1. This plan: coordinator foundation domain, immutable approval binding, hard denials, and persistence.
2. Follow-up plan: graph validation, canonical resource scopes, budget reservations, and idempotency records.
3. Follow-up plan: authenticated child leases, launcher mutation fences, process-tree cancellation, recovery, and redacted audit events.
4. Follow-up plan: user-facing root-plan review, escalation decisions, and desktop API/UX.

Each later plan must cite this design and this foundation's public types. No later plan may add launch behavior to this PR.

## File map

- Create `crates/orkworksd/src/taskmaster/coordinator.rs`: coordinator domain types, canonicalization, digest binding, capability lattice, and lifecycle predicates.
- Create `crates/orkworksd/src/taskmaster/coordinator_store.rs`: bounded atomic persistence for plan revisions and approval records.
- Modify `crates/orkworksd/src/taskmaster/mod.rs`: register the two coordinator modules and re-export only the domain/store interfaces needed by later Taskmaster code.
- Create `crates/orkworksd/src/taskmaster/coordinator_tests.rs`: focused integration-style unit tests for the foundation module and store.
- Modify `crates/orkworksd/src/main.rs`: none in this slice; no route, AppState field, or runtime launch path is added.
- Modify `docs/superpowers/specs/2026-09-24-taskmaster-bounded-coordinator-design.md`: none; implementation must conform to the reviewed design already committed.

---

### Task 1: Add immutable coordinator domain types and canonical digests

**Files:**
- Create: `crates/orkworksd/src/taskmaster/coordinator.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs:1-12`
- Create: `crates/orkworksd/src/taskmaster/coordinator_tests.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs` to register `#[cfg(test)] mod coordinator_tests;`

**Interfaces:**
- Produces `PlanRevision`, `PlanNode`, `WorkspaceChangeSubject`, `PlanEvidence`, `PlanApproval`, `PlanStatus`, `CapabilityKind`, `CapabilityRequest`, `CapabilityDecision`, `CoordinatorError`.
- Produces `canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CoordinatorError>`.
- Produces `sha256_hex(bytes: &[u8]) -> String`.
- Produces `PlanRevision::compute_plan_digest(&self) -> Result<String, CoordinatorError>` and `PlanRevision::compute_evidence_digest(&self) -> Result<String, CoordinatorError>`.
- Produces `PlanRevision::validate(&self) -> Result<(), CoordinatorError>` and `PlanApproval::validate_against(&self, plan: &PlanRevision) -> Result<(), CoordinatorError>`.
- Produces `PlanStatus::allows_transition(self, next: PlanStatus) -> bool` and `CapabilityKind::is_hard_denied(self) -> bool`.

- [ ] **Step 1: Write failing tests for canonical bytes and digest binding**

Add tests that:
  - serialize two semantically identical objects with different JSON object insertion order and assert identical canonical bytes and SHA-256 digest;
  - preserve array order while sorting object keys;
  - reject non-finite JSON numbers if the canonicalizer encounters them;
  - reject a caller-supplied plan or evidence digest that differs from the server-computed digest;
  - reject a plan whose workspace change subject is inconclusive.

Example test shape:

```rust
#[test]
fn canonical_json_sorts_objects_but_preserves_arrays() {
    let left = serde_json::json!({"b": 2, "a": [3, 1]});
    let right = serde_json::json!({"a": [3, 1], "b": 2});
    assert_eq!(
        canonical_json_bytes(&left).unwrap(),
        canonical_json_bytes(&right).unwrap()
    );
    assert_ne!(
        canonical_json_bytes(&serde_json::json!({"a": [1, 3], "b": 2})).unwrap(),
        canonical_json_bytes(&left).unwrap()
    );
}
```

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::coordinator_tests::canonical_json_sorts_objects_but_preserves_arrays`
Expected: FAIL because the coordinator module and canonicalizer do not exist.

- [ ] **Step 2: Write failing tests for plan/approval immutability and lifecycle**

Cover:
  - required node fields include task, success criteria, prompt/context digest, output contract, scope, role, provider allowlist, budgets, retry limit, concurrency class, and parent ID;
  - changing any approved field changes the computed plan digest;
  - `draft -> proposed -> active -> completed` is legal;
  - only `proposed -> active` with a matching approval is legal;
  - active plans cannot be edited in place;
  - `ready_for_user_review` is not represented as terminal plan success;
  - expired, revoked, failed, cancelled, and recovery-required plans cannot launch;
  - an approval with another instance, workspace, plan digest, evidence digest, or revocation generation is rejected.

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::coordinator_tests::approved_plan_rejects_digest_or_scope_changes`
Expected: FAIL because the domain types and predicates do not exist.

- [ ] **Step 3: Write failing tests for hard capability denials**

Cover every denial through both direct capability construction and parent grant requests:
  - Git mutation, credentials, permission changes, destructive commands, merge approval, provider substitution, and scope/budget/retry/concurrency expansion return a hard-denied error;
  - a non-default role cannot make a hard-denied kind grantable;
  - a capability request with an unknown or unbounded tool identifier fails;
  - a valid request records a structured reason, canonical scope, expected cost, and current capability digest without accepting child prose as authority.

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::coordinator_tests::hard_denied_capabilities_never_become_grantable`
Expected: FAIL because the capability lattice does not exist.

- [ ] **Step 4: Implement the minimal domain model and canonicalizer**

Implement:
  - serde structs with `#[serde(deny_unknown_fields)]` on persisted request/approval/node records;
  - bounded string/vector validators for IDs, prompts, scopes, tool identifiers, graph references, and output contracts;
  - recursive JSON canonicalization using sorted object keys and preserved array order;
  - SHA-256 hex digests computed only by the server;
  - explicit plan status and capability-kind enums;
  - validation that inconclusive workspace attribution cannot be approved;
  - immutable approval matching on instance, workspace, plan/evidence digest, expiry, and revocation generation.

Do not add a launch method, process spawn, HTTP route, or runtime mutation hook.

- [ ] **Step 5: Run focused tests and formatting**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::coordinator_tests`
Expected: PASS with all Task 1 coordinator domain tests passing.

Run: `cargo fmt --manifest-path crates/orkworksd/Cargo.toml`
Expected: exit 0 and only the new/modified coordinator files are formatted.

Run: `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`
Expected: PASS.

- [ ] **Step 6: Commit Task 1**

```bash
git add crates/orkworksd/src/taskmaster/coordinator.rs \
  crates/orkworksd/src/taskmaster/coordinator_tests.rs \
  crates/orkworksd/src/taskmaster/mod.rs
git commit -m "feat: add bounded coordinator domain model"
```

---

### Task 2: Add atomic plan and approval persistence

**Files:**
- Create: `crates/orkworksd/src/taskmaster/coordinator_store.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs` to register `coordinator_store`
- Modify: `crates/orkworksd/src/taskmaster/coordinator_tests.rs`

**Interfaces:**
- Produces `CoordinatorStore::open(root: PathBuf) -> Result<Self, CoordinatorStoreError>`.
- Produces `CoordinatorStore::put_proposed(&self, plan: &PlanRevision) -> Result<(), CoordinatorStoreError>`.
- Produces `CoordinatorStore::get(&self, plan_id: &str, revision: u64) -> Result<Option<StoredPlan>, CoordinatorStoreError>`.
- Produces `CoordinatorStore::activate(&self, approval: &PlanApproval) -> Result<StoredPlan, CoordinatorStoreError>`.
- Produces `CoordinatorStore::transition(&self, plan_id: &str, revision: u64, expected_digest: &str, next: PlanStatus) -> Result<StoredPlan, CoordinatorStoreError>`.
- Produces `CoordinatorStore::recover(&self) -> Result<(), CoordinatorStoreError>`.
- Persists only opaque capability IDs/digests and revocation generations; it never persists bearer token material or live leases.

- [ ] **Step 1: Write failing tests for bounded durable storage**

Test:
  - proposed plans persist under a dedicated coordinator directory and round-trip with exact digests;
  - unknown fields and oversized records are rejected without replacing the previous file;
  - duplicate plan/revision writes with a different digest return a stale/conflict error;
  - missing plans return `None`;
  - malformed JSON fails closed and does not get silently deleted;
  - temporary files and an interrupted publication are recovered deterministically;
  - approval activation requires a matching plan digest and current proposed status;
  - completed/failed/cancelled/revoked/recovery-required plans reject later activation or launch-state transitions.

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::coordinator_tests::coordinator_store_rejects_stale_activation`
Expected: FAIL because the store does not exist.

- [ ] **Step 2: Implement the store using existing atomic replacement patterns**

Use the existing Taskmaster store conventions:
  - create `coordinator/plans/` beneath the workspace metadata root;
  - use server-generated plan IDs and revision filenames after validating safe IDs;
  - write bounded JSON to a temporary file, sync it, atomically publish it, and sync the parent directory;
  - preserve malformed or conflicting files for diagnosis;
  - validate all records on open and before mutation;
  - use expected digest checks to reject stale updates;
  - recover only incomplete file publication; never revive a bearer capability, lease, process, or launch.

- [ ] **Step 3: Add store tests for crash and stale-action boundaries**

Add tests that simulate:
  - a temp file left by a failed write;
  - a target externally changed between read and transition;
  - an approval for a superseded revision;
  - a recovery-required record after a simulated shutdown marker;
  - a late transition against a revoked plan.

Each test must assert the old durable record remains available and the attempted mutation cannot advance state.

- [ ] **Step 4: Run focused store tests and formatting**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::coordinator_tests`
Expected: PASS.

Run: `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`
Expected: PASS.

- [ ] **Step 5: Commit Task 2**

```bash
git add crates/orkworksd/src/taskmaster/coordinator_store.rs \
  crates/orkworksd/src/taskmaster/coordinator_tests.rs \
  crates/orkworksd/src/taskmaster/mod.rs
git commit -m "feat: persist coordinator plan approvals safely"
```

---

### Task 3: Verify the foundation and hand off to the next implementation plan

**Files:**
- Modify: `docs/superpowers/plans/2026-09-24-taskmaster-coordinator-foundation.md`: record verification evidence only if implementation changes require a plan correction.
- No launch/API files may be added by this task.

- [ ] **Step 1: Run the full Rust validation suite**

Run:
```bash
cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Expected: formatting passes and the full suite reports zero failures. If a pre-existing unrelated failure remains, record its exact test name and output in the handoff; do not mask or rewrite it.

- [ ] **Step 2: Audit the diff against the reviewed design**

Check:
```bash
git diff --check
rg -n 'spawn|Command::|launch|/taskmaster.*coordinator' crates/orkworksd/src/taskmaster/coordinator.rs crates/orkworksd/src/taskmaster/coordinator_store.rs
```

Expected: no process-launch or child-session API was introduced by the foundation slice.

- [ ] **Step 3: Commit verification evidence**

If the plan required no edits, leave it unchanged. Commit only implementation files from Tasks 1 and 2; do not create a documentation-only follow-up commit for a passing check.

- [ ] **Step 4: Handoff boundary**

Stop this plan after the foundation PR is opened. The next plan must separately specify graph/resource scope, budget/idempotency, launcher/recovery, and user escalation UI before those systems are implemented.

