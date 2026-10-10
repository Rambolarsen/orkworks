---
type: "Architecture Decision"
title: "User-approved session review prompt"
description: "Architecture decision record: User-approved session review prompt."
tags: ["orkworks", "architecture"]
---

# User-approved session review prompt

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-08-01

## Context

The observer-only MVP prevented every form of automatic terminal input. A user reviewing a session-owned plan needs a direct, intentional way to ask that same session for review; requiring a separate session obscures that action and adds unnecessary workflow.

## Decision

Permit exactly one user-initiated terminal write: the Details-panel review action. Electron main authenticates it with the sidecar secret. The renderer submits only the session ID; the sidecar canonicalizes and contains the persisted workspace-relative Markdown path, rejects control characters, constructs the fixed review prompt itself, writes it once to a live PTY, and records an event. No generic terminal-write API exists.

## Consequences

Review remains explicit and auditable while avoiding a broad terminal-control surface. The authoring session may perform the review, so independence is not implied; users can still start another session manually when they require it.

## Amendment — 2026-10-08: central Review surface

[ADR 0078](0078-fixed-desktop-shell-and-central-navigation.md) replaces the
Details-panel placement with a selected-session action that opens the single
central Review surface. The existing approval and authority contract is
unchanged: the renderer submits only the exact session ID, Electron
authenticates the request, and the sidecar revalidates the artifact before one
fixed prompt is written to a live PTY. Reading or refreshing Review does not
submit that prompt.
