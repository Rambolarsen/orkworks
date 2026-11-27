---
type: Evidence Acquisition Proposal
title: One positive Windows SDK product observation
description: Proposed bounded handling of unclassified registry names for a stable positive SDK record without inventory completeness or runtime qualification.
tags: [orkworks, hierarchy, windows, sdk, qualification]
status: draft
workflow_status: proposed-awaiting-owner-approval
---

# One positive Windows SDK product observation

Outcome: observe a stable positive SDK product record in two bounded passes, or
retain one terminal unavailable result. Tracking: [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
The [source review](../../validation/pair-resource-enforcement-coverage.md#sdk-value-format-source-review-2026-11-27)
identified a local DisplayName admission failure without identifying its exact
key or format defect. All earlier runner allowances remain spent.

This proposal requests one combined owner decision covering the changed evidence
contract, its isolated tooling, and the exact finite job below. Merging this
proposal alone authorizes neither implementation nor dispatch. The
[resource qualification gates](2026-11-24-pair-resource-assurance-qualification.md)
remain unchanged; this is evidence acquisition under the existing investigation,
not product activation or a new accepted compiler pin.

## Decision and evidence meaning

Recommend a **positive product observation** rather than a complete classified
uninstall inventory. Preserve strict ASCII admission for the SDK record itself;
represent an unrelated locally undecodable DisplayName as unclassified. Such an
entry might be another SDK record. Neither unclassified nor a literal predicate
nonmatch establishes semantic absence of SDK software.

Keeping the old all-values admission would repeat the known failure mode.
Decoding or repairing arbitrary names would broaden parsing and interpretation.
A direct product-key lookup remains unavailable because no exact key was retained.
The selected approach changes only how this new observation treats bounded,
successfully queried names that fail the local decoder.

Use scripts/qualification/pair-windows-sdk-positive-record.mjs and its own .py
collector, plus .github/workflows/pair-windows-sdk-positive-record.yml, separate
from every consumed collector and workflow. Use schema version2 and artifact `pair-windows-sdk-positive-record`
with `result.json`, `positive-record.json`, and `diagnostics.txt`. A successful
receipt must explicitly contain:

- evidence_kind: sdk-positive-product-observation.
- positive_record_observed: true.
- inventory_complete: false, including when no unclassified names were observed.
- runtime_qualified: false and replacement_pin_accepted: false.

Failure never sets positive_record_observed true. Consumers must discriminate
this schema and evidence_kind; the old inventory success predicate cannot accept
it. No complete-inventory, SDK-absence, compiler/SDK co-availability, authenticity,
helper closure, build readiness, compatibility, recovery or durability claim is
established. Known conflicting versions still deny; unknown entries leave an
explicit limitation even when a positive record is stable.

## Registry handling and stability

Reuse only the [previous exact registry surface and runtime](2026-11-26-pair-sdk-servicing-inventory.md#exact-registry-surface-and-controller):
local HKLM64, the two immediate uninstall roots, and KitsRoot10. Open children
relative to held parent handles. Preserve strict child-component validation,
case-insensitive duplicate rejection and link/namespace handling. Require KitsRoot10 to normalize to
C:/Program Files (x86)/Windows Kits/10/. No recursion, command values, filesystem
follow-through, alternate view, expansion, normalization of names, fallback,
repair, resized buffers or retry.

After the existing fixed-buffer Win32 status and length checks, classify each
DisplayName with one deterministic outcome:

| Outcome | Meaning and handling |
| --- | --- |
| absent_name | Optional value returned ERROR_FILE_NOT_FOUND; do not infer undefined output type or bytes. |
| accepted_ascii | Existing nonempty printable ASCII REG_SZ decoder succeeds; apply the case-insensitive literal Windows Software Development Kit predicate. Record predicate match/nonmatch privately. |
| unclassified | Successful bounded query fails that decoder; retain only an aggregate reason count and private stability commitment. |

Use deterministic first-failure reasons in decoder order: unsupported_type,
invalid_length, invalid_utf16, invalid_termination (missing final or embedded
NUL), empty, non_ascii_or_nonprintable. These are local admission reasons, not
claims that Windows considers the value invalid. Query errors, ERROR_MORE_DATA,
oversize data, namespace/identity errors, budget/deadline failures and failed
closes remain terminal. Do not hash undefined buffers or fields on API failure.
The [Win32 API contract](https://learn.microsoft.com/en-us/windows/win32/api/winreg/nf-winreg-regqueryvalueexw)
does not guarantee a terminated string on a successful query.

Only a fully admitted matching DisplayName permits a DisplayVersion query.
That version retains the old strict decoder and128-character limit; any failure
is terminal. Require at least one stable matching product with exact servicing
version10.1.26100.9457; preserve the maximum32 candidates and known conflicting
same-name version rejection. No candidate or no exact version is unavailable,
never proof of absence. DisplayName remains capped at256 characters.

Each pass privately commits to root identity, exact child component, outcome,
predicate result, and defined successful query type/returned byte count plus a
SHA-256 of the exact returned bytes. Hash every successful bounded DisplayName,
queried DisplayVersion and KitsRoot10 payload, including unclassified payloads.
Absent values use a distinct marker. No unrelated child name, raw value or
individual payload digest is exported or placed in diagnostics. Only aggregate
reason counts, pass digests and matching public SDK key/name/version records are
retained. Discard private rows and buffers after collection.

Perform exactly two planned passes. Require unchanged before/after root counts
and write markers within each pass, identical canonical membership/value digests
and matching product records across passes, and equal classification counts.
Sort canonical rows deterministically; enumeration order is not an identity.
A changed unclassified payload of the same type and length must change the pass
digest and deny success. Any detected drift is terminal; there is no third pass.

## Exact requested allowance

One manual dispatch at a frozen reviewed merged main SHA, one windows-2022 X64
job with a three-minute timeout, one controller and one isolated CPython3.12.10
collector at C:/hostedtoolcache/windows/Python/3.12.10/x64/python.exe with
-I -S -B. Collector deadline60 seconds. No interpreter search, installation,
package download or inspected-tool execution. Standard CI may use fakes and
isolated HKCU fixtures only; it must not query the installed SDK inventory.

Preserve maximum1,024 aggregate child entries per pass, two passes (2,048 enum
calls), at most2,114 value queries,10,000 registry API calls including reserved
close work and failures, and4 MiB precharged metadata. Reserve all charges before
work. Retain the fixed read sizes:514-byte DisplayName/KitsRoot10 payload buffers,
258-byte DisplayVersion buffers, and existing name/metadata charges. No capacity
increase is authorized by this proposal.

The8 MiB total hashing allowance includes all new payload commitments. Enforce
these subordinate ceilings, with any excess terminal:

| Hash input | Maximum bytes across the job |
| --- | ---: |
| Successful raw values: 2,048 names,64 versions,2 KitsRoot10 values | 1,070,212 |
| Canonical observation rows: at most2,048 rows, each at most2,048 encoded bytes | 4,194,304 |
| Canonical pass envelopes including matching products | 262,144 |
| Two source-sealing passes, each at most1 MiB total | 2,097,152 |
| Retained observation bytes hashed by the controller | 262,144 |
| Total | 7,885,956 |

This leaves502,652 bytes below8,388,608; the shared ledger still charges every
hash input and rejects overflow. Use payload digests in canonical rows rather
than duplicating DisplayName/version text there. Enforce the encoded row ceiling
before hashing; do not rely on character counts or assume key names are ASCII.
The pass envelope cap is128 KiB per pass. A source-seal pass covers the new own
controller/collector and any imported guard helpers; all source bytes combined
must fit1 MiB, with fixed identity/size checks before and after bounded reads.

Combined evidence remains256 KiB, retained14 days, three fixed files only.
Keep stdout192 KiB and stderr8 KiB limits and bounded publication; overflow must
produce a small unavailable result, never truncated success. Do not retain raw
stderr. No compiler/native tool, fetch, Cargo, build, target execution, PE reads,
directory inventory, installer or runner repair is allowed.

A new one-shot concurrency/history guard, with unique concurrency group
pair-sdk-positive-record-allowance-2026-11-27 and cancellation disabled, must
reject an existing attempt before collection. Restrict admission to this exact
repository/manual workflow on main, expected SHA and attempt1, using read-only
contents/actions permissions and checkout without persisted credentials. Freeze the exact reviewed head and source identities on #1176 before
the sole dispatch. Any attempt, including preflight or infrastructure failure,
spends1/1; zero reruns, replacement jobs, alternate images/selectors or quota
increases. Pre-scratch/checkout/runner/storage failure may leave only an Actions
unavailable record; do not invent receipt fields or promise an artifact.
Cleanup only owned working scratch; preserve the terminal receipt until expiry.

## Delivery and checks after approval

One owner implements new isolated controller, collector, workflow and synthetic
tests; existing consumed files and historical receipts remain unchanged. Tests
must demonstrate:

- An earlier unclassified name can coexist with a later stable exact SDK record;
  positive observation succeeds while inventory_complete remains false.
- Absent, predicate nonmatch and every unclassified reason remain distinct;
  unrelated names, raw bytes and individual digests never escape projection.
- Same-length changed raw bytes deny stability; valid SDK name with invalid
  DisplayVersion remains terminal; unknown-only and conflicting-version inputs
  cannot succeed.
- Boundaries1,024/1,025 entries,32/33 candidates, encoded rows, pass envelopes,
  source/read/hash/API/output/time caps, parent-relative opens and failed-close
  accounting deny without extra work. Charge payload hashing before hashing.
- Fixed runtime selector, Windows ABI and controller orchestration pass on
  windows-2022 using fakes/isolated HKCU fixtures without installed SDK reads.
- New schema cannot be admitted as old complete inventory; failure publication
  cannot retain a success flag. One-shot history/concurrency guards deny reuse.

Run required repository verification, the current-diff code review and all
required CI before tooling merge. After approval and tooling merge, freeze,
dispatch once, independently check bounded artifact hashes/flags/source/counters,
record success or terminal unavailable in the canonical coverage document, close
the allowance and clean up the owned worktree. Qualification gates remain closed.

## Plan review and authorization

Disposition: **Ready for owner decision; implementation and dispatch blocked on
that decision.** Scope and checks cover the changed admission meaning, private
stability inputs, resource limits and terminal recovery. The hash ceiling has an
explicit worst-case bound rather than a larger quota.

Risk assessment: dependencies2 (existing guard imports and hosted runtime), blast
radius1 (isolated evidence workflow), state changes1 (additive artifacts),
reversibility1 (disable unused new workflow; executed allowance stays spent),
uncertainty3 (future image contents/runtime and positive record remain unknown).
Total8/25. Missing positive evidence remains an acceptable unavailable outcome.
No unresolved design choice is delegated to a live runner. Owner approval must
cover this contract, tooling and the single exact job together; a later scope
change needs a new decision before dependent work.
