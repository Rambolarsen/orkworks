# Turn workflow friction into a next step

Taskmaster looks across recorded workflow observations and suggests
improvements to the way you work. For example, repeated missing context may
lead to a suggestion to improve repository instructions.

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
  backlog. A Cleanup card lists recommendations that no longer qualify for
  their evidence (too little support, probable noise, stale, or duplicates),
  so you can dismiss several stale cards at once with **Run cleanup**.
  Nothing is dismissed until you approve the card.

Analysis recommendations judge what your workflow is missing; Observations
recommendations target what already went wrong (and an Analysis rollup may
bundle both angles). Both are suggestions you review and act on yourself.

## Model choices and analysis status

In Taskmaster settings, model suggestions follow the selected provider. For
built-in Codex and Ollama, choose **Refresh models** to request the provider's
current model list; opening Settings does not make a provider request. Claude
Code and custom providers keep their configured suggestions, and you can still
enter a model ID manually.

Taskmaster shows analysis activity separately from provider readiness. The
Settings page and Recommendations panel report when an analysis is queued or
running, and its latest success, failure, or interruption. A failure includes
the provider/model and a short error summary, so you can distinguish a failed
analysis from a provider that is simply not configured.
If the model's rollup grouping could not be used, the analysis still succeeds
with your individual recommendations and the status reads `Rollups degraded`
followed by a short reason code.

A rollup card also shows a proposed change: the model's suggested edit or new
file targets and how to verify the change. It is a suggestion for the
receiving session to check, not evidence; paths that look sensitive are
flagged.

## Inspect before acting

A workflow-improvement card includes the proposed change, its target, and
supporting observations. Read that evidence before deciding whether the
suggestion is useful. Observations can come from coding agents or Peon;
their confidence is evidence to weigh, not a guarantee.

- **Dismiss** declines the suggestion.
- **Fix with AI** sends a scoped fix prompt into your currently active
  session. It requires an active session and your explicit action.
- **Analyze now** is currently unavailable until privacy-qualified signed reference
  knowledge is ready. Once that prerequisite is met, it asks the Brain to look for
  an improvement on demand. If one
  is already proposed or being implemented, Taskmaster asks you to handle that
  recommendation first. It works even when automatic Background discovery is
  off, and manual analyses are not limited by the daily analysis allowance —
  that limit governs automatic Background discovery only.

This action does not start a new session. The coding agent in your selected
session carries out the work under your repository’s normal instructions.
Taskmaster does not merge changes or accept work on your behalf.

## Current scope and future direction

The current workflow-improvement surface is one part of the broader
[Taskmaster design](/specs/taskmaster). That specification also describes
review and verification chains and approved session transitions. Treat it
as design scope, not a checklist of features available in an installer.
