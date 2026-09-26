---
type: Troubleshooting Guide
title: Peon model detection troubleshooting
description: How to interpret Peon model-detection observations and model metadata without chasing self-referential noise.
tags: [peon, troubleshooting, providers, diagnostics, taskmaster]
status: stable
---

# Peon model detection troubleshooting

Use this runbook when Peon reports a workflow observation about "Peon model
detection", when a Taskmaster recommendation asks you to "remove or document
the obstacle: Peon model detection issues", or when a recommendation asks you
to remove or document a capacity-related obstacle such as "Tracing signal
capacity in Claude harness to OpenCode". Before treating it as a product
defect, check whether the observation is self-referential noise.

## Two different things called "model detection"

- **The Peon inference provider/model.** The provider/model pair Peon uses for
  its own session inference is chosen in Settings → Model providers. That pair
  is authoritative for what runs session inference. Verify a suspected
  inference problem there first.
- **The session's displayed model.** Peon may infer a `detectedModel` from
  terminal output (a model identifier the coding tool shows on screen). It is
  best-effort metadata that fills the session's model field:
  - It fills the session model only while that field is empty — the first
    detection wins and nothing overwrites it for the session's lifetime.
  - When Peon's applied provider model is explicitly known, Peon does not
    attribute that exact model to the session. With a default or unknown
    provider model the exclusion cannot match, so Peon's own actual model can
    be attributed.
  - When no model was detected, the session view falls back to the model
    requested at launch or the harness definition's default; if neither
    exists the UI shows `Unknown`. That fallback is launch configuration, not
    a runtime harness report.
  - It is not purely display metadata: a custom harness whose resume
    template contains `{model}` substitutes the persisted session model into
    the resume command, so a stale detection changes what that command
    targets.
  - There is no manual override route for a wrong session model today. The
    user-override gap tracked for Peon-derived status/label (#473) does not
    cover model. Do not attempt to "fix" a stale detection by reopening or
    resuming another session.

## Known self-referential observation noise

Peon's inference prompt contains worked examples, and sessions that work on
OrkWorks itself make that vocabulary appear in terminal output (source code,
test fixtures, prompt-example labels such as "Fixing peon model detection").
An observing Peon can then report the session's own work topic as an
obstacle — descriptions like "Peon model detection", "model detection issue",
or "fixing peon model detection" with generic evidence such as "Terminal
output". The label side of this contamination is guarded (#558); the
workflow-observation side relies on the model obeying the prompt, and the
rollup design names this exact fingerprint cluster as motivating noise.

Signals that an observation is this noise, not a verified defect:

- The description names Peon's prompt-example vocabulary rather than a
  concrete failure in the session.
- The evidence field is a generic string ("Terminal output", "Code changes")
  rather than a specific excerpt. Evidence must appear verbatim in the
  captured terminal output, so such a match only proves those words appeared
  somewhere in the capture — UI chrome, quoted text, or the session's own
  work — which is low-specificity and a strong noise signal.
- Several near-identical observations were recorded in one burst from a
  single final scan.
- Rechecking the current code finds no detection defect: the
  `detectedModel` merge rules above are intentional behavior.

## Capacity/cap obstacle noise

A related fingerprint family turns harness UI chrome about usage caps into a
false "capacity" obstacle. Peon's inference prompt itself asks for
`capacityHints` (cap/rate-limit strings), so "capacity" is Peon prompt
vocabulary; when a session's terminal capture shows another session's title
or a resume banner (OpenCode session lists and resume output in particular),
Peon can conflate the two and report an obstacle like
"Tracing signal capacity in Claude harness to OpenCode" that names a
subsystem which does not exist.

Verified example (September 2026, one observation, one OpenCode session): the
only terminal grounding was the OpenCode banner lines
`Session   Claude cap affecting opencode usage` and
`Continue  opencode -s <id>` — a human-written label about Claude usage-cap
limits plus a resume hint. The phrases "signal capacity" and "Search results
show" appeared zero times in the ANSI-stripped terminal capture, so the
required verbatim evidence excerpt was ungrounded; the description
paraphrased a label, not the session's work.

Signals that an observation is this noise, not a verified defect:

- The obstacle description names "signal capacity" or describes capacity
  being "traced" from one harness to another. OrkWorks has no cross-harness
  capacity propagation: capacity is per-session, per-harness usage-limit
  detection via harness `capacity_patterns()` scans held in session and
  provider state, and Peon `capacityHints` are strings attached to the
  session that produced them — they never propagate to another session as
  capacity state. (Terminal text can still be sent to whatever model
  provider Peon inference has applied, including a provider backed by a
  different coding tool; that is inference traffic, not capacity state
  moving between sessions. The `capacity/<id>.json` metadata path is
  protocol design, not an implemented detection mechanism.)
- The observation's `evidence` field does not appear verbatim in the
  ANSI-stripped terminal capture (strip `\x1b[` escape sequences before
  searching). Peon's prompt and `evidence_is_grounded` pin only the
  evidence field as a contiguous verbatim excerpt; the `description` may
  legitimately paraphrase, so a paraphrased description alone is not a
  noise signal.
- The only verbatim match is UI chrome: session labels, banners, resume
  hints, or session lists. Those strings are other sessions' titles and
  launch commands, not this session's work.

Handling is the same as below: if it is confirmed noise, documentation is a
valid resolution — do not act on the obstacle, and do not modify
recommendation or observation files directly.

## Recommended handling

1. Recheck the current files. Peon observations are hypotheses, not proof of
   recurrence or of absent policies; a single high-impact observation from one
   session does not establish a real defect.
2. Determine first whether the report is noise. If it is, documenting the
   known limitation in repository tooling or documentation is a valid
   resolution; do not modify recommendation or observation files directly.
   If investigation instead finds a real, reproducible defect, follow the
   normal issue and implementation workflow — the documentation-only path
   applies to confirmed noise, not to a genuine regression.
3. If you are genuinely debugging inference output, verify the applied
   provider/model in Settings → Model providers, and use
   [Peon timeout troubleshooting](peon-timeout-troubleshooting.md) for
   provider timeouts. Keep the two diagnoses separate.
4. Do not resume, reopen, or modify another session to work around the issue,
   and do not loop retries.
