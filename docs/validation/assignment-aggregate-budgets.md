---
type: Validation Evidence
title: Assignment aggregate budget boundary components
description: Independent isolated cap and shared-scope arithmetic vectors under the accepted retained record and result payload budget policy.
tags: [orkworks, hierarchy, evaluation, budgets, validation]
status: draft
---

# Assignment aggregate budget boundary components

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
The [scoped owner acceptance](../superpowers/specs/2026-10-29-assignment-aggregate-budgets-design.md#owner-acceptance-and-component-handoff--2026-10-30)
selects numerical/admission policy for documentation components and further
qualification. This packet changes no runtime behavior.

## Reproduce

```bash
python3 docs/validation/generate-assignment-aggregate-budgets.py > /tmp/assignment-aggregate-budgets.json
cmp /tmp/assignment-aggregate-budgets.json docs/validation/assignment-aggregate-budgets.json
node --test docs/validation/assignment-aggregate-budgets.test.mjs
pnpm --dir docs docs:build
```

Use repository Node 22. The Python stdlib generator emits 26 vectors independently
of the JavaScript comparator. LF storage preserves regeneration bytes on Windows.
The docs check includes this test suite; earlier catalogs/checkers are unchanged.

## Evidence boundary

`comparePublicationTotals` adds two closed sets of nonnegative canonical u64 TEXT
values using checked BigInt arithmetic. `records` denotes assignment-record-bytes;
`payloads` denotes result-payload-bytes. Each has assignment, run and workspace
components. These names are documentation inputs, not source/wire schema fields.
It compares all six totals to the inclusive 64/128/256 MiB ceilings, with no pool
borrowing, and returns record-capacity reason precedence when both fail.
Malformed counters, missing/extra fields and addition overflow reject before
any capacity result. The maximum u64 is valid TEXT but exceeds the policy.

Eighteen vectors isolate below/at/above each resource/scope ceiling. Four
hierarchically consistent vectors show one byte failing only at a shared run or
workspace pool although the assignment fits. The other four pin zero delta at
all full pools, simultaneous resource failure and no borrowing from an unused
other pool. Tests also check input immutability, malformed counters, closed fields,
checked overflow and a malformed later field after an earlier capacity overflow.

Inputs are arithmetic quantities, **not trusted caller usage, reconciled stores,
source objects or executable publications**. Isolated boundary vectors need not
satisfy aggregate ownership relationships or all composing storage limits.
The comparator does not validate historical inventory/ledgers, reserved=0,
identity/owner joins, source bodies, operation eligibility, reload corruption,
receipts or authentication. It cannot distinguish a legitimate new overflow from
an already-corrupt over-limit reload; the owning store must fence the latter as
unavailable before using any comparison. No purge or negative delta is modeled.

A zero-delta fit does not establish retry authorization, receipt equality or
writability under metadata/command/whole-store limits. Input immutability proves
only that this pure comparator performs no mutation; it does not demonstrate
transaction rollback, protected-credit preservation or candidate/head retention.
Full history and duplicate payload charging remain accepted policy awaiting
source-traced accounting witnesses. No maximum production profile is qualified.

## Next qualification

Pin complete source-owned deltas and retained inventory for root/child owners,
cross-run evaluation targets, old epochs, corrections/invalidation, duplicate and
zero-byte present payloads versus missing outputs. Demonstrate unchanged complete
state on each capacity denial, protected credits and open-candidate work retained,
exact currently authorized retries and fail-closed corrupt/missing ledgers. Only
then combine maximum source/payload/key/wrapper/proof widths with all command,
whole-store and native limits. Missing source/producer/custody codecs or authority
fences stay blocking; this component packet does not invent them. #740, native
resource/durability, broader component and executable-plan gates remain open.
