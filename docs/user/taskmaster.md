---
type: "User Guide"
title: "Turn workflow friction into a next step"
description: "User guide to turn workflow friction into a next step in OrkWorks."
tags: ["orkworks", "user-guide"]
---

# Turn workflow friction into a next step

Taskmaster looks across recorded workflow observations and suggests
improvements to the way you work. For example, repeated missing context may
lead to a suggestion to improve repository instructions.

In current source builds, **Recommendations** opens in the main view at every
window width. **Back to Terminal** (or **Back to Review**) returns to the previous
view; Escape within Recommendations does the same. Selecting a session opens
its terminal. Sessions keeps its normal responsive placement. Published installers
may still show Recommendations in the inspector.

## Two kinds of recommendations

Recommendations from either pipeline carry an origin badge (anything with an
unrecognized history shows none), and the Recommendations panel header has an
All · Analysis · Observations · Cleanup filter so you can focus on one kind at
a time.

- **Analysis** — recommendations produced by the Brain's model analysis,
  which looks for missing agentic-workflow capabilities by comparing your
  setup against signed reference knowledge, repository facts, and current
  best practices — sometimes grounded in repository facts alone, without
  any recorded session observation. The Analysis badge also covers
  model-assisted rollups that group several similar recommendations into
  one card; such a rollup draws on observed friction rather than purely on
  best practices. These appear from the Background discovery poll or when
  you press **Analyze now** — either way they are the same kind of
  recommendation.
- **Observations** — recommendations built deterministically from problems
  actually observed in your sessions: friction that coding agents or Peon
  recorded while work was happening, such as repeated obstacles or
  workarounds.
- **Cleanup** — maintenance proposals Taskmaster generates about its own
  backlog. A Cleanup card lists its observation-based recommendations that
  no longer qualify for their evidence (too little support, probable noise,
  stale, or duplicates), so you can dismiss several stale cards at once with
  **Run cleanup**.
  Nothing is dismissed until you approve the card.

  If a later audit finds that every recommendation is healthy, it retires the
  older cleanup proposal without dismissing its listed recommendations.

Analysis recommendations judge what your workflow is missing; Observations
recommendations target what already went wrong (and an Analysis rollup may
bundle both angles). Both are suggestions you review and act on yourself.

## Model choices and analysis status

In current source builds, **Settings → Recommendations** shows a model dropdown
for the selected provider. Built-in Codex and Ollama load their current lists
automatically; **Refresh model suggestions** retries or updates the list. Claude
Code and custom providers use configured suggestions. An empty model gets the
first suggestion as a draft default; saved models and edits made while loading
are preserved. You can enter a custom model ID, and changes wait for **Save
recommendation settings**. Older installers may require manual refresh.

Taskmaster shows analysis activity separately from provider readiness. The
Settings page and Recommendations panel report when an analysis is queued or
running, and its latest success, failure, or interruption. A failure includes
the provider/model and a short error summary, so you can distinguish a failed
analysis from a provider that is simply not configured.
New successful analyses also report how many recommendations were created or
suppressed as duplicates. A confirmed cache hit reports unchanged inputs and
keeps the previous analysis result. A successful run with no new proposals is
a valid outcome; older saved results without counts remain generic success.

If the model's rollup grouping could not be used, the analysis still succeeds
with your individual recommendations and the status reads `Rollups degraded`
followed by a short reason code.

A rollup card also shows a proposed change: the model's suggested edit or new
file targets and how to verify the change. It is a suggestion for the
receiving session to check, not evidence; paths that look sensitive are
flagged.

## Assess one next improvement

Current source builds offer **Assess workflow** beside **Analyze now**. It uses
your selected Taskmaster model, current context settings, and signed assessment
guidance to assess one next workflow improvement. Its report explains the
observed state and presents one grounded proposal or a reason why the supplied
evidence does not justify a proposal. It starts with current instructions,
manifests, workflow guidance and CI, then may select one bounded set of related
permitted files before judging the candidate. It makes at most two calls through
the same selected model, without retries. Progress shows discovery, reading
evidence and assessment. Workflow-context access supplies workflow files;
source-code access also permits related code and tests. Observation-only access
cannot supply the repository facts this action requires.

The report shows cited repository excerpts, Brain page provenance, and checks
that remain unverified. Bounded excerpts establish what was supplied; omitted
files remain unknown. A reused report is labeled explicitly. Assess workflow
works with Background discovery off and does not spend its daily allowance or
cooldown. An active Brain recommendation must be handled first. **Retry assessment**
starts a fresh attempt after interruption; discovery is never replayed after
restart, and a live run is not started twice.

A proposed improvement uses the existing recommendation card and **Fix with AI**
action. Assessment itself does not edit files, run repository checks, or start a
session. Changed inputs hide an old report and prevent stale recommendations
from being handed off; reducing context access redacts saved assessment evidence
and generated text, including dismissed or completed recommendations.

## Inspect before acting

A workflow-improvement card includes the proposed change, its target, and
supporting observations. Read that evidence before deciding whether the
suggestion is useful. Observations can come from coding agents or Peon;
their confidence is evidence to weigh, not a guarantee.

- **Dismiss** declines the suggestion.
- **Fix with AI** sends a scoped fix prompt into your currently active
  session. It requires an active session and your explicit action.
- **Analyze now** asks the Brain to look for an improvement on demand when a
  supported Taskmaster model and verified signed knowledge are available.
  Current source builds include an eligible offline starter. If an improvement
  is already proposed or being implemented, Taskmaster asks you to handle that
  recommendation first. It works even when automatic Background discovery is
  off, and manual analyses are not limited by the daily analysis allowance —
  that limit governs automatic Background discovery only.

If a saved analysis attempt remains queued or running, **Retry analysis** asks
the app to recover its status and try again. A live run is not started twice.
A skipped request keeps the previous actual analysis outcome.

This action does not start a new session. The coding agent in your selected
session carries out the work under your repository’s normal instructions.
Taskmaster does not merge changes or accept work on your behalf.

## Current scope and future direction

The current workflow-improvement surface is one part of the broader
[Taskmaster design](/specs/taskmaster). That specification also describes
review and verification chains and approved session transitions. Treat it
as design scope, not a checklist of features available in an installer.

## Agent-readiness score and history

Current source adds **Assess readiness** in Recommendations. It assesses only
the selected workspace and shows an overall score out of 100, with fourteen
individual 0–4 maturity ratings, confidence, saved evidence and suggested next
improvements. Run it manually after repository changes to compare assessments.
The overall score weights every area equally; a partially assessed repository
shows coverage and individual ratings without an overall number.

Each completed result retains its date, commit/worktree state and evaluation
conditions. Select an older assessment or a comparison baseline in history.
Deltas appear only for full assessments with comparable rubric, model and access
conditions. An unchanged-input rerun is a new model judgment; a higher score does
not prove that one particular fix caused the improvement. Saved evidence is a
historical excerpt, not a claim that current tests passed. The action itself
does not execute repository commands.

History retains up to 100 snapshots locally for this canonical workspace. Failed
attempts preserve previous results; repository changes mark the last assessment
for a rerun. Reducing Taskmaster context access redacts affected historical scores
and evidence, and expanding access again does not restore them.

Current source reports the failed stage and a safe reason when a readiness
attempt cannot finish, such as a model timeout, invalid response, changed inputs,
or unavailable history storage. Older attempts with only a generic failure code
remain generic. Input changes interrupt an attempt; they do not replace a saved
score. Workflow assessments likewise retain safe provider and validation reasons
without exposing raw provider output.

This feature requires complete verified signed Brain rubric guidance and an
available configured Taskmaster model. Signed publication and the generated
starter are verified. The [qualified fixture profile](../validation/repository-readiness-runtime.md#qualified-profile)
is Codex **GPT-6.1-Sol with low reasoning effort**; GPT-6-Luna medium did not meet
the quality bar. Select the desired model in Taskmaster settings, which are shared
with other Taskmaster analysis. The feature does not change your model selection.
Source validation does not establish availability in an existing installer. Existing **Assess workflow** continues
to suggest one bounded improvement separately.
