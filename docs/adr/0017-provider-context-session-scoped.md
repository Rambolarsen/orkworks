# Provider context is session-scoped, not app-wide

- Status: superseded
- Superseded by: peon model picker feature (2026-06-22) — per-provider `peonModel` selection restored in Settings modal
- Supersedes: 0016 (Settings surface only)
- Date: 2026-06-22
- Deciders: OrkWorks maintainers

## Context

ADR 0016 moved provider context into read-only session `Details` fields and kept provider editing app-wide in `Settings`. In practice, provider context is session-specific: each session records which provider ran its Peon inference, and that context lives in the session metadata. App-wide provider configuration surfaced in Settings is unnecessary and conflicts with the session-centric design principle.

## Decision

Remove the app-wide Providers section from the Settings modal. Provider context (`Provider`, `Model`, `State`) remains in the read-only session `Details` panel, sourced from per-session metadata.

The backend fallback system from ADR 0015 remains in place — Peon still respects provider settings for fallback order, enable/disable, and capacity state — but those settings are no longer user-editable through the Settings UI. Defaults are used.

## Consequences

- Provider is a session-scoped concept: shown only in session details, never in app-wide settings.
- The Settings modal contains only Hotkeys and Session Retention sections.
- Provider fallback configuration is not user-editable in MVP; defaults are used.
- ADR 0016's decision to keep provider editing in Settings is superseded.
