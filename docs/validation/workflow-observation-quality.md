---
type: Investigation Report
title: Workflow observation support and freshness
description: Sanitized support-quality corpus and source trace separating reported friction from current actionable workflow improvements.
tags: [taskmaster, peon, workflow, evidence, validation]
status: draft
---

# Workflow observation support and freshness

This is a bounded investigation for [#1077](https://github.com/Rambolarsen/orkworks/issues/1077),
using source `32a0bae4f419` and read-only local evidence inspected on 2026-11-23.
It delivers a retained corpus and a policy proposal for review. It changes no
observation admission, recommendation eligibility, retention, model configuration
or installed behavior. The issue remains open for policy acceptance and its
subsequent implementation and evaluation.

## Evidence and privacy

The [corpus](workflow-observation-quality-corpus.json) contains eight purposively
selected observed cases and six explicitly synthetic controls. Observed rows use
fixed-field excerpts from the recommendation API; the completed main-title card
was fetched separately. The live list had 33 proposed cards at the read. This is
a snapshot count, not a stable backlog size or a random sample.

Local session IDs become per-case aliases; native identities, owner paths,
credentials, complete prompts and transcripts are not retained. Recommendation
IDs and observation sequences support owner-local reinspection. API evidence
snapshots are historical reference data, never instruction authority. No other
session was resumed, reopened or modified.

The title case is recommendation `recommendation-482fd6ec9d3418d2`.
Its six citations quote the source session's work title. The read-only native
transcript and terminal OSC title identify the topic; actual work diagnosed
WebGL font rendering, delivered merged [#1850](https://github.com/Rambolarsen/orkworks/pull/1850),
and tracked separate API errors in [#1851](https://github.com/Rambolarsen/orkworks/issues/1851).
Five citations arrived within about 32 milliseconds. That timing is not proof
of five actions or a particular inference-call schedule.

Three positive-support cases have independent source context:

- Native-addon load failure: the native session transcript includes actual
  verifier-prerequisite output at 2026-11-17T19:51:51.695Z with
  `ERR_DLOPEN_FAILED`. Later assistant reports say native validation passed;
  this investigation does not qualify the complete repair-command trace.
- Missing documentation dependencies: actual verifier output and repeated
  assistant reports establish that broad verification stopped. The current
  `starting-work` skill already requires per-surface dependency installation.
  Real friction therefore does not establish that an installation rule is absent.
- Ambiguous app identifier: a failed native inspection at
  2026-11-23T05:14:28.265Z contains the ambiguous-identifier diagnostic. Local
  application paths are omitted. Its current cause and recovery remain unverified.

These are concrete failures at the time, not evidence of useful new repository
changes today. Later success, a different worktree or missing retained output
cannot alone establish that an earlier observation was fabricated.

## Source trace

Source links below are pinned to the inspected revision. The accepted
[eligibility and audit contracts](../../specs/taskmaster.md#eligibility) remain
authoritative; the proposal below is not an amendment to them.

| Boundary | Inspected behavior | Consequence |
| --- | --- | --- |
| Capture and inference | [Peon scheduling](https://github.com/Rambolarsen/orkworks/blob/32a0bae4f419/crates/orkworksd/src/runtime/peon_runtime.rs) requires active identity-owned sessions with new output after the minimum revision; pending evidence bounds continuous-output delay. The [prompt/parser](https://github.com/Rambolarsen/orkworks/blob/32a0bae4f419/crates/orkworksd/src/peon.rs) asks for at most five concrete friction candidates and forbids ordinary progress/redraws. | A prompt instruction is not a semantic guarantee. |
| Grounding | `peon::evidence_is_grounded` trims evidence and checks its literal substring in captured output joined by newlines. Wrap reassembly occurs on the captured view. | A title, quote, status string or `FAILED` can be grounded without supporting the description. ANSI and capture-window limits still matter. |
| Recording and replay | [SessionApplication](https://github.com/Rambolarsen/orkworks/blob/32a0bae4f419/crates/orkworksd/src/session_application.rs) checks workspace/session/attempt identity, then grounds each Peon candidate. The live key hashes runtime, session, generation, first/last output revision **and candidate index**. Final keys include generation and index. Agent reports follow their separate authenticated path. | Replay protection distinguishes keys, not real-world incidents. Different candidate indices can have identical evidence. Timestamp bursts alone cannot recover their original scan/window identities. |
| Durable storage | [Observation storage](https://github.com/Rambolarsen/orkworks/blob/32a0bae4f419/crates/orkworksd/src/workflow_observations.rs) validates fields, confidence, v1/v2 fingerprints, rate bounds and 15-minute key replay; it preserves source provenance. | Optional model-supplied `problemArea` identifies a family, not artifact validity. |
| Redundant trimming | [Retention](https://github.com/Rambolarsen/orkworks/blob/32a0bae4f419/crates/orkworksd/src/runtime/retention.rs) periodically trims same-session Peon occurrences within 30 minutes, retaining first/latest, tombstones and protected acted-on citations. Agent reports do not participate. | Two burst endpoints can survive. Embedded recommendation snapshots can also outlive trimmed rows; trimming is not semantic rejection or present-day resolution. |
| Exact-family publication | [Deterministic evaluator](https://github.com/Rambolarsen/orkworks/blob/32a0bae4f419/crates/orkworksd/src/taskmaster/mod.rs) requires two stored qualifying records per fingerprint, confidence at least 0.6 and at least 0.8 for high impact; it merges prior proposed evidence by observation ID. There is no semantic-support or age predicate here. | Two distinct IDs support a count, not two independent actions or an unresolved obstacle. Completion/dismissal lineage and watermark rules are separate. |
| Ordinary Brain analysis | [Context selection](https://github.com/Rambolarsen/orkworks/blob/32a0bae4f419/crates/orkworksd/src/taskmaster/context.rs) filters alphanumeric evidence before bounding. The [ordinary evaluator](https://github.com/Rambolarsen/orkworks/blob/32a0bae4f419/crates/orkworksd/src/taskmaster/evaluator.rs) takes the latest sixteen, passes kind/time and tells the model timestamps are not proof of current applicability. | Symbol-only text is excluded from this feed, but durations, titles, quoted errors and old text can remain. This is distinct from deterministic publication and manual investigation. |
| Explicit audit | [Audit classifier](https://github.com/Rambolarsen/orkworks/blob/32a0bae4f419/crates/orkworksd/src/taskmaster/audit.rs) labels all-Peon/no-`problemArea` cards as noise and newest-evidence age over 14 days as stale; unreadable timestamps suppress the stale label. Approval precedes cleanup. | A supplied problem area defeats this noise proxy even for chrome; a real artifact without a problem area can match it. The audit is not proof of semantic correctness. |

Observed citations extend through 2026-11-23T05:29:38.525822Z. The October
capture-inactivity finding in #1077 is historical, not evidence that present
workspace capture is still inactive. Fresh persisted records establish some
recent capture, not coverage of every session, successful inference on every
attempt, or absence of dropped/failed candidates. Those questions need
attempt/capture diagnostics and denominators, not just the observation count.

## Corpus assessment

Judgements classify the **support supplied**, not all possible defects in the
session or the value of an automatically generated fix:

| Observed stratum | Cases | Judgement |
| --- | ---: | --- |
| Title, spinner, background status, CI description | 4 | Unsupported as evidence of the described friction |
| Addon failure, missing packages, ambiguous app | 3 | Concrete failure at the time; current usefulness unverified |
| Generic `FAILED` with mismatched problem area | 1 | Unresolved attribution |

Seven observed cases have at least two sampled citations meeting the current
numeric confidence/impact thresholds: three unsupported, three concrete-at-time
and one unresolved. The spinner case samples only one citation from a mixed
five-citation card; it cannot classify that entire card.

This is an arithmetic replay of supplied fields, not execution of the Rust
evaluator or proof that a new model/policy works. The main-title card was
actually published and independently adjudicated unsupported. The other
support labels are this investigation's bounded manual assessment. A population
false-positive rate, precision, recall or useful-recommendation rate cannot be
estimated from these eight selected cases. There are **zero independently
qualified current useful improvements** here; that is an evidence gap, not a
claim that useful improvements do not exist.

The six constructed controls cover duration-only text, a quoted error in test
source, a current confirmed failure with the same error wording, resolved
historical friction, old friction independently reconfirmed current, and an
agent report with non-terminal evidence. All six meet the numeric threshold in
their constructed two-row sets, despite requiring different support/currentness
decisions. They are scenario oracles, not captured production outcomes.

A short reproducible arithmetic/integrity check, run from the repository root:

```python
import collections
import json
from pathlib import Path

data = json.loads(Path("docs/validation/workflow-observation-quality-corpus.json").read_text())
assert data["schemaVersion"] == 1
assert len({case["id"] for case in data["cases"]}) == 14
for origin in ("observed", "synthetic"):
    cases = [case for case in data["cases"] if case["origin"] == origin]
    counts = collections.Counter(case["judgement"] for case in cases)
    def qualifies(row):
        return row["confidence"] >= 0.6 and (
            row["reportedImpact"] != "high" or row["confidence"] >= 0.8
        )
    eligible = sum(sum(map(qualifies, case["observations"])) >= 2 for case in cases)
    print(origin, len(cases), dict(counts), "two-citation eligible:", eligible)
```

Expected observed output: eight cases, unsupported 4, concrete-at-time 3,
unresolved 1, two-citation eligible 7. Expected synthetic output: six cases,
unsupported 2, current-actionable 3, historical-resolved 1, eligible 6.

## Proposed direction and acceptance boundary

Prefer evaluating a **support-and-currentness gate for new proposals**, while
leaving raw history intact. A global keyword blacklist would lose the
real-error/quoted-error pair. Treating every old report as false would lose the
old-but-reconfirmed control. Relying on the existing audit alone would leave
title/status observations with `problemArea` eligible.

The next written contract should resolve these bounded choices:

1. Define separately whether an excerpt supports the reported friction and
   whether it supports an actionable improvement now. A title, status, quoted
   fixture or path alone cannot establish friction. Evidence of a failed action
   establishes only failure at that time until current applicability is checked.
   Unknown provenance stays unknown; terminal grounding is not imposed on
   authenticated agent evidence.
2. Specify how one captured Peon evidence window contributes at most one
   occurrence per fingerprint, including candidate reordering and live/final
   overlap. Window/incident provenance must be designed rather than reconstructed
   from timestamps or rewritten into old observations. Distinct real repeated
   actions must remain countable.
3. Evaluate a 14-day recent-input window for **new** automatic proposals and
   ordinary Brain input, matching the existing audit's horizon as a candidate
   default. Bound before taking sixteen. Old records stay inspectable; new
   independently grounded evidence may reconfirm old friction. Missing/bad
   timestamps need an explicit unavailable outcome, not silent deletion or
   fabricated freshness. Age alone must never mean that a defect was fixed.
4. Preserve immutable observations, existing proposed/accepted/terminal records,
   dismissal watermarks, supersession and completion semantics. State exactly
   which creation/update paths adopt a new policy version. No automatic cleanup,
   retroactive recurrence rewrites or silent acceptance rejection follows from
   this report.

These choices are **proposals awaiting scoped owner acceptance**, including any
new provenance schema, age default or authority rule. Before runtime work, update
the authoritative Taskmaster/observation contract, resolve compatibility and
recovery, and review a concise implementation plan. No fixture label is a
runtime classifier and no canned model response qualifies semantic improvement.

## Next verification

Freeze this corpus as the initial investigation sample. Add independently
adjudicated current useful improvements and a separate held-out set before
claiming quality gains. Measure support errors, correct unknowns, current-action
usefulness, false suppression and actionable positive retention separately.
For a paired policy/model comparison, report per-case input/output identities
and denominators; require explicit finite authorization before any live model
calls. Do not recredit synthetic controls as production success.

Approved implementation coverage should exercise actual capture through
publication, same-window replay/index reordering, live/final overlap, true
later repetitions, malformed/expired times, agent evidence, immutable reads,
watermark resurfacing, restart and protected history. Existing substring or
transport tests alone cannot prove semantic quality.

Verification of this documentation/corpus increment is recorded with its PR.
No inference call, coding harness, native app session or recommendation cleanup
was started for this investigation.
