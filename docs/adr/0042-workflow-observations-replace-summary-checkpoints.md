# Workflow observations replace summary checkpoints

- Status: accepted (the reachable-status clause of the "Embedded
  recommendation evidence" decision below is superseded by ADR 0045)
- Deciders: Rambolarsen
- Date: 2026-08-14

## Context

ADR 0024 gave the event log durable summary checkpoints derived from accepted
Peon inference and attention reports, exposed through `GET
/sessions/:id/summary-log` and rendered by the desktop as "Task history." ADR
0029 built the stable, one-shot session label on top of that same `summary`
field and its checkpoint history.

A prose activity-summary log cannot reliably represent the recurring actions,
obstacles, missing context, assumptions, corrections, workarounds, and
verification gaps that should drive better repository instructions, skills,
tests, tooling, or documentation. OrkWorks needs two deliberately separate
records: current session situation, used for coordination, and durable
workflow-friction evidence, used for improvement recommendations. See
`docs/superpowers/specs/2026-08-14-workflow-observation-feedback-loop-design.md`
for the full design this ADR ratifies.

Note on numbering: the design that produced this ADR was authored on
2026-08-14 and anticipated ADR number 0041. By the time this ADR was written,
ADR 0041 ("Session runtime generation ownership") had already been accepted
under that number for an unrelated decision. This ADR is filed as 0042 to
avoid colliding with it; its `Date` still reflects when the underlying design
was approved.

## Decision

This ADR supersedes ADR 0024 and ADR 0029. It restates their still-true
decisions and replaces the durable-summary-checkpoint mechanism:

- **Bounded terminal replay stays** (surviving ADR 0024 decision): persisted
  raw terminal replay remains bounded to the newest 1,000 lines and 1 MiB,
  trimmed on append; existing oversized dormant files are not proactively
  migrated.
- **Stable one-shot label stays** (surviving ADR 0029 decision): `label`
  remains a one-shot, Peon-authored topic seeded synchronously from the first
  descriptive user input and refined once by Peon's `InputLabel` inference. It
  remains outside the ADR 0005 metadata source/confidence precedence system.
- **Current-summary snapshot replaces borrowed provenance**: `summary`
  becomes a first-class snapshot carrying its own `summarySource` (`agent` |
  `peon`), `summaryConfidence`, and `summaryObservedAt` fields, all four
  updated or cleared together. An accepted non-empty Peon summary, or an agent
  attention message that includes a message, replaces all four fields; an
  attention report without a message leaves them unchanged; a new descriptive
  user instruction or an accepted label-reset command clears all four
  synchronously. Taskmaster's session-coordination inputs use only summaries
  carrying these dedicated fields; legacy sessions with only a flat `summary`
  remain displayable in the selected-session headline but are not Taskmaster
  handoff evidence until a new accepted summary populates the dedicated
  fields.
- **Workflow observations provide durable improvement evidence**: immutable
  `WorkflowObservation` records (`id`, `sequence`,
  `sessionId`, `observedAt`, `kind`, `description`, `evidence`,
  `reportedImpact`, `source`, `confidence`, `fingerprint`,
  `idempotencyKeyHash`) are accepted through one workflow-evidence module,
  reachable only from an authenticated explicit agent-report route (`POST
  /sessions/:id/workflow-observations`, guarded by a per-session,
  non-persisted `ORKWORKS_REPORT_TOKEN` capability) and a Peon inference
  adapter. They are workspace-scoped but session-segmented NDJSON under
  `~/.orkworks/workspaces/<hash>/workflow-observations/`, bounded per session
  to the newest 1,000 records/2 MiB, with a durable `sequence` counter file
  ordering workspace-wide reconstruction (newest 10,000 records). The
  separate current-summary migration and removal of summary checkpoints are
  planned follow-up work; the existing `GET /sessions/:id/summary-log` route,
  desktop "Task history" surface, and event records remain supported and
  readable for now.
- **Exact deterministic Taskmaster correlation**: Taskmaster evaluates
  accepted observations within the active workspace, five seconds after the
  latest accepted observation. A cluster qualifies when it contains at least
  two distinct observations sharing a fingerprint with individual confidence
  at least `0.6`, or one observation with `reportedImpact: high` and
  confidence at least `0.8`. Each observation kind maps to one fixed target
  surface and recommendation-text template; version 1 never combines
  different fingerprints.
- **Embedded recommendation evidence**: a qualifying cluster produces the
  passive `improve_workflow` variant of the canonical Taskmaster
  recommendation contract — `requiresApproval: false`, no accept/execute
  action, and only `proposed`/`dismissed` reachable in this version (the
  original clause listing `superseded` as reachable was superseded by
  ADR 0045; see there for the dismissal-immutability rationale). Each recommendation embeds immutable snapshots of every cited
  observation, so ordinary segment trimming cannot invalidate an existing
  card. Dismissal persists an evidence watermark (`dismissedAt`,
  `dismissedThroughSequence`, qualifying observation IDs/count, highest
  impact, affected sessions); Taskmaster creates a new linked successor only
  when the highest impact increases, or when at least two qualifying
  observations past the watermark include a session absent from it.
- **Planned removal of summary checkpoints**: ADR 0024's checkpoint-writing
  and `summary-log` read paths will be removed in favor of the mechanisms
  above once the current-summary migration lands.
  `DELETE /sessions/:id/forget` and automatic retention delete a session's
  observation segment and every recommendation referencing it, in the same
  cleanup path as session metadata and events.

## Consequences

- The session detail panel keeps using the latest `summary` for its situation
  headline; the removed "Task history" list is not replaced by workflow
  observations in the session-detail surface — improvement evidence belongs
  to Taskmaster, not session-detail activity presentation.
- Coding agents gain a harness-neutral, authenticated way to report workflow
  friction without a harness-specific protocol; Peon gains a second, optional
  inference output alongside session-situation inference. Neither worker
  silently changes the repository.
- Taskmaster gains a concrete, deterministic, evidence-backed improvement
  surface without expanding its explicit-approval model — `improve_workflow`
  recommendations are dismissible only, never auto-applied, and never create a
  GitHub issue or start a session on their own.
- ADR 0024 and ADR 0029 remain historically accurate for the decisions they
  made at the time; both are marked superseded by this ADR rather than
  deleted, and their surviving decisions restated above remain authoritative
  going forward.
- Existing NDJSON event logs and terminal replay files require no migration;
  only new checkpoint writes and the summary-log route are removed.
