---
type: Architecture Decision
title: Antigravity native terminal attention
description: Reuse native terminal UI status for qualified Antigravity input waits without Peon prompt inference.
tags: [orkworks, architecture, antigravity, attention]
---

# Antigravity native terminal attention

- Status: accepted; limited 1.5.16/macOS arm64 source activation approved 2026-11-26
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
1.5.16 on 2026-11-26 after the retained strict-session trial. Literal Deny-button
and automatic-command behavior, human-visible confirmation and descendant
cleanup remain unverified. See the [qualification record](../validation/antigravity-native-status.md#owner-accepted-limited-source-activation-2026-11-26).
Other profiles retain the qualification gate. Reverting source and relaunching restores prior
behavior without migrating stored attention or changing installed hooks.
