---
type: Validation Record
title: Antigravity native terminal status qualification
description: Native question, cancellation, explicit rejection and automatic-command traces with retained coverage and cleanup limits.
tags: [orkworks, antigravity, attention, validation]
status: draft
---

# Antigravity native terminal status qualification

Supports [the approved design](../superpowers/specs/2026-11-24-antigravity-native-attention-design.md)
and [#1996](https://github.com/Rambolarsen/orkworks/issues/1996).
The owner accepted the limited 1.5.16 and 1.5.17 macOS arm64 profiles below for
source activation on 2026-11-26. The untested cases remain unqualified.

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


## Owner-accepted limited source activation, 2026-11-26

After the evidence was merged in [PR #2062](https://github.com/Rambolarsen/orkworks/pull/2062),
the owner directed continued work in direct response to the request to enable
native Needs You for macOS arm64 **1.5.16** with these documented limits. This
accepts narrowed empirical coverage for that exact source profile; it does not
claim the original full qualification gate passed. Literal Deny-button behavior,
automatic-command behavior on 1.5.16, human-visible dialog confirmation and
possible-descendant cleanup remain unverified as described above.

Source activation admits only built-in launches/exact resumes on macOS arm64
whose canonical executable returns the exact successful bounded version probe
`1.5.16`. Versions 1.5.15, 1.5.17 and prereleases, other platforms and custom
commands remain unqualified. The runtime forces OSC emission and brief status
messages for new qualified runtimes, without the experimental strict-permission
setting or any persistent configuration change. Existing sessions are not
mutated. This describes current source rather than a published installer or
an independent installed end-to-end OrkWorks UI test.


## Explicit rejection and automatic-command follow-up, 2026-11-26

The owner directed this follow-up after [activation PR #2068](https://github.com/Rambolarsen/orkworks/pull/2068).
It changes retained evidence, not source eligibility or permission ownership.
The installed executable had advanced to 1.5.17, which remains outside the
source profile. The official [1.5.16 macOS arm64 release](https://github.com/google-antigravity/antigravity-cli/releases/tag/1.5.16)
was downloaded into a disposable directory. Its archive matched the release
asset digest; its executable matched the earlier retained hash and reported
1.5.16. Both hashes are in the [derived follow-up record](antigravity-native-denial-automatic-2026-11-26.json).
The installed CLI was not downgraded or replaced.

Four interactive native starts occurred, each capped at 180 seconds and one prompt. The
first exited with code 1 before any prompt or status; its error text was not
retained and its cause remains unknown. The second reached a startup permission
wait, which was canceled without approval. That test copy changed bytes during
startup, so none of its frames qualify 1.5.16. A subsequent hash preflight
refused before another native start. The test artifact was then restored,
placed in its own read-only directory and launched with the process-only
`AGY_CLI_DISABLE_AUTO_UPDATE=true` override. The two successful trials retained
unchanged executable hashes. These measures pin this experiment, not production
launches or future artifacts.

The successful trials used the existing workspace that supported the earlier
capture, with disposable fixture files outside the checkout. Neither inspected
or edited repository files. The denial trial used the session-only strict tool
setting; the automatic trial used no mode or tool-permission override, retaining
the existing native configuration. No user settings, hook installations,
permission lists or persistent grants were changed. No inherited OrkWorks
reporting capability or model/proxy credential was forwarded. Normal native
login and global configuration still applied. Exactly one fixture prompt was
submitted in each successful trial; model-call accounting was not measured.

| Case | Retained native sequence | Independent observation and boundary |
| --- | --- | --- |
| Explicit command rejection | Working 21.913s → blocked/permission 31.979s → Working 52.530s → Idle 52.601s | Fixed native permission text and numbered Yes/No controls were observed. The operator sent the observed No option number; its selected marker appeared before the native wait cleared. No Escape or approval was sent for this resolution. This establishes the native rejection control, not a human-visible confirmation or retained structured tool-error receipt. |
| Automatic command | Working 18.281s → done 30.931s | Fixed shell-activity text appeared at 23.098s and the prepared disposable file contained the exact execution marker at 26.815s. No permission/question blocked frame occurred during this turn and no approval input was sent. Startup auth had already cleared to Idle at 1.511s. This is one automatic-command negative case under the existing configuration, not a guarantee for all commands or policies. |

The denial fixture initially used a relative output target, whereas its
collector checked a temporary path. Its absent marker is therefore not an
independent proof of nonexecution and is not used to qualify rejection. The
qualification relies on the observed No control/selection and native state
transition. The automatic fixture used an absolute temporary output target;
its exact marker is independent of echoed prompt text.

All successful frames had root `app=antigravity`, no child ID/title, the existing
app/state/kind/msg field shape and at most 73 payload bytes. Raw PTY, native IDs,
message/title bodies, prompts and answer text were omitted. Fixed generic
permission-control labels are retained only to establish the rejection action.
This remains an agent-operated terminal capture, not an installed OrkWorks UI
end-to-end test or a human-confirmed dialog test.

The denial collector attempted process-group teardown but checked root exit
before closing its PTY, then reported exit unconfirmed. A subsequent process
inventory found no process using this task's unique pinned executable; this is
not a retrospective child-reap or all-descendant proof. A dummy-only test then
established that closing the PTY before waiting lets a TERM-resistant child
exit. The automatic collector used that order, delivered the group signal and
confirmed root exit with code 0. Possible-descendant cleanup remains unproven.

Explicit native rejection and one independently witnessed automatic command
are now retained for this artifact. Human-visible dialog confirmation, installed
OrkWorks UI behavior, other versions/platforms, child aggregation and complete
descendant cleanup remain unqualified. The source table stays exactly 1.5.16;
this evidence does not enable 1.5.17, broaden production permission policy or
claim the original full qualification gate passed. No native test remains
running and no further native start is part of this evidence update.


## Installed 1.5.17 qualification, 2026-11-26

After the 1.5.16 [rejection/automatic evidence PR #2078](https://github.com/Rambolarsen/orkworks/pull/2078),
the owner directed qualification of the installed 1.5.17 artifact. A disposable,
read-only snapshot reported exactly 1.5.17, had no version-probe stderr, and
retained the same hash before and after both interactive trials. The installed
executable was not replaced. Both snapshots used the process-only
`AGY_CLI_DISABLE_AUTO_UPDATE=true`, forced OSC emission and brief status messages.
This pins the experimental artifact, not production launches or every binary
with the same version. The [derived 1.5.17 record](antigravity-native-1517-2026-11-26.json)
retains its hash, frames and fixed control/marker observations.

Exactly two interactive starts were used, each capped at 180 seconds. The
strict session submitted three fixture prompts and lasted 137.264 seconds;
the existing-policy automatic session submitted one and lasted 45.386 seconds.
No approval input or persistent permission grant was sent. The strict session
used `--mode default` and session-only `toolPermission: strict`; the automatic
session used no mode or tool-permission override. Both used the previously
working workspace, while their fixture scripts and absolute marker targets
were confined to disposable directories. No repository files, settings, hooks
or permission lists were edited. No inherited OrkWorks reporting capability
or model/proxy credential was forwarded; native saved login and configuration
still applied. Model-call accounting was not measured.

| Case | Retained native sequence | Operator observation/action |
| --- | --- | --- |
| Question answered | Working 17.878s → blocked/question 23.679s → Working 39.661s → done 42.394s | Submitted the default response with Enter after a retained native blocked hold. Question/answer text was not retained. |
| Command rejected | Working 61.063s → blocked/permission 67.497s → Idle 85.219s | Fixed native command-permission text and a numbered No control were observed. The operator selected that observed number; its selected marker appeared at 85.202s. No Escape or approval resolved this wait. The absolute command marker remained absent. |
| Question canceled | Working 96.013s → blocked/question 99.964s → Working 120.765s → done 127.680s | Sent Escape without submitting an answer. |
| Automatic command | Working 18.313s → done 34.843s | Fixed shell-activity text appeared at 28.111s and the exact independently inspected disposable marker at 31.112s. No permission/question block or approval input occurred during the command turn; startup auth had already cleared to Idle. This is one command under the existing policy, not universal automatic-command behavior. |

The generic control extractor may see repeated/stale labels during terminal
redraws; the rejection finding uses the observed No selection immediately
followed by the recognized native Idle transition. It does not infer permission
resolution from a control label, keypress or silence alone. The marker in the
automatic trial uses an absolute disposable target, so prompt echo cannot prove
its success. The strict fixture used the same absolute-target arrangement;
its absent marker is an additional nonexecution observation for that fixture.

All recorded frames use root `app=antigravity`, the existing app/state/kind/msg
shape, no child ID/title and payloads within the current 256-byte parser bound.
Message/title bodies, raw terminal output, prompts/answers and native IDs are
omitted. Only generic fixed permission-control labels and marker-result booleans
are retained beyond the restricted native status fields. Both collectors
closed their PTYs before waiting, confirmed root exit code 0 and retained
unchanged binary hashes. No process using the unique task executable remained
in the follow-up inventory; complete descendant cleanup is not proven.

These are agent-operated native terminal trials, not human-visible confirmation
or installed OrkWorks UI end-to-end qualification. Windows, child aggregation,
terminal negotiation, errors/unknown states and other artifacts remain outside
this evidence. The source version table remains 1.5.16 pending explicit owner
acceptance of the narrower 1.5.17 coverage; the original full qualification gate
is not claimed passed. No additional native start is part of this update.


## Owner-accepted 1.5.17 source activation, 2026-11-26

After the 1.5.17 evidence merged in [PR #2081](https://github.com/Rambolarsen/orkworks/pull/2081),
the owner directed continued implementation in direct response to the concrete
request to enable exactly 1.5.17 on macOS arm64 alongside 1.5.16 with the stated
limits. This accepts narrowed empirical coverage for that additional source
profile; it does not retroactively establish human-visible/installed OrkWorks UI
behavior or complete descendant cleanup. Those cases remain unqualified.

The existing source table now admits exactly 1.5.16 and 1.5.17. Built-in launch,
exact resume/initial-prompt, platform and successful bounded executable-version
checks remain in force. Fresh qualified runtimes force native OSC emission and
brief messages; existing sessions are not changed. The diagnostic strict tool
setting and process-only update suppression are not applied in production.
Other versions/platforms and custom commands retain prior hook fallback.
Explicit attention ownership, generation fencing, live-only ingestion and Peon
preservation remain unchanged. This describes source behavior, not a published
installer or installed UI test. No further native start is authorized by this
activation approval, and none is needed for this implementation.
