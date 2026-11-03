---
type: Validation Evidence
title: Retained child source component bytes
description: Independent synthetic child configuration, retained input, plan, approval and event-source bytes with strict join and denial checks; no source event, custody or runtime qualification.
tags: [orkworks, child-sources, components, canonical-bytes, qualification]
status: stable
workflow_status: component-bytes-only
---

# Retained child source component bytes

Tracking: [#1183](https://github.com/Rambolarsen/orkworks/issues/1183),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Contract: [accepted child source package](../superpowers/specs/2026-10-31-child-review-source-preparation-design.md#owner-acceptance-and-bounded-component-handoff--2026-11-02).
Base: `197c46e472b6bd232bf98635795aeea7a04c9e49` (#1321).
This is Gate B component evidence. Every identity, native descriptor, report,
approval/event outcome and commit link is synthetic. Nothing here establishes
an actual reservation, allocation, unique attachment, approved grant, current
read, eligible adapter, source-valid transaction or installed product behavior.

## Delivered packet

The standard-library [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-child-source-vectors.py) emits
[28 pinned component vectors](child-source-vectors.json). A separate
[JavaScript checker](child-source-components.mjs) reconstructs the sources and
verifies their exact canonical bytes, raw SHA-256 and applicable source domains.
The [tests](child-source-components.test.mjs) also reproduce the complete packet
independently and compare it byte for byte.

- Full child v3 verification-worker and review-reviewer configurations use their
  own profiles, settings/model/projection artifact kinds, requirement task bindings
  and child frames. The worker has inline input and an empty output contract;
  the reviewer receives the exact retained snapshot and declares one evaluation.
- Exact plan, approval request/source/receipt, reservation, allocation, attachment
  and assignment-source records reconstruct their references and frozen identity.
  Allocated worktree descriptors cannot replace the original source descriptor.
- One target result, its original criteria/rubric projection and a retained
  research-report descriptor form the review snapshot. Selected evidence IDs,
  source keys, report identity/version/size/digest and reviewer startup delivery
  are checked. No report excerpts are fetched or appended to startup.
- Source evidence retains each reference's exact raw UTF-8/Base64 bytes and its
  task owner. Empty opaque evidence and identical content under distinct IDs
  remain legal and are charged per occurrence; they do not become settings or
  passing capability checks merely because they are retained.

The root packet supplies only its already explicit common repository/worktree/
executable leaves and retained run. Common snapshot/evidence checks accept an
explicit role/task; their root defaults and historical catalog bytes stay unchanged.
No child record is validated by rewriting it into a root record. Root and child
instruction frames remain distinct from the later dormant instruction utility.

## Checks and bounds

Run from the repository root with Node 22:

```bash
rtk proxy python3 docs/validation/generate-child-source-vectors.py --output /tmp/child-source-vectors.json
rtk proxy cmp docs/validation/child-source-vectors.json /tmp/child-source-vectors.json
rtk proxy node --test docs/validation/child-source-components.test.mjs docs/validation/root-bootstrap-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

The child suite covers independent golden bytes; duplicate/unknown fields;
floats, exponents, negative zero, rounded integers, invalid Unicode/UTF-8 and
noncanonical wire order/spacing; wrong root/child/profile/settings/input/plan/
approval/reservation/allocation/attachment bindings; missing or altered retained
bytes; and source/criteria/report drift. Recomputed outer hashes do not excuse
changed frozen identities, approved groups, prompts or criteria projections.
Reviewer-role eligibility uses the target's immutable rubric; changing the reviewer's
own rubric cannot override it. Distinct child IDs cannot disguise shared worker/
reviewer allocation descriptors.

Bounds include inclusive multibyte labels, role/rubric and original reservation
versions through 2^31−1, textual CommitRefs beyond 2^53 through 2^64−1, path/base/
branch syntax, 16 KiB complete metadata wrappers, and exact 256 KiB frame/startup
ceilings. A combination of individually valid native path components can still
fail its complete metadata wrapper. This is a denial oracle, not a promise that
all accepted maxima fit together.

The complete-plan count includes plan/configuration/input bytes and every closed
artifact-reference occurrence, including repeated settings/policy/projection
links; there is no content-alias discount. Declared startup accounting includes
all enumerated automatic content plus supplied frames and exact retained-input
framing. Native adapter framing/overhead and lower verified limits remain unknown,
so these counts cannot establish delivery or native eligibility.

## Verification and review

Node 22 docs:build passed all 909 documentation component tests, including 129
child-source cases and the existing 78 root-bootstrap cases. Independent Python
regeneration reproduces the complete packet. Diff, documentation currency and
worktree checks passed. The existing VitePress circular/large-chunk warnings do
not concern source formats or these component assertions.

One bounded pair of read-only Luna reviewers checked contract correctness and
coverage/qualification claims. Both identified the same assertion-helper issue:
an undefined explicit message could produce TypeError on a mismatch. The helper
now supplies a fixed default, and rejection tests pass. Additional rehashed
regressions bind reviewer role to the target rubric and reject shared allocation
descriptors. No broader source or native qualification is inferred from review.

## Remaining qualification

The checker returns only `unverified-source-bytes-only`. Its complete fixture is
one selected two-assignment example with one report context, not a general
orchestration implementation or a supported maximum native profile. Candidate,
prior report, parent preparation and commit references remain supplied synthetic
context; actual event authority, historical custody and direct proof validation
are not inferred from matching fields or checksums.

No complete storage state, metadata transaction, consumer index, fresh zero-ledger
proof, manifest, currentness/health proof or native identity/fence is generated.
The metadata wrapper calculation measures exact proposed key/value shapes with
synthetic CommitRefs; it does not validate their owning manifests. Root/run
preparation, lifetime debit accounting and complete source/control/consumer effects
still require their real owners. No source pool is initialized or charged by this
packet, and no accepted debit history is pruned or repaired.

Gate B's component-byte step is delivered; the actual source/control/consumer and
native authority steps remain open in the
[qualification handoff](../superpowers/plans/2026-10-31-child-review-source-qualification.md).
Gate C complete transactions and Gate D runtime/platform qualification remain gated.
#1183/#1176 stay open, #740 stays closed, and no model/native probe or runtime
change is included.


## Counter range correction — 2026-11-02

Tracing the transition/preparation contracts found that the child checker used
#742's signed-63-bit run/admission maximum for a storage CommitRef. The
[storage Counter contract](../superpowers/specs/2026-10-24-hierarchy-storage-contract-design.md#proposed-strict-storage-representation)
instead requires canonical u64 TEXT through 18446744073709551615. The child
checker now accepts that complete positive range and rejects its successor;
run/admission sequence and JSON task/version bounds are unchanged. Two boundary
cases failed before the correction and pass after it. Existing golden bytes,
fields, domains and older root fixture catalogs are unchanged. This is a checker
correction, not a new source policy or proof that a commit occurred.

## Dormant Rust source codec — 2026-11-03

The first coding unit in [#1345](https://github.com/Rambolarsen/orkworks/issues/1345)
adds the crate-private
[child source validator](../../crates/orkworksd/src/taskmaster/child_sources.rs).
Its borrowed Rust input supplies complete canonical run/initial input, both
configuration/evidence/event-source bundles, and the retained target/report
snapshot with declared validation context. Read-only results retain the original
configuration/source bytes and digests, classified only
`unverified-source-bytes-only`. There is no production caller or store access.

Common strict bytes, repository/worktree/executable leaves and explicit role/task
snapshot checks reuse the dormant root validator; root entrypoint, error type and
root framing remain unchanged. Child profiles, settings/model artifacts, frames,
approval/event joins, frozen identity and target-owned review joins are separate.
All existing fields, domains and selected source bounds are preserved.

Rust tests pin all 28 independent vectors. A test-only
[parity bridge](../../scripts/child-source-parity-cases.mjs) executes the existing
129-case JavaScript assay and supplies its 115 complete-packet verdicts to the
Rust entry, plus two fully rebound complete metadata wrappers at the 16 KiB
ceiling and its successor. The remaining assay cases exercise standalone framing/decoding/generation;
Rust additionally checks raw input decoding across both bundles, complete frame/
startup limits, per-occurrence complete-plan accounting at its 2 MiB ceiling and successor,
and rehashed report semantics with exact supplied captures, URI
and UTC timestamp grammar. No test invokes a coding harness or performs a model
call. These checks establish retained-byte validation only; the remaining
qualification above still applies.

The final 12 focused Rust tests and full repository verification passed, including
2,726 Rust unit tests (17 intentionally ignored), existing root validation,
reporter/desktop checks and documentation build. One bounded correctness/coverage
review pair cleared the final code; coverage feedback added direct vector and
byte/accounting boundary regressions. The coding PR records the reviewed head.
The user-doc advisory needs no edit for this dormant module; the worktree
advisory concerns a foreign branch and does not authorize its cleanup.

PR CI additionally checks literal embedded-input routing and caught the new
child catalog missing from the Rust path filter. The routing now includes that
catalog and every dynamic parity-bridge dependency. A local regression reproduced
both the catalog and bridge omissions before the fix; all six routing cases and
the complete 42-case documentation/tooling contract suite pass after it. This
changes CI selection only; the reviewed Rust codec and source bytes are unchanged.
