---
type: Source Compatibility Audit
title: Native history assumptions and fixed check reporting
description: Offline comparison of pinned path/event bridges and schema with the observer, plus bounded history-check diagnostics.
tags: [copilot, orchestration, qualification, history]
status: stable
workflow_status: offline-reporting-corrected-not-authorized
---

# Native history assumptions and fixed check reporting

[#1619](https://github.com/Rambolarsen/orkworks/issues/1619) follows the
[UUID-corrected attempt](parent-native-canary.md#uuid-corrected-packet-attempt--2026-11-09).
Its native-history/check-failed result and exit zero do not identify the failed
condition. This audit found no proven static path/event incompatibility. It fixes
the concrete reporting gap without loosening any observer check.

## Static findings and limits

[Audit data](native-history-source-audit.json) pins 1.0.90 app.js to
f84061b62eeb1cd78a655555890fe1a776509203fca68144745467e26e443813,
session-events.schema.json to
c1b43f27c015b87bbb74072c6591227e860d6b1862aed9edecfe5aacd29a16c3,
and runtime.node to
ad89ad135478cd2b19a5b99bb19064d0be281961cab0506f3767f39234d4c591.
Only static bytes were read; vendor JavaScript/addons were not imported/executed.
Zero-based UTF-8 app.js byte anchors:

| Anchor | Byte offset |
| --- | --- |
| settingsPrecedence | 139454 |
| sessionsHomeBridge | 139191 |
| sessionStateBridge | 139250 |
| eventsPathBridge | 139317 |
| nativeEmissionBridge | 1502660 |
| nativeEventStream | 1369900 |
| nativeResumeMetadata | 1611318 |
| nativeHistoryLoad | 1616511 |

WW passes configDir, COPILOT_HOME and OS home into native path resolvers. Tf, Gp
and Eg delegate sessions home, session state and event paths. JavaScript names
session-state and binary strings mention events.jsonl, but these do not prove
the native resolver's concrete result or exclusive directory layout. The
observer's owned settings/session-state/UUID/events.jsonl expectation remains
a runtime assumption. No personal directory or native history was enumerated.

Ordinary local emitInternal calls sessionEmitInternalJson; event streaming,
filesystem loading and resume metadata also cross native bridges. Some MCP field
names are absent as JavaScript literals but present in the pinned schema. Their
absence from app.js is not evidence that native events omit them.

The selected schema definitions contain start identity, resume eventCount and
tool model, turnId, MCP names/tool/transport, completion toolCallId and success.
Its transport enum supports stdio. Several fields required by the stronger
observer correlation oracle are schema-optional; presence does not prove a run
populates them. No path, parser, field requirement, alias, model, transport,
permission or capture setting was changed.

This review covers the relevant path precedence/delegation, selected start/resume
and tool metadata, History layout/prefix/tail/event checks and their reporting.
It cannot recover a deleted historical branch or establish compiled runtime
behavior. Actual failure cause and all three native capabilities remain
unverified. Independent owned synthetic failures demonstrate the reporting gap;
they do not reproduce the native failure.

## Minimal reporting correction

History.scan annotates caught ValueError/OSError with the current fixed check
identifier and rethrows the same exception. failure_details emits optional
checkCode only for native-history and one exact allowlisted string. Existing
exception types/categories, numeric OS errors, guards, ordering, partial progress,
process supervision and cleanup are unchanged. Exception text, paths, event
names and payloads are never copied into this field.

| Code | Failed check or operation |
| --- | --- |
| history-root-missing | Final scan requires the history root. |
| history-root-layout | Root symlink guard or directory enumeration. |
| history-session-layout | Exact session entry and its symlink guard. |
| history-events-missing | Final scan requires the expected event file. |
| history-events-read | Bounded regular-file read and source guards. |
| history-prefix | History length and prefix commitment. |
| history-tail | Complete final newline. |
| history-event-cap | At most 4,096 projected events. |
| history-event-contract | UTF-8, lexical projection or Trace checks. |

Codes identify a check group, not a native root cause. Event parsing/correlation
remain deliberately grouped. Unknown codes, nonstrings and other stages omit
the field. No historical receipt receives a retroactive code.

Failing-first tests observed missing codes on independent synthetic failures.
All 30 packet tests pass: layout/prefix/tail/event limits/contracts, a narrow
simulated OS-denial read preserving errno13, symlink guard and unknown-code,
wrong-stage/private-payload denials. Full repository/current-code review evidence
is recorded in the PR before merge. No native execution is part of this task.

## Future packet and authority

Proposal SHA-256 remains
39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
New manifest SHA-256: 809bcb6f61df6803e6fe4d080072d45b1d739a1d8a69ec4e41d9ecc3d2e0d7a2.
Only its observer pin changes. All three historical batch approvals are spent
and none matches this manifest. No live budget is created. #742 remains open and
#740 closed. A future exact owner-approved batch could identify the failed check
group; there is no automatic retry or native support claim.


The later [approved diagnostic attempt](parent-native-canary.md#history-diagnostic-packet-attempt--2026-11-09)
used this exact manifest and reported history-session-layout. It does not recover
the entry shape or the earlier failure cause. That fourth approval is spent;
no further live budget remains.
