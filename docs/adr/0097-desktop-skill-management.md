---
type: Architecture Decision
title: Desktop-owned skill inventory and APM installation
description: Keep workspace and personal skill filesystem and process operations behind narrow Electron preload requests.
tags: [orkworks, desktop, skills, apm]
status: stable
---

# Desktop-owned skill inventory and APM installation

- Status: accepted
- Deciders: owner, approved 2026-10-28
- Date: 2026-10-28

## Context

The approved [workspace and personal skills design](../../specs/workspace-skills.md)
adds user-scope discovery and explicit package installation. The renderer has no
filesystem/process authority; the sidecar is workspace-scoped.

## Decision

Electron main owns bounded local skill discovery and qualified APM execution.
Typed preload operations expose inventory, bounded detail, native source picking,
preview and confirmed installation. Main derives workspace and personal roots;
the renderer cannot supply arbitrary read/execute paths or command arguments.
APM remains the dependency/deployment authority. Unsupported/incomplete profiles
explain their limitation instead of silently broadening installation.

## Consequences

Personal discovery works without a ready workspace sidecar. Existing terminal,
metadata and agent lifecycle authority do not change. Shared IPC shapes remain
intentionally duplicated across electron/ and src/. Installation failure may
leave partial APM changes and is reported honestly; no automatic rollback or
session restart is added. Known skill roots are inventory evidence, never proof
of native loading, invocation, permissions or effective precedence.
