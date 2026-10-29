---
type: Validation Evidence
title: Conditional terminal report publication fixtures
description: Independently generated terminal prestates, complete correction row effects, commit proofs and no-commit rejection oracles under explicitly unqualified producer and authority context.
tags: [orkworks, reports, hierarchy, storage, dependencies, validation]
status: draft
---

# Conditional terminal report publication fixtures

Tracking: [#1136](https://github.com/Rambolarsen/orkworks/issues/1136),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744).
Source: [accepted terminal profile](../superpowers/specs/2026-10-29-report-publication-profile-design.md#scoped-owner-acceptance--2026-10-29).
Inspection base: `d605efca282de52b7a5f5e965f555d23c43e8370`.

## Evidence and boundary

[report-publication-vectors.json](report-publication-vectors.json) contains
independent Python standard-library canonical JSON and SHA-256 witnesses.
It reuses the unchanged [R2 source packet](r2-report-components.md), under a
**separate, synthetic terminal context**. Four prestates cover current, cancelled,
maximum revision and never-initialized summaries. Seven accepted transitions cover
required C1, optional O, cancelled C1, reconciling C1, exhausted C1,
never-initialized C1 and a later C1 version. Seven retained no-commit oracles
cover authority/epoch/rate/source/consumer failure, command budget and stale CAS.

This qualifies conditional row/commit/source joins and deterministic assay behavior,
not an admitted terminal run, supported producer or durable publication. Admission,
research plan/configuration, termination settlement, scope/authority/epoch/lease,
rate admission, other-consumer absence and tracked-run membership are supplied
captured inputs, **not proved by booleans**. No real UI sender, credential, clock,
SQL transaction, index adapter, file retention, browser or process participates.

The captured seed proof is hash-consistent but its creation effects are not a
qualified producer transition. Its historical subject-preparation tag is retained
only to exercise format-3 row/proof links; this packet does not assert that such
a command can create terminal state, historical reports or evaluations. The checker
validates seed byte/domain/link and selected source invariants, then independently
derives the new report-publication effects. Do not use the captured seed as a
migration/import/initialization recipe or call its historical command admissible.
A complete historical producer proof remains a separate prerequisite.

```bash
python3 docs/validation/generate-report-publication-vectors.py --output /tmp/report-publication-vectors.json
cmp docs/validation/report-publication-vectors.json /tmp/report-publication-vectors.json
node --test docs/validation/report-publication-components.test.mjs
pnpm --dir docs docs:build
```

## Exact witnesses

The generator retains complete sorted `{table,key,body}` inventories and format-3
commit manifests/proofs. Row commitments omit only the accepted outer links and
named checkpoint/pending paths; inventory fingerprints include full links and
header/proof bytes. The verifier reconstructs the complete expected changed set,
request/receipt/source joins, checked run/subject/material counters and shared
resource deltas. Unknown rows or unexplained links are rejected; unrelated immutable
objects and retained edges remain byte-identical. No generic #744 receipt slot or
claim-first authority is borrowed for report publication.

| Conditional transition | Changed rows including header/proof | Complete command bytes | Material / subject revision |
| --- | ---: | ---: | --- |
| Required C1 | 11 | 15,145 | 3→4 / 10→11 |
| Optional O | 9 | 14,400 | 3→3 / 10→11 |
| Cancelled run, C1 | 11 | 15,145 | 3→4 / 10→11; lifecycle remains cancelled |
| C1 with current-row capacity unavailable | 13 | 18,923 | row fenced at 3; exact pending transition assigned 4 / 10→11 |
| C1 at maximum material revision | 10 | 13,223 | terminal exhausted fence at 2147483647 / 10→11 |
| C1 with never-initialized summary | 9 | 10,118 | no row/checkpoint is invented / 10→11 |
| Later C1 version 3 after v2 | 8 | 9,023 | 4→4 / 11→11; original cited v1 was already stale |

The same immutable evaluation remains historical after both original corrections;
C2 retains Needs rework. C1 removes only f-c1 from current material. Optional O
retains both required findings and material revision, while changing a selected
source-currentness tuple and therefore the accepted subject CAS counter.
Both invalidate the single captured owner/source run snapshot exactly once.
A later report version still changes the source run's report detail, even when
that already-stale cited source and material do not change again.

Maximum-width field components use a valid 84-byte epoch/sequence run ID and
maximum bounded IDs/counters. The full receipt key plus value is **1,925 bytes**,
including the complete expected head, response and outer CommitRef; it fits 2 KiB.
The complete head value fits 512 bytes. These are component widths, not a complete
eight-version stream or maximum command. Real command measurements include full
keys, row values, new source bytes, summary/checkpoint/pending duplication, ledgers,
manifest/proof and header. No general maximum fanout/physical fit is established.

## Rejection, retry and logical work

The suite first ran against explicit no-validation stubs and failed. The verifier
then passed positive state/commit oracles and negative source/effect checks. A
separate failing-first regression exposed inventory cardinality as insufficient
work accounting; the completed assay shares one bounded row-read counter across
repeated inventory scans and final validation. This is conservative logical assay
work, not SQLite VM instructions, physical query cost or a native deadline claim.
All reads of query inventory rows are charged; in-memory comparisons of already
captured manifest contents are not additional database rows.

Schema, source size/domain, fixed report ID, contiguous shared eight-version
history, expected head/run CAS and exact canonical retry bytes are checked.
Revoked/rate-denied captured context cannot return a saved receipt. An authorized
exact v2 retry after v3 returns the original receipt before mutable CAS, preserving
all state and causing no reinvalidation. Rate charging itself is supplied context;
no rolling-minute limiter is implemented or qualified by this assay.

Adversarial tests rebind the captured outer row/manifest proofs before checking
missing/unsupported retained edges, receipt request drift, incorrect source size,
quota mismatch and unknown terminal fields. Their rejection is therefore not
merely a stale outer fingerprint. Other tests reject omitted/extra command effects,
retokened material, link-only churn, altered command totals and missing tracked-run
invalidation membership. Non-accepted oracles require complete unchanged durable
inventories, equal fingerprints, null commit/response and no invalidation. Corrupt
prestate is preserved and remains unavailable rather than silently repaired.

Test-only lower admission ceilings exercise command bytes/rows, logical visits,
run/workspace report bytes and durable lineage protection. They are fault limits
within the accepted maxima, not new product defaults or proof of a full workspace.
The selected packet has no other preparation occurrences; its captured extra
non-report preparation usage is zero and does not prove real source completeness.

The exact retained two-hop graph is checked for this one evaluation and three
report streams. Unsupported consumers reject; supplied inventory completeness
and absence of any unclosed proposal/plan/learning consumer remain unqualified.
Physical index recovery and complete production caller discovery are not simulated.
Read-fence tests pin stale-reference for a changed snapshot and unavailable for
missing proof; they do not implement renderer presentation or viewed marking.

## Remaining handoff

The next boundary is the complete producer/authority qualification plan: how real
admission and termination establish this retained terminal state, how source-owned
consumer inventories and index recovery prove completeness, and how #742 UI/epoch/
lease/revocation ordering reaches durable disposition. The #744 claim-first exception
remains excluded. Active preparation/plan/launch/learning consumers require their
own reviewed effects, not an expansion inferred from these fixtures.

Native SQLite/OS/flush/crash/resource evidence, full source custody, #740 eligibility
and separately approved executable runtime plans remain gates. No live database,
migration, native/model probe or runtime route is delivered. Keep broader
#742/#744/#746/#1089 open. #1136's scoped specification/acceptance prerequisite can
be tied off independently of those production obligations.

## Verification and bounded review

All 79 new tests and the complete 539-test documentation suite passed under
pinned Node 22, along with the metadata/index/dead-link build. Independent Python
regeneration/cmp and a temporary core.autocrlf=true checkout preserved exact packet
bytes. The original R2 catalog/checker/tests remain unchanged; six owning contract/
audit bodies preserve baseline bytes with appended acceptance/evidence routing only.
Full verify-repo was attempted and stops at missing desktop dependencies; no broad
Rust/desktop test result is claimed. Existing circular/large chunk build warnings
are unrelated to this diff. Whitespace/doc/worktree checks passed.

Two fresh bounded read-only reviews covered correctness and test/evidence scope.
Neither found a concrete important defect. The correctness reviewer separately
ran all 79 tests successfully; the coverage reviewer did not run tests. Both kept
the fixed-R2/single-captured-run limitation explicit. No general eligible-evaluation
producer discovery, authoritative inventory/index completeness or native atomicity
is established by their review or the passing suite.
