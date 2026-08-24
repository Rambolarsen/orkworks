# Session Context Follow-up Plan

## Audit result

The application-seam work did not leave one obvious `AppState` abstraction to
extract. Remaining production callers use different parts of the state with
different ownership and lifecycle rules:

- `http/session_handlers.rs` coordinates HTTP authorization and compatibility
  mapping, plus a small amount of workspace/session projection that is not yet
  part of `SessionApplication`.
- `runtime/session_runtime.rs` owns PTY startup, output persistence, terminal
  attachment, runtime generations, and workspace reconciliation. It needs both
  the live session map and workspace metadata.
- `runtime/terminal_runtime.rs` owns input, status transitions, label resets,
  capacity state, and terminal transport. It crosses live handles, workspace
  metadata, harness definitions, and provider state.
- `runtime/peon_runtime.rs` owns inference and observation epochs. It crosses
  live handles, workspace observations/recommendations, Peon state, and
  provider state.

## Decision

Do not introduce a generic `SessionContext` wrapper around `Arc<AppState>` yet.
That would preserve the same broad dependency surface while adding another
name and ownership layer. The existing `SessionApplication` is a meaningful
seam because it owns user-facing workflows; the runtime modules are cohesive
around lifecycle responsibilities and should not be split by field access
alone.

## Next bounded slice

If this backlog is resumed, start with a characterization-only slice around
one invariant that crosses a clear boundary:

1. Choose one production invariant, preferably terminal finalization or
   workspace-scoped session lookup, not the entire `AppState`.
2. Add tests at the existing runtime/application boundary that pin its lock
   ordering, generation behavior, and metadata/live-handle consistency.
3. Extract only the named operation and its narrow inputs/outputs.
4. Re-run the full Rust suite and inspect lock ordering before considering a
   second extraction.

The legacy `*_legacy` handlers and remaining direct handler state access should
be treated as a separate HTTP-thinning task; combining that with runtime state
narrowing would make failures difficult to localize.

## Current evidence

- Cross-seam Rust/renderer session and workspace JSON contracts align.
- Full Rust suite passes 743/743 after the application seam.
- No second session owner was introduced; all runtime code still uses the
  existing `AppState.sessions` map.
- The next implementation should be driven by a specific duplicated invariant,
  not by the number of `AppState` references.
