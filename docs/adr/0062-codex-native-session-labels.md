# Codex native session names enrich automatic session labels

- Status: accepted
- Deciders: Rambolarsen, Codex
- Date: 2026-09-24

## Context

ADR 0042 preserves a stable one-shot session label, and ADR 0047 adds a
replaceable initial-prompt fallback. Codex already exposes a native session ID
through the accepted hook integration, and Codex persists saved names and
generated titles locally. OrkWorks currently ignores those names and relies on
the generic placeholder/terminal/Peon label flow.

The local Codex state database is useful but private and not a public schema
contract. A direct database read must not let an unauthenticated localhost
caller publish arbitrary Codex text, guess among user profiles, mutate Codex's
database, or race a newer session label into the wrong OrkWorks session.

## Decision

Add a read-only Codex state adapter behind the existing accepted native-session
identity path. The first supported store is exactly `$CODEX_HOME/state_5.sqlite`
or `~/.codex/state_5.sqlite`; unknown paths and schemas are unsupported and
fall back silently. The adapter queries `threads` by the exact native session
ID and selects `name`, then `title`. It never reads `first_user_message`,
rollout JSONL, or `rollout_path` for labeling.

Persist label provenance separately from the global metadata source ladder.
Native Codex labels may replace only automatic sources (placeholder, bootstrap,
terminal, and Peon). A future explicit OrkWorks user source wins. Missing,
blank, unreadable, locked, or unsupported native data preserves the current
label and never signals deletion.

Codex label enrichment requires the live session's existing reporting
capability. The Codex reporter forwards `ORKWORKS_REPORT_TOKEN` when available;
identity reporting itself remains backward-compatible when the token is absent.
Accepted reports trigger bounded, opportunistic read attempts. Applying a
result revalidates the live session, native ID, runtime/workspace identity, and
automatic label ownership before writing the durable and live projections.

Native title text is display data only. It is normalized and bounded, controls
are rejected, and it is never used as attention, summary, workflow evidence,
or instructions. SQLite work is bundled, read-only, parameterized, bounded,
WAL-aware, and executed outside async/application locks.

## Consequences

Codex sessions acquire recognizable names without spending Peon inference on a
task that Codex already names. The existing Peon and terminal paths remain the
fallback for other harnesses and for unsupported Codex installations.

The sidecar gains a packaged SQLite dependency and a small private-schema
adapter that must be feature-verified when Codex changes its store. Native
renames are opportunistic rather than guaranteed to be observed while idle;
filesystem watching and broad profile discovery remain future work. A separate
label provenance field is required for safe coexistence with Peon and the
future manual override in #473.
