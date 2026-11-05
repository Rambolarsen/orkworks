---
type: Architecture Decision
title: Retained source registry roots order registration against invalidation
description: Accepted immutable RegistryRoot compare-and-close direction for further qualification, separate from durable source authority.
tags: [orkworks, hierarchy, sources, authority, concurrency]
status: draft
---

# Retained source registry roots order registration against invalidation

- Status: accepted for further qualification; implementation/activation gated
- Deciders: owner, explicit “go ahead” on 2026-11-05
- Date: 2026-11-05
- Tracking: [#1434](https://github.com/Rambolarsen/orkworks/issues/1434), [#1176](https://github.com/Rambolarsen/orkworks/issues/1176)
- Canonical contract: [source-publisher registry ordering](../superpowers/specs/2026-11-05-source-publisher-execution-design.md#proposed-registry-ordering)
- Decision record: [scoped acceptance](../superpowers/specs/2026-11-05-source-publisher-execution-design.md#scoped-registryroot-acceptance-and-readiness--2026-11-05)

## Context

Durable source membership at H cannot prove that no live binding registered after
affected-set discovery. Successful registration need not advance a source fence.
Checking only that fence can therefore leave a newly registered dependent binding
open through a source mutation. Caller-supplied affected lists or long authority
locks around discovery/SQL do not satisfy the accepted interface.

## Decision

Retain one immutable live RegistryRoot reference. Prebuild replacement roots and
complete affected-binding closure outside the latch. Registration compares the
captured root, source fence, scopes and actual admitted source/runtime authority,
then installs the replacement and transitions the exact binding together.
Source begin compares that same root/fence before terminally closing the fence,
pending tickets and every exact affected binding under the same short cutoff.
A changed root/fence denies the prepared transition; no patched subset or silent
retry. Every insertion/removal replaces the root, and retained old roots prevent
address reuse from substituting equality. Retire old references/destruction after
unlocking. Immediate exact runtime/run/workspace close requires no discovery.

This is the unchanged mechanism reviewed in PR #1414, head
`77b05e9f1b60a753762261dd14fb607d1906bf53`, merged
`260f84cd07cd3a58919eb2c12c5d797a732e5cbe`. Acceptance is for further qualification,
including conservative stale denial and retained-root/copy costs under existing
limits. No new sequence, serialized codec, bearer, numerical pool or source-write
permit is selected. This complements ADRs 0106–0108; none is superseded.

## Consequences

The registry is live admission state, not the durable physical membership index.
It cannot supply native runtime evidence, original publisher truth, a physical
source/revocation cutoff or qualified resource/recovery bounds. Unknown membership
closes the enclosing scope and leaves hierarchy mutation unavailable. Removed or
fresh roots never reopen terminal bindings or restore a credential.

Actual source/control/ordinary owners, complete initializer/event contracts and
hard native/resource evidence remain prerequisites to executable work. No code,
SQL/database/schema/migration, admitted fixture, live source construction, model/
native probe or activation is authorized. Immediate ordinary stop/input remains
independent. Earlier accepted policies and physical/logical representations stand.
