---
type: Design
title: Harness integration application module
description: Move existing harness-integration orchestration behind a typed Rust application interface while preserving transport and runtime behavior.
tags: [harness, integration, architecture]
status: accepted
---

# Harness integration application module

The owner approved this written design on 2026-10-10 after independent review
and its clarification, with “looks good. keep going”.
Tracked by [#816](https://github.com/Rambolarsen/orkworks/issues/816).

## Outcome

Callers request integration operations without learning HTTP implementation,
probe sequencing, state locks, or authority-revocation bookkeeping. HTTP
handlers become transport adapters. Existing observable behavior is preserved.

Pass condition: the same integration scenarios produce the same configuration,
attention state, cleanup outcomes, and HTTP status/body shapes through the new
module, and non-HTTP callers no longer depend on integration HTTP helpers.

## Sources and scope

Product scope comes from [resolved harness capabilities and integrations](../../../specs/orkworks-mvp.md#resolved-harness-capabilities-and-integrations)
and [deterministic harness-supplied signals](../../../specs/orkworks-mvp.md#deterministic-harness-supplied-signals).
[ADR 0026](../../adr/0026-resolved-harness-capability-registry.md) constrains
registry identity, closed bindings, ownership-aware mutations and authority;
[ADR 0028](../../adr/0028-generation-aware-harness-version-probe-cache.md) and
[ADR 0030](../../adr/0030-integration-lock-check-await-helper.md) constrain
probe/cache/revalidation sequencing. [ADR 0073](../../adr/0073-other-harness-prompt-attention-authority.md)
and the [integration contracts](../../agents/harness-integration-contracts.md)
constrain existing readiness and prompt-authority revocation.

Extract only orchestration currently in `http/integration_handlers.rs`, and
rewire its existing callers. Preserve the existing tool adapters, reporter
scripts, configuration transaction implementation, launch arguments, public
routes, persistence formats, and Electron confirmation flow. Keep probe process
ownership and [#271](https://github.com/Rambolarsen/orkworks/issues/271)'s
JsonHookHandler generalization separate. This does not implement agent roles,
child orchestration, or currently gated attention behavior.

## Module and interface

Add `crates/orkworksd/src/harness_integration_application.rs`, registered by
`main.rs`. A concrete `HarnessIntegrationApplication` accepts the existing
`Arc<AppState>` and coordinates its existing stores and locks. It owns no
second registry, session map, workspace store, cache, or persistence layer.

The intended caller interface is:

| Operation | Inputs | Output |
| --- | --- | --- |
| Inspect | Harness ID or integration key | Typed harness or grouped status, or application error |
| Mutate | Target, install/repair/uninstall, revision expectation where currently required | Typed harness or grouped status, or application error |
| List workspace integrations | Current application state | Grouped statuses, or application error |
| Reconcile unreferenced integrations | Integration keys and expected workspace path | Typed cleanup outcome |
| Check prompt-attention launch readiness | Harness ID and executable | Conservative readiness boolean |

Inspect and List are observations with an existing reconciliation side effect:
for eligible Claude/Copilot targets, discovering a missing or drifted prompt
hook can revoke session prompt authority and clear the prompt tuple under the
existing source rules. They do not install, repair or uninstall configuration,
and they do not activate prompt authority. Document this side effect on the
Rust interface methods themselves. Launch readiness is only a conservative
boolean query and performs no revocation.

Inspect completes any required captured revocation before returning its status
or adapter failure. A revocation failure takes precedence over that status or
failure and maps to the existing HTTP 500 response. List attempts every captured
revocation in collection order, including after another attempt fails and before
returning a later group's revalidation error. Any revocation failure takes
precedence over the later revalidation error; otherwise return that error,
without a partial list. Successful prior revocations are not rolled back.
Listing may await subsequent group probes after capturing an earlier snapshot;
preserve this existing timing, without adding new suspension points.

Use enums for target, mutation and the harness/grouped result distinction.
Group mutation requests retain required document and active-selection revisions;
legacy harness requests retain their existing contract. Encode this distinction
in the request types so callers cannot construct a group mutation without its
revision expectation. Repair retains the current install/reconcile behavior.
The module owns the typed grouped-status, cleanup-outcome and revision-conflict
records consumed by both HTTP adapters and other application callers.

Errors distinguish missing workspace, unknown target, workspace/definition
changes, revision conflicts carrying current revisions, configuration or
adapter failure, infrastructure failure, and authority-revocation failure.
Application errors contain domain data rather than HTTP status codes or Axum
responses. HTTP mapping retains existing error text and body shapes.

The module also owns reporter-asset resolution currently exposed by the HTTP
implementation. Keep ordinary configuration/asset helpers private where
possible; share an existing helper at the harness implementation seam only if
another existing caller actually needs it.

## Ownership and sequencing

The external seam sits between callers and the complete integration operation.
Callers do not supply callbacks that must obey the lock/probe/revalidation
protocol, or receive state guards, mutable configuration documents, reporter
paths, or unfinished revocation effects.

Keep the existing legacy and grouped revalidation paths private inside the
module during this extraction. They have different compatibility contracts;
combining them is not a prerequisite for depth. Keep the existing probe cache
and its invalidation behavior.

An operation captures the current target identity and workspace, probes without
holding synchronous guards, then performs the existing final checks and adapter
action under the existing projection/workspace coordination. Preserve the
current lock order and synchronous mutation window. No synchronous guard may
cross an await. Do not add a new asynchronous gap between configuration mutation
and required revocation finalization.

Prompt readiness is inspected at the same existing points before/after an
action. The module captures the existing revocation snapshot when required,
releases operation guards, and asks `SessionApplication` to perform the existing
fenced revocation. `SessionApplication` remains the owner of session attention
and token/generation state. Launch readiness stays a synchronous conservative
query; it does not activate authority or issue generations.

Workspace listings retain their existing partial-progress behavior: if a later
group fails revalidation, finish all already-captured revocation attempts before
returning that error. Attempt every captured snapshot even after another
revocation fails. Preserve current error precedence.

Cleanup retains its existing selection, ownership checks, diagnostics and
complete/cleanup-needed outcomes; extraction must not add new mutation or
revocation policy to that path.

## Compatibility and failures

Grouped adapter failures currently become a grouped status with registration
`error`, often returned with HTTP 200. Legacy adapter failures become HTTP
errors. Preserve this difference in the typed operation result and HTTP mapping;
do not normalize it into one new failure convention. Preserve absent/untracked,
ambiguous ownership, trust-pending, disabled, unsupported and shared-consumer
states independently.

Revision conflicts retain the existing code and current-revision fields.
Workspace or harness replacement during a probe must target neither stale
configuration nor the replacement workspace. Cache hits do not bypass final
identity checks.

Configuration publication and runtime revocation are not one transaction.
If configuration changes successfully but required revocation fails, report
the existing failure and retain the durable configuration change. Do not claim
rollback, report success, retry silently, or restore configuration behind the
user's back. Reinspection/reconciliation remains the existing recovery path.

## Callers and dependencies

- `http/integration_handlers.rs`: keep request extraction, strict mutation-body
  parsing, transport error/status/body mapping, and handler entrypoints. Existing
  router middleware remains responsible for mutation authorization.
- `http/harness_handlers.rs` and `http/session_handlers.rs`: call application
  cleanup directly and serialize the same cleanup result.
- `session_application.rs`: query application launch readiness directly rather
  than calling into `http`. Interaction with session revocation remains an
  explicit cooperation between the two concrete application modules; it adds
  no recursive lock acquisition or new state ownership.
- `harness/integration.rs`, `harness/integrations/*`, registry and detection:
  retain their existing interfaces and behavior.
- Electron/renderer: consume the unchanged protocol without code changes.

Dependency categories: registry/state derivation is in-process; filesystem and
Git behavior are locally testable with temporary workspaces/repositories;
tool probing uses controlled local executables; actual coding tools are true
external dependencies whose contracts are unchanged. Reuse existing adapters
and probe fixtures. Add no general storage/probe trait or plugin framework for
this extraction. The shared AppState and internal test setup are implementation
details, not an expanded caller interface.

## Verification

Before moving behavior, establish the existing integration-handler baseline.
Exercise application operations directly with the existing temporary workspace,
FakeHome, controlled executable and AppState fixtures. Cover:

- Fresh/installed/drifted/ambiguous configuration; unrelated hooks preserved.
- Shared consumers; disabling one consumer retains the shared installation;
  unreferenced cleanup preserves foreign fragments and reports manual action.
- Stale active-selection/document revisions, workspace switching and harness
  edits during a slow probe; rejected mutations write no target configuration.
- Version thresholds, probe-cache reuse and generation invalidation; a slow
  probe leaves workspace requests responsive.
- Readiness before/after mutation, drift demotion, current session-generation
  fencing, and existing behavior when required revocation fails after mutation.
- Workspace listing attempts every captured revocation and finalizes captured
  effects before surfacing a later group's revalidation failure.
- Direct Inspect/List tests assert authority demotion and prompt-tuple handling
  under the existing source rules without configuration mutation. Assert that
  revocation failure takes precedence over adapter failure or a later listing
  revalidation failure, that successful prior revocations remain applied, and
  that launch-readiness queries leave authority unchanged.

Move orchestration tests to the application interface as their behavior moves;
retain focused HTTP tests for request validation, authorization, exact conflict
payloads and grouped-versus-legacy failure mapping. Keep adapter-specific tests
and existing reporter fixtures. Replace redundant orchestration tests rather
than layering a second mirrored suite over the HTTP handlers.

Run Rust build, the full Rust test suite, formatting check, `git diff --check`,
and `scripts/doc-check.sh`. Update the architecture reference when the module
lands. Review user-facing docs for changed claims; this extraction should
require none because behavior is unchanged. A concurrency/lifecycle-aware
`/code-review medium` must cover the final code head before merge.

## Uncertainty checkpoint and alternatives

Least confidence: failure and cancellation ordering across configuration writes
and runtime revocation. Inspection shows existing action/revocation sequencing
and grouped/legacy error distinctions are load-bearing. Keep those sequences
intact and exercise partial failures through the new interface.

Project blind spot: a status read is not universally side-effect-free; existing
inspection can demote stale prompt authority. The interface must document this,
and tests must assert it. Making inspection pure is a separate behavior change.

Alternative 1: only move functions to another file while retaining Axum results
and callback choreography. Rejected: callers still learn transport and ordering.
Alternative 2: generalize tool adapters first. Deferred: #271 covers part of
that work and it does not fix application callers depending on HTTP helpers.
Selected approach: one concrete application module, typed outcomes, existing
private orchestration and adapters.

This continues ADR 0030's integration-specific choreography decision. Clarify
its ownership in a dated amendment before implementation, keep its accepted
status, update the architecture reference and review the ADR index/README;
do not supersede it merely because the implementation moves.

## Independent design review

A fresh-context `gpt-6-luna` agent at high reasoning effort reviewed design
commit `a1e2f15c4d5f271b2f081a2b313ac42cd9c8594d` using the repository's
`reviewing-plans` skill. It inspected the actual orchestration, callers and
tests. One finding requested that inspection side effects and failure ordering
be stated at the interface. The existing sequencing guarantee was retained and
made explicit above, with direct verification criteria. No other actionable
findings were reported. The same reviewer rechecked the clarification against
source and reported no remaining actionable findings.

The review assessed complexity separately from design quality:

| Dimension | Rating | Evidence |
| --- | --- | --- |
| Dependencies | 3/5 | Known HTTP, session application, AppState, registry, probe and adapter interactions |
| Blast radius | 5/5 | Integration inspection/mutation and configuration/attention handling across consumers |
| State changes | 3/5 | Existing configuration writes and authority revocation must be preserved; no new format |
| Reversibility | 2/5 | Code extraction and ADR amendment are revertible without a format migration |
| Uncertainty | 3/5 | Inspected sequencing, with partial failures and lock behavior requiring verification |

Total: **16/25**, as assessed by the reviewer; the score neither grants owner
approval nor replaces implementation verification. Initial quality was
**Revise** for the interface clarification; recheck quality is **Ready for owner
review**. The owner subsequently approved the revised design before implementation
planning.

## Approval and next step

Owner review of the written design is complete. Execute the reviewed
[implementation plan](https://github.com/Rambolarsen/orkworks/blob/main/docs/superpowers/plans/2026-10-10-harness-integration-application.md),
including the behavior checks above. Implementation and documentation belong
to one logical PR for #816.
