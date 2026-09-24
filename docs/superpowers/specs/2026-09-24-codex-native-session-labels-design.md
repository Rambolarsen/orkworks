# Codex Native Session Labels

Status: approved design
Date: 2026-09-24
Issue: #602

## Goal

Use Codex's own saved session name or generated title as the OrkWorks session
label when the sidecar can resolve the exact native session safely. Keep the
existing label fallback behavior for every other case.

## Contract

For a live Codex session whose accepted native session ID is known, OrkWorks
may read the supported local Codex state store and select the first non-blank,
display-safe value in this order:

1. `threads.name` (the saved name exposed by Codex's `/rename` flow)
2. `threads.title` (the Codex-generated title)

The first implementation does not use `first_user_message` or parse rollout
JSONL. Raw prompt text stays out of the session label path.

The label is automatic display metadata, separate from attention, summary, and
the global metadata-source priority ladder. Persisted label provenance is
explicit so a native result can replace placeholder, bootstrap, terminal, or
Peon labels but cannot replace a future explicit OrkWorks user override.

## Store boundary

The adapter supports only the known local Codex store shape and path contract:
`$CODEX_HOME/state_5.sqlite`, or `~/.codex/state_5.sqlite` when `CODEX_HOME`
is absent. It does not scan for databases, select by modification time or
numeric suffix, follow `rollout_path`, or accept a caller-supplied path. Other
profiles, managed stores, remote/WSL/container namespaces, and unknown schema
versions fall back without enrichment until separately verified.

The adapter uses a bundled Rust SQLite implementation, read-only access,
parameterized exact-ID queries, bounded busy waits, and no journal or schema
mutation. Blocking database work runs outside Tokio and application locks.

## Identity and lifecycle

Codex hook identity reports remain backward-compatible for existing callers.
Native label enrichment is attempted only when the report is accepted for a
Codex session and the request carries that live session's
`ORKWORKS_REPORT_TOKEN` bearer capability. The reporter forwards the token
when it is present; external/untrusted callers can still report identity under
the existing contract but cannot trigger private-state enrichment.

Enrichment is opportunistic and triggered by accepted Codex identity reports,
with bounded retries for a row that has not yet been committed. A failed or
missing read never clears a previously accepted native label. The result is
applied only if the session is still live, still bound to the same native ID,
and still carries an automatic label source. Workspace replacement, runtime
replacement, label reset, or a user override invalidates the result.

Native text is treated as display data only: controls are rejected, whitespace
is normalized, and the display length is bounded. Candidate text and full
database rows are never logged.

## Non-goals

- Historical backfill of ended sessions.
- Filesystem watching or a permanent Codex database index.
- Raw transcript/JSONL parsing.
- Using `first_user_message` as a label source.
- A user-facing rename/override control (tracked separately by #473).
- Inferring attention, summary, workflow evidence, or task state from Codex
  title text.

## Verification

Tests cover name/title precedence, blank fields, unsupported/missing stores,
sanitization, exact native-ID matching, label-source ownership, capability
gating, and stale identity/result guards. Required sidecar build, test, and
format checks remain the completion gate.
