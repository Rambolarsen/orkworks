---
type: Architecture Decision
title: Antigravity native terminal attention
description: Reuse native terminal UI status for qualified Antigravity input waits without Peon prompt inference.
tags: [orkworks, architecture, antigravity, attention]
---

# Antigravity native terminal attention

- Status: accepted; limited 1.5.16/1.5.17 macOS arm64 source activation approved 2026-11-26; 1.5.18 added 2026-11-27
- Deciders: Lars-Erik, Codex
- Date: 2026-11-24
- Extends: ADR 0093's terminal-status approach; Codex behavior remains unchanged.

## Context

Antigravity hooks report model-start and idle boundaries, not a qualified
human-prompt lifecycle. The owner rejected Peon prompt inference as unstable.
Antigravity documents native OSC 7501 UI state, but the installed artifact's
question/answer and working/done sequence is retained. Manual tool denial and
question cancellation were initially unverified. Later native traces and owner
acceptance of narrowed coverage are recorded below.

## Decision

Implement the approved [native terminal attention design](../superpowers/specs/2026-11-24-antigravity-native-attention-design.md).
Reuse a bounded, profile-selected live PTY parser and the existing generation-
and provenance-fenced attention application. Qualify exact platform/version
and built-in launch routes before enabling Antigravity emission/consumption.
Hashes identify retained experiments rather than enforce executable integrity.

Hooks retain identity capture and non-emitting Working/Idle fallback. After
native status activation they cannot overwrite terminal attention. Peon
preserves Antigravity's attention tuple while continuing descriptive metadata;
there is no rollout bulk clear. Explicit user/separate-agent attention wins.
No permission decisions, native request ownership, ACP connection or history
replay authority is introduced.

## Consequences

A qualified native blocked state can report Needs You for structured questions,
permissions or authentication, without reading conversational prose. Typing or
silence does not clear it; only recognized native updates or existing authorized
lifecycle operations do. Missing/spoofed frames and nonaggregate displayed-thread
state remain accepted presentation limits. Prose questions with no native wait
are unsupported. The owner accepted limited activation for exactly macOS arm64
1.5.16 on 2026-11-26 after the retained strict-session trial. A later
[qualification follow-up](../validation/antigravity-native-status.md#explicit-rejection-and-automatic-command-follow-up-2026-11-26)
retains explicit native rejection and one independently witnessed automatic
command without a prompt block. Human-visible confirmation, installed OrkWorks
UI behavior and complete descendant cleanup remain unverified.
Other profiles retain the qualification gate. Reverting source and relaunching restores prior
behavior without migrating stored attention or changing installed hooks.


## Amendment: additional 1.5.17 profile, 2026-11-26

After [qualification PR #2081](https://github.com/Rambolarsen/orkworks/pull/2081),
the owner directed continued implementation in response to the concrete request
to admit exactly macOS arm64 CLI 1.5.17 alongside 1.5.16 with the retained limits.
The 1.5.17 trials establish question answer/cancel, explicit No rejection and one
independently witnessed automatic command without a prompt block. Both root
exits and unchanged artifact hashes were confirmed. Human-visible/installed UI
qualification and complete descendant cleanup remain unverified. This accepts
limited best-effort UI coverage for the additional exact version; it changes no
architecture, permission ownership or eligibility of other versions/platforms.
The experimental strict setting and update-suppression override are not added
to production launches. Existing sessions and installed hooks are unchanged.


## Amendment: additional 1.5.18 profile, 2026-11-27

After [qualification PR #2121](https://github.com/Rambolarsen/orkworks/pull/2121),
the owner directed continued implementation in response to enabling exactly
macOS arm64 CLI 1.5.18 with its [retained evidence and limits](../validation/antigravity-native-status.md#owner-accepted-1518-source-activation-2026-11-27).
The two agent-operated trials retain question answer/cancel, explicit No rejection
and one independently witnessed automatic command. Both root exits and unchanged
binary hashes were confirmed. Human-operated confirmation, OrkWorks UI/installer
behavior for 1.5.18 and complete descendant cleanup remain unqualified, along
with the other recorded limits. This accepts limited coverage for an additional
exact-version profile without changing architecture, permission ownership,
launch checks or parser semantics. Diagnostic strict-permission and updater
suppression are not applied in production. Other versions/platforms, existing
sessions and installed hooks are unchanged.


## Later desktop question evidence, 2026-11-27

The [source-build 1.5.18 UI check](../validation/antigravity-native-status.md#source-build-desktop-1518-question-check-2026-11-27)
observed Needs You during a structured question, then Working/Idle and no
attention item after the answer, with Peon disabled. This adds agent-observed
question UI evidence without changing eligibility or architecture. It does not
qualify 1.5.18 command-permission UI, a published installer, human-operated
confirmation or complete descendant cleanup. Historical approval limits above
retain their original context.
