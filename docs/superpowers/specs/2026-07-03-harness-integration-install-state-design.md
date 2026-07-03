# Harness Integration Install State Design

- Date: 2026-07-03
- Status: proposed

## Summary

OrkWorks should make harness readiness visible and actionable in Settings by separating three different questions that are currently conflated:

- `enabled`: is this coding tool available in the current workspace
- `detected`: can the OrkWorks app environment resolve the harness command on `PATH`
- `installed`: has OrkWorks installed the harness-specific integration it owns, such as hooks or config entries

The first slice should add a small internal integration-driver layer in the Rust sidecar for built-in harnesses. Each driver is responsible for status detection plus install/uninstall behavior for OrkWorks-managed integration only. Built-in harnesses remain visible at all times; uninstalling a built-in harness disconnects OrkWorks integration but does not remove the external CLI from the machine or delete the harness row from OrkWorks.

This slice should include:

- typed integration status per harness, not just a boolean
- explicit ownership tracking for any hook/config OrkWorks installs
- generic install/uninstall/status endpoints for harness integration
- a Settings UI that shows `Enabled`, `Detected`, and `Installed`
- Claude Code migrated from a one-off attention-hook button to the shared harness integration model

This slice should not include:

- uninstalling third-party CLIs from the machine
- deleting built-in harness definitions
- a dynamic plugin runtime, external installer marketplace, or third-party extension API
- speculative integration logic for harnesses that do not yet have a concrete OrkWorks-managed hook/config path

## Problem

The current Settings UI has workspace-level active-harness toggles and a Claude-specific "Install attention hook" action, but it does not answer the actual operational questions users need:

- Can this harness be used from this app launch?
- Has OrkWorks finished its own setup for this harness?
- Can I safely remove the OrkWorks-managed integration later?

Without a shared model:

- harness readiness is inconsistent and partly hidden
- Claude-specific install logic becomes a pattern copy trap for every future harness
- uninstall has no clear ownership boundary and risks deleting user-managed config
- the UI cannot distinguish "disabled", "command missing", and "integration not installed"

## Design Goals

- Make harness readiness explicit with separate `enabled`, `detected`, and `installed` concepts.
- Keep built-in harnesses always visible, even when disconnected.
- Scope uninstall to OrkWorks-managed integration only.
- Make install/uninstall idempotent and ownership-safe.
- Keep the architecture small: a built-in Rust driver registry, not a heavyweight plugin system.
- Preserve existing harness CRUD boundaries: custom harness deletion remains separate from integration uninstall.

## Non-Goals

- No attempt to install or uninstall the external CLI binary.
- No automatic integration writes at session spawn time.
- No dynamic driver loading, WASM/plugin host, or config-defined installer DSL.
- No requirement that every built-in harness must immediately support install/uninstall beyond status reporting.
- No change to the workspace `enabled` toggle semantics beyond clearer display and behavior.

## Proposed Design

### State Model

Each harness row combines three independent axes:

#### Enabled

Workspace-scoped existing toggle:

- `true`: available for new sessions in this workspace
- `false`: hidden from the new-session picker for this workspace

`enabled` does not imply the command exists or that OrkWorks integration is installed.

#### Detected

App-environment command discovery:

- `detected`: command resolved from the sidecar/app environment
- `not_detected`: command not found in the sidecar/app environment
- `unknown`: detection failed or could not be completed

The status payload should include the resolved executable path when detected, or an explanatory note when not detected, because the Electron/sidecar environment may differ from the user's interactive shell.

#### Installed

OrkWorks-managed integration state:

- `not_installed`
- `installed`
- `partial`
- `outdated`
- `conflict`
- `unsupported`
- `unknown`

`installed` is intentionally typed rather than boolean because hook/config state can drift, partially fail, or conflict with user-managed edits.

Each installed-state payload must also carry ownership/provenance information for uninstall decisions:

- `owned`: OrkWorks can prove the installed integration was created or adopted by OrkWorks
- `unowned`: integration exists, but OrkWorks cannot prove ownership
- `unknown`: ownership could not be determined

This ownership field is separate from the install-state enum because `installed` alone is not enough to decide whether uninstall is safe.

### Behavior Matrix

- `enabled=false`, `installed=installed`: valid. The harness stays disconnected from session creation in this workspace, but any already-installed hook/config remains present until explicitly uninstalled.
- `enabled=true`, `detected=not_detected`: valid. The UI should show the harness as available in principle but not runnable from the current app environment.
- `enabled=true`, `detected=detected`, `installed=not_installed`: valid. The harness can launch, but OrkWorks-specific integration is not configured.
- `installed=partial|outdated|conflict`: the UI should not flatten these to "installed". They require specific messaging and usually a repair/reinstall action.
- `installed=unsupported`: valid. The harness has no current OrkWorks-managed integration flow, so the UI should show status only and no install/uninstall action.
- `installed=installed`, `ownership=unowned|unknown`: valid. The UI should show the integration as present but uninstall must be blocked.

This avoids implying that a harness is fully ready when only one of the three axes is satisfied.

### Internal Driver Layer

Add a small internal integration-driver registry in the Rust sidecar for built-in harnesses only.

Each driver answers:

- detection status for the harness command
- current integration install state
- install operation for OrkWorks-managed integration
- uninstall operation for OrkWorks-managed integration
- user-facing status detail text

This is a narrow in-process abstraction, not a plugin framework. It should be implemented as a small trait or enum-backed registry internal to `orkworksd`, with one driver per built-in harness where integration support exists.

For the first slice:

- Claude Code gets full detect/install/uninstall/status support for the Notification hook path
- other built-in harnesses may initially return detection status plus `unsupported` integration state if no OrkWorks-owned integration exists yet

### Ownership Tracking

Uninstall safety depends on explicit ownership tracking.

Install operations must either:

- write identifiable OrkWorks-owned markers into the installed config entries, or
- persist a manifest of exactly what OrkWorks added and where

The uninstall path must remove only OrkWorks-owned entries. If the config has drifted and ownership cannot be proven, the driver must return `conflict` or `not_owned_by_orkworks` rather than deleting ambiguous user-managed configuration.

For Claude Code specifically, this means the current substring-based idempotency/status check is not sufficient for safe uninstall on its own. The Claude integration needs a stronger ownership marker or manifest before uninstall is added.

Ownership data for workspace-local integrations should live with the workspace-scoped integration, not as a global machine-wide assertion. For Claude Code, that means any manifest, if used, should live in workspace-scoped OrkWorks metadata rather than a global cross-workspace store.

### Scope Rules

The three axes intentionally live at different scopes, and the API/UI must not blur them:

- `enabled` is workspace-scoped
- `detected` is app-environment scoped for the current OrkWorks launch
- `installed` is integration-scoped; for Claude Code in this slice, it is workspace-scoped because `.claude/settings.local.json` is workspace-local

`GET /harnesses/integration-status` should return the mixed-scope view explicitly. When no workspace is open:

- `detected` should still be reported
- `enabled` should be omitted or reported as unavailable with a workspace-scoped detail
- workspace-local `installed` states should return `unknown` with a detail like "Open a workspace to inspect integration state."
- install/uninstall actions should be unavailable

The response model should make this explicit rather than implying every field is always meaningful in every app state.

### API Shape

Keep harness inventory and harness integration state separate.

`GET /harnesses` should remain the stable inventory/config endpoint for harness definitions. Integration status should be exposed through a companion endpoint so status checks can evolve independently and avoid slowing basic harness reads if some checks involve filesystem or config inspection.

Proposed endpoints:

- `GET /harnesses` — existing harness inventory/config
- `GET /harnesses/integration-status` — per-harness integration status map
- `POST /harnesses/:id/install` — install OrkWorks-managed integration for that harness
- `POST /harnesses/:id/uninstall` — uninstall OrkWorks-managed integration for that harness

Suggested status response shape:

```json
{
  "harnessId": "claude-code",
  "enabled": {
    "state": "enabled",
    "scope": "workspace"
  },
  "detected": {
    "state": "detected",
    "scope": "app_environment",
    "resolvedPath": "/usr/local/bin/claude",
    "detail": "Command found in app environment."
  },
  "installed": {
    "state": "installed",
    "scope": "workspace",
    "ownership": "owned",
    "detail": "Notification hook installed in .claude/settings.local.json."
  },
  "actions": {
    "canInstall": false,
    "canUninstall": true
  }
}
```

Install/uninstall responses should carry typed error codes rather than generic strings when possible, for example:

- `command_not_found`
- `permission_denied`
- `config_conflict`
- `not_owned_by_orkworks`
- `partial_install`
- `unsupported_harness`
- `workspace_required`

Install/uninstall should be idempotent success operations when the end state is already satisfied. Repeating install on an already-installed owned integration should return success plus current status; repeating uninstall when nothing OrkWorks owns is installed should also return success plus current status. Typed errors should be reserved for real failures or unsafe states, not for harmless repeats.

### Settings UI

Replace the current bare "Active coding tools" checkbox rows with richer harness rows/cards that show:

- harness name
- workspace `Enabled` state
- `Detected` state
- `Installed` state
- detail text explaining the current condition
- action buttons appropriate to the current state

Examples:

- `Detected: Yes`
- `Installed: Not installed`
- `Claude CLI found in app environment. Notification hook not installed.`

Action behavior:

- `Install` when the harness is detected and integration is not installed
- `Uninstall` when OrkWorks-managed integration is installed and ownership is `owned`
- no install/uninstall action for `unsupported`
- disabled or replaced with explanatory text for `partial`, `conflict`, `unknown`, `ownership=unowned|unknown`, or workspace-unavailable states

The existing workspace-level save flow for enabled harnesses can remain, but the status/action portion should refresh from the backend after every install/uninstall attempt so the UI reflects the real state rather than an optimistic assumption.

### Built-In vs Custom Harnesses

Built-in harnesses:

- always remain visible
- are never deleted through uninstall
- retain independent `enabled`, `detected`, and `installed` state; uninstall changes only OrkWorks-managed integration state

Custom harnesses:

- continue to use existing harness CRUD behavior
- may or may not participate in the integration-driver system later
- deletion remains a separate action from integration uninstall

## Claude Code Migration

Claude Code is the concrete first migration:

- move the current Settings-specific attention-hook affordance behind the shared integration status model
- keep the underlying Notification hook mechanism and explicit user confirmation
- strengthen install ownership tracking so uninstall can safely remove only the OrkWorks-installed hook/config

This design does not require equivalent install support for Codex, OpenCode, Gemini CLI, or Aider immediately. Until a concrete OrkWorks-owned integration exists, they should report `detected` plus `installed.state = unsupported`.

For the first slice, built-in harnesses without a concrete OrkWorks-managed integration should report `installed.state = unsupported`, not `not_installed`.

### Legacy Claude Migration

Existing Claude hook installs may predate ownership tracking. The first implementation must define a stable backward-compatibility rule:

- if a Claude hook entry is present but matches only the old substring-based detection and has no ownership marker/manifest, report it as `installed.state = conflict` with `ownership = unknown`
- offer reinstall/adopt behavior only through an explicit user action
- do not allow uninstall of that legacy state until OrkWorks can either adopt it safely or replace it with an owned install

This keeps old installs visible without pretending uninstall is safe.

## Error Handling And UX

- If no workspace is open, install/uninstall should fail with a clear workspace-scoped error.
- If the command is missing from the app environment, install should fail with `command_not_found`.
- If the target config is malformed or modified in a way ownership cannot be proven, return `config_conflict` or `not_owned_by_orkworks` and leave files untouched.
- If uninstall is requested when nothing OrkWorks owns is installed, return success with unchanged current status.
- After every action, the renderer should re-read integration status from the backend before updating badges or button states.

## Testing And Validation

Implementation should verify:

- driver-level status evaluation for detected/not-detected/unknown command states
- typed installed-state transitions, including `partial`, `conflict`, and `unknown`
- install idempotency for repeated actions
- uninstall safety when ownership markers/manifests are missing or ambiguous
- Claude-specific hook install/uninstall round trip, including malformed config and drifted config cases
- legacy Claude hook detection without ownership markers, including blocked uninstall behavior
- handler coverage for status/install/uninstall endpoints and typed errors
- renderer coverage for the Settings state matrix so actions and messaging change correctly across combinations

Important negative cases:

- uninstall must not remove user-managed config it cannot prove OrkWorks created
- detected state must reflect the sidecar/app environment, not assume the user's login shell
- disabling a harness must not silently uninstall integration

## Open Questions

- For Claude Code ownership tracking, is an inline marker in the config entry sufficient, or is a manifest file under `~/.orkworks/` safer? Either is acceptable as long as uninstall safety is explicit and testable.

## Documentation Impact

Required before implementation lands:

- `docs/agents/architecture.md` for the new harness integration status/install/uninstall endpoints and preload/Electron surface changes
- `README.md` and `AGENTS.md` only if the user-visible harness setup workflow or repo workflow description changes materially
- any harness-specific setup notes that reference the old Claude-only button path

No ADR is required unless implementation expands into a general plugin/runtime extension system or changes the core trust boundary around config ownership.
