---
type: Architecture Decision
title: Joint child preparation uses a private attached transaction broker
description: Accepted scoped broker, lineage profile and linked tombstone direction for further qualification, without source permission or activation.
tags: [orkworks, architecture, hierarchy, sessions, sqlite]
status: draft
---

# Joint child preparation uses a private attached transaction broker

- Status: accepted architecture for further qualification; implementation gated
- Deciders: owner, scoped “keep going” on 2026-11-05
- Date: 2026-11-05
- Tracking: [#1410](https://github.com/Rambolarsen/orkworks/issues/1410), [#1176](https://github.com/Rambolarsen/orkworks/issues/1176)
- Decision source: [scoped acceptance](../superpowers/specs/2026-11-05-joint-child-preparation-design.md#scoped-architecture-acceptance--2026-11-05), reviewed PR #1407

## Context

Original child preparation must prove ordinary creation/lineage and complete
hierarchy source/control/summary/five-ledger effects together. Independent
ordinary create and hierarchy commits do not supply that original disposition.
ADR 0090 kept one separate hierarchy connection with no ATTACH; ADR 0103 retained
ordinary SQL authority behind its own private worker. The reviewed design makes
the linked-profile integration change and its availability/retention cost explicit.

## Decision

For a separately qualified linked workspace, use one ordinary-worker-owned
connection with exactly its verified hierarchy peer attached in rollback
DELETE/EXTRA mode. Typed private source and ordinary writers borrow one
transaction; neither commits independently. Accept this direction for further
qualification, not runtime implementation or a demonstrated physical profile.

Accept the reviewed immutable ordinary lineage/index and schema-2 direction,
new-store-only hierarchy format-5 projection/commitment, and retained hidden
linked tombstones until qualified joint purge. Preserve existing numerical
budgets, source/request/claim codecs and their authority/retention rules.

This supersedes ADR 0090's separate-connection/no-ATTACH direction and ADR 0103's
independent worker separation **only for this linked profile**. Their ordinary
and hierarchy authorities remain distinct; all other decisions, the dormant
lifecycle-close amendment and source/activation/native gates remain. Existing
production and unlinked ordinary profiles do not change by this record.

## Consequences

One engine transaction can provide a candidate joint atomic outcome without an
application-level prepare/commit recovery log. A blocked hierarchy transaction
can also occupy the linked ordinary SQL worker; history/metadata may report
bounded busy/unavailable results while immediate stop and terminal I/O continue.
A deleted linked session may retain hidden metadata as a source consumer until
safe joint purge. Both costs were included in the scoped owner decision.

The complete named catalog still needs closure; paired ordinary schema activation,
metadata-only membership, bounded memory/journals/super-journal and native
crash/lease/VFS behavior require qualification. The current source-authority-at-
physical-commit rule is unchanged and unresolved; atomicity is no permission to
write after revocation. No new source permit, executable plan, database/SQL,
migration, registration, live capability or probe is accepted. The three
architecture choices need no repeated approval within this scope.
