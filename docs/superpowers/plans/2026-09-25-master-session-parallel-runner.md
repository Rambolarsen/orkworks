# Master-Session Parallel Runner Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` to implement this plan task by task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user approve one immutable master plan, launch its independent child sessions concurrently in approved batches and isolated worktrees, inspect their results, manually integrate any wanted changes, and clean up only plan-owned worktrees that are safe and clean.

**Architecture:** Extend the existing data-only Taskmaster coordinator foundation with a bounded batch runner. A master-session skill produces a structured candidate that the user imports into OrkWorks; proposal and approval use Electron-only user authority, and only explicit approval activates the plan. The master sidecar owns plan state and a bounded parent/child broker association, while each child has a dedicated one-workspace sidecar, its own lease, and an OS-enforced execution boundary. Children report results and server-observed completion evidence; only validated receipts advance a batch. Child code remains in the child worktree for manual user integration.

**Tech Stack:** Rust/Axum sidecar, serde-backed durable records, Electron main/preload/React UI, existing harness launch definitions and report capabilities, repository skills and hooks, native process containment per supported OS.

**Spec:** [`docs/superpowers/specs/2026-09-25-master-session-parallel-runner-design.md`](../specs/2026-09-25-master-session-parallel-runner-design.md), with the Coordinator contract in [`docs/superpowers/specs/2026-09-24-taskmaster-bounded-coordinator-design.md`](../specs/2026-09-24-taskmaster-bounded-coordinator-design.md).

## Global Constraints

- The user approves one exact immutable plan revision before any child worktree or child session is created.
- The runner exposes ordered parallel batches, not a general DAG; every child in an approved batch is required, and children in that batch are independent.
- No dynamic or recursive child creation, shared-worktree execution, changes to existing branches, commits, merges, pushes, branch deletion, user acceptance on behalf of the user, or automatic integration of child changes. Each allocated worktree is attached to its unique plan-owned branch, created as part of the approved worktree lifecycle. Each child has one attempt; rerunning failed work requires a new plan revision and approval.
- Child edits remain in their allocated worktrees. The user reviews and manually integrates or discards them. Cleanup refuses dirty, active, mismatched, or foreign worktrees and preserves branches and commits.
- Skills may help a master session prepare a structured candidate. The user imports and approves it in OrkWorks. Hooks may report lifecycle observations. Neither skills nor hooks grant approval, launch sessions, prove task success, or advance a batch.
- Each OrkWorks instance continues to own one workspace. The parent/child broker is plan-scoped control authority, not a peer-instance registry or multi-workspace metadata owner.
- Do not enable launch on an OS/harness combination without native evidence that filesystem/process confinement, resource ceilings, and generation-specific termination are enforced. Unsupported combinations fail closed.
- Every child invocation uses finite approved resource limits and the server-owned broker. No harness receives a direct command path that bypasses enforcement.
- Existing Taskmaster recommendations and completion packets keep their current lifecycle and authority.
- The plan and runner scope documentation must merge before implementation begins; the architecture amendments and confinement gate below are prerequisites to runtime launch.
- For every code task, use the repository's test-driven-development workflow: add the listed focused contract test first, run it to confirm the missing behavior, implement the smallest change, and rerun the focused test before continuing.

---

### Task 1: Prove eligible process confinement before enabling launches

**Files:**
- Create: `docs/validation/master-session-runner-confinement.md`
- Modify: `docs/adr/0060-independent-workspace-instances.md` to record the tested platform mechanisms and limitations
- Test: `crates/orkworksd/tests/master_session_runner_confinement.rs`

**Interfaces:**
- Consumes: the existing interactive PTY launch in `runtime/session_runtime.rs`, the provider-only containment in `providers/windows_process.rs`, and the generation/ownership requirements in ADR 0060.
- Produces: an explicit OS × harness eligibility table, an enforceable spawn/command boundary, and generation-bound evidence that child descendants cannot escape or survive an acknowledged stop.

- [ ] Inventory the OS-specific controls available to supported desktop targets and record which enforce canonical worktree access, process-tree ceilings, child termination on cancellation and master-sidecar crash, credential isolation, and a clean successful harness exit after a completion request.
- [ ] Write native fixtures for out-of-worktree writes, descendants, process/resource exhaustion, cancellation, master-sidecar crash with an acknowledged child, successful EOF/completion, and fake home/keychain credentials.
- [ ] Run each fixture on every proposed supported OS/harness and verify allowed operations succeed, denied operations fail, credentials stay unavailable to the child unless brokered without exposing material, foreign sentinels survive, a completed child exits successfully without `Kill`, and the complete owned tree is quiescent after cancellation or master crash.
- [ ] Exclude every OS/harness combination without enforceable evidence from launch eligibility; do not treat process groups, provider Job Objects, harness prompts, stop hooks, or asserted PIDs as confinement proof.
- [ ] Update the validation record and the ADR 0060 amendment with the mechanism, platform limits, failure behavior, and evidence links.

**Gate:** If no supported OS/harness combination passes, stop before launch implementation and return the limitation for spec review.

### Task 2: Persist immutable batch plans and approval authority

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/coordinator.rs`
- Modify: `crates/orkworksd/src/taskmaster/coordinator_store.rs`
- Modify: `crates/orkworksd/src/taskmaster/coordinator_tests.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Create: `crates/orkworksd/src/http/taskmaster_runner_handlers.rs`
- Test: `crates/orkworksd/src/http/taskmaster_runner_handlers.rs`

**Interfaces:**
- Consumes: `PlanRevision`, `PlanApproval`, `PlanStatus`, `CoordinatorStore::{put_proposed,get,activate,transition,resume}`, canonical JSON/digest validation, and the existing Electron-only sidecar authority.
- Produces: a runner plan revision with ordered batches, a desktop-imported candidate bound to its source master session, user-authenticated proposal/approval/revocation and post-run disposition, and durable status that cannot launch work by itself.

- [ ] Add a batch-plan representation that reuses the existing coordinator identity, canonicalization, immutable revision, and approval checks while rejecting dependency edges outside the declared batch order.
- [ ] Persist a typed post-run disposition (`accept_success`, `reject`, `abandon`, or `discard`) separately from lifecycle status; validate allowed dispositions against the current state and record authenticated actor/time before authorizing cleanup. Keep pre-approval proposal rejection separate from cleanup authorization.
- [ ] Validate every child contract, exact rendered prompt/context and derivation inputs, harness/model identity and definition generation, repository-relative resource scope, worktree policy, output contract, one-attempt limit, concurrency ceiling, finite budget, and required-child membership before persisting a proposal. Show the effective prompt in the approval UI and revalidate its inputs/definition generation before dispatch.
- [ ] Enforce the coordinator contract's aggregate retention caps for plan/approval digests, evidence/audit records, tombstones, and idempotency/recovery records; fail closed before mutation when admission would exceed a cap, and test bounded eviction/compaction and pinned live recovery evidence.
- [ ] Reject conflicting same-batch read/write or write/write scopes using the shared repository identity and canonical repository-relative resources, including shared lockfiles, build outputs, caches, and declared external side effects; physical worktree paths must not make the same source path appear unrelated.
- [ ] Add `POST /taskmaster/plans` for the desktop to import and persist a candidate under Electron-only user authority; bind its source session and active workspace, and reject the per-session report capability as proposal or approval authority.
- [ ] Add `GET /taskmaster/plans/:id` for the current proposal, immutable plan, approval summary, batch state, and child result projection; redact bearer values and private environment data.
- [ ] Add an Electron-only approval operation that binds the user's approval to the exact plan/evidence digests, clean repository revision, empty dirty-path set, workspace identity, attribution confidence, expiry, and revocation generation.
- [ ] Reject proposal edits after approval; require a new immutable revision and fresh approval for any changed task, prompt, batch, harness/model, scope, evidence subject, limit, or output contract.
- [ ] Add persistence tests for deterministic digests, unknown fields, duplicate IDs, invalid batch dependencies, dirty/inconclusive workspace subjects, stale revisions, idempotent retries, approval binding, revocation, and restart recovery.

### Task 3: Add the user approval and master proposal flow

**Files:**
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/taskmaster.ts`
- Modify: `apps/desktop/electron/preload.ts`
- Modify: `apps/desktop/src/components/RecommendationsPanel.tsx`
- Create: `skills/master-session-runner/SKILL.md`
- Modify: `apm.yml`
- Modify: `docs/agents/apm.md`
- Test: focused desktop API, preload, and renderer tests in `apps/desktop/tests/`

**Interfaces:**
- Consumes: the proposal and detail routes from Task 2 plus existing narrow Electron-to-sidecar authentication.
- Produces: a user-visible complete plan review, explicit proposal approval/rejection, distinct post-run result acceptance or reject/abandon/discard actions, status/results view, and neutral master-session instructions that can prepare a candidate but cannot create an authorized proposal or launch it.

- [ ] Add typed desktop API methods for proposing, reading, approving, revoking, pausing, cancelling, and requesting cleanup of a plan; expose only the methods required by the UI through preload. Revocation must enter its own fence, not alias pause or cancel.
- [ ] Render the exact batches, server-rendered child prompts/context and derivation inputs, contracts, harness/model definition generations, repository-relative scopes, worktree/base subject, limits, and cleanup behavior before approval; make approve/reject and post-approval revoke explicit user actions.
- [ ] After execution, expose separate authenticated actions for accepting a successful result (`accept_success`) and rejecting, abandoning, or discarding results; show which disposition is recorded and explain that none integrates code or overrides clean-worktree cleanup checks.
- [ ] Show progress and validated child reports without adding parallel terminal rendering or treating a hook/idle signal as completion.
- [ ] State in the UI that child edits remain in separate worktrees and require manual integration; show each worktree's clean/dirty status and explain why cleanup is blocked while dirty.
- [ ] Add a paste/import field for the structured JSON candidate emitted by the master-session skill; show the normalized candidate for review, and keep import separate from the user's explicit approval of the exact plan revision. Do not scrape or interpret terminal output automatically.
- [ ] Add the master-session skill to instruct bounded decomposition, independent batch selection, candidate formatting, and escalation; state that the skill cannot create an authorized proposal, approve, launch, change an approved plan, or certify success.
- [ ] Test approval visibility and exact rendered-prompt digest binding, prompt/definition changes before dispatch, malformed proposal display, pre-approval rejection without cleanup authority, each allowed post-run disposition by lifecycle state, conflicting/idempotent disposition requests, dirty-worktree cleanup refusal, revoke distinct from pause/cancel, and the single-active-context UI invariant.

### Task 4: Allocate plan-owned worktrees and launch dedicated child runtimes

**Files:**
- Create: `crates/orkworksd/src/taskmaster/runner/worktrees.rs`
- Create: `crates/orkworksd/src/taskmaster/runner/child_runtime.rs`
- Create: `crates/orkworksd/src/taskmaster/runner/capability.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Test: new runner unit tests and native process fixtures; do not launch real coding tools in tests

**Interfaces:**
- Consumes: an approved `PlanRevision`, Task 1's proven platform launcher, the existing generic session creation path, and the immutable allocation ID recorded by the plan store.
- Produces: idempotent allocation records and dedicated child sidecars, each owning exactly one assigned worktree and presenting a child capability bound to one plan, batch, attempt, runtime, and expiry.

- [ ] Implement a durable allocation intent (`prepared`) that records repository identity, clean base revision, unique worktree path, and unique owner-authorized plan branch before any Git mutation; then create the linked worktree and atomically mark it allocated before child launch. Refuse the primary checkout, detached worktrees, branch/path collisions, and foreign branches.
- [ ] On restart, reconcile prepared allocations against the exact recorded repository/path/branch. Adopt only an exact plan-owned match; if partial state is ambiguous, enter recovery and do not delete or relaunch.
- [ ] Ensure retries for the same approved idempotency key return the existing allocation and that mismatched retries fail without creating another worktree.
- [ ] Add a dedicated child-runtime launcher that resolves cwd from the server-owned allocation ID, starts one-workspace sidecars, and never lets a request choose an arbitrary path or sidecar address.
- [ ] Issue child capabilities separately from the master coordinator capability; bind each to the plan digest, approval, allocation/worktree, child session, batch, attempt, allowed broker audience, expiry, and revocation generation.
- [ ] Persist launch-pending and launch-acknowledged states atomically; uncertain launch responses retain reservations and enter recovery instead of retrying or refunding.
- [ ] Persist the single attempt's normalized budget reservation atomically with lease, launch-token, attempt/plan lifecycle, and coordinator graph state: reserve before dispatch, refund only after server-proven non-start, and mark the unit consumed once work starts. Preserve reservations while launch/start state is uncertain; crash recovery must never leave reservation and launch/attempt/graph state divergent.
- [ ] Prove concurrent allocations are unique, branch-attached and never detached, prepared-allocation recovery is safe at every crash point, unauthorized paths and cwd substitutions are rejected, cross-workspace token reuse fails, and startup fails closed when platform confinement or credential isolation is absent.

### Task 5: Broker child commands and enforce finite execution limits

**Files:**
- Create: `crates/orkworksd/src/taskmaster/runner/broker.rs`
- Create: `crates/orkworksd/src/taskmaster/runner/limits.rs`
- Modify: the native process-control module selected in Task 1
- Modify: `crates/orkworksd/src/runtime/session_runtime.rs`
- Test: broker unit tests and platform confinement fixtures

**Interfaces:**
- Consumes: the master-authorized dispatch envelope, a live child capability, the approved child contract, and Task 1's native enforcement adapter.
- Produces: an allow/deny decision and a server-attested invocation receipt for every command/tool operation, with cumulative attempt accounting.

- [ ] Validate executable identity, arguments, canonical cwd, environment allowlist, declared file/resource effects, lease state, capability revision, hard denials, and all remaining ceilings atomically before each invocation.
- [ ] Enforce wall-clock, aggregate process-tree CPU, memory, process count, output bytes, input/output tokens, cost, and invocation count; reject a provider or platform that cannot enforce any required ceiling.
- [ ] Deny Git mutation, credentials, permission changes, destructive commands, scope/provider/budget/attempt expansion, direct child process access, and any invocation that can bypass the broker. A harness that needs user home/keychain credentials is ineligible unless a broker-owned authentication path keeps credential material out of the child; test with fake login state.
- [ ] Reserve approved worst-case token/cost and tool units atomically before dispatch; consume the reservation once start is confirmed, and refund only when server evidence proves the invocation never started. A reserved/launch-pending state or missing acknowledgement alone is not proof of non-start.
- [ ] Key read/write resource leases by repository identity plus canonical repository-relative resource, retaining physical-path confinement for each child; reject same-batch conflicting scopes even when each worktree resolves to a different physical path.
- [ ] Test each hard denial, exhausted limit, shell/interpreter indirection, descendant process, same-repository conflicts across separate worktrees, lease revocation during execution, credential isolation/fail-closed behavior, sanitized receipt output, atomic reservation under concurrent dispatch, proven-non-start refund, started-work consumption after termination, uncertain-launch retention, and crash/restart consistency across reservation, launch-token, lease, attempt/plan lifecycle, and coordinator graph transitions.

### Task 6: Validate child completion and advance parallel batches

**Files:**
- Create: `crates/orkworksd/src/taskmaster/runner/reports.rs`
- Create: `crates/orkworksd/src/taskmaster/runner/batches.rs`
- Create: `crates/orkworksd/src/http/taskmaster_child_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: the plan and attempt persistence from Task 2
- Test: report, handshake, receipt, and batch tests

**Interfaces:**
- Consumes: an authenticated child report, the child capability, current allocation/attempt state, the observed child session/process exit, and the broker receipts from Task 5.
- Produces: an immutable validated attempt receipt and a monotonic current-batch transition; child text alone never changes plan state.

- [ ] Add a strict child-report route that authenticates only the child capability and verifies plan, revision, batch, allocation, worktree, attempt, output contract, idempotency key, and size limits.
- [ ] Implement the one-shot report-to-exit handshake using a broker-fenced EOF or harness-specific clean-completion operation: seal the report, request graceful harness termination, observe a successful non-killed exit and complete process-tree quiescence, verify outputs, then persist the receipt. Do not treat `Kill` as success.
- [ ] Bind each receipt to the report digest, observed runtime/session/process identity, exit result, lease, workspace input/output subjects, verified output hashes, and verifier result.
- [ ] Advance a batch only after every required child has a successful current receipt; pause on missing, stale, failed, conflicting, or ambiguous evidence.
- [ ] Permit the next approved batch to consume persisted reports and declared artifacts only; reject any attempt to treat unmerged child worktree edits as available inputs.
- [ ] Test duplicate/late/cross-plan reports, report without process exit, nonzero/killed exit, timeout, unverified output, stale workspace subjects, one failed sibling, unsupported clean-exit harness, and every prior-batch gate.

### Task 7: Implement pause, cancellation, recovery, acceptance, and guarded cleanup

**Files:**
- Modify: `crates/orkworksd/src/taskmaster/runner/` modules from Tasks 2, 4, and 6
- Modify: `crates/orkworksd/src/taskmaster/coordinator_store.rs`
- Modify: `crates/orkworksd/src/http/taskmaster_runner_handlers.rs`
- Modify: the Taskmaster plan UI from Task 3
- Test: lifecycle, crash-recovery, and filesystem ownership tests

**Interfaces:**
- Consumes: durable plan/approval/attempt/allocation records and generation-specific process ownership proof.
- Produces: a fenced terminal/recovery state and a cleanup outcome that removes only clean, quiescent worktrees owned by the approved plan while preserving branches and commits.

- [ ] Make pause, cancellation, expiry, and revocation install a durable mutation fence, revoke child leases, stop new launches, and request bounded termination of every owned process tree.
- [ ] Preserve a single recovery `fence_reason`; reconcile uncertain launch, termination, and cleanup only from generation-specific server-observed evidence.
- [ ] Prove a master-sidecar crash terminates every acknowledged child sidecar and owned process tree through a crash-surviving process owner; otherwise mark that OS/harness combination unavailable. On restart, never relaunch on uncertain child state.
- [ ] Give each child exactly one attempt in v1. Keep failed or cancelled plans and their child worktrees for diagnosis; do not retry or spawn a replacement in the same plan. Any rerun requires a new immutable plan and fresh approval. Allow cleanup from failure/cancellation only after recorded user rejection/abandonment/discard, never `accept_success`.
- [ ] Define `accept_success` as acceptance of a successful result for user review/integration, not code integration; permit it only from `awaiting_acceptance`. Permit reject/abandon/discard only from the states specified in the runner design. Require the user to integrate or discard desired edits manually before cleanup; do not copy edits, commit, merge, or delete branches.
- [ ] Allow cleanup only after a valid, authenticated, recorded disposition, proven child quiescence, and fresh checks that each path is plan-owned, unchanged, and clean; a dirty or ambiguous worktree remains and reports a recovery/user action.
- [ ] Test the disposition-by-lifecycle-state matrix, including reject/abandon/discard from `recovery_required`; verify disposition and cleanup fence are recorded atomically, cleanup is refused until reconciliation proves quiescence and cleanup state, wrong/conflicting dispositions are rejected, same-disposition retries are idempotent, and no invalid disposition authorizes cleanup. Also test cancellation during provisioning and execution, expiry/revocation races, restart with pending launch, uncertain process exit, dirty and foreign worktree refusal, path replacement, clean-worktree removal, and branch/commit preservation.

### Task 8: Integrate the runner into Taskmaster and complete cross-platform verification

**Files:**
- Modify: `docs/agents/architecture.md`
- Modify: `docs/user/sessions.md`
- Modify: `specs/taskmaster.md` acceptance criteria and implemented-status notes
- Modify: `AGENTS.md` and scoped instructions only where implemented behavior changes repository workflow
- Test: Rust unit/integration suites, focused desktop tests, typecheck, documented native confinement fixtures

**Interfaces:**
- Consumes: completed API, runner, UI, capability, persistence, and lifecycle contracts from Tasks 1–7.
- Produces: accurate implemented-behavior documentation and a release gate listing exactly which OS/harness combinations are enabled.

- [ ] Document only behavior that passed its native gate; identify unsupported OS/harness combinations as unavailable rather than describing intended behavior as implemented.
- [ ] Verify all approval, launch, report, cancellation, recovery, and cleanup acceptance criteria in the approved runner spec against a concrete test or native fixture.
- [ ] Run the Rust commands from the repository root: `cargo build --manifest-path crates/orkworksd/Cargo.toml`, `cargo test --manifest-path crates/orkworksd/Cargo.toml`, and `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check`.
- [ ] Run the desktop commands from `apps/desktop/`: `pnpm exec tsc --noEmit`, `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`, and `node --experimental-strip-types --test tests/api.test.ts`.
- [ ] Run `bash scripts/doc-check.sh` and `git diff --check`; record failures without weakening launch gates.
- [ ] Run the repository's explicit `/code-review medium` gate on the integrated implementation because this runner crosses component, concurrency, lifecycle, and protocol boundaries; if implementation is split into smaller PRs, use the documented effort for each resulting diff. Resolve each actionable finding before handoff.

## Planning checkpoint: investigated risks and assumptions

- **Resolved output-handling question:** child code remains isolated. The user manually integrates or discards edits; the runner never combines or transfers code, changes existing branches, or commits. Allocation creates a unique plan-owned branch per linked worktree and cleanup preserves it. The master may remove an allocated worktree only after the state-appropriate recorded disposition, quiescence proof, and a fresh clean/ownership check.
- **Confinement risk:** current code has PTY session startup in `runtime/session_runtime.rs` and a Windows Job Object provider runner in `providers/windows_process.rs`. The provider runner is not proof that interactive coding tools, their subprocesses, or their file effects are confined. Task 1 is a hard launch gate.
- **Workspace ownership risk:** issue #545 is closed, but its closure is not evidence that the runner's native process-ownership and confinement gates passed. ADR 0060 and the runner design still require generation-specific proof for child runtime shutdown; the implementation must link qualifying evidence.
- **Foundation reuse:** the repository already has a data-only coordinator model and durable store in `taskmaster/coordinator.rs` and `taskmaster/coordinator_store.rs`. Reuse their plan digests, immutable revisions, approval validation, atomic publication, and recovery behavior; do not fork ordinary Taskmaster recommendation state or expose a general DAG as this runner's product model.
- **Proposal boundary:** the approved design requires user authority for proposal and approval transitions. This plan treats master output as an untrusted structured candidate that the user imports through the desktop; the desktop authenticates proposal persistence and the separate approval action. A session-report capability cannot create proposals or authorize launches.
- **Disposition boundary:** pre-approval rejection is not cleanup authorization. After execution, `accept_success` is distinct from reject/abandon/discard, persisted separately from lifecycle status, and valid only for the specified states; every cleanup path checks that recorded disposition plus quiescence and clean ownership.
- **Reservation lifecycle:** the existing coordinator contract defines normalized per-attempt reservations. The runner must atomically reserve before dispatch, refund only on proven non-start, consume once work starts, and preserve uncertain reservations through recovery. This runner allows one attempt per child; retries require a new approved plan revision.

## Self-review

- **Spec coverage:** Tasks 1–8 cover exact prompt approval, aggregate retention, repository-relative conflicts, branch-attached prepared allocations, credential/exit/crash gates, revocation, one-attempt children, broker limits, authenticated reports, receipts, parallel batches, failure/cancellation/recovery, manual integration, guarded cleanup, and tests. Stop hooks and skills are limited to their specified roles; combined-code verification remains outside the runner.
- **Placeholder scan:** no task delegates unspecified work with “TBD”, “handle edge cases”, or “write tests for the above”; the platform choice is intentionally gated on native evidence before launch is enabled.
- **Type consistency:** shared authority originates in existing `PlanRevision`, `PlanApproval`, `PlanStatus`, and `CoordinatorStore`; runner-specific batch and attempt records reference those identities and never mint user approval in the child runtime.
- **Scope check:** the plan is large because it contains required authority, native confinement, process ownership, UI approval, and recovery work. Its staged gates prevent the data-only coordinator foundation from being mistaken for runtime authorization. If the confinement gate yields no eligible platform, stop and return for a spec decision instead of implementing a weaker runner.
