---
type: Architecture Decision
title: Ordinary sessions use SQLite with bounded history reads
description: Proposed single-authority ordinary-session storage and quiescent migration, separate from hierarchy storage.
tags: [orkworks, architecture, sessions, sqlite, migration]
status: draft
---

# Ordinary sessions use SQLite with bounded history reads

- Status: proposed; concrete design and limits await owner approval
- Deciders: owner review pending; SQLite direction accepted in conversation
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

## Proposed decision

Store ordinary metadata, revisions and organization in one workspace-local
SQLite database behind a private worker with typed operations. Retain metadata
JSON inside a row with transactionally maintained query columns/search indexes.
Use one connection and rollback DELETE/EXTRA journaling for short captured reads;
verify the linked proposed limits and effective database settings.

Separate live reads/updates from bounded indexed history and exact detail reads.
Preserve complete internal-domain queries, historical facts, existing authority
checks and native eligibility gates. Keep large replay/images and existing event
history external, with identity-bound recoverable cleanup and complete backups.

Perform a maintenance-only, verified import and single activation with preserved
source backups, explicit incompatible-writer/downgrade behavior and no automatic
JSON fallback. Keep current retention settings and archive protection.

On acceptance, record the precise replacement relationship to ADR 0004/0018's
ordinary session-file authority while preserving global workspace placement;
do not mark their existing decisions superseded merely by publishing a proposal.
ADR 0088's state semantics and ADR 0092's authoritative synchronization guarantees
remain; their storage/transport amendments must be synchronized at acceptance.
ADR 0090's separately gated hierarchy store is not implemented or superseded.

## Consequences

Indexed history becomes naturally bounded and metadata/organization updates can
share transactions. The implementation must migrate direct producers and existing
synchronous lock-held writers, negotiate transport versions, qualify crash and
durability behavior, and handle external payload recovery explicitly.

No code, live migration, destructive retention, native enablement, platform
qualification or approved implementation plan is supplied by this proposed ADR.
