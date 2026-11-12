---
type: Validation Evidence
title: Offline research membership and local effects
description: Independently derived research reference sets, synthetic compact registry joins and finite input/control value comparisons with explicit completeness and history limits.
tags: [orkworks, research, consumers, publication, history, qualification]
status: draft
workflow_status: membership-components-only
---

# Offline research membership and local effects

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689) and
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089).
Inspection base: `bcd651c7b39c9655c844e3cb808bc93408d5c0cc`.

The owner [accepted the three membership choices](../superpowers/specs/2026-11-12-research-membership-publication-design.md#scoped-owner-acceptance-and-offline-component-handoff)
for offline fixtures only. This packet derives finite reference sets from the
unchanged [research prefix packet](research-prefix-components.md), compares local
values, and measures conditional discovery traces. Gate 2 remains Investigate.
It supplies no store, source/index completeness, authority, publisher, reducer,
legal event history or whole-transaction/accounting qualification.

## Retained evidence and independent derivation

- [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-membership.py) reads only the unchanged prefix JSON, implements its own reference recipes and reverse construction, and never invokes JavaScript.
- [JSON oracle](research-membership-components.json) retains seven separate groups, a synthetic report registry join, an independent standalone-source example, four local value comparisons and labelled discovery/owner assumptions.
- [Checker](research-membership-components.mjs) composes the unchanged prefix validator with independent recipe derivation. It reuses the accepted child component's conditional discovery measurement without modifying that helper or its tests.
- [Tests](research-membership-components.test.mjs) regenerate the oracle, rebuild reverse sets independently, mutate semantic joins and compare literal independently measured counts. Copied paths with spaces/percent signs preserve exact UTF-8/LF output.

The prefixFingerprint hashes canonical decoded prefix packaging as a raw
fingerprint, not a new source domain, proof, authority or storage commitment.
The oracle is pretty-printed test packaging. The checker receives decoded
objects and is not a duplicate-aware wire decoder; raw stored canonical bytes
still require the existing strict source decoder. No old codec is widened.

## Separate reference groups

| Group | Consumer entries | Unique target occurrences within each consumer | Reverse targets |
| --- | ---: | ---: | ---: |
| Sources | 11 | 36 | 17 |
| Historical inputs/runtime and source metadata | 8 | 39 | 21 |
| Synthetic report registry | 2 | 6 | 3 |
| Initial control view | 1 | 3 | 3 |
| Proposed view | 5 | 19 | 10 |
| Before-selection view | 7 | 36 | 16 |
| Selected-source view | 7 | 37 | 17 |
| Sum of separate examples | 41 | 176 | 87 |

These are alternative/partial examples, not one retained workspace, a global
graph or a physical index. Reverse sets are derived separately for each group;
flattening views with the same mutable keys is denied. Targets outside a sample
remain source-group/root/proof references, not established complete owner rows.
Canonical tuple deduplication affects reachability only and discounts no retained
field, body, repeated source copy or physical occurrence.

The source group covers the six research kinds plus the existing report kind.
Scope retains both captures; configuration retains all four capability-evidence
occurrences and its scope/plan/root group. The plan retains its original input-2;
assignment retains original approval/reservation/allocation/attachment. The
synthetic report has no evidenceReferences, so its positive recipe covers the
original assignment/scope/runtime group, not a nonempty report-excerpt witness.
The scope still protects its captures. Arbitrary text/paths create no fetch grant.

Historical input-3 retains its own child RuntimeRef and separately the original
input-2/parent RuntimeRef. The composed prefix validator verifies the exact older
assessment bytes and producer. Removing that original row target denies even
when the supplied reverse set is rebuilt consistently. Symbolic own/receipt
CommitRefs are direct targets only; no rowDigest/storage format or proof manifest
is invented. Historical refs are not current credentials.

The proposed view preserves null latestApproval, task reservation and expectation
reservation/selectedReport: no target is fabricated to fill a null. It positively
retains collection/expectation ownership before any report. The selected view
adds exact report retention. An inline report source points from its containing
input/expectation to the report; it does not invent an assignment-evidence-source
object or evaluation-evidence edge. A **separate alternative example** explicitly
models the unchanged standalone #744 source→report relation. It is not inserted
into any of the seven groups or counted among their 41 consumers.

## Synthetic registry and finite effect comparison

The registryComponents request is a new synthetic normalized first-child request
over the prefix's exact report/stream and before-selection live taskVersion 7.
expectedRunVersion and expectedReportHead remain null. It uses the unchanged
request domain, compact head shape, v1 original receipt and closed response.
It is not an accepted request, successful receipt or real report-head row. The
retained R2 expected-version-1 vectors remain unchanged; this does not adopt them
or turn frozen assignment version 2 into live CAS.

Independent canonical UTF-8 measurements are:

| Quantity | Bytes |
| --- | ---: |
| Normalized synthetic request | 1,363 |
| Complete compact head value, including symbolic lastCommit | 356 |
| Complete receipt key/value wrapper | 962 |
| Seven forward-entry arrays, summed with their packaging | 72,528 |
| Local effectComparison packaging | 14,706 |

The checker enforces request 32 KiB, head value 512 bytes and receipt wrapper
2 KiB. These examples do not provide maximum-width witnesses. The larger forward
and comparison sums include repeated packaging/copies; they are not command,
retention-pool or transaction charges, and grant no new memory allowance.

The comparison names the before-selection and selected-source views and contains
three complete value replacements (run, collection, expectation) plus the new
input-3 value. The latter includes its child producer, not just its body. It
compares unchanged plan/task/launch/session locations separately. No function
applies these values to a store. It creates no event transition engine, command
API, report commit or authority result.

The samples retain run 10→11, input 2→3 and collection/expectation 2→3; taskVersion
stays 7 and frozen assignment identity stays 2. Report task/launch/session writes
are denied in the comparison. These numbers are examples, never wire defaults.
Actual report object/head/receipt insertion, resources, custody, indexes,
summary/tracking fences, header/proof and ordinary bridge effects are **not**
completed by four local comparisons. No complete publication count is qualified.

## Conditional discovery and qualification limits

prospectiveOwnerAssumption supplies one local collection/task example. The
checker denies an extra/missing supplied owner for this finite slice. An empty
additional list or matching singleton is not evidence that every real owner was
enumerated. Global source-owner/index correspondence remains unproven.

discoveryAssumption supplies equal symbolic headers and complete=true. These are
explicit fixture assumptions consumed by the unchanged conditional trace helper,
not observations of a database or accepted caller completeness flags in product
code. The three-event example measures 3 observations and 2,362 canonical bytes.
Tests admit 4,096 conditional misses and deny the next; repeated owned copies
reach exactly 16,777,216 cumulative bytes and the next copy denies independently
of observation count. These traces do not prove source coverage, real range
completion, row/physical limits, metadata-only precharge or feasible native work.
Actual limits, including the smaller applicable pools/deadline and 100,000 visited
rows, remain separate obligations. No query, database open or content fetch runs.

The generator/checker began as empty stubs; positive derivation and measurements
failed before implementation. Resealed/consistent mutations include missing,
extra and foreign owners/targets; dropped historical origins; fabricated pending
references; inline/standalone confusion; stale registry request/head links; false
live/frozen CAS; omitted producer/pointer/version changes; unsupported prospective
consumers; and discovery/header/overflow denials.

Pinned Node 22 verification passed all 1,607 documentation tests, including 43
new tests, OKF metadata/index checks and rendered-page/dead-link validation. All
20 OKF regression tests, documentation drift, whitespace and worktree checks
passed. All 545 prior non-Markdown validation artifacts remain byte-identical;
the prior membership proposal and first-report technical bodies are append-only.
Temporary-copy mutations of forward-recipe trust, standalone source validation,
local effect validation, prospective-owner validation and discovery routing were
caught by targeted tests. A further mutation silently truncating discovery at
4,096 was caught by the next-miss denial. Owned files were not mutated for those
checks. Existing VitePress circular/large-chunk warnings concern unchanged
bundler configuration and require no edit for this packet.

One bounded review round used two fresh read-only Luna reviewers. Correctness
found no material join issue. Evidence review identified that the two packaging
byte totals above had been independently measured but were not asserted by the
retained checker/tests. The author added canonical UTF-8 measurements and literal
Python-derived assertions after first observing their failure against the prior
checker. The change adds reproducible measurements, not a quota or publication
claim. Neither reviewer executed code; the author applied this correction after
the single review round. Hosted review covers the final committed head separately.

Use pinned Node 22 from the repository root:

```bash
rtk proxy python3 docs/validation/generate-research-membership.py --output /tmp/research-membership-components.json
rtk proxy cmp docs/validation/research-membership-components.json /tmp/research-membership-components.json
rtk proxy node --test docs/validation/research-membership-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

Every previous non-Markdown validation artifact remains unchanged. This packet
does not select or adopt format-3/4/6 rows, a physical catalog or initializer,
qualified paired index, RegistryRoot, resource balances or old proof history.
Actual source custody/currentness, complete consumer membership, authentication,
child revocation/commit ordering, ordinary bridge, recovery and atomic accounting
remain gates. Broader consumers, correction/terminal histories and runtime plans
are not authorized. No Rust/Electron/native/model execution, adapter, migration
or product module is supplied. #1689 and #1089 remain open.
