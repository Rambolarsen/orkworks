---
type: Troubleshooting Guide
title: Peon model detection troubleshooting
description: How to interpret Peon model-detection observations and model metadata without chasing self-referential noise.
tags: [peon, troubleshooting, providers, diagnostics, taskmaster]
status: stable
---

# Peon model detection troubleshooting

Use this runbook when Peon reports a workflow observation about "Peon model
detection", "Model detection is blocked", or "Model detection failed"; when a
Taskmaster recommendation asks you to "remove or document the obstacle" for
any of those phrasings (for example: "Peon model detection issues", "Model
detection is blocked", a capacity-related obstacle such as "Tracing signal
capacity in Claude harness to OpenCode", or a rate-limit obstacle such as
"rate limit reached"). Before treating it as a product
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
obstacle. The descriptions vary in how much of the "Peon" qualifier they
keep — from "Peon model detection", "model detection issue", or "fixing peon
model detection" down to bare statements such as "Model detection is blocked"
that name no observer at all — and the evidence can be a generic string
("Terminal output") or a plausible-looking error excerpt ("Error message:
Model detection failed") that no current OrkWorks code path emits. The label
side of this contamination is guarded (#558); the workflow-observation side
relies on the model obeying the prompt, and the rollup design names this
exact fingerprint cluster as motivating noise.

Signals that an observation is this noise, not a verified defect:

- The description names Peon's prompt-example vocabulary rather than a
  concrete failure in the session — including phrasings that drop the
  "Peon" qualifier entirely, such as "Model detection is blocked".
- The evidence field is a generic string ("Terminal output", "Code changes")
  rather than a specific excerpt. Evidence must appear verbatim in the
  captured terminal output, so such a match only proves those words appeared
  somewhere in the capture — UI chrome, quoted text, or the session's own
  work — which is low-specificity and a strong noise signal.
- The evidence is a specific-looking error excerpt such as "Error message:
  Model detection failed" that no application code path emits. Search for
  the persisted `evidence` value bounded to application sources and
  configuration — `apps/desktop/src/`, `apps/desktop/electron/`,
  `crates/orkworksd/src/`, plus the workspace's committed configuration —
  not the full tree or all history: once this runbook documents the
  phrasing, the runbook, the AGENTS.md pointer, and their commits are
  deliberately assumed hits, so a search that includes documentation can
  never confirm noise. Peon grounding is checked server-side against the
  output captured at record time, but the retained `events/<id>.terminal`
  file is bounded and long ago may have trimmed the excerpt's rows, so
  absence from the capture today proves nothing on its own. Check the
  applied provider/model too: an observation from a small local inference
  model alongside garbled-evidence siblings from the same scan burst fits
  this noise pattern.
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
limits plus a resume hint. The observed session's captured harness session ID
came from the OpenCode hook (`harnessSessionIdSource: opencode_hook`), and the
banner's resumable session ID differed from it — a non-circular comparison, so
the banner named another session. The persisted evidence value — "Search
results show signal capacity in Claude harness to OpenCode" — contains the
words "signal" and "capacity", and reproducing Peon's reassembly on the
retained replay (column width from `<id>.terminal-size`, escape sequences
intact) leaves zero occurrences of either word. The session kept running long
after the observation, so the retained replay may have evicted the window
Peon scanned and the reconstruction is not conclusive on its own — but it is
consistent with the other signals: the description paraphrases UI chrome, the
evidence phrase pairs Peon's own prompt vocabulary with that chrome, and no
verbatim excerpt from the observation's own work appears anywhere. Taken
together, the observation was noise; the description paraphrased a label,
not the session's work.

Signals that an observation is this noise, not a verified defect:

- The observation's `source` is `peon`. The terminal grounding checks below
  gate only Peon-origin observations; agent-reported observations are
  recorded through a separate path with no terminal grounding gate, so an
  agent observation legitimately cites evidence from repository inspection
  or another non-terminal source. Do not apply the grounding test to it.
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
- The observation's `evidence` field fails the implemented grounding gate.
  `evidence_is_grounded` performs a literal substring match against the
  hard-wrap-rejoined snapshot — reassembly runs before the provider call
  and observation recording — with ANSI escape sequences intact, so an
  excerpt split by an escape sequence is rejected even though it reads as
  contiguous after stripping. Searching an ANSI-stripped copy is a
  readability aid only: absence there proves absence only when no wrap
  reassembly could join rows into the phrase, so reproduce the reassembly —
  on the raw capture, with the recorded column width — before concluding an
  accepted observation was ungrounded; and a stripped hit does not prove
  Peon accepted it. Peon's prompt pins only the evidence field as a
  contiguous verbatim excerpt; the `description` may legitimately
  paraphrase, so a paraphrased description alone is not a noise signal.
  Test the persisted `evidence` value, not the description's wording. Treat
  reconstructed absence as inconclusive, not proof: the retained replay is
  bounded to the newest 1,000 lines / 1 MiB, so the window Peon scanned may
  have been evicted, and `<id>.terminal-size` records the last-known width,
  not necessarily the inference-time one — absence in the reconstruction is
  consistent with ungrounded, while presence in the retained capture is
  strong evidence the gate could have passed.
- The only verbatim match is UI chrome — session labels, banners, resume
  hints, or session lists — and the displayed session ID or title does not
  correlate with the observed session. A session's own resume banner names
  itself, so chrome can describe this session's work. Correlate only
  against a captured harness session ID with non-Peon provenance (hook or
  agent source): Peon-inferred IDs are persisted from visible resume
  output, so comparing a banner ID against a peon-sourced captured ID is
  circular — the banner itself may have created the match. Without an
  independently sourced ID, UI-chrome-only evidence is low-specificity,
  not proof either way.

Handling is the same as below: if it is confirmed noise, documentation is a
valid resolution — do not act on the obstacle, and do not modify
recommendation or observation files directly.

## Rate-limit obstacle observations

Another fingerprint family reports a "rate limit reached" obstacle that names
no OrkWorks failure path. Verified example (September 2026): two peon-sourced
observations across two OpenCode sessions working in this repository — one
labeled "Invalid Peon inference JSON", one "Fixing peon model detection" —
each recorded a high-impact obstacle described as "rate limit reached". Both
predate the terminal grounding gate (#589, September 2026), so their
acceptance does not assert grounded evidence.

The persisted evidence supports the noise reading rather than a defect:

- "Output indicates rate limit exceeded" is a paraphrase, not a quoted
  excerpt: the phrase appears nowhere in the retained replay and the wording
  describes output instead of quoting it.
- " rate limit reached" is a bare fragment with no source, recovery step, or
  session context attached.
- Neither session's retained replay contains "rate limit" or "usage limit" in
  any form. The windows Peon scanned have been evicted (the replay is bounded
  to the newest 1,000 lines / 1 MiB), so the absence is inconclusive by the
  grounding standard in the capacity section above — but it is consistent
  with the paraphrase and the bare fragment. Both sessions' recorded activity
  continued for hours after the observation; that is weak evidence only,
  since a genuine limit that resets allows later output just the same, and
  the evicted replay cannot date when output resumed.

The vocabulary is ordinary OrkWorks work output. Peon's inference prompt asks
for "cap/rate-limit related strings" (`capacityHints`), provider and Peon
tests pin banner shapes such as "usage limit reached, resets in 2h", and the
harness usage-limit design docs discuss rate limits. A session developing
OrkWorks can put any of that into its own captured output, and an observing
Peon model asked to find friction can lift the phrase as an obstacle.

No OrkWorks detection path keys on the phrase "rate limit reached". Among the
built-in harnesses only OpenCode ("usage limit reached"), Codex ("you've hit
your usage limit"), and Claude Code ("you've hit your session limit") carry
terminal capacity patterns; Copilot, Aider, Gemini, Antigravity, and the
generic shell define no capacity capability, so their terminal output cannot
set `at_usage_limit` or a reset hint at all. For the instrumented three, the
deterministic cap signals live on the session view and the provider state.
This observation kind is not one of them — though a genuine banner can also
produce a grounded workflow observation, since the Peon recording path
accepts any candidate whose evidence is grounded and does not filter
capacity banners, so the observation may corroborate a real cap rather than
refute it:

- Session side: the harness capacity-pattern scan sets `at_usage_limit` on
  the session view, which becomes the "capped" attention status, and, when
  the banner carries one, the reset hint renders as a "Capped · resets in
  2h" badge in the session detail panel; a cap without hint text shows just
  "Capped". The capped flag and hint are harness-wide state:
  `session_projection` aggregates by harness and copies both onto every
  session of that harness, so a banner in one session marks its peers too.
- Provider side: the providers API and the new-session dialog reflect a
  genuine terminal cap automatically — the response overlays a live
  session-capped map populated from the same harness capacity detection
  before the configured state, and shows "checking capacity" while that
  scan is pending. Separately, a provider whose configured capacity state
  is capped in Settings → Model providers is skipped for session inference
  instead of retried, and a failed provider invocation's stderr is parsed
  into an error summary plus a reset hint on the same API response. The
  configured default or override is not mutated by provider stderr, and
  Settings itself does not render that runtime state today.

When a recommendation asks you to "remove or document the obstacle: rate
limit reached", read the current signals for corroboration, not dismissal.
The session's capped attention status, reset hint, and the provider state
are present-tense, harness-wide checks: the latched cap clears once fresh
banner-free output follows accepted input, the capped flag and hint are
copied onto every session of the same harness, and the provider response
overlays the same flag — so a peer session's banner can mark the observed
session, and a limit that already reset shows none of them. A genuine
banner can also yield a grounded workflow observation of this kind, so the
existence of a current cap neither dismisses the observation nor proves
it. Ground the judgment in evidence tied to the observation time: whether
the evidence value is a paraphrase or a bare fragment, and whether the
captured vocabulary matches the session's own work. If the fingerprints
above hold — or the harness has no capacity patterns and no independently
verified limit exists — treat it as this noise family: documentation is
the valid resolution, and no code change can "remove" the obstacle. Do not
act on the obstacle, and do not modify recommendation or observation files
directly.

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
