---
type: Design Proposal
title: Keep SDK collection unavailable after an unclassified name
description: Proposed bounded continuation after a DisplayName decoder refusal, retaining valid SDK observations without declaring a complete or successful inventory.
tags: [orkworks, hierarchy, sdk, qualification, evidence]
status: draft
workflow_status: proposed-awaiting-owner
---

# Keep SDK collection unavailable after an unclassified name

Outcome: obtain more useful bounded metadata after a name-format refusal while
preserving the refusal's uncertainty. Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Inspection base: 31265769a2b316ddfba04901d189cbf0311b3ca4.
**Proposed policy, not implementation or execution authority.** Merging this
text does not change a consumed controller, accept new receipt bytes, open a
runner allowance or qualify a build. Existing acceptances remain in their scope.

## Evidence and selected tradeoff

The [source-only audit](../../validation/pair-resource-enforcement-coverage.md#consequence-for-the-next-decision)
places the spent run's decoder refusal at the25th first-root DisplayName visit,
conditional on sealed source and counters. It recovered no key, string, type,
length or formatting subtype. That ordinal is not a stable selector. Strict
ASCII admission precedes SDK matching, so an unclassified name can block later
SDK records without proving that the unknown name belongs to an SDK.

Author recommendation: defer the final unavailable disposition for this one
field's local decoder refusal, continue within the same remaining limits, and
retain only independently valid SDK metadata. **Any unclassified name still
keeps overall success false.** Unknown is never converted into a nonmatch.
The cost is additional bounded reads after success has already become impossible;
a cap or later hard error may still stop before a useful SDK record is found.
A valid retained SDK key could ground a later exact-selector proposal instead of
guessing a product code. Its observation would not make that key a permanent
identity, repair the old run or authorize a subsequent lookup.

Alternatives: fail-fast preserves the current method but may repeat its evidence
gap. A named positive-record selector could read less, but no exact key was
retained; neither the ordinal nor another release's product code supplies it.
Allowing unknown names while returning success would weaken the completeness
claim. This proposal selects none of those alternatives and relaxes no decoder
predicate or selected-SDK field requirement.

## Field-specific continuation rule

Only a successful, in-bound DisplayName native read followed by the existing
local `value-format` decoder refusal is eligible. Preserve fixed buffers,
read-only parent-handle binding and the original decoder's type, length, UTF-16,
termination, nonempty and printable-ASCII rules. Do not decode malformed data
again, resize, retry or request another value to classify it.

For that eligible refusal, consume the normal slot/charges, increment this pass's
unclassified-name count and continue to the next admitted child. Do not attempt
DisplayVersion for that child: its SDK membership is unknown. Close its handle
normally. Missing optional DisplayName retains the existing explicit-absence
handling; it is not a decoder refusal or new evidence about installation identity.

Every other failure keeps its existing terminal behavior, including native
query errors or oversized native results, missing/disappearing child, invalid
KitsRoot10, any matched-SDK DisplayVersion decoder refusal, budget/deadline,
handle closure, membership/marker drift and authority/history/source failures.
A generic catch-and-continue is prohibited. No selector, field, API call, queue,
worker, fallback, repair, reserved slot or future credit is added.

## Proposed partial representation and truth limits

Use a distinct **inventory schema2** for this method. Reuse the existing bounded
inventory fields and SDK product/root shapes, with these exact additions:

- Every pass carries `unclassified_display_names`, integer0..1024, counting
  eligible refusals in that pass. Counts are not refunded and do not reset caps.
- After a pass completes all admitted positions and original before/after checks,
  status is `partial` when that count is positive, otherwise `complete`.
  An interrupted pass stays `collecting`; no full-pass conclusion follows.
- A positive count in either pass forces top-level `success:false`. If no later
  hard failure or comparison drift wins, retain reason `value-format`.
  Do not replace this with sdk-service/absence merely because no match was found.

Keep all valid matched SDK name/version/key records under the existing bounds;
their presence means only that those metadata records were observed. It proves
neither uniqueness/completeness nor SDK authenticity, helper closure, same-run
compiler co-availability, an accepted replacement pin or build eligibility.
A report with no matched SDK record and any unclassified name cannot prove absence.

Unclassified key names, raw values, lengths, types, character/code-point samples,
per-value hashes and exception text are never exported. Add only a fixed
unclassified marker to that entry's existing ephemeral sorted comparison row.
The complete comparison digest may commit to the already admitted key and marker,
just as current rows commit to their keys; no new raw opaque payload is hashed.
Rows stay in bounded owned memory and are discarded after the comparison.

Both planned passes still use the original counts/markers and sorted-row digest
checks. A changed key, classification, known value or SDK record denies. Equal
partial digests do **not** establish unchanged opaque bytes: an unclassified
payload could change while remaining unclassified. The result must state that
limit and never call partial evidence a stable complete inventory or an atomic
registry snapshot. No third pass or scan-until-stable is permitted.

A schema2 reader must explicitly project the new count/status and enforce the
success latch before retaining output. It rejects unsupported versions, unknown
variants, invalid counts or inconsistent partial/complete/success combinations.
The old schema1 receipt remains immutable and is never upgraded/reinterpreted.
Do not pass schema2 through the old projector and silently discard its uncertainty.
The outer run/evidence envelope and existing privacy/byte limits are unchanged.

## Budgets, denial and examples

Reuse the previous method's limits as upper bounds for a future design: at most
two passes,1024 child-name visits per pass/2048 total,32 SDK matches per pass,
2114 values,10000 registry calls,4MiB charged metadata,8MiB total hash work,
60-second collector and256KiB evidence. Count all refusals and ephemeral markers;
reserve before I/O/hash/output as before. No quota increase or fit guarantee.
An eligible refusal permits only work already within those same caps.

| Synthetic situation | Required result |
| --- | --- |
| One unclassified name, later valid expected SDK record, unchanged bounded second pass | Retain valid record and positive per-pass count; partial passes, success false/value-format |
| Unclassified names and no valid SDK record | Unavailable; no SDK absence or complete-inventory claim |
| Unclassified DisplayName becomes valid or its key changes on pass2 | Comparison drift, unavailable; no third pass |
| Matched SDK name then malformed/missing version, native error, close failure or cap exhaustion | Terminal unavailable under the existing owning failure; no further read/retry |
| No unclassified names and all original positive/negative checks pass | Existing success semantics, represented explicitly under schema2 |

The SDK acquisition batch remains spent1/1, with every unused slot closed. These
upper bounds are not a new allowance. Any later live request needs a separate
exact frozen-source/tooling/operation/retention decision; none is requested here.

## Review and handoff

Owner decision requested: accept or revise the field-specific deferred-unavailable
rule, schema2 count/partial/success semantics and privacy/comparison limits above
**for further design and offline components only**. No live work follows.

After acceptance, a separately owned offline task may implement/test these rules
with synthetic registry inputs, including one hard-stop case for every excluded
failure and preservation of all original caps. Prove the success latch, privacy,
version rejection and limited partial-digest meaning with independent negative
controls. Keep spent controllers/workflows/history byte-identical; do not wire
this method into a runner or reuse their allowance. Reviewed future tooling and
an exact new finite execution decision remain separate gates.

#1176 remains open and Gate1 Investigate. Ordinary compatibility, source/cutoff,
bootstrap/allocator/VFS/context, initializer/recovery/durability and production
activation are unaffected. This policy does not diagnose the old subtype or
require another vote on unchanged earlier contracts.

Review disposition: **Ready for scoped owner review; unaccepted.** One bounded
round used two fresh read-only Luna reviewers, root as sole writer. Correctness
found no material contradiction; builder/completeness found no consequential
unanswered question. The schema1-projector incompatibility remains an explicit
implementation boundary, not a qualified schema2 reader. No second review round,
policy acceptance, component execution or new live allowance is claimed.
