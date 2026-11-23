---
type: Validation Evidence
title: Offline research clarification components
description: Accepted material-clarification source examples with user-origin answer admission, copied-decision outcomes and gate-respecting state selection; supplied authority facts stay unqualified.
tags: [orkworks, research, clarification, answers, qualification]
status: draft
workflow_status: clarification-components-only
---

# Offline research clarification components

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: the merged clarification contract plus the acceptance record
(`b9f2524d8acf`).

The owner [accepted the five clarification choices](../superpowers/specs/2026-11-18-research-clarification-design.md#scoped-owner-acceptance-—-2026-11-19)
for offline components only on 2026-11-19. These are separate decoded examples
and supplied-fact oracles — independent of the prefix packet's revision
numbering — not an event sequence, complete store, publisher or authority
implementation. Gate 2 remains **Investigate**.

## Artifacts and source boundary

- [Independent Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-clarification.py)
  reads the unchanged prefix packet; it never calls JavaScript.
- [JSON oracle](research-clarification-components.json) retains the
  clarification assessment manifest with the closed
  `ResearchClarificationDecisionV1`, the immutable request row with its exact
  pending binding and gated/stage-fence views, the Main-authorized answer
  request with the `ResearchUserActionOriginV1` user-origin provenance, the
  answer family row with its receipt, the copied-decision answer outcome with
  the one `clarification-answer` record and cleared pending pointer, the fresh
  parent assessment at revision 4 retaining the applicable answer entries, the
  four gate-respecting state alternatives, the changed-answer re-ask at
  `questionVersion+1`, the exhaustion/resume labels and effect labels.
- [JavaScript checker](research-clarification-components.mjs) independently
  rebuilds finite source bodies, validates exact keys/values/domains/joins and
  digest exclusions, the decision-union variant discipline, the producer
  union (runtime versus user origin), wrapper widths, and supplies the
  admission/state helper oracles.
- [Tests](research-clarification-components.test.mjs) regenerate exact
  UTF-8/LF bytes, exercise fourteen semantic-mutation denials with correct
  reseals, and pin literal domains, bounds and labels.

## What the packet establishes

- Exactly one bounded material question per clarification assessment; the
  fresh decision carries its own revision; the request row binds the asking
  manifest and byte-equal question; `pendingClarification` becomes the exact
  `{questionId,questionVersion,requestDigest}` binding; `awaiting_clarification`
  is selected only when the runtime is active with no higher gate, and a
  retained blocker keeps the run blocked with the pending binding recorded.
- The answer admission follows the Main authority/shown-context order; an
  identical retained retry returns the original receipt before live CAS even
  after the run becomes terminal; stale request versions and stale inputs
  reject with the current request state; the stage fence rejects clarification
  mutations.
- The answer outcome follows the accepted event-induced pattern: the next
  input revision carries the one `clarification-answer` record variant and the
  byte-unchanged copied decision with the user-origin producer; the pending
  pointer clears; `runVersion` advances once; the fresh parent assessment is a
  separate later outcome at its own revision that retains the applicable
  answer entries; a paused run retains `resumeTarget=assessing`.
- A changed answer re-asks at `questionVersion+1` with the original version
  retained; the 64-revision and 32 question/answer pairs bounds deny
  atomically; exact-identity resume restores the same pending binding.

## Supplied and unqualified facts

Current Main authority, epoch, owner, lease, health, rate and shown context;
sidecar-derived user-origin provenance; runtime eligibility; complete
source/consumer/index membership; original H/C/proofs; resource ledgers and
actual effects. Matching digests and repeated source bytes establish none of
those facts. Source fingerprint hashes cover documentation bytes; CommitRef
changeDigest hashes explicitly unqualified labels. No wire decoder, header,
manifest, row/change hash, runtime reducer, native/model probe, database,
migration or executable product plan is supplied. The execution-proposal and
terminal-completion packets are later separate bounded tasks under their own
acceptances.

## Verification and measurements

Node 22 `node --test` runs all prior documentation packets unchanged (the
packet's own 27 regressions pass, including independent regeneration
equality, twenty-two semantic-mutation denials with correct reseals and
asserted reasons, helper oracles distinguishing retained identical retries
from new admissions and preserving paused states, gate-respecting state
selection, and literal domain/bound pins). The generator-derived
measurements record the exact key plus committed occurrence projection for
the retained rows; they are not storage charges or a legal transaction
witness.

## Hosted review corrections — 2026-11-20

The hosted review of the first packet head reported nine P2 findings, all
addressed in this revision: the request/answer family keys nest the RunKey
member exactly as accepted (re-ask row included); the fresh assessment
manifest now derives the decision-required capture records alongside the
retained answer; `questionVersion` stays out of the closed request body
(version lives in the family key and references); the pending-identity
comparison is built from the request row's key and digest; the answer-state
oracle preserves retained paused states without requiring an active parent;
the run's current-input advance is validated against the answer manifest;
the persisted answer body is verified against its digest and receipt;
unsupported decision flag combinations are rejected explicitly; and the
unchanged 32 KiB prepared-input body ceiling is enforced independently of
the wrapper check.
