---
type: Validation Evidence
title: Offline fresh report-ledger creation witnesses
description: Separate conditional initial-store and run-admission witnesses for accepted report-ledger initialization, with independent byte/proof checks and explicit producer limitations.
tags: [orkworks, hierarchy, reports, preparation, fixtures, accounting]
status: draft
---

# Offline fresh report-ledger creation witnesses

Tracking: [#1679](https://github.com/Rambolarsen/orkworks/issues/1679),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Accepted contract: [fresh-ledger extension](../superpowers/specs/2026-11-11-report-retention-initialization-design.md#scoped-owner-acceptance--2026-11-12).
Input base: `59ffe0fc79967743c14d99cc2c6792a2a036e1a1`.

**Evidence: finite offline creation/accounting witnesses only.** No real sender,
credential, namespace reservation, store, SQL/native process, lease, clock,
transport or publication worker participates. Passing these checks establishes
neither an admitted run nor Gate 2 completion. All source/capability/authority
limitations of the [original initial-effect packet](root-preparation-effects.md)
remain. In particular, original report/terminal producer legality and complete
consumer/index membership are unqualified.

## Independent artifacts and scope

- `generate-report-ledger-initialization.py` uses Python standard-library JSON and
  SHA-256 only. It reads the original root-preparation-effects.json, never imports
  or invokes the JavaScript verifier, and writes a separate UTF-8/LF oracle.
- `report-ledger-initialization.json` retains eight full conditional states and
  seven commands. Its baselineSha256 binds the exact original catalog bytes.
- `report-ledger-initialization.mjs` checks the baseline object against its trusted
  retained bytes, reuses the original source/effect validators, independently
  derives complete owner-ledger/proof effects and measures changed rows/bytes.
  It is a finite packet checker, not a general store parser or producer API.
- `report-ledger-initialization.test.mjs` runs creation, unchanged-owner, exact
  receipt inspection, regeneration and semantic corruption/limit cases in docs CI.

The new packet is a **separate synthetic history**, not an upgrade of an accepted
store. Existing preparation catalogs/checkers and report/authority components are
unchanged. The original domain-body projections and artifact bytes carry forward;
the new history necessarily has distinct manifests/digests and replaces only the
named outer storage links, preparation receipt commit and checkpoint applied
references with its own commit references. Never retoken an existing stored
receipt or pass these witnesses to a production adapter.

The baseline raw bytes are trusted retained documentation inputs supplied to the
checker, not credentials or a constructor of real authority. Matching their hash
proves input identity only. The inherited context still supplies synthetic UI,
review, reservation and source eligibility, and the original fixture ceilings
still select its two summary branches. This does not establish production
capacity or a maximum valid profile.

## Complete logical command measurements

| Transition | Changed rows | Complete changed logical bytes |
| --- | ---: | ---: |
| New-store initialization | 12 | 6,084 |
| Other run draft preparation (four references) | 10 | 23,697 |
| Other run admission | 19 | 33,978 |
| Target run draft preparation (five references) | 11 | 24,726 |
| Target run admission, initialized summary | 19 | 34,219 |
| Target run admission, never-initialized summary | 16 | 30,017 |
| Exact target draft discard | 11 | 4,713 |

Initialization creates ten workspace zero resources plus header/proof. Admission
adds one run zero report resource to the inherited complete effect. Drafts create
no report resource and discard deletes none. All report ledgers in this packet
have used/reserved zero: no research-report object/head/receipt is created.
Both target admission branches and discard preserve the existing workspace/other
run report ledger bytes and links. The draft's preparation ledger is preserved
and updated by its original owner recipe; it is not reset by report initialization.

Command bytes include canonical keys/full new values and the full proof/header,
including zero resource rows and their manifest members. Deletion charges its
key and actual control effects, with no negative byte credit. Whole logical
inventory/proof bounds and complete 16 MiB/4,096 changed-row ceilings are checked;
exact measured command capacity fits and a smaller byte/row budget denies.
These are logical bytes, not SQL pages, indexes, journal size, RSS, query visits
or native latency. Repeated inherited validator scans are not an instrumented
100,000-row SQL-work proof; finite fixture size cannot qualify physical discovery.

## Rejection and inspection boundaries

Self-consistent rehashed mutations reject missing/extra ledgers, nonzero usage
without any report producer, reserved report credits, unknown fields, substituted
source approval and wrong original creation links. A nine-ledger legacy genesis
does not become report-capable. An orphan report ledger during a draft makes the
packet invalid rather than being discarded. Understated rows/bytes, changed
command kind, wrong baseline input/hash, duplicate states and extra commands reject.
No rejection path writes or repairs any supplied inventory.

Receipt inspection returns the exact receipt in this new history without any
state changes; a discarded/missing source cannot be reconstructed. This is **byte
inspection only**, not an authenticated preparation retry. No current UI/rate/
scope proof is supplied. Nonzero workspace report usage and retry after later
qualified report publication remain unqualified because this initial-only packet
has no legal report-producing command. It does not fill those gaps with fake
usage, a report catalog entry or an asserted producer boolean.

Run from the repository root with Node 22:

```bash
node --test docs/validation/report-ledger-initialization.test.mjs
python3 docs/validation/generate-report-ledger-initialization.py --output /tmp/report-ledger-initialization.json
cmp docs/validation/report-ledger-initialization.json /tmp/report-ledger-initialization.json
pnpm --dir docs docs:build
```

Tests regenerate into an isolated temporary directory and remove it; a copied
generator/baseline pair under a path containing spaces and URL escapes yields
identical bytes. The generator writes explicit bytes and the retained JSON has
LF checkout attributes. No native Windows run is claimed.

## Failing-first evidence and remaining gates

The first three behavioral tests failed against explicit empty generator/verifier
stubs (0/3). Python generation then passed the owner-ledger cardinality test while
the two verifier behaviors still failed. After independent JavaScript checks,
the three passed; the author corrected the draft-count expectation by recounting
the retained four/five reference occurrences (n+6), without changing the contract.
Adversarial cases then exercised the completed checks.

The focused suite has 24 passing tests. A temporary mutation bypassing the supplied
command-byte/row limits caused the capacity regression to fail with a missing
rejection; the original guard was restored. Two bounded fresh read-only Luna
reviews found no remaining issue in correctness or evidence/test completeness.
One reviewer initially ran while an original-link check was temporarily removed
for a mutation assay; after restoration the targeted test passed and the reviewer
withdrew that finding. The clean final suite is used for completion evidence.

Keep #1089 Gate 2 Investigate. #1679's scoped initialization specification and
acceptance prerequisite is separate from real initial/child report publication,
finish/cancel effects and complete consumer/owner-run/index proof. Native/UI/
lease/lifecycle/storage/custody evidence, applicable capability qualification and
separately approved executable plans remain independent. No runtime/native probe,
adapter, migration, production repair or launch is supplied.
