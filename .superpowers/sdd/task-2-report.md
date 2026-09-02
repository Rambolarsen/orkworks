# Task 2 — Runtime harness profiles and provider reconciliation

## Scope delivered

- `crates/orkworksd/src/harness/registry.rs`
  - Added read-only `CompatibilityMetadata` to `ResolvedHarness`.
  - Built-in bindings remain code-owned and intact.
  - Custom compatibility profiles derive bindings only while building the
    immutable runtime definition; the stored user document remains declarative.
  - Custom Peon providers retain the custom harness ID and inherit the custom
    launch/model configuration.
- `crates/orkworksd/src/providers.rs`
  - Added explicit `reconcile_harness_provider_settings()`.
  - Reconciliation preserves settings for unchanged IDs, removes deleted or
    non-Peon IDs, appends new projected providers with standard defaults, and
    clears a Peon selection whose provider disappeared.
  - Reconciliation is deliberately not performed during provider GETs, so an
    existing `apply_settings` payload is not silently expanded on read. The
    catalog-refresh/CRUD wiring will invoke this operation in the next task.

## Tests

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::registry
```

Result: **PASS** — 22 tests.

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests
```

Result: **PASS** — 69 tests.

```bash
rtk git diff --check
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
```

Result: **PASS**.

## Notes

- `HarnessStore` needs no standalone change for this slice. Its catalog
  mutation boundary remains the source of the refresh event; Task 3 will wire
  provider reconciliation to the revision-aware CRUD handlers.
- A prior implementation reconciled from `get_providers_response()`, which
  broke the existing provider-settings contract by re-adding built-ins after
  `apply_settings()`. That call was removed and covered by the existing
  regression suite.
