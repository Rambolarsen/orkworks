# ADR 0063: Bounded Taskmaster coordinator uses approved root plans

- Status: proposed
- Deciders: OrkWorks maintainers
- Date: 2026-09-24
- Tracking issue: [#604](https://github.com/Rambolarsen/orkworks/issues/604)

## Context

Taskmaster v1 recommends transitions and requires explicit approval for every
action. Phase 2 may need bounded recursive delegation so a parent can give a
child an issue, tools, and limits and receive a structured result. This is a
new authority model, not a continuation of the recommendation projection.

Unbounded delegation could exceed scope, budget, permissions, or the user’s
approval while still appearing to be a chain of ordinary recommendations.

## Decision

Defer coordinator implementation behind a written design gate. If approved,
the coordinator will execute only an immutable, user-approved root-plan
revision represented as a bounded DAG. Server-issued coordinator and child
lease capabilities bind work to one OrkWorks instance, workspace, plan
revision, capability revision, scope, tool envelope, budget reservation, and
expiry. Runtime grants may allocate unused authority already inside the
approved envelope; any expansion requires a new revision and renewed user
approval.

Budgets use normalized execution units with atomic reservations, explicit
retry/refund rules, concurrency ceilings, and crash-safe recovery. Dependencies
are fail-closed: cycles, duplicate spawns, stale results, failed dependencies,
late reports, and ambiguous leases cannot advance work. Authenticated,
version-checked, strict child reports are validated against server-owned
capabilities and idempotency keys.

Children may edit files and run bounded commands within their declared scope.
They do not receive Git mutation, merge approval, credentials, permission
changes, destructive actions, scope expansion, or implicit delegation. A plan
may explicitly declare bounded delegation slots, but a child cannot create
authority outside those slots. Product/architecture decisions, authority
expansion, conflicting results, and unrecoverable blockers escalate to the
user.

The existing single-active-context rule, explicit user escalation rules, and
Phase 1 completion-packet lifecycle remain unchanged. `ready_for_user_review`
never means user acceptance.

## Consequences

- A root plan can solve a bounded issue through parent-supplied tools without
  becoming an unrestricted autonomous swarm.
- Capability validation, budget accounting, and graph recovery become
  first-class coordinator responsibilities and require a separate PR sequence.
- Dynamic child requests can be satisfied efficiently within pre-approved
  authority, while new authority remains a user decision.
- The coordinator must preserve detailed lineage and evidence for stale,
  cancelled, failed, and escalated work.
- Until this ADR is accepted with the accompanying design, no coordinator
  implementation is authorized.
