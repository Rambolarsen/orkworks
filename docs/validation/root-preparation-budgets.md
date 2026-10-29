---
type: Validation Evidence
title: Root preparation budget boundary components
description: Independent full-state measurements, isolated inclusive-limit comparisons and conservative source/artifact/proof envelope bounds for the accepted initial budget policy.
tags: [orkworks, hierarchy, validation, preparation, budgets, custody]
status: draft
---

# Root preparation budget boundary components

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
The [scoped acceptance](../superpowers/specs/2026-10-29-root-preparation-budgets-design.md#owner-acceptance-and-boundary-fixture-handoff--2026-10-29)
selects numerical policy for further logical qualification only. No native,
authority, physical storage, runtime admission or executable-plan eligibility
follows from this packet.

## Reproduce

```bash
python3 docs/validation/generate-root-preparation-budgets.py > /tmp/root-preparation-budgets.json
cmp /tmp/root-preparation-budgets.json docs/validation/root-preparation-budgets.json
node --test docs/validation/root-preparation-budgets.test.mjs
pnpm --dir docs docs:build
```

Use repository Node 22. The Python stdlib generator is independent of the
JavaScript consumer. The new catalog fingerprints exact bytes of the earlier
root-preparation-effects.json; LF storage preserves its input hash on Windows.
Earlier catalogs, fixture caps, generators/checkers and measurements are unchanged.

## Three distinct kinds of evidence

The catalog contains eight measurements of the existing complete states. Before
summing, the consumer reuses the earlier full closed source/row/proof/ledger
validator with that state's original fixture caps. It then independently derives
preparation charges from rows and exact embedded owners, never quota values or a
selected-run pointer. Full key/value bytes include control rows, zero ledgers,
manifest metadata, commit links, receipts, checkpoint links and summary markers.
Each proof contributes once to the proof total and once within the whole-store
total. The maximum individual run charge and workspace sum are checked separately.
The absent state is an empty measured inventory, not successful initialization.
All states remain synthetic and their bootstrap/native authority unverified.

Twenty-two isolated arithmetic vectors pin all seven inclusive limits below,
at and above the cap, plus simultaneous overflow. Their usage fields are
canonical bounded Counter strings; missing/extra fields, wrong types, noncanonical
spelling and overflow reject. compareBudgetTotals is a pure arithmetic component,
not an admission API and not permission to trust caller-supplied totals. These
vectors are not valid stores: many individual boundaries cannot be reached by
this narrow initial profile before another limit binds. In particular a row-count
sentinel does not fabricate thousands of extra eligible subjects.

Conservative maximum-envelope vectors use the existing source/draft ceilings,
maximum ASCII key widths, 4 KiB membership values and actual artifact/proof wrapper
serialization. They are upper-bound calculations, not new source-valid bootstrap
fixtures. A 1 MiB aggregate split into fifteen one-byte occurrences plus 1,048,561
bytes produces 1,398,144 Base64 bytes, including per-occurrence padding. Actual
binary buffers and strict artifact validation reproduce that maximum; adding
one raw byte fails even with repaired size/digest and still-small encoded rows.
Equal small contents under distinct reference IDs remain separate occurrences.

| Conservative quantity | Canonical bytes / consequence |
| --- | --- |
| Pending run preparation projection | At most 2,488,110 bytes, below 8 MiB |
| Admitted run preparation projection | At most 2,496,756 bytes, below 8 MiB |
| One full artifact wrapper excluding Base64 characters | 361 bytes; even one maximum raw occurrence fits the 2 MiB value ceiling |
| Proof value with 4,096 maximal root-summary change envelopes | 4,067,971 bytes, plus 21-byte sequence key |
| Same manifest plus mandatory new proof/header effects | At least 4,098 changed rows; fails the independent 4,096 complete changed-row cap |

The proof envelope repeats maximal keys and uses maximum-width placeholder
counter tokens. It is deliberately not a valid unique-key manifest, legal
initial-profile command or eligible publication. It bounds structural byte size
only; it proves neither that maximal source values are jointly attainable nor
that all independent maxima coexist. Storage command bytes, source nested limits
and physical/query/native bounds still apply separately.

## Behavior and limits

The tests cover all full-state totals, cancellation's preparation-byte/run-slot
release alongside proof growth, and admission's unchanged run count with increased
source/membership bytes. Both summary branches have the same retained row count:
the initialized branch replaces existing workspace summary-ledger values and
retains larger summary/checkpoint/proof bytes. Its three extra changed rows do
not imply three extra retained keys. Never-initialized markers still count fully.
Missing proofs/ledgers, duplicate keys, unregistered tables and byte drift fail
before a usage total can be trusted.

Exact-fit/overflow comparisons isolate the policy behind draft/admission denial,
32 retained runs including old epochs, 4,096-proof exhaustion even after logical
cancellation releases all run bytes, and whole-store overhead denial. They do not
execute full source-valid near-cap transactions, repeated namespace reservation,
authenticated retry, cancellation races, bounded native scans or crash recovery.
The earlier catalog's command and receipt checks are unchanged; no new runtime
retry authority is inferred from a pure read or equal measured totals.

This completes the component accounting/boundary slice. Full maximum-profile
transaction witnesses, a production profile's bounded native enumeration and
counter/authority revalidation, shared/external custody and finite reservation
history, complete purge/pruning/recovery policy, #740 evidence and separately
approved executable plans remain gates. The no-pruning fixture validator requires
its retained predecessor chain; it is not a generic production chain-retention
requirement. No SQL, model/native probe, migration or database was run.
