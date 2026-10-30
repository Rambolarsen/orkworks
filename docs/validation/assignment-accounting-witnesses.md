---
type: Validation Evidence
title: Retained assignment source accounting witnesses
description: Independent retained-body and complete-command recounts of the existing root empty-output profile, plus archived child review owner-charge components and an explicit missing-history counterexample.
tags: [orkworks, hierarchy, accounting, validation, evaluation]
status: draft
---

# Retained assignment source accounting witnesses

Tracking: [#744](https://github.com/Rambolarsen/orkworks/issues/744),
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
This continues the [accepted aggregate policy and arithmetic packet](assignment-aggregate-budgets.md)
with source-traced accounting. No new source schema, quota, producer, authority,
database or runtime behavior is introduced.

## Reproduce

```bash
python3 docs/validation/generate-assignment-accounting-witnesses.py > /tmp/assignment-accounting-witnesses.json
cmp /tmp/assignment-accounting-witnesses.json docs/validation/assignment-accounting-witnesses.json
node --test docs/validation/assignment-accounting-witnesses.test.mjs
pnpm --dir docs docs:build
```

Use repository Node 22. Python independently measures the exact frozen input
catalogs and complete encoded rows. The Node consumer checks those measurements;
it also executes the original empty-output checker unchanged, including that
checker's full golden regeneration, transition/receipt/proof/ledger checks and
sixteen negative cases. Exact SHA-256 fingerprints bind both input catalogs.
LF attributes preserve the measured inputs and output on Windows.

## Complete existing root witness

The existing [empty-output profile](../superpowers/specs/2026-10-25-hierarchy-empty-output-profile-design.md)
contains explicit absent, initialized, prepared, admitted and accepted states.
The new inspector reconstructs canonical key/value bytes, rejects duplicate keys
and unsupported tables, rechecks local row commitments, and validates retained
candidate/result body bytes, domain digests and exact source-derived keys against
the unchanged profile. It recounts complete object bodies rather than current
heads and reconciles all six record/payload owner ledgers with reserved=0.
Unprepared states have no subject or source-ledger initialization claim.

| State or transition | Exact retained record charge at each owner | Payload charge | Complete changed rows / key-value bytes |
| --- | ---: | ---: | ---: |
| Prepared | 0 | 0 | See catalog; explicit initialization only |
| Candidate admitted | 618 bytes | 0 | 23 / 28,371 |
| Result accepted | 1,362 bytes (618 candidate + 744 result) | 0 | 23 / 30,151 |

Each source charge applies once to the root assignment, once to its owner run,
and once to its workspace ledger. The three ledger scopes are projections of
one retained inventory; they do not triple physical object-body storage.
Acceptance adds 744 record bytes and retains the entire candidate. No head-only
recount or quota credit for receipt metadata is allowed.

For all four existing commands, independently compare complete before/after row
sets with the manifest and original declared command totals. Count full changed
key/value bytes, including header/proof, slots, claim, lookup, quota and summary
metadata. The format-3 lookup key is a 64-byte BLOB rather than its JSON string
wrapper. These are the original 1 MiB/zero-payload fixture caps, not production
64/128/256 MiB near-cap admissions. Neither unchanged record totals nor the
aggregate policy grants space for metadata or a successful native write.

Local commitment repair tests prove that ledger recount catches a head-only
744-byte total, a redirected target owner and nonzero reserved source credit,
even when the changed quota row's own commitment is rehashed. The original full
checker remains the complete fixed-profile validator: the accounting inspector
alone does not establish proof-chain authority, source producer validity or
arbitrary-store recovery.

## Archived child-review charge components

Sixteen existing claim-proof source vectors provide exact candidate, evaluation
and disposition bytes for root/child variants, including maximum-width examples.
The component reader rehashes their complete source objects in the source-owned
domains and derives assignment/run/workspace charges from the source target or
producer identity. It does not use reviewer runtime identity to select the owner.
The original corpus is fingerprinted and unchanged; this reader is not a general
source-codec or cryptographic authentication implementation.

The cross-run child reviewer has a distinct reviewer run. Its initial evaluation
is 2,086 canonical bytes; its correction is 2,244; the matching child-stream
invalidation disposition is 1,149. Each individual target-record charge belongs
to the evaluated child assignment's run, not the reviewer run. The reviewer's
artifact production still needs its own additional producer custody/budget proof.
No state claims those three records were published together.

**Retained-history counterexample:** the archived child correction names
`cccc…cccc` as the preceding evaluation digest. That differs from the archived
initial evaluation's actual domain digest. Although the disposition targets the
correction digest, those components cannot certify an initial → correction →
invalidation producer history. Summing their sizes as a source-valid retained
history would invent a missing join. The regression retains this distinction;
no older source or digest is rewritten to make it appear valid.

## Remaining witness boundaries

| Required evidence | This packet establishes | Still needed |
| --- | --- | --- |
| Root admission and acceptance | Complete existing conditional logical rows, source charges and command recount | Actual bootstrap/admission and native authority/durability qualification |
| Child and cross-run review ownership | Exact archived source-body charge components at the target owner | Coherent source-valid history, target-result/criteria/rubric/receipt/producer and artifact-custody joins |
| Historical corrections/invalidation | A concrete placeholder-predecessor gap | Complete immutable history and ledger deltas from real source-owned publication recipes |
| Present/duplicate/zero-byte payloads | No payload rows in this profile; unknown payload tables reject | Closed producer/artifact custody, full result/declaration-bound byte occurrences and missingness joins |
| Capacity denial and protected credits | Existing policy/arithmetic only | Exact unchanged complete-state witnesses, work/head/credit preservation and independent metadata-cap failure |
| Authenticated retries, old epochs, purge | No new claim | Exact authority/receipt/retention/consumer/custody proof, complete accounting and recovery effects |

Next bounded prerequisite: build a coherent conditional child evaluation history
with exact predecessor/result/snapshot joins and enumerate its complete source-
owned publication effects; resolve any missing artifact/custody/receipt codec
before claiming a complete transaction. Reuse accepted schemas and submit any
new protocol or authority choice for owner review before dependent fixtures.
The accounting above does not qualify maximum production profiles, physical
storage, native enumeration, #740 capabilities or an executable runtime plan.
Keep #744/#746 and the separate gates open.
