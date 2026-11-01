---
type: Architecture Decision
title: Ordinary sessions use SQLite with bounded history reads
description: Accepted single-authority ordinary-session storage and quiescent migration, separate from hierarchy storage.
tags: [orkworks, architecture, sessions, sqlite, migration]
status: stable
---

# Ordinary sessions use SQLite with bounded history reads

- Status: accepted; implementation and qualification pending
- Deciders: repository owner; approved the concrete design on 2026-11-01
- Date: 2026-11-01
- Tracking: [#1241](https://github.com/Rambolarsen/orkworks/issues/1241)
- Product contract: [session storage](../../specs/session-storage.md)
- Design: [SQLite session storage and bounded history](../superpowers/specs/2026-11-01-sqlite-session-storage-design.md)

## Context

The workspace retains hundreds of ordinary session records and roughly a GiB
of event/replay files. The complete session projection feeds routine list and
push refresh; indexed metadata queries and bounded pages are useful independently
of the live-capacity scan fix in ADR 0102. The sidecar already bundles rusqlite.
Existing agents can still write JSON directly, and organization state is stored
separately; a database is not authoritative until those writers are migrated.

## Decision

Store ordinary metadata, revisions and organization in one workspace-local
SQLite database behind a private worker with typed operations. Retain metadata
JSON inside a row with transactionally maintained query columns/search indexes.
Use one connection and rollback DELETE/EXTRA journaling for short captured reads;
verify the linked accepted limits and effective database settings.

Separate live reads/updates from bounded indexed history and exact detail reads.
Preserve complete internal-domain queries, historical facts, existing authority
checks and native eligibility gates. Keep large replay/images and existing event
history external, with identity-bound recoverable cleanup and complete backups.

Perform a maintenance-only, verified import and single activation with preserved
source backups, explicit incompatible-writer/downgrade behavior and no automatic
JSON fallback. Keep current retention settings and archive protection.

This supersedes ADR 0018 only for ordinary session-file authority and watching
after qualified format-2 activation; global workspace-scoped placement remains.
ADR 0004 was already superseded by ADR 0018 and is retained as history. This also
supersedes ADR 0088's organization-file persistence mechanism and ADR 0092's
complete-all-history REST/stream projection for format-2 workspaces, preserving
their state/retention and authoritative synchronization guarantees. Existing
format-1 source behavior remains until implementation and activation. ADR 0090's
separately gated hierarchy store is not implemented or superseded.

## Consequences

Indexed history becomes naturally bounded and metadata/organization updates can
share transactions. The implementation must migrate direct producers and existing
synchronous lock-held writers, negotiate transport versions, qualify crash and
durability behavior, and handle external payload recovery explicitly.

Implementation follows the [reviewed delivery plan](../superpowers/plans/2026-11-01-sqlite-session-storage.md).
Acceptance supplies no runtime code, live migration, destructive retention, native
enablement or platform qualification.
