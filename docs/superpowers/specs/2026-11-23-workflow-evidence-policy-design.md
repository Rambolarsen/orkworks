---
type: Design Proposal
title: Workflow evidence payload exclusion and observation freshness
description: Proposed first policy slice for Peon evidence matching and age-bounded new recommendation and ordinary Brain inputs.
tags: [taskmaster, peon, workflow, evidence, policy]
status: draft
workflow_status: accepted
---

# Workflow evidence payload exclusion and observation freshness

Owner decision: **accepted 2026-11-24**. This proposal and its
[implementation plan](../plans/2026-11-23-workflow-evidence-policy.md) are for
[#1077](https://github.com/Rambolarsen/orkworks/issues/1077). Neither a proposal
merge nor a request to continue writing approves runtime implementation.

## Outcome and evidence

New Peon reports must not ground solely in hidden ESC-based terminal payloads.
New exact-family cards and ordinary Brain observation input should use recent,
date-valid reports. Preserve existing evidence, acted-on history and explicit
agent-report authority.

The [retained investigation](../../validation/workflow-observation-quality.md)
at source `32a0bae4f419` distinguishes title/status/prose support failures from
real historical verifier and inspection errors. Source was rechecked at
`3c3ccd91`: `peon::strip_ansi` already removes ESC OSC/DCS payloads;
`evidence_is_grounded` currently matches raw captured text; exact-family
eligibility has no age filter; ordinary Brain selection takes sixteen
alphanumeric-evidence rows. The shared recent-observation helper also serves
assessment/readiness, so changing it globally would widen this proposal.

This slice adds no model call, classifier, schema, API field, permission,
storage format or raw-terminal capture. It does not establish semantic truth:
visible titles, quoted code, durations and unsupported descriptions can still
pass. Independent current useful examples and held-out empirical outcomes remain
missing; this design cannot close #1077 or qualify a semantic-quality gain.

## A. Peon workflow grounding

For each already-captured, already-reassembled output snapshot:

- `E` is the evidence string with the existing edge trimming.
- `R` is that exact snapshot joined with newline, as today.
- `V = peon::strip_ansi(R)`, using the existing regular parser unchanged.
- Accept the grounding check only when `E` is nonempty and **both**
  `R.contains(E)` and `V.contains(E)`.

The raw match remains mandatory: stripping must not introduce new matches
across colored spans or removed control sequences that were not contiguous in
the original evidence view. Do not strip, rewrite or paraphrase the candidate.
The persisted evidence remains the original validated string.

Apply the shared predicate to every existing Peon persistence path: live file
recording, final file scan, captured ending evidence, and captured SQL-origin
batches. Existing identity/owner/fence/replay/retention/bounds remain binding.
Explicit authenticated agent reports do not acquire a terminal-grounding gate.

The additional view exists only for this grounding check. Do not change the
terminal buffer, replay, Peon situation/label/capacity inference prompt or the
shared ANSI parser's behavior. Reuse the captured bounded snapshot; linear
work and one normalized view, no extra filesystem reads or retained transcript.

### Parser limits and examples

`strip_ansi` is an ESC-sequence normalizer, not a screen emulator. This
proposal covers the ESC OSC/DCS and other sequences it already recognizes,
including OSC BEL/ST endings, split rows in the joined snapshot, and incomplete
trailing payloads. It does not add C1 control parsing or recover an opener
evicted before the capture window. Those remain limitations, not permission to
invent missing context or silently widen the parser.

| Captured text and evidence | Grounding |
| --- | --- |
| Only `ESC ] 0 ; Investigate main sync issue BEL`; evidence is the title | Reject: the visible view has no title |
| Same title as ordinary printed text | May pass; no semantic truth is established |
| OSC hyperlink URL as sole evidence | Reject; displayed hyperlink caption may pass if raw-contiguous |
| Ordinary complete `fatal: cannot open build.lock` | Pass if present verbatim in both views |
| Candidate spans an embedded SGR escape and appears only after stripping | Reject; the raw match fails |
| Identical quoted error in printed source code | May pass; this slice has no source/prose classifier |
| An unknown or malformed capture has no usable normalized match | Grounding rejects; raw history and other Peon metadata are untouched |

The title case in the retained corpus lacks its full raw capture in the JSON.
It is motivating evidence, not a replay fixture proving this predicate would
reject every historical citation. End-to-end implementation tests must supply
actual ESC payloads and positive visible text.

## B. Freshness predicate

Use a single shared pure predicate with an explicit server-owned UTC evaluation
instant `T`. A report is recent iff its RFC3339 `observedAt` parses and:

`0 <= T - observedAt <= 1,209,600 seconds` (fourteen exact 24-hour days).

Both ends are inclusive; fractional seconds and explicit timezone offsets are
compared as instants. Missing, malformed and future times are unavailable for
this selection, not rewritten or treated as fixed/resolved. Do not substitute
file mtime, session activity, current time or recommendation `updatedAt`.
Apply this age rule to both Peon and agent rows; their trust and grounding
policies remain distinct.

Fourteen days is a proposed product default aligned with the existing explicit
audit's horizon, not a measured optimum. The consequence is deliberate: a
genuine old failure contributes to a new card only after new grounded reporting
reconfirms it. Merely mentioning it again is not independent semantic proof.
No new report may be synthesized from an old timestamp or the current summary.

## C. Exact-family creation and compatibility

Keep confidence/impact thresholds, fingerprints, dedupe families, recurrence
units and lifecycle/watermark/supersession rules unchanged.

For a pass that would allocate a **new recommendation ID**, use only recent
retained observations that also meet current confidence/impact eligibility.
At least two such observations sharing a fingerprint are required. Compute the
new card's evidence, count, affected sessions, impact and confidence from that
set only. Existing terminal snapshots cannot supply a missing recent occurrence.
For a successor, preserve all additional existing resurfacing/terminal-lineage
conditions; recency is necessary, never sufficient.

An existing proposed exact card being updated under its current ID, and an
existing rolled-up member updated through its live proposed parent, retain
their current update/merge rules. They are explicitly grandfathered: this slice
does not prune embedded evidence, change old counts or automatically retire
them. Old visible noise may therefore remain until the user's existing
audit/dismiss/completion path acts. Rollup-family snapshots are unchanged.

Existing accepted/executing/terminal cards remain inspectable and usable under
their current rules. List/get/accept/complete do not run a new age rejection.
The gate concerns new record creation, not retroactive authorization withdrawal.
A date-valid old stored observation can contribute while within fourteen days;
no installation-time fence or backfill is introduced.

Use one `T` for the deterministic pass. An invalid caller-supplied evaluation
time yields no proposed writes and a bounded existing diagnostic; never
partially apply a pass or persist malformed timestamps. Production captures
`T` server-side. Pure tests inject it.

## D. Ordinary Brain observation input

At ordinary Brain analysis dispatch, capture `T` once. Filter age-valid rows
and existing alphanumeric-evidence requirements **before** taking the latest
sixteen by sequence. Feed exactly that selection to all observation-derived
relevance signals, repository excerpts, knowledge-page ranking and serialized
observation context. Timestamps and kinds stay visible.

This applies to ordinary legacy Brain analysis, whether manually triggered or
background-scheduled. It does not change repository-readiness, repository-led
assessment or the separately gated Brain-guided manual investigation. Leave
their existing 128-row/shared-helper contracts intact. Repository evidence may
still support a proposal with zero observations under the existing rules.

Bind the ordinary request to an in-memory ordered selection identity covering
all supplied observation fields/IDs plus policy tag
`workflow-observation-input-v2`. Include the policy tag in the ordinary prompt
so the existing full-prompt cache key cannot reuse a pre-policy response.
Preserve existing provider/settings/knowledge/workspace/rollup currentness fences.

Before applying a model reply, recompute the same bounded observation selection
from current retained rows at a new server-owned apply instant. If its identity
differs, reject the **whole** reply as stale before enrichment/proposal/rollup
effects; record the existing failed/stale evaluation outcome. This includes a
cutoff crossed in flight, a new row displacing one of sixteen, deletion,
workspace replacement and unavailable selection. Do not retry the model,
refund usage or change automatic scheduling/call budgets. A later ordinary
scheduled run remains subject to all existing budgets and admission.

Do not reject an in-flight reply merely because wall-clock time advanced if
the full selected observation identity is unchanged. No persisted token, model-
supplied authority or new bearer is introduced. Selection comparison must occur
inside the existing current-workspace/application fence; a clone checked outside
the fence cannot authorize a later stale write.

## Compatibility, recovery and limits

- Existing codecs, stored observations, recommendation IDs, evidence snapshots,
  raw replay and the frozen corpus are unchanged. No migration or startup write.
- Age filtering is derived, bounded by the existing 10,000-row reconstruction;
  ordinary context remains sixteen rows. No extra observation read or model
  call is allowed solely to infer age or source meaning.
- No automatic cleanup or new audit criterion. The existing 14-day audit
  behavior, including its handling of malformed dates, stays as documented.
- Reverting the future implementation restores old selection/grounding behavior;
  new records remain readable by old code. That rollback may again admit noisy
  inputs. It repairs no history and removes no owner's evidence.
- Clock jumps may temporarily exclude future reports or newly age reports out.
  Preserve their bytes. After restart derive selection from current UTC and the
  retained records; never infer inactivity or successful recovery from exclusion.
- No promise of complete UI-chrome removal, incident-level deduplication,
  immediate backlog cleanup or semantic-quality improvement.

## Alternatives and known tradeoffs

A keyword blacklist cannot distinguish quoted errors from real errors.
Normalizing only the haystack would broaden the raw literal gate, so require
both views. A separate terminal emulator or classifier is disproportionate to
the observed OSC-title mechanism and is not selected. Fourteen-day filtering
cannot decide whether old friction remains real; new reporting and separate
human adjudication retain that distinction.

Grandfathering existing IDs avoids retroactive evidence/count/lifecycle
changes, but leaves old cards and their rollups visible. Filtering every update
would need a separate evidence/currentness/display contract and is not selected.
Same-window candidate coalescing would need provenance and representative/
replay decisions; it remains outside this slice.

## Review and owner decision

The requested scoped decision is whether to accept A–D, including the
dual-view raw gate, fourteen-day boundaries for both sources, explicit
grandfathering, and whole-reply currentness rejection. Existing general
permission to investigate or prepare a proposal is not that decision.

After acceptance, amend the authoritative Peon/Taskmaster sections and execute
the reviewed plan with TDD, repository checks and the required current-code
review. Deterministic tests can establish these invariants, not model semantic
quality. Do not mark #1077 complete without independently adjudicated current
useful/held-out examples and measured support/usefulness outcomes.

## Owner acceptance — 2026-11-24

The owner replied “keep going” directly to the explicit implementation approval
question after reviewed proposal PR #2024 merged. This accepts A–D and the linked
three-task plan for implementation: dual-view grounding, inclusive fourteen-day
new-input age policy for both sources, existing-ID grandfathering and fenced
whole-reply currentness. No empirical model/native trial or semantic-quality
qualification is granted. #1077 remains open for its independent quality criteria.
