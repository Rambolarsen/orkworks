---
type: Architecture Decision
title: Durable session archive state
description: Separate session organization from runtime lifecycle and serialize close-first archive and deletion.
tags: [orkworks, architecture, sessions, archive]
---

# Durable session archive state

- Status: accepted
- Deciders: user
- Date: 2026-10-18

## Context

[Issue #439](https://github.com/Rambolarsen/orkworks/issues/439) needs Active,
Closed and Archived organization with explicit row actions. External legacy
metadata snapshots must not erase an archive decision. Kill acceptance does
not establish completed process cleanup or durable ending finalization.

## Decision

Implement the [owner-approved design](../superpowers/specs/2026-10-18-session-archive.md).
Persist bounded versioned, identity-bound organization records in
`session-state/<id>.json`, separately from producer-written session snapshots.
Project organization and a stale-action token without rewriting lifecycle.
Serialize actions, resume and retention admission. An Active Archive or Delete
closes its captured runtime first and waits at most 15 seconds for cleanup and
durable ending; failure or timeout never performs the second step. Delete is
confirmed before requesting Close. Restore returns Archived to Closed.

Archived and unreadable organization states are protected from automatic
session retention; Archived does not consume the unarchived count budget.
Archive does not accept work, affect Git, or synchronize native tool archives.

## Consequences

Existing records need no bulk migration. Organization failures disable row
mutations while preserving history. Old binaries cannot honor archive-aware
retention; rollback must retain those guards to preserve the guarantee.
The [implementation plan](../superpowers/plans/2026-10-18-session-archive.md)
defines runtime, storage, accessibility and verification checks.
