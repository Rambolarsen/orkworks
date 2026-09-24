# ADR 0063: Bounded Taskmaster coordinator uses approved root plans

- Status: proposed
- Deciders: OrkWorks maintainers
- Date: 2026-09-24
- Tracking issue: [#604](https://github.com/Rambolarsen/orkworks/issues/604)

## Context

Taskmaster v1 recommends transitions and requires explicit approval for every
action. A future coordinator may need bounded predeclared delegation so a
parent can give a child an issue, tools, and limits and receive a structured result. This is a
new authority model, not a continuation of the recommendation projection.

Unbounded delegation could exceed scope, budget, permissions, or the user’s
approval while still appearing to be a chain of ordinary recommendations.

## Decision

Defer coordinator implementation behind the written Coordinator design gate.
Approval of the design gate authorizes only a separate implementation plan and its review;
it does not authorize coordinator code, child APIs, or runtime launches. If a
later implementation plan is approved, the coordinator will execute only an
immutable, user-approved root-plan revision represented as a bounded DAG.
Server-issued coordinator and child lease capabilities bind work to one
OrkWorks instance, workspace, canonical plan/evidence digests, approval ID,
capability revision, scope, tool envelope, provider allowlist, budget
reservation, and expiry. Runtime grants must fit the intersection of the active
parent lease, target node's pre-approved maximum envelope, and an unconsumed,
explicitly grantable predeclared slot; any expansion requires a new
revision and renewed user approval.

Budgets use normalized execution units with atomic reservations, explicit
retry/refund rules, concurrency ceilings, launch acknowledgements, and
crash-safe orphan recovery. Dependencies are fail-closed: cycles, duplicate
spawns, stale results, failed dependencies, late reports, and ambiguous leases
cannot advance work. Authenticated, version-checked, strict child reports are
validated against server-owned capabilities, canonical request hashes, and
idempotency records. Resource scopes are canonicalized and shared-workspace
conflicts are rejected before launch.

Children may edit files and run bounded commands within their declared scope.
Server-enforced hard denials apply to every role and arbitrary command path:
children do not receive Git mutation, merge approval, credentials, permission
changes, destructive actions, scope expansion, provider substitution, or
implicit delegation. A plan may explicitly declare bounded delegation slots,
but a child cannot create authority outside those slots. Structured capability
insufficiency requests are validated against the active node contract and
grant slots. Product/architecture decisions, authority expansion, conflicting
results, and unrecoverable blockers escalate to the user.

The existing single-active-context rule, explicit user escalation rules, and
Phase 1 completion-packet lifecycle remain unchanged. `ready_for_user_review`
never means user acceptance or dependency success. Audit records are bounded
and redacted; raw bearer capabilities, credentials, and unredacted command
output are never persisted.

## Amendment — 2026-09-24: enforceable design contract

This amendment clarifies the proposed decision; its status remains `proposed`.
The [Coordinator design gate](../superpowers/specs/2026-09-24-taskmaster-bounded-coordinator-design.md)
is authoritative for the future contract and is separate from Taskmaster
rollout Phase 2, the deterministic evaluator. Neither this ADR nor written
design approval authorizes implementation; a separate implementation plan
must still be reviewed and approved.

- Every child command/tool invocation crosses a server-owned broker checking
  executable identity, arguments, canonical cwd, allowlisted environment,
  declared resource effects, live lease, capability revision, resource ceilings,
  and hard denials. Children have no direct shell/process or tool access that
  bypasses it. Commands and descendants require enforceable confinement;
  child prose and asserted effects are not authority. Unsupported enforcement
  rejects the invocation.
- Each attempt requires finite wall-clock, aggregate CPU, memory, process-count,
  output-byte, token, cost, and tool-invocation ceilings; execution-unit counts
  alone are insufficient. Lost or uncertain launch responses retain budget and
  concurrency reservations as `orphaned`. No refund, retry, or replacement is
  allowed until explicit reconciliation proves non-start or termination;
  only proven non-start permits refund, and started work consumes its unit.
- Every write/write and write/read overlap is rejected unless an approved
  exclusive resource lease serializes access, including verification.
- Immutable nodes and delegation-slot templates declare `required` explicitly.
  Completion evaluates exactly that set, including transitive dependencies;
  unused optional slots do not block it. Launched optional work must be
  quiescent, and orphaned work cannot be omitted to claim completion.
- `paused -> active` requires fresh user approval of the same immutable
  revision bound to the current server-observed workspace subject, expiry,
  and revocation generation, with capability rotation. A changed evidence
  subject, scope, or budget requires a new revision. Pause quiesces tools.
- Every attempt binds server-observed input/output workspace revisions and
  change subjects with scoped hashes and write attribution. Acceptance
  atomically revalidates these against current content, dependencies, and
  lease state, rejecting stale or conflicting results.
- Cancellation, expiry, and revocation fence mutations, revoke tool channels,
  and require process-tree termination within a finite approved deadline.
  Unproven termination becomes `orphaned`/`recovery_required`; reservations and
  conflicting locks remain held without refund or relaunch pending reconciliation.
- Only server-attested broker/verifier receipts, observed command identity and
  result, and scope-bound output hashes may advance graph state. Receipts bind
  attempt, lease/capability, plan, and workspace subjects; child-authored claims
  and hashes remain untrusted context.
- The immutable revision includes effective prompt/context templates,
  deterministic derivation rules and exact inputs, server-rendered bytes, and
  their digest. Parent edits require a new revision and approval.
- The design's [hard portable limits](../superpowers/specs/2026-09-24-taskmaster-bounded-coordinator-design.md#hard-portable-limits)
  define finite graph/depth, field, prompt, command, evidence, report/output,
  and retained audit/lineage/idempotency bounds using foundation documentation
  defaults. Over-limit input is rejected before mutation. Redacted tombstones
  replace evicted evidence/audit payloads and count toward retention limits;
  immutable plan/approval/lineage digests survive in a bounded registry.
  Pinned proof or registry exhaustion blocks admission rather than silently
  increasing storage or discarding recovery evidence.

## Consequences

- A root plan can solve a bounded issue through parent-supplied tools without
  becoming an unrestricted autonomous swarm.
- Capability validation, budget accounting, and graph recovery become
  first-class coordinator responsibilities and require a separate PR sequence.
- Dynamic child requests can be satisfied efficiently within pre-approved
  authority, while new authority remains a user decision.
- The coordinator must preserve immutable plan/approval/lineage digests and
  bounded, redacted evidence or tombstones for stale, cancelled, failed, and
  escalated work under the design's admission and retention limits.
- Acceptance of this ADR and the accompanying Coordinator design gate
  authorizes only a separate implementation plan and its review. Coordinator
  implementation remains unauthorized until that separate plan is approved.
