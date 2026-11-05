---
type: Architecture Decision
title: Original child preparation may settle after its exact permit
description: Accepted one-action late-saving policy for further design, excluding spawn and all other source authority.
tags: [orkworks, hierarchy, authority, preparation]
status: draft
---

# Original child preparation may settle after its exact permit

- Status: accepted scoped policy for further design; representation/implementation gated
- Deciders: owner, exact “keep going” decision on 2026-11-05
- Date: 2026-11-05
- Tracking: [#1415](https://github.com/Rambolarsen/orkworks/issues/1415), [#1176](https://github.com/Rambolarsen/orkworks/issues/1176)
- Decision: [scoped acceptance](../validation/child-preparation-commit-authority.md#scoped-original-preparation-policy-acceptance--2026-11-05)

## Context

ADR 0106 accepts candidate joint storage atomicity but leaves original source
permission unqualified. The investigation distinguishes a source check before
SQL from revocation-before-physical-commit denial. Holding ordinary/publication
locks or delaying immediate stop through fsync is outside the accepted boundary.
The owner separately chose one exact late-saving/no-spawn exception.

## Decision

Only for original child-source-attach, the eligible original source/lineage owner
may have the sole private worker acquire one immutable exact permit under the
shared short revocation cutoff. Revoke-first denies. Permit-first allows that
already-authorized complete original preparation to reach proven joint commit/
rollback after later volatile revocation. Durable acceptance remains the joint
physical commit, with complete source/header/membership/health/quota validation
and actual lease ownership still required. This is no blanket source-write claim.

This supersedes the stronger original-preparation permission requirement named
in ADR 0106's qualification consequences only for that one command and further
design scope. Its broker, versioned lineage, budgets, availability/retention
choices and other gates remain accepted and unchanged. Other source commands,
spawn and source-admission/runtime authority do not inherit this exception.

## Consequences

Cancellation may leave complete approved but unstarted ordinary/hierarchy
preparation and its original history/charges/capacity. It cannot schedule a child,
issue a grant, refund or erase that state. A permit does not freeze mutable source
eligibility or excuse actual ownership loss. Unknown disposition retains the
original worker/paired resources; no replacement worker or automatic retry.

The exact nonsecret decision codec, proof/lookup/manifest/accounting and version
compatibility require their own reviewed representation. No existing evaluation,
debit or ordinary execution token is reused. The policy acceptance grants no
proof/format approval, fixture/executable plan, SQL/database/migration, source
registration, live capability or native/model probe. The named three architecture
choices and this unchanged one-action policy need no repeated approval.


## Amendment: authorization evidence accepted — 2026-11-05

The owner subsequently accepted [PR #1420's exact authorization representation](../superpowers/specs/2026-11-05-child-preparation-authorization-design.md#scoped-representation-acceptance--2026-11-05)
for further qualification, against reviewed head
`8a63849ac23d4bf09dca7f93e6e418b898c96306` / merge
`8e885c5bcd7d59184f032a1e3c9ee3e7da74ee8d`. This operationalizes the required
nonsecret proof/lookup/manifest/accounting compatibility consequence above; it
does not reverse this policy or broaden its one-action permission.

The separate permit identity and exact intent/snapshot codec, immutable P-keyed
row with permit uniqueness/direct retention, and new-store-only format6 placement
under existing limits are accepted. Earlier representation-pending wording is
historical for those exact choices. Physical catalog/membership, actual source/
ordinary owners, resource/native qualification and separately approved executable
work remain gates. No code, database, migration, new quota/bearer or activation is
authorized. RegistryRoot remains a separate proposal. Tracking: [#1421](https://github.com/Rambolarsen/orkworks/issues/1421).
