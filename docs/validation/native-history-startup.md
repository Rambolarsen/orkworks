---
type: Qualification Fixture Correction
title: Wait for initial history directory creation
description: Synthetic proof and bounded correction of premature empty-root rejection, with fixed layout failure flags.
tags: [copilot, history, qualification, startup]
status: stable
workflow_status: offline-corrected-not-authorized
---

# Wait for initial history directory creation

[#1631](https://github.com/Rambolarsen/orkworks/issues/1631) follows the
[session-layout diagnostic](parent-native-canary.md#history-diagnostic-packet-attempt--2026-11-09).
A synthetic staged-startup sequence reproduces a real observer inconsistency:
periodic scanning accepts an absent root and an exact session directory without
an event file, but rejected the intervening empty root. This correction handles
that state without asserting it caused the historical native failure.

## Source and synthetic evidence

[Bounded audit data](native-history-startup.json) records literal failure flag
examples and app.js SHA-256
f84061b62eeb1cd78a655555890fe1a776509203fca68144745467e26e443813.
At zero-based UTF-8 byte offset 143181, the Ry filesystem adapter's appendFile
catches ENOENT, recursively creates the parent directory, then retries append.
The adapter supports staged filesystem creation; its calls are separate operations.
The direct local constructor/path/event work also crosses compiled native bridges,
as the [source audit](native-history-source-audit.md) explains. This does not prove
that a particular native run used the adapter or exposed an empty root.
No vendor code/addon was imported or executed; only pinned bytes were read.

Independent synthetic directories exercise:

1. Root absent: periodic scan remains pending.
2. Root created but empty: the old scanner raised history-session-layout.
3. Exact UUID directory created without events: periodic scan remains pending.
4. Complete session.start line appended: final scan consumes it and validates identity.

The failing-first test observed step2 rejection before the fix. It now completes
the entire sequence without advancing offsets/events during the waiting states.
This is an observer regression test, not a native canary receipt.

## Bounded waiting rule

An empty root waits only when all conditions hold: phase1, nonfinal scan, byte
offset0 and projected event count0. Root symlink/type/enumeration checks still
run first. Empty final scans, phase2 scans and empty roots after consumed history
still fail. Wrong/extra entries and expected/root symlinks still fail during
polling. The native process/batch deadlines and output limits remain in force.

Waiting grants no identity, authority or success: exact session.start, source,
model/tool/nonce/completion joins and final/resume checks remain required. No
parser, permission, model, tool, startup instruction or provider fallback changes.

## Fixed layout flags

Only native-history/history-session-layout failures may add optional layout:

| Field | Retained meaning |
| --- | --- |
| empty | The enumerated entry list was empty. |
| expectedPresent | The expected UUID path was in that list. |
| exclusiveExpected | The list equaled the single expected path. |
| expectedSymlink | Its symlink result when exclusiveExpected is true; null otherwise. |

The symlink check keeps the existing short-circuit behavior: no extra stat is
performed for wrong/extra entries. Flags derive from the same enumerated list
and guard result. The projection requires exactly these four keys, three exact
booleans and a boolean/null symlink value. Extra fields, wrong types and other
stage/check-code contexts omit layout. No names, paths, target strings, counts,
payloads or exception text are copied. Existing failure categories/check codes
and numeric OS errors are preserved.

All35 packet tests pass, including final/resume/post-consumption empty-root
rejection, wrong/extra/symlink flags and malformed/private diagnostic denial.
The actual historical entry shape remains unknown; all four receipts stay
unchanged and receive no retroactive flags or cause. Full repository/current-code
review and CI evidence are recorded in the PR before merge.

## Future packet and decision

Proposal SHA-256 remains
39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
New manifest SHA-256: e89cd3cf0ba4c9b91f7a752edecd6a59f815d02d087503a4ac577959f90f7c9a.
Only its observer pin changes. All four prior batches are spent and none binds
this manifest. Live budget is zero; another native batch needs a new exact owner
decision. #742 stays open, #740 closed and all three native capabilities unverified.
