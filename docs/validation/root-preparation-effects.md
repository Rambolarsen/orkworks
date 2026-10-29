---
type: Validation Evidence
title: Root preparation logical effect witnesses
description: Independent synthetic full-state maps, manifests, protected source joins and shared accounting for initial root preparation and both summary branches without runtime admission evidence.
tags: [orkworks, hierarchy, preparation, effects, fixtures, accounting]
status: draft
---

# Root preparation logical effect witnesses

Tracking: [#741](https://github.com/Rambolarsen/orkworks/issues/741),
[#742](https://github.com/Rambolarsen/orkworks/issues/742),
[#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
Source: [accepted initial-effects choices](../superpowers/specs/2026-10-28-root-preparation-effects-design.md#owner-acceptance-for-documentation-fixtures--2026-10-29).

## Packet and independent reproduction

[root-preparation-effects.json](root-preparation-effects.json) retains eight
complete logical states and seven commands. `generate-root-preparation-effects.py`
uses Python standard-library canonical JSON, Base64 and SHA-256 independently of
the JavaScript checker. It pins the exact file fingerprints of the existing
[bootstrap](root-bootstrap-vectors.json) and
[preparation-join](root-preparation-join-vectors.json) catalogs and derives two
fresh synthetic root cases, without changing or upgrading either earlier catalog.
All capability snapshots stay unverified. Every native/session/settings/artifact
identity and source is synthetic; no live path, model, database or credential is read.

The prefix initializes all nine workspace resources, prepares and logically admits
an unrelated earlier-epoch root, then prepares the target draft. Three independent
branches admit the target with initialized summary, admit it with a never-initialized
marker, or cancel its pending draft. They fork from identical target-draft bytes;
they are alternatives, not a sequence that reuses a burned target identity.
The unrelated source, membership, artifact and run-ledger bytes are retained
unchanged through all target branches. Shared workspace bytes/counts remain nonzero.

The target contains five capability-reference occurrences, including two distinct
IDs with identical gap/source bytes. Both retain separate Base64 rows and charges.
The earlier root has four references. Artifact links remain on the original draft
commit after source admission, and the required old proofs stay retained.
The complete stored initialInput, initial run state and reserved parent all join
the original frozen bootstrap/run/approval bytes. No generation or parent process
is supplied by the reserved attachment.

From the repository root with Node 22:

```bash
python3 docs/validation/generate-root-preparation-effects.py --output /tmp/root-preparation-effects.json
cmp docs/validation/root-preparation-effects.json /tmp/root-preparation-effects.json
node --test docs/validation/root-preparation-effects.test.mjs
pnpm --dir docs docs:check
```

## What the consumer checks

[root-preparation-effects.mjs](root-preparation-effects.mjs) is documentation-only.
It reuses the accepted bootstrap, preparation-join and initial-codec validators.
It checks strict canonical stored key/value bytes, closed table/value membership,
exact original source identities, per-reference byte/hash/length and prefix closure,
initial state/parent/summary variants, and complete workspace/run/assignment ledgers.
Full-state fingerprints and every row-domain commitment are recomputed against
independent Python values. Native scope and semantic evidence remain unverified.

For each transition, it derives the complete expected data map from the exact
pre-state, source intent and fixture capacity schedule. It compares that map
with the actual after-state and independently recomputes the complete before/after
manifest difference, both commitments, all outer links, header/new proof and exact
command-byte/changed-row totals. A matching hash cannot excuse an omitted row,
reset shared usage, moved source owner, link-only rewrite or partial admission.
The finite no-pruning packet also resolves every predecessor edge to the exact
retained prior proof. This is not a new production rule to retain an unbounded
chain; the broader store's proven-unreferenced pruning contract is unchanged.
All occurrences count; deletion charges its key and actual control/proof effects,
with no negative command-byte credit. These are canonical logical charges,
not SQLite pages, physical BLOB encoding, journal, memory or I/O measurements.

Receipt inspection returns the original retained canonical receipt without
changing the logical map. This is inspection only: the checker accepts no
simulated authorization flag and performs no current-authority mutation retry.
Likewise, empty maps stand only for the explicit new-store witness; missing or
corrupt live metadata cannot be treated as an empty store by this packet.

[root-preparation-effects.test.mjs](root-preparation-effects.test.mjs) compares the
independent states/commands and deliberate corruptions. Tests repair the latest
manifest, links, row commitments and state fingerprint for fabricated passes,
changed initial manifests/runtime, reset usage and substituted frozen source
identities, so semantic rejection cannot rely only on a stale hash. Other cases
cover missing/duplicate artifacts, missing shared ledgers/proofs/parent membership,
changed original approval/source bytes, wrong before/after commitments, partial
or under-budgeted commands, capacity-derived branch selection, exact byte ceilings,
binary artifact bytes and separately charged equal-content occurrences.
The initial no-validation stub failed the golden and rejection tests before
implementation. Existing catalogs/checkers and baseline source bodies are preserved.

## Exact measured logical commands

| Resulting state | Changed rows | Complete command bytes |
| --- | ---: | ---: |
| initialized | 11 | 5,539 |
| other-draft | 10 | 23,697 |
| other-admitted | 18 | 33,291 |
| target-draft | 11 | 24,726 |
| target-admitted | 18 | 33,532 |
| target-never-initialized | 15 | 29,330 |
| target-cancelled | 11 | 4,713 |

The unavailable branch is selected by an explicit fixture-only summary-count
ceiling of one, already occupied by the earlier root. All four workspace summary
ledgers are preserved for that target branch; the new marker has row/checkpoint
null and no invented summary revision. The fitting branch adds the exact initial
row and checkpoint and three summary-ledger deltas. Both branches include the
entire source, membership, marker, zero-ledger, preparation-byte, header and proof set.
All command amounts above fit the existing proposed 16 MiB/4,096 logical ceilings;
no maximum-size/native/production profile follows from these small measurements.

The complete cap schedule is retained under classification
`fixture-only-not-production`: preparation bytes are 16 MiB per run and 32 MiB
per workspace, retained-run count is eight; the unavailable branch uses one summary
row instead of the fitting comparison's 4,096. Summary byte/lineage/pending and
receipt comparisons retain their existing bounded values. Empty-output record/payload
comparisons are 1 MiB/zero. These are finite witness comparisons, never production
settings or acceptance of missing aggregate/history/shared-custody defaults.

## Qualification boundaries and next work

Success remains `synthetic-logical-effects-only-not-runtime-admission`. Manifest,
row, source and ledger coherence does not prove that the user reviewed those
bytes, authority was current, an admission sequence was durably reserved, source
capture was permitted, or SQL committed them atomically. No authority token,
claimed-mutation proof or current parent runtime is fabricated. Stored unverified
capability snapshots remain ineligible; this packet cannot promote old source
seeds, grant permissions or establish adapter/source/native eligibility.

Native cancellation/approval races, crash recovery, durable fence/high-water and
burned identity enforcement, excluded-action revocation cutoff, source retirement,
physical/query/resource limits, cross-workspace custody, production preparation/
history/run-count caps and later parent startup/lifecycle remain separate gates.
A missing ledger/proof or partial map is rejected logically; no native crash or
race is executed here. Same-user rollback/authenticity guarantees are not claimed.

Next, close the finite production preparation/source-history/custody limits and
qualify the excluded-action authority/native boundary before any executable plan.
Keep #741/#742/#744/#746 and #740's November/deferred no-go gate open. No migration,
DDL, runtime API, model/native probe or production launch is delivered.


## Bounded review disposition

One read-only correctness pass found no material contradiction in the selected
initial effects and reproduced the then-current 60 tests. A separate completeness
pass identified that standalone state validation checked predecessor syntax and
adjacent sequence but did not resolve the predecessor digest. The author reproduced
a fully rehashed forged edge, added its failing regression and then required exact
predecessor resolution for this finite no-pruning packet. Its independent golden
states, command measurements and prior artifacts stay unchanged. This does not
change production proof pruning or claim native history authenticity. No post-fix
independent rereview or native qualification is claimed.
