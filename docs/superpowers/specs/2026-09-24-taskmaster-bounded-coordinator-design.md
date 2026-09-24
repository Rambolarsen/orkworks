# Taskmaster Bounded Coordinator Design

- Status: proposed design gate
- Date: 2026-09-24
- Tracking issue: [#604](https://github.com/Rambolarsen/orkworks/issues/604)
- Related ADR: [0063](../../adr/0063-bounded-taskmaster-coordinator.md)

## Purpose and boundary

Phase 1 guided completion packets remain recommendation projections. This
design defines the boundary for a later Phase 2 coordinator; it does not
authorize coordinator implementation, recursive orchestration, autonomous
swarming, Git workflow ownership, or user-acceptance inference.

The coordinator may execute a user-approved bounded root plan. The root plan
is a versioned, immutable DAG of child tasks. It may authorize child sessions
to solve their assigned issues with bounded file-editing and command-running
tools, while preserving explicit user escalation for decisions and authority
outside the plan.

## Authority model

The user approves one exact root-plan revision. The approval binds to:

- one OrkWorks instance identity;
- one workspace identity and workspace snapshot/change subject;
- one plan revision and evidence fingerprint;
- the declared graph, task scopes, roles, tool envelope, budgets, retries,
  and concurrency ceilings; and
- an expiry and revocation generation.

Approval issues a server-generated coordinator capability. The capability is
opaque, non-forgeable, revocable, expiring, and never supplied by a model or
accepted from a child report. It is not valid in another OrkWorks instance,
workspace, or plan revision.

Each child receives a separate lease capability bound to its child ID, parent
ID, plan revision, capability revision, task scope, tool subset, budget
reservation, and lease expiry. A parent may grant or revoke tools at runtime
only from the already-approved envelope and without exceeding any total scope,
authority, budget, retry, or concurrency ceiling. Such grants receive a
monotonic capability revision and are auditable.

A child may request a missing capability from its parent. The request includes
the task need, proposed command/tool, bounded scope, expected cost, and
evidence that the current tool set is insufficient. The parent may satisfy the
request only from its own active envelope. An expansion beyond the root
envelope creates a new plan revision and requires renewed user approval.

## Plan and role model

Every node declares:

- a stable node ID and immutable parent ID;
- a concrete issue, success criteria, and output contract;
- workspace and path scope;
- an explicit role and allowed tool set;
- dependency IDs and dependency outcome requirements;
- model/harness constraints, if any;
- retry and attempt limits; and
- its budget reservation and concurrency class.

The default role set is:

- `implementation`: edit files and run bounded commands in the declared scope;
- `verification`: inspect and test without editing unless the plan explicitly
  grants a narrowly scoped correction capability;
- `review`: inspect and report findings without editing by default; and
- `remediation`: edit and verify a declared finding scope.

Git read operations may be available when declared. Git mutation, merge
approval, credentials, permission changes, destructive commands, and scope
expansion are not available through the default roles. A node cannot grant
itself a role or tool. Delegation is disabled by default; if a plan explicitly
includes delegation slots, a child may create only those predeclared nodes
within the parent’s remaining envelope and budget.

The parent supplies the child’s task, success criteria, prompt/context, role,
tool lease, and reporting contract. The child may use those tools to solve the
issue rather than merely perform one fixed command sequence.

## Plan revisions and lifecycle

Plan revisions are immutable. Changing graph edges, parentage, task scope,
role, tool envelope, model constraints, budgets, retries, concurrency, or
output contract creates a new revision and invalidates capabilities for the
old revision. Runtime status, progress, evidence, and reports do not change
the plan definition.

The lifecycle is:

```text
draft → proposed → active → completed
                    ├── paused
                    ├── failed
                    ├── cancelled
                    ├── expired
                    └── revoked
```

Only the user approves a proposed revision. Approval activates the coordinator
capability for that exact revision. Pausing prevents new launches while
preserving leases and evidence. Revocation or expiry prevents new work and
cancels descendants. Completion requires all required nodes to reach terminal
success. Failure preserves the blocking evidence. A superseding revision
preserves lineage and invalidates older capabilities.

## Budgets and capacity

The root plan declares hard integer budgets in normalized execution units,
per-role reservations, maximum concurrent children, maximum attempts per node,
and maximum delegation depth. A reservation is made atomically before launch.

An attempt consumes a new reservation. A failed launch releases its
reservation; cancellation refunds only work that never began; completed work
does not refund its consumed unit. Capacity exhaustion prevents a launch but
does not expand the plan or silently switch to an unapproved model. Provider
capacity is availability, not authority. Provider-specific token or time
limits may be enforced as additional tool ceilings, but they do not replace
the portable root budget.

The coordinator must account for reservations and refunds transactionally with
plan revision and lease state. A crash must not allow two coordinators to spend
one reservation or revive a refunded attempt.

## Graph and execution semantics

The plan graph is a DAG with immutable parentage. Cycles, unknown dependency
IDs, duplicate logical spawns, overlapping exclusive scopes, and dependency
requirements that cannot be satisfied are rejected before activation.

A dependent node launches only after all required dependencies report success.
Failed, cancelled, stale, conflicting, or orphaned results block dependents
and escalate to the parent. Cancellation propagates to descendants. Duplicate
spawn requests use an idempotency key and return the existing child lease.

Child results are terminal only when they include the declared output contract,
verification evidence, capability revision, and the current plan/lease
identity. Late results are retained as audit evidence when safe to do so but
cannot advance graph state.

## Authentication, validation, and recovery

Coordinator and child APIs are authenticated, version-checked, strict about
unknown fields, and fail closed. Every mutation/report carries the plan
revision, capability revision, actor/node ID, lease ID, and an idempotency key.
The server derives authority from the live capability registry; child payloads
cannot select their own scope, role, budget, parent, or status transition.

Launch, grant, retry, cancel, report, and recovery operations are idempotent.
Partial launch failure releases the reservation and records the failed
attempt. Cancellation creates a tombstone that rejects late reports. On
restart, the coordinator reconciles persisted plans and leases before new
launches. Ambiguous leases become orphaned and require parent/user-directed
recovery; they are never silently re-executed.

Capabilities use OS randomness, are not logged or serialized as bearer values,
and are revoked on instance shutdown, workspace loss, plan supersession,
expiry, or explicit cancellation. Reports from another instance, workspace,
revision, node, or lease are rejected.

## Mandatory user escalation

The coordinator must pause the affected branch and request the user for:

- product or architecture decisions;
- credentials or permission changes;
- destructive actions;
- Git mutation or merge approval;
- scope, role, tool, budget, retry, or concurrency expansion;
- conflicting high-confidence results; and
- unrecoverable blockers.

The escalation includes the blocked node, parent chain, requested authority,
remaining budget, evidence, and the exact decision required. The coordinator
must not infer approval from silence, a child report, or a `ready_for_user_review`
state.

## Design-gate acceptance criteria

Before implementation planning begins, written review must confirm that:

1. bounded root-plan approval does not silently become unrestricted autonomy;
2. capability and lease checks are enforceable at every launch, grant, report,
   retry, cancellation, and recovery boundary;
3. the total budget and concurrency ceilings cannot be exceeded by delegation,
   retries, refunds, duplicate spawns, or crash recovery;
4. stale, malformed, unauthorized, cancelled, cross-workspace, and late
   results fail closed; and
5. the implementation will be split into a separate plan and PR sequence from
   the Phase 1 completion-packet work.

No coordinator code, child-session API, or autonomous launch path should be
implemented until this design and [ADR 0063](../../adr/0063-bounded-taskmaster-coordinator.md)
are approved in writing.
