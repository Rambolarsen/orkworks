---
type: "Architecture Decision"
title: "Accumulate alternate-buffer trackpad input"
description: "Architecture decision record: Accumulate alternate-buffer trackpad input."
tags: ["orkworks", "architecture"]
---

# Accumulate alternate-buffer trackpad input

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-10-03
- Supersedes: [ADR 0070](0070-alt-buffer-wheel-input-survives-xterm-dampening.md)

## Context

ADR 0070 restored wheel responsiveness by forwarding every nonzero wheel event
in the alternate buffer. A trackpad gesture contains many small pixel events:
an Electron reproduction of fifty 1px events produced fifty application scroll
commands with and without mouse tracking, at both sensitivity 1 and 0.1.
Changing ordinary scrollback sensitivity cannot control this path. See
[issue #720](https://github.com/Rambolarsen/orkworks/issues/720).

## Decision

Keep the version-scoped xterm 6.0.0 patch and its tests for both shipped bundles.
In the live renderer, install a public `attachCustomWheelEventHandler` before
opening each terminal. Only alternate-buffer pixel-mode vertical input is
filtered. Forward the first nonzero movement immediately; accumulate subsequent
movement until it reaches one rendered CSS cell height, then forward a direction
event and retain the fractional remainder. At most one event is forwarded for
each incoming event, preserving xterm's direction-only protocol behavior.

A pause of at least 150ms, a direction or modifier change, or a buffer transition
starts a new gesture. This preserves small isolated gestures and immediate
reversal without forwarding every momentum packet. Line/page-mode wheel input
and normal-buffer scrollback keep their existing paths. Suppressed events are
cancelled so they cannot scroll the surrounding application. Each terminal owns
its accumulator; buffer-change subscriptions are disposed with the terminal.

## Consequences

- Trackpad bursts no longer amplify tiny packets into full scroll commands.
- Small gestures stay responsive, including shift-wheel and both mouse-tracking
  modes. The patch still prevents the original dead-wheel regression.
- Alternate-buffer pixel filtering uses rendered line height independently of
  ordinary scrollback sensitivity; ordinary and Alt fast-scroll settings remain
  as configured by PR #708.
- No private xterm API, extra timer, or dependency-patch regeneration is needed.
- Tests cover bursts, first movement, pauses, reversal, buffer switches,
  modifiers, line/page events, and normal scrollback. Native subjective feel
  still needs validation with the user's device.
