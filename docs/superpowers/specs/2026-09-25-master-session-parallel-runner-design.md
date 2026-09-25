# Master-session parallel runner

- Status: proposed
- Date: 2026-09-25
- Tracking issue: [#610](https://github.com/Rambolarsen/orkworks/issues/610)
- Related design: [bounded Taskmaster coordinator](2026-09-24-taskmaster-bounded-coordinator-design.md)
- Related ADR: [0064](../../adr/0064-bounded-taskmaster-coordinator.md)

## Purpose

Give one user-approved master session enough bounded authority to split a
larger task into independent pieces and run those pieces in parallel child
sessions. OrkWorks owns the durable approval, worktree allocation, child
launch, completion collection, and guarded cleanup. Harness skills and hooks
provide instructions and observations, but never become the approval or launch
authority.

The design deliberately uses **parallel batches** instead of exposing a
general dependency graph. A master plan contains an ordered list of batches;
children within one batch are independent and may run concurrently. A later
batch starts only after the required children in the previous batch have
reached a validated terminal result.

## Boundary and non-goals

The user approves one exact master-plan revision. The approved revision fixes
the goal, batches, child task contracts, prompts, harness/model choices,
working-directory policy, cleanup policy, and the server-observed workspace
snapshot/change subject from which worktrees will be provisioned. The
approval also binds a server-issued coordinator capability and the finite
child lease/attempt envelope. The master may choose the decomposition before
approval, but it cannot add, remove, reorder, or broaden child authority after
approval. Any such change creates a new proposed revision and requires
approval.

This slice does not provide:

- a general DAG or arbitrary dependency engine;
- dynamic child creation, recursive delegation, or unrestricted swarming;
- parallel children in one shared working directory;
- autonomous commit, merge, rebase, push, branch deletion, or user acceptance;
- a general-purpose command broker, budget engine, or provider substitution
  system beyond the bounded broker, budget, and provider controls required by
  the existing coordinator contract; or
- a new harness-specific replacement for existing hooks and skills.

## Roles and responsibilities

### Master session

The master session is the planning and synthesis participant. It proposes the
complete batch decomposition, supplies bounded child prompts, and receives
the resulting child summaries through the persisted plan/event projection and
existing Taskmaster handoff mechanisms. It cannot silently change the
approved plan or treat child prose as proof of completion.

### OrkWorks

OrkWorks is the authority for the approved plan. It:

- binds approval to the immutable plan revision and workspace;
- issues and checks a coordinator capability and one child lease per attempt;
- creates one isolated worktree per child after approval;
- starts each child through a dedicated one-workspace child runtime bound to
  its worktree; a master sidecar never owns child workspaces;
- records parent session, plan, batch, worktree, and child identities;
- routes every child tool invocation through the coordinator's server-owned
  broker with finite per-attempt ceilings, normalized reservations, and hard
  denials;
- accepts only authenticated, current child reports that can be combined with
  server-observed session/process state into an attempt receipt;
- pauses the plan on stale, ambiguous, failed, or conflicting results; and
- performs the master's cleanup request only after explicit user acceptance of
  the combined work or an explicit authenticated user discard authorization.

### Child sessions

Children receive a fixed task contract, prompt, assigned worktree, and
approved harness/model selection, plus a short-lived lease bound to the plan,
batch, attempt, and worktree. They may work only in that worktree and report
results through a plan-bound authenticated report. A child cannot create
another child, acquire a broader lease, or alter the master plan.

### Harness skills and hooks

Skills describe the master and child behaviors and the report vocabulary.
Existing hooks report lifecycle signals such as turn completion or idle
state. A stop/idle signal is not task success and cannot launch another child.
No hook or skill is trusted as the durable approval authority.

### Workspace ownership and runtime topology

Each child worktree is a distinct workspace identity. The master sidecar owns
only the master workspace and its plan metadata; it must not adopt child
workspaces or launch their sessions through the master's existing workspace
runtime. OrkWorks starts each child in a dedicated child runtime/sidecar whose
sole workspace is the assigned worktree, and the child runtime reports through
the plan-bound authenticated coordinator channel.

The plan-bound channel uses a server-issued child runtime capability distinct
from the master coordinator capability. It is bound to the plan revision and
digest, child allocation/worktree identity, dedicated runtime identity,
current attempt and lease, allowed report/tool audience, and expiry/revocation
generation. The child runtime presents that capability only to the
server-owned coordinator broker; it cannot use it as a master capability, in
another child workspace, or to address a peer sidecar. The broker validates
each caller's capability at its own boundary and joins the parent plan,
allocation, runtime, and attempt through a durable server-side association; no
request carries both the master and child capabilities. The master sidecar
authenticates a launch, pause, or recovery command with its master capability.
The child runtime authenticates a report or runtime acknowledgement with its
child capability. For a child invocation, the broker uses the already
authorized master dispatch and sends an allocation-scoped envelope over the
child runtime's authenticated broker connection. The dedicated runtime
separately proves its own workspace-scoped identity to its local sidecar.

This is an explicit parent/child capability channel, not a peer instance
registry, cross-instance focus mechanism, attention rollup, or general
cross-workspace controller. It does not weaken ADR 0060's one-workspace-per-
instance boundary. Before implementation, ADR 0060 and the related workspace
spec must be amended or explicitly extended to record this bounded
coordinator-broker authority, child-runtime capability, metadata ownership,
shutdown proof, and boundary between master-plan records and child-workspace
records. Until that amendment is accepted, this runner is design-only and
cannot be implemented by reusing the ordinary workspace-scoped capability or
by having one sidecar own several workspaces.

The companion coordinator contract must add this broker audience and durable
plan-to-runtime association while retaining the root plan's master
instance/workspace binding. ADR 0060 and the workspace specification must
record that the broker is a bounded parent/child control plane, not an
OrkWorks instance, workspace metadata owner, or peer registry; that each child
sidecar still owns only its assigned workspace; and that broker shutdown and
child-runtime termination require server-observed, generation-specific proof.
Those companion amendments are prerequisites for implementation, not changes
that this design may silently assume.

The bounded coordinator contract remains in force: every child invocation
crosses the server-owned broker, uses the approved tool/provider envelope,
consumes finite wall-clock, CPU, memory, process, output, token, cost, and
tool-invocation ceilings, and is rejected when confinement or a ceiling cannot
be enforced. This design narrows the plan topology and batch lifecycle; it
does not replace or relax those controls.

## Lifecycle

```text
draft
  -> proposed
  -> approved
  -> provisioning
  -> running_batch
  -> awaiting_acceptance
  -> cleaned

running_batch -> paused
provisioning -> paused
paused -> approved (fresh approval, unchanged plan and workspace subject)
paused -> recovery_required (termination or launch state is uncertain)
paused -> cancelling (user-authenticated cancellation)
approved / provisioning / running_batch / paused -> revoking
revoking -> revoked (all children quiesced)
revoking -> recovery_required (quiescence is unproven)
recovery_required -> paused (fence_reason=paused; reconciliation proves quiescence)
recovery_required -> expired (fence_reason=expiry; reconciliation proves quiescence)
recovery_required -> cancelled (fence_reason=cancellation; reconciliation proves quiescence)
recovery_required -> revoked (fence_reason=revocation; reconciliation proves quiescence)
recovery_required -> discarding (fence_reason=discarding; only after cleanup state is reconciled)
recovery_required -> proposed (fence_reason=other; a new revision is required)
proposed / approved / provisioning / running_batch -> cancelling
cancelling -> cancelled (all children quiesced)
cancelling -> recovery_required (quiescence is unproven; cancellation intent retained)
provisioning / running_batch -> failed (no live child remains)
approved / provisioning / running_batch -> expiring
expiring -> expired (all children quiesced)
expiring -> recovery_required (quiescence is unproven)
expiring -> cancelling (user-authenticated cancellation only)
expired -> cancelling (user-authenticated cancellation only)
expired -> proposed (user creates a new revision; old approval remains invalid)
awaiting_acceptance -> discarding (explicit user acceptance or discard/rejection/abandonment)
failed / cancelled / expired / revoked -> discarding (explicit user discard/rejection/abandonment)
discarding -> cleaned (the discard authorization's cleanup completed)
discarding -> recovery_required (cleanup or quiescence is uncertain)
```

Approval activates exactly one plan revision. Provisioning is idempotent: a
retry returns the already-recorded worktree/child allocation when the
canonical request matches, and refuses a collision when it does not. Every
post-approval runtime mutation checks the live coordinator capability, approval
expiry and revocation generation, plan digest, workspace identity, and
idempotency key. Draft proposal, user approval, pre-approval cancellation,
post-terminal discard use authenticated user authority; system-generated
expiry/revocation transitions use server authority. These transitions do not
require a coordinator capability that has not yet been issued. User
cancellation and discard, including discard from `recovery_required`, remain
available after coordinator expiry or revocation. The recovery discard action
installs `fence_reason=discarding` atomically while the plan remains in
`recovery_required`; only reconciliation can then transition it to `discarding`
and permit cleanup.

Pausing is an atomic mutation fence: it revokes all active child leases,
prevents new launches, and requests bounded termination of every acknowledged
child process tree. The plan may return to `approved` only after all children
are quiescent and the user grants fresh approval. Provisioning can therefore
be paused safely even when only some worktrees have been allocated. Expiry
uses the same fence and termination rule, but the coordinator capability is
not required for the resulting user-authenticated cancellation; an expired
plan cannot resume or launch new work. The expiry reason is retained through
`recovery_required`; only reconciliation to `expired` or a new plan revision
can follow an expired approval, never the generic `recovery_required -> paused
-> approved` path. The server persists a single recovery `fence_reason`
(`paused`, `expiry`, `cancellation`, `revocation`, `discarding`, or `other`)
and evaluates it atomically;
the cancellation reason can reach only `cancelled`, never `paused`, `proposed`,
or `approved`. An `expired -> proposed` transition creates a new immutable
revision and approval request; it never resumes or mutates the expired one.

Revocation uses the same mutation fence and bounded process-tree termination as
pause, cancellation, and expiry. A revoked plan cannot resume, relaunch, or
advance; it can only enter user-authorized `discarding`. A user rejection or
abandonment in `awaiting_acceptance`, `failed`, `cancelled`, `expired`, or
`revoked` likewise enters `discarding`; reaching one of those terminal states
alone never authorizes cleanup. The authenticated discard transition records
the cleanup authorization. Cleanup requires every child to be quiescent,
validates that each worktree is still plan-owned and safe to remove, and
preserves branches and commits. If cleanup or quiescence is uncertain,
`fence_reason=discarding` keeps the plan in recovery until reconciliation; it
cannot become runnable again.

Children in a batch launch concurrently only after all their worktrees have
been created and recorded. A later batch cannot launch until the previous
batch's required children have server-attested attempt receipts. A failed,
missing, stale, or ambiguous result pauses the plan; it does not trigger an
unapproved replacement child.

Child completion uses a runner-specific one-shot handshake rather than an
ordinary stop/idle event. The child submits its authenticated result report;
the coordinator seals that attempt and asks the dedicated child runtime to
gracefully terminate the harness. The runtime acknowledges the sealed report,
closes the session, and reports the observed process exit. Only a server-
observed successful exit/status within the finite termination deadline can
produce a successful attempt receipt. A child that remains running, exits with
an error, is killed, or misses the handshake is failed or enters recovery and
cannot advance the batch. This keeps the existing interactive harnesses
usable while giving coordinator children an explicit one-shot completion mode.

When a batch completes, the coordinator advances the current-batch pointer to
the next already-approved batch. Later batches may consume only persisted
reports and explicitly declared artifacts. They cannot assume that code edits
from an earlier worktree are present, because this slice does not merge,
cherry-pick, or copy code between worktrees. A code-dependent follow-up needs
an explicit integration step and a new approved plan revision. The master may
use persisted results to prepare synthesis or a cleanup request, but this
slice does not inject a new prompt into a running master terminal
automatically.

Child completion is distinct from user acceptance. After all approved batches
finish, the plan enters `awaiting_acceptance`. User acceptance approves the
combined result for cleanup only and transitions the plan into `discarding`; it
does not imply merge or Git acceptance. Rejection, abandonment, or discard
uses the same cleanup path but records a non-acceptance disposition.

## Worktree lifecycle

After plan approval, OrkWorks provisions a unique worktree for every approved
child from the plan's recorded clean base revision. This is an infrastructure
operation performed on behalf of the approved plan; it does not grant child
sessions Git mutation authority. The allocation records the repository
identity, base revision, worktree path, branch or detached identity, owning
plan revision, batch, and child session.

Provisioning must refuse paths or repository identities that are outside the
workspace policy, collide with another live allocation, point at the primary
checkout, or no longer match the recorded repository. The server passes a
worktree allocation ID to the launch path and resolves the real cwd from its
own allocation record; callers cannot substitute an arbitrary cwd. The launch
fence must also prevent the child process and its descendants from escaping
the assigned worktree. If the host platform cannot enforce that confinement,
the child is not launched. Children receive the assigned working directory
and must not provision or remove worktrees themselves.

After explicit user acceptance, or an explicit authenticated user rejection,
abandonment, or discard transition into `discarding`, the master may request
cleanup of only worktrees created by that plan. OrkWorks validates and
executes that request. Cleanup preserves branches and commits unless the user
separately authorizes their deletion. A dirty worktree, active child, path
mismatch, or ownership mismatch blocks cleanup and leaves the plan in
`discarding` or `recovery_required` for user intervention.

## Persistence and safety

The plan, approval, batch definitions, worktree allocations, child links,
completion reports, server-attested attempt receipts, lease generations, and
cleanup outcome, and any recovery fence reason are durable records. Approval is
allowed only when the server
observes a clean workspace: the repository revision, an empty dirty-path set,
workspace identity, and clean-state observation are recorded as the base
subject. The design does not snapshot or replay uncommitted content. Approval
and provisioning fail if any dirty path appears or the recorded base subject
changes.
Mutations carry the current plan revision/digest, live capability or lease,
and an idempotency key. Old revisions and late child reports cannot advance
the current plan.

The coordinator never infers completion from terminal text, a stop hook, an
idle signal, or child-authored prose. The child report is evidence only. The
server advances a batch only after it validates a report against the assigned
child, worktree, plan revision, current batch, and lease, observes the child
session's successful terminal status and exit result, verifies the declared
output contract or artifact evidence through the server/trusted verifier, and
records a machine-attested attempt receipt that binds the report digest,
observed process/session identity, attempt ID, lease ID, observed terminal
status and exit result, output contract, verified output or artifact digests,
and workspace change subject. A killed, errored, nonzero, or otherwise
unsuccessful child cannot advance a batch even if its report claims success.

The existing recommendation lifecycle remains unchanged for ordinary
Taskmaster recommendations. The approve-once behavior is available only for
an explicitly created master plan and does not make ordinary recommendations
autonomous.

## Failure handling

- Worktree creation failure pauses provisioning without launching a partial
  batch. A durable partial-allocation record may only resume idempotent
  provisioning or roll it back; no child launches until every worktree in the
  batch is ready and recorded.
- A child failure pauses the plan and retains all worktrees for diagnosis.
- A lost or stale report cannot advance the batch or start a replacement.
- Pausing during provisioning or execution revokes every child lease, fences
  new launches, and quiesces acknowledged children before fresh approval is
  possible.
- A user cancellation installs a mutation fence, revokes active child leases,
  and requests bounded termination of every child process tree. It retains
  worktrees until termination is proven and the user chooses cleanup.
- Approval expiry installs the same fence and termination request. User-
  authenticated cancellation remains available after the coordinator
  capability expires; expiry never authorizes resume, relaunch, or cleanup by
  itself.
- Approval revocation installs the same fence and termination request. A
  revoked plan cannot resume or relaunch; it remains available only for
  reconciliation and explicit user-authorized discard.
- If termination or launch acknowledgement is uncertain, the plan enters
  `recovery_required`; reservations and worktrees remain held until
  reconciliation proves non-start or termination. No replacement launch or
  cleanup is allowed before then.
- Cleanup never removes a dirty or mismatched worktree automatically.
- A plan revision change invalidates old child reports and allocations for
  execution purposes; their records remain available for audit.

## Verification

The implementation must test, without launching real coding tools:

- exact-plan approval and rejection of post-approval edits;
- rejection of approval for dirty workspaces, plus binding to the observed
  clean base revision, workspace identity, expiry, and revocation generation;
- concurrent provisioning of unique worktrees and collision refusal;
- child launch records bound to an allocation ID, lease, assigned worktree,
  plan revision, and batch, with arbitrary cwd substitution refused;
- refusal to launch when worktree confinement cannot be enforced;
- stale, cross-plan, duplicate, unauthenticated, and child-authored-only
  completion reports;
- machine-attested attempt receipt creation and rejection of reports that do
  not match observed session/process state;
- receipt binding to the exact attempt and lease plus observed terminal status
  and exit result;
- successful report-to-exit handshake, including timeout, running-session,
  killed-process, and nonzero-exit rejection;
- rejection of nonzero, killed, errored, or unverified-output attempts even
  when child prose claims success;
- batch pause on failure or ambiguity;
- later-batch gating on required prior results;
- refusal to treat unmerged earlier worktree edits as later-batch input;
- pause, provisioning cancellation, and expiry lease revocation, process-tree
  quiescence, and
  `recovery_required` reconciliation;
- cancellation during expiry and preservation of cancellation intent through
  uncertain termination, with no return to approval;
- pre-approval user authorization and post-approval capability enforcement;
- server-side parent/child capability association without cross-workspace token
  reuse or peer-sidecar control;
- revocation fencing and recovery without relaunch;
- creation of a new revision after expiry without reviving the expired approval;
- partial provisioning recovery without launching an incomplete batch;
- cleanup authorization after user acceptance or explicit discard;
- discard authorization from `recovery_required` before cleanup reconciliation;
- refusal to remove dirty, active, mismatched, or foreign worktrees; and
- preservation of branches and commits during cleanup.

No implementation should add autonomous merge behavior or treat a stop hook
as proof that a child completed its assigned work.
