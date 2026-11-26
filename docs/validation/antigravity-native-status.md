---
type: Validation Record
title: Antigravity native terminal status qualification
description: Retained native question-answer and cancellation traces; literal denial and automatic-command coverage remain unqualified.
tags: [orkworks, antigravity, attention, validation]
status: draft
---

# Antigravity native terminal status qualification

Supports [the approved design](../superpowers/specs/2026-11-24-antigravity-native-attention-design.md)
and [#1996](https://github.com/Rambolarsen/orkworks/issues/1996).
No source profile is qualified or activated by this record.

## Owner-authorized first run, 2026-11-24

The owner authorized exactly one disposable interactive Antigravity probe,
capped at 180 seconds and three fixture prompts. The installed native binary
reported version 1.5.15 on macOS arm64. A temporary PTY relay retained no raw
native output. It requested native OSC emission and brief status messages;
it forwarded no OrkWorks reporting capabilities or inherited model/proxy
credentials. Native saved login and global configuration still applied.

The initial command wrapper refused before launching a native process because
stdout was not a TTY. A dummy-only check established that behavior, then a
PTY relay passed an offline TTY/output-omission check before the native start.
Exactly one actual native start followed. The owner-authorized run ended after
about 37 seconds rather than consuming its full time budget.

Retained [derived status data](antigravity-native-status-2026-11-24.json):

| Elapsed seconds | State | App | Kind |
| --- | --- | --- | --- |
| 0.209 | blocked | antigravity | auth |
| 1.442 | blocked | antigravity | permission |
| 36.125 | idle | antigravity | absent |

All three frames had no nonempty child-record ID. The blocked frames carried
message fields, whose values were neither decoded nor retained. Root native
process exit was confirmed. Raw PTY bytes, native IDs, account information,
message/title bodies and user answers were not retained.

The first question fixture was sent while a startup wait was outstanding.
The relay never observed its fixed Alpha/Beta marker in native output. No
question lifecycle or ordinary command approval was established. Escape was
sent without an approval, and the native process returned idle and exited.
The exact startup dialog type beyond its native kind remains unverified;
this record must not identify it as a workspace-trust prompt or a successful
tool-permission test. Native model-call usage was not measured.

## First-run disposition

This establishes emitted `app=antigravity`, root blocked auth/permission and
idle-after-cancellation shape for one artifact. It does not establish question,
working/done, approved/denied command, negative automatic execution, child
aggregation, terminal negotiation, Windows or version-wide behavior.
The original three-minute/one-start allowance is consumed; unused time does
not authorize another start. A separately authorized second start is recorded
below; this first-run record alone granted no further execution.

The capture has since been extended to retain payload byte counts and safe
field names, so a later authorized run can set the parser bound/allowlist from
evidence. Do not retroactively add those missing measurements to this trace.

## Separately authorized second run

The owner approved one further 180-second probe from the existing checkout,
again limited to three fixture prompts and no persistent permission grants or
hook changes. It used the same native 1.5.15 artifact. The native process exited
with cleanup confirmation at the deadline. All thirteen derived frames and
safe field names/byte counts are retained in the
[second derived trace](antigravity-native-status-second-2026-11-24.json).

Startup moved from blocked/auth to idle. After the first fixture was submitted,
the native sequence was working (41.069s), blocked/question (45.836s), working
after selecting the default fixture answer (65.647s), then done (68.932s).
The second fixture requested a harmless printf command and produced working
(88.068s) then done (96.118s), with no blocked frame. Its actual tool result was
not retained, so this is a no-blocked native-turn observation rather than
independent proof of command execution. The third fixture requested an explicit
permission for that same harmless command; it also returned done without a
manual permission dialog. No approval or persistent grant was sent.

The final fixture was adjusted within the approved three-prompt scope to request
permission instead of a second question. Accordingly, question cancellation and
manual tool-permission denial remain unqualified. Model-call accounting was
not measured; exactly three fixture prompts were submitted. Terminal output
was not retained. The relay's marker-presence observation includes echoed input
and must not by itself be cited as proof of a native dialog or tool execution.

All observed frames used app/state, with kind/msg on blocked states and msg on
done; no id/title field was observed. Payloads were 31–73 bytes. The source
consumer can retain Codex's 256-byte bound, accept native msg only to discard
it, and reject all child/id and unobserved fields. Forced emission is proven
for this artifact; terminal negotiation, ordinary permission denial, errors,
clear/unknown, children, other versions/platforms and all-client behavior remain
limits. No full qualification row is enabled by this partial record.


## Separately authorized 1.5.16 trials, 2026-11-26

The owner authorized one 180-second native session with at most three prompts.
That attempt exited in under a second, before any prompt was sent or any OSC
status was retained. Its root process exit was confirmed. The collector did not
retain the child exit code or error classification at that point; the cause is
unknown. Preserve this [failed-start record](antigravity-native-startup-failure-2026-11-26.json)
as an unqualified attempt. Later version/help checks succeeded; they do not
prove why interactive startup failed. A dummy-only collector regression then
verified exit-code retention, fixed error labels and private error-text omission.

The owner separately approved one retry with the same 180-second/three-prompt
limits. The bounded preflight reported CLI **1.5.16** on macOS arm64 and pinned
the executable bytes before launch. The retained hash matches the earlier
record's hash; the reported version is a separate observation and must not be
used to claim a different binary hash. The probe used `--mode default` plus
`--settings '{"toolPermission":"strict"}'` for this session only. This stricter
trial does not replace existing allow/ask/deny lists or establish behavior under
unmodified default permission policy. The installed CLI help identifies
`--settings` as session-only; the [official settings reference](https://antigravity.google/docs/settings?tab=cli)
and [settings schema](https://antigravity.google/schemas/v1/settings.json)
document strict tool confirmation. The capture edited no global settings or
installed hooks and forwarded no OrkWorks reporting capability or inherited
model/proxy credentials. Normal native login/configuration still applied.

The retry was agent-operated through the redacting PTY relay. Three fixture
prompts were submitted. No command execution or persistent permission was
approved. Raw terminal output, message/title bodies, prompt/answer text and
native IDs were not retained. Native model-call accounting was not measured.
The fourteen frames are retained in the [1.5.16 derived trace](antigravity-native-status-2026-11-26.json).

| Case | Retained native sequence | Operator action and limit |
| --- | --- | --- |
| Structured question answered | Working 26.440s → blocked/question 30.958s → Working 48.881s → done 51.831s | Submitted the default answer after the wait. This is not a retained human-visible dialog confirmation. |
| Command permission canceled | Working 68.814s → blocked/permission 73.181s → Idle 121.028s | Sent Escape without approval. The relay also detected fixed native permission-prompt text. This proves cancellation of a pending permission, not selection of a literal Deny button or a tool-denial response. |
| Structured question canceled | Working 135.025s → blocked/question 138.807s → Working 155.584s → done 158.299s | Sent Escape after the question wait. [The CLI interaction reference](https://antigravity.google/docs/cli/prompting) documents Escape as its cancellation control. No answer was submitted for this question. |

Every frame had `app=antigravity`, no child ID or title, and a payload of at most
73 bytes. The known app/state/kind/msg shape remains within the source parser's
256-byte bound. Message bodies were discarded without decoding. This qualifies
the retained field shape and cancellation transitions for this trial; it does
not qualify child aggregation, unknown/error states, terminal negotiation,
Windows or other versions. Permission cancellation returned Idle, while question
cancellation returned Working then done; the consumer must follow native state,
rather than infer the outcome from a keypress or silence.

At the deadline, the collector reported a denied process-group cleanup signal,
then confirmed the root child's exit with code 0 and the relay's exit. This is
root-exit evidence, not proof that every possible descendant exited. No process
was targeted by a reconstructed PID or name after reaping.

Literal Deny-button behavior, an independently verified automatically executed
command on 1.5.16, and human-visible dialog confirmation remain unqualified.
The strict trial cannot establish the automatic-command negative case. No source
profile is enabled by this evidence update. Enabling the exact 1.5.16 profile
with these documented limits requires explicit owner acceptance; otherwise the
original qualification gate remains in force. Both allowances are consumed;
no further native session is authorized by unused time or this record.
