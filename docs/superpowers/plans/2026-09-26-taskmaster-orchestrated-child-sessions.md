# Taskmaster Orchestrated Child Sessions Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a user-approved parent session plan and coordinate a complete workflow, delegating every implementation and review task to ordinary OrkWorks child sessions.

**Architecture:** Extend the existing Rust session runtime with a small durable orchestration-plan store, an explicit UI approval path, parent-scoped launch/status APIs, and parent/child metadata. Use existing PTY sessions for all children and plan-owned Git worktrees for independent code-writing tasks. Extend the desktop session list and plan view to review the whole plan and show child progress. Orchestration approval governs OrkWorks launch requests; it does not confine commands run by coding tools.

**Tech Stack:** Rust, Axum, Tokio, serde, Electron main/preload IPC, React, TypeScript, existing PTY session runtime, Git worktrees.

**Spec:** [Taskmaster orchestrated child sessions](../specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md)

## Global Constraints

- A user must approve the exact complete plan revision before any plan-owned child session or worktree is created through the orchestration API.
- The parent session coordinates only; every implementation and review task in the workflow is delegated to a child session.
- Through the orchestration API, a child may launch only a declared task whose dependencies are complete; duplicate launch requests return the already-created child. This is not a host-process security boundary.
- Orchestration API requests must match the approved plan. This does not prevent a coding-tool process from calling the existing unauthenticated ordinary `POST /sessions` endpoint directly; OrkWorks does not provide a security boundary against same-user processes.
- A scope change, retry, harness/model change, or concurrency increase requires a new plan revision and explicit approval.
- User-authorized Orchestrator creation, approval, resume, and cancellation use the existing Electron-issued `ORKWORKS_OPEN_PLAN_TOKEN`. Electron main holds it and supplies it to the sidecar; it is withheld from renderer JavaScript and parent/child coding-tool environments. It is not an OS-enforced boundary against same-user processes that can inspect the sidecar environment.
- Orchestrator-mode creation uses a separate token-protected route that Electron main invokes for the UI flow. Generic `POST /sessions` remains unauthenticated for ordinary sessions, so a same-user coding-tool process can create ordinary sessions outside the approved orchestration plan.
- Child sessions use the existing workspace sidecar, session metadata store, PTY runtime, and ordinary session lifecycle.
- Parallel independent task chains use separate plan-owned worktrees. A dependent child may reuse its chain's worktree only after its predecessor session is terminal and the user confirms that no remaining process is using the worktree; this is a user acknowledgement, not OS proof. OrkWorks does not integrate, commit, merge, rebase, push, or automatically clean up child work.
- Workflow scope is not OS security confinement. Child processes retain the normal user permissions and harness login behavior.
- The parent reports child results to coordinate only the approved dependency graph. This report is not proof of quality or user acceptance. The scope-alignment task must amend the existing Taskmaster receipt requirement for this proposed orchestrator mode; user escalation and merge/acceptance gates remain in force.
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
| `crates/orkworksd/src/runtime/workspace_gc.rs` | Preserve workspace metadata while orchestration plans or allocated worktrees still need recovery; fail closed on malformed ownership records and keep dirty plan worktrees when the source workspace disappears. |
| `crates/orkworksd/src/runtime/retention.rs` and session forget handling | Protect orchestrator parent metadata while its plan is nonterminal, a child is live, an allocation is unreconciled, or any plan-owned worktree remains available for manual integration. |
| `crates/orkworksd/src/session_application.rs` | Route approved child creation through the existing session creation/runtime path and create assigned worktrees after approval. |
| `crates/orkworksd/src/http/session_handlers.rs` and `CreateSessionCommand` | Add an Orchestrator creation route protected by the existing UI token, separate from generic session creation, plus an internal validated cwd override for approved child launches. |
| `crates/orkworksd/src/git.rs` | Verify the approved clean base revision and provision unique plan-owned worktree paths/branches. |
| `crates/orkworksd/src/http/orchestration_handlers.rs` | Expose plan proposal, read, child launch, and child status operations with distinct parent and UI authority checks. |
| `crates/orkworksd/src/main.rs` and `crates/orkworksd/src/http/mod.rs` | Register orchestration routes and reuse the current sidecar-generation approval authority. |
| `crates/orkworksd/scripts/report-harness-event.sh`, `report-harness-event.ps1`, and `crates/orkworksd/src/harness/integrations/codex.rs` | Send an authenticated child turn-completion receipt for Codex `Stop`; any live running child without a recorded receipt can use the explicit UI action. |
| `crates/orkworksd/src/session_types.rs` and session projection modules | Return parent/child links, plan task labels, and ordinary lifecycle/status data to the desktop. |
| `apps/desktop/electron/main.ts`, `preload.ts`, and preload contract types | Keep the approval credential in Electron main and expose narrow review/approve/reject and manual turn-completion methods to the renderer. |
| `apps/desktop/electron/planOpener.ts` | Reuse the existing Electron-issued UI-token request pattern for user plan approvals; do not create a second UI token. |
| `apps/desktop/src/api.ts`, `apps/desktop/src/domain/session.ts`, and `apps/desktop/src/workspaceSessionController.ts` | Add validated orchestration plan/session-lineage types; route Orchestrator creation through Electron main while ordinary creation stays on the existing API path. |
| `apps/desktop/src/components/NewSessionDialog.tsx`, `apps/desktop/src/harnessTypes.ts`, and `apps/desktop/src/App.tsx` | Keep the mode choice in UI options and route ordinary versus Orchestrator creation to the appropriate existing or new path; ordinary remains the default. |
| `apps/desktop/src/components/SessionListPanel.tsx` and a focused orchestration plan component | Nest child sessions and show plan approval, task assignment, dependencies, and progress. |
| `apps/desktop/src/App.tsx` and session controller | Wire user approval actions, plan refresh, and child selection into existing app state. |

## Task 1: Align authoritative scope and architecture records

**Files:**
- Modify: `specs/taskmaster.md`
- Modify: `AGENTS.md`
- Modify: `docs/agents/product-boundaries.md`
- Modify: `specs/orkworks-mvp.md`
- Modify: `specs/multi-workspace.md`
- Modify: `README.md`
- Modify: `docs/validation/master-session-runner-confinement.md` (preserve as historical evidence; retire its gate for this design)
- Modify: `docs/superpowers/specs/2026-09-25-master-session-parallel-runner-design.md` (mark superseded by this proposed launch design)
- Create: `docs/adr/0066-taskmaster-orchestrated-child-sessions.md`
- Modify: `docs/adr/0060-independent-workspace-instances.md`
- Modify: `docs/adr/0064-bounded-taskmaster-coordinator.md`
- Modify: `docs/adr/README.md`
- Update: GitHub issues `#610` and `#617`

- [ ] Replace Taskmaster's v1 and coordinator-gate wording that excludes task decomposition, child launches, and plan-owned worktrees with the approved bounded orchestrator behavior; update `AGENTS.md`, `docs/agents/product-boundaries.md`, and `specs/orkworks-mvp.md` in the same scope-alignment task so repository guidance agrees.
- [ ] Align `specs/multi-workspace.md` with ordinary child sessions launched through the parent's existing sidecar; remove the conflicting dedicated-child-sidecar/native-proof prerequisite while preserving the single-workspace-per-instance invariant and manual integration boundary.
- [ ] Update `README.md` to describe this as the proposed Orchestrator design, not the old brokered runner, and keep all claims clear that it is not implemented.
- [ ] Retain `docs/validation/master-session-runner-confinement.md` as historical evidence, but state that its no-go applies only to the superseded confined-runner design and does not imply native confinement for this proposal.
- [ ] Mark the old master-session runner design superseded by this proposal so it no longer reads as the current launch architecture.
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
- `OrchestrationPlanRevision` contains immutable approved definition data (`id`, `parent_session_id`, revision, workspace identity, canonical repository root, clean base SHA, ordered batches, tasks, worktree groups with exact branch/path, harness/provider bindings, concurrency ceiling, and scope) and an approval digest calculated over this definition only. `OrchestrationExecutionState` stores mutable plan/task statuses, results, task versions, completion evidence, reservations, child IDs, and allocations separately. Runtime updates never change the approved digest.
- Every plan contains at least one task, and every task belongs to an explicit ordered batch. A task in a later batch cannot launch until every task in every earlier batch is parent-reported complete; task dependency edges still express within-batch and same-worktree-chain prerequisites. Cross-worktree task dependencies remain invalid.
- Each worktree allocation records the exact plan revision, canonical repository root, base commit, deterministic branch `orkworks/taskmaster/<plan-id>/<group-id>`, and approved path in a durable `preparing | ready | interrupted` allocation record before any Git mutation. The branch and path are included in the immutable plan digest and approval UI. Startup reconciliation adopts only an exact repository/branch/path/HEAD match; otherwise it leaves the allocation interrupted for explicit recovery.
- Allocation recovery is user-visible: adopt only an exact repository/branch/path/base match; retry only after verifying no branch or worktree was created; otherwise leave all artifacts untouched, cancel the allocation, and require a newly approved path/revision. OrkWorks never deletes an ambiguous partial allocation.
- `OrchestrationTask` contains `id`, `description`, `initial_prompt`, `depends_on`, `worktree_group_id`, `harness_id`, the exact harness-document revision/digest and resolved provider/model identity, closed `status`, `task_version`, and optional `launch_reservation_id` and `child_session_id`. Before dispatch, the sidecar revalidates that the approved binding still resolves to the same harness definition and provider/model identity; mutable defaults cannot silently change an approved task.
- Task status is `planned | ready | launching | running | needs_parent_result | awaiting_worktree_reuse_approval | reported_complete | reported_failed | reported_blocked | launch_interrupted`; store `completion_evidence` as `codex_stop_receipt | ui_action | child_terminal_without_receipt`. After a receipt or UI action, the parent reports the task outcome to coordinate its declared dependencies. A live task still `running` with no recorded receipt remains eligible for the explicit UI action, even when its Codex hook is verified; this covers a missed or delayed hook. A duplicate receipt after the UI action is idempotent and does not repeat the transition. If a child becomes terminal while its task is still `running`, atomically move it to `needs_parent_result` with `completion_evidence: child_terminal_without_receipt`; never infer success from process exit. `reported_complete` does not mean user acceptance. Before a dependent child reuses the chain worktree, the predecessor session must be terminal and the UI must collect the user's explicit quiescence acknowledgement. A failed/blocked task pauses new launches and requires an explicitly approved recovery/retry revision; already-running children retain their ordinary lifecycle and are not implicitly killed.
- Each worktree group has one canonical proposed absolute path under `~/.orkworks/taskmaster/worktrees/<workspace_hash>/<plan_id>/<group_id>`, outside `~/.orkworks/workspaces/<hash>/`, which startup GC may recursively remove. The path, exact branch, and IDs are part of the plan revision and displayed before approval; no directory or Git worktree exists yet. Startup GC must also preserve the workspace metadata root while a nonterminal plan, unreconciled allocation, or recorded plan worktree exists; malformed ownership records fail closed.
- Independent groups may run in parallel; tasks in one group run sequentially and reuse its working directory only after the terminal-session and user quiescence gate. Dependency edges may not cross worktree groups.
- Plan status is the closed enum `proposed | approved | paused | cancelled | complete`; a failed/blocked task moves the plan to `paused`, records the reason, and requires a newly approved recovery/retry revision before another launch. Atomically move an `approved` plan to `complete` when every task is `reported_complete`, every launch reservation is attached and settled, and there is no interrupted allocation or pending proposed scope revision. Live child PTYs do not delay completion; completion revokes the parent capability and prevents further launches, while child sessions and worktree records remain visible and available for ordinary management/manual integration.
- `max_parallel_children` counts live child sessions in any nonterminal lifecycle state plus launch reservations without an attached child. A child in `needs_parent_result` or `reported_complete` still consumes a slot while its ordinary session remains live; the slot releases only when that session is terminal.
- A plan-control bearer is an OS-random workflow credential bound in memory to one parent session and sidecar generation. It expires when the parent ends, the workspace/sidecar generation changes, or the plan is cancelled, revoked, or complete; it is never persisted or logged. It is injected into the parent coding-tool environment, so it is not confidential from same-user processes that can inspect that environment. While paused it remains valid for plan read/proposal and for reporting outcomes of already-launched children; it cannot launch new children. This lets the live parent collect outstanding results and propose a recovery revision without credential rotation.
- Resuming the parent creates a fresh capability but leaves the plan paused. The UI must approve the exact current plan revision again before the parent can launch more children.
- `OrchestrationStore` serializes each plan's read/validate/write transition under a per-plan mutex and atomically replaces one bounded JSON document; malformed or over-limit records fail closed without overwriting the source.
- Existing sessions gain optional `parentSessionId`, `planId`, and `planTaskId`; old session files deserialize with all three absent.
- Parent sessions persist `sessionMode: ordinary | orchestrator`; absent mode deserializes as ordinary. This marker lets the sidecar create a fresh parent capability when the user resumes an orchestrator after restart.
- Orchestrator mode is available only for harness definitions with a usable resume recipe. Plan approval and child launch remain disabled until the created parent has a captured, usable resume strategy and harness session identity; if identity capture fails, keep the plan unapproved and explain how to enable/repair identity reporting or cancel and recreate the parent with a supported harness. Resuming through the UI must use that captured identity.
- Child session metadata is written with `parentSessionId`, `planId`, `planTaskId`, and `launchReservationId` before PTY spawn, so restart recovery can match an existing child to its durable task reservation.

- [ ] Write tests for accepting a valid nonempty plan, rejecting an empty task list, duplicate task IDs, unknown/self dependencies, dependency cycles, cross-worktree dependencies, invalid parallelism, oversized prompts/task counts, and noncanonical worktree paths.
- [ ] Write tests proving approval binds to an exact revision digest and edits require a strictly incremented revision.
- [ ] Test that proposal-time worktree path generation is stable and the exact absolute paths appear in the approved plan digest without creating filesystem paths.
- [ ] Test ordered batch barriers prevent later-batch launch while any task in an earlier batch is incomplete.
- [ ] Test mutable execution-state updates leave the immutable approved-definition digest unchanged, and scope edits require a new definition digest and approval.
- [ ] Test live sessions in `needs_parent_result` and `reported_complete` continue consuming concurrency slots until terminal; test reservations consume slots before child attachment.
- [ ] Test that a terminal child whose task is still `running` moves atomically to `needs_parent_result` with `completion_evidence: child_terminal_without_receipt`, including kill, crash, and startup orphan reconciliation; verify no result is inferred and the parent must report completed/failed/blocked.
- [ ] Test that accepting the final `reported_complete` task atomically marks the plan `complete` only when all reservations are attached/settled and no interrupted allocation or pending scope revision remains; failed/blocked/incomplete tasks prevent completion. Verify live child PTYs do not delay completion and the capability is revoked.
- [ ] Test rejecting a pending scope revision reruns the completion check and completes a plan whose tasks and allocations are otherwise settled.
- [ ] Test that resolving the last interrupted allocation reruns the completion predicate when every task is already reported complete.
- [ ] Test Orchestrator creation rejects harnesses without a usable resume recipe (including Aider and Generic Shell), and missing parent resume strategy/session identity blocks approval and launch with a recoverable UI explanation.
- [ ] Test a paused plan still permits the live parent's authenticated read/proposal operations and version-checked results for already-launched children, but rejects new launches; ending or resuming the parent revokes/rotates the capability as specified.
- [ ] Test parent metadata cannot be forgotten or retention-pruned while its plan is nonterminal, a child is live, an allocation is unreconciled, or any plan-owned worktree remains; test a dirty plan worktree and its ownership metadata survive startup GC after the source workspace disappears.
- [ ] Write migration/deserialization tests for old session records and round-trip tests for new lineage fields.
- [ ] Test old session metadata defaults `sessionMode` to ordinary and orchestrator parent metadata retains its mode through terminal exit, sidecar restart, and resume.
- [ ] Implement plan validation and deterministic serialization/digest generation.
- [ ] Persist the immutable approved definition separately from execution state and hash only the definition; mutable task/result/reservation updates must preserve the approved digest.
- [ ] Implement atomic bounded persistence and load-time validation using the repository's existing metadata write patterns.

## Task 3: Add parent capability and reuse UI authority

**Files:**
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/runtime/terminal_runtime.rs`
- Test: sidecar capability tests and Electron/preload contract tests

- [ ] Add a narrow Orchestrator creation path invoked by Electron main using the existing Electron-issued `ORKWORKS_OPEN_PLAN_TOKEN` at `POST /sessions/orchestrator`; generic `POST /sessions` remains ordinary-only, including when its JSON contains an unrecognized `mode` field. Any same-user process that obtains the token can invoke this loopback route; the UI authority pattern is not OS-enforced against processes that can inspect the sidecar environment. Ordinary session creation remains unchanged.
- [ ] For orchestrator sessions created through the token-protected route only, generate an OS-random plan-control bearer, fail session creation closed if randomness is unavailable, inject it into the parent coding-tool environment, and never persist or log it. Document that same-user processes may inspect/replay it; it is a workflow credential, not an OS-enforced parent identity.
- [ ] In `SessionApplication::resume_session` and `resume_session_workflow`, detect persisted `sessionMode: orchestrator` and generate a fresh capability for the resumed parent runtime; ordinary session resume stays unchanged.
- [ ] Require orchestrator resume to pass through Electron main using `ORKWORKS_OPEN_PLAN_TOKEN`. The resumed plan remains paused until the UI approves its exact current revision again.
- [ ] Permit Orchestrator creation only when the selected harness definition has a usable resume recipe; revalidate that recipe at creation. Before plan approval or child launch, require a captured usable `resumeStrategy` and `harnessSessionId`. If the running parent never reports a resumable identity, keep the plan unapproved, disable approval/launch, show how to enable/repair identity reporting, and allow refresh/retry if identity arrives later. If it cannot be recovered, require cancellation/recreation with a supported harness; do not offer an implicit replacement-parent path.
- [ ] Reuse the existing `ORKWORKS_OPEN_PLAN_TOKEN` held by Electron main and available in the sidecar for Orchestrator creation, plan review/approval, and resume; do not create a second UI token or expose the existing token to renderer JavaScript or coding-tool environments. Document that same-user processes able to inspect the sidecar environment may obtain it.
- [ ] Verify `session_env_overrides` continues to filter `ORKWORKS_OPEN_PLAN_TOKEN`; add a regression test proving parent and child coding-tool processes receive no UI approval authority.
- [ ] Test generic `POST /sessions` cannot create an orchestrator, the token-protected `POST /sessions/orchestrator` route can, an ordinary session receives no plan-control bearer, and a stale sidecar generation cannot approve a plan. Do not claim protection from a same-user process that can copy either bearer.
- [ ] Test the paused parent's read/proposal and already-launched-child result authority remains valid until parent end or a sidecar/workspace generation change; new launches remain rejected.

## Task 4: Implement plan proposal, approval, and scope revisions

**Files:**
- Create: `crates/orkworksd/src/http/orchestration_handlers.rs`
- Modify: `crates/orkworksd/src/http/mod.rs`
- Modify: `crates/orkworksd/src/main.rs`
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/taskmaster/orchestration.rs`
- Test: handler and application tests in the same modules

**HTTP contract:**
- `POST /sessions/orchestrator` accepts the user-selected harness/model/goal only with the existing Electron-issued UI token in `ORKWORKS_OPEN_PLAN_TOKEN`; it always creates an orchestrator parent. Electron main invokes this token-protected route, and the sidecar validates the token. Generic `POST /sessions` remains ordinary-only and does not create an orchestrator even if its JSON contains an unrecognized mode field. The token does not establish OS process identity and may be exposed to same-user processes that can inspect the sidecar environment.
- `POST /sessions/{parent_id}/orchestration/plans` accepts a plan proposal with `expectedRevision` and requires the parent's plan-control bearer capability.
- `GET /sessions/{parent_id}/orchestration/plan` returns the current revision and child task states to the parent capability or the Electron UI authority.
- `POST /sessions/{parent_id}/orchestration/plans/{revision}/approval` accepts `{ "decision": "approve" | "reject" }`, requires the existing Electron-issued UI token in `ORKWORKS_OPEN_PLAN_TOKEN`, and rejects if the displayed revision/digest is stale.
- `POST /sessions/{parent_id}/orchestration/cancel` requires `ORKWORKS_OPEN_PLAN_TOKEN`, durably fences future launches, revokes the parent plan capability, and marks the plan cancelled. Already-running child sessions remain ordinary manageable sessions and are not implicitly killed; the UI continues to show them under the cancelled plan.
- `POST /sessions/{parent_id}/orchestration/plans` with an approved plan creates a new proposed revision; it cannot mutate the approved revision in place.
- Orchestrator creation rejects harnesses whose active definition has no usable resume recipe. Plan approval and launch require the parent session to have a captured usable resume strategy and harness session identity; an absent identity leaves the plan unapproved and surfaces a UI recovery message.
- [ ] Test that a terminal-authored string or parent capability cannot approve/reject a proposal; verify the UI token is never present in a child session environment.
- [ ] Test exact-revision approval, stale approval rejection, immutable approved revisions, and scope expansion requiring a new approval.
- [ ] Test cancellation/revocation rejects later parent launches and leaves already-running children visible and manageable through ordinary session controls.
- [ ] Test allocation recovery adopts only an exact repository/branch/path/base match, permits retry only when no Git branch/worktree artifacts exist, and leaves partial or ambiguous artifacts untouched.
- [ ] Test that no plan-owned worktree is created while a proposal is pending or rejected.
- [ ] Test the approved branch is shown beside the exact path, is included in the immutable digest, and branch/path collisions are detected before any Git mutation and require renewed approval.
- [ ] Test retention and explicit forget reject deletion of a parent whose plan, live children, unreconciled allocation, or retained plan-owned worktrees still need parent-linked recovery.
- [ ] Implement proposal/read/approval handlers and map application errors to stable HTTP statuses.

## Task 5: Launch declared child sessions through the existing runtime

**Files:**
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `crates/orkworksd/src/http/orchestration_handlers.rs`
- Modify: `crates/orkworksd/src/metadata.rs`
- Modify: `crates/orkworksd/src/session_types.rs`
- Modify: session view/projection tests
- Modify: `crates/orkworksd/scripts/report-harness-event.sh`
- Modify: `crates/orkworksd/scripts/report-harness-event.ps1`
- Modify: `crates/orkworksd/src/harness/integrations/codex.rs`
- Test: reporter and Codex integration tests

**HTTP contract:**
- `POST /sessions/{parent_id}/orchestration/tasks/{task_id}/launch` requires the parent's plan-control capability and takes the approved revision.
- A successful response returns the existing or new child `SessionInfo` including parent, plan, and task identifiers.
- For the initial automatic integration, the existing Codex `Stop` reporter posts an authenticated child turn-completion receipt. Add `POST /sessions/{child_id}/orchestration/turn-completion`, authenticated with that child's `Authorization: Bearer <ORKWORKS_REPORT_TOKEN>`. The handler derives the parent, plan, task, and reservation from stored child metadata, accepts only the supported Codex `Stop` event for a live child whose task is `running`, and idempotently moves it to `needs_parent_result`. Do not accept caller-supplied lineage as authority. Update both reporter scripts and Codex integration installation/configuration and tests to send this event. The UI fallback remains available for any live child whose task is still `running` with no recorded receipt, including a verified Codex hook that failed or has not delivered its event.
- `POST /sessions/{parent_id}/orchestration/tasks/{task_id}/turn-completion` is the Electron-authorized UI fallback for any live child whose task is `running` and has no recorded receipt, whether or not its completion integration is verified. It requires `ORKWORKS_OPEN_PLAN_TOKEN`, the exact approved plan revision, launch reservation ID, child session ID, and expected task version; it is accepted only while that child is nonterminal and the task is `running`, then idempotently moves it to `needs_parent_result`. If an authenticated receipt arrives after this transition, return idempotent success without applying it twice.
- If a child reaches a terminal lifecycle state while its task remains `running` because the receipt was absent, atomically move the task to `needs_parent_result` with `completion_evidence: child_terminal_without_receipt`. Allow the parent to report `completed`, `failed`, or `blocked` for that exact task version; do not infer a result from exit status. Cover normal terminal transitions and startup/orphan reconciliation so a missed hook cannot strand a plan.
- A launch is rejected unless the plan is approved, the task is declared and not already assigned to another child, every dependency is parent-reported `reported_complete`, its pinned harness-document revision and resolved provider/model identity still match, any predecessor in the same worktree group is terminal and has a recorded user quiescence acknowledgement, and the concurrency ceiling has capacity.
- An ordered batch is launchable only after every task in every earlier batch is parent-reported complete; same-batch tasks may launch in parallel subject to the live-child ceiling.
- The live-child ceiling counts all nonterminal child sessions, including tasks in `needs_parent_result` and `reported_complete`, plus unattached launch reservations. A slot is released only when its ordinary child session reaches a terminal lifecycle state.
- Repeating a launch for the same task returns its existing launch reservation/session; it never starts a duplicate. While creation is in progress, return HTTP 202 with the reservation ID and `launching` status.
- `POST /sessions/{parent_id}/orchestration/tasks/{task_id}/result` requires the parent capability and accepts the exact approved plan revision, launch reservation ID, child session ID, expected task version, and `{ "result": "completed" | "failed" | "blocked", "summary": "..." }` only after the task is `needs_parent_result`. The sidecar records the parent-authenticated coordination report and increments `task_version`; `completed` unlocks only declared dependents and is not user acceptance. In the same atomic store transition, if this is the final task, move the plan to `complete` only when all tasks are `reported_complete`, every reservation is attached with no `launching` or `launch_interrupted` task, no allocation is interrupted, and no proposed scope revision is pending. Failed/blocked results pause the plan; incomplete or unsettled state does not complete it. Live child PTYs do not delay completion, but remain visible and manageable through ordinary session controls. While paused, this route remains available only for already-launched children on the exact paused revision; it cannot authorize a new launch. Identical retries return the stored result; conflicting, cross-revision, wrong-child, or stale versions return conflict.
- `POST /sessions/{parent_id}/orchestration/tasks/{task_id}/worktree-reuse-approval` requires `ORKWORKS_OPEN_PLAN_TOKEN` and an exact task/worktree-group revision. It succeeds only after the predecessor session is terminal, records the user's explicit quiescence acknowledgement, and does not claim the sidecar proved detached descendants exited.

- [ ] Test unauthorized, wrong-parent, stale-revision, unapproved, unknown-task, unmet-dependency, same-group concurrency, exhausted-concurrency, duplicate-launch, and premature-result behavior without spawning a PTY.
- [ ] Test the Electron-authorized manual turn-completion action moves a live `running` child with no receipt to `needs_parent_result`, checks revision/reservation/session/task-version identity, and rejects stale or mismatched requests; verify it is offered for Codex children both with and without a verified active hook.
- [ ] Test the authenticated child receipt route rejects missing/wrong child report tokens, derives lineage from child metadata, accepts Codex `Stop` once idempotently, rejects unrelated event types and terminal tasks, and returns idempotent success if the UI fallback already moved that task to `needs_parent_result`; update reporter/integration tests.
- [ ] Test child terminalization before receipt (including startup reconciliation) moves `running` to `needs_parent_result` with explicit missing-receipt evidence and permits only a version-checked parent result; process exit never implies success.
- [ ] Test final successful result atomically transitions the plan to `complete` only if every task is `reported_complete`, all reservations are attached/settled, and no allocation is interrupted or scope revision pending. Failed/blocked/incomplete plans stay non-complete; live child PTYs may remain open, remain visible, and do not delay completion. Verify completion revokes the plan bearer and blocks future launches.
- [ ] Test a successful launch with the existing session runtime and verify child lineage persists across session listing and sidecar restart.
- [ ] Before approval, capture and display canonical repository root, `HEAD` SHA, and clean `git status`; reject approval if the repository head or clean state changed.
- [ ] Capture the exact harness-document revision and resolved harness/provider/model identity in the approved plan; before dispatch, compare the active harness snapshot against that binding and pause for a new approved revision on any mismatch.
- [ ] At proposal time, assign each group the deterministic absolute path `~/.orkworks/taskmaster/worktrees/<workspace_hash>/<plan_id>/<group_id>` and branch `orkworks/taskmaster/<plan_id>/<group_id>`, outside the recursively collectible workspace metadata root, and include both in the immutable revision and approval UI. Do not create the directory or worktree before approval.
- [ ] Before `git worktree add`, atomically persist a `preparing` allocation with the approved plan revision, repository root, base SHA, branch, and exact path. On restart, verify and adopt an exact matching allocation; mark partial or mismatched allocations `interrupted` and require explicit recovery rather than guessing ownership.
- [ ] After approval, create one worktree per independent chain from the pinned base commit. Before any dependent child reuses it, require a terminal predecessor session and a separate UI confirmation that the user checked the worktree is quiescent; persist this acknowledgement against the exact task/group revision and do not describe it as OS proof.
- [ ] If an approved path becomes occupied by a location not recorded as plan-owned, pause and propose a new revision with the replacement path; do not silently change the approved path.
- [ ] Check branch-name collisions as well as path collisions before mutation; on either collision, pause and require a newly approved branch/path instead of silently selecting another.
- [ ] Add an internal validated `working_directory` to `CreateSessionCommand`; ordinary `POST /sessions` cannot select arbitrary cwd, while approved child launch supplies only the canonical plan-owned path.
- [ ] Preserve the active-workspace cwd default for every ordinary session; test that the approved child path reaches harness launch resolution and ordinary `POST /sessions` cannot choose an arbitrary cwd.
- [ ] Under a per-plan mutation lock, atomically persist a unique launch reservation and set the task to `launching` before spawning a child. A concurrent duplicate request sees the reservation and returns HTTP 202 instead of spawning again.
- [ ] Pass the reservation ID and plan/task lineage into child creation and persist them in child session metadata before PTY spawn. After successful creation, atomically attach the child session ID and set the task to `running`.
- [ ] On startup, reconcile every `launching` task against session metadata: attach a matching child without relaunching; if no child exists, set `launch_interrupted` and require a newly approved plan revision before retry.
- [ ] Persist the worktree path before child spawn and retain it if session creation fails so recovery is explicit; do not automatically remove branches or worktrees.
- [ ] In `workspace_gc`, keep the workspace metadata directory while any nonterminal plan, unreconciled allocation, or recorded plan worktree exists; malformed ownership records fail closed. Test that a dirty worktree is never recursively removed just because its original workspace path vanished.
- [ ] In retention and explicit forget, protect parent session metadata while its plan is nonterminal, any child session is live, an allocation is unreconciled, or a plan-owned worktree still exists for manual integration; return a conflict that directs the user to finish/cancel or resolve/remove the retained worktree first.
- [ ] Reuse `SessionApplication::create_session` for harness resolution, PTY startup, tokens, and ordinary lifecycle; do not introduce a second runtime or sidecar.
- [ ] Before child launch, materialize the approved harness's existing workspace-local reporter integration into the allocated worktree using the current ownership/revision checks; never overwrite a conflicting user-owned configuration. Bind the integration generation to approval and pause for a new revision if it changed.
- [ ] On an authenticated child turn-completion receipt (or explicit UI action whenever a live child task remains `running` without a recorded receipt), atomically move the task to `needs_parent_result` without requiring the PTY to exit. The parent may report `completed`, `failed`, or `blocked` with the exact current task version; `completed` unlocks only declared dependencies and never indicates user acceptance. Identical retries return the stored result and conflicting/stale results are rejected. Failed/blocked results pause new launches; no automatic retry occurs.
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

- [ ] Add an explicit mode choice to `NewSessionDialog`; ordinary session stays the default. Carry the choice in UI-only `CreateSessionOptions` through `App.tsx`; ordinary creation keeps using the existing controller and generic `POST /sessions`, while Orchestrator creation uses a narrow preload/main method to the token-authorized `POST /sessions/orchestrator`, retaining the user's goal, harness, and model. Do not pass an orchestrator mode field through generic `api.ts` session creation or `CreateSessionRequest`.
- [ ] Render parent rows with expandable child rows, child count, ordinary lifecycle status, and selection to each child's existing terminal.
- [ ] Render the complete proposed plan with task descriptions, dependencies, exact precomputed worktree branch and absolute path, harness/model choices, ordered batches, and parallelism before enabling Approve. Clarify that paths are reserved in the plan but not created until approval, and that approval governs the orchestration API rather than same-user direct sidecar calls or shell commands.
- [ ] Make approval and rejection call narrow preload methods handled by Electron main; main reuses the Electron-issued `ORKWORKS_OPEN_PLAN_TOKEN` and sends the exact displayed revision/digest. Display a stale-revision error and refresh instead of silently approving a newer plan.
- [ ] Filter Orchestrator mode choices to harnesses with a usable resume recipe; the sidecar independently rejects unsupported choices. Keep plan approval and launch disabled until the parent reports a usable resume strategy and harness session identity. Show how to enable/repair identity reporting or refresh for a late identity; require cancel/recreate with a supported harness only if identity cannot be recovered. Add UI tests for unsupported harnesses and missing identities.
- [ ] Add an Electron-authorized Cancel Orchestration action that fences future launches and revokes the parent capability; show that already-running children remain visible and can be managed through ordinary session controls.
- [ ] Before launching a dependent task into a reused worktree, require a separate Electron-authorized user acknowledgement after the predecessor session is terminal. Explain that OrkWorks cannot detect detached processes and that this acknowledgement is the user's quiescence check.
- [ ] For any live child task still `running` with no recorded receipt, expose an Electron-authorized "Mark turn ready for parent review" action (including Codex when its verified Stop hook missed or delayed the receipt); bind it to the exact revision, reservation, child session, and task version, and refresh the plan after success.
- [ ] When a child ends before a receipt, show the task as awaiting parent result with missing-receipt evidence; allow the parent to report the outcome without treating the terminal status as success.
- [ ] Render interrupted allocation recovery choices: adopt only an exact verified allocation, retry only after verifying no Git artifacts exist, or abandon/cancel and propose a new approved path. Never silently delete partial or ambiguous artifacts.
- [ ] Route resuming an orchestrator parent through a narrow Electron-main-owned preload method using `ORKWORKS_OPEN_PLAN_TOKEN`; after resume, show the paused plan and require approval of the exact current revision before enabling child launch.
- [ ] Show which declared tasks are planned, ready, launching, running, interrupted during launch, waiting for a parent result, parent-reported complete/failed/blocked, or awaiting user scope approval. Never label a parent-reported result as user-accepted work.
- [ ] Test ordinary session creation remains unchanged, Orchestrator mode reaches the sidecar, keyboard navigation, nested row selection, approval payload revision/branch/path, manual turn-completion authorization, cancellation authorization, interrupted allocation recovery choices, rejected/stale proposals, and ordinary session list accessibility.

## Task 8: Add bounded orchestration instructions and end-to-end coverage

**Files:**
- Modify: `crates/orkworksd/src/session_application.rs`
- Modify: `apps/desktop/src/api.ts`
- Modify: `docs/agents/architecture.md`
- Modify: `docs/agents/domain-entities.md`
- Test: sidecar orchestration integration tests and desktop API/component tests

- [ ] Append the orchestrator operating instructions at session creation: plan the complete workflow, delegate every execution task to children, launch only approved ready tasks, report progress, and pause for user approval when adding/retrying/broadening work.
- [ ] Include the stable plan API paths and the parent capability usage in the prompt without including the UI-approval secret.
- [ ] Test a complete flow: start orchestrator in the New Session dialog → propose plan → approve exact revision through Electron main → launch independent task chains in separate worktrees → receive an authenticated child turn-completion receipt (or use the Electron-authorized UI action) while the child session remains manageable → parent reports the child result → end the predecessor session → acknowledge worktree quiescence in the UI → launch its dependent task → request wider scope → observe no launch until reapproval.
- [ ] Test Codex `Stop` automatically moves a live task to `needs_parent_result`; test a killed/crashed child without receipt reaches parent-result review through terminal recovery and requires an explicit parent outcome.
- [ ] Test a Codex child with a verified active hook but no recorded receipt can use the explicit UI turn-completion fallback; a delayed receipt after that action is idempotent.
- [ ] Test the final successful task moves the plan to `complete`, revokes its capability, and leaves still-live child sessions visible and ordinarily manageable.
- [ ] Test Aider/Generic Shell cannot create Orchestrator parents, and a supported harness without a captured resume identity cannot approve a plan until identity reporting is repaired or the parent is recreated with valid identity reporting.
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
- The existing coordinator contract requires server-attested success receipts. Task 1 must amend that contract for this Orchestrator mode: a version-checked parent result advances only the user-approved dependency graph, and never means user acceptance. This is an intentional trust change that relies on the approved plan and parent coordination rather than a process broker.
- UI approval must use Electron main's existing `ORKWORKS_OPEN_PLAN_TOKEN`, unavailable in coding-tool session environments. A renderer-only button or unauthenticated localhost route does not prove that approval came from the user.
- Plan approval limits OrkWorks child-launch requests. Without OS confinement, it cannot guarantee that a coding tool obeys its assigned prompt or avoids unrelated direct filesystem/process actions.
- A child session ending is not task success. Only an explicit, version-checked parent result can unlock declared dependencies, and even `reported_complete` is not proof of quality or user acceptance.
- Parallel worktrees must start from the exact clean base commit shown in the proposal. Dependency tasks in one chain reuse a worktree sequentially only after a user quiescence acknowledgement; independent task chains have separate worktrees.
- Preserve user decisions for product/architecture questions, ambiguous requirements, credentials/permissions, destructive actions, Git mutation or merge approval, conflicting high-confidence results, and high-risk acceptance.
- Worktree creation and restart failure handling must preserve recoverable metadata and must not silently delete branches or dirty worktrees.
