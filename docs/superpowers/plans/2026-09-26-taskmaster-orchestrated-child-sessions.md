# Taskmaster Orchestrated Child Sessions Implementation Plan

> **For agentic workers:** Use superpowers:executing-plans for the documentation
> alignment task below. Runtime units are planning boundaries, not executable
> tasks; each needs its own reviewed specification and approved execution plan.
> Steps use checkbox (`- [ ]`) syntax. Work in an owned worktree.

**Goal:** Align #610's documentation with ordinary child sessions and prepare
independently testable runtime units after the remaining contract/evidence gates.

**Architecture:** Children reuse the selected workspace's existing sidecar,
metadata store and ordinary PTY lifecycle. A UI-created run binds immutable
parent bootstrap/goal/capacity; its planning bearer is separate from each exact
approved plan's server-held launch grant. Approved independent task chains use
separate worktrees, leaving code for manual integration.

**Tech Stack:** Markdown, GitHub issue tracking and VitePress for this task;
later scoped units use Rust/Axum/Tokio and Electron/React/TypeScript.

**Spec:** [Scope alignment](../specs/2026-10-04-taskmaster-orchestration-scope-design.md),
[ordinary-child baseline](../specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md),
[preparation lifecycle](../specs/2026-10-04-orchestrator-preparation-design.md).

## Status and global constraints

This replaces the September plan's stale one-plan-bearer execution sequence.
The old confined-runner [plan](2026-09-25-master-session-parallel-runner.md)
and [native validation](../../validation/master-session-runner-confinement.md)
remain historical. ADR 0066 is already hook-owned prompt attention; the new
accepted scope decision is [ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md).

- Scope was accepted on 2026-10-04 for PR #747. Component contracts and runtime code remain separately gated.
- Parent-only delegation; explicit ordered batches/dependencies; one attempt per task; no recursive child control or automatic retries.
- Exact-plan UI approval precedes every plan-owned branch/worktree/child. A run bearer alone cannot launch.
- Same live parent prepares execution after research; research grant ends, planning continues, every new execution plan requires its own exact approval.
- Run-wide live-child count includes all plans and unattached reservations; a task/plan result does not free a live child's slot.
- Independent chains use separate worktrees. Sequential same-plan reuse needs a terminal predecessor and explicit user quiescence acknowledgement; no cross-plan reuse.
- Parent results coordinate declared dependencies only; quality, completeness and user acceptance remain separate.
- Ordinary lifecycle and exact-identity UI resume/reapproval; no automatic child relaunch or replacement-parent fallback.
- No OS confinement, credential broker, child sidecars, hard resource budgets or same-user process-origin claims. Independent-instance cleanup/replacement proof remains binding.
- No automatic integration, code transfer, commit, merge, rebase, push, existing-branch change, branch deletion or automated worktree cleanup.
- Preserve one selected terminal, Electron-main authority and independently defined preload/renderer types; ordinary sessions default to absent lineage/ordinary mode.
- Apply reviewed configuration/preparation/report/history bounds before mutation. Lower verified adapter limits win; exhaustion preserves proof and blocks admission.
- Use RTK and pnpm. Before source changes read both scoped AGENTS files, architecture/domain references and the approved unit specification.

## Task 1: Reconcile authoritative scope and ADR history

**Files:**
- Modify: `specs/taskmaster.md`, `specs/orkworks-mvp.md`, `specs/multi-workspace.md`.
- Modify: `AGENTS.md`, `README.md`, `docs/agents/product-boundaries.md`, `docs/agents/architecture.md`.
- Create: `docs/adr/0077-taskmaster-orchestrated-child-sessions.md` and `docs/superpowers/specs/2026-10-04-taskmaster-orchestration-scope-design.md`.
- Modify: `docs/adr/0060-independent-workspace-instances.md`, `docs/adr/0064-bounded-taskmaster-coordinator.md`, `docs/adr/README.md`.
- Modify: September 24/25 coordinator/runner proposals, September 25 historical runner plan, September 26 ordinary-child baseline, October 4 hierarchy/preparation/work-specification routing and `docs/validation/master-session-runner-confinement.md`.

**Interfaces:**
- Consumes: user-authorized specification direction, #610/#617's older criteria, accepted workspace/session ownership decisions, proposed #741/#742 contracts and #740 evidence register.
- Produces: an accepted scope decision and aligned issue criteria; scope acceptance is distinct from runtime approval.

- [x] Compare each conflicting statement with its Decision/Consequences; preserve accepted independent-instance ownership, supersede proposed broker architecture and only ADR 0060's conflicting runner amendments.
- [x] Align the run planning bearer/exact-plan grant lifetime, research-to-execution transition and run-wide child capacity; retire the old one-plan authority instructions.
- [x] Preserve the old no-go evidence as historical, allocate ADR 0077 without overwriting 0066, and stage #610/#617 disposition in the scope artifact until acceptance.
- [x] Validate the concrete draft with `rtk git diff --check`, `rtk proxy bash scripts/doc-check.sh` and `rtk proxy pnpm --dir docs docs:build`; inspect output.
- [x] Obtain written scope acceptance; then update #610's criteria and retire #617 as superseded, without closing #610 or claiming eligibility.

## Repository identity draft: 2026-10-04

The early #745 input has a [proposed repository-identity contract](../specs/2026-10-04-configuration-learning-design.md)
and [documentation task plan](2026-10-04-repository-identity-specification.md).
It specifies local common-directory registration, replacement/relocation
behavior, exact configuration/approval/resume binding and finite persistence.
Written contract review, filesystem/platform evidence and executable code
planning remain open. It does not complete #745's downstream learning/history
scope or satisfy baseline unit 4's reviewed identity prerequisite yet.

## Runtime planning boundaries

No unit below has an approved executable code plan. Create its tracker only
after authoritative scope acceptance and relevant detailed contract review;
link it to #610/#738 rather than expanding those umbrella issues into one large
PR. Capture exact APIs/types, migrations, source/test files and red/green
commands in each subsequent plan. Units sharing a file execute sequentially;
the table is dependency order, not an instruction to dispatch parallel writers.

| Unit | Consumes | Produces | Candidate source ownership | Required acceptance evidence |
| --- | --- | --- | --- | --- |
| 1. Data-only run/plan persistence | Accepted alignment and reviewed #741/#742 identities/bounds | Immutable run/bootstrap/plan definitions; separate execution state; bounded admission/replay fence; no runtime authority | New `taskmaster/orchestration.rs`, `orchestration_store.rs`; `metadata.rs`; existing coordinator persistence only where compatible | Digest stability; strict cross-language counters/generations; invalid/malformed/over-limit load rejection; crash-safe admission; forgotten-ID replay rejection; ordinary deserialization |
| 2. Configuration/adapter eligibility | #740 version-specific role/delivery/resume/event evidence and reviewed #741 composition/digests | Closed exact-profile eligibility and startup binding with deny reasons | `harness/definition.rs`, `registry.rs`, compiled integrations; isolated adapter fixtures | Required rules/skills delivered; instruction/model/path/action drift denies; inherited shell/network/MCP bypass probes; no silent fallback; exact resume and event-loop continuation |
| 3. Run authority and UI approval | Units 1–2 and reviewed #742 lifecycle | Volatile generation-bound run bearer; server-held exact-plan grant; UI creation/resume/approval/revocation | `session_application.rs`, `http/session_handlers.rs`, new `http/orchestration_handlers.rs`, `runtime/terminal_runtime.rs`, main/router; narrow Electron-main/preload methods | Ordinary-only generic creation; bearer cannot approve/launch without grant; exact input/config revalidation; old grant rejection; token filtering; cancellation/launch race; parent resume and between-stage planning |
| 4. Worktree allocation and ordinary child admission | Units 1–3, approved launch definition and the reviewed repository-identity-across-worktrees portion of #745 required by #741 | Durable allocation/reservation and one child linked to exact run/plan/task/group | `git.rs`, `session_application.rs`, `metadata.rs`, `runtime/workspace_gc.rs`, `retention.rs` | Resolved repository/worktree binding; foreign or ambiguous identity denies before allocation/launch; clean base; exact branch/path collision denial; intent before mutation; concurrent duplicate launch; recovery before/after spawn; no foreign adoption/relaunch; run cap includes prior plans/reservations; terminal+acknowledged same-group reuse; serialized child-resume/group handoff and capacity; predecessor resume denied after durable successor reservation |
| 5. Turn readiness, coordination and recovery | Units 1–4; verified turn-event adapter | Authenticated readiness evidence; version-bound parent outcomes; declared batch/dependency gates | `http/orchestration_handlers.rs`, orchestration owner; reporter scripts/integrations and ordinary terminal reconciliation | Receipt/UI races; missed receipt fallback; exit never means success; stale/conflicting outcomes rejected; atomic completion; failed/blocked pause; no retries; ordinary children survive parent exit |
| 6. Preparation/report continuation | Units 1–5 and reviewed #742 report/event contracts | Bounded clarification/research reports/input manifests; same-parent event tool; synthesis/proposal transition | Orchestration owner, scoped report/event handlers and adapter; narrow UI answer/proposal methods | Report producer generations before dedupe; receipt distinct from task result; corrected inputs invalidate proposals; event gaps refresh; research completion removes grant but preserves planning; final completion and exact UI research-only/decline-execution finish fence all run authority; unsettled allocation/finish races denied |
| 7. Usage/evaluation/history | Final #743/#744 contracts; #745 consumes both | Dedicated skill evidence and independent assignment evaluation, then repository-local future advice | Separate scoped evidence/evaluation/history owners; no workflow-observation format reuse | Delivery distinct from usage; missing remains unknown; authenticated bounded replay/retention/deletion; result-revision binding; conflicts not averaged; advice cannot change active approval/profile |
| 8. Desktop projection and approval surface | Units 1–6 and final #741/#743/#744/#746 interfaces | Validated lineage/run/task projections, one-terminal hierarchy and reviewed approval UI | `session_types.rs`, `session_view.rs`, `session_projection.rs`; `src/api.ts`, `domain/session.ts`, controller/list/detail/new-session components; independent `electron/` IPC copies | Ordinary fallback; selected session restoration; stale workspace/runtime results fenced; keyboard/focus/reduced motion; blockers/capacity visible; exact displayed digest approval; badges/evaluation evidence accurate |

Units 1–6 are the launch/preparation baseline. Before unit 4, finalize/review
#745's repository-identity-across-worktrees contract consumed by #741; an
unresolved binding blocks worktree admission and child launch. This identity
prerequisite is separate from later usage/evaluation/history learning, so it
cannot wait for unit 7. Unit 7 splits into usage,
evaluation and then learning, each its own scoped plan/PR. Unit 8 may draft
mockups independently, but implementation consumes final projections. It also
needs separate approval/main and renderer plans if the diff exceeds the repo's
review thresholds. No evidence implies that all units are currently ready.

### Contract cases that each scoped plan must carry

- Approved definitions remain immutable through mutable results/reservations; changed instructions, input manifests, base or tool/model generations require a new exact revision/approval.
- Only one current grant per run; paused stages allow collection but not launch. Stage cancellation requires explicit UI continuation; run cancellation ends authority.
- Research children may remain live after their plan completes; execution approval does not free capacity or terminate them. Worktree artifacts are not copied across plans.
- Owned-child resume revalidates current group ownership and live-child capacity under the same admission lock as successor reservation. Handoff durably denies predecessor resume thereafter; a pre-handoff resume invalidates old quiescence evidence. Ordinary no-lineage resumes remain unchanged.
- Research-only/no-go/decline-execution has an exact version-bound UI finish transition to `complete`; revoke bearer/grants and invalidate pending proposals without execution allocation, quality acceptance or cancelled disposition.
- Pending scope revisions/unsettled reservations/interrupted allocations block completion; clearing a blocker reruns the predicate atomically. Final completion does not require child PTYs to exit and is not user acceptance.
- Turn readiness can come from an authenticated verified adapter, an exact Electron-authorized UI action or terminal reconciliation. Reports and readiness alone do not advance dependencies.
- Parent end, sidecar/workspace replacement and runtime resume fence old generations. Missing exact native identity blocks resume; no terminal typing or scheduled replacement session supplies event continuation.
- Parent metadata, allocation records and referenced report/evidence survive retention/forget/GC while live work or manual-integration artifacts need them. Malformed ownership fails closed.
- Worktree materialization of an already approved owned integration uses existing ownership/revision/confirmation rules. No tracked/user configuration is overwritten and no installation is assumed from a capability declaration.
- Mandatory user escalations remain: product/architecture, ambiguous requirements, credentials/permissions, destructive actions, Git mutation/merge, conflicting high-confidence results and high-risk acceptance.

## Execution approval and verification handoff

For each unit, first re-read the accepted scope and final consuming contracts.
Create a PR-sized implementation tracker and a written plan with exact signatures,
fixtures and meaningful failure expectations. Obtain its explicit approval
before source changes. Follow TDD for runtime changes; use disposable helpers
and primary-source payload fixtures rather than live coding agents in tests.

Run focused tests, then the scoped AGENTS-required Rust build/test/fmt or desktop
typecheck/tests and documentation checks. Use explicit `/code-review low` for
small code diffs; lifecycle/protocol/concurrency/authority changes require the
documented escalation, usually medium. Required CI and all actionable review
dispositions gate merge. Do not revive the old single oversized runtime PR.

Keep #610 open until the accepted runtime criteria are actually delivered.
#617 retires only after scope acceptance, with a supersession reason. Keep
#738/#740–#746 open for their own contract/evidence handoffs. No new coding tool,
production role profile or successful runtime is claimed by this plan.
