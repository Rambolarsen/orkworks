# Taskmaster Bounded Coordinator Design

- Status: proposed design gate
- Date: 2026-09-24
- Tracking issue: [#604](https://github.com/Rambolarsen/orkworks/issues/604)
- Related ADR: [0063](../../adr/0063-bounded-taskmaster-coordinator.md)

## Purpose and boundary

Phase 1 guided completion packets remain recommendation projections. This
document defines the boundary for a later Phase 2 coordinator; it does not
authorize coordinator code, child-session APIs, runtime launches, recursive
orchestration, autonomous swarming, Git workflow ownership, or user-acceptance
inference.

Approval of this design gate authorizes only a separate implementation plan and
its own review. It is not implementation authorization. The implementation
plan must preserve this document's invariants and must be approved before any
coordinator code or launch path is written.

If that later implementation is approved, the coordinator may execute a
user-approved bounded root plan. The root plan is a versioned, immutable DAG of
child tasks. It may authorize child sessions to solve assigned issues with
bounded file-editing and command-running tools while preserving explicit user
escalation for decisions and authority outside the plan.

## Authority model

The user approves one exact root-plan revision. The approval record is
server-generated and immutable. It contains:

- one OrkWorks instance identity;
- one workspace identity and canonical workspace snapshot/change subject;
- one canonical plan serialization and SHA-256 plan digest;
- one canonical evidence serialization and SHA-256 evidence digest;
- the declared graph, task scopes, roles, tool envelope, budgets, retries,
  concurrency ceilings, provider allowlists, and output contracts;
- an approval ID, user identity, approval timestamp, expiry, and revocation
  generation; and
- a supersession/lineage reference when this revision replaces another one.

The workspace snapshot/change subject is a server-produced record of the
workspace identity, repository revision, dirty-path set, and attribution
confidence at approval time. If concurrent dirty-workspace attribution is
ambiguous, the plan is inconclusive and cannot become active.

The canonical serializations use a versioned, deterministic representation:
object keys are ordered, absent and null values are distinct, arrays retain
declared order, strings are UTF-8, and digests are computed by the server over
the exact serialized bytes. Models and child reports cannot supply or replace
these digests.

Approval issues a server-generated coordinator capability. The capability is
opaque, non-forgeable, revocable, expiring, audience-bound to one OrkWorks
instance, and never supplied by a model or accepted from a child report. It is
not valid in another instance, workspace, plan revision, evidence digest, or
revocation generation.

Each child receives a separate lease capability bound to its child ID, parent
ID, plan revision and digest, coordinator approval ID, task scope, tool subset,
provider allowlist, budget reservation, and lease expiry. The server checks
the live capability before every tool invocation and report. A cancelled,
expired, revoked, paused, orphaned, or superseded lease cannot perform a new
mutation.

### Capability lattice and insufficiency requests

The server defines a closed capability lattice. A runtime grant is valid only
when the requested capability is an explicitly grantable subset of the active
parent lease and an unconsumed, predeclared grant slot. The server checks the
subset relation, scope containment, provider allowlist, budget, retry count,
concurrency class, and expiry atomically before issuing a new capability
revision.

The following are hard denials for every role, plan, delegation path, and
runtime grant. They cannot be represented as grantable capabilities:

- Git mutation, including commit, reset, checkout, merge, rebase, stash,
  branch mutation, push, and merge approval;
- credential acquisition or use and permission changes;
- destructive commands or deletion outside an explicitly user-approved,
  product-defined recovery operation; and
- workspace, path, role, tool, provider, budget, retry, or concurrency scope
  expansion.

Requests for those actions always become user escalations. An arbitrary
command tool cannot bypass the denials; the launcher rejects prohibited
commands and the server rejects prohibited capability declarations.

A child may request a missing capability from its parent using a structured
insufficiency report containing the node ID, task-contract reference, current
capability digest, requested capability enum, canonical scope, proposed
command or tool identifier, expected cost, reason the current set is
insufficient, and machine-checkable supporting evidence. The server validates
the report against the active node contract and current lease; child prose is
not authority. A valid request can consume one predeclared grant slot within
the parent envelope. A denied request leaves the child unchanged and records
the reason. A request outside the envelope or without a slot pauses the branch
and escalates to the user; it cannot create a new plan implicitly.

Capabilities are delivered through a channel-bound in-memory reference or a
short-lived protected transport token whose audience, lease, and expiry are
server-checked. Capability bearer values are never persisted, logged,
serialized into child reports, or placed in prompts. Audit records contain
capability IDs and digests, not secret token material.

## Plan and role model

Every node declares, as part of the immutable plan revision:

- a stable node ID and immutable parent ID;
- a concrete issue, success criteria, prompt/context, and output contract;
- a server-generated digest of the rendered prompt/context;
- workspace and canonical path/resource scopes;
- an explicit role and allowed tool set;
- dependency IDs and dependency outcome requirements;
- an approved model/harness identity or a server-enforced allowlist;
- retry and attempt limits;
- its budget reservation and concurrency class; and
- any delegation-slot constraints.

Changing the task, success criteria, prompt/context, output contract, rendered
prompt digest, role, tool set, scope, provider constraints, or any limit
creates a new plan revision and invalidates the old approval.

The default role set is:

- `implementation`: edit files and run bounded commands in the declared scope;
- `verification`: inspect and test without editing unless the plan explicitly
  grants a narrowly scoped correction capability;
- `review`: inspect and report findings without editing by default; and
- `remediation`: edit and verify a declared finding scope.

Git read operations may be available when declared. The hard denials above
apply even if a non-default role is added later. A node cannot grant itself a
role, tool, provider, scope, or limit.

Delegation is disabled by default. If a plan explicitly includes delegation
slots, each slot predeclares its complete constraints: slot ID, allowed parent,
node/task template, dependency shape, scope, role, tool subset, provider
allowlist, output contract, budget reservation, concurrency class, and retry
limit. The server instantiates a slot exactly once and records the child ID,
parentage, dependencies, and reservation atomically. A child cannot alter the
DAG or create an unlisted slot; otherwise a new approved plan revision is
required.

The parent supplies the child issue, success criteria, prompt/context, role,
tools, provider choice within the approved allowlist, budget, and reporting
contract. The child may use those tools to solve the issue rather than merely
perform one fixed command sequence.

## Plan revisions and lifecycle

Plan revisions are immutable. Changing graph edges, parentage, task content,
prompt/context, output contracts, scopes, roles, tools, provider constraints,
budgets, retries, concurrency, delegation slots, or evidence subject creates a
new revision. Runtime status, progress, evidence, and reports do not mutate
the plan definition.

The lifecycle is:

```text
draft -> proposed -> active -> completed
                    |          |
                    |          +-> failed
                    |          +-> cancelled
                    |          +-> expired
                    |          +-> revoked
                    +-> paused
                    +-> recovery_required
```

Only the user approves a proposed revision. Approval activates the coordinator
capability for that exact revision and approval record. A paused plan accepts
no new launches or tool invocations; its leases remain records but fail the
live invocation check until an explicit, still-valid resume action. Revocation,
expiry, or cancellation revokes leases, terminates each owned process tree,
and installs a hard mutation fence before recording the terminal state.

Completion requires every required node to have machine-validated terminal
success. `ready_for_user_review` is nonterminal and is never sufficient for a
dependent node or plan completion. Failure preserves blocking evidence. A
superseding revision preserves lineage and invalidates older capabilities.

## Budgets and capacity

The root plan declares hard integer budgets in normalized execution units,
per-role reservations, maximum concurrent children, maximum attempts per node,
maximum delegation depth, and per-node wall-clock, CPU, memory, output-size,
and process-count ceilings. A normalized unit is a server-owned reservation
for one declared attempt; the plan records the conversion from provider/tool
limits to units, and the launcher enforces the independent resource ceilings.

Reservations use this durable state machine:

```text
reserved -> launch_pending -> started -> completed
                         |       |\-> failed
                         |       +--> cancelled_after_start
                         +----------> cancelled_before_start/refunded
                         +----------> orphaned
```

The one-time launch token and authenticated launch acknowledgement define the
authoritative start point. A failed launch before acknowledgement releases the
reservation. Work in `reserved`, `launch_pending`, or an explicitly confirmed
pre-start cancellation is the only work that can be refunded. A crash after
the ambiguity boundary becomes `orphaned`; it is not refunded or relaunched
until explicit recovery creates a new lease and reservation. Completed or
started work consumes its reserved unit.

Reservation transitions, lease state, launch token use, and graph state are
committed atomically. A retry consumes a new reservation. Capacity exhaustion
prevents a launch and never expands authority or silently switches provider.
Each selected provider/harness must be in the node's approved identity or
allowlist, and the selected identity is recorded in the launch and audit
events.

## Graph, resource scopes, and execution semantics

The plan graph is a DAG with immutable parentage. Cycles, unknown dependency
IDs, duplicate logical spawns, unsatisfied dependency requirements, and
invalid slot instantiation are rejected before activation.

Workspace resources are canonicalized before planning and before launch.
Relative paths resolve under the workspace root; the canonical physical path
must remain inside that root; symlink traversal outside it is rejected. The
server uses the host's case-sensitivity rules and records the canonical form.
Resource scopes include paths, the `.git` directory, shared lockfiles, build
outputs, caches, ports, and declared external side-effect names. Concurrent
write/write and write/read overlap is rejected unless an approved exclusive
resource lease serializes it. Unlisted command side effects are rejected by
the launcher. This is the isolation boundary for the shared-workspace risk in
`specs/taskmaster.md`.

A dependent node launches only after all required dependencies report machine-
validated terminal success. Failed, cancelled, stale, conflicting, orphaned,
or expired results block dependents and escalate to the parent. Cancellation
propagates to descendants. Duplicate spawn requests use an idempotency key and
return the existing child outcome only when the canonical request matches.

Child results are terminal only when they include the declared output contract,
bounded structured evidence, verifier identity/result, capability revision,
current plan/lease identity, and the server-observed attempt identity. A child
statement alone is not verification evidence. The server validates evidence
schema, size, digest, and verifier authority; declared verifiers run through a
server-controlled command/result channel. `ready_for_user_review` cannot
advance graph state. Late or conflicting results are retained only through the
redacted audit policy below and cannot advance state.

## Authentication, validation, idempotency, and recovery

Coordinator and child APIs use authenticated, versioned request schemas with
strict unknown-field rejection. The server derives actor identity, instance,
workspace, node, scope, role, provider, and status authority from the live
capability registry. Every mutation/report carries the API version, plan
revision and digest, approval ID, capability revision, actor/node ID, lease ID,
attempt ID, and idempotency key; the server rejects client-supplied values that
disagree with live state.

Tokens are audience-bound, expiry-checked against server time, replay-protected
by nonce and idempotency records, and rejected across instance/workspace/plan
boundaries. Request size bounds apply to graphs, prompts, commands, reports,
output, evidence, and audit records. API-version changes are explicit
compatibility revisions; an unsupported version fails closed.

Idempotency keys are scoped to actor, operation, plan digest, and canonical
request hash. Pending and terminal outcomes are persisted atomically. Reusing
a key with a different request hash is a collision error; reusing an in-flight
key returns the same pending operation, never a second launch. Records have a
bounded retention period sufficient for recovery and reject reuse after expiry.

Launch, grant, retry, cancel, report, escalation decision, and recovery
operations are idempotent. On instance shutdown, all live capabilities and
leases are invalidated; active work becomes `recovery_required`/`orphaned` and
is never silently rerun. Restart reconciliation records process identity,
reservation state, and last accepted event before permitting any explicit
recovery. Only an explicit user action or an authenticated parent action under
a newly valid approved capability may create a new lease and reservation.

Cancellation and expiry revoke the lease, terminate the owned process tree,
close the tool channel, and enforce a mutation fence checked immediately
before every tool invocation. A late child report may be stored as a redacted
audit event but cannot reopen work or advance graph state.

## Mandatory user escalation

The coordinator must create a durable escalation decision object, bound to the
plan revision/digest, blocked node, parent chain, requested authority, request
digest, remaining budget, conflicting resource locks, evidence, and exact
decision required, for:

- product or architecture decisions;
- credentials or permission changes;
- destructive actions;
- Git mutation or merge approval;
- scope, role, tool, provider, budget, retry, or concurrency expansion;
- conflicting high-confidence results; and
- unrecoverable blockers.

The affected branch is paused. Conflicting resource scopes and the requested
budget reservation are frozen; unrelated branches may proceed only if their
live scopes and reservations do not conflict. Decisions transition exactly
once from `pending` to `approved`, `rejected`, or `expired`. Only the user can
approve. Approval is valid only for the exact request digest and current plan
revision; rejection/expiry fails or cancels the blocked branch. An expansion
requires a new proposed revision and renewed approval. The coordinator must
not infer approval from silence, a child report, or a `ready_for_user_review`
state.

## Audit and evidence handling

The audit stream is append-only and records event type, server timestamp,
instance/workspace IDs, actor/node/parent/lease/attempt IDs, plan and evidence
digests, approval ID, capability revision, idempotency key hash, state
transition, verifier identity, and redacted outcome. Raw capability values,
credentials, permission material, environment secrets, and unredacted command
output are never persisted or logged. Redaction is applied before storage and
before user display; hashes preserve correlation without revealing content.

Audit records and evidence are size-bounded and subject to a finite configured
retention limit. Eviction records a redacted tombstone and never removes the
immutable plan, approval, or lineage digest needed to prove what was approved.

## Design-gate acceptance criteria

Before implementation planning begins, written review must confirm that:

1. bounded root-plan approval is explicitly distinct from design approval and
   does not become unrestricted autonomy;
2. hard-denied capabilities cannot be reintroduced by roles, arbitrary
   commands, grants, delegation, or provider changes;
3. canonical plan/evidence digests and immutable approval records bind every
   later action to exactly what the user approved;
4. capability and lease checks are enforceable at every launch, tool, grant,
   report, retry, cancellation, escalation, and recovery boundary;
5. the total budget and concurrency ceilings cannot be exceeded by delegation,
   retries, refunds, duplicate spawns, or crash recovery;
6. canonical resource scopes prevent unsafe concurrent mutation in a shared
   workspace;
7. stale, malformed, unauthorized, cancelled, cross-workspace, orphaned, and
   late results fail closed, and `ready_for_user_review` is nonterminal;
8. evidence, secrets, outputs, and audit lineage have bounded, redacted
   persistence; and
9. implementation is split into a separate plan and PR sequence from the
   Phase 1 completion-packet work.

No coordinator code, child-session API, or autonomous launch path should be
implemented until this design and [ADR 0063](../../adr/0063-bounded-taskmaster-coordinator.md)
are approved in writing. Once approved, that approval authorizes only the
separate implementation plan and its review gates.
