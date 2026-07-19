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
