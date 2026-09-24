# Taskmaster Bounded Coordinator Design

- Status: proposed Coordinator design gate
- Date: 2026-09-24
- Tracking issue: [#604](https://github.com/Rambolarsen/orkworks/issues/604)
- Related ADR: [0064](../../adr/0064-bounded-taskmaster-coordinator.md)

## Purpose and boundary

Phase 1 guided completion packets remain recommendation projections. This
document defines the boundary for the separate Coordinator design gate; it
does not authorize coordinator code, child-session APIs, runtime launches, recursive
orchestration, autonomous swarming, Git workflow ownership, or user-acceptance
inference.

The existing Taskmaster rollout Phase 2 is the deterministic evaluator. The
Coordinator design gate does not rename or extend that rollout phase.

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
- the declared graph, required-node set, effective prompt/context templates,
  derivation inputs and rules, server-rendered bytes and their digests,
  task scopes, roles, tool envelope, budgets, retries,
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
parent lease, the target node's pre-approved maximum capability envelope,
and an unconsumed, predeclared grant slot. All three bounds apply together;
an implementation parent's editor cannot be granted to a review node whose
maximum envelope excludes editing. The server checks the
subset relation, scope containment, provider allowlist, budget, retry count,
concurrency class, and expiry atomically before issuing a new capability
revision.

The following are hard denials for every role, plan, delegation path, and
runtime grant. They cannot be represented as grantable capabilities:

- Git mutation, including commit, reset, checkout, merge, rebase, stash,
  branch mutation, push, and merge approval;
- credential acquisition or use and permission changes;
- destructive commands or deletion; and
- workspace, path, role, tool, provider, budget, retry, or concurrency scope
  expansion.

Requests for those actions always become user escalations. An arbitrary
command tool cannot bypass the denials; the launcher rejects prohibited
commands and the server rejects prohibited capability declarations.
Any product-defined destructive recovery operation requires separate explicit
user approval and cannot run through a coordinator child or grant.

### Tool-broker enforcement

Every child command and tool invocation crosses a server-owned policy broker.
Immediately before execution, it atomically validates the executable identity,
arguments, canonical cwd, allowlisted environment, declared read/write and
external resource effects, active lease, capability revision, remaining
resource ceilings, and hard denials. It holds the applicable resource leases
through execution and result capture. Child-authored prose, tool names, and
claimed side effects cannot authorize execution.

Children have no direct shell, process-spawn, filesystem, or tool-channel
access that bypasses the broker. Approved commands and their descendants must
run under enforceable confinement to the declared resources and ceilings;
checking a command name alone is insufficient. Shell/interpreter indirection,
scripts, and subprocesses receive the same restrictions. If the server cannot
prove or enforce the requested effects on a supported platform, it rejects the
invocation and escalates instead of running an unrestricted command.

A child may request a missing capability from its parent using a structured
insufficiency report containing the node ID, task-contract reference, current
capability digest, requested capability enum, canonical scope, proposed
command or tool identifier, expected cost, reason the current set is
insufficient, and machine-checkable supporting evidence. The server validates
the report against the active node contract and current lease; child prose is
not authority. A valid request can consume one predeclared grant slot within
the intersection of parent authority and the target node's approved maximum
envelope. A denied request leaves the child unchanged and records the reason.
A request outside either envelope or without a slot pauses the branch
and escalates to the user; it cannot create a new plan implicitly.

Capabilities are delivered through a channel-bound in-memory reference or a
short-lived protected transport token whose audience, lease, and expiry are
server-checked. Capability bearer values are never persisted, logged,
serialized into child reports, or placed in prompts. Audit records contain
capability IDs and digests, not secret token material.

## Plan and role model

Every node declares, as part of the immutable plan revision:

- a stable node ID and immutable parent ID;
- an explicit `required` boolean (also on every delegation-slot node template);
- a concrete issue, success criteria, and output contract;
- the effective prompt/context template, deterministic derivation rules and
  exact input snapshots, server-rendered bytes, and their SHA-256 digest;
- workspace and canonical path/resource scopes;
- an explicit role, initial allowed tool set, and maximum capability envelope;
- dependency IDs and dependency outcome requirements;
- an approved model/harness identity or a server-enforced allowlist;
- retry and attempt limits;
- its budget reservation and concurrency class; and
- any delegation-slot constraints.

Changing the task, success criteria, prompt/context, output contract, rendered
prompt digest, role, tool set, scope, provider constraints, or any limit
creates a new plan revision and invalidates the old approval.

Rendering happens before approval; later workspace reads or parent messages
cannot silently replace the approved context inputs. A parent edit to the
template, derivation rule, input, or rendered content requires a new revision
and renewed user approval, including for an unused delegation slot.

Completion evaluates exactly the immutable required-node set. A required
delegation-slot node must be instantiated and succeed; an unused optional slot
does not block completion. Required nodes must include all their transitive
dependencies in the required set; an invalid declaration is rejected before
approval. Optional work cannot satisfy a required node by substitution. Before
the plan becomes completed, any launched optional work must also be quiescent;
failed required nodes and unresolved orphaned work cannot be silently omitted.

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

The parent proposes the child issue, success criteria, prompt/context, role,
tools, provider allowlist, budget, and reporting contract before approval.
At dispatch the server supplies the approved content and validates the selected
provider against that allowlist. The child may use those tools to solve the
issue rather than merely perform one fixed command sequence.

## Plan revisions and lifecycle

Plan revisions are immutable. Changing graph edges, parentage, task content,
prompt/context, output contracts, scopes, roles, tools, provider constraints,
budgets, retries, concurrency, delegation slots, or evidence subject creates a
new revision. Runtime status, progress, evidence, and reports do not mutate
the plan definition.

The lifecycle is:

```text
draft -> proposed -> active -> completed
                    active -> paused
                    paused -> active (fresh user approval)
           active / paused -> failed / cancelled / expired / revoked
           active / paused -> recovery_required
```

Only the user approves a proposed revision. Approval activates the coordinator
capability for that exact revision and approval record. A paused plan accepts
no new launches or tool invocations; its leases remain records but fail the
live invocation check. `paused -> active` requires fresh user approval of the
same immutable revision, bound to the server-observed current workspace change
subject, expiry, and current revocation generation. The server revalidates
those bindings atomically and rotates coordinator and child capabilities;
old capabilities remain invalid. A changed evidence subject, scope, or budget
requires a new revision, not resume. Expired or revoked approval cannot be
revived by a parent action. Pause quiesces in-flight tools using the bounded
termination rule below; uncertainty enters `recovery_required` and blocks resume.
Revocation, expiry, or cancellation installs the mutation fence, revokes tool
channels and leases, and requests bounded termination of each owned process
tree before recording stopped work as terminal.

Completion requires every required node to have machine-validated terminal
success. `ready_for_user_review` is nonterminal and is never sufficient for a
dependent node or plan completion. Failure preserves blocking evidence. A
superseding revision preserves lineage and invalidates older capabilities.

## Budgets and capacity

The root plan declares hard integer budgets in normalized execution units,
per-role reservations, maximum concurrent children, maximum attempts per node,
maximum delegation depth, and mandatory per-attempt ceilings for wall-clock
milliseconds, aggregate process-tree CPU milliseconds, peak aggregate memory
bytes, process count, total output bytes, total input/output tokens, cost in a
declared integer currency unit, and tool-invocation count. Every ceiling is a
finite, explicit approved integer; absent or unlimited values are invalid.
A zero ceiling prohibits consuming that resource, and a zero cost ceiling is
valid only for a demonstrably free provider. These ceilings apply cumulatively
across all tools, descendants, and provider requests within the attempt.

A normalized unit is a server-owned reservation for one declared attempt;
the plan records the conversion from provider/tool limits to units. Attempt
counts never replace the independent ceilings. The broker reserves worst-case
token/cost and tool usage before dispatch and enforces time/process limits
during execution. A provider or platform unable to enforce a ceiling is
ineligible. Exhaustion stops work under the cancellation rules and escalates;
retries require new reservations and cannot reset a running attempt's counters.

Reservations use this durable state machine:

```text
reserved -> launch_pending -> started -> completed
                         |       |\-> failed
                         |       +--> cancelled_after_start
                         +----------> cancelled_before_start/refunded
                         +----------> orphaned
```

The one-time launch token and authenticated launch acknowledgement record a
confirmed start; absence of acknowledgement is not proof of non-start. A lost
or uncertain launch response retains the budget and concurrency reservation,
becomes `orphaned`, and fences further invocations until explicit reconciliation
proves whether work exists. No refund, retry, or replacement lease is allowed
before that proof. Only server-proven non-start (including cancellation before
dispatch) permits refund; `reserved` or `launch_pending` status alone does not.
If work started, its unit remains consumed even after proven termination.
Recovery may then authorize a new lease and reservation within the original
attempt, budget, and concurrency limits. Completed or started work consumes
its reserved unit.

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
write/write and write/read overlap is always rejected unless an approved
exclusive resource lease serializes the accesses, including commands and
verifiers; declaring a scope non-exclusive is not an exemption. The broker
rechecks scopes and lease ownership at every invocation and rejects unlisted
command side effects. This is the isolation boundary for the shared-workspace
risk in `specs/taskmaster.md`.

A dependent node launches only after all required dependencies report machine-
validated terminal success. Failed, cancelled, stale, conflicting, orphaned,
or expired results block dependents and escalate to the parent. Cancellation
propagates to descendants. Duplicate spawn requests use an idempotency key and
return the existing child outcome only when the canonical request matches.

Every attempt carries server-observed input and output workspace revisions and
change subjects, including scoped content hashes and attribution of writes.
The input records the state actually read at dispatch, including accepted
dependency outputs; the output records the state actually produced or verified.
Missing or ambiguous attribution is inconclusive. Result acceptance atomically
compares these subjects, live scoped content, dependency evidence, and lease
state under the resource fence before committing graph progress. A user or
other session changing the relevant content makes the result stale or
conflicting and blocks acceptance, even if the plan ID and Git HEAD match.
If a coherent comparison cannot be guaranteed, the server fails closed.

Child results are terminal only when the declared output contract is satisfied
by server-attested evidence. Only broker/verifier receipts generated and
recorded by the server (or signed by its trusted verifier) can advance graph
state. Receipts bind the observed executable/tool identity and arguments,
result/exit status, verifier identity, scope-bound output hashes, input/output
workspace subjects, attempt ID, plan/lease identity, and capability revision.
The server validates schema, size, digest, provenance, and verifier authority
against its receipt registry. Child-authored test claims, hashes, and prose
remain untrusted context, even when structurally valid; a child cannot mint a
receipt. `ready_for_user_review` cannot advance graph state. Late or conflicting
results are retained only through the redacted audit policy below.

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
boundaries. The hard portable limits below apply to graphs, prompts, commands,
reports, output, evidence, and audit records. API-version changes are explicit
compatibility revisions; an unsupported version fails closed.

Idempotency keys are scoped to actor, operation, plan digest, and canonical
request hash. Pending and terminal outcomes are persisted atomically. Reusing
a key with a different request hash is a collision error; reusing an in-flight
key returns the same pending operation, never a second launch. Records have a
bounded retention under the hard portable limits below. Live recovery records
are pinned; terminal records may be evicted only after invalidating their
capability generation, so missing or expired keys can never authorize replay.

Launch, grant, retry, cancel, report, escalation decision, and recovery
operations are idempotent. On instance shutdown, all live capabilities and
leases are invalidated; active work becomes `recovery_required`/`orphaned` and
is never silently rerun. Restart reconciliation records process identity,
reservation state, and last accepted event before permitting any explicit
recovery. It must prove non-start or complete process-tree termination before
any refund or replacement, retaining reservations and conflicting resource
locks while uncertain. Only an explicit user action or an authenticated parent
action under a newly valid approved capability may then create a new lease
and reservation within the approved limits; neither bypasses fresh user
approval for resuming a paused plan.

Cancellation, expiry, and revocation immediately revoke the lease and all tool
channels, enforce the invocation fence, and terminate the entire owned process
tree within a mandatory finite termination deadline recorded in the plan.
Cancellation is complete only after server proof of termination or durable
classification as `orphaned` with the plan in `recovery_required`. An orphan is
not successful or proven stopped: retain its reservation and conflicting locks,
and permit no refund or relaunch until reconciliation proves non-start or
termination under the accounting rules above. A late child report may be
stored as a redacted audit event but cannot reopen work or advance graph state.

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

Within the retained window, audit events are append-only and record event type,
server timestamp, instance/workspace IDs, actor/node/parent/lease/attempt IDs, plan and evidence
digests, approval ID, capability revision, idempotency key hash, state
transition, verifier identity, and redacted outcome. Raw capability values,
credentials, permission material, environment secrets, and unredacted command
output are never persisted or logged. Redaction is applied before storage and
before user display; hashes preserve correlation without revealing content.

Eviction redacts payloads and replaces evidence/audit records with bounded
tombstones recording their digests, sequence range, reason, and lineage
reference. Adjacent tombstones may be compacted into a digest-linked range;
tombstones count toward the same retention caps. Immutable plan, approval, and
lineage digests remain in the separate bounded registry and are never evicted.
Evidence needed by live work or reconciliation is pinned. If no safe eviction
can fit a new record, admission stops before mutation or dispatch; it cannot
drop proof, expand storage silently, or append unlimited rejection events.

## Hard portable limits

These are versioned documentation defaults for the proposed contract, using
the foundation's 128-node, 128-byte ID, 16 KiB text, 256-item collection, and
2 MiB revision bounds. They do not assert runtime enforcement or authorize
implementation. Sizes are UTF-8 or serialized bytes, with 1 KiB = 1,024 bytes
and 1 MiB = 1,048,576 bytes. All applicable limits apply together.

| Surface | Hard maximum |
| --- | --- |
| Graph | 128 total nodes, including all possible delegation-slot nodes; dependency and delegation depth 127 edges (root depth 0) |
| IDs and collections | 128 bytes per ID; 256 items per collection, including dependencies, scopes, tool/provider lists, arguments, and environment entries |
| Text fields | 16 KiB each, including task, success criterion, path, reason, and output contract |
| Effective prompt/context | 16 KiB each for the template, total derivation inputs, and final rendered prompt/context per node; all are included in the revision |
| Plan revision or incoming request | 2 MiB total serialized bytes, including nested values; structured payload nesting depth 128 |
| Command/tool request | 16 KiB total, including executable, arguments, cwd, environment, and declared effects |
| Evidence | 256 items and 2 MiB aggregate per report; 16 KiB per item including receipt metadata |
| Child report and output | 2 MiB per serialized report; 2 MiB total captured output per attempt before redaction, or the approved attempt ceiling if lower |
| Retained evidence/audit window | 256 records and 2 MiB aggregate per workspace, including tombstones; 16 KiB per record |
| Immutable plan/approval/lineage digest registry | 256 revision entries and 2 MiB aggregate per workspace; 16 KiB per entry, preserving all approval and supersession digests |
| Retained plan definitions | At most the registry's 256 revisions per workspace, each at most 2 MiB |
| Idempotency/recovery records | 256 records and 2 MiB aggregate per retained revision; 16 KiB per record; at most 256 retained revisions per workspace |

The server bounds reads before parsing and validates count, depth, individual
field, and aggregate limits before any state mutation, reservation, or launch.
Oversized input is rejected, never truncated into an apparently valid request.
Output capture stops at its limit and fails the attempt under the termination
rules. Raw output is not persisted; retained redacted receipts must also fit
the evidence/audit window. Redaction and tombstone creation reserve space
before accepting the operation. Admission stops when the immutable registry is
full, including when one revision would overflow its approval/lineage entry;
preserving immutable digests does not imply unlimited revision retention.
Configured or approved limits may be lower, never higher within this contract
version. Raising a hard maximum requires a reviewed contract revision and new
plan approval; no child or parent can increase it at runtime.

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
implemented until this design and [ADR 0064](../../adr/0064-bounded-taskmaster-coordinator.md)
are approved in writing. Once approved, that approval authorizes only the
separate implementation plan and its review gates.
