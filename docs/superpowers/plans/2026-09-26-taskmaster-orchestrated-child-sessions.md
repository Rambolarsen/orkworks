# Taskmaster Orchestrated Child Sessions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user-approved parent session plan and coordinate a complete workflow, delegating every implementation and review task to ordinary OrkWorks child sessions.

**Architecture:** Extend the existing Rust session runtime with a small durable orchestration-plan store, an explicit UI approval path, parent-scoped launch/status APIs, and parent/child metadata. Use existing PTY sessions for all children and plan-owned Git worktrees for independent code-writing tasks. Extend the desktop session list and plan view to review the whole plan and show child progress. Orchestration approval governs OrkWorks launch requests; it does not confine commands run by coding tools.

**Tech Stack:** Rust, Axum, Tokio, serde, Electron main/preload IPC, React, TypeScript, existing PTY session runtime, Git worktrees.

**Spec:** [Taskmaster orchestrated child sessions](../specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md)

## Global Constraints

- A user must approve the exact complete plan revision before any child session or plan-owned worktree is created.
- The parent session coordinates only; every implementation and review task in the workflow is delegated to a child session.
- A child may launch only a declared task whose dependencies are complete; duplicate launch requests return the already-created child.
- A scope change, retry, harness/model change, or concurrency increase requires a new plan revision and explicit approval.
- Plan approval must use the existing Electron-main-owned `ORKWORKS_OPEN_PLAN_TOKEN`, which is filtered from parent and child coding-tool environments.
- Child sessions use the existing workspace sidecar, session metadata store, PTY runtime, and ordinary session lifecycle.
- Parallel independent task chains use separate plan-owned worktrees; dependent tasks in a chain reuse that worktree sequentially. OrkWorks does not integrate, commit, merge, rebase, push, or automatically clean up child work.
- Workflow scope is not OS security confinement. Child processes retain the normal user permissions and harness login behavior.
- Task completion is a parent-reported coordination state after child termination, not proof of quality or user acceptance. Existing user escalation and merge/acceptance gates remain in force.
- Do not add XPC, a VM, a child-specific sidecar, a process broker, credential proxy, resource budget system, or cross-instance registry.
- Before runtime changes, update the authoritative Taskmaster spec and the relevant ADR/issue records to approve this proposed product direction.

---

## File Map

| File | Responsibility |
| --- | --- |
| `specs/taskmaster.md` | Make the orchestrated child-session scope authoritative, preserve existing user escalations, and retire conflicting v1/master-runner wording. |
| `docs/adr/README.md` and new `docs/adr/0066-taskmaster-orchestrated-child-sessions.md` | Record the session-runtime, approval, and worktree architecture decision. |
| `crates/orkworksd/src/metadata.rs` | Persist parent mode (`ordinary`/`orchestrator`) plus optional parent session, plan, task, and launch-reservation identifiers on parent and child records. |
| `crates/orkworksd/src/taskmaster/orchestration.rs` | Validate bounded plans, immutable revisions, dependencies, proposed worktree paths, launch idempotency, and parent/child authorization. |
| `crates/orkworksd/src/taskmaster/orchestration_store.rs` | Serialize per-plan mutations and persist proposals, approvals, launch reservations, task results, and scope-change revisions under the active workspace metadata root. |
| `crates/orkworksd/src/session_application.rs` | Route approved child creation through the existing session creation/runtime path and create assigned worktrees after approval. |
| `crates/orkworksd/src/http/session_handlers.rs` and `CreateSessionCommand` | Accept the explicit Orchestrator mode for user-created parents and an internal validated cwd override for approved child launches. |
| `crates/orkworksd/src/git.rs` | Verify the approved clean base revision and provision unique plan-owned worktree paths/branches. |
| `crates/orkworksd/src/http/orchestration_handlers.rs` | Expose plan proposal, read, child launch, and child status operations with distinct parent and UI authority checks. |
| `crates/orkworksd/src/main.rs` and `crates/orkworksd/src/http/mod.rs` | Register orchestration routes and reuse the current sidecar-generation approval authority. |
| `crates/orkworksd/src/session_types.rs` and session projection modules | Return parent/child links, plan task labels, and ordinary lifecycle/status data to the desktop. |
| `apps/desktop/electron/main.ts`, `preload.ts`, and preload contract types | Keep the approval credential in Electron main and expose narrow review/approve/reject methods to the renderer. |
| `apps/desktop/electron/planOpener.ts` | Reuse the existing Electron-main-only `ORKWORKS_OPEN_PLAN_TOKEN` request pattern for user plan approvals; do not create a second UI token. |
| `apps/desktop/src/api.ts`, `apps/desktop/src/domain/session.ts`, and `apps/desktop/src/workspaceSessionController.ts` | Add validated orchestration plan/session-lineage types and preserve mode when creating a parent session. |
| `apps/desktop/src/components/NewSessionDialog.tsx` | Let the user explicitly choose ordinary session or Orchestrator mode; ordinary remains default. |
| `apps/desktop/src/components/SessionListPanel.tsx` and a focused orchestration plan component | Nest child sessions and show plan approval, task assignment, dependencies, and progress. |
| `apps/desktop/src/App.tsx` and session controller | Wire user approval actions, plan refresh, and child selection into existing app state. |

## Task 1: Align authoritative scope and architecture records

**Files:**
- Modify: `specs/taskmaster.md`
- Create: `docs/adr/0066-taskmaster-orchestrated-child-sessions.md`
- Modify: `docs/adr/0060-independent-orkworks-instances.md`
- Modify: `docs/adr/0064-bounded-taskmaster-coordinator.md`
- Modify: `docs/adr/README.md`
- Update: GitHub issues `#610` and `#617`

- [ ] Replace Taskmaster's v1 and coordinator-gate wording that excludes task decomposition, child launches, and plan-owned worktrees with the approved bounded orchestrator behavior.
- [ ] Preserve the core ADR 0060 decision that each OrkWorks instance owns at most one workspace and sidecar; supersede only the proposed dedicated child-sidecar amendment.
- [ ] Amend or supersede proposed ADR 0064 where its broker, lease, hard confinement, and durable process-owner requirements conflict with ordinary session children.
- [ ] Preserve the existing requirement that product/architecture decisions, ambiguous requirements, credentials/permissions, destructive actions, Git mutation or merge approval, conflicting high-confidence results, and high-risk acceptance escalate to the user.
- [ ] Add ADR 0066 describing exact-revision UI approval, Electron-owned approval authority, normal child session lifecycle, task-scoped launches, and the absence of OS confinement.
- [ ] Update #610 acceptance criteria to match the approved plan/task/child workflow; close or replace #617 because native process confinement is no longer a prerequisite for this product scope.
- [ ] Review doc links and verify the docs build's dead-link checks before any code task starts.

**Gate:** Stop if the user has not approved the proposed design and reviewed implementation plan. No runtime work begins until the spec, ADRs, and tracking issue agree on scope.

## Task 2: Add durable orchestration plan types and store

**Files:**
- Create: `crates/orkworksd/src/taskmaster/orchestration.rs`
- Create: `crates/orkworksd/src/taskmaster/orchestration_store.rs`
- Modify: `crates/orkworksd/src/taskmaster/mod.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Test: unit tests in the new orchestration modules and metadata migration tests

**Interfaces:**
- `OrchestrationPlan` contains `id`, `parent_session_id`, `revision`, `workspace_identity`, canonical `repository_root`, clean `base_commit_sha`, ordered `tasks`, `max_parallel_children`, `status`, `approved_at`, and a digest of the approved serialized revision.
- `OrchestrationTask` contains `id`, `description`, `initial_prompt`, `depends_on`, `worktree_group_id`, `harness_id`, `model`, closed `status`, `task_version`, and optional `launch_reservation_id` and `child_session_id`.
- Task status is `planned | ready | launching | running | needs_parent_result | reported_complete | reported_failed | reported_blocked | launch_interrupted`; `reported_complete` is a parent assertion used only to unlock declared dependencies, not acceptance.
- Each worktree group has one canonical proposed absolute path under `<workspace_metadata_root>/taskmaster/worktrees/<plan_id>/<group_id>`. The path and IDs are part of the plan revision and displayed before approval; no directory or Git worktree exists yet.
- Independent groups may run in parallel; tasks in one group run sequentially and reuse its working directory. Dependency edges may not cross worktree groups.
- Plan status is the closed enum `proposed | approved | paused | cancelled | complete`.
- A plan-control capability is an OS-random secret bound in memory to one parent session and sidecar generation. It expires when the parent ends, the workspace/sidecar generation changes, or the plan is paused, cancelled, revoked, or complete; neither the secret nor a reusable bearer copy is persisted or logged.
- Resuming the parent creates a fresh capability but leaves the plan paused. The UI must approve the exact current plan revision again before the parent can launch more children.
- `OrchestrationStore` serializes each plan's read/validate/write transition under a per-plan mutex and atomically replaces one bounded JSON document; malformed or over-limit records fail closed without overwriting the source.
- Existing sessions gain optional `parentSessionId`, `planId`, and `planTaskId`; old session files deserialize with all three absent.
- Parent sessions persist `sessionMode: ordinary | orchestrator`; absent mode deserializes as ordinary. This marker lets the sidecar create a fresh parent capability when the user resumes an orchestrator after restart.
- Child session metadata is written with `parentSessionId`, `planId`, `planTaskId`, and `launchReservationId` before PTY spawn, so restart recovery can match an existing child to its durable task reservation.

- [ ] Write tests for accepting a valid plan, rejecting duplicate task IDs, unknown/self dependencies, dependency cycles, cross-worktree dependencies, invalid parallelism, oversized prompts/task counts, and noncanonical worktree paths.
- [ ] Write tests proving approval binds to an exact revision digest and edits require a strictly incremented revision.
- [ ] Test that proposal-time worktree path generation is stable and the exact absolute paths appear in the approved plan digest without creating filesystem paths.
- [ ] Write migration/deserialization tests for old session records and round-trip tests for new lineage fields.
- [ ] Test old session metadata defaults `sessionMode` to ordinary and orchestrator parent metadata retains its mode through terminal exit, sidecar restart, and resume.
- [ ] Implement plan validation and deterministic serialization/digest generation.
- [ ] Implement atomic bounded persistence and load-time validation using the repository's existing metadata write patterns.

## Task 3: Add parent capability and reuse UI authority

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Test: sidecar capability tests and Electron/preload contract tests

- [ ] Add an optional session creation mode `orchestrator`; ordinary session creation remains unchanged.
- [ ] For orchestrator sessions only, generate an OS-random plan-control capability, fail session creation closed if randomness is unavailable, inject it only into that parent's environment, and never persist or log it.
- [ ] In `SessionApplication::resume_session` and `resume_session_workflow`, detect persisted `sessionMode: orchestrator` and generate a fresh capability for the resumed parent runtime; ordinary session resume stays unchanged.
- [ ] Require orchestrator resume to pass through Electron main using `ORKWORKS_OPEN_PLAN_TOKEN`. The resumed plan remains paused until the UI approves its exact current revision again.
- [ ] Reuse the existing `ORKWORKS_OPEN_PLAN_TOKEN` held by Electron main and the sidecar for plan review/approval; do not create a second UI token or expose the existing token to renderer JavaScript.
- [ ] Verify `session_env_overrides` continues to filter `ORKWORKS_OPEN_PLAN_TOKEN`; add a regression test proving parent and child coding-tool processes receive no UI approval authority.
- [ ] Test that a parent capability cannot call UI approval routes, an ordinary session receives no plan-control capability, and a stale sidecar generation cannot approve a plan.

## Task 4: Implement plan proposal, approval, and scope revisions

**Files:**
- Create: `crates/orkworksd/src/http/orchestration_handlers.rs`
- Modify: `crates/orkworksd/src/http/mod.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/taskmaster/orchestration.rs`
- Test: handler and application tests in the same modules

**HTTP contract:**
- `POST /sessions/{parent_id}/orchestration/plans` accepts a plan proposal with `expectedRevision` and requires the parent's plan-control bearer capability.
- `GET /sessions/{parent_id}/orchestration/plan` returns the current revision and child task states to the parent capability or the Electron UI authority.
- `POST /sessions/{parent_id}/orchestration/plans/{revision}/approval` accepts `{ "decision": "approve" | "reject" }`, requires the existing Electron-main-only `ORKWORKS_OPEN_PLAN_TOKEN`, and rejects if the displayed revision/digest is stale.
- `POST /sessions/{parent_id}/orchestration/plans` with an approved plan creates a new proposed revision; it cannot mutate the approved revision in place.

- [ ] Test that a terminal-authored string or parent capability cannot approve/reject a proposal; verify the UI token is never present in a child session environment.
- [ ] Test exact-revision approval, stale approval rejection, immutable approved revisions, and scope expansion requiring a new approval.
- [ ] Test that no plan-owned worktree is created while a proposal is pending or rejected.
- [ ] Implement proposal/read/approval handlers and map application errors to stable HTTP statuses.

## Task 5: Launch declared child sessions through the existing runtime

**Files:**
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/http/orchestration_handlers.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: session view/projection tests

**HTTP contract:**
- `POST /sessions/{parent_id}/orchestration/tasks/{task_id}/launch` requires the parent's plan-control capability and takes the approved revision.
- A successful response returns the existing or new child `SessionInfo` including parent, plan, and task identifiers.
- A launch is rejected unless the plan is approved, the task is declared and not already assigned to another child, every dependency has status `reported_complete`, its harness/model match the approved task, no task in the same worktree group is still alive, and the concurrency ceiling has capacity.
- Repeating a launch for the same task returns its existing launch reservation/session; it never starts a duplicate. While creation is in progress, return HTTP 202 with the reservation ID and `launching` status.
- `POST /sessions/{parent_id}/orchestration/tasks/{task_id}/result` requires the parent capability and accepts `{ "expectedTaskVersion": 3, "result": "completed" | "failed" | "blocked", "summary": "..." }` only after that task's child session is terminal and its status is `needs_parent_result`. The sidecar atomically records the result and increments `task_version`; identical retries return the stored result, while conflicting duplicates or stale versions return conflict. `completed` advances dependencies but is not user acceptance.

- [ ] Test unauthorized, wrong-parent, stale-revision, unapproved, unknown-task, unmet-dependency, same-group concurrency, exhausted-concurrency, duplicate-launch, and premature-result behavior without spawning a PTY.
- [ ] Test a successful launch with the existing session runtime and verify child lineage persists across session listing and sidecar restart.
- [ ] Before approval, capture and display canonical repository root, `HEAD` SHA, and clean `git status`; reject approval if the repository head or clean state changed.
- [ ] At proposal time, assign each group the deterministic absolute path `<workspace_metadata_root>/taskmaster/worktrees/<plan_id>/<group_id>` and include it in the immutable revision. Do not create the directory or worktree before approval.
- [ ] After approval, create one branch/worktree per worktree group from the pinned base commit; subsequent dependent tasks reuse that group's worktree only after its prior child is terminal.
- [ ] If an approved path becomes occupied by a location not recorded as plan-owned, pause and propose a new revision with the replacement path; do not silently change the approved path.
- [ ] Add an internal validated `working_directory` to `CreateSessionCommand`; ordinary `POST /sessions` cannot select arbitrary cwd, while approved child launch supplies only the canonical plan-owned path.
- [ ] Preserve the active-workspace cwd default for every ordinary session; test that the approved child path reaches harness launch resolution and ordinary `POST /sessions` cannot choose an arbitrary cwd.
- [ ] Under a per-plan mutation lock, atomically persist a unique launch reservation and set the task to `launching` before spawning a child. A concurrent duplicate request sees the reservation and returns HTTP 202 instead of spawning again.
- [ ] Pass the reservation ID and plan/task lineage into child creation and persist them in child session metadata before PTY spawn. After successful creation, atomically attach the child session ID and set the task to `running`.
- [ ] On startup, reconcile every `launching` task against session metadata: attach a matching child without relaunching; if no child exists, set `launch_interrupted` and require a newly approved plan revision before retry.
- [ ] Persist the worktree path before child spawn and retain it if session creation fails so recovery is explicit; do not automatically remove branches or worktrees.
- [ ] Reuse `SessionApplication::create_session` for harness resolution, PTY startup, tokens, and ordinary lifecycle; do not introduce a second runtime or sidecar.
- [ ] On child exit, atomically move the task to `needs_parent_result`. Only a parent-authenticated result with the exact current task version, after the child is terminal, may set `reported_complete`, `reported_failed`, or `reported_blocked`; identical retries return the stored result and conflicting/stale results are rejected.
- [ ] Persist each task result and incremented task version in the same atomic plan-store replacement that advances its state, so a crash cannot record the result without its dependency transition.
- [ ] Test simultaneous launch requests for one task produce one reservation and at most one child.
- [ ] Test concurrent results for separate tasks in the same plan preserve both results and task versions.
- [ ] Test crash recovery both after reservation but before spawn (mark `launch_interrupted`) and after spawn but before task attachment (reconcile the child by its metadata reservation ID without a duplicate launch).
- [ ] Add tests for parent shutdown not implicitly killing children and for ordinary child kill/forget behavior remaining unchanged.

## Task 6: Expose lineage and plan progress to the desktop

**Files:**
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: `crates/orkworksd/src/session_view.rs`
- Modify: `crates/orkworksd/src/session_projection.rs`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/domain/session.ts`
- Modify: `apps/desktop/src/workspaceSessionController.ts`
- Test: Rust projection tests and `apps/desktop/tests/api.test.ts` plus focused session tests

- [ ] Add optional camelCase parent/plan/task fields to API session summaries and preserve absence for ordinary sessions.
- [ ] Add client response validation for plan revisions, task dependencies, child IDs, and allowed status enums.
- [ ] Add an API method to fetch the plan and children for a selected parent; refresh it using the existing polling/controller lifecycle.
- [ ] Test that hierarchy fields survive session sorting/merging and that sessions without orchestration fields render as before.

## Task 7: Add Orchestrator creation, approval, and hierarchy UI

**Files:**
- Modify: `apps/desktop/src/components/SessionListPanel.tsx`
- Create: `apps/desktop/src/components/OrchestrationPlanPanel.tsx`
- Modify: `apps/desktop/src/components/NewSessionDialog.tsx`
- Modify: `apps/desktop/src/App.tsx`
- Modify: `apps/desktop/src/api.ts`
- Modify: `apps/desktop/src/workspaceSessionController.ts`
- Modify: `apps/desktop/electron/main.ts`, `apps/desktop/electron/preload.ts`, and duplicate IPC contract types
- Modify: `apps/desktop/electron/planOpener.ts`
- Modify: `apps/desktop/src/components/DockviewApp.tsx` only if a new panel registration is needed
- Modify: focused component/API tests and existing session-list styles

- [ ] Add an explicit mode choice to `NewSessionDialog`; ordinary session stays the default, while Orchestrator creation sends `mode: "orchestrator"` and retains the user's goal, harness, and model. Carry the mode through `CreateSessionOptions`, `api.ts`, `workspaceSessionController.ts`, `App.tsx`, `CreateSessionRequest`, and `CreateSessionCommand`.
- [ ] Render parent rows with expandable child rows, child count, ordinary lifecycle status, and selection to each child's existing terminal.
- [ ] Render the complete proposed plan with task descriptions, dependencies, exact precomputed absolute worktree paths, harness/model choices, and parallelism before enabling Approve. Clarify that paths are reserved in the plan but not created until approval.
- [ ] Make approval and rejection call narrow Electron-main-owned preload methods; main reuses `ORKWORKS_OPEN_PLAN_TOKEN` and sends the exact displayed revision/digest. Display a stale-revision error and refresh instead of silently approving a newer plan.
- [ ] Route resuming an orchestrator parent through a narrow Electron-main-owned preload method using `ORKWORKS_OPEN_PLAN_TOKEN`; after resume, show the paused plan and require approval of the exact current revision before enabling child launch.
- [ ] Show which declared tasks are planned, ready, launching, running, interrupted during launch, waiting for a parent result, parent-reported complete/failed/blocked, or awaiting user scope approval. Never label a parent-reported result as user-accepted work.
- [ ] Test ordinary session creation remains unchanged, Orchestrator mode reaches the sidecar, keyboard navigation, nested row selection, approval payload revision, rejected/stale proposals, and ordinary session list accessibility.

## Task 8: Add bounded orchestration instructions and end-to-end coverage

**Files:**
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `apps/desktop/src/api.ts`
- Modify: `docs/agents/architecture.md`
- Modify: `docs/agents/domain-entities.md`
- Test: sidecar orchestration integration tests and desktop API/component tests

- [ ] Append the orchestrator operating instructions at session creation: plan the complete workflow, delegate every execution task to children, launch only approved ready tasks, report progress, and pause for user approval when adding/retrying/broadening work.
- [ ] Include the stable plan API paths and the parent capability usage in the prompt without including the UI-approval secret.
- [ ] Test a complete flow: start orchestrator in the New Session dialog → propose plan → approve exact revision through Electron main → launch independent task chains in separate worktrees → observe child exit as `needs_parent_result` → report a terminal child result → launch its dependent task in the same worktree → request wider scope → observe no launch until reapproval.
- [ ] Test identical task-result retry idempotency, conflicting/stale task-result rejection, and atomic task result/version persistence.
- [ ] Test parent end and sidecar restart pausing the plan, persisted orchestrator-mode recognition, capability revocation, UI-authorized parent resume with a fresh capability, exact-revision UI reapproval before more launches, no automatic child relaunch, and no detached child process-tree guarantees implied by UI status.
- [ ] Update architecture and domain references to document the plan store, token ownership, API flow, and workflow-vs-security boundary.

## Task 9: Verify and prepare the change for review

**Files:**
- Review all implementation files above

- [ ] Run focused Rust tests for orchestration, session creation, metadata migration, and HTTP handlers.
- [ ] Run `cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check` and `cargo test --manifest-path crates/orkworksd/Cargo.toml`.
- [ ] From `apps/desktop/`, run `pnpm exec tsc --noEmit` and `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`.
- [ ] Run the desktop API suite separately with `node --experimental-strip-types --test tests/api.test.ts`.
- [ ] Run the docs build/link check after updating authoritative docs.
- [ ] Run `/code-review medium` because this change crosses session lifecycle, authorization, and UI authority boundaries; resolve or document each finding.
- [ ] Update issue #610 with implementation status and close/update #617 according to the approved replacement scope.

## Reviewed design risks to resolve before implementation

- The current Taskmaster v1 spec and ADRs conflict with this proposal. Task 1 is a mandatory scope-alignment gate, not optional cleanup.
- UI approval must use Electron main's existing `ORKWORKS_OPEN_PLAN_TOKEN`, unavailable in coding-tool session environments. A renderer-only button or unauthenticated localhost route does not prove that approval came from the user.
- Plan approval limits OrkWorks child-launch requests. Without OS confinement, it cannot guarantee that a coding tool obeys its assigned prompt or avoids unrelated direct filesystem/process actions.
- A child session ending is not task success. Only an explicit, version-checked parent result can unlock declared dependencies, and even `reported_complete` is not proof of quality or user acceptance.
- Parallel worktrees must start from the exact clean base commit shown in the proposal. Dependency tasks in one chain reuse a worktree sequentially; independent task chains have separate worktrees.
- Preserve user decisions for product/architecture questions, ambiguous requirements, credentials/permissions, destructive actions, Git mutation or merge approval, conflicting high-confidence results, and high-risk acceptance.
- Worktree creation and restart failure handling must preserve recoverable metadata and must not silently delete branches or dirty worktrees.
