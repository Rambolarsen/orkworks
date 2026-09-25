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
working-directory policy, and cleanup policy. The master may choose the
decomposition before approval, but it cannot add, remove, reorder, or broaden
child authority after approval. Any such change creates a new proposed
revision and requires approval.

This slice does not provide:

- a general DAG or arbitrary dependency engine;
- dynamic child creation, recursive delegation, or unrestricted swarming;
- parallel children in one shared working directory;
- autonomous commit, merge, rebase, push, branch deletion, or user acceptance;
- a generic command broker, budget engine, or provider substitution system; or
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
- creates one isolated worktree per child after approval;
- starts children through the existing session-creation path;
- records parent session, plan, batch, worktree, and child identities;
- accepts only authenticated, current child completion/failure reports;
- pauses the plan on stale, ambiguous, failed, or conflicting results; and
- performs the master's cleanup request only after explicit user acceptance of
  the combined work.

### Child sessions

Children receive a fixed task contract, prompt, assigned worktree, and
approved harness/model selection. They may work only in that worktree and
report results through the existing authenticated completion mechanism. A
child cannot create another child or alter the master plan.

### Harness skills and hooks

Skills describe the master and child behaviors and the report vocabulary.
Existing hooks report lifecycle signals such as turn completion or idle
state. A stop/idle signal is not task success and cannot launch another child.
No hook or skill is trusted as the durable approval authority.

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
proposed / approved / running_batch -> cancelled
provisioning / running_batch -> failed
```

Approval activates exactly one plan revision. Provisioning is idempotent: a
retry returns the already-recorded worktree/child allocation when the
canonical request matches, and refuses a collision when it does not.

Children in a batch launch concurrently only after all their worktrees have
been created and recorded. A later batch cannot launch until the previous
batch's required children have authenticated terminal results. A failed,
missing, stale, or ambiguous result pauses the plan; it does not trigger an
unapproved replacement child.

When a batch completes, the coordinator advances the current-batch pointer to
the next already-approved batch. The master may use the persisted results to
prepare synthesis or a cleanup request, but this slice does not inject a new
prompt into a running master terminal automatically.

Child completion is distinct from user acceptance. After all approved batches
finish, the plan enters `awaiting_acceptance`. User acceptance approves the
combined result for cleanup only; it does not imply merge or Git acceptance.

## Worktree lifecycle

After plan approval, OrkWorks provisions a unique worktree for every approved
child from the plan's recorded base revision. This is an infrastructure
operation performed on behalf of the approved plan; it does not grant child
sessions Git mutation authority. The allocation records the repository
identity, base revision, worktree path, branch or detached identity, owning
plan revision, batch, and child session.

Provisioning must refuse paths or repository identities that are outside the
workspace policy, collide with another live allocation, point at the primary
checkout, or no longer match the recorded repository. Children receive the
assigned working directory and must not provision or remove worktrees
themselves.

After explicit user acceptance, the master may request cleanup of only
worktrees created by that plan. OrkWorks validates and executes that request.
Cleanup preserves branches and commits unless the user separately authorizes
their deletion. A dirty worktree, active child, path mismatch, or ownership
mismatch blocks cleanup and leaves the plan paused for user intervention.

## Persistence and safety

The plan, approval, batch definitions, worktree allocations, child links,
completion reports, and cleanup outcome are durable records. Mutations carry
the current plan revision/digest and an idempotency key. Old revisions and
late child reports cannot advance the current plan.

The coordinator never infers completion from terminal text, a stop hook, an
idle signal, or child-authored prose. It requires the existing authenticated
report path and validates that the report belongs to the assigned child,
worktree, plan revision, and current batch.

The existing recommendation lifecycle remains unchanged for ordinary
Taskmaster recommendations. The approve-once behavior is available only for
an explicitly created master plan and does not make ordinary recommendations
autonomous.

## Failure handling

- Worktree creation failure pauses provisioning without launching a partial
  batch unless the durable record proves which children were safely created.
- A child failure pauses the plan and retains all worktrees for diagnosis.
- A lost or stale report cannot advance the batch or start a replacement.
- A user cancellation stops future launches and retains worktrees until the
  user chooses cleanup.
- Cleanup never removes a dirty or mismatched worktree automatically.
- A plan revision change invalidates old child reports and allocations for
  execution purposes; their records remain available for audit.

## Verification

The implementation must test, without launching real coding tools:

- exact-plan approval and rejection of post-approval edits;
- concurrent provisioning of unique worktrees and collision refusal;
- child launch records bound to the assigned worktree and batch;
- stale, cross-plan, duplicate, and unauthenticated completion reports;
- batch pause on failure or ambiguity;
- later-batch gating on required prior results;
- cleanup authorization after user acceptance;
- refusal to remove dirty, active, mismatched, or foreign worktrees; and
- preservation of branches and commits during cleanup.

No implementation should add autonomous merge behavior or treat a stop hook
as proof that a child completed its assigned work.
