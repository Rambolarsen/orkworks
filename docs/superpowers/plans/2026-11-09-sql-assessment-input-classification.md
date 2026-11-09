---
type: Implementation Plan
title: Private SQL assessment input classification
description: Approved private semantic classification and bounded source-data parity without production claim minting or routing.
tags: [orkworks, sqlite, assessments, provenance, plan]
status: draft
workflow_status: implementation-active
---

# Private SQL assessment input classification

Parent: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Spec: [accepted classifier contract](../specs/2026-11-09-sql-assessment-input-classification.md).
Owner accepted after #1642 on 2026-11-09. Root implements inline.
Pass condition: actual typed Unknown refuses a complete set whose identical
value succeeds only as Qualified; exact bounds and existing codec digest parity
are preserved, with no production claim constructor or effects.

## Decisions and ownership

Keep the existing assessment-source owner. Add its private `classification`
child module and direct tests; do not widen the old fixture or OriginalInputs API.
Opaque owned set/input/scope fields have no Deserialize, Clone or production
constructor. Only explicit test utilities construct qualified values/scopes.
Return existing SqlAssessmentSources data, with separate Unqualified, Conflict,
Invalid and TooLarge outcomes; leave SourceError and worker mappings unchanged.

A borrowed serialization view counts the accepted envelope shape with a bounded
writer before value cloning/conversion. This view is internal, not a serialized
claim. Reject above 1 MiB. Consume the set, preserve input order/values, and reuse
the existing source codec only after preflight; its subsequent canonical tree
and encoding are therefore bounded by that cap. No new canonical serializer,
source wire format, dependencies, global budget or SQL/file/application owner.

## Deliverable and checks

- [x] Write valid-shaped semantic tests for Unknown in each input position,
  identical Qualified zero-pair value, qualified empty set, scope/creation
  conflict and complete order/value binding. Observe behavioral RED before logic.
- [x] Implement sealed types and smallest classifier, borrowed byte preflight,
  exact source union and existing codec reuse. Expect those tests GREEN.
- [x] Pin independent canonical vectors and codec parity, 128/129 sources, exact/
  one-over 16 KiB source packet and 1 MiB envelope, including escaping and late
  overflow. Preserve independent 64 KiB record tests. No partial result or trim.
- [ ] Update canonical evidence; run focused classification/source/record tests,
  formatting/build and `bash scripts/verify-repo.sh`; complete current-diff review,
  required/routed CI, recommendation tie-off if matched and owned PR cleanup.

No production constructors, SQL capture/routing, model/evaluator/scheduler,
publication/recovery, deletion owner or migration/activation changes.

## Risks and review disposition

Dependencies2: one existing codec and its consumers; blast2: new private pure
policy only; state1: no effects/schema; reversibility1: no persisted output
change; uncertainty2: exact serializer accounting needs direct boundary tests.
Total8/25 is a preparation judgment, not runtime/physical qualification.
Review once inline: scope is one coupled policy; all contract distinctions and
bounds map to the checks above. Existing approved codec/capture contracts remain
unchanged. Ready for inline execution; no additional owner approval is needed.

Review focus: no semantic upgrade from serialized data, absent-vs-unknown
readiness, exact value/order binding, source-union growth, and byte counting
before full encoding. Root is sole writer; whole-diff read-only review follows
implementation, with at most two fresh reviewers and bounded finding followup.

Implementation note: Unknown has no accepted wire envelope, so it refuses
before serialization. An all-Qualified set receives bounded borrowed accounting
before conversion or source-union allocation. The new error family stays local
to this policy. Seven initial semantic/boundary tests were RED then GREEN.
