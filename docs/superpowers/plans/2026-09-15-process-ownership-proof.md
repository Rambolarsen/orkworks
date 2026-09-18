# Process Ownership Proof Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a cross-platform native fixture that proves or rejects crash-surviving ownership of sidecar, PTY, and inference descendants for issue #545, then audit the real launch seams before any recovery behavior is implemented.

**Architecture:** A standalone Rust fixture crate runs an Electron-like parent, an owner/supervisor, sidecar-shaped roots, adversarial descendants, and a foreign sentinel. The supervisor issues generation-bound one-use tickets, owns the native containment mechanism, independently observes process identity/liveness, and emits authenticated rendezvous and cleanup receipts. Platform adapters compare Windows Job Objects, macOS launchd/process supervision candidates, and portable Linux candidates against one shared adversarial matrix.

**Tech Stack:** Rust 2021, `std::process`, `serde`/`serde_json`, `libc`, `windows-sys`, Windows Job Objects, macOS `launchd`, Unix process/session primitives, Cargo integration tests, and shell/PowerShell process-control helpers only where the host platform requires them.

**Spec:** `docs/superpowers/specs/2026-09-15-process-ownership-proof-design.md`

## Global Constraints

- This work is investigation and fixture validation, not authorization to ship multi-workspace runtime changes before spec acceptance.
- Every process root is admitted through a supervisor-issued authenticated one-use ticket bound to generation, role, executable identity, and request nonce.
- Registration or containment failure holds the root before target execution and fails closed.
- Owner loss freezes admission, drains in-flight launches, and uses supervisor-owned five-second graceful plus five-second owned-termination phases.
- A missing, unreachable, malformed, stale, or ambiguous rendezvous is unresolved, never an empty generation.
- Persisted PIDs, executable names, working directories, and released metadata leases never prove ownership or process exit.
- The fixture must run on Windows and macOS, with portable Linux coverage.
- The existing `providers::windows_process::ProcessJob` helper is not evidence for PTY or native coding-tool containment.
- No unavailable-runtime cleanup, automatic recovery, replacement, or reconciliation code may be changed until the evidence and ADR 0056 update are complete.

---

### Task 1: Scaffold the standalone fixture and authenticated protocol

**Files:**
- Create: `crates/process-ownership-fixture/Cargo.toml`
- Create: `crates/process-ownership-fixture/src/main.rs`
- Create: `crates/process-ownership-fixture/src/protocol.rs`
- Create: `crates/process-ownership-fixture/tests/protocol.rs`
- Modify: `docs/superpowers/specs/2026-09-15-process-ownership-proof-design.md` only if the final fixture command or protocol names differ from the approved draft

**Interfaces:**
- `protocol::GenerationId = u64` identifies one supervisor generation.
- `protocol::Role` is the closed enum `{ Sidecar, Pty, Inference }`; target
  behavior names are test-driver inputs and are not accepted as ownership
  roles.
- `protocol::LaunchTicket { generation: GenerationId, role: Role, executable_identity: String, request_nonce: String, ticket_id: String }` is issued only by the supervisor and is rejected after one use.
- `protocol::RendezvousState` is the closed enum `{ Live, CompleteExit(CompleteExitReceipt), Unresolved(String) }`.
- `protocol::RendezvousRecord { generation: GenerationId, nonce: String, endpoint: String, state: RendezvousState }` is written by the supervisor; a persisted record can locate the live endpoint but cannot authenticate ownership without the live IPC challenge.
- `protocol::CompleteExitReceipt { generation: GenerationId, owned_processes: Vec<NativeIdentity>, observed_at_ms: u128 }` is emitted only after independent observation shows no owned process remains.
- `protocol::NativeIdentity` contains the platform-specific birth identity and diagnostic PID; the PID is never sufficient for equality.
- `protocol::FixtureCommand` and `protocol::FixtureReply` are newline-delimited JSON messages with a bounded 64 KiB message size; every request carries the generation, rendezvous nonce, and authentication tag except the initial `prepare` request.
- `protocol::PreparedGeneration { record: RendezvousRecord, endpoint_token: String }` is returned by `supervisor::prepare(generation)`; both values are supervisor-generated.

- [ ] **Step 1: Write the failing protocol tests**

  Add tests showing that a ticket with a wrong generation, role, executable identity, request nonce, or reused `ticket_id` is rejected, and that adoption rejects a missing, stale, malformed, or incomplete receipt.

  ```rust
  let ticket = issue_ticket(7, Role::Inference, "fixture-inference", "req-a");
  assert!(accept_ticket(ticket.clone()).is_ok());
  assert!(accept_ticket(ticket).is_err());
  assert!(adopt(RendezvousState::Live).is_err());
  assert!(adopt(RendezvousState::CompleteExit(incomplete_receipt())).is_err());
  ```

- [ ] **Step 2: Run the focused tests and verify failure**

  Run `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test protocol`.

  Expected: compilation or assertion failures because the fixture crate and protocol state machine do not exist yet.

- [ ] **Step 3: Implement the bounded protocol and state machine**

  Add strict JSON decoding with unknown-operation rejection, a supervisor-only secret for local IPC authentication, a generation check on every command, a consumed-ticket set, and an explicit `Unknown`/`Unresolved` result distinct from `Empty`. Keep diagnostic PIDs in replies but exclude them from ownership decisions.

- [ ] **Step 4: Run the focused tests and verify success**

  Run `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test protocol`.

  Expected: all protocol tests pass, including replay, cross-generation, malformed-receipt, and unavailable-owner cases.

- [ ] **Step 5: Commit the scaffold**

  ```bash
  git add crates/process-ownership-fixture docs/superpowers/specs/2026-09-15-process-ownership-proof-design.md
  git commit -m "test: scaffold process ownership proof fixture"
  ```

### Task 2: Implement supervisor-owned launch admission and independent observation

**Files:**
- Create: `crates/process-ownership-fixture/src/supervisor.rs`
- Create: `crates/process-ownership-fixture/src/observation.rs`
- Create: `crates/process-ownership-fixture/src/targets.rs`
- Create: `crates/process-ownership-fixture/tests/admission.rs`

**Interfaces:**
- `supervisor::Supervisor::prepare(generation) -> PreparedGeneration` creates the rendezvous state and starts no target.
- `Supervisor::issue_launch_ticket(role, executable_identity, request_nonce) -> LaunchTicket` issues one ticket for the prepared generation.
- `supervisor::LaunchSpec { executable: PathBuf, args: Vec<OsString>, behavior: TargetBehavior }` is assembled from fixture-owned executable paths and a closed behavior enum; callers cannot supply an arbitrary command string.
- `Supervisor::spawn(ticket, launch_spec: LaunchSpec) -> Result<NativeIdentity, SpawnError>` creates a paused/pre-exec root, records identity and containment, then releases execution.
- `supervisor::PausedRoot { identity: NativeIdentity, process_handle: OwnedProcessHandle }` is the only value that may be released after registration; a failed registration closes the handle without executing the target.
- `supervisor::SpawnError` distinguishes `InvalidTicket`, `AdmissionClosed`, `ContainmentFailed`, `IdentityUnavailable`, and `ObserverUnavailable`.
- `Supervisor::observe() -> ObservationSnapshot` reports roots, descendants, birth identities, containment membership, and exit state using supervisor/OS observations rather than target messages.
- `targets::TargetBehavior` implements `pty`, `inference`, `forked`, `new-group`, `daemonized`, `reparented`, and `silent`; `ready` and `children` messages are optional diagnostics and are intentionally untrusted.
- `targets::run(role, behavior)` executes only after the supervisor releases the registered root.
- `observation::is_complete(snapshot) -> bool` returns true only when every registered identity has independently exited and no unresolved survivor exists.

- [ ] **Step 1: Write the failing admission tests**

  Test that a target cannot execute before registration, that a registration failure leaves it paused, that an unregistered child is not admitted, and that owner loss closes the admission barrier before accepting late tickets.

  ```rust
  let prepared = start_supervisor(11);
  let ticket = prepared.issue(Role::Pty, "fixture-pty", "req-pty");
  prepared.inject_registration_failure_once();
  assert!(prepared.spawn(ticket).is_err());
  assert!(!prepared.target_marker("pty-executed").exists());
  prepared.owner_lost();
  assert!(prepared.issue(Role::Inference, "fixture-inference", "req-late").is_err());
  ```

- [ ] **Step 2: Run the focused tests and verify failure**

  Run `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test admission`.

  Expected: failures because no supervisor launch barrier or independent observation exists.

- [ ] **Step 3: Implement the launch gate**

  Use a platform adapter with the common sequence `create_paused_root → capture_native_identity → attach_containment → register_identity → release_exec`. Make the control endpoint non-inheritable. On any failure, terminate the paused root, discard the ticket, and return an error without running target code.

- [ ] **Step 4: Implement independent observation**

  Have the supervisor maintain the registered identity set and query the platform-native birth/liveness data. Treat target `ready` and `children` messages as diagnostics only. A missing observer result, PID reuse, or ambiguous identity produces `Unresolved` rather than an empty set.

- [ ] **Step 5: Run the focused tests and verify success**

  Run `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test admission`.

  Expected: all admission and independent-observation tests pass on the host platform.

- [ ] **Step 6: Commit the supervisor boundary**

  ```bash
  git add crates/process-ownership-fixture
  git commit -m "test: enforce generation-bound launch admission"
  ```

### Task 3: Validate the Windows Job Object implementation

**Files:**
- Create: `crates/process-ownership-fixture/src/platform/windows.rs`
- Create: `crates/process-ownership-fixture/tests/windows_job.rs`
- Modify: `crates/process-ownership-fixture/Cargo.toml`

**Interfaces:**
- `platform::OwnerDomain::create(generation) -> Result<Self, PlatformError>` creates a private Job Object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` and no breakaway allowance.
- `OwnerDomain::launch_paused(spec) -> Result<PausedRoot, PlatformError>` starts the root suspended, assigns it to the job, records the native process identity, and resumes only after registration.
- `OwnerDomain::terminate_owned() -> Result<(), PlatformError>` terminates the job and waits for the independently observed job completion.
- `OwnerDomain::attempt_breakaway() -> BreakawayResult` is a negative test; an unapproved successful breakaway fails the suite.

- [ ] **Step 1: Write the failing Windows tests**

  Cover normal cleanup, sidecar-first crash, PTY-like and inference descendants, nested descendants, breakaway rejection, non-inheritable supervisor handles, and a force-terminated Electron-like parent whose supervisor remains alive.

- [ ] **Step 2: Run the Windows tests and verify failure**

  Run `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test windows_job` on Windows.

  Expected: failures until the Job Object adapter and suspended launch sequence are implemented.

- [ ] **Step 3: Implement Job Object ownership**

  Use the Windows APIs through `windows-sys`. Keep the supervisor's Job Object handle alive after the sidecar exits, set child-process handles and the supervisor IPC endpoint non-inheritable, and reject any process that can execute before assignment and registration.

- [ ] **Step 4: Run the Windows tests and verify success**

  Run `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test windows_job` and record the exact host, architecture, build mode, and test command.

  Expected: all Windows rows pass, including the actual `TerminateProcess` parent-kill scenario.

- [ ] **Step 5: Commit the Windows adapter**

  ```bash
  git add crates/process-ownership-fixture
  git commit -m "test: validate Windows job ownership boundary"
  ```

### Task 4: Compare macOS and portable Unix candidates

**Files:**
- Create: `crates/process-ownership-fixture/src/platform/unix.rs`
- Create: `crates/process-ownership-fixture/src/platform/macos_launchd.rs`
- Create: `crates/process-ownership-fixture/tests/unix_candidates.rs`
- Create: `crates/process-ownership-fixture/scripts/run-launchd-fixture.sh`

**Interfaces:**
- `platform::UnixCandidate` has `ProcessGroup`, `RegisteredRoot`, and `Launchd` variants.
- `UnixCandidate::launch_paused(spec)`, `observe()`, and `terminate_owned()` implement the same `OwnerDomain` contract used by the Windows adapter.
- `macos_launchd::LaunchdDomain::bootstrap(label, plist) -> Result<Self, PlatformError>` creates a temporary per-generation job; `kill_and_wait()` returns only after launchd reports completion.
- `unix::RegisteredRootDomain` must retain birth identity, ancestry, and liveness evidence for every registered root and reject PID-reuse ambiguity.

- [ ] **Step 1: Write the failing candidate matrix**

  Run each candidate through a PTY child calling `setsid()`, a new-process-group child, forked descendants, daemonized descendants, reparented descendants, and silent descendants. Assert that a candidate is selected only when every row has independent cleanup evidence.

- [ ] **Step 2: Run the focused Unix tests and verify failure**

  Run `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test unix_candidates` on macOS and portable Linux.

  Expected: at least one candidate fails until its containment and observation semantics are implemented; `PR_SET_PDEATHSIG` alone must remain insufficient.

- [ ] **Step 3: Implement and measure the Unix candidates**

  Implement the process-group experiment, the registered-root experiment with PID-reuse checks, and the macOS launchd control. On macOS, exercise the documented process-group cleanup behavior and intentional reparenting. On Linux, record pidfd/cgroup-v2 observations when available without using them as a substitute for the macOS result.

- [ ] **Step 4: Record explicit pass/fail outcomes**

  Emit one JSON evidence record per candidate and scenario containing mechanism, native identities, containment evidence, cleanup phases, survivor state, and failure reason. A missing observer result is `unresolved`; it cannot be recorded as a pass.

- [ ] **Step 5: Run the candidate tests and verify success or an explicit rejection**

  Re-run the focused tests. Expected: the selected candidate passes the complete matrix, or the test records that the macOS prerequisite remains unresolved and blocks issue #545 closure.

- [ ] **Step 6: Commit the Unix experiments**

  ```bash
  git add crates/process-ownership-fixture
  git commit -m "test: compare Unix process ownership mechanisms"
  ```

### Task 5: Exercise cleanup races, foreign ownership, and relaunch barriers

**Files:**
- Create: `crates/process-ownership-fixture/tests/cleanup_matrix.rs`
- Create: `crates/process-ownership-fixture/src/foreign_owner.rs`
- Modify: `crates/process-ownership-fixture/src/supervisor.rs`

**Interfaces:**
- `foreign_owner::ForeignOwner::start(workspace, metadata_revision) -> ForeignSnapshot` holds the real workspace lease, writes known metadata bytes, and updates a heartbeat independently of the test driver.
- `ForeignOwner::snapshot() -> ForeignSnapshot` reads heartbeat, lease state, and metadata bytes/revision without asking the foreign process to self-attest.
- `Supervisor::cleanup() -> CleanupResult` freezes admission, drains in-flight launches, performs the fixed 5s + 5s phases, and returns `Acknowledged(CompleteExitReceipt)` or `Unresolved(Vec<NativeIdentity>)`.
- `RendezvousClient::adopt(record) -> AdoptionResult` succeeds only for an authenticated complete-exit receipt from the same generation.

- [ ] **Step 1: Write the failing race and isolation tests**

  Add focused tests for owner loss during launch, graceful-ignore escalation, supervisor death before receipt, foreign lease/metadata survival, two open generations A/B, late obsolete-generation events, immediate relaunch, and blocked inference admission while an older inference root survives.

- [ ] **Step 2: Run the matrix and verify failure**

  Run `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test cleanup_matrix -- --nocapture`.

  Expected: failures until cleanup serialization, foreign-owner observation, and unresolved adoption state are implemented.

- [ ] **Step 3: Implement the cleanup barrier and receipts**

  Serialize `owner_lost`, ticket admission, launch registration, cleanup census, and acknowledgement behind one supervisor state machine. Reject late tickets before target execution. Do not emit `Acknowledged` until every registered identity is independently observed exited; emit `Unresolved` after the fixed deadline with the surviving identities and observer failure details.

- [ ] **Step 4: Implement foreign-owner and generation isolation checks**

  Compare the foreign heartbeat, lease owner, metadata bytes, and revision before and after each cleanup attempt. Require unchanged values. Tag every ticket, rendezvous event, observation, and receipt with its generation and reject obsolete events.

- [ ] **Step 5: Run the matrix and verify success**

  Run the same command on each supported host. Expected: all isolation and bounded-failure assertions pass; any unresolved survivor is reported as a bounded failure with no false-success acknowledgement.

- [ ] **Step 6: Commit the adversarial matrix**

  ```bash
  git add crates/process-ownership-fixture
  git commit -m "test: cover process ownership cleanup races"
  ```

### Task 6: Audit and gate the production launch seams

**Files:**
- Create: `docs/superpowers/evidence/2026-09-15-process-ownership-proof.md`
- Inspect and modify only after the selected mechanism passes: `apps/desktop/electron/main.ts`
- Inspect and modify only after the selected mechanism passes: `apps/desktop/electron/sidecarLifecycle.ts`
- Inspect and modify only after the selected mechanism passes: `crates/orkworksd/src/runtime/session_runtime.rs`
- Inspect and modify only after the selected mechanism passes: `crates/orkworksd/src/providers.rs`
- Inspect and modify only after the selected mechanism passes: `crates/orkworksd/src/taskmaster/runtime/inference.rs`
- Create if production ownership becomes implementable: `crates/orkworksd/src/process_ownership.rs`
- Create if production ownership becomes implementable: `apps/desktop/electron/processSupervisor.ts`

**Interfaces:**
- `processSupervisor.ts` must expose only generation start, authenticated launch-ticket request, owner-loss notification, and complete-exit/unresolved receipt retrieval to Electron main.
- `process_ownership.rs` must expose one sidecar-owned launch adapter used by both `SessionRuntime` PTY creation and provider/inference process creation; direct `Command::spawn` calls cannot bypass the adapter.
- The evidence document records the exact call sites, whether each path is routed through the proven boundary, and the remaining blocker when it is not.

- [ ] **Step 1: Write the seam-audit assertions**

  Add a checklist-backed test or static audit that enumerates the sidecar spawn in `SessionRuntime`, the provider process runner, custom inference spawn callback, native inference path, and Electron sidecar lifecycle. Each path must be either routed through the owner adapter or explicitly marked as an open blocker.

- [ ] **Step 2: Run the audit before changing production behavior**

  Run `/opt/homebrew/bin/rg -n "Command::spawn|portable_pty|spawn\(" crates/orkworksd/src apps/desktop/electron` and inspect each result against the fixture protocol. Run the existing Rust baseline with `cargo test --manifest-path crates/orkworksd/Cargo.toml`.

  Expected: the audit identifies the current direct PTY and inference spawn seams; it must not claim that the existing Windows provider Job helper proves the full boundary.

- [ ] **Step 3: Integrate only the proven boundary**

  If and only if Tasks 3–5 have a passing native mechanism, route the actual production roots through the supervisor-issued ticket and registration sequence. Preserve the Electron main/renderer boundary, keep the owner outside the Electron kill domain, and keep existing process timeout behavior subordinate to owner cleanup rather than replacing it.

- [ ] **Step 4: Add production regression tests**

  Add Rust tests for PTY and inference launch denial before registration, generation mismatch, owner-loss admission freeze, and no new inference admission while a prior root survives. Add Electron tests for forced sidecar-parent termination, stale-generation receipt rejection, and unavailable-versus-empty state.

- [ ] **Step 5: Run production verification**

  Run `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`, `cargo test --manifest-path crates/orkworksd/Cargo.toml`, `cd apps/desktop && npx tsc --noEmit`, and the focused Electron test files covering sidecar lifecycle and backend restoration.

  Expected: existing behavior remains green and new tests prove the production seams cannot bypass the selected ownership boundary. If the platform mechanism remains unresolved, leave production files unchanged and record the blocker.

- [ ] **Step 6: Commit the seam audit or guarded integration**

  ```bash
  git add docs/superpowers/evidence apps/desktop/electron crates/orkworksd/src
  git commit -m "docs: record process ownership production seam audit"
  ```

### Task 7: Record evidence, ADR consequences, and issue status

**Files:**
- Modify: `docs/superpowers/specs/2026-09-15-process-ownership-proof-design.md`
- Modify: `docs/adr/0056-one-sidecar-per-open-workspace.md`
- Modify: `docs/adr/README.md` if ADR status or index text changes
- Create or modify: `docs/superpowers/evidence/2026-09-15-process-ownership-proof.md`
- Modify: GitHub issue #545 with the evidence summary and remaining blockers

**Interfaces:**
- The evidence record has one row per matrix scenario and platform with `mechanism`, `host`, `architecture`, `build_mode`, `test_command`, `forced_parent_termination`, `native_identities`, `containment_observation`, `cleanup_latency_ms`, `result`, and `survivors`.
- ADR 0056 records only mechanisms actually demonstrated by the fixture and keeps unresolved platform limitations explicit.
- Issue #545 is checked off only for evidence actually produced; it remains open when the macOS mechanism or production seam audit is unresolved.

- [ ] **Step 1: Write the evidence validation test**

  Validate that every required matrix row has a platform result, that a pass includes independent observation and forced-parent proof, and that unresolved rows include a concrete failure reason.

- [ ] **Step 2: Run evidence validation and verify failure**

  Run `cargo test --manifest-path crates/process-ownership-fixture/Cargo.toml --test evidence`.

  Expected: failure when any required platform/scenario record is absent or falsely marked successful.

- [ ] **Step 3: Produce the evidence bundle and update ADR 0056**

  Copy exact commands and bounded outputs into the evidence document, update the design's selected-direction and limitations sections, and amend ADR 0056 only to reflect demonstrated facts. Do not convert a fixture-only result into a production recovery guarantee.

- [ ] **Step 4: Run the complete verification set**

  Run the fixture tests, the Rust baseline, `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`, the desktop type check, and the documentation checks required by CI. Review the evidence document line by line against the design and issue acceptance criteria.

- [ ] **Step 5: Comment on issue #545 and request review**

  Post the evidence document, exact commit/PR references, passed and unresolved matrix rows, and the explicit decision whether #545 is complete or remains blocked pending platform/production work. Use the repository's normal PR review process before any recovery implementation begins.

- [ ] **Step 6: Commit the evidence record**

  ```bash
  git add docs/superpowers/specs docs/adr docs/superpowers/evidence
  git commit -m "docs: record process ownership proof evidence"
  ```

## Self-Review Checklist

- [ ] Windows Job Object behavior is tested with suspended registration, breakaway attempt, inherited-handle prevention, and actual parent termination.
- [ ] macOS candidates are tested through `setsid()`, new process groups, daemonization, reparenting, and silent descendants.
- [ ] Portable Linux results are recorded separately and do not substitute for macOS evidence.
- [ ] Ticket replay, cross-generation use, forged identity, and stale rendezvous are rejected before target execution.
- [ ] Cleanup races are serialized and bounded; pre-barrier census results cannot produce a complete-exit acknowledgement.
- [ ] Foreign lease, heartbeat, metadata bytes, and revision survive every owned cleanup attempt.
- [ ] Replacement/adoption remains blocked on missing, stale, malformed, or ambiguous supervisor state.
- [ ] Actual `portable_pty` and inference launch seams are audited before issue #545 is declared complete.
- [ ] No multi-workspace runtime recovery or replacement behavior is implemented by this plan.
