---
type: Validation Record
title: Antigravity native terminal status qualification
description: Retained startup and question-answer status evidence; manual denial and question cancellation remain unqualified.
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
