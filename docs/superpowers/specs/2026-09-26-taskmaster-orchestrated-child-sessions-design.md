# Taskmaster orchestrated child sessions

- Status: proposed
- Date: 2026-09-26
- Tracking issue: [#610](https://github.com/Rambolarsen/orkworks/issues/610)
- Supersedes as the launch design: [master-session parallel runner proposal](2026-09-25-master-session-parallel-runner-design.md)
- Replaces the native-boundary prerequisite for this scope: [native child-launch boundary design](2026-09-25-taskmaster-native-child-launch-boundary-design.md)

## Purpose

Let a user-approved OrkWorks session act as an orchestrator for the whole
workflow. The parent plans and coordinates the work; all implementation and
review tasks in that workflow are delegated to ordinary OrkWorks child
sessions. The user approves the complete bounded workflow once, and the parent
starts its declared children. OrkWorks records the parent/child relationship
and presents the hierarchy and progress in the existing Sessions UI. A
requested task outside the approved plan pauses for a new user approval.

A child is a normal OrkWorks session created and owned by the active sidecar's
existing session runtime. This design does not create a second session
runtime, a child-specific process supervisor, or an OS sandbox.

## User flow

1. In the New Session dialog, the user selects **Orchestrator** mode, chooses
   the coding tool/model, and gives the parent its goal. The parent's role is
   planning, delegation, and tracking; it does not carry out the workflow's
   implementation or review tasks itself. Ordinary session creation remains
   the default and is unchanged.
2. The parent proposes the complete, bounded workflow as a task list with
   task dependencies, requested harness/model choices, repository scope, and
   parallelism. Every execution task is assigned to a child session.
3. OrkWorks renders that proposal in the UI. The user can review and approve
   the exact revision or reject it.
4. The approved parent launches ready tasks from the declared plan. OrkWorks
   checks that each request matches the approved plan and starts it through
   the normal session creation path. A harness turn-completion/idle event
   moves the assigned task to parent-result review; the PTY session may remain
   open at its prompt. The parent reports the result to advance declared
   dependencies without changing the approved plan. Before a dependent child
   reuses the worktree, the prior session must end and the user must confirm
   that the worktree is quiescent.
5. The Sessions panel shows the children under their parent, with ordinary
   session status and terminal selection. The parent tracks child progress,
   collects summaries, and coordinates the next declared tasks through the
   orchestration interface.
6. A request for an additional task, a broader repository/worktree scope, a
   higher parallelism limit, or a harness/model outside the approved choices
   creates a new proposed revision. No child for that revision starts before
   the user approves it.

Approval is a UI action bound to the exact plan revision. Text in a terminal,
a harness hook, or a child report is not approval.

## Approved plan

A plan is immutable after approval and contains:

- parent session and workspace identity;
- canonical Git repository root, clean working-tree evidence, and exact base
  commit SHA from which approved worktrees will be created;
- bounded child task IDs, task descriptions, and initial prompts;
- one assigned plan-owned worktree group per independent task chain; dependent
  tasks in a chain reuse that worktree sequentially only after the child
  session is ended and the user confirms the worktree is quiescent; independent
  chains receive separate worktrees and may run in parallel. Each group's
  exact absolute worktree path is proposed and shown before approval;
- the allowed harness/model choices for each child;
- ordered task batches and a maximum number of concurrent children;
- the user-approved repository/base revision and the plan's stated scope.

The parent may launch only declared task IDs whose dependencies are
parent-reported complete, once each. A duplicate request is idempotent and
returns the existing child session. Tasks sharing a worktree group may not
run concurrently, and a dependent task cannot reuse its predecessor's
worktree until the predecessor session is terminal and the user confirms that
no remaining process is using the worktree. This is a user acknowledgement,
not OS proof. A child session ending is not the task-turn boundary.
Supported harness completion/idle integrations send an authenticated,
one-shot turn receipt that moves the task to `needs_parent_result` without
requiring the interactive PTY process to exit; unsupported integrations expose
an explicit UI action to mark the turn ready for review. The task outcome and
dependency gate is the parent's explicit coordination report after reviewing
the child result; it can advance only declared dependencies and does not mean
quality approval or user acceptance. This amends the current coordinator
proposal's stronger machine-attested receipt gate for this proposed
orchestrator mode. Neither process exit nor terminal text alone proves task
success. The parent cannot add
tasks, broaden the orchestration scope, change the approved harness/model
choices, increase concurrency, or recursively launch grandchildren without a
new approved plan revision. Failures do not trigger automatic retries; a retry
requires a new plan revision and approval.

The sidecar derives each worktree path from the active workspace metadata
root, plan ID, and worktree-group ID, then includes that exact path in the
proposed revision for user review. It creates no worktree before approval. If
an approved path becomes occupied by something not recorded as that plan's
worktree, OrkWorks pauses and proposes a revised path for renewed approval; it
does not silently choose another location.

The plan and child launch capability govern what the OrkWorks orchestration
interface permits. This is a workflow boundary, not a security boundary
against commands the parent or child can run directly. Children run with the
same user permissions, inherited harness login behavior, and ordinary session
environment as other user-created sessions. The approved worktree is the
child's starting directory and collaboration boundary; macOS, Windows, and
Linux process/filesystem confinement is not claimed. Instructions that the
parent delegate all work and that children stay within their assigned tasks
are not OS-enforced. A future requirement for OS-enforced access restrictions
would need a separate design.

## Session creation and ownership

Only a session explicitly started in orchestrator mode receives a
plan-control capability in its launch environment. The parent session record
stores `sessionMode: "orchestrator"` so the sidecar can recognize the mode
after restart; ordinary sessions default to `sessionMode: "ordinary"` when
the field is absent. The capability is bound to
that parent session and the active sidecar generation; it is valid only while
the parent session and plan are active. It is revoked when the parent ends,
the workspace or sidecar generation changes, or the plan is paused, cancelled,
revoked, or complete. Before plan approval, the capability may submit a
bounded plan proposal but cannot launch children. After approval, it
authorizes only the declared child-plan operations (launch a declared task,
report a result after that child's authenticated turn receipt, and read that
plan's child status); it
cannot create ordinary sessions, change the plan, control unrelated sessions,
or grant a child its own launch capability. It is distinct from the existing
workflow-report token, is never persisted or logged. Resuming an ended
orchestrator session through the UI creates a fresh capability, but the plan
stays paused until the user approves its exact current revision again through
the UI. The resume request uses Electron main's existing UI authority. A
sidecar restart follows this same pause-and-reapprove path.

The parent submits proposals and child requests over the sidecar's existing
local HTTP channel using its plan-control capability. The UI approves through
Electron main using the existing `ORKWORKS_OPEN_PLAN_TOKEN` authority, which
is held by Electron main and the sidecar, is not exposed to renderer code, and
is filtered from coding-tool child environments. No second UI token is
introduced. The sidecar validates
the current plan revision, parent identity, task ID, launch count, approved
harness/model, and canonical assigned worktree before calling the existing
session creation workflow. It supplies the worktree as the session's working
directory and persists `parentSessionId`, `planId`, and `planTaskId` with the
child metadata. Child sessions use the active workspace's existing metadata
store and PTY lifecycle. This replaces the 2026-09-25 master-runner amendment
to ADR 0060 that proposed a dedicated sidecar for each child worktree; it does
not add a peer-sidecar registry or allow one instance to own multiple selected
workspaces.

Plan mutations for one plan are serialized. Before spawning a child, the
sidecar durably records a unique launch reservation and the task's `launching`
state. Duplicate launch requests observe that reservation and never spawn a
second child. Child session metadata records the plan, task, and reservation
IDs before PTY launch. After a restart, OrkWorks attaches a recorded child to
its reservation; if no child record exists, the task becomes
`launch_interrupted` and cannot retry until a new approved plan revision. No
launch is automatically repeated after a crash.

The parent may report a child task result after the authenticated turn receipt
puts that task in `needs_parent_result`; the child PTY can remain open and
manageable as an ordinary session. The result request includes the task's
expected state version. The sidecar atomically stores the parent's
coordination report and next version; a `completed` report unlocks only
declared dependencies and does not mean user acceptance. An identical retry
returns the stored result; a conflicting duplicate or stale version is
rejected.
Product and architecture decisions, ambiguous requirements, credentials or
permissions, destructive actions, Git mutation or merge approval, conflicting
high-confidence results, and acceptance of high-risk work remain user-owned
escalations. A sidecar restart never automatically relaunches a child. If the
parent's plan capability is no longer valid, the plan pauses for user review
before orchestration continues. Resuming the parent rotates its capability;
no child launches until the user reapproves the exact current plan revision.

Child sessions have the same lifecycle as ordinary sessions. A parent ending
does not implicitly kill its children; each remains visible and manageable
through the existing session controls. A sidecar restart follows the existing
session reconciliation behavior. This design does not claim crash-surviving
process-tree ownership or infer success from a parent/child process exit.

## Worktrees and integration

Parallel independent task chains receive separate plan-owned worktrees so
their edits do not share a checkout. Tasks in a dependency chain reuse the
same worktree sequentially, so a later child sees its predecessor's files;
two live children never share a worktree. Worktrees are provisioned only
after the user approves the exact plan and only from its approved clean base
commit. Child changes remain separate for manual user integration; OrkWorks
does not combine edits, commit, merge, rebase, push, or delete branches.

The initial implementation leaves worktrees available for review and manual
integration. It does not automatically clean them up. Any later cleanup must
follow the repository's existing rule: only a clean, quiescent, plan-owned
worktree may be removed.

## UI

- The Sessions list nests child rows under the parent and shows a child count
  and each child's ordinary status.
- Selecting a child opens its normal terminal session.
- The parent detail view shows the approved plan, current batch, child task
  status, and pending scope-change approvals.
- The plan approval view shows the complete task list, prompts, worktrees,
  harness/model choices, and concurrency ceiling. A scope-change request shows
  the proposed delta before approval.

No parallel-terminal panel or second session dashboard is introduced.

## Explicitly out of scope

- Native filesystem/process sandboxing, XPC helpers, Virtualization.framework,
  per-child launch agents, Linux namespaces/cgroups/Landlock, and Windows
  AppContainer/Job Object containment.
- A tool or model broker, credential proxy, per-child API-key isolation, or
  hard token/cost/CPU/memory ceilings. Existing harness policies and ordinary
  user credentials apply.
- Recursive delegation, dynamic tasks outside an approved plan, automatic
  retries, automatic code integration, and automated worktree cleanup.
- Replacing or changing the lifecycle guarantees of ordinary OrkWorks
  sessions.

## Evidence and limitations

The existing sidecar exposes `POST /sessions` through
`SessionApplication::create_session` and starts PTYs through the ordinary
session runtime. Each live session already receives `ORKWORKS_SESSION_ID`,
`ORKWORKS_PORT`, and a session-scoped workflow-report capability. It does not
currently expose parent-initiated child launch, plan approval, or parent/child
metadata; those are the feature work.

The disposable macOS fixture showed that an XPC service with a
security-scoped folder grant can confine a basic helper and its direct child to
a selected directory. It also showed that a detached descendant can outlive
that helper. A separate launchd probe confirmed that a `setsid()` descendant
survives the launch job's normal exit. These experiments rule out the tested
XPC/launchd path as the strict process-tree boundary previously proposed; they
do not block ordinary child sessions, whose lifecycle is the existing PTY
session lifecycle.
