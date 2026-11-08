---
type: Capability Plan
title: Scope continuous-parent event and resume checks
description: Three finite functional checks for the accepted preparation lifecycle, with model-free preparation and no authorized live calls.
tags: [orchestration, continuation, resume, capabilities]
status: stable
workflow_status: offline-fixture-implemented
---

# Scope continuous-parent event and resume checks

Outcome: specify the smallest fixture that distinguishes a continuously waiting
parent, event-gap recovery and exact restart/resume from ordinary session logs.
Pass condition for this handoff: three checks with observable oracles, owned setup,
denial cases and an endpoint. This change does not execute them.
Tracking: [#742](https://github.com/Rambolarsen/orkworks/issues/742), under #738.

## Authority and reused basis

The owner accepted the [logical lifecycle and existing limits](../specs/2026-10-04-orchestrator-preparation-design.md#logical-lifecycle-acceptance--2026-11-08)
in #1559. The [#740 final handoff](../../validation/copilot-practical-research-handoff.md)
is complete; its same-parent cases were never exercised. This scopes that
feature-specific work without repeating role research or universal containment.
Ordinary native/instruction-only limits remain disclosed.

**Current authorization: documentation and model-free inspection only; zero live
model calls.** No coding harness/server, native session, credential/configuration
change or production integration is started. Historical probe approval is spent.
Offline fixture preparation is a separate bounded implementation task; native
execution requires the concrete configuration and finite budget described below.

## Investigated source facts

At main `0793fb1a`, the accepted protocol requires a declared event-wait tool,
one outstanding wait per bearer, maximum 30 seconds per wait, authoritative state
reads, a 256-event metadata ring and explicit gap/closed-run results. The Rust and
Electron search found no `waitForRunEvent`, `PlanExecutionGrant`,
`stageContinueRequired` or `awaiting_resume_approval` implementation. Existing
coordinator/store definitions explicitly carry no runtime authority. Ordinary
session resume is not this orchestration state machine.

Pinned local Copilot 1.0.90 package/API/event-schema bytes still match the
[retained identities](../../validation/fixtures/copilot-1.0.90-native/runtime-identity.json).
The [static selection](../../validation/fixtures/copilot-1.0.90-native/continuation-api-selection.json)
records exact JSON pointers from API SHA-256
`749f184068021a6a05b0d8234ecb8cb4d392a5b9b044c87f619bde374eab928d`.
References are not expanded: this is neither a closed wire schema nor a receipt.

- `session.tools.handlePendingToolCall` names a session and pending request:
  a candidate tool-result boundary, not evidence of a model wakeup.
- `session.eventLog.read` observes native events, optionally waiting 30 seconds.
  Its native log cursor is distinct from the OrkWorks run-event cursor.
- `sessions.open` offers specific resume and returns status/session identity.
  Create/latest/attach/remote/fork are not exact-resume fallbacks.
- `session.sendMessages` can start another turn over existing history. It cannot
  prove delivery into an already-waiting loop and is excluded from Check 1.

No SDK/ACP route is selected or installed. The next preparation task must bind
one fixture-only coordination-tool route (disposable stdio MCP is the candidate),
using the existing event-wait/run-read action contract. Discovery cannot replace
observed behavior. Recheck actual loaded runtime/startup/settings before a later
native launch; this selected-cache comparison is narrower.

## Three checks and their oracles

| Check | Synthetic setup and observable pass | Required negatives |
| --- | --- | --- |
| 1. Same waiting parent | One isolated parent calls the declared wait tool. Record its pending request, native ID, host process/runtime generation and fixture connection. Queue report, approval, blocker and capacity events separately, returning metadata-only references through that pending tool result. Without another prompt, session or process, observe the same parent read authoritative fixture state and emit the event-bound acknowledgement/action request. A report alone grants neither task success nor launch. | Duplicate notification has no duplicate effect; no-change timeout invents no progress; stopped model loop is unavailable; revoked/closed/wrong-generation requests fail. Fake approval grants no production authority. |
| 2. Cursor-gap recovery | Overrun the existing 256-event fixture ring, then present an old run cursor. Return `refresh_required` and current run version. Observe a bounded authoritative snapshot read, then the next wait using its returned cursor, without inferring omitted transitions. | Native log cursor is not a run cursor; partial history/stale snapshot cannot authorize a repeated effect. Run closure wakes the wait with `run_closed`, allowing no subsequent authorized read. |
| 3. Exact resume | End the owned parent runtime, retaining only synthetic native identity/history in the disposable root. Explicitly resume that exact identity in the same configuration/workspace with a new runtime generation and fixture-only bearer. Observe native ID equality and current-state read. Planning/collection resumes; unused tasks stay unlaunched until exact fresh approval. | Missing/wrong ID never creates, forks, attaches elsewhere or resumes latest. Reject old bearer/grant. Preserve blocker, clarification and stage-continue gate. Never retry a reserved task or revive a terminal run. |

Retain correlated native/tool and fixture receipts, not only a final model claim.
Bind each to the fixture instance, native ID, process/runtime generation, pending
request and run version/cursor. A marker or agent-supplied ID without independent
host/transport correlation is insufficient. Report fixture-owned fencing and
idempotency separately from native wake/resume: a passing mock does not qualify
production authority.

## Delivery boundary and next action

One writer owns one fixture packet; reuse the accepted logical shapes without
inventing a production API, persisted grant/store or protocol version.

The next offline unit owns only
`crates/orkworksd/tests/fixtures/parent-continuation/peer.py` and
`test_peer.py`, plus its indexed evidence note. Use Python stdlib direct calls and
synthetic identifiers; no listening server, subprocess harness, MCP configuration
or credentials. This isolates fake-client oracles from the later native transport
binding and leaves production modules untouched.

1. Prepare a disposable model-free coordination peer and scripted fake client for
   the table's pending-result, duplicate, gap, closed-run and stale-generation
   cases. Test failing-first. Pin exact server/tool schemas, code/configuration
   digests, ordered transcript and expected receipts. This proves fixture oracles
   only, not native support.
2. Select one native route after those oracles work. Review exact launch argv,
   settings/tool map, synthetic startup/graph-skill bytes, loaded runtime,
   model/provider, observation fields, native-history exception, authentication
   exposure, retention/cleanup and network boundary. These metadata oracles need
   no broad prompt/content/reasoning capture or ambient environment dump.
3. Add a finite owner-approved live budget to the same packet before launch:
   permitted processes/model requests or a measurable turn limit, wall time,
   resource/log caps and cleanup. One CLI process does not prove a hidden provider
   request count. Until specified and approved, the live budget remains zero.
4. Run at most that batch. Unknown identity, unavailable route, unexpected access,
   timeout, missing correlation or exhausted budget ends it with per-check
   passed/failed/unverified results. No automatic retry, alternative harness/model/
   transport, wider capture or permission widening follows.

Completion of this handoff requires reviewed source pointers/oracles/boundaries,
exact selection-to-source comparison and docs checks. All three native checks
remain **unverified**. A decisive unsupported route may end the later batch as a
feature limitation; it neither reopens #740 nor blocks ordinary review-child work.
A later runtime plan still needs successful native observations and the other
selected-role/source/runtime prerequisites.

## Plan review

**Ready for offline fixture preparation; Investigate for native execution.**
Least confidence: whether returning a tool result actually continues the same
model loop. Matching static schemas give a lead, not an answer. Investigated blind
spot: native history-event cursors and fixture-owned run cursors/authority are
different; their separate oracles prevent a mock from certifying its integration.
No additional role, storage or capture research is bundled in.

| Dimension | Rating | Reason |
| --- | --- | --- |
| Dependencies | 2 | Accepted lifecycle/pinned source; one future fixture owner. |
| Blast radius | 1 | Documentation/static projection only. |
| State changes | 1 | No native process/session/model/runtime mutation. |
| Reversibility | 1 | Revert isolated docs; future fixture owns disposable roots. |
| Uncertainty | 4 | Native wake/resume unobserved; explicit zero-call gate. |

Total: 9/25 for this handoff, not future live execution. Verification: Node 22
frozen docs build/metadata/index/links/component tests, source comparison,
diff/doc/worktree checks and applicable PR CI. Local plan review found no material
scope/coverage gap; each of the three accepted functional requirements has an
observable oracle, denial cases and a bounded next action.


## Offline fixture implementation — 2026-11-08

The owner authorized #1564's direct-call fixture unit after this handoff merged.
The [evidence note](../../validation/parent-continuation-fixture.md) records the
synthetic peer/client, eighteen tests, independent transcript and code/config
hash pins. The existing docs test runner now executes the standalone Python suite
through docs:check/CI. No production module or native transport was introduced.
All three native capabilities remain unverified; the next step is the one-route
native packet review above, with live budget still zero until separately approved.
