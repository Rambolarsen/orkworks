---
type: Architecture Decision
title: Fixed inspector overlay and pin
description: Allow the fixed right inspector to overlay content or reserve space when pinned.
tags: [orkworks, architecture, desktop]
status: stable
---

# Fixed inspector overlay and pin

- Status: accepted
- Deciders: user
- Date: 2026-10-15

## Context

The user requested a right inspector that preserves central content width until
pinned, with subtle transparency and focus-scoped Escape dismissal. The owner
approved the concrete exception in [#862](https://github.com/Rambolarsen/orkworks/issues/862).

## Decision

Partially supersede the floating/docking prohibition in [ADR 0078](0078-fixed-desktop-shell-and-central-navigation.md)
and the inspector layout and preference contract in [ADR 0082](0082-react-grid-desktop-shell.md).
Only the existing fixed right-edge inspector gains two presentation modes.
Unpinned is the default: overlay central content with a 96% opaque background,
keeping text and controls fully opaque. Pinning reserves width beside the main
view and uses an opaque background. Both modes retain 280–420px resizing.
Overlay resizing never reduces central content width or Sessions width.
Pinned layout preserves the existing 560px central minimum and region clamps.
Below 1180 CSS px retain the existing temporary inspector page regardless of
pin preference. Do not introduce movable panels or a layout library.

The pin toggle exposes its state to assistive technology. Persist the preference
through the existing Electron-owned shell layout API as an optional boolean;
missing means unpinned. Keep legacy records valid and preserve the revisioned
store, reset barrier, and explicit corrupt-record recovery. Reset unpins.
This additive field introduces no new IPC authority or sidecar behavior.

Escape with focus inside the inspector or its temporary page closes it and
restores the original invoker or existing safe heading fallback, whether pinned
or unpinned. Respect nested dialogs and already-handled events. Escape outside
the inspector retains the owning surface's existing behavior.

## Consequences

Users inspect without resizing the terminal, and pin when they want dedicated
space. Overlay content covers part of the central view; the opaque pin mode is
available for sustained reading. One central context and terminal remain.
Responsive pages, selection, acknowledgement, execution, and grants retain their
existing contracts. Tests must cover geometry, transparency, pin transitions,
resize bounds, persistence, legacy reads, and focus-scoped dismissal.
