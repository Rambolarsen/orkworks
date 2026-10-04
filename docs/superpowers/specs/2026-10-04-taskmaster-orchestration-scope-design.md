# Taskmaster ordinary-child orchestration scope alignment

- Status: proposed documentation alignment; written scope review pending
- Date: 2026-10-04
- Tracking: [#610](https://github.com/Rambolarsen/orkworks/issues/610), initiative [#738](https://github.com/Rambolarsen/orkworks/issues/738)
- Decision: proposed [ADR 0077](../../adr/0077-taskmaster-orchestrated-child-sessions.md)
- Sources: [ordinary-child baseline](2026-09-26-taskmaster-orchestrated-child-sessions-design.md), [hierarchy direction](2026-10-04-agent-hierarchy-and-configuration-learning-design.md), [role configuration](2026-10-04-agent-role-configuration-design.md), [preparation lifecycle](2026-10-04-orchestrator-preparation-design.md)

## Scope and authority

This deliverable executes #610's existing documentation alignment prerequisite.
It reconciles the older confined runner with the newer ordinary-child direction
and includes the continuously running parent's proposed run/grant lifecycle.
The user chose this alignment task after prioritizing #738 and its prerequisites.
This is authorization to prepare reviewable documentation, not approval of
detailed contracts, proof of tool support or permission to implement runtime code.

The accepted v1 recommendation behavior remains authoritative and unchanged.
The orchestration extension in `specs/taskmaster.md`, ADR 0077 and the component
designs stays proposed. Historical proposals are marked superseded as the
target direction, not replaced by an implemented or accepted runtime. Approval
of this written alignment records the product/architecture scope decision;
runtime still needs component review, capability evidence and scoped plan approval.

## Investigated uncertainty checkpoint

**What am I least confident about?** Whether the older ownership/receipt
requirements are merely wording differences or decisions that need supersession.
Reading ADR 0060's Decision and Consequences shows that its accepted one-instance,
one-workspace lease and cleanup/replacement proof still apply. Only its
2026-09-25 dedicated-child-runtime amendments conflict. Preserve the accepted
ADR and label those amendments historical, with the proposed replacement link.
ADR 0064's broker, grant slots, hard budgets and native success receipts are a
different proposed runtime architecture; supersede that proposal through a
new ADR rather than describing an intentional trust change as a clarification.
The index ends at 0076, so use 0077; 0066 belongs to hook-owned attention.

**What is the biggest missing dependency?** The old September baseline plan
uses one plan bearer revoked on completion. The October preparation contract
requires automatic synthesis by the same parent after research. Aligning only
native confinement would leave contradictory launch authority and capacity
rules. Carry the run-bound immutable bootstrap, distinct planning bearer,
server-held exact-plan grants, verified event continuation and run-wide live-child
count into the authoritative proposed extension and the replacement handoff.

Source inspection confirms `taskmaster/coordinator.rs` and `coordinator_store.rs`
are data-only. Existing persistence cannot authorize children or migrate old
approvals into run grants. The capability register remains unverified/no-go;
this alignment cannot turn documentation or a running PTY into eligibility.
No new tool probes or runtime tests were run for this documentation task.

## Decision matrix

| Contract | Historical runner | Proposed replacement | Disposition |
| --- | --- | --- | --- |
| Runtime owner | Dedicated one-workspace child sidecars and broker association | Ordinary PTYs in the parent's selected-workspace sidecar/store | Replace only ADR 0060's child-runtime amendments; keep its accepted base decision |
| Execution access | Server command broker, native confinement, isolated credentials, hard budgets | Verified coding-tool role/settings/content delivery; normal host permissions/logins | Supersede ADR 0064's proposed runtime, retain data-only foundation |
| Parent authority | One plan bearer, revoked on plan completion | Run planning bearer plus exact-plan server-held grant | Preparation contract owns lifecycle, fencing and numeric limits |
| Research transition | No continuous multi-plan authority | Same parent synthesizes automatically; execution awaits fresh exact approval | No approval transfer or bearer revival |
| Coordination outcome | Native server-attested successful attempt | Turn readiness followed by explicit exact-version parent result | Parent result advances declared dependencies only; evaluation/acceptance are separate |
| Capacity | Plan-local bounded batch | Plan limit plus immutable run limit, counting all live prior-plan children/reservations | Task/plan completion does not release live-session slots |
| Worktree chain | Independent children only | Independent chains isolated; sequential same-plan reuse after terminal predecessor/user acknowledgement | No concurrent shared-worktree children or cross-plan reuse |
| Recovery | Native child supervisor/crash ownership | Ordinary lifecycle, revoked volatile authority, exact identity resume/reapproval | No automatic relaunch, no new process-ownership guarantee |
| Integration/cleanup | Manual integration and automatic disposition-bound clean removal | Manual integration; no automated cleanup in initial slice | Later cleanup would need separate review and clean/quiescent/owned gates |

Preserve parent-only delegation, explicit ordered batches/dependencies,
one attempt per task, immutable approval, finite admission/storage limits,
separate worktrees, one selected terminal and ordinary-session compatibility.
No model/provider substitution, wider permissions, new task, retry or changed
bootstrap may hide behind learning, research completion or a report.

The limits govern OrkWorks' orchestration APIs, not arbitrary same-user commands.
Ordinary `POST /sessions` remains unauthenticated. Environment bearers can be
inspected/replayed by same-user processes; neither a hook nor report proves
OS process origin. Exact coding-tool controls need verified adapters, with
unsupported combinations unavailable. No native confinement is claimed.
ADR 0060's separate independent-instance cleanup/replacement proof stays binding.

## Staged issue disposition after written acceptance

Do not close #610: its runtime is still absent. Keep the old live issue body
until this written scope decision is accepted. Then replace its title/scope
with ordinary-child orchestration and the following unchecked criteria;
link the merged alignment, ADR 0077 and scoped handoff. Preserve prior comments
as historical evidence rather than deleting them.

- [ ] UI-created run binds immutable reviewed bootstrap, goal, repository/workspace identity and run cap; ordinary session creation remains ordinary-only.
- [ ] Every exact plan binds tasks/configurations/input digests, tool/model identities, clean base, exact branches/paths, ordered batches/dependencies and plan cap; only Electron-authorized exact approval creates its launch grant.
- [ ] Run planning bearer and server-held plan grants remain distinct; research completion revokes its grant, synthesis continues on the same parent, and every new execution plan needs its own exact approval.
- [ ] Verified coding-tool permissions, required instruction/skill delivery, exact resume and same-parent event continuation qualify each supported slice; unverified combinations have no fallback launch.
- [ ] Allocation intent and task reservation are durable before mutation/spawn; duplicates launch at most once and restart recovery never silently relaunches or adopts foreign/ambiguous artifacts.
- [ ] Children reuse the selected workspace's ordinary runtime/store; one instance/lease and one selected terminal remain intact, with backward-compatible ordinary records.
- [ ] Each task has one attempt; failed/blocked/interrupted work pauses launches and retries/additions require a newly approved revision.
- [ ] Run capacity counts every nonterminal child across all plans and unattached reservation; completion reports do not release a live slot.
- [ ] Independent chains have separate plan-owned worktrees; dependent same-plan reuse requires a terminal predecessor and explicit user quiescence acknowledgement, with no cross-plan reuse or automatic integration.
- [ ] Authenticated turn readiness or explicit UI action/terminal reconciliation gates exact version-bound parent outcomes; reports/exit alone never prove success, quality or user acceptance.
- [ ] Cancellation/final completion fence grants; parent end/workspace/sidecar change revoke volatile credentials and pause; UI exact-identity resume requires current-plan reapproval before launches.
- [ ] Children/artifacts remain manageable after parent exit or completion; protected ownership/evidence, admission/replay fencing and finite limits survive retention, deletion and recovery.
- [ ] No orchestration-owned transfer, commit, merge, rebase, push, existing-branch change, branch deletion or automated worktree cleanup; required user escalations remain explicit.
- [ ] Focused contract, race/recovery, adapter and desktop fixtures cover each scoped unit; review and required CI pass before feature availability is claimed.

Retire [#617](https://github.com/Rambolarsen/orkworks/issues/617) after written
scope acceptance, with reason **superseded prerequisite, not implemented**.
Its production native confinement boundary is outside this ordinary-session
slice. Preserve the [validation record](../../validation/master-session-runner-confinement.md)
and the old runner plan. A future request for native confinement would require
a new reviewed scope and its own implementation tracker; closing #617 must
not imply the old evidence gate passed or the ADR 0060 ownership gate is gone.

#738 and #740–#746 remain open. Alignment removes contradictory scope; it does
not complete their detailed contract/evidence/handoff acceptance criteria.

## Runtime handoff and review checklist

The replacement baseline plan at
`docs/superpowers/plans/2026-09-26-taskmaster-orchestrated-child-sessions.md`
(repository-only; plans are excluded from the published site) contains reviewable
unit boundaries and acceptance evidence, not executable runtime instructions. Finalize contracts first, then create scoped implementation
issues/plans linked to #610 and #738. No new implementation issue or child launch
is authorized merely by merging this documentation.

- [ ] Owner reviews the scope/trust changes and accepts or requests corrections to ADR 0077 and this alignment.
- [ ] Record the accepted decision on #610, retire #617 with the explicit supersession reason, and synchronize #738 without claiming component completion.
- [ ] Review #741/#742's immutable bootstrap, configuration and run/grant lifecycle together; retain #740's unverified/no-go entries until exact evidence is available.
- [ ] Finalize #743/#744 reporting/evaluation and #746 projections before the units consuming them; #745 remains downstream of usage/evaluation.
- [ ] Review and explicitly approve each scoped runtime execution plan with meaningful failing/passing fixtures before code starts.

Documentation verification for this alignment: diff whitespace check,
repository documentation drift check and VitePress build/dead-link validation.
Independent artifact review checks both correctness of retained claims and
completeness of scope routing. Passing these checks is documentation evidence,
not a passed runtime or adapter gate.
