---
type: Architecture Decision
title: Isolated headless replay fixture
description: Keep the existing Linux replay fixture sandbox exception inside a bounded offline non-root container path.
tags: [orkworks, testing, containers, security]
---

# Isolated headless replay fixture

- Status: accepted
- Deciders: repository owner
- Date: 2026-11-09

## Context

[#95](https://github.com/Rambolarsen/orkworks/issues/95) needs reproducible
headless replay evidence. The owner approved the concrete
[design and implementation contract](../superpowers/specs/2026-11-09-headless-replay-container-design.md)
in chat before implementation. Existing Linux Electron fixtures use
`--no-sandbox`; this is not the production policy in [ADR 0009](0009-electron-security-posture.md).

## Decision

Provide a separate fixture snapshot image and a fixed host entry command.
Only allowlisted tracked fixture inputs reach the build daemon. Install frozen
dependencies and the locked Electron binary during build. Run the existing
replay fixture offline as UID/GID 1000 with all capabilities dropped,
no-new-privileges, default seccomp, init and the approved resource/time limits.
Pass no host mounts, display, credentials, devices or engine socket.

Retain the Chromium exception only for this trusted-source developer fixture.
Do not enable the production app, arbitrary commands/sites, sidecar or coding
harnesses through this path. Source/dependencies are root-owned; artifacts use
the disposable writable layer so they survive fixture exit for bounded export.
Operate only on the CID written by the invocation's own create command.
Export regular allowlisted artifacts as the host caller and remove only that
owned container. Report unsettled cleanup rather than claiming success.

## Consequences

Existing dev Compose caches/binds, native development and releases are unchanged.
Each engine/host profile needs real offline rendering, ownership and lifecycle
qualification before support claims. A VM profile does not qualify native
rootless Linux. No privileged mode, host display/IPC or broader sandbox exception
may be added to bypass a failed qualification. The approved design retains the
exact boundaries, budgets, evidence requirements and rollback contract.
