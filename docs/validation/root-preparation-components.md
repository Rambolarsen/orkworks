---
type: Validation Evidence
title: Root preparation component bytes and source-closure audit
description: Exact run and approval component fixtures, with the missing bootstrap and lifecycle codecs that prevent source-valid preparation fixtures.
tags: [orkworks, hierarchy, preparation, validation]
status: draft
---

# Root preparation component bytes and source-closure audit

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Inspection base: `288bcede6a0d8bcd9474ab9decae5215b55102d6`.

These are **component byte fixtures only**, not source-valid bootstrap,
approval/admission, retained-source or transition fixtures. The accepted
[root producer flow](../superpowers/specs/2026-10-25-root-preparation-producer-design.md)
requires complete validated source objects. Inspection found that more source
codecs need closure before those complete fixtures can be constructed honestly.
No new schema, production quota, authority or lifecycle choice is made here.
#740 remains deferred/no-go until November. Existing synthetic bootstrap bindings
and all prior hierarchy catalogs remain unchanged and UNVERIFIED.

## Delivered components

[root-preparation-component-vectors.json](root-preparation-component-vectors.json)
pins the complete accepted OrchestrationRunDefinitionV1 and
RootPreparationApproveRequestV1 shapes, canonical UTF-8 bytes, byte lengths,
run-definition digests and approval-request digests. Expected values were
calculated separately with Python standard-library JSON/SHA-256, then checked
by the JavaScript validator. The inclusive 16 KiB fixture includes its digest
field in the measured complete run object.

| Vector | Boundary exercised |
| --- | --- |
| unicode-lf | Composed accented text, supplementary Unicode and one literal terminal LF |
| decomposed-unicode | Visually equivalent goal bytes remain distinct; no Unicode normalization |
| maximum-counter | String counter 9223372036854775807 and maxParallelChildren 16 |
| run-byte-limit | Complete run exactly 16,384 bytes; one extra goal byte is rejected |

The catalog explicitly lists unbound bootstrap, repository, worktree, draft and
review-context fields. Their repeated hexadecimal values are synthetic test
inputs, not evidence that referenced objects exist, a draft was reviewed or a
context is current. A valid request component can reference a nonexistent draft;
these tests therefore never label it an approved or admitted preparation.

[root-preparation-components.mjs](root-preparation-components.mjs) checks only
these two closed components. It checks the run's epoch/sequence identity and
raw goal digest, domain-separated run/request hashes, inclusive byte limit,
strict fields/versions, IDs, counters and approval-to-run workspace/run equality.
Stored-byte decoding requires canonical equality, rejecting duplicate keys,
noncanonical spellings, malformed UTF-8 and unpaired surrogates. This is a
small documentation checker, not a general wire parser: it supports only the
integer/string/object subset used by these records. It must not be imported by
production or used as a substitute for complete bootstrap/source validation.

Run the focused suite:

```bash
node --test docs/validation/root-preparation-components.test.mjs
```

The documentation check also runs it. Recomputed-digest negative cases isolate
semantic rejection from stale-hash rejection. Other cases exercise mismatched
approval identities, wrong action/version, extra authority fields, missing
fields, malformed digests, invalid counters, unsafe integers, duplicate keys,
invalid UTF-8 and noncanonical stored bytes. All checks run locally without a
model, native probe, database, sidecar, credential or session mutation.

## Source-closure findings

“Closed” below means enough field names/variants are specified to write exact
bytes, not owner acceptance of the broader component or runtime qualification.

| Source/owner | Inspected definition | Missing work before full source-valid fixtures |
| --- | --- | --- |
| Root bootstrap envelope / #741 | [v1 field list](../superpowers/specs/2026-10-04-agent-role-configuration-design.md#run-bound-orchestrator-bootstrap) plus [v2 output amendment](../superpowers/specs/2026-10-04-agent-role-configuration-design.md#placement-versions-and-canonical-bytes) | Top-level envelope is named, but its nested objects must close as below; do not substitute an empty object or an illustrative projection |
| assignment / #741 | [Approved logical records](../superpowers/specs/2026-10-04-agent-role-configuration-design.md#approved-logical-records) describes label, description, inputs, output text, criteria, dependencies and worktree/base/scope bindings | Exact root-specific property names, tagged input/dependency/scope variants, nullable/absent behavior, ordering and their bounds. Preserve the existing explanatory output text and its distinction from structured outputContract; do not invent future child plan/task bindings |
| executableIdentity / #741 | Same section requires canonical executable path/content identity and pins wrapper, interpreter and package identities when applicable | Exact direct/wrapped variants, required fields, path/platform representation, ordering and digest coverage. Sharing the same undefined representation between harness and capability evidence does not close it |
| Snapshot provenance / #741 | Same section names sourceKind/sourceIdentity/sourceRevision and a canonical local source path | Exact local versus nonlocal representation and path property/absence rules; retained source cannot depend on a writer choosing its own key name |
| Capability/model/permissions / #741 | [Capability snapshot](../superpowers/specs/2026-10-04-agent-role-configuration-design.md#immutable-capability-evidence-snapshot) and [permission profiles](../superpowers/specs/2026-10-04-agent-role-configuration-design.md#requested-and-effective-permission-profiles) | Resolve the shared executable/provenance codecs first. Nonempty command/effect/environment, external-source and connector entries also need closed schemas when selected. Empty lists cannot establish an orchestrator's required coordination functionality or verified enforcement |
| Run definition and approval request / #742 producer | [Closed source records](../superpowers/specs/2026-10-25-root-preparation-producer-design.md#closed-source-records) | Component bytes delivered here; complete bootstrap/repository/worktree/draft joins remain absent. There is no evidence of fence reservation, current authentication or frozen review bytes |
| Draft, receipt and root_preparations / #741/#742 | Same closed-source section names their exact envelopes and reconstruction joins | Inline complete bootstrap first; then prove exact draft reconstruction, all repeated identities/digests, original approval request and receipt/root identity. Do not add opaque source blobs or accept a hash-only bootstrap |
| Initial run state and parent attachment / #742 | [Logical records](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#logical-records-and-immutable-parent-configuration) lists logical state; [atomic effects](../superpowers/specs/2026-10-25-root-preparation-producer-design.md#storage-membership-and-atomic-effects) explicitly leaves codecs open | Exact initial-state and reserved/attached-parent record variants, keys, membership proof, generation/null rules, legal transitions and complete changed-row effects; source validity alone cannot imply a parent has started |
| Complete command / #742/#744 storage | Same atomic-effects section and [registry prerequisites](../superpowers/specs/2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation) | Complete initial summary/capacity branch, source/receipt/quota/dependency/lineage changes, shared usage preservation, draft deletion, header/proof and finite aggregate/custody limits. Partial row counts cannot be called full-command measurements |

Repository/worktree descriptors have a separate accepted
[identity contract](../superpowers/specs/2026-10-04-configuration-learning-design.md).
Their bytes and native revalidation evidence still need to be joined; this audit
does not claim that the identity codec itself is missing or that synthetic paths
qualify a platform. Similarly, a structurally closed unverified capability
snapshot remains ineligible; tests must not manufacture a verified decision.

## Bounded handoff

Next, close the root assignment, executable-identity and provenance codecs in
#741 with their source owner, and the initial-state/parent-attachment records in
#742. New schema/default choices require their own written review and acceptance
before dependent implementation. Then supply complete unverified source-byte
fixtures with explicit eligibility rejection, and eligible fixtures only when
real qualifying evidence exists. Bind full draft/request/receipt/retained rows
and complete transition effects before updating any old synthetic seed.

Preserve current authentication before retry lookup, original receipt identity,
revoked-context denial, frozen-byte equality, burned-sequence recovery, no source
reconstruction at/below high-water and shared-ledger usage. Their written oracles
remain in the producer contract; this component checker does not execute them.
Excluded-action authority ordering, native storage/resource qualification, shared
custody, #740 eligibility and separately approved executable plans remain open.
Keep all four component issues open; this deliverable closes no runtime gate.
