# Bounded terminal replay with durable summary checkpoints

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-07-21

## Context

Persisted terminal output serves two different needs: recent raw replay for
reattaching a terminal, and longer-term context about what a session has done.
Keeping enough raw output for both makes terminal history unnecessarily large,
even after the byte limit introduced by PR #185. Raw output is also a noisy and
fragile source for reconstructing meaningful session progress.

The existing append-only NDJSON event log already records accepted Peon
inferences and deterministic attention reports. Those events can carry durable,
derived summary checkpoints without introducing another persistence format.

## Decision

- Persisted raw terminal replay is bounded to the newest 1,000 lines and 1 MiB.
  Existing oversized files are trimmed on their next append; dormant files are
  not proactively migrated.
- Accepted Peon inference and attention-report events may include optional
  `summary` and `source` fields. Their absence remains valid so existing NDJSON
  logs stay readable.
- A checkpoint preserves the accepted summary text exactly, rejects
  whitespace-only text, and is appended only when it differs exactly from the
  most recent stored checkpoint. Returning to earlier text after an intervening
  summary creates another checkpoint.
- Checkpoints use the provenance of the accepted update: `peon` for Peon
  inference, and the accepted `agent` or `debug` source for attention reports.
  Ignored, discarded, missing, or persistence-failed updates do not create a
  checkpoint.
- `GET /sessions/:id/summary-log` exposes checkpoints in append order as
  timestamp, summary, source, and nullable confidence. It does not expose
  internal event types or status fields. Missing data returns an empty list.

## Consequences

- Terminal reattachment uses substantially less persisted raw output while the
  event log retains a compact history of meaningful session summaries.
- Consumers can read old and new event records without a migration.
- Consecutive duplicate summaries do not inflate the log, while real summary
  transitions remain visible.
- Summary checkpoints are derived history, not a new authoritative session
  state source.
- Pagination, event-log retention, renderer consumption, proactive migration,
  and the whole-file terminal read/trim allocation tracked in #192 remain
  separate work.
