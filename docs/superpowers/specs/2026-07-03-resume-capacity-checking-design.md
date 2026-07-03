# Resume Capacity Checking

**Date:** 2026-07-03
**Status:** Approved

## Problem

When a capped session is resumed, OrkWorks briefly shows the old or inherited capacity state in the UI before the next live capacity scan completes. The visible result is misleading: the resumed session and the provider panel can momentarily imply either that the cap has already cleared or that the session is definitively still capped, even though OrkWorks has not yet revalidated the harness against fresh terminal output.

This is a state reconciliation problem, not a launch-policy problem. Users should still be able to select and launch Codex during this window.

## Scope

- Add an explicit transient post-resume capacity-checking state for live sessions
- Show that transient state on resumed session surfaces and in the provider/capacity panel
- Clear the transient state after the first real post-resume capacity reconciliation pass
- Keep harnesses selectable while the transient state is active

Out of scope: blocking resume while capped, timer-based heuristics, changing provider launch gating, changing usage-limit detection patterns.

## Design

### 1. Backend-owned transient state

Add a transient boolean on live session handles and session API payloads for sessions whose capacity must be revalidated after resume:

```rust
capacity_check_pending: bool
```

This is not a persisted capacity verdict. It only means:

> This session was just resumed and OrkWorks has not yet completed its first post-resume capacity reconciliation pass.

The existing `at_usage_limit` field remains the actual verdict once reconciliation completes.

### 2. Set pending on resume

In `POST /sessions/:id/resume`:

- preserve the current behavior that clears `at_usage_limit_latched`
- set `capacity_check_pending = true` when the harness supports capacity detection
- leave `at_usage_limit = None` until the next `list_sessions` reconciliation

Harnesses without capacity detection should not enter this transient state.

### 3. Clear pending on the first real reconciliation pass

`list_sessions` already performs the authoritative post-resume check by:

- collecting live session buffers
- recomputing `at_usage_limit`
- propagating harness-wide capped state across matching sessions

Extend that flow so that, for any live session with `capacity_check_pending = true`, OrkWorks clears the pending flag during the first pass that evaluates fresh usage-limit state for that session.

The important rule is:

- `capacity_check_pending` is tied to reconciliation work, not elapsed time
- once cleared, the normal `at_usage_limit` and harness-wide propagation logic remain the only source of truth

### 4. Provider-panel display state

While any live session for a harness has `capacity_check_pending = true`, the provider response for that harness should expose a transient display state of:

```text
checking_capacity
```

This display state takes precedence over `healthy` and `capped` in the provider/capacity panel until the pending session finishes its first reconciliation pass.

This does not change provider enablement or launch rules. It is informational only.

### 5. Session-surface display state

For a live resumed session with `capacity_check_pending = true`, session-facing UI should render:

```text
Checking capacity
```

This state replaces any immediate `capped` or `healthy` label during the post-resume window. After the pending flag clears, the session returns to the existing capacity display based on the reconciled `at_usage_limit` result.

### 6. Selectability rule

`checking_capacity` must not be treated as an unavailable or blocked provider state.

Codex and other harnesses remain selectable during this window because:

- the user explicitly wants selection to remain available
- the transient state means "verification in progress," not "launch denied"
- OrkWorks already treats capacity as informational rather than a hard gate for session creation in this area of the product

## API impact

Add an optional session field:

```rust
#[serde(rename = "capacityCheckPending", skip_serializing_if = "Option::is_none")]
capacity_check_pending: Option<bool>,
```

TypeScript session types should mirror it as:

```ts
capacityCheckPending?: boolean;
```

Provider responses should surface `checking_capacity` as an additional effective display state without changing the underlying provider configuration model.

## Implementation notes

- Prefer keeping the transient flag on the in-memory `SessionHandle`, because this is runtime reconciliation state rather than durable workspace metadata.
- Do not use a fixed timeout. Slow scans and fast scans should both produce honest UI.
- Do not broaden this into a generic provider-state refactor; keep the change constrained to the post-resume path.

## Verification

Add tests for:

- resume marks a capacity-detecting session as pending
- the first `list_sessions` reconciliation pass clears the pending flag
- the provider panel shows `checking_capacity` while any live session for that harness is pending
- `checking_capacity` does not make the provider unselectable
- once pending clears, existing `capped` propagation behavior still works unchanged
