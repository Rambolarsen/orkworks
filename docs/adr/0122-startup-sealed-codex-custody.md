---
type: Architecture Decision
title: Keep Codex capture custody in startup-sealed workers
description: Record owner acceptance of a bounded macOS qualification for startup-sealed channels and persistent custody workers.
tags: [orkworks, architecture, codex, custody]
status: stable
---

# Keep Codex capture custody in startup-sealed workers

- Status: accepted
- Deciders: project owner
- Date: 2026-11-27

## Context

[Issue #1981](https://github.com/Rambolarsen/orkworks/issues/1981) tracks descriptor isolation during concurrent macOS child launch. The earlier capture design expected a short-lived helper to return resource descriptors to the sidecar. The [launch audit](../validation/ordinary-review-codex-spawn-audit.md) found no common exclusion covering both descriptor receipt and existing application launch families. The channel setup itself also needs to occur before concurrent launches.

## Decision

The owner approved the [startup-sealed design](../superpowers/specs/2026-11-22-review-codex-custody-owner-design.md) from [PR #1992](https://github.com/Rambolarsen/orkworks/pull/1992) for bounded macOS qualification. Create and seal all worker channels during exclusive startup before any controlled child starts. Use at most four persistent workers that own capture resources and create no child processes. The sidecar sends resource descriptors to those workers and receives bounded bytes. An exact worker lease retains custody until close is confirmed.

Use one three-second publication budget for startup. Late or uncertain attempts remain charged to their slot until termination and reap are confirmed. A failed slot retires for that sidecar lifetime. The [qualification plan](../superpowers/plans/2026-11-22-review-codex-custody-owner-qualification.md) governs the proof and keeps production activation subject to separate approval.

## Consequences

Capture needs native evidence for sealed channels, exact worker binding, lease release, both concurrent launch orders, and cleanup. The qualification also measures startup, idle memory, and idle CPU costs. Production rollout remains a separate decision.
