---
type: Architecture Decision
title: React and CSS Grid desktop shell
description: Replacement of Dockview with explicit React regions and accessible resizing.
tags: [orkworks, architecture, desktop]
status: stable
---

# React and CSS Grid desktop shell

- Status: accepted
- Deciders: user
- Date: 2026-10-10

## Context

The owner requested removal of Dockview while taking over [PR #796](https://github.com/Rambolarsen/orkworks/pull/796).
[ADR 0078](0078-fixed-desktop-shell-and-central-navigation.md) originally retained
Dockview for resizing and hosting fixed regions. Its compact Sessions, one
central context, optional inspector, responsive navigation, and persistence
contracts remain the intended product behavior. A layout engine with panel
registration and group activation adds indirection to that fixed structure.

## Decision

Replace Dockview with a renderer-owned React shell and CSS Grid.
Remove both `dockview` and `dockview-react`, their stylesheet, panel registry,
API refs, group activation, and library-specific styling. Do not introduce a
replacement docking or layout dependency. This replaces ADR 0078's library
choice only; all its other decisions remain binding.

`App` retains session/workspace authority and the existing shell-navigation
reducer. A focused `ApplicationShell` renders fixed Sessions, a central host,
and at most one optional inspector. Use explicit destination state and callbacks
rather than a compatibility facade shaped like Dockview's panel API. Keep
region rendering separate from session selection, acknowledgement, and PTY input.

A small resize component owns pointer capture and keyboard interactions.
Separators expose orientation, current width, and bounds to assistive technology;
arrows adjust in bounded steps and Home/End select bounds. Cancellation or
unmount releases pointer interaction. Sessions remains 200–320 CSS px and the
inspector 280–420px. Preserve at least 560px of central content at wide widths;
clamp region widths to available space and use a temporary inspector page if
minimum regions cannot fit. At 860–1179px, retain Sessions when it fits and
show utilities as temporary central pages; below 860px show one page. Effective
logical width at text zoom follows the same rules.

Use existing Electron-owned shell preference APIs for widths, visibility, and
density. Hydrate once without saving initialization as a user change. Serialize
user changes through the existing store; disposal cancels pending presentation
saves, reset remains an ordering barrier, and stale replies cannot overwrite a
new user intent. Existing legacy `layout.json` bytes remain untouched.

Temporary pages have labeled return controls and scoped Escape handling.
Opening a page focuses its heading or Sessions list; closing restores the
connected invoker or a visible safe heading. Navigation never forwards keys
to a hidden terminal. Hiding Terminal detaches presentation only: the retained
runtime continues output draining and reattaches/refits without a second PTY.

Delivery remains [#779](https://github.com/Rambolarsen/orkworks/issues/779).
Review-specific entry/rendering/return belongs to #780; expose no Review
control or terminal plan-link handler until its destination works. Preserve
the Review component and authenticated explicit-request API for that follow-up.
Actions belongs to #805. Workflow remains behind its existing independent gates.

## Consequences

The shell directly expresses the approved fixed structure and removes panel
lifetime/group activation from navigation. There is no library tab or drag
surface to disable. Resizing, focus restoration, responsive rendering, and
visibility reporting become explicit renderer responsibilities requiring
behavioral coverage rather than source-pattern checks.

Rollback restores the dependency and renderer from Git. Shell preferences keep
their existing schema; legacy layout bytes and session metadata are unchanged.
No storage migration, sidecar change, or new IPC authority is required.

The owner approved this concrete replacement on 2026-10-10 after reviewing
the visual preview and its description. ADR 0078's library choice and the
accepted navigation design were reconciled before runtime changes.
The amended [delivery plan](../superpowers/plans/2026-10-10-application-shell-fixed-shell.md)
records implementation ownership and verification. Native Windows/Linux chrome
and screen-reader walkthroughs require actual platform evidence; headless tests
cannot substitute for them.

### Delivery exception — approved 2026-10-10

The owner explicitly deferred the native walkthroughs until after PR #796
merges ("ill check after the merge"). Track the pending keyboard, screen-reader,
reduced-motion, zoom, narrow-width and macOS/Windows/Linux chrome evidence in
[#836](https://github.com/Rambolarsen/orkworks/issues/836), required before a release
containing this change. This scoped timing exception does not claim the checks
passed or waive code review and automated verification.
