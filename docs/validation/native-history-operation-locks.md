---
type: Qualification Fixture Correction
title: Recognize the native operation-lock namespace
description: Narrow offline compatibility correction for the pinned operation-lock directory with exact session checks preserved.
tags: [copilot, history, qualification]
status: stable
workflow_status: offline-corrected-not-authorized
---

# Recognize the native operation-lock namespace

[#1646](https://github.com/Rambolarsen/orkworks/issues/1646) follows the
[corrected-startup attempt](parent-native-canary.md#corrected-startup-packet-attempt--2026-11-09).
The continuation contract requires exact session identity and rejects another
session; it does not require the absence of bookkeeping directories. This
synthetic qualification utility now recognizes one fixed bookkeeping namespace.
It is not a production authority or OS containment boundary.

## Static evidence and limits

Only pinned source bytes were read; no vendor code or native addon was loaded.
Copilot 1.0.90 app.js SHA-256 is
f84061b62eeb1cd78a655555890fe1a776509203fca68144745467e26e443813.
At zero-based byte offset 1595580, creation computes the session path and
registers a session lock before emitting session.start. registerSessionLock at
1625314 delegates to inUseLocksRegisterSession with the session path and PID.
The pinned runtime.node SHA-256 is
ad89ad135478cd2b19a5b99bb19064d0be281961cab0506f3767f39234d4c591.
It contains the exact ASCII string .session-operation-locks at byte offset
72575518, adjacent to lock-related strings.

These facts support a narrow compatibility choice. They do not establish the
compiled runtime's concrete parent placement or creation path. The previous
receipt retained no extra entry names or types: this is not proof that the
namespace caused that stop. All five historical receipt bytes remain unchanged.

## Correction and synthetic checks

The scanner excludes only the exact .session-operation-locks entry, after one
lstat confirms a real directory. It neither traverses nor reads its contents.
A symlink or regular file with that name fails history-operation-lock-layout.
Every other extra entry remains a session candidate and fails the existing
exclusive-expected-session rule. Expected-session symlinks remain rejected.

An otherwise empty candidate list waits only during pristine phase1 polling,
with offset0 and event count0. Final and resume checks still require the exact
session. Prefix commitments, session.start/session.resume identity, event limits,
model, tool, nonce, runtime and receipt joins remain required. No proposal,
permissions, source guards, peer oracle, tool set, fallback or process budget changes.

Future session-layout failures use sessionLayout with the four existing typed
flags: empty, expectedPresent, exclusiveExpected and expectedSymlink. Those flags
now describe the filtered candidate list. Historical layout fields describe the
original raw root listing and are not reinterpreted. Projection still rejects
extra fields, wrong types and wrong stage/check-code contexts; no names, counts,
paths, lock contents or exception text are retained.

Failing-first tests reproduced rejection of the real lock directory. All40
packet tests pass, covering selected-session start and append-only resume beside
unread private lock contents, pristine waiting, final/resume rejection without a
session, foreign/unknown entries, wrong event identity, reserved-name files and
both namespace and selected-session symlinks. These are synthetic results, not
native capability evidence. Repository and independent review evidence belongs
to the associated PR.

## Future packet and decision

Proposal SHA-256 remains
39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
Manifest SHA-256 is 1c07a44951b076dacadbd2a35611ef1cf3077d56dec37294eeae92b3cdff1d22.
Only its observer pin changes. All five previous batch approvals are spent;
none authorizes this manifest. Live budget remains zero. Another exact native
batch requires a fresh owner decision. #740 remains closed, #742 open, and all
three native capabilities remain unverified.
