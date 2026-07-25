# Harness tool detection (auto + manual override)

## Context

Settings shows an `Enabled` / `Detected` / `Installed` row per coding tool
(issue #180, originally closed as superseded by #204, reopened after
inspection showed the `Detected` half was never implemented). Every harness
currently renders "The coding tool was not detected, so integration
activation is unknown" regardless of whether the tool is actually installed,
because `IntegrationContext.detected_tool` is hardcoded to `None` at its one
production call site:

```rust
// crates/orkworksd/src/http/integration_handlers.rs:105-112
let ctx = IntegrationContext {
    ...
    detected_tool: None,
    ...
};
```

The `DetectedTool { executable, version, compatible }` type and the
`IntegrationContext.detected_tool` field already exist
(`crates/orkworksd/src/harness/integration.rs:496-504`) and are already
consumed by the diagnostic logic in
`crates/orkworksd/src/harness/integrations/mod.rs:213-239` — nothing
downstream needs to change, only the thing that populates the field.

Each harness definition already carries the real binary name it launches
(`launch.command`, e.g. `"claude"`, `"codex"`, `"opencode"`,
`resources/harnesses-v2.json`), and the backend already has a generic
mechanism to override that field per harness: `PATCH /harnesses/:id` with a
`BuiltinPatch { launch: { command } }`, persisted in
`~/.orkworks/harnesses.json` (`crates/orkworksd/src/http/harness_handlers.rs`,
`crates/orkworksd/src/harness/store.rs`). No renderer/Electron code calls this
endpoint today — there is no UI for editing any harness definition field yet.

## Goals

1. **Automatic detection**: probe whether a harness's configured launch
   command resolves to a real, executable file, and feed that result into
   `IntegrationContext.detected_tool` instead of the hardcoded `None`.
2. **Manual override**: when the automatic probe is a false negative (most
   likely cause: Electron's process `PATH` is thinner than the user's
   interactive shell `PATH` — missing `nvm`/`asdf`/Homebrew shims), let the
   user type an absolute path to the binary. The override is stored as the
   harness's `launch.command`, so it fixes both detection and the actual
   launch/resume/Peon invocation together. The user can clear it back to the
   built-in default.

## Non-goals

- Version parsing or minimum-version compatibility gating. `compatible` is
  always `true` once a binary is found; nothing downstream currently
  consumes a real version number or gates on one.
- Any caching layer for probe results. The probe is pure filesystem
  metadata checks (no subprocess spawn), cheap enough to run on every
  Settings status request.
- A new persisted field for the override. It reuses the existing
  `launch.command` override mechanism.
- Editing any other harness definition field (args, models, capacity, etc.)
  from this UI. Only the launch command, and only in service of detection.

## Design

### Backend: detection probe

New file `crates/orkworksd/src/harness/detect.rs`:

```rust
pub(crate) fn probe_installed_tool(command: &str) -> Option<DetectedTool>
```

Behavior:

- Empty `command` (e.g. a `PlatformShell` harness, which has no fixed binary
  name) → `None`.
- If `command` looks like a path (contains a path separator, or
  `Path::new(command).is_absolute()`) → treat it as a direct reference: check
  that the path exists and is executable. Return `Some`/`None` accordingly.
  This is also the path a manual override takes, since an override replaces
  `launch.command` with an absolute path.
- Otherwise (a bare command name) → walk `PATH` via
  `std::env::split_paths(&env::var_os("PATH")?)` (stdlib; handles the
  POSIX/Windows separator difference automatically — no need to hand-roll
  platform splitting the way `ReporterPlatform` does for hook scripts).
  For each directory, check `dir.join(command)` for existence + executable
  bit on POSIX (`PermissionsExt::mode() & 0o111 != 0`), and
  `dir.join(command).with_extension(ext)` for each extension in `PATHEXT`
  (falling back to `exe;cmd;bat` if unset) on Windows. Return the first
  match as `Some(DetectedTool { executable, version: None, compatible: true })`.

No subprocess is spawned anywhere in this probe.

### Backend: wiring into `IntegrationContext`

`crates/orkworksd/src/http/integration_handlers.rs`, in
`run_integration_action`: the resolved `harness: &ResolvedHarness` is already
in scope before the `IntegrationContext` is built. Extract its effective
launch command using the same match on `LaunchCapability` that
`crates/orkworksd/src/harness/registry.rs:408-411` already performs — factor
that match into one small shared helper (e.g. on `ResolvedHarness` or as a
free function in `registry.rs`) since it will now have two call sites. Call
`detect::probe_installed_tool(&command)`, bind the result to a local, and
pass `detected_tool: probed.as_ref()` into the context instead of `None`.

### Backend: manual override

No new backend surface. The frontend calls the existing
`PATCH /harnesses/:id`:

```json
{ "kind": "BuiltinPatch", "patch": { "launch": { "command": "/opt/homebrew/bin/claude" } } }
```

"Clear override" calls the existing `DELETE /harnesses/:id`, which removes
the harness's entire override document
(`crates/orkworksd/src/http/harness_handlers.rs::delete_harness`). Today
that's equivalent to clearing just the launch-command override, since no
other UI writes harness overrides yet. If a future feature adds per-field
override editing elsewhere, `DELETE` will need to become field-scoped instead
of whole-document — leave a comment at the clear call site noting this.

### Frontend

- `apps/desktop/electron/preload.ts` / `main.ts`: add
  `setHarnessCommandOverride(harnessId, path)` and
  `clearHarnessCommandOverride(harnessId)`, mirroring the existing
  `callIntegrationRoute` helper and its `{ ok, ... } | { ok: false, error }`
  result shape (`main.ts:343-360`). `set` issues the `PATCH` above; `clear`
  issues the `DELETE`.
- `apps/desktop/src/orkworksWindow.d.ts`: add the two methods to the
  `window.orkworks` type.
- `apps/desktop/src/components/SettingsModal.tsx`: for a harness row whose
  integration status carries the `tool_not_detected` diagnostic, render a
  "Custom path" text input + Save button. Prefill the input if the harness's
  currently effective `launch.command` already looks like an absolute path
  (the same path-like heuristic as the backend probe) — that's the signal an
  override is already active, since a bare name like `"claude"` is clearly
  the un-overridden built-in default. Show "Clear" only when that heuristic
  fires. This avoids adding a new `isOverridden` flag to the harness API
  response.

### Testing

- `detect.rs` unit tests: bare command found on `PATH`, bare command not
  found, absolute path that exists and is executable, absolute path that
  doesn't exist, empty command, (Windows) extension resolution.
- `integration_handlers`-level test (or a test on `run_integration_action`'s
  probe wiring) confirming a harness whose `launch.command` resolves now
  reports `tool_detected: true` and a real `IntegrationActivation`, not
  `Unknown`.
- Manual/browser check: Settings renders a real Detected state for Claude
  Code, Codex, and OpenCode when installed, and the custom-path override
  flow works end to end for a deliberately-wrong command.
