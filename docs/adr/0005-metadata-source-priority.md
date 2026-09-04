# Metadata source priority

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

Multiple systems can provide session metadata: the user manually, agents writing to `.orkworks/`, Peon inference, backend deterministic inference, and bare process state. When sources disagree, OrkWorks needs a deterministic way to pick the authoritative value.

## Decision

Metadata priority is explicit and ordered: user > agent > peon > backend_inference > process > unknown > debug. Every piece of session metadata carries a `metadataSource` and `metadataConfidence` field. Higher-priority sources are never overwritten by lower-priority ones unless the higher-priority data is stale or explicitly cleared.

## Consequences

- User overrides always win, preserving manual control
- Agent-written metadata is trusted more than Peon inference
- Peon can fill gaps without overriding intentional agent reports
- Debug-only injections stay visible long enough for convergence testing without outranking real runtime writes
- Confidence fields let the UI surface uncertainty (e.g., "Peon thinks this is blocked")
- Clear ordering prevents conflicting writes from causing flip-flopping state

## Amendment (2026-09-03, issue #400)

The ladder is now enforced as a unit by `source_priority::can_overwrite` in
`crates/orkworksd/src/metadata.rs` instead of being re-derived per write path
(the split across `metadata.rs`, `peon.rs`, and `session_application.rs` had
drifted into conflicting staleness windows). Two operational decisions are
encoded there and pinned by tests:

- The "stale" escape hatch for the peon→agent pair is 15 seconds of metadata
  inactivity (not the historical 300s variant, which had no production
  caller): long enough to avoid Peon flickering against a hook signal that
  just landed, short enough that fresh terminal output can correct a stuck
  attention signal quickly.
- "Debug-only injections ... without outranking real runtime writes" is
  enforced literally: `debug` overwrites every source except the two
  live-signal tiers (`user`, `agent`), so debug injection remains usable on
  live sessions whose state is `process`/`peon`.
