---
type: Design Proposal
title: Antigravity attention from native terminal status
description: Qualify Antigravity OSC 7501 status and reuse the live terminal attention path without Peon prompt inference.
tags: [orkworks, antigravity, attention, qualification]
status: stable
workflow_status: accepted
---

# Antigravity attention from native terminal status

Research and proposed implementation boundary for [#1996](https://github.com/Rambolarsen/orkworks/issues/1996).
The owner requested Needs You for permissions and questions, rejected Peon for
this purpose, and requested research into native signals. The owner approved this concrete design on 2026-11-24 by directing continued
implementation after review of the published proposal. Native activation
remains subject to retained live qualification; design approval is not that
evidence.

## Outcome and proposed decision

Observe Antigravity's own terminal program status in the existing live PTY
reader. Keep hooks for native session identity and legacy Working/Idle reports.
Do not install decision-bearing tool hooks or introduce an ACP client.
Use native blocked permission/question/auth states for Needs You, Working for
working, Idle for idle/done, and Failed for error. A structured native question
can qualify; an ordinary prose question without a native wait cannot be
reliably recognized under this design and remains unsupported.

For Antigravity, Peon may continue summaries and other descriptive metadata,
but preserves the entire attention tuple, including before native status
activation. Do not use a nonprompt merge policy that would silently reconcile
an existing Peon wait. Existing Peon-owned waits are not bulk-cleared at rollout;
only an ordinary authorized hook/native/input/lifecycle transition may change
them. Retain existing hook Working/Idle fallback for non-emitting runs.
This changes the currently documented Peon prompt fallback and requires an
approved MVP/contract amendment before dependent runtime implementation.

## Evidence established on 2026-11-24

- [Official CLI 1.4.0 release notes](https://github.com/google-antigravity/antigravity-cli/blob/main/CHANGELOG.md#140)
  document OSC 7501 status for working, input wait, completion and failure;
  `AGY_CLI_OSC7501=on` forces emission, and
  `AGY_CLI_OSC7501_MESSAGES=brief` omits detailed pending command/question text.
- A bounded `agy --version` inspection returned **1.5.15** on macOS arm64.
  The installed binary contains program-status reporting functions and the
  same embedded release note. Binary strings establish presence, not behavior.
- The [hook reference](https://antigravity.google/docs/hooks.md) exposes six
  CLI events, with no documented prompt-displayed notification. Native
  `toolCall.id` correlation was added in 1.5.4, but before/after tool dispatch
  does not by itself prove an ordinary permission dialog is awaiting a user.
- T3 Code's [Antigravity protocol adapter](https://github.com/pingdotgg/t3code/blob/main/apps/server/src/provider/acp/AntigravityProtocol.ts)
  uses ACP requests, including `interaction_` permission-method IDs for
  questions; it does not qualify an observer of an existing CLI session.
- OrkWorks already has a bounded [Codex terminal reader](../../../crates/orkworksd/src/runtime/terminal_status.rs)
  and generation-fenced attention application. Both select Codex explicitly;
  they do not accept Antigravity frames. See the
  [accepted Codex design](2026-10-25-codex-automatic-approval-signal-design.md).
- Read-only inspection found 12 saved Antigravity records for OrkWorks and
  11 retained terminal files. After decoding 9,081 stored records, one support
  probe and zero status reports were found. This is consistent with missing
  support negotiation, not proof of that cause or complete historical coverage.

A later owner-authorized run retained a partial startup-only auth/permission
wait and idle-after-cancellation sequence; see the [qualification record](../../validation/antigravity-native-status.md).
No ordinary tool-permission/question lifecycle is qualified. The first
human-started capture produced no evidence file. A capture exit-path defect
was independently reproduced with a dummy process: group cleanup denial
raised before evidence publication. The revised temporary fixture checkpoints
before cleanup and passed dummy normal-exit, timeout and simulated-denial
checks. Those checks are not Antigravity qualification. Temporary fixture
paths are not durable qualification evidence or distributable product assets.

## Qualification gate

Before activation, retain a sanitized sequence from one human-operated,
three-minute disposable interactive run of the installed artifact. Use at most
three fixture prompts: a structured question answered after a visible hold,
a harmless `printf ORKWORKS_STATUS_PROBE` command denied after a visible
permission hold, and a second structured question canceled through native UI.
If the command is already allowed and runs without a dialog, record that
negative case; do not change permission policy or call it a permission test.
An additional permission trial requires its own bounded scope. Do not use
headless print mode as evidence of an interactive wait.

The capture uses forced OSC emission and brief messages, never sends prompts
or answers itself, and forwards no inherited OrkWorks report capability or
model/proxy credential. Native saved login and global CLI configuration still
apply. Retain only binary hash, platform/version, elapsed event order, state,
restricted app/kind and root/child presence; do not retain message/title bodies,
raw PTY output, commands, answers or native session IDs. The human must confirm
which dialog was visible and that answer/denial/cancellation was accepted.
An unconfirmed or failed cleanup must be recorded, not treated as success.

Qualification must establish:

1. Exact emitted app value, recognized fields, root-record convention and
   ST/BEL framing; do not guess these from another implementation.
2. Visible question/permission holds produce recognized blocked state, followed
   by a recognized state when answered, denied or canceled. Silence is not clear.
3. A command that proceeds without human approval does not introduce Needs You.
   If this case is absent from the bounded run, keep that criterion open.
4. Root/child display semantics and unsupported waits are explicit. Do not
   promote a child frame into a root request or claim all-child aggregation.
5. Forced emission is qualified separately from support-query negotiation.
   This proposal uses source-owned launch environment for the initial profile;
   adding terminal negotiation would require separate evidence and scope.

The source qualification table starts empty. The first candidate is the exact
observed macOS arm64 1.5.15 artifact; documentation alone does not admit it.
Windows, remote sessions, other versions and custom commands remain unqualified.

Implementer proposal: the binary hash identifies the retained experiment; it is
not a per-launch executable-integrity gate. Initial runtime eligibility uses an
explicit source-owned platform/exact-version profile and the built-in launch
route, with a bounded probe of the selected executable. A same-version binary
replacement is not detected by this policy, and PTY frames remain spoofable.
Do not add artifact hashing, code-signature verification or an executable trust
system as an implicit dependency of best-effort status presentation.
Do not launch another coding harness from this active session without an
explicit owner exception; a human-started diagnostic is the supported path.

## Attention and parser contract

Reuse the streaming parser structure with an explicit source profile rather
than accepting every application's frames. Preserve Codex's existing accepted
fields, 256-byte limit and behavior. The Antigravity profile may allow up to
4 KiB only if captured frames require it; bound before parsing, reject duplicate
or unsupported fields, and omit title/message values from application state.
Optional known text fields can be skipped without decoding their content.
Reject foreign apps, malformed control strings and non-root records. Exact
app/field/root matching remains a qualification deliverable.

The first recognized frame activates terminal status only for the current live
session generation. Before that, retain hook Working/Idle behavior without
Peon prompt inference. Once activated, ignore later Antigravity hook attention,
including delayed reports; continue authenticated hook identity reporting.
Only live PTY bytes enter this reader. Persisted output replay never activates
or updates attention. A replacement runtime starts inactive.

Use the existing attention transaction and ownership checks. Proposed provenance
is `backend_inference` / `antigravity_terminal`. Native terminal evidence may
replace Antigravity hook attention and inference, but never explicit user or
separate agent attention. Revalidate both live and persisted ownership under
the existing locks; do not bypass the normal source ladder for other writes.

Clear or a recognized unsupported native state withdraws only this terminal
source's tuple. Unknown/clear before activation has no effect. Activation stays
sticky for that runtime, matching Codex's behavior; silence does not re-enable
competing hook attention. Typing alone does not clear a wait. Session end uses
the existing lifecycle path. Do not couple this source to native approval
ownership, request IDs, orchestration authority or hook trust.

Terminal output is spoofable by programs writing to the PTY. Treat it as
best-effort display evidence, never permission authority. A missed frame can
leave stale attention, and displayed native thread state may not aggregate all
children. These limits need owner acceptance; no authenticated pending-request
ledger is claimed.

## Deliverables after approval and qualification

1. Amend the relevant MVP attention contract and canonical harness documentation;
   record the additive terminal-source decision through the ADR lifecycle.
2. Generalize the parser and runtime terminal-source selection, preserving
   Codex regression fixtures; qualify the Antigravity profile and source-owned
   forced-emission/brief-message launch environment without forwarding it to
   other harnesses. No user settings or installed hook files are rewritten.
3. Generalize terminal attention application and runtime flags just enough for
   these two profiles. Update hook rejection, Peon policy, provenance/domain
   documentation and lifecycle fences together.
4. Test actual captured traces, every chunk boundary, duplicate/foreign/child
   frames, text omission, bounds/recovery, missing emission, cancellation,
   late hook reports, explicit user/agent ownership, unknown/clear, persistence
   failure, restart and output replay. Keep unsupported profiles on the stated
   hook fallback with no Peon-created prompt state.
5. Run scoped Rust checks, full repository verification, documentation checks
   and current-diff code review before the implementation PR can merge.

This is a design-only increment. No source qualification row, launch setting,
parser behavior, live session, hook installation or attention authority changes
as a result of publishing this draft. Rollback of a later implementation is a
source revert and relaunch; never rewrite an existing user's attention to hide
a missing or unsupported native signal.

## Open decisions for owner review

- Accept native terminal status as best-effort UI evidence with the explicit
  spoofed/missed-frame and displayed-thread limitations above.
- Accept structured-question coverage only: prose questions without native wait
  are unsupported, and Peon is excluded from prompt inference.
- Accept forced emission in source-owned qualified launches; existing sessions
  are not mutated to turn it on. Keep other profiles visibly limited.

## Builder check disposition

A fresh read-only builder check identified two material ambiguities. The
implementer settled them above: artifact hashes are qualification evidence,
with exact platform/version and built-in-route admission rather than per-launch
hash enforcement; existing Peon attention is preserved until an ordinary
authorized transition, with no rollout migration or nonprompt reconciliation.
These implementer choices were included in the published design the owner
approved for implementation.
No other unanswered builder questions were reported in the one-round review.
Implementation approval is recorded above. Retained live qualification remains
outstanding; no activation is authorized by the design approval alone.


## Later qualification evidence

The [qualification record](../../validation/antigravity-native-status.md) now
retains an agent-operated 1.5.16/macOS arm64 strict-session trial from 2026-11-26.
It establishes native question answering and cancellation, plus permission
cancellation without approval. Literal Deny-button behavior, automatic-command
behavior on that version and human-visible dialog confirmation remain
unqualified. Root exit was confirmed, but a denied group signal leaves
possible-descendant cleanup unproven. No production profile is enabled by this
record; the original gate or explicit owner acceptance of narrowed coverage
still governs activation. This evidence pointer does not amend the gate.


## Owner-accepted activation amendment, 2026-11-26

The owner directed continued work in direct response to the request to enable
macOS arm64 1.5.16 with the [retained evidence and documented limits](../../validation/antigravity-native-status.md#owner-accepted-limited-source-activation-2026-11-26).
This is explicit acceptance of narrowed coverage for that exact source profile,
not evidence that the original human-operated, literal-denial or automatic-
command criteria passed. Source activation may admit 1.5.16 on macOS arm64 using
the existing built-in-route and bounded exact-version checks. The source must
not apply the diagnostic strict-permission setting to production launches.
Other versions, platforms and custom routes remain gated. Existing source
priority, live-generation, no-replay and non-permission-authority contracts are
unchanged. No additional native probe is authorized by this amendment.


## Later explicit-rejection and automatic-command evidence

The [2026-11-26 follow-up](../../validation/antigravity-native-status.md#explicit-rejection-and-automatic-command-follow-up-2026-11-26)
retains agent-operated native No selection followed by Working/Idle, and one
independently witnessed automatic command with Working/done and no prompt block.
Human-visible confirmation, installed OrkWorks UI behavior and complete
cleanup remain unqualified. This improves retained evidence for the existing
1.5.16 profile without changing the approved architecture, permission policy or
other-version gates. Historical qualifications and failed attempts above remain
limited to their actual observations.
