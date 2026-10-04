# ADR 0077: Orchestrated children use the ordinary session runtime

- Status: proposed; written scope review pending
- Deciders: owner
- Date: 2026-10-04
- Tracking: [#610](https://github.com/Rambolarsen/orkworks/issues/610), initiative [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Supersedes the proposed runtime architecture in [ADR 0064](0064-bounded-taskmaster-coordinator.md) and the 2026-09-25 child-runtime amendments to [ADR 0060](0060-independent-workspace-instances.md); ADR 0060's accepted instance/workspace decision remains in force.

## Context

The confined master-session runner proposed a command broker, dedicated child
sidecars, isolated credentials, hard resource ceilings and machine-attested
success. The data-only coordinator foundation exists, but it provides no
runtime authority. Its native validation record established no eligible launch
slice. That finding remains historical evidence for that architecture.

The newer [ordinary-child design](../superpowers/specs/2026-09-26-taskmaster-orchestrated-child-sessions-design.md)
and [hierarchy direction](../superpowers/specs/2026-10-04-agent-hierarchy-and-configuration-learning-design.md)
instead use existing coding-tool sessions. The user authorized specification
work, including a continuously running parent that prepares execution after
research. Detailed contracts, tool eligibility and runtime implementation
approval remain open. This ADR records the proposed replacement consistently;
it does not mark those gates passed.

## Decision

One UI-created orchestrator coordinates declared tasks; implementation,
investigation, review and verification belong to child sessions. Children use
the selected workspace's existing sidecar, metadata store and PTY lifecycle,
with an assigned worktree as their launch directory. No child sidecar,
cross-instance registry or additional selected workspace is created. Keep
ADR 0013's single selected terminal, ADR 0022's runtime-owned PTYs and
ADR 0052's workspace metadata lease.

The [preparation contract](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md)
governs the run lifecycle. Electron main uses the existing
`ORKWORKS_OPEN_PLAN_TOKEN` authority to create a run with reviewed immutable
bootstrap instructions/configuration, goal and maximum live-child capacity.
The sidecar injects a distinct volatile run planning bearer into the verified
parent startup integration. The bearer permits bounded proposals, clarification,
coordination reads/event waits and explicit results; it cannot approve work.
Approval of one exact plan revision creates a separate server-held launch
grant. Every launch revalidates that grant, parent/runtime/workspace identities,
input/configuration digests, batches, dependencies, capacity and worktree.
No plan-owned branch, worktree or child is created before exact-plan approval.

Successful research completion revokes that plan's grant and retains the same
parent's planning bearer for automatic synthesis and an execution proposal.
Every new execution plan needs its own exact approval; research completion
never transfers approval into that new plan. Only one plan revision holds a launch grant per run. Final execution
completion or run cancellation revokes all run authority. Parent end,
workspace change and sidecar replacement revoke volatile authority and pause
coordination. UI-authorized exact-identity resume creates fresh authority;
launches require exact-plan reapproval. Between stages, resume restores
planning only. Changed bootstrap bytes require a newly reviewed UI-created run,
with the old run fenced and no approval transfer.

Plans use explicit ordered batches and declared dependencies, one attempt per
task and bounded records. All nonterminal children from every plan in the run,
plus unattached reservations, count toward its ceiling. A completed task or
research plan does not free a live child's slot. Independent chains use
separate approved worktree groups. Sequential reuse within a plan requires a
terminal predecessor and explicit user quiescence acknowledgement; this is
not OS proof. No cross-plan reuse or automatic transfer of code is introduced.

Persist allocation intent and a unique task launch reservation before mutation
or spawn. Recovery attaches only an exact recorded allocation/child; ambiguous
state blocks launches and never causes automatic relaunch. Authenticated turn
receipts, the narrow Electron-authorized ready-for-review action, or terminal
reconciliation make the task eligible for an explicit version-bound parent
result. Only that coordination result advances declared dependencies. A turn
receipt, research report, process exit or terminal text is not success proof,
independent quality review or user acceptance. A failed/blocked result pauses
launches; retries and added work require a new approved revision.

Coding-tool-enforced roles and skill delivery require exact version/profile
evidence under [#740](https://github.com/Rambolarsen/orkworks/issues/740) and
the [role contract](../superpowers/specs/2026-10-04-agent-role-configuration-design.md).
Unverified combinations block launch, with no wider-profile fallback.
Automatic parent continuation additionally needs a verified event/wait channel;
a live PTY does not prove it. Skills, hooks and evaluations confer no authority.

These are workflow and coding-tool controls. OrkWorks does not claim native
filesystem/process confinement, credential isolation, hard CPU/memory/token/cost
ceilings or crash-surviving ownership for orchestration. Ordinary sessions
retain their existing host permissions and login behavior. Same-user processes
may inspect/replay environment credentials; generic `POST /sessions` remains
an unauthenticated ordinary-session route. Withholding the UI token from
renderer and coding-tool environments does not establish OS process identity.
The independent-instance cleanup/replacement proof requirements in ADR 0060
remain unchanged; this proposal does not claim they have passed.

Children and worktrees remain visible after cancellation/completion and parent
exit. Orchestration never integrates edits, commits, merges, rebases, pushes,
changes existing branches or deletes branches. The initial slice has no
automated worktree cleanup. Any later separately reviewed removal requires a
clean, quiescent, plan-owned worktree and preserves branches and commits.
Product/architecture decisions, ambiguous requirements, credentials/permissions,
destructive actions, Git mutation or merge approval, conflicting high-confidence
results and high-risk acceptance remain user-owned escalations.

## Consequences

The old broker/lease/native-receipt contracts are historical rather than launch
prerequisites for this slice. Their data-only canonicalization/persistence may
be reused only after checking compatibility; their stored approvals cannot
become new run grants. Existing ordinary sessions keep their lifecycle and
absence-of-lineage compatibility.

The replacement narrows enforcement claims and adds explicit trust in parent
coordination results. Independent evaluation and user acceptance remain
separate. Concurrency, approval, persistence and recovery still need meaningful
cross-component fixtures; documentation changes do not enable endpoints.

The [scope alignment](../superpowers/specs/2026-10-04-taskmaster-orchestration-scope-design.md)
records the decision matrix and staged issue disposition. Acceptance of this
ADR must be recorded through written review before replacing #610's live
criteria and closing obsolete #617. Then finalize component contracts and
capability evidence and review each scoped execution plan before code starts.
