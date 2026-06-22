# Session Details Provider Context Design

- Date: 2026-06-22
- Status: proposed

## Summary

Provider information should not remain a primary surface in the main OrkWorks window. For v1, provider context should appear only in the selected session's read-only `Details` view as lightweight runtime metadata:

- `Provider`
- `Model`
- `State`

This keeps the UI aligned with the product's single-active-context rule and with the expectation that terminal-native users do not want an always-visible provider control plane.

## Problem

The current provider feature is justified by Peon runtime needs: Peon may need app-wide provider defaults, fallback behavior, and state overrides so observation does not fail silently when one harness is capped or unavailable.

That backend need does not imply that a dedicated, always-visible Providers panel should be a first-class workflow surface in the main window. A top-level provider ops panel risks making OrkWorks feel like it is trying to manage harness usage directly, which is misaligned with the product goal of observing and recommending before it controls.

## Design Goals

- Keep the main window centered on sessions and user attention.
- Preserve the single-active-context model.
- Show only the provider information relevant to the selected session.
- Keep the `Details` view read-only and interactionally consistent.
- Preserve app-wide provider configuration for Peon without surfacing it as a main-pane dashboard.

## Proposed UX

### Session Details

The selected session's `Details` view includes three read-only fields:

- `Provider`
- `Model`
- `State`

These fields are factual runtime context, not controls.

`State` reflects only the current provider state for that session, such as:

- `healthy`
- `degraded`
- `capped`
- `unknown`
- `disabled`

No fallback-order display, override controls, diagnostics drawer, or explanatory fallback text appears in `Details` for v1.

### Main Window

The main window should not expose a dedicated Providers panel as a primary concept. Provider context is supporting metadata for the currently selected session, not a dashboard in its own right.

### Settings

Provider editing remains app-wide and moves out of the main window into `Settings`.

Settings remains the place for:

- default provider preferences
- override state configuration
- fallback ordering
- any future app-wide provider behavior

This preserves a simple interaction rule:

- Main window: observe the selected session
- Settings: change app-wide behavior

## Data Semantics

The values shown in `Details` should be session-specific runtime metadata, not global defaults.

- `Provider`: the provider Peon used for the latest successful observation of the selected session
- `Model`: the model associated with that provider run, if known
- `State`: the provider state associated with the latest assessment for that session

Behavior:

- If no provider-backed observation has happened yet, show unresolved values such as `unknown` or `—`.
- If Peon is unavailable or disabled, keep the fields visible but unresolved.
- If provider state changes later, the selected session's details update through the existing session metadata refresh flow.

## Non-Goals

- Turning provider management into a primary dashboard surface
- Adding inline editing controls to `Details`
- Showing fallback chains, advanced diagnostics, or provider history in the main window
- Making provider information multi-session or app-global inside the `Details` view

## Testing and Validation

Implementation should verify:

- `Details` renders `Provider`, `Model`, and `State` only for the selected session.
- The fields remain read-only in all states.
- Empty and unresolved provider metadata display predictably.
- Existing app-wide provider configuration still works through `Settings`.
- The main window no longer depends on a dedicated Providers panel for core provider visibility.

## Open Questions

None for this slice. The agreed v1 endpoint is intentionally narrow.
