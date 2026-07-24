# Observed-status/attention writes owned by one runtime module

- Status: accepted
- Deciders: Rambolarsen
- Date: 2026-07-24

## Context

Writing a session's `observed_status`/`attention` requires updating two
stores in agreement: the in-memory `SessionHandle.info` (`main.rs`) that the
frontend reads live, and the persisted `SessionMetadata` (`metadata.rs`) that
survives a sidecar restart. `metadata.rs` has one well-encapsulated function
for the persisted half, `merge_agent_attention_signal_with_plan`, with real
priority rules (`user` > `agent`/`debug` per ADR 0005's source priority). But
every caller that also needs to keep the live handle in sync re-derives the
same field set by hand, with subtly different rules each time:

- `report_attention` (`http/session_handlers.rs:521`) — external hook reports,
  `source: "agent"`, gates the write on `observed_at` freshness.
- `apply_debug_attention` (`http/session_handlers.rs:661`) — debug injection,
  `source: "debug"`, additionally manages `usage_limit_reset_hint` and derives
  `attention` from `req.attention` directly instead of through
  `canonical_attention`.
- `mark_committed_input_working` (`runtime/terminal_runtime.rs:401`) — the
  sidecar's own observation that committed input implies `working`, which
  must additionally bump `input_generation`/`accepted_input_at` atomically
  with the status write (see issue #193, which tracks a related staleness
  race this ADR does not fix but must not reopen).
- The `peon_loop` idle-timer sweep (`runtime/peon_runtime.rs:253`) — the
  sidecar's own observation that a session has gone silent, which currently
  never sets `metadata_confidence` on either store.

This pattern of drift is visible in the commit history: `#199` touched 7
files in one PR to close a staleness race across these stores, `#202` needed
4 follow-up commits inside one PR to get an input-attention gate right. ADR
0023 established the canonical attention vocabulary (`working`, `idle`,
`needs_you`, `blocked`, `failed`, `capped`, valid only while `lifecycle ==
alive`); this ADR is about *where the code lives that applies that
vocabulary*, not the vocabulary itself.

A related but distinct bug — two functions in `terminal_runtime.rs`
(`collect_input_line` and `frame_completes_a_real_line`) disagreeing on
whether bracketed-paste content contains a submitted line — is tracked
separately as issue #207 and is out of scope here.

## Decision

- One module, `runtime/observed_status.rs`, owns every write to
  `observed_status`/`attention` across both stores. No other module writes
  these fields directly to `handle.info` or `SessionMetadata` going forward.
- The module splits into a pure decision function (plain structs in, plain
  structs out — no locks, no I/O, directly unit-testable) and a thin apply
  layer that performs the actual `workspace` → `sessions` locking (the
  documented lock order) and the writes.
- Two named entry points on the apply layer, matching two distinct signal
  shapes rather than one generic parameterized function:
  - `apply_attention_signal` — external status reports (`report_attention`,
    `apply_debug_attention`). Always requires a workspace, matching both
    current callers, which already reject with `409` without one. Self-locks.
  - `apply_process_transition` — the sidecar's own observations (committed
    input, idle timeout). Persistence is an internal
    `Option<&WorkspaceState>` branch, matching `mark_committed_input_working`'s
    existing detached-runtime support. Self-locks — **except**
    `mark_committed_input_working` calls the pure decision function directly
    instead of the self-locking shell, since it must keep both locks held to
    atomically bump `input_generation`/`accepted_input_at`/
    `min_peon_output_revision` alongside the status write. This is the one
    caller allowed to bypass the shell, specifically to avoid reopening the
    race issue #193 tracks.
  - Callers that don't need to hold the lock for anything else
    (`report_attention`, `apply_debug_attention`, the idle-timer sweep) use
    the self-locking entry points and never acquire `sessions`/`workspace`
    directly for this purpose again.
- Known drift is fixed, not preserved: the idle-timer sweep starts setting
  `metadata_confidence` on both stores, and `apply_debug_attention`'s
  hand-derivation of `attention` is replaced by `canonical_attention` (which
  produces the same values as the current passthrough for every input, so
  this is a no-op for existing behavior — see verification in `metadata.rs`'s
  `canonical_attention`).

## Consequences

- Adding a fifth call site (a new attention source, a new self-observed
  transition) means calling one of two existing entry points, not
  re-deriving the field-sync logic again.
- The pure decision function is testable with plain structs — no `Mutex`,
  `MetadataStore`, async runtime, or `tokio::spawn`/`sleep` required. Field-
  sync and concurrency-race tests currently living on `report_attention`/
  `apply_debug_attention` (`http/session_handlers.rs:2267`, `:2483`, `:2563`)
  move down to test `apply_attention_signal` directly; the HTTP handlers keep
  only their own validation/status-code tests plus one wiring smoke test
  each.
- `mark_committed_input_working` remains the one place allowed to hold both
  locks across a combined status-and-generation transition; this is a
  documented exception, not a precedent for future callers to bypass the
  shell casually.
- Does not change the canonical attention vocabulary (ADR 0023) or PTY-
  lifetime ownership (ADR 0022); purely relocates who writes already-agreed
  fields.
- Does not fix issue #193's staleness race; the lock-ownership rule here is
  chosen specifically so it doesn't make that race worse.
