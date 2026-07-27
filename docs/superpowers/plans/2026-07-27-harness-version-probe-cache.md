# Harness Version Probe Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cache harness `--version` probe output so repeated integration polling stays fast without weakening the existing TOCTOU revalidation.

**Architecture:** Add an AppState-owned in-memory cache that stores `probe_tool_version` output, not the final compatibility verdict. `resolve_tool_gate` still resolves the executable path first, then consults the cache, then parses the cached text against the current `min_version`, and `run_integration_action` still revalidates the captured workspace and harness definition after the await. Workspace switches and successful harness edits/deletes bump a shared generation counter so stale cache entries stop matching immediately.

**Tech Stack:** Rust 2021, tokio, std::sync::Mutex, std::sync::atomic, axum, serde, tempfile, existing crate test helpers.

**Global Constraints**

- `main is the trunk, not the workspace. Use branches and PRs for code; keep main fast for low-risk writing.`
- `Everything else requires a branch + PR, including any change to apps/desktop/src/, apps/desktop/electron/, apps/desktop/tests/, or crates/orkworksd/, regardless of commit-type prefix.`
- `PRs that touch code under apps/desktop/ or crates/orkworksd/ must have a /code-review run before merge.`
- `Use pnpm for all Node.js package management. Do not use npm or yarn for project package management tasks.`

---

### Task 1: Add the cache module and AppState plumbing

**Files:**
- Create: `crates/orkworksd/src/harness/probe_cache.rs`
- Modify: `crates/orkworksd/src/harness.rs`
- Modify: `crates/orkworksd/src/main.rs`

**Interfaces:**
- Produces: `harness::probe_cache::VersionProbeCache`, `VersionProbeCache::new()`, `VersionProbeCache::bump_generation()`, `VersionProbeCache::probe_or_get`
- Produces: `AppState::bump_harness_probe_generation()`
- Consumes: `HarnessStore`, `WorkspaceState`, `test_support::test_app_state_with_workspace`, `test_support::swap_workspace`

- [ ] **Step 1: Write the failing cache unit tests**

Create `crates/orkworksd/src/harness/probe_cache.rs` with the cache types and tests first. The tests should cover:

```rust
#[tokio::test]
async fn reuses_a_positive_probe_until_ttl_expires() {
    let cache = VersionProbeCache::new();
    let key = VersionProbeCacheKey {
        harness_id: "copilot".into(),
        launch_command: "copilot".into(),
        executable: PathBuf::from("/tmp/copilot"),
    };
    let calls = Arc::new(AtomicUsize::new(0));
    let now = Instant::now();

    let first = cache
        .probe_or_get(
            key.clone(),
            now,
            Duration::from_secs(30),
            Duration::from_secs(5),
            {
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Some("copilot-cli 1.2.3".into())
                }
            },
        )
        .await;
    let second = cache
        .probe_or_get(
            key,
            now + Duration::from_secs(1),
            Duration::from_secs(30),
            Duration::from_secs(5),
            {
                let calls = calls.clone();
                move || async move {
                    calls.fetch_add(1, Ordering::SeqCst);
                    Some("copilot-cli 1.2.3".into())
                }
            },
        )
        .await;

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(first.as_deref(), Some("copilot-cli 1.2.3"));
    assert_eq!(second.as_deref(), Some("copilot-cli 1.2.3"));
}
```

Add a second test that proves a `None` result is cached for the shorter negative TTL, and a third test that proves `bump_generation()` invalidates an entry even before TTL expiry.

- [ ] **Step 2: Run the new tests and confirm they fail to compile**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml probe_cache -- --nocapture
```

Expected: compile failure because the module, cache type, and `AppState` field do not exist yet.

- [ ] **Step 3: Implement the cache and AppState field**

Add the module registration in `crates/orkworksd/src/harness.rs`:

```rust
pub(crate) mod definition;
pub(crate) mod detect;
pub(crate) mod integration;
pub(crate) mod integrations;
pub(crate) mod probe_cache;
pub(crate) mod registry;
pub(crate) mod store;
```

Implement `crates/orkworksd/src/harness/probe_cache.rs` with:

```rust
pub(crate) struct VersionProbeCache {
    generation: AtomicU64,
    entries: Mutex<HashMap<VersionProbeCacheKey, VersionProbeCacheEntry>>,
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub(crate) struct VersionProbeCacheKey {
    harness_id: String,
    launch_command: String,
    executable: PathBuf,
}

#[derive(Clone)]
struct VersionProbeCacheEntry {
    generation: u64,
    expires_at: Instant,
    version_output: Option<String>,
}
```

Give it:

```rust
impl VersionProbeCache {
    pub(crate) fn new() -> Self;
    pub(crate) fn bump_generation(&self);
    pub(crate) async fn probe_or_get<F, Fut>(
        &self,
        key: VersionProbeCacheKey,
        now: Instant,
        positive_ttl: Duration,
        negative_ttl: Duration,
        probe: F,
    ) -> Option<String>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Option<String>>;
}
```

`probe_or_get` should:

1. Read the current generation.
2. Return a cached entry only if its generation matches and it has not expired.
3. Otherwise await the probe, then insert the result only if the generation did not change while the probe was running.
4. Cache both `Some(version_text)` and `None`, using the positive or negative TTL respectively.
5. Prune expired or superseded entries and cap the map at 64 entries.

Add `integration_probe_cache: harness::probe_cache::VersionProbeCache` to `AppState`, initialize it with `VersionProbeCache::new()` in `main()`, and add:

```rust
impl AppState {
    fn bump_harness_probe_generation(&self) {
        self.integration_probe_cache.bump_generation();
    }
}
```

Update `test_app_state_with_workspace()` and `swap_workspace()` in `crates/orkworksd/src/main.rs` to construct and preserve the new cache field.

- [ ] **Step 4: Verify the cache unit tests pass**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml probe_cache -- --nocapture
```

Expected: the cache tests pass.

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/harness.rs crates/orkworksd/src/harness/probe_cache.rs crates/orkworksd/src/main.rs
git commit -m "feat: add harness version probe cache state"
```

### Task 2: Thread the cache into version detection and invalidation

**Files:**
- Modify: `crates/orkworksd/src/harness/detect.rs`
- Modify: `crates/orkworksd/src/http/integration_handlers.rs`
- Modify: `crates/orkworksd/src/http/harness_handlers.rs`
- Modify: `crates/orkworksd/src/http/session_handlers.rs`
- Modify: `crates/orkworksd/src/main.rs`

**Interfaces:**
- Consumes: `harness::probe_cache::VersionProbeCache`, `VersionProbeCacheKey`
- Produces: `resolve_tool_gate(cache: &VersionProbeCache, harness_id: &str, command: &str, min_version: Option<&VersionRequirement>)`

- [ ] **Step 1: Write the failing integration test**

Add a status-poll regression in `crates/orkworksd/src/http/integration_handlers.rs` that uses a fake executable which increments a counter file every time it is spawned:

```rust
#[tokio::test]
async fn repeated_status_polls_reuse_one_version_probe_within_ttl() {
    use crate::test_support::{make_test_executable, FakeHome, FakePath};
    use std::fs;

    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());

    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".to_string(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();

    let fake_bin_dir = tempfile::tempdir().unwrap();
    let counter = fake_bin_dir.path().join("probe-count.txt");
    let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
    let bin = fake_bin_dir.path().join(bin_name);
    fs::write(
        &bin,
        format!(
            "#!/bin/sh\nprintf '1.2.3\\n'\nprintf 'probe\\n' >> '{}'\n",
            counter.display()
        ),
    )
    .unwrap();
    make_test_executable(&bin);
    let _fake_path = FakePath::prepend(fake_bin_dir.path());

    let first = get_integration_status(State(state.clone()), AxumPath("copilot".into()))
        .await
        .into_response();
    let second = get_integration_status(State(state), AxumPath("copilot".into()))
        .await
        .into_response();

    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(fs::read_to_string(counter).unwrap().lines().count(), 1);
}
```

Run it now; it should fail because `resolve_tool_gate` still probes every request.

- [ ] **Step 2: Rewire `resolve_tool_gate` to use the cache**

Change `crates/orkworksd/src/harness/detect.rs` to import the new cache module and make `resolve_tool_gate` take the cache handle plus harness id:

```rust
pub(crate) async fn resolve_tool_gate(
    cache: &crate::harness::probe_cache::VersionProbeCache,
    harness_id: &str,
    command: &str,
    min_version: Option<&VersionRequirement>,
) -> Option<DetectedTool> {
    let mut tool = probe_installed_tool(command)?;
    let Some(requirement) = min_version else {
        return Some(tool);
    };
    let version_output = cache
        .probe_or_get(
            crate::harness::probe_cache::VersionProbeCacheKey {
                harness_id: harness_id.to_owned(),
                launch_command: command.to_owned(),
                executable: tool.executable.clone(),
            },
            std::time::Instant::now(),
            std::time::Duration::from_secs(30),
            std::time::Duration::from_secs(5),
            || probe_tool_version(&tool.executable),
        )
        .await;
    match version_output.as_deref().and_then(parse_version_token) {
        Some(parsed) => {
            tool.compatible = parsed >= requirement.min;
            tool.version = version_output;
        }
        None => {
            tool.compatible = false;
            tool.version = None;
        }
    }
    Some(tool)
}
```

Update the `integration_handlers.rs` call site to pass `&state.integration_probe_cache` and the resolved harness id, then keep the existing post-probe definition and workspace revalidation exactly as-is.

Add `state.bump_harness_probe_generation();` after successful workspace switches in `set_workspace` and after successful `mutate()` calls in `create_harness`, `update_harness`, and `delete_harness`.

Also call the same helper from `swap_workspace()` in `crates/orkworksd/src/main.rs` so tests that simulate a workspace change invalidate the cache the same way production does.

- [ ] **Step 3: Run the targeted regression and fix whatever it exposes**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml repeated_status_polls_reuse_one_version_probe_within_ttl -- --nocapture
```

Expected: PASS once the cache is wired in and the invalidation hooks are in place.

- [ ] **Step 4: Commit**

```bash
git add crates/orkworksd/src/harness/detect.rs crates/orkworksd/src/http/integration_handlers.rs crates/orkworksd/src/http/harness_handlers.rs crates/orkworksd/src/http/session_handlers.rs crates/orkworksd/src/main.rs
git commit -m "feat: wire harness version probe cache"
```

### Task 3: Add invalidation coverage and run the full verification pass

**Files:**
- Modify: `crates/orkworksd/src/http/integration_handlers.rs`
- Modify: `crates/orkworksd/src/harness/probe_cache.rs`

**Interfaces:**
- Consumes: `VersionProbeCache::bump_generation()`, `swap_workspace()`, `state.bump_harness_probe_generation()`
- Produces: a regression test proving workspace switches and harness edits invalidate cached probe output

- [ ] **Step 1: Write the invalidation regression test**

Add a second integration test that proves the cache is invalidated by a workspace switch:

```rust
#[tokio::test]
async fn workspace_switch_forces_a_fresh_version_probe() {
    use crate::test_support::{make_test_executable, swap_workspace, FakeHome, FakePath};
    use std::fs;

    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());

    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".to_string(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();

    let fake_bin_dir = tempfile::tempdir().unwrap();
    let counter = fake_bin_dir.path().join("probe-count.txt");
    let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
    let bin = fake_bin_dir.path().join(bin_name);
    fs::write(
        &bin,
        format!(
            "#!/bin/sh\nprintf '1.2.3\\n'\nprintf 'probe\\n' >> '{}'\n",
            counter.display()
        ),
    )
    .unwrap();
    make_test_executable(&bin);
    let _fake_path = FakePath::prepend(fake_bin_dir.path());

    let _ = get_integration_status(State(state.clone()), AxumPath("copilot".into()))
        .await
        .into_response();

    let other_workspace = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(other_workspace.path());
    swap_workspace(&state, other_workspace.path());

    let _ = get_integration_status(State(state), AxumPath("copilot".into()))
        .await
        .into_response();

    assert_eq!(fs::read_to_string(counter).unwrap().lines().count(), 2);
}
```

Add a matching harness-edit regression as well:

```rust
#[tokio::test]
async fn harness_edit_forces_a_fresh_version_probe() {
    use crate::harness::definition::{HarnessPatch, VersionRequirement};
    use crate::test_support::{make_test_executable, FakeHome, FakePath};
    use std::fs;

    let dir = tempfile::tempdir().unwrap();
    init_git_workspace_with_copilot_settings_ignored(dir.path());
    let home = tempfile::tempdir().unwrap();
    let _fake_home = FakeHome::set(home.path());
    let state = test_app_state_with_workspace(dir.path());

    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".to_string(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (0, 0, 1) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();

    let fake_bin_dir = tempfile::tempdir().unwrap();
    let counter = fake_bin_dir.path().join("probe-count.txt");
    let bin_name = if cfg!(windows) { "copilot.exe" } else { "copilot" };
    let bin = fake_bin_dir.path().join(bin_name);
    fs::write(
        &bin,
        format!(
            "#!/bin/sh\nprintf '1.2.3\\n'\nprintf 'probe\\n' >> '{}'\n",
            counter.display()
        ),
    )
    .unwrap();
    make_test_executable(&bin);
    let _fake_path = FakePath::prepend(fake_bin_dir.path());

    let _ = get_integration_status(State(state.clone()), AxumPath("copilot".into()))
        .await
        .into_response();

    state
        .harness_store
        .mutate(&state.harness_catalog, |document| {
            document.overrides.insert(
                "copilot".to_string(),
                HarnessPatch {
                    min_version: Some(Some(VersionRequirement { min: (0, 0, 2) })),
                    ..Default::default()
                },
            );
            Ok(())
        })
        .unwrap();

    let _ = get_integration_status(State(state), AxumPath("copilot".into()))
        .await
        .into_response();

    assert_eq!(fs::read_to_string(counter).unwrap().lines().count(), 2);
}
```

- [ ] **Step 2: Run the full harness and crate verification**

Run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml repeated_status_polls_reuse_one_version_probe_within_ttl -- --nocapture
cargo test --manifest-path crates/orkworksd/Cargo.toml workspace_switch_forces_a_fresh_version_probe -- --nocapture
cargo test --manifest-path crates/orkworksd/Cargo.toml
cargo fmt --manifest-path crates/orkworksd/Cargo.toml --check
```

Expected: both regressions pass, the full crate suite passes, and rustfmt is clean.

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/src/http/integration_handlers.rs crates/orkworksd/src/harness/probe_cache.rs
git commit -m "test: pin harness version probe cache invalidation"
```

---
