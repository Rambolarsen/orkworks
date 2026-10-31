---
type: Architecture Decision
title: Gather readiness investigation evidence before requiring a candidate
description: Readiness-only evidence planning that preserves independent proposal grounding and the four-call ceiling.
tags: [orkworks, architecture, taskmaster, readiness, evidence]
status: stable
workflow_status: approved
---

# Readiness evidence before candidates

- Status: accepted; owner approved implementation on 2026-10-31 ("go ahead"). No additional live provider calls are included.
- Tracking: [#1175](https://github.com/Rambolarsen/orkworks/issues/1175), draft PR #1178.
- Scope: partially supersedes the combined suggestion-stage discovery in
  [ADR 0100](0100-score-directed-readiness-improvements.md), leaving standalone
  workflow discovery and all score/approval authority unchanged.
- Delivery: [the existing plan's approved follow-up](../superpowers/plans/2026-10-30-score-directed-readiness.md#approved-evidence-first-follow-up-adr-0099).

## Evidence and outcome

Two retained fixed experiments failed independent usefulness qualification:
[initial prompts](../validation/fixtures/score-directed-readiness-2026-10-30/index.md)
and [v4/focus-v2](../validation/fixtures/score-directed-readiness-v4-2026-10-31/index.md).
The latter contains a runnable test body without assertions, but suggestion
discovery returns no candidate because its seed evidence does not contain the
implementation. Another proposal ambiguously duplicates existing assertions
instead of explicitly connecting their existing test target.

Standalone discovery and the previous combined protocol require positive candidate evidence before granting its one
bounded follow-up read. A related implementation can therefore remain unknown
because there is not yet enough evidence to justify a candidate. More prompt
wording did not reliably bridge this ordering problem.

The intended result is one concrete, independently grounded next change, or an
honest no-proposal result, after reading the relevant implementation. Low or
unknown scores never establish a defect or require a proposal.

## Accepted contract

Only a server-owned combined run with its exact newly saved readiness reference
uses `taskmaster-readiness-investigation-v1`. Its first suggestion call
plans evidence; it does not invent or approve a candidate. A closed response is:

```json
{"version":1,"conceptPageIds":["concepts/verification.md"],"requestedFileIds":[7]}
```

- `conceptPageIds`: one to three distinct existing signed concept pages from
  the supplied readiness index, with exact canonical capability mapping. Missing
  concept guidance makes continuation unavailable before model dispatch.
- `requestedFileIds`: zero to eight distinct numeric IDs from the existing
  permitted catalog. Empty requests still reach final assessment from seeds.
- No paths, commands, tools, URLs, prose, candidate, outcome or early
  `no_candidate` field. Unknown/duplicate fields and invalid IDs are rejected.

The ordinary workflow collector remains version 3, with its strict complete
selection and governing-instruction closure. Existing access/exclusion, 32-file,
64-KiB, prefix and excerpt limits remain. An unaffordable or unreadable complete
selection produces the existing fixed insufficient-evidence/interrupted result;
it does not publish a partial-evidence proposal. No dynamic tool access is added.
If needed implementation evidence is outside the permitted catalog, it remains
unknown; this design does not grant source-code access or guarantee a candidate.

The second suggestion call assesses the selected capabilities against the fresh
bounded facts and pinned signed pages. It keeps the existing final assessment-v1
shape, required current repository and selected concept citations, positive gap
threshold, no-proposal option, duplication/counterevidence checks, practical
benefit, validation/rollback and ban on promised numeric gains. Existing checks
are reused when the defect is disconnected wiring. Missing assertions can be
claimed only for a complete visible body, not for omitted repository text.

Every affordable valid investigation reaches final assessment, including unknown
or already-satisfied cases. The final model owns neither evidence permissions
nor action authority. Actual changes still require the existing Fix with AI flow.

## Owners, identity and compatibility

- `assessment_discovery` owns the new closed response parser alongside the
  unchanged standalone discovery-v1 parser. The native response-constraint owner
  owns its new code-selected shape; model output cannot choose the protocol.
- `assessment` branches only on the captured server-owned readiness purpose.
  It chooses the discovery prompt/decoder, captures the same bounded read set,
  and always reaches final assessment for a valid affordable readiness plan.
  Standalone candidate/refutation behavior is unchanged. New standalone admission
  uses its own purpose and cannot reuse a combined report, focus or read selection;
  existing combined recommendations retain their independent currentness fences.
- `readiness_focus` advances its prompt identity to v3. Source reference,
  selected plan response digest, actual read manifest and signed guidance remain
  bound to cache/currentness checks before every dispatch and publication.
- Existing assessment record format 3 and recommendation provenance remain.
  The plan response is memory-only; its exact digest uses the existing manifest
  discovery identity, and the admitted IDs restore the same evidence after a
  restart. Existing phase names and desktop contracts remain unchanged.

The total ceiling remains four provider calls: two scoring and two suggestion
calls. No retries or replacement samples. Pending recovery replays publication
only, never inference. Old history remains readable but the changed focus prompt
identity invalidates reuse/currentness. Rollback disables combined admission and
forward-fixes; do not rewrite or downgrade retained records. An older binary must
fail the changed fingerprint rather than recreate new dispatch authority.

## Verification and qualification

Keep all prior failed samples unchanged. Deterministic tests must establish:

1. A readiness plan without a candidate can select permitted implementation
   evidence and reach the final call; a standalone response still requires the
   existing candidate/no-candidate contract.
2. Already-satisfied and unknown facts reach an honest final no-proposal;
   requested bounds, governing instructions, signed concept selection, changed
   inputs/access/rubric, restart recovery and the four-call ceiling remain fenced.
3. A disconnected script reuses its existing asserted suite; a complete visible
   body without assertions supplies a bounded distinct gap. Positive proposals
   need current fact and selected concept citations; omitted text proves nothing.

After implementation, request a separate owner-approved fixed five-sample budget
(maximum twenty calls) on the configured profile. Require at least two of three
positive samples to earn independent usefulness credit. Both negative controls
must finish valid scoring and final no-proposal; a failed control is not a pass.
Do not count syntax-valid outputs, duplicate work, or offline repair as quality.
No additional live calls are authorized by this proposal.

Deterministic scripted responses verify contract enforcement and that the final
stage receives the selected implementation. They do not establish model usefulness
or change the adjudication of either retained failed experiment.
