---
type: Design Contract
title: SQL assessment input classification
description: Proposed typed unknown-versus-qualified source classification before the existing assessment provenance codec, with a private policy-only first deliverable.
tags: [orkworks, sqlite, assessments, provenance, admission]
status: draft
workflow_status: proposed
---

# SQL assessment input classification

Tracking: [#1243](https://github.com/Rambolarsen/orkworks/issues/1243).
Source pin: `5bc01fb75a6ae6f1d4a1362d280a035617448fe8` (#1635).
Reuse the accepted [source provenance](2026-11-08-sql-assessment-source-provenance.md)
and [observation origin](2026-11-08-sql-observation-origin-capture.md) contracts.
This proposal selects a private semantic policy boundary and its first delivery;
it requires owner acceptance before dependent code. It changes neither accepted
format4 nor the production JSON selector.

## Outcome and source evidence

An assessment must refuse its complete selected input when even one member has
unknown origin. Empty dependencies are eligible only when a qualified owner
knows the complete input has no session dependencies. Valid JSON, an empty
visible session list and a current SQL row cannot establish that knowledge.

| Source at the pin | Verified behavior | Consequence |
| --- | --- | --- |
| [Assessment capture](../../../crates/orkworksd/src/taskmaster/assessment.rs), `capture_workspace_input_for` and `build_workspace_input` | Captures full selected observation/recommendation lists, subject to the existing derived-recommendation exclusion, and optional readiness focus; fingerprints those selected lists although the prompt uses bounded subsets | Classification must use the complete identity snapshot, not the prompt subset |
| [Source codec](../../../crates/orkworksd/src/taskmaster/assessment_sources.rs), `OriginalInputs::fixture` and `OriginEnvelope` | The trusted test-only constructor accepts typed source arrays, including valid empty arrays; malformed/missing arrays fail decoding | Shape rejection does not distinguish unknown input from qualified source-free input |
| [Copied qualification](../../../crates/orkworksd/src/runtime/peon_runtime/sql_observation_assessment_tests.rs) | Actual producer values and original pairs survive checked copied reopen and point-in-time SQL validation; strict inventory retains a legacy value without origin | Parsed origin data and copied-fixture success do not mint an original-input claim |
| [SQL validation](../../../crates/orkworksd/src/session_store/assessment_sources.rs) | Checks original epoch/workspace/exact live creations in one bounded transaction, including header checks for empty sets | Semantic classification and SQL currentness are separate checks |

Current recommendations and readiness snapshots have no qualified complete-
closure owner. They remain Unknown even when direct arrays are empty or a saved
provenance object is present. The existing legacy derived-recommendation identity
exclusion remains with its exact current owner; this proposal adds no exclusion.
Repository, signed knowledge, access/settings and evaluation fences remain.

## Decision and alternatives

Choose typed classification within the existing private assessment-source owner,
before constructing the accepted envelope. Do not introduce a general provenance
framework or new durable representation.

Rejected alternatives:

- Treat an empty parsed `sources` array as source-free: loses the owner proof.
- Filter unknown members: changes the complete fingerprinted snapshot and hides
  the reason admission is unavailable.
- Infer creations from current IDs, timestamps or saved report fingerprints:
  assigns old input to a possibly different incarnation.
- Upgrade every producer in this first slice: recommendation mutation, inherited
  dependencies and readiness context require distinct concrete owner contracts.

The new choices here are my proposed implementation decisions, not prior owner
approvals. The accepted refusal policy and durable source format are reused.

## Typed states and value binding

Each observation, recommendation and present readiness input has exactly one
classification:

| State | Meaning | Result |
| --- | --- | --- |
| Unknown | No qualified owner attests its complete original closure | Refuse the whole set with a typed Unqualified outcome |
| Qualified | An opaque claim binds this exact immutable input value, original SQL scope and complete sorted source pairs | Eligible for union and existing codec validation |
| Qualified with zero pairs | The same owner proof explicitly establishes no session dependencies in that complete input | Eligible; still retain and validate original SQL scope |

Zero pairs are a case of Qualified, not a default or a synonym for Unknown.
Optional readiness absence means no input; present readiness with unknown origin
means refusal. Empty observation/recommendation lists are genuinely empty lists,
not permission to discard unknown members. An entirely empty set still needs
its selected original scope from a qualified capture context.

A qualified input's fields are private. Bind value, original scope and closure
in one immutable opaque object. No Deserialize, generic `From<Value>`, public
field construction, value replacement, detachable reusable proof or caller-
controlled success callback. A model result or parsed origin object is data,
never an input claim. Do not introduce a public raw-pairs constructor.

The complete set owns ordered input members and an original selected scope;
its constructor and membership are private to the capture owner. It is not a
caller-filtered iterator. The policy consumes that complete set once. Existing
observation/recommendation order and the readiness reference value are preserved
exactly. All qualified members must match the set's original epoch/workspace.
Conflicting creations for one ID refuse; equal pairs deduplicate only in the
source union, never by dropping input values from the envelope.

Repository facts and signed knowledge do not acquire session-free authority
from missing IDs or prose inspection. A future capture owner must establish
that classification for its actual complete inputs while preserving existing
repository/knowledge/currentness checks. No such production owner is added by
the first deliverable below.

## Pure policy and unchanged durable data

The classifier runs before the existing source-data codec:

1. Check the owned set's original scope and complete encoded input bound.
2. Refuse any Unknown member, without replacing it with an empty closure.
3. Check every qualified claim's scope and closure; union exact original pairs.
4. Preserve the complete ordered values/pairs in the accepted envelope and use
   the existing canonical digest/source-data rules.
5. Return immutable classified source data or a typed failure, never a partial
   packet, empty fallback, OriginalInputs grant or application operation permit.

This result is data only. Parsing it later cannot reconstruct Qualified claims.
The first slice adds no production OriginalInputs constructor. Future original-
input capture still needs producer/application qualification followed by the
existing typed SQL check. SQL row validation grants no file publication or
serialization with deletion, as the accepted contract already states.

Keep 128 distinct sources, 16 KiB compact source-packet and 64 KiB assessment-
record limits. Propose an additional 1 MiB compact canonical origin-envelope
limit, including all values, claims and JSON structure. It is independent of
and does not enlarge the observation reader's 768 KiB selected snapshot or the
128 KiB prompt limit. Refuse the complete set rather than trimming it to fit.
Count with a bounded writer before an additional full encoding/clone; hash
canonical data without retaining a second unbounded envelope. This is an
encoded limit, not a measured heap or native-memory guarantee. No new global
worker pool, file slot, SQL queue or timeout policy is introduced by pure code.

Unqualified, conflicting scope/creation, invalid data and TooLarge remain
separate private outcomes. Do not wire new HTTP/error UI or silently treat SQL
Busy/unavailable/corrupt as Unknown or empty data. The classifier performs no
SQL, file, model, scheduler, publication or cleanup effects.

## First deliverable and later owner gates

After acceptance, implement only the private typed policy in the existing
assessment-source module and meaningful direct tests. Qualified claims and a
qualified empty-set scope may be minted only by explicit trusted `cfg(test)`
factories in this first slice. Production has no Qualified minting path and no
call from assessment scheduling. Do not expose the old fixture constructor as
an application API. A green policy test qualifies the semantic algorithm, not
an end-to-end runtime input owner or durable-origin authenticity.

Later producers must separately qualify their actual claim-minting boundary:

- Observations: original final-scan provenance, strict checked owner, complete
  immutable selected inventory, original opening/root/lease and source rows.
  A deserializer or value digest alone is insufficient.
- Recommendations: the complete mutable source/evidence, affected/dismissal,
  target/completion and inherited assessment closure on every relevant mutation.
- Readiness: original observation/context closure and immutable reference,
  including dependencies absent from visible focus JSON.
- Capture/application: full snapshot membership, original workspace/root/lease/
  worker/evaluation authority before waits, retained finite ownership and no
  shared application guard across SQL/file awaits.

Only after those concrete integrations are qualified may a separate reviewed
change mint production OriginalInputs, call typed SQL capture and route SQL
assessment work. Pending publication/recovery, provenance-aware deletion,
actions, full startup/migration and activation retain their existing gates.
This contract gives no authority to implement those later boundaries by guess.

## Qualification and implementation handoff

Use valid-shaped data to test the actual policy, rather than expecting a JSON
shape error:

- One Unknown observation, recommendation or present readiness input refuses
  the full otherwise-qualified set; include valid empty source arrays in the
  underlying untrusted data. Preserve input membership, order and values.
- Qualified zero-pair input succeeds in its original scope; the identical value
  classified Unknown refuses. An empty set requires a qualified original scope.
- Mixed scope, equal ID with differing creation, duplicate/noncanonical pairs
  and invalid identity refuse. Equal pairs union without dropping values.
- Pin the accepted canonical digest for qualified vectors and parity with the
  existing codec. Change value, order or original source and require a changed
  binding or explicit refusal.
- Exercise exact/one-over 128-source, 16 KiB packet and 1 MiB envelope limits,
  with no partial output or shrinking selection. Retain existing independent
  64 KiB assessment-record tests without changing that store's behavior.
- Demonstrate that serialized source data is not a qualified token and no
  non-test constructor or production assessment call is added. Use Rust's
  privacy/type boundary and consumer tests; no source-text grep assertions.

The concise implementation sequence is test the typed semantic distinctions,
add the smallest private classifier reusing the codec, then qualify bounds and
binding parity. Update canonical claims with actual evidence, run focused tests
and the full repository verifier, complete current-code manual review and the
normal PR/CI/owned-worktree lifecycle. After owner acceptance, prepare the concise
reviewed implementation plan and follow the ADR 0103 lifecycle before dependent
code. No implementation starts from this draft.

## Review and owner decision

Root owns this document. Before handoff, at most two fresh read-only reviewers
(`gpt-6-luna`, high effort) get one round plus bounded clarification: one checks
source/authority claims, the other checks whether the policy-only first slice
is concrete and testable without inventing a production grant. Root reconciles
findings in this document. No reviewer edits or implements code.

Accept or revise the typed Unknown/Qualified distinction, value-bound private
claims, data-only policy result and the additional 1 MiB envelope bound for this
first private policy/test deliverable. This does not approve producer minting,
production OriginalInputs construction or runtime routing. Existing approvals
remain valid within their original scope.

Independent review disposition: the builder found no important unanswered
question for this first slice. Source/authority review found no scope or authority
blocker; its wording correction about the existing derived-recommendation
exclusion is applied above. The proposal is Ready for the owner decision, not
accepted for implementation. No runtime or physical qualification follows.
