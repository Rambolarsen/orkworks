---
type: Architecture Decision
title: Read-only selected-session Git changes
description: Bind bounded Git inspection to the selected session through the existing authenticated desktop transport.
tags: [orkworks, git, security, desktop]
status: stable
---

# Read-only selected-session Git changes

- Status: accepted
- Deciders: user
- Date: 2026-10-24

## Context

The owner approved the [concrete design](../../specs/session-git-changes.md)
and [implementation plan](../superpowers/plans/2026-10-24-session-git-changes.md)
in the conversation for [PR #1003](https://github.com/Rambolarsen/orkworks/pull/1003).
The existing Details diff action is a placeholder.

## Decision

Add distinct Uncommitted changes and Branch changes comparisons, hosted as
alternative content in the existing fixed inspector under ADR 0083. Preserve
central document Review and the selected terminal runtime. Session identity
provides context, not authorship attribution.

Rust owns bounded immutable captures, exact registered-session/effective-worktree
resolution and opaque snapshot/file lookup. Electron main uses the existing
plan-content desktop credential and ready-backend generation fence; renderer
access is limited to Changes-specific preload methods. No arbitrary path,
revision-expression, command or object-content bridge is exposed. Filesystem
access must be contained and must not follow symlinks. No original repository
writes, Git network operations or external filters are allowed.

## Consequences

Baselines and capture time stay visible; local refs may be stale until the user
fetches through their coding tool. Captures are bounded and may explicitly omit
large/binary/unsupported content. Worktree reads describe a capture interval,
not an atomic filesystem transaction. Ephemeral snapshots require no persisted
schema migration and can be removed with their routes/bridge/panel on rollback.
