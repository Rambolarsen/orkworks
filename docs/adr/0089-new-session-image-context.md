---
type: Architecture Decision
title: New sessions own bounded local image context
description: Approved attachment ingestion, authenticated creation, storage lifetime, and coding-tool delivery.
tags: [orkworks, sessions, images, protocol]
status: stable
---

# New sessions own bounded local image context

- Status: accepted
- Deciders: repository owner, approved 2026-10-23
- Date: 2026-10-23

## Context

New agent session currently accepts only text. The user requested clipboard
paste, drag and drop, and an attachment button. Browser image ingestion crosses
the renderer/main/sidecar boundary and requires explicit storage and delivery
ownership rather than assuming a filename is visible to the selected model.

## Decision

Use the approved [image-context contract](../../specs/session-image-context.md),
tracked by [#979](https://github.com/Rambolarsen/orkworks/issues/979).
The composer owns transient previews. Electron main validates bounded bytes
and captures the selected workspace. Authenticated additive session creation
lets the sidecar validate and retain private, session-owned image files.
A compiled, evidence-gated delivery policy supplies native image argv or verified
image references to supported coding tools. Unsupported routes fail visibly.
Existing text-only creation and permission policies remain compatible.

## Consequences

All three input methods share behavior and validation. Images remain available
through remembered-session resume and are deleted with their owning session.
Creation gains bounded binary data handling, main-only authorization for images,
and an explicit sidecar capability handshake. Tool/provider support requires
real launch evidence; documentation and synthetic fixtures alone cannot enable
an adapter. The owner approved dependent implementation on 2026-10-23.
