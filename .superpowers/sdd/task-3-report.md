# Task 3 — Revision-aware harness CRUD and deletion semantics

## Scope delivered

- `crates/orkworksd/src/harness/store.rs`
  - Added opaque SHA-256 `HarnessDocumentRevision` values, snapshots, and
    revision-checked mutations under the existing write lock.
  - Revision conflicts return the current revision without changing disk or
    the live catalog; successful mutations return the new revision and stored
    patch metadata.
  - v2 documents are promoted to v3 on the next save.
- `crates/orkworksd/src/http/harness_handlers.rs`
  - Added revision-bearing list/mutation responses and strict create/update/
    delete request parsing.
  - Added duplicate preview and profile-removal routes.
  - Duplicate final saves derive the profile from the source ID; profile data
    is never accepted from the request body.
  - Custom deletion is blocked when active in the current workspace, and
    successful harness mutations return the new document revision and
    reconcile projected provider settings.
  - Duplicate previews return an editable definition without compiled binding
    fields, so the preview can be submitted unchanged.
- `crates/orkworksd/src/session_application.rs`, `metadata.rs`,
  `http/session_handlers.rs`
  - Added active-harness revisions to workspace memory and responses.
  - Active-harness writes require the expected revision, reject missing or
    retired IDs, normalize stale IDs on workspace load, and increment the
    revision on successful selection or pruning.
- `crates/orkworksd/src/main.rs` and compatibility tests
  - Registered the new routes and updated existing direct-handler tests to use
    the explicit revision contract.
- `crates/orkworksd/src/harness/definition.rs`
  - Persisted custom definitions tolerate the generated null representation of
    unassigned compiled fields while still rejecting non-null authority fields.

## TDD evidence

RED: the initial focused handler run failed 3 new tests because persisted
custom definitions serialized null `integration`/`sessionSignals` fields and
the strict reload rejected them as authority bindings. After that root cause
was fixed, one test exposed an order assumption and one attempted to parse a
204 response body; both tests were corrected to assert the actual contracts.

GREEN:

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::store -- --nocapture
```

Result: **PASS** — 24 tests.

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::harness_handlers -- --nocapture
```

Result: **PASS** — 8 tests.

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml session_application::tests -- --nocapture
```

Result: **PASS** — 98 tests.

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::session_handlers
```

Result: **PASS** — 99 tests.

```bash
rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Result before the review-fix round: **PASS** — 931 tests. Two reruns after
the review fixes stalled without producing test output and were interrupted;
the focused suites above passed after the fixes.

```bash
rtk git diff --check
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
```

Result: **PASS**.

## Concerns

- Desktop consumers still expect the pre-CRUD harness response shape. The
  planned Task 5 contract update must consume the new metadata envelope before
  this branch is usable end to end.
- Provider reconciliation is called from harness HTTP mutations; direct test
  or internal store mutations remain lower-level operations and do not own
  application-level provider refresh.
- Shared hook cleanup after deleting a profile or pruning stale active IDs is
  intentionally deferred to Task 4's adapter/target reconciliation slice;
  Task 4 must cover those paths before integration lifecycle work is complete.
