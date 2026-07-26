# Harness tool version detection (per-harness minimum-version gating)

## Context

Issue #227, split out of #103 (Codex deterministic attention adapter). Codex
CLI shipped a project-level hooks framework (`<repo>/.codex/hooks.json` or an
inline `[hooks]` table in `<repo>/.codex/config.toml`) starting ~v0.114
(March 2026). An OrkWorks Codex integration adapter needs to distinguish a
Codex install new enough to have that mechanism from an older one that
doesn't — but nothing in the codebase can answer that question today.

`DetectedTool { executable, version, compatible }`
(`crates/orkworksd/src/harness/integration.rs:500-504`) and the diagnostic
logic that consumes it already exist:
`JsonHookHandler::status_from_document`
(`crates/orkworksd/src/harness/integrations/mod.rs:225-231`) already branches
on `detected_tool.compatible` to produce `IntegrationActivation::NeedsTrust`
plus an `unsupported_tool_version` diagnostic when a tool is detected but
incompatible. That branch is currently dead code: every production call site
sets `compatible: true` unconditionally.

The predecessor design
(`docs/superpowers/specs/2026-07-25-harness-tool-detection-design.md`, which
shipped `crates/orkworksd/src/harness/detect.rs::probe_installed_tool`)
explicitly deferred this as a non-goal:

> Version parsing or minimum-version compatibility gating. `compatible` is
> always `true` once a binary is found; nothing downstream currently
> consumes a real version number or gates on one.

This design picks up that deferred item. `probe_installed_tool` itself is a
pure filesystem/PATH existence check — it never spawns the tool. Real version
detection means introducing subprocess execution where none currently
exists, scoped narrowly to the harnesses that actually need it.

## Goals

1. Let a harness definition declare a minimum version it requires for some
   capability (starting with Codex's hooks framework, v0.114).
2. When such a harness is detected on `PATH`/at an override path, probe its
   real version and compare against the declared minimum, setting
   `DetectedTool.compatible` accordingly instead of the current hardcoded
   `true`.
3. Fail safe: any probe failure (spawn error, timeout, unparseable output)
   for a harness that *does* declare a minimum version results in
   `compatible = false` — never a silent `true`. See Non-goals for the
   "no minimum declared" case, which is unaffected.

## Non-goals

- The Codex hooks adapter itself (#103's remaining scope once this and #228
  land).
- Any behavior change for harnesses that don't declare a minimum version.
  Claude/Gemini/Copilot/Aider/OpenCode/generic-shell keep today's
  `version: None, compatible: true` exactly as-is, and — because the version
  probe only runs when `min_version` is set — never pay the cost of a
  subprocess spawn either.
- A per-harness configurable version-check command/flag. Every probe always
  runs `<command> --version`; if a future harness needs something else, that
  is a follow-on design, not speculative work here.
- A configurable timeout. Fixed at 3 seconds in code, not exposed as a
  harness-definition field — `--version` should return near-instantly, and a
  per-harness knob for a fixed safety bound is unneeded surface.
- A new dependency for parsing. No `semver`/`regex` crate exists in
  `crates/orkworksd/Cargo.toml` today; parsing stays a small hand-rolled
  scanner for the first `\d+\.\d+\.\d+`-shaped token.
- Caching probe results. Same reasoning as the predecessor design: this
  fires on-demand from a Settings status/install/uninstall request, not in a
  polling loop.

## Design

### Data model

New optional capability field on `HarnessDefinition`
(`crates/orkworksd/src/harness/definition.rs`), following the existing
`peon`/`capacity`/`resume` optional-capability pattern:

```rust
pub struct HarnessDefinition {
    ...
    pub min_version: Option<VersionRequirement>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VersionRequirement {
    pub min: (u64, u64, u64), // (major, minor, patch)
}
```

`resources/harnesses-v2.json`'s `codex` entry gets:

```json
"minVersion": { "min": [0, 114, 0] }
```

Every other builtin entry omits the field (`null`). `HarnessPatch` gets the
matching override field, same shape as every other capability:

```rust
pub min_version: Option<Option<VersionRequirement>>,
```

### Version probe

New function in `crates/orkworksd/src/harness/detect.rs`, alongside (not
replacing) `probe_installed_tool`:

```rust
pub(crate) async fn probe_tool_version(executable: &Path) -> Option<String> {
    let result = tokio::time::timeout(
        Duration::from_secs(3),
        tokio::process::Command::new(executable).arg("--version").output(),
    ).await;
    let output = result.ok()?.ok()?;
    let text = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    Some(text)
}
```

Stdout and stderr are combined because some CLIs print version information to
stderr. A separate pure function parses the first `major.minor.patch`-shaped
token out of that text into `(u64, u64, u64)`; comparison against
`VersionRequirement.min` is a plain tuple `>=`.

`probe_installed_tool` itself is unchanged — still the fast, pure PATH check,
still used as-is at every existing call site.

### Wiring and failure handling

`crates/orkworksd/src/http/integration_handlers.rs::run_integration_action`
becomes `async fn` (it is already called from `async` axum handlers that
currently call it synchronously — this is a mechanical change, not a new
architectural layer). New flow, after the existing `probe_installed_tool`
call:

- Tool not found → unchanged (`detected_tool: None`), independent of version
  logic.
- Tool found, harness has no `min_version` → unchanged
  (`version: None, compatible: true`), no subprocess spawned.
- Tool found, harness has `min_version` → `.await probe_tool_version`, parse,
  compare:
  - Parsed version `>=` minimum → `compatible: true`,
    `version: Some(parsed_string)`.
  - Parsed version `<` minimum → `compatible: false`,
    `version: Some(parsed_string)`.
  - Any failure (spawn error, timeout, no numeric token found) →
    `compatible: false`, `version: None`. We never fabricate a version string
    we couldn't actually parse, and we never default to `true` on failure —
    per the Goals section, "can't confirm" is treated the same as "below
    threshold."

Nothing downstream of `DetectedTool` changes: `JsonHookHandler` already
consumes `compatible` correctly (`unsupported_tool_version` diagnostic +
`IntegrationActivation::NeedsTrust`); this design only makes that existing
consumer receive real data for the first time.

## Testing

- `detect.rs`: unit tests for `probe_tool_version` using a fake test
  executable (mirroring the existing `FakePath`/`make_test_executable`
  helpers) covering: parseable version above threshold, below threshold,
  unparseable/no-numeric-token output, nonzero exit with no meaningful
  output, and a deliberately slow/hanging fake binary to exercise the
  3-second timeout path.
- `definition.rs`: round-trip serde test for `VersionRequirement` in both
  `HarnessDefinition` and `HarnessPatch`, mirroring the existing
  `codex()`/`apply_patch` capability round-trip tests already in that file.
- `integration_handlers.rs`: an integration-style test asserting a harness
  with `min_version` set and a fake below-threshold binary on `PATH`
  produces `IntegrationActivation::NeedsTrust` + `unsupported_tool_version`,
  and one with an above-threshold fake binary produces normal activation —
  reusing the existing `FakeHome`/`test_app_state_with_workspace` scaffolding
  already present in that file.
