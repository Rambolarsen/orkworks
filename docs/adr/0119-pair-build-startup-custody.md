---
type: Architecture Decision
title: Pair candidate build and process startup preserve allocation custody
description: Accepted shared SQLite memory-management undefinition and early allocator bootstrap recipe for further qualification, with ordinary-consumer consequences and execution gates retained.
tags: [orkworks, hierarchy, sqlite, windows, startup, qualification]
status: draft
---

# Pair candidate build and process startup preserve allocation custody

- Status: accepted for further qualification; implementation and execution gated
- Deciders: repository owner, “keep going” on 2026-11-24 to the explicit #2043 build/startup amendment question
- Date: 2026-11-24
- Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176)
- Supersedes: [ADR 0112](0112-pair-native-enforcement.md) only for the unamended bundled-feature candidate assumption; refines its early bootstrap custody
- Preserves: [ADR 0118](0118-pair-resource-assurance.md) and the original worker/VFS/router/PairCore/source constraints
- Canonical contract: [accepted build/startup recipe](../superpowers/specs/2026-11-24-pair-build-startup-design.md#scoped-build-and-startup-acceptance--2026-11-24)

## Context

The pinned bundled build defines SQLITE_ENABLE_MEMORY_MANAGEMENT. Its default
cache then uses one PGroup; the source recycling path can move a compatible page
between pair and unpaired caches without allocating that page again. Allocator
callbacks alone do not establish pair ownership on that path. This is conditional
source reachability, not an observed overrun or an impossibility finding.

Existing Codex/OpenCode read-only SQLite users can initialize the library before
a future pair starts. SQLite configuration is process-global, not threadsafe,
and most options reject late use. Some getters also populate default tables.
An explicit build/startup recipe is needed before claiming pair eligibility.

## Decision

Accept the exact reviewed #2043 package for further qualification:

1. Keep one bundled SQLite 3.46.0 instance. For the selected future Windows x64
   MSVC candidate use build-owned
   `LIBSQLITE3_FLAGS=-USQLITE_ENABLE_MEMORY_MANAGEMENT`; require the macro absent,
   not merely zero. Retain THREADSAFE=1/core mutexes and proven cache/configuration
   custody so the selected default per-cache groups prevent the identified reuse.
   No unknown external PAGECACHE/custom PCACHE2/shared-cache configuration is
   adopted or overwritten to force availability.
2. One private startup owner runs after Git-scan early dispatch and before Tokio/
   sidecar runtime consumers. Establish stable saved-method/pAppData/registry
   storage before installing the already accepted allocator router. Initialize
   global SQLite outside pair context and preserve saved allocator delegation for
   unpaired Codex/OpenCode/ordinary consumers.
3. Preserve exact query/failure state: GETMALLOC can fill the default table;
   GETPCACHE2 is deliberately omitted. A failure cannot claim pristine global
   state. After router installation, retain callback storage/delegation for its
   process lifetime and keep pair eligibility false on failure. No restore,
   unregister, shutdown/reset or restart-to-fresh forces qualification.

The source/feature/compiler/target/effective-argument and linked-artifact recipe
must be fixed and verified for the actual candidate. A floating release runner,
version label, caller flag or compile-option string alone is not that proof.
No actual build or startup implementation is qualified by the decision.

## Consequences

The shared change affects unpaired SQLite too: sqlite3_release_memory becomes a
no-op and cache pressure/reclamation/performance can change. Actual Codex,
OpenCode and ordinary storage compatibility must be qualified against the changed
artifact; no identical behavior is claimed from unchanged SQL or source searches.

This supersedes only the assumption that the unamended bundled-feature build is
the selected pair candidate. It does not adopt a private engine copy, custom
PCACHE2 wrapper, source/dependency change, global heap limit or extra numeric
settings. The accepted 1 GiB hierarchy/shared 2 GiB auxiliary file-length and
fixed 8 MiB arena meanings, original owner and all lower budgets remain binding.

All source/newness/cutoff/initializer/catalog/paired-recovery/durability/ordinary
activation gates and consumed probe budgets remain. Acceptance enables further
qualification planning, not flags, startup/configuration changes, target metadata
retry, code or compiler/native/model/database/SQL execution, migration, source
registration or runtime activation. #1176 remains open and the backend unavailable.
