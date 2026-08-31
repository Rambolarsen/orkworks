# Rust Sidecar Instructions

Read the root [`AGENTS.md`](../../AGENTS.md) first. These instructions apply before changing anything under `crates/orkworksd/`.

## Validation

Run these commands from the repository root:

```bash
cargo build --manifest-path crates/orkworksd/Cargo.toml
cargo test --manifest-path crates/orkworksd/Cargo.toml
cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
```

The desktop package can also build the sidecar with `cd apps/desktop && pnpm build:rust`.

### Formatting footgun

`cargo fmt -- <files>` run from the repository root forwards the arguments differently than expected and formats the **entire crate**, silently reformatting untouched files. To format specific files, either run from inside `crates/orkworksd/` or invoke rustfmt directly:

```bash
cd crates/orkworksd && cargo fmt -- src/foo.rs
rustfmt --edition 2021 crates/orkworksd/src/foo.rs
```

PR CI runs `cargo fmt --check` as a blocking gate, so format before committing.

## Module layout

- `metadata.rs` — `SessionMetadata` and the on-disk metadata store, the source of truth for session state.
- `session_types.rs`, `session_view.rs` — session-facing types and pure view helpers; `session_projection.rs` owns the stateful session-listing projection and its write-back policy.
- `harness.rs` and its `definition`, `registry`, and `store` submodules — versioned harness definitions, sparse overrides, resolved immutable capability snapshots, and persistence.
- `providers.rs` — model provider registry, fallback, and capacity state.
- `peon.rs` — terminal-output observation and label/status inference.
- `git.rs`, `watcher.rs`, `migration.rs`, `workspace_runtime.rs` — Git context detection, metadata file watching, on-disk migrations, and workspace bootstrap.
- `http/` — thin HTTP handler submodules (session, harness, provider, retention, and attention hook) delegating to `AppState`.
- `runtime/` — background tasks: terminal/PTY runtime (`SessionRuntime`, PTY lifecycle), Peon observation loop, and retention cleanup.
- `main.rs` — Axum router and the `AppState`/`SessionHandle` definitions and startup.

## Protocol and architecture references

The root [metadata-protocol constraints](../../AGENTS.md#metadata-protocol) remain authoritative, including provenance, retention, detached runtime, and explicit-approval rules. Read [`docs/agents/architecture.md`](../../docs/agents/architecture.md) for inter-component flow and [`docs/agents/domain-entities.md`](../../docs/agents/domain-entities.md) before changing `SessionMetadata`, session status/lifecycle vocabulary, or related session/API mappings.

For cross-component work, also read [`apps/desktop/AGENTS.md`](../../apps/desktop/AGENTS.md).
