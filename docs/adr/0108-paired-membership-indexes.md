---
type: Architecture Decision
title: Paired hierarchy membership uses source-derived physical indexes
description: Accepted physical index, artifact metadata and sparse recovery direction for further qualification without an independent graph authority.
tags: [orkworks, hierarchy, membership, storage, qualification]
status: draft
---

# Paired hierarchy membership uses source-derived physical indexes

- Status: accepted for further qualification; implementation/activation gated
- Deciders: owner, scoped “keep going” decision on 2026-11-05
- Date: 2026-11-05
- Tracking: [#1431](https://github.com/Rambolarsen/orkworks/issues/1431), [#1176](https://github.com/Rambolarsen/orkworks/issues/1176)
- Canonical contract: [physical paired membership design](../superpowers/specs/2026-11-05-paired-membership-index-design.md#scoped-physical-design-acceptance--2026-11-05)

## Context

The accepted logical catalog, joint ordinary lineage and original authorization
records require complete forward/reverse/direct retained membership, full-key
uniqueness and metadata-only artifact discovery. A matching returned row or a
partial test catalog cannot prove absence. Separate graph registration would
create a source/index gap; per-request canonical content scans violate the
artifact precharge boundary and bounded discovery contract.

## Decision

Use the exact physical representation reviewed in #1430 at
`b12837e2114106031a6166b15a0b4aaf48d3b886`, merged
`866ed60d85a90946269cd99b6a7a347374a174ad`, for further qualification:
source-derived membership owner stamps, one exact endpoint relation with reverse
index, fixed identity bindings/contributors and artifact-read metadata projection.
Use its closed physical V2 locator extensions and paired H/G checks. Maintain all
projections with their original owning transaction; they add no logical family,
manifest change, object purpose, source digest domain, allowance or authority.

Require complete source/index correspondence before dependent admission, full-key
absence/uniqueness including history/tombstones, and sparse directly required
origin-proof recovery. Missing/corrupt/unsupported membership fences the pair;
there is no read-triggered content reload, graph rebuild, automatic repair or
old-profile adoption. Preserve existing logical budgets and count actual physical
index/write amplification against the existing physical/resource limits.

## Consequences

Actual producer participation, qualified recovery ownership and achievable native/
resource bounds remain required. Any content-bearing recovery validation needs
its separately approved maintenance owner/budget before artifact read admission;
this decision creates no uncharged content-read cache or recovery allowance.
Ordinary immediate controls remain independent of storage qualification and I/O.

This complements ADRs 0106/0107 and their accepted logical representations; it
supersedes no earlier decision. RegistryRoot remains a separate proposal. No
implementation, SQL/DDL/database/schema/migration, executable fixture/native plan,
new quota/bearer, source registration, model probe or activation is authorized.
