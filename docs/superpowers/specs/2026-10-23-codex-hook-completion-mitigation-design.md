---
type: Design
title: Codex permission waits resolved by exact tool completion
description: Define conservative completion-based resolution without enabling the gated native Codex runtime.
tags: [orkworks, codex, attention, hooks, metadata]
status: draft
---

# Codex permission waits resolved by exact tool completion

Status: **owner-approved for implementation on 2026-10-23; rollout qualification remains gated**.
Tracks [#690](https://github.com/Rambolarsen/orkworks/issues/690), with the
coordinated-producer prerequisite in
[#789](https://github.com/Rambolarsen/orkworks/issues/789).
The owner requested this smaller mitigation after the native route's blockers
were reproduced on 2026-10-23, then approved implementation with “go ahead” after
PR #992 stored the proposal. This authorizes scoped implementation, not removal
of the rollout evidence gates below.

## Outcome and limits

A Codex tool can complete after automatic permission review while OrkWorks
continues showing Needs You. For an eligible, unambiguous serial invocation,
change its owned permission wait to working when that exact tool completes.
A held manual permission must remain Needs You until native approval, accepted
input, or another already-authorized transition resolves it.

This is a partial mitigation. PermissionRequest continues to set Needs You
immediately, including for automatic review. A long automatically approved tool
may show false Needs You throughout execution. Completion supplies no proof of
when approval happened and does not classify the reviewer. There is no grace
timer, inactivity timeout, terminal-text classifier, or generic PostToolUse clear.
Ambiguous or missing evidence keeps today's conservative wait until an existing
trusted transition. This proposal does not close #690 or address queued questions
in [#632](https://github.com/Rambolarsen/orkworks/issues/632).

No app-server, remote TUI, native observer, new approval controller, or metadata
mailbox transport is introduced. The accepted native design and its listener,
configuration, and signal gates remain unchanged; VERIFIED stays empty.

## Evidence and current blockers

[ADR 0051](../../adr/0051-codex-deterministic-attention-hooks.md) records the
passed serial manual/automatic payload gate: PreToolUse and PostToolUse have
matching tool_use_id; PermissionRequest has no invocation ID. Turn ID and tool
name alone are not unique. Serial captures do not qualify parallel or subagent
correlation, reliable delivery, or a distributed installer.

The existing native reducer/application and Codex report relay provide bounded
correlation and source fencing patterns, but their native baseline and root
checks cannot simply be removed. This proposal defines a separately qualified
completion path; it does not enable the dormant native implementation.

On 2026-10-23, this live OrkWorks-launched Codex agent ran the installed
session-metadata.py bootstrap helper from its ordinary restricted tool context.
The installed helper matched repository bytes. It failed before reaching the
API; a credential-free connection probe returned PermissionError/EPERM for
IPv4 localhost. Session-record bytes were unchanged. The existing 14 helper
tests passed when fixture loopback access was separately allowed, after their
restricted-context listener binds failed. Those elevated fixture tests do not
qualify the actual agent sandbox. No global configuration or hook trust changed.

The latest-release API returned Codex rust-v0.164.0. Its audited
[remote connector](https://github.com/openai/codex/blob/6aaac694803004a63c6f393d153c96d22521a6b9/codex-rs/app-server-client/src/remote.rs)
still inserts Authorization before URL-based connect_async_with_config, with no
expected owned-server instance parameter. This source observation does not
qualify an installed binary and does not remove the independent native gate.

**Consequence:** this proposal removes the native-client dependency for the
completion mitigation, but does not remove the coordinated-writer requirement.
The current restricted sandbox is not eligible. An already-supported execution
context with verified API reachability may qualify; do not weaken a sandbox or
invent a filesystem metadata transport to obtain that result. Supporting a
sandbox with no metadata API access needs a separately designed transport.

## Eligibility and compatibility

Propose a separate source-controlled completion capability and exact
version/platform/configuration qualification table for the built-in Codex
adapter. Custom wrappers and user overrides cannot add qualification. The table
starts empty. A row requires all of the following:

- Installed, trusted current hook bundle; exact accepted native identity and
  live generation; authenticated hook reports with allowlisted bounded scalars.
- Qualified hook ordering and invocation identity for that binary and effective
  configuration. The initial scope excludes internal subagents: qualify a
  supported configuration that demonstrably prevents them and whose effective
  state can be checked. Do not infer absence of children from a quiet stream,
  a parent session ID, or a single turn ID. If that configuration cannot be
  established, no row may ship. No new upstream flag is assumed here.
- All supported agent metadata producers migrated to sidecar-v1 and qualified
  under the same actual execution context, including its effective sandbox.
  The closed source-owned capability alone is insufficient.

The explicit specification change on approval is to permit the existing API-only
metadata marker for a qualified **non-native completion** launch as well as a
qualified native launch. Static completion eligibility sets the marker before
agent startup, requiring API-only writes immediately. A failed or pending
child-context authenticated GET/validation-only PATCH/follow-up GET keeps writes
API-only and completion clearing disabled. Instructions/helpers perform the
handshake before each mutation; direct JSON fallback remains forbidden.

There is no live-session upgrade: an existing session retains its launch
contract. Disabling the completion feature stops future clears but does not
restore direct-file writes to an API-only running session. A new launch may
choose the unchanged legacy contract. Native eligibility and native clear
remain separate and disabled. This non-native marker extension was owner-approved with this design on
2026-10-23; it was not authorized by the earlier native-only design.

## Correlation and transition contract

Use a separate runtime-local completion tracker in the session runtime. Bound
records and completed-ID tombstones to 64 per accepted root turn. Overflow,
missing required scalars, equal/reversed event ordering, corrupt relay records,
unknown completion IDs, uncertain delivery, or overlapping open invocations lock
completion clearing for that turn. Never evict a tombstone to recover authority.
Late duplicates cannot reopen or clear a later wait. Transport ordering must be
qualified against the real reporter; sorting UUID filenames is not event order.
An undetectable delivery gap is a qualification failure, not permission to clear.

A clean baseline requires an accepted UserPromptSubmit in this live generation,
the accepted root identity, working attention without prompt fields, no retained
wait, successful child-context metadata qualification, and qualified single-root
execution. SessionStart, resume, idle output, or a new turn_id alone is not that
baseline. Resume into an existing prompt and startup without the boundary stay
conservative. Root-identity replacement, committed input, Stop, hook revocation,
runtime end, workspace change, or a competing attention write invalidate old
correlation; only a later qualified prompt submission can establish a new baseline.

| Event | Proposed effect |
| --- | --- |
| PreToolUse with a new nonempty invocation ID in a clean serial turn | Record the sole open invocation and its root, turn, tool and ordering fences. |
| PermissionRequest with the same root/turn/tool and exactly one open invocation | Apply today's immediate wait; associate it with that invocation only while the serial/order gates hold. |
| PermissionRequest without that chain, or a second outstanding request | Preserve today's immediate wait and lock completion clearing until a trusted boundary. |
| Exact PostToolUse for the candidate after its PermissionRequest | Retire that invocation; conditionally change only its still-owned wait to working. Completion is not an idle signal. |
| Generic, duplicate, missing-ID, reordered or overlapping completion | No clear; retain conservative wait. |
| Stop, UserPromptSubmit, accepted committed input, reset/revocation or lifecycle end | Keep the existing authorized transition; invalidate old completion effects. |

Denied, cancelled, or failed tools do not synthesize a successful completion.
If Codex supplies no qualifying PostToolUse, the wait remains until an existing
trusted transition. No elapsed-time rule reconstructs the missing event.
A new question or another permission always invalidates this candidate's ability
to clear. Unknown child/root attribution never regains authority mid-turn.

## Attention ownership and persistence

Keep one canonical session JSON and reuse the sidecar transaction boundary from
the [approved mediated metadata design](2026-10-08-codex-native-attention-layer-design.md).
An accepted candidate wait stores the runtime attention ownership token and the
store attention-write token, alongside generation, identity, turn, input and
hook-order fences. Every accepted competing attention write, even an identical
one, revokes ownership. Descriptive work updates do not revoke it.

Under the same transaction that performs the final read/check/replace, require
both tokens, current fences and hook authority, agent-tier harness_hook origin,
and the exact complete permission tuple still owned by this candidate. Reject
user overrides and explicit questions/options. A failed check or persistence
failure leaves attention unchanged and publishes no clear. The accepted clear
writes the complete working tuple, clears prompt fields, and retains
agent-tier harness_hook provenance. It does not borrow native_codex ownership
or bypass COORDINATED_METADATA_WRITERS for the native path.

All supported producers must share this boundary. Arbitrary file replacement
cannot be made atomic by checking file identity before rename. A launch still
supporting direct agent JSON writes is ineligible; adopting a weaker ownership
guarantee merely to ship this mitigation is outside this proposal.

## Implementation sequence after approval

1. Amend the MVP's native-only marker eligibility and ADR 0051's proposed
   completion rule, explicitly preserving the accepted native design. Finish
   #789 producer/actual-context qualification for the intended non-native row.
   Record an empty row rather than inventing missing platform or sandbox evidence.
2. Implement the independently gated pure completion tracker and conditional
   application through existing serialized attention writes. Reuse bounded
   validation patterns without refactoring the dormant native reducer.
3. Route qualified hook correlation through the existing private hook relay,
   preserving identity-only reports and ordinary attention behavior on legacy
   launches. Qualify actual ordering, loss behavior, bounded retries and reporter
   delivery on each supported platform. Reporter changes require normal Settings
   reconciliation and native /hooks trust; OrkWorks never grants trust itself.
4. Qualify one exact installed version/platform/configuration end to end before
   adding a source-owned completion row. Unsupported launches retain existing
   behavior. Document the remaining long-tool false wait and keep #690 open.

## Acceptance evidence

- Serial automatic and held manual permission sequences: permission wait is
  immediate; only the matched post clears it to working. A long tool remains
  Needs You until completion, demonstrating the stated limitation.
- Same-tool repeat calls, duplicate/late posts, missing pre/post, equal/reversed
  ordering, relay retry/overflow/corruption/loss and 64-record exhaustion cannot
  clear an unrelated prompt. Test delivery gaps as well as reducer inputs.
- Startup/resume into a held prompt, root changes, subagent-capable configurations,
  overlapping requests and queued questions remain conservative. Real installed
  fixtures must prove the effective single-root configuration before qualification.
- Real-router races exercise both commit orders for an identical agent attention
  write and conditional completion clear; user/question writes win their proper
  order. Summary/work updates remain intact. Inject write/read failures.
- The actual agent-context handshake makes no metadata change, fails closed on
  blocked localhost, and cannot be replaced by a host-hook or elevated probe.
  Source capability, marker, instructions and every producer path agree.
- Detach/reconnect, session death, cancellation, workspace switch and feature
  disable cannot replay an old clear. Legacy/direct-file and unsupported
  version/configuration rows never enter the completion path.

Future code uses TDD, full verify-repo and scoped sidecar checks plus the required
current-diff code review before merge. The design approval does not qualify a session or permit resuming, editing,
approving, or reconfiguring another session.

## Design review

The independent builder check found nothing material left unanswered. A separate
contract review found no substantive conflict with the MVP or accepted native
and metadata designs. Both reviewed the proposed document, not runtime code.
No shipping qualification or implementation approval is inferred from review.

The owner approved this bounded mitigation and its non-native metadata
eligibility extension on 2026-10-23. The blocked sandbox, real transport ordering/loss,
effective single-root configuration and installed qualification remain explicit
release prerequisites. If any cannot be proved, retain the empty qualification
table rather than shipping an unsafe clear.
