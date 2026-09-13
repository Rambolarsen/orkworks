# Taskmaster Recommendation Rollups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Persistently combine semantically related proposed Taskmaster workflow recommendations into one bounded, user-actionable rollup while preserving exact evidence families and legacy history.

**Architecture:** Add structured problem identity at observation ingress, retain the existing deterministic exact-family evaluator, and place a bounded semantic rollup pass above it. Rollup parent/member changes are applied as one recoverable recommendation-graph transaction; the API exposes parents as actionable cards and members as audit-only history.

**Tech Stack:** Rust sidecar with serde, Tokio/Axum, filesystem-backed JSON/NDJSON metadata, React/TypeScript desktop UI, pnpm, Cargo tests.

**Spec:** `docs/superpowers/specs/2026-09-13-taskmaster-recommendation-rollups-design.md`

## Global Constraints

- Preserve v1 observation records and v1 fingerprints; missing `problemArea` is a legacy input and is never silently discarded.
- V2 fingerprints are sidecar-owned and use the specified NFKC/lowercase/whitespace canonicalization and SHA-256 format.
- Exact recommendations remain usable without a configured or available Taskmaster model.
- Rollups combine only supplied proposed exact-family snapshots, use bounded input/output, and reject invalid or overlapping clusters as a whole.
- In v1, all members of a rollup must have the same target surface.
- Parent/member transitions use a recoverable transaction and maintain the graph invariants after startup recovery.
- No autonomous session focus, terminal input, file edit, Git action, or recommendation acceptance.
- Use TDD: write and run a failing test before each production behavior change.
- Rust validation is `cargo build --manifest-path crates/orkworksd/Cargo.toml`, `cargo test --manifest-path crates/orkworksd/Cargo.toml`, and `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- Desktop validation uses pnpm and runs from `apps/desktop/`: `npx tsc --noEmit` and the repository's Node test command.
- All changes remain on `taskmaster-recommendation-rollups`; do not push, merge, or modify another worktree.

---

### Task 1: Record the recommendation identity and lifecycle decision

**Files:**
- Create: `docs/adr/0057-taskmaster-recommendation-rollups.md`
- Modify: `docs/adr/README.md`
- Modify: `specs/taskmaster.md`
- Modify: `docs/agents/architecture.md`
- Test: repository documentation check, if available

**Interfaces:**
- Produces the accepted architectural contract referenced by all later tasks.
- Does not change runtime behavior.

- [ ] **Step 1: Write the ADR.** Record that exact v1 evidence families remain deterministic audit units, structured `problemArea` creates v2 identity for new records, bounded model output may create one parent over proposed exact families, and the sidecar owns validation, identity, lifecycle, and recoverable persistence.
- [ ] **Step 2: Update the ADR index.** Add ADR 0057 with status `proposed` and the exact title to the historical table.
- [ ] **Step 3: Update `specs/taskmaster.md`.** Replace the v1-only prohibition on combining different fingerprints with the two-layer contract. Document `rolled_up`, parent/member fields, list filtering, terminal behavior, and the existing exact thresholds.
- [ ] **Step 4: Update `docs/agents/architecture.md`.** Add the observation `problemArea` field, v2 fingerprint, recommendation graph transaction/recovery, and API/list semantics to the metadata protocol section.
- [ ] **Step 5: Run the documentation check.** Run `bash scripts/doc-check.sh` from the repository root; if it reports an environment-only limitation, run `git diff --check` and record the limitation.
- [ ] **Step 6: Commit.** `git add docs/adr/0057-taskmaster-recommendation-rollups.md docs/adr/README.md specs/taskmaster.md docs/agents/architecture.md && git commit -m "docs: record Taskmaster rollup architecture"`

### Task 2: Add structured observation identity with legacy compatibility

**Files:**
- Modify: `crates/orkworksd/src/workflow_observations.rs`
- Modify: `crates/orkworksd/src/peon.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/http/workflow_observation_handlers.rs`
- Test: module tests in `workflow_observations.rs` and `peon.rs`

**Interfaces:**
- `ObservationCandidate` gains `problem_area: Option<String>`.
- `WorkflowObservation` gains `problem_area: Option<String>` with serde default.
- `record_observation` persists v1 identity when the field is absent and v2 identity when it is explicit.
- Peon JSON accepts an omitted field and preserves the existing best-effort candidate parsing behavior.

- [ ] **Step 1: Add failing observation tests.** Cover v2 canonicalization for NFKC/lowercase/Unicode whitespace, punctuation retention, bounded SHA-256 fingerprint format, legacy v1 fallback, optional persisted field deserialization, and explicit empty/control/overlong handling.
- [ ] **Step 2: Run the focused tests and confirm they fail for the missing field/identity behavior.** Run `cargo test --manifest-path crates/orkworksd/Cargo.toml workflow_observations`.
- [ ] **Step 3: Implement the smallest identity change.** Add the optional field, canonicalizer, v2 fingerprint helper, and preserve v1 behavior for omitted fields. Keep all existing description/evidence validation and idempotency hashes intact; include `problem_area` in the v2 payload hash so changing it is not treated as an idempotent replay.
- [ ] **Step 4: Add failing Peon and agent-ingress tests.** Verify old candidate JSON without `problemArea` is accepted, explicit malformed candidates are isolated/rejected as specified, and the sidecar receives the field from both Peon and authenticated agent paths.
- [ ] **Step 5: Implement ingress plumbing.** Add `problemArea` to the Peon prompt/schema, candidate parsing, session application, and HTTP request mapping. Do not make malformed optional workflow candidates discard the core Peon inference.
- [ ] **Step 6: Run focused tests and format.** Run the observation/Peon tests and `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- [ ] **Step 7: Commit.** `git add crates/orkworksd/src/workflow_observations.rs crates/orkworksd/src/peon.rs crates/orkworksd/src/session_application.rs crates/orkworksd/src/http/workflow_observation_handlers.rs && git commit -m "feat: add structured Taskmaster observation identity"`

### Task 3: Add recommendation schema and pure rollup validation

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/mod.rs`
- Create: `crates/orkworksd/src/taskmaster/rollup.rs`
- Modify: `crates/orkworksd/src/taskmaster/evaluator/identity_tests.rs` if required by type construction
- Test: `crates/orkworksd/src/taskmaster/rollup.rs` and existing Taskmaster identity tests

**Interfaces:**
- `RecommendationStatus` gains `RolledUp`.
- `Recommendation` gains defaulted `rollup_member_ids: Vec<String>`, `rollup_member_dedupe_keys: Vec<String>`, `rollup_generation: Option<u64>`, and `rolled_up_by: Option<String>`.
- `WorkflowObservationEvidence` gains defaulted `problem_area: Option<String>`.
- `RollupFamilySnapshot` contains the exact recommendation ID, dedupe key, target surface, bounded problem/context fields, representative evidence, source-session IDs, and an evidence snapshot hash.
- `validate_rollup_clusters` accepts supplied snapshots and parsed model clusters and returns either all validated clusters or a validation error; it rejects unknown IDs, empty/duplicate clusters, fewer than two members, more than eight clusters, out-of-bound sizes, overlapping families, mismatched target surfaces, and generated text outside bounds.
- `stable_rollup_id` hashes sorted member recommendation IDs only.

- [ ] **Step 1: Write failing schema and validation tests.** Cover legacy recommendation JSON defaults, `rolled_up` serialization, parent/member invariants, stable order-independent IDs, valid same-target clusters, and every rejection rule.
- [ ] **Step 2: Run the focused tests and confirm the new status/fields/helpers are absent.** Run `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::rollup`.
- [ ] **Step 3: Implement defaulted serde fields and the pure rollup module.** Keep terminal status deserialization strict; return diagnostics/quarantine at the store boundary for malformed relationship fields.
- [ ] **Step 4: Implement deterministic snapshot bounds.** Select earliest/latest/highest-impact distinct evidence, cap 32 families/96 observations/eight source IDs per family/128 KiB input, and cap parent evidence projection at 64 entries/128 KiB.
- [ ] **Step 5: Run the focused tests and Rust formatter.** Confirm the tests pass and run `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- [ ] **Step 6: Commit.** `git add crates/orkworksd/src/taskmaster/mod.rs crates/orkworksd/src/taskmaster/rollup.rs crates/orkworksd/src/taskmaster/evaluator/identity_tests.rs && git commit -m "feat: define Taskmaster rollup schema"`

### Task 4: Make parent/member persistence recoverable

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/store.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs` for transaction-facing types, if needed
- Test: `crates/orkworksd/src/taskmaster/store.rs`

**Interfaces:**
- `RecommendationStore::apply_rollup_transaction(expected, parent, members)` atomically commits a complete parent/member graph or returns an error without hiding members.
- `RecommendationStore::open` recovers unfinished rollup manifests before reads are served.
- Existing single-record `put`, dismiss, accept, and completion behavior remains compatible.

- [ ] **Step 1: Write failing store tests.** Cover a complete parent/member transition, no partial graph after injected failure before commit, recovery of committed and uncommitted manifests, stale expected-file hash rejection, legacy JSON defaults, and graph invariant validation.
- [ ] **Step 2: Run the focused tests and confirm failure.** Run `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::store`.
- [ ] **Step 3: Implement the transaction manifest.** Stage replacement JSON and durable old-content backups under a transaction-only directory, fsync staged files and the manifest, mark commit, publish replacements, fsync the recommendations directory, and remove the manifest only after completion.
- [ ] **Step 4: Implement startup recovery.** Roll back uncommitted transactions, finish committed transactions, validate the resulting graph before reads, and expose a diagnostic/unavailable read result if neither complete graph can be established. Ignore transaction files during normal listing.
- [ ] **Step 5: Implement graph-aware lifecycle helpers.** Supersede changed proposed parents, update current `rolledUpBy`, retain terminal parent/member history, create new exact-family generations for post-terminal evidence, and prevent session cleanup from deleting a parent/member in isolation.
- [ ] **Step 6: Run focused tests and all sidecar tests.** Run the store tests, `cargo test --manifest-path crates/orkworksd/Cargo.toml`, and `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- [ ] **Step 7: Commit.** `git add crates/orkworksd/src/taskmaster/store.rs crates/orkworksd/src/taskmaster/mod.rs && git commit -m "feat: persist Taskmaster rollups transactionally"`

### Task 5: Integrate the bounded rollup model pass

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/evaluator.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/taskmaster/runtime.rs` only where existing scheduler/cache state must expose the rollup token
- Test: `crates/orkworksd/src/taskmaster/evaluator/` and `crates/orkworksd/src/session_application.rs`

**Interfaces:**
- The rollup pass uses the existing Taskmaster scheduler/provider reservation and receives a `RollupEvaluationToken` containing workspace instance, generation, provider/model identity, prompt version, and family snapshot hash.
- The model response is parsed as bounded clusters of supplied family IDs plus title/summary text.
- Applying a validated result re-reads the workspace, exact recommendations, provider identity, and evidence snapshot under the workspace lock before calling `apply_rollup_transaction`.

- [ ] **Step 1: Write failing evaluator tests.** Cover prompt bounds and untrusted-data labeling, same-target clusters, invalid/overlapping model output, stale instance/generation/snapshot rejection, provider/budget unavailability fallback, same-set update-in-place, changed-set supersession, and idempotent rerun.
- [ ] **Step 2: Run the focused evaluator tests and confirm failure.** Run `cargo test --manifest-path crates/orkworksd/Cargo.toml taskmaster::evaluator`.
- [ ] **Step 3: Implement snapshot/token construction and bounded prompt generation.** Do not include raw terminal replay; serialize only supplied exact-family data and clearly label all evidence/prose as untrusted reference data.
- [ ] **Step 4: Implement response validation/application.** Reject the complete response on any invalid cluster, normalize ordering, compute sidecar-owned derived counts/evidence/sessions, and persist parent/member state through the transactional store.
- [ ] **Step 5: Integrate scheduling and failure isolation.** Reuse minimum interval, daily reservation, provider identity, workspace invalidation, and cache identity. If the rollup call fails or is unavailable, preserve exact recommendations and observations unchanged.
- [ ] **Step 6: Run sidecar tests and formatter.** Run `cargo test --manifest-path crates/orkworksd/Cargo.toml` and `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- [ ] **Step 7: Commit.** `git add crates/orkworksd/src/taskmaster/evaluator.rs crates/orkworksd/src/session_application.rs crates/orkworksd/src/taskmaster/runtime.rs crates/orkworksd/src/taskmaster/evaluator && git commit -m "feat: evaluate bounded Taskmaster recommendation rollups"`

### Task 6: Expose rollups through HTTP and the desktop card

**Files:**
- Modify: `crates/orkworksd/src/http/taskmaster_handlers.rs`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/components/RecommendationsPanel.tsx`
- Modify: the existing desktop recommendation tests covering API/card behavior
- Test: Rust HTTP tests and `apps/desktop/tests/`

**Interfaces:**
- List returns active rollup parents and proposed exact families without an active parent; `rolled_up` members remain available from detail only.
- Detail responses expose member IDs/dedupe keys, `rollupGeneration`, `rolledUpBy`, and optional `problemArea` evidence.
- Actions against a `rolled_up` member return conflict without side effects.
- The card shows combined counts/sessions/evidence and sends a bounded, delimited fix prompt containing parent/member IDs.

- [ ] **Step 1: Write failing Rust HTTP and TypeScript tests.** Cover list filtering, detail fields, conflict on hidden members, status parsing, card rendering, and prompt metadata/safety text.
- [ ] **Step 2: Run the focused tests and confirm failure.** Run the existing sidecar HTTP test filter and from `apps/desktop/` run `node --experimental-strip-types --test tests/api.test.ts`.
- [ ] **Step 3: Implement server/API behavior.** Add serialization fields, filter normal list output, preserve audit detail access, and leave existing explicit-action routes unchanged except for rejecting hidden members.
- [ ] **Step 4: Implement desktop types and rendering.** Add `rolled_up`, rollup fields, optional `problemArea`, parent/member evidence presentation, and the existing Dismiss/Fix with AI actions without adding multi-terminal UI.
- [ ] **Step 5: Run desktop validation.** From `apps/desktop/`, run `npx tsc --noEmit` and `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`.
- [ ] **Step 6: Commit.** `git add crates/orkworksd/src/http/taskmaster_handlers.rs apps/desktop/src/api.ts apps/desktop/src/components/RecommendationsPanel.tsx apps/desktop/tests && git commit -m "feat: present Taskmaster rollup recommendations"`

### Task 7: Update migration fixtures, documentation, and run the full verification gate

**Files:**
- Modify: `docs/superpowers/specs/2026-09-13-taskmaster-recommendation-rollups-design.md` only if implementation reveals a contract discrepancy
- Modify: relevant Rust fixtures/tests and desktop tests
- Test: full Rust and desktop validation commands

**Interfaces:**
- No new production interface; this task verifies the complete contract across legacy, migration, lifecycle, failure, and prompt-safety paths.

- [ ] **Step 1: Add end-to-end fixture tests.** Start with v1 observations/recommendations, run the first rollup projection, verify terminal records stay untouched, verify parent/member graph recovery, and verify later evidence creates a new exact-family generation.
- [ ] **Step 2: Add failure-path tests.** Exercise provider timeout, invalid JSON, oversized input/output, stale result, session deletion/orphan scrub, and transaction interruption without losing exact evidence.
- [ ] **Step 3: Run the full verification gate.** Run `cargo build --manifest-path crates/orkworksd/Cargo.toml`, `cargo test --manifest-path crates/orkworksd/Cargo.toml`, `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`, `npx tsc --noEmit`, and the documented desktop Node tests.
- [ ] **Step 4: Run `git diff --check` and inspect the complete diff for scope violations.** Confirm no autonomous action, raw terminal replay, vector database, Git workflow, or multi-terminal UI was introduced.
- [ ] **Step 5: Commit any final test-only changes.** `git add crates/orkworksd apps/desktop docs && git commit -m "test: verify Taskmaster recommendation rollups"`
- [ ] **Step 6: Request the required low-effort code review before PR handoff.** Use the repository's explicit `/code-review low` gate for code changes, then address or document every finding.
