# Taskmaster orchestrated child sessions

- Status: accepted launch scope; detailed contracts and runtime implementation remain gated
- Date: 2026-09-26
- Tracking issue: [#610](https://github.com/Rambolarsen/orkworks/issues/610)
- Supersedes as the launch design: [master-session parallel runner proposal](2026-09-25-master-session-parallel-runner-design.md)
- Replaces the native-boundary prerequisite for this scope: [native child-launch boundary design](2026-09-25-taskmaster-native-child-launch-boundary-design.md)
- Adopted lifecycle scope: [Clarification and research-to-execution](2026-10-04-orchestrator-preparation-design.md) — preserves a live parent’s run planning authority after research and uses separate exact-plan execution grants; included in this baseline; detailed contract review remains open
- Proposed extension: [Agent hierarchy and configuration learning](2026-10-04-agent-hierarchy-and-configuration-learning-design.md) — optional research preparation, role profiles, loaded skills, and assignment evaluation; awaiting written review

## Purpose

Let a user-approved OrkWorks session act as an orchestrator for the whole
workflow. The parent plans and coordinates the work; all implementation and
review tasks in that workflow are delegated to ordinary OrkWorks child
sessions. Within one run, the user approves each exact bounded plan revision
before the parent launches its declared children. Successful research permits
the same parent to synthesize and propose execution; every new execution plan
needs its own exact approval. OrkWorks records the parent/child relationship
and presents the hierarchy and progress in the existing Sessions UI. A
request through the orchestration API for work outside the approved plan
pauses for a new user approval.

A child is a normal OrkWorks session created and owned by the active sidecar's
existing session runtime. This design does not create a second session
runtime, a child-specific process supervisor, or an OS sandbox.

## Scope alignment and current lifecycle

The accepted [#610 alignment](2026-10-04-taskmaster-orchestration-scope-design.md)
and [ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md) route
this ordinary-session design consistently through the authoritative specs.
Scope is accepted; detailed contracts and adapter evidence remain open.
The preparation contract's run planning bearer/server-held exact-plan grants
replace the earlier single-plan parent bearer. Its run bootstrap/input bindings,
event continuation, run-wide capacity and recovery rules apply below.
No runtime launch is authorized by this documentation.

## User flow

1. In the New Session dialog, the user selects **Orchestrator** mode, chooses
   a coding tool with a usable resume recipe, and gives the parent its goal.
   The sidecar independently verifies resumability. Electron main invokes the
   token-protected Orchestrator route for creation. The parent's role is
   planning, delegation, and tracking; it does not carry out the workflow's
   implementation or review tasks itself. Ordinary session creation remains
   the default and is unchanged.
2. The parent assesses the supplied goal, asks material clarification questions
   and proposes a bounded research or execution plan as a task list with
   task dependencies, requested harness/model choices, repository scope, and
   parallelism. Every execution task is assigned to a child session.
3. OrkWorks renders that proposal in the UI. The user can review and approve
   the exact revision or reject it.
4. The approved parent requests ready tasks through the orchestration API.
   OrkWorks checks that each request matches the approved plan and starts it
   through the normal session creation path. A Codex `Stop` hook sends an
   authenticated turn-completion receipt and moves the assigned task to
   parent-result review; the PTY session may remain open at its prompt. The
   explicit UI action is available for any live child whose task is still
   `running` with no recorded receipt, including Codex if its verified hook
   misses or delays the event. If a child becomes terminal before its receipt arrives,
   OrkWorks moves the task to parent-result review with missing-receipt
   evidence and does not infer an outcome. The parent reports the result to
   advance declared dependencies without changing the approved plan. Before a
   dependent child reuses the worktree, the prior session must end and the
   user must confirm that the worktree is quiescent.
5. The Sessions panel shows the children under their parent, with ordinary
   session status and terminal selection. The parent tracks child progress,
   collects summaries, and coordinates the next declared tasks through the
   orchestration interface.
6. A request through the orchestration API for an additional task, a broader
   repository/worktree scope, a higher parallelism limit, or a harness/model
   outside the approved choices creates a new proposed revision. No child for
   that revision starts before the user approves it.
7. Successful research ends its launch grant; the same live parent synthesizes
   the results and proposes a new execution plan for exact UI approval. Final
   execution ends the run authority. Capacity still counts earlier-plan live
   children, and no code or worktree is transferred into the new plan.

Approval is a UI action bound to the exact plan revision. Text in a terminal,
a harness hook, or a child report is not approval. The existing ordinary
`POST /sessions` route remains unauthenticated; a coding-tool process with
`ORKWORKS_PORT` can call it to create an ordinary session outside the approved
plan. This workflow does not prevent direct sidecar API calls by same-user
processes.

## Approved plan

A plan is immutable after approval and contains one or more tasks:

- parent session and workspace identity;
- the preparation contract's immutable `ParentPlanBinding`: run, parent, plan
  and revision identities, `runDefinitionDigest`, `bootstrapConfigurationDigest`,
  preparation revision and exact input-manifest digest; these bindings are
  part of the approved definition and its digest;
- exact task configuration/instruction/skill digests and consumed input record
  identities, versions and digests, with effective coding-tool/model bindings;
- canonical Git repository root, clean working-tree evidence, and exact base
  commit SHA from which approved worktrees will be created;
- bounded child task IDs, task descriptions, and initial prompts;
- one assigned plan-owned worktree group per independent task chain; dependent
  tasks in a chain reuse that worktree sequentially only after the child
  session is ended and the user confirms the worktree is quiescent; independent
  chains receive separate worktrees and may run in parallel. Each group binds
  a deterministic branch and exact absolute worktree path, both shown before
  approval;
- the allowed harness/model choices for each child;
- ordered batches with explicit task membership, plus a maximum number of
  concurrent live child sessions;
- the user-approved repository/base revision and the plan's stated scope.

Through the orchestration API, the parent may launch only declared task IDs
whose dependencies are parent-reported complete, once each. A duplicate
request is idempotent and returns the existing child session. Tasks sharing a
worktree group may not run concurrently, and a dependent task cannot reuse its predecessor's
worktree until the predecessor session is terminal and the user confirms that
no remaining process is using the worktree. This is a user acknowledgement,
not OS proof. Every entry point that resumes an orchestration-owned child
serializes its admission with group handoff and run-capacity admission. The
current task/session must still own the group and have a live-child slot. A
durable successor reservation transfers ownership before spawn and permanently
blocks predecessor resume in that group, even after the successor ends. A resume
before handoff invalidates the old terminal/quiescence acknowledgement, requiring
a fresh one after it ends. Resume does not retry a task or restore revoked
approval; ordinary sessions without lineage retain their existing resume path.
A child session ending is not the task-turn boundary. Initially,
the existing Codex `Stop` reporter sends the one-shot receipt to
`POST /sessions/{child_id}/orchestration/turn-completion`, authenticated with
that child's `Authorization: Bearer <ORKWORKS_REPORT_TOKEN>`. The sidecar
derives parent, plan, task, and reservation identity from child metadata and
accepts only the Codex `Stop` event for a live child whose task is `running`.
Store turn-completion evidence as `codex_stop_receipt`, `ui_action`, or
`child_terminal_without_receipt`. If the child becomes terminal while still
`running`, terminal handling moves
the task to `needs_parent_result` with
`completion_evidence: child_terminal_without_receipt`, including during
startup reconciliation. This lets the parent report an explicit outcome
without treating process exit as success. The explicit Electron-authorized UI
action is available for any live task still `running` with no recorded receipt,
even when an active integration is verified. That action marks the turn ready for review through a
version-checked transition bound to
the exact approved plan revision, launch reservation, child session ID, and
task version; it is accepted only while that exact child is live and its task
is `running`, then idempotently moves it to `needs_parent_result`. If a receipt
arrives after the UI transition, it returns idempotent success without applying
the transition twice. The task outcome and
dependency gate is the parent's explicit coordination report after reviewing
the child result; it can advance only declared dependencies and does not mean
quality approval or user acceptance. This amends the current coordinator
proposal's stronger machine-attested receipt gate for this proposed
orchestrator mode. Neither process exit nor terminal text alone proves task
success. Through the orchestration API, the parent cannot add tasks, broaden
the orchestration scope, change the approved harness/model choices, increase
concurrency, or recursively launch grandchildren without a new approved plan
revision. These limits do not block direct calls to the existing ordinary
session endpoint. Failures do not trigger automatic retries; a retry requires
a new plan revision and approval.

`max_parallel_children` counts every nonterminal child session and every
unattached launch reservation. The run's immutable ceiling additionally counts
children/reservations across all its plans, including completed research plans;
each plan's ceiling may be lower but cannot raise the run ceiling. A child continues to consume capacity while
its task is `needs_parent_result`, awaiting worktree reuse approval, or
parent-reported complete; the slot is released only when that ordinary child
session becomes terminal. Ordered batches are explicit in the approved
definition. A task is launchable only after every required task in all
earlier batches has a parent-reported complete result; task dependencies also
remain explicit within the graph.

When the final parent result changes every task to `reported_complete`, the
sidecar atomically marks the plan `complete` only if all launch reservations
are attached and settled and there is no interrupted allocation or pending
proposed scope revision. Incomplete tasks, failed/blocked results, unsettled
reservations, interrupted allocations, or a pending scope revision prevent
completion. Any transition that clears a blocker, including rejecting a
pending scope revision or resolving an interrupted allocation, reruns this
completion check atomically. Live child PTYs do not delay it: completion
revokes that exact plan's server-held execution grant and fences its launches.
For a successful research stage, the run planning bearer remains valid so the
same parent automatically synthesizes and proposes execution. Final execution
completion ends run authority. After completed research, the version-bound UI
finish action may complete a research-only run or decline execution, including
a no-go conclusion, before execution approval and with no execution allocation.
It revokes all run authority
and invalidates pending proposals; unsettled reservations/interrupted allocations
block finish. Completion does not assert quality or user acceptance. Child sessions and worktree records remain
visible for ordinary management/manual integration. A live child neither
reopens a completed plan nor frees a run-capacity slot.

The sidecar derives each worktree path from the installation-level
`~/.orkworks/taskmaster/worktrees/<workspace-hash>/<plan-id>/<group-id>` root,
outside the workspace metadata tree that startup GC may remove. It derives a
branch as `orkworks/taskmaster/<plan-id>/<group-id>`. Both exact values are
part of the immutable approved definition and UI. It creates no branch or
worktree before approval. If an approved path or branch is occupied by
something not recorded as that plan's allocation, OrkWorks pauses and proposes
a revised value for renewed approval; it does not silently choose another
location. Startup GC retains workspace metadata while any nonterminal plan,
unreconciled allocation, or recorded plan worktree exists; malformed plan
ownership records fail closed and keep the directory.

Session retention and explicit forget protect the orchestrator parent's
session record while its plan is nonterminal, any child session is live, an
allocation is unreconciled, or any plan-owned worktree remains available for
manual integration. Once these conditions clear, normal retention and forget
behavior may remove the session record; the plan/worktree ownership records
remain governed by workspace GC and are not implicitly deleted with the parent
session.

The run planning bearer plus the separate server-held exact-plan grant govern
what the orchestration endpoints permit. The bearer alone never permits a
launch or approval. It is injected into the verified parent startup environment;
another same-user process may inspect/replay it. This is a workflow boundary,
not OS process-origin proof. Generic `POST /sessions` remains unauthenticated
for ordinary creation outside the plan. Orchestrator creation uses the
Electron-main UI token; any same-user process obtaining that token can call
its protected route. The run bearer is not accepted there. No second UI token
or automatic terminal-input continuation is introduced.

Children run with the same user permissions, inherited harness login behavior,
and ordinary session environment as other user-created sessions. The approved
worktree is the child's starting directory and collaboration boundary; macOS,
Windows, and Linux process/filesystem confinement is not claimed. Instructions
that the parent delegate all work and that children stay within their assigned
tasks are not OS-enforced. A future requirement for OS-enforced access
restrictions would need a separate design.

## Session creation and ownership

Orchestrator creation is limited to harness definitions with a usable resume
recipe; the sidecar validates this independently of the New Session UI. Plan
approval and child launch remain disabled until the parent has reported a
usable resume strategy and harness session identity. If a selected resumable
harness fails to provide that identity, the plan stays unapproved and the UI
explains how to enable or repair identity reporting, and allows refresh/retry
if the identity arrives later. If it cannot be recovered, the user must cancel
and recreate the parent with a supported harness whose identity reporting
works. No replacement-parent recovery path is introduced. In particular,
harnesses such as Aider and Generic Shell whose definitions have no resume
strategy cannot be Orchestrator parents.

Only Electron-authorized creation can establish an orchestrator run. Generic
`POST /sessions` remains ordinary-only. Persist `sessionMode: "orchestrator"`
on the parent; absence defaults to `ordinary`. At creation, bind the immutable
reviewed goal/bootstrap/configuration and run cap under the role/preparation
contracts. Later task/report inputs are coordination data and cannot silently
replace parent startup instructions, selected skills or effective settings.
A changed bootstrap fences the old run and requires a newly reviewed UI-created
run, without transferring approvals or assignments.

The volatile run planning bearer is bound to the run/parent, parent runtime,
workspace and sidecar generation. It is distinct from workflow-report and
child report credentials and is never persisted/logged. It permits bounded
clarification, proposal, state/event reads and exact child coordination results.
Electron-main approval through `ORKWORKS_OPEN_PLAN_TOKEN` creates a separate
server-held grant for one exact plan revision. Launch requires both identities
and fresh locked grant/configuration/input/batch/dependency/capacity/allocation
validation. The run bearer cannot approve work or control unrelated sessions;
ordinary same-user API calls remain outside this boundary.

Paused plans allow report collection/proposals but deny launches. Successful
research completion revokes its grant before synthesis; the same parent's
planning bearer continues to a linked execution proposal requiring its own
approval. Stage cancellation pauses the run for explicit UI continuation; run
cancellation/final execution or explicit UI research-only/decline-execution
finish ends run authority. At most one plan revision has
a launch grant. Older live children still count toward run capacity. New plans
have distinct worktree groups, no cross-plan reuse or code transfer.

Parent end, workspace or sidecar replacement revoke volatile run authority
and pause coordination. Resume uses Electron main and the exact captured
native identity, verifies unchanged bootstrap/settings and creates fresh
runtime-bound authority. No launch occurs before exact current-plan reapproval;
between stages resume restores planning only. Unsupported exact resume blocks
recovery without replacement-parent fallback. No restart automatically
relaunches children or reconnects event automation.

Electron main retains `ORKWORKS_OPEN_PLAN_TOKEN`, withheld from renderer and
coding-tool environments. The sidecar uses the existing session-creation path,
setting only a validated approved worktree directory and persisting parent,
run, plan, task and reservation lineage. Children remain in the active
workspace's metadata store and PTY lifecycle; no dedicated child sidecar or
extra selected workspace is adopted. This replaces ADR 0060's historical
runner amendments through accepted ADR 0077 and preserves its accepted
independent-instance ownership and cleanup/replacement proof.

Run/grant/capacity and plan mutations serialize together, so revocation,
completion and concurrent launches cannot admit work after fencing. Before spawning a child, the
sidecar durably records a unique launch reservation and the task's `launching`
state. Duplicate launch requests observe that reservation and never spawn a
second child. Child session metadata records the plan, task, and reservation
IDs before PTY launch. After a restart, OrkWorks attaches a recorded child to
its reservation; if no child record exists, the task becomes
`launch_interrupted` and cannot retry until a new approved plan revision. No
launch is automatically repeated after a crash.

The parent may report a child task result after an authenticated turn receipt,
the explicit UI action or terminal reconciliation puts it in `needs_parent_result`; the child PTY can remain open and
manageable as an ordinary session. The result request includes the exact
approved plan revision, task version, launch reservation ID, and child session
ID. The sidecar accepts it only for that exact launched task instance and
atomically stores the parent's coordination report and next version; a
`completed` report unlocks only declared dependencies and does not mean user
acceptance. An identical retry returns the stored result; a conflicting
duplicate or stale version is rejected. A proposed scope revision may be
drafted while children are live, but it cannot be approved until all launched
children from the prior revision have a result and are terminal.
Product and architecture decisions, ambiguous requirements, credentials or
permissions, destructive actions, Git mutation or merge approval, conflicting
high-confidence results, and acceptance of high-risk work remain user-owned
escalations. A sidecar restart never automatically relaunches a child. If the
parent's run capability is no longer valid, coordination pauses for user review.
UI resume creates fresh runtime-bound authority; no child launches until the
user reapproves the exact current plan revision.

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

The plan definition and its digest are immutable. The digest covers only the
approved definition, including `ParentPlanBinding` (run definition/bootstrap
configuration, preparation revision and input-manifest digests), task
configuration/instruction/skill and consumed input bindings, tasks, batches,
paths, branches, harness/model bindings and limits. Changed bound inputs or
configuration require a new exact revision and UI approval; changed immutable
run bootstrap requires a new UI-created run. A launch grant cannot authorize
changed inputs under an old digest. Mutable execution state (plan status, task results, reservations,
child IDs, and allocation progress) is stored separately and never changes the
approved digest.

## UI

- The Sessions list nests child rows under the parent and shows a child count
  and each child's ordinary status.
- Selecting a child opens its normal terminal session.
- The parent detail view shows the run/stage, approved plan, current batch,
  child task status, capacity held by earlier-plan live children, and pending
  scope-change approvals. Research summary accompanies the execution proposal.
- The verified adapter's machine-readable event/wait channel must continue the
  same parent for approvals, reports and capacity changes. A live PTY alone
  does not prove continuation; unsupported automation is visibly unavailable.
- The plan approval view shows the complete task list, prompts, each worktree's
  exact branch and path, harness/model choices, ordered batches, and
  concurrency ceiling. It also explains that approval governs OrkWorks'
  orchestration API; same-user coding-tool processes can still call the
  unauthenticated ordinary session endpoint or run commands directly. A
  scope-change request shows the proposed delta before approval.

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

The feature adds `POST /sessions/orchestrator`, which accepts a new parent
only with the existing Electron-issued UI token in `ORKWORKS_OPEN_PLAN_TOKEN`;
Electron main sends the token, and the sidecar validates it. The token is
withheld from renderer and coding-tool environments but may be inspected by
same-user processes that can inspect the sidecar environment. Generic
`POST /sessions` remains ordinary-only and does not create an orchestrator even
if its JSON contains an unrecognized mode field. The token is supplied through
an Electron-main preload method, not renderer code. This uses the existing UI
authority mechanism; it does not provide OS-enforced process identity,
authenticate ordinary session creation, or provide process isolation.

The disposable macOS fixture showed that an XPC service with a
security-scoped folder grant can confine a basic helper and its direct child to
a selected directory. It also showed that a detached descendant can outlive
that helper. A separate launchd probe confirmed that a `setsid()` descendant
survives the launch job's normal exit. These experiments rule out the tested
XPC/launchd path as the strict process-tree boundary previously proposed; they
do not block ordinary child sessions, whose lifecycle is the existing PTY
session lifecycle.
