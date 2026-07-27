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
  scanner (see Version probe below for the exact rule and its known
  residual limitations).
- Caching probe results. Same reasoning as the predecessor design: this
  fires on-demand from a Settings status/install/uninstall request, not in a
  polling loop.
- Reconciling `min_version` against a user-overridden `launch.command`. The
  probe always runs against whatever binary `launch.command` currently
  resolves to, built-in or overridden — the override contract already
  requires that path to point at a real instance of that harness's CLI (see
  the predecessor detection design). If a user points it at an unrelated
  binary instead, the most likely outcomes are an unparseable `--version`
  (→ conservative `compatible: false`, correct) or, in a rare coincidence, an
  unrelated binary whose `--version` output happens to parse as a
  sufficiently high version number (→ an incorrect `compatible: true`). This
  is a known, accepted limitation of reusing `launch.command` for both
  launch and version probing, not something this design adds new handling
  for.

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
replacing) `probe_installed_tool`. Reads stdout/stderr through an explicit
`AsyncReadExt::take(MAX_PROBE_OUTPUT_BYTES)` cap (64 KiB per stream) rather
than `Command::output()`, which buffers both streams with no size limit —
added after the initial implementation, per code review, to close a
resource-exhaustion gap: a broken or hostile tool writing continuously for
the full timeout could otherwise let a single Settings status request buffer
unbounded memory. Once a stream's cap is hit, the read stops there (not an
error); the `Child` handle is dropped without an explicit `.wait()`, since a
genuinely gluttonous writer may never exit once its own write blocks on the
now-undrained pipe — waiting for its exit would just re-block on the outer
timeout for no benefit, and `kill_on_drop(true)` still cleans it up on drop
exactly as it does on the timeout-cancellation path. The accepted worst case
for a truly adversarial writer is bounded *memory* and bounded *latency by
the existing 3s timeout*, not a fast return — a well-behaved tool's stdout
and stderr both close together when it exits normally, long before either
the cap or the timeout matters.

`kill_on_drop(true)` matters independently of the above for the ordinary
timeout path: when `tokio::time::timeout` fires, the read future is dropped
without ever completing, and without this flag the spawned child is orphaned
rather than killed — a hanging binary would keep running in the background
after every timed-out probe, one per Settings poll.

Stdout and stderr are combined because some CLIs print version information to
stderr; the two capped reads run concurrently via `tokio::join!`, so this
doesn't risk the classic pipe-buffer deadlock a naive `stdout`-then-`stderr`
read would.

A separate pure function extracts a version token from that text, scanning
by byte position (not a naive split) so it can inspect the character
immediately following each candidate token: a run of ASCII digits/`.`
matching `\d+\.\d+(\.\d+)?` (two or three numeric components — CLIs that
report `MAJOR.MINOR` without a patch number are common and shouldn't
hard-fail the probe) is accepted only if it is *not* immediately followed by
`-` or `+` (semver's prerelease/build-metadata separators). A token like
`0.114.0-alpha.1` is therefore treated as unparseable rather than silently
accepted as if it were the stable `0.114.0` — added per code review, since a
prerelease must not compare equal to its corresponding stable release under
a `>=` gate. A missing third component defaults to `0`. Comparison against
`VersionRequirement.min` is a plain tuple `>=`.

Known residual limitation, accepted rather than solved here: this is still a
generic heuristic, not a real version-output parser, for anything other than
the prerelease/build-metadata case above. A `--version` banner that prints
an unrelated numeric-looking token before the actual version (e.g. a bundled
dependency or schema version) could be picked up instead. Solving that
properly would mean either a per-harness version-extraction pattern
(rejected above as unneeded surface) or a real parsing dependency (rejected
above per the no-new-dependency non-goal). If this bites in practice for a
specific harness, that's grounds for a follow-up, not a reason to add
speculative robustness now.

`probe_installed_tool` itself is unchanged — still the fast, pure PATH check,
still used as-is at every existing call site.

### Wiring and failure handling

`crates/orkworksd/src/http/integration_handlers.rs::run_integration_action`
becomes `async fn`. This is **not** the purely mechanical change the first
draft of this design claimed. Today the function holds a
`std::sync::MutexGuard` (`state.workspace.lock()`) and a
`std::sync::RwLockReadGuard` (`state.harness_catalog.read()`) across its
entire body, including the final `action(harness, &ctx)` call — both guard
types are `!Send`, so an `.await` anywhere while either is still in scope
would make the handler future non-`Send`, which axum requires. Even setting
the compile error aside, holding the workspace mutex for up to 3 seconds
(the version-probe timeout) would block every other endpoint that touches
workspace state for the duration of a slow or hanging probe.

The fix is to reorder the function so no lock guard is alive across the
`.await` point, not to sprinkle `.await` into the existing shape:

1. Acquire `state.harness_catalog.read()`, look up the harness, and `.clone()`
   the resolved `ResolvedHarness`/`HarnessDefinition` (both already
   `#[derive(Clone)]`) into an owned local. Drop the read guard (end of its
   block, or an explicit `drop(registry)`).
2. With no locks held: run `probe_installed_tool` (sync, unchanged) against
   the cloned harness's launch command. If it's found and the harness
   declares `min_version`, `.await probe_tool_version(...)`, parse, and
   compare — this is the only point in the function that awaits, and it now
   does so with zero guards in scope.
3. *After* the probe resolves, re-validate both pieces of state captured
   before it (added post-implementation, per code review — see below), then
   acquire `state.workspace.lock()` as before to build `reporter_assets`,
   `orkworks_root`, and `ctx`, then call `action(&harness, &ctx)` using the
   owned clone from step 1.

Net effect: `detected_tool` construction moves earlier in the function
(before the workspace lock is even taken) rather than staying where it is
today; everything else keeps its current order. The workspace mutex is now
held only across the same fast, synchronous work it always was — the new
subprocess spawn happens entirely outside both locks.

**Re-validation after the probe.** The first implementation of this reorder
shipped with two real TOCTOU regressions, both caught by code review, since
dropping each lock before the probe trades away a consistency guarantee the
old fully-synchronous function had for free:

- *Workspace switch mid-request*: `ctx.workspace` used to come from the same
  single lock acquisition held for the whole function; after the reorder it
  came from a second, later acquisition taken after the probe, so a user
  switching OrkWorks's active workspace during the probe's window could
  cause install/uninstall to silently target the new workspace instead of
  the one active when the request was made.
- *Harness-catalog clone staleness*: the `ResolvedHarness` clone (step 1) was
  never re-checked; a concurrent `PATCH /harnesses/:id` landing mid-probe
  would leave the request executing `action()` against the stale pre-patch
  definition instead of blocking behind the write (the old read-guard-held-
  through-`action()` behavior) or picking up the fresh value.

Both are fixed the same way — capture an identity key *before* the probe,
and after it resolves, re-check the key and return `409 Conflict` if it
changed, rather than silently proceeding against stale or mismatched state:

- Workspace: capture `ws.path.clone()` at the same point the original
  `NoWorkspace` check already runs (before the harness lookup). After the
  probe, re-acquire the lock and compare `ws.path` against the captured
  value; mismatch → 409.
- Harness: after the probe, re-acquire `state.harness_catalog.read()`,
  re-fetch by `harness_id`, and compare `.definition` against the step-1
  clone's `.definition` (`HarnessDefinition` already derives `PartialEq`);
  mismatch or now-missing → 409.

This is simpler than adding a generation/revision counter to
`WorkspaceState` (none exists today) and matches the "reject on change"
semantics already used elsewhere in the codebase (`ConfigFileTransaction`'s
own revision guard). A narrower, related race remains *not* fixed: a
concurrent request could still clear the workspace in the gap between the
initial `NoWorkspace` check and the harness-registry lookup (both now
separate lock scopes), which could flip which error a request with both a
missing workspace and an unknown harness ID receives. Rated PLAUSIBLE rather
than CONFIRMED by review (the window is two back-to-back synchronous lock
operations with nothing in between) and left as an accepted, undocumented-
by-test edge case — it only affects which error status a rare dual-failure
request receives, not which workspace/harness gets acted on.

Resulting `DetectedTool` states:

- Tool not found → unchanged (`detected_tool: None`), independent of version
  logic.
- Tool found, harness has no `min_version` → unchanged
  (`version: None, compatible: true`), no subprocess spawned.
- Tool found, harness has `min_version` → probe, parse, compare:
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
  helpers) covering: parseable 3-component version above threshold, below
  threshold, parseable 2-component version (no patch number), a noisy
  `--version` banner with an unrelated leading numeric token, nonzero exit
  with no meaningful output, and a deliberately slow/hanging fake binary to
  exercise the 3-second timeout path. The hanging-binary test also asserts
  the child process is actually gone after the timeout fires (not just that
  the function returns), to pin the `kill_on_drop(true)` behavior rather
  than just its absence-of-crash.
- `definition.rs`: round-trip serde test for `VersionRequirement` in both
  `HarnessDefinition` and `HarnessPatch`, mirroring the existing
  `codex()`/`apply_patch` capability round-trip tests already in that file.
- `integration_handlers.rs`: an integration-style test asserting a harness
  with `min_version` set and a fake below-threshold binary on `PATH`
  produces `IntegrationActivation::NeedsTrust` + `unsupported_tool_version`,
  and one with an above-threshold fake binary produces normal activation —
  reusing the existing `FakeHome`/`test_app_state_with_workspace` scaffolding
  already present in that file.
- `integration_handlers.rs`: a concurrency test targeting the lock-reordering
  fix specifically — issue a request for a harness whose fake binary hangs
  past the probe timeout, and concurrently issue a second request (any
  harness, or a different endpoint that touches `state.workspace`); assert
  the second request completes promptly rather than waiting on the first
  request's ~3-second timeout. This is the regression test for "no guard is
  held across the `.await`," since a future incorrect reordering that
  reintroduces the held-lock bug would otherwise only surface as a compile
  failure (if the guard type stays `!Send`) or a latent contention bug (if
  it doesn't) — neither of which today's other tests would catch.
- `detect.rs`: two tests added post-implementation per code review —
  `parse_version_token` rejecting `-`/`+`-suffixed (prerelease/build-
  metadata) tokens instead of silently matching their numeric prefix, and
  `probe_tool_version` terminating within the timeout (returning `None`,
  not hanging or buffering unbounded memory) against a binary that never
  stops writing far past `MAX_PROBE_OUTPUT_BYTES`.
- `integration_handlers.rs`: two regression tests added post-implementation
  per code review, both using the same slow-hanging-binary pattern as the
  concurrency test above — one swaps the active workspace mid-probe (via a
  new `swap_workspace` test-support helper in `main.rs`, since
  `WorkspaceState` is private to that module) and asserts `409 Conflict`
  rather than the request silently completing against the new workspace;
  the other swaps the resolved harness-catalog registry directly mid-probe
  (bypassing `HarnessStore::mutate` — calling it twice in one test hits an
  unrelated pre-existing bug, filed as issue #230) and asserts `409
  Conflict` rather than the request completing against the stale
  pre-patch harness definition.
