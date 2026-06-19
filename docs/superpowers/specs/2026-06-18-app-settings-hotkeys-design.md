# App Settings and Configurable Hotkeys — Design

> **Date:** 2026-06-18
> **Scope:** Desktop app settings persistence and hotkey customization

## Goal

Introduce a persisted app-level settings object for the Electron desktop app, with the first implemented settings slice limited to configurable hotkeys for the shortcuts that already exist today.

The user-facing outcome is:

- a stable `settings` object that can grow over time
- an in-app settings UI for editing hotkeys
- Electron menu accelerators that reflect saved user preferences instead of hard-coded values

## Why This Change

The desktop app already persists app-owned state in Electron `userData`:

- workspace memory in `workspace-memory.json`
- Dockview layout in `layout.json`

Hotkeys are the remaining obvious piece of app behavior that is currently hard-coded in `electron/main.ts`. That makes them:

- difficult to customize
- difficult to centralize as product settings
- awkward to evolve into a broader settings model later

Adding a proper app settings document now creates a clean ownership boundary before more settings categories exist.

## Recommended Approach

Adopt a main-process-owned settings model:

- persist `settings.json` in Electron `userData`
- define a broad top-level app settings object
- implement only the `hotkeys` section in the first version
- expose read/write access to the renderer through the preload bridge
- rebuild the Electron menu from saved settings so menu accelerators remain the source of truth for active shortcuts
- keep settings persistence and menu application consistent so a failed menu rebuild cannot leave disk state ahead of the active app menu

This keeps settings persistence and native shortcut behavior in the same privileged layer instead of splitting responsibility between renderer-only keyboard handlers and Electron menu state.

## Non-Goals

- Do not add new hotkey actions beyond what already exists today.
- Do not introduce renderer-only shortcut behavior that diverges from the Electron menu.
- Do not add non-hotkey settings in this change.
- Do not redesign the rest of the titlebar or panel structure beyond what is needed to open and use the settings UI.
- Do not treat this design doc as authoritative product scope for implementation.

## Current Hotkey Scope

The first version only covers shortcuts already implemented in `apps/desktop/electron/main.ts`:

- `newSession`
- `toggleSessionsPanel`
- `toggleDetailPanel`
- `toggleTerminalPanel`
- `toggleCapacityPanel`
- `toggleRecommendationsPanel`
- `resetLayout`

Default values must match the current hard-coded accelerators exactly:

- `newSession`: `CmdOrCtrl+N`
- `toggleSessionsPanel`: `CmdOrCtrl+Shift+S`
- `toggleDetailPanel`: `CmdOrCtrl+Shift+D`
- `toggleTerminalPanel`: `CmdOrCtrl+Shift+T`
- `toggleCapacityPanel`: `CmdOrCtrl+Shift+C`
- `toggleRecommendationsPanel`: `CmdOrCtrl+Shift+R`
- `resetLayout`: unset by default unless the product later assigns one intentionally

`toggleSessionsPanel` has a slightly different current behavior than the other panel shortcuts: it focuses or restores the sessions list, and closes the panel only when the list is already focused. The implementation must preserve that behavior even if the settings key keeps the existing `toggleSessionsPanel` name for compatibility with the rest of the panel shortcut group.

## Settings Shape

The persisted object should be broad from the start even though only one section is populated:

```ts
export interface AppSettings {
  version: 1;
  hotkeys: HotkeySettings;
}

export interface HotkeySettings {
  newSession: string;
  toggleSessionsPanel: string;
  toggleDetailPanel: string;
  toggleTerminalPanel: string;
  toggleCapacityPanel: string;
  toggleRecommendationsPanel: string;
  resetLayout: string | null;
}
```

This shape provides:

- explicit versioning for future migrations
- a stable top-level contract for later categories such as `ui` or `terminal`
- no need to refactor persistence if more settings are added later

## Architecture

Ownership should follow existing Electron security boundaries:

- `electron/main.ts` owns menu construction and active accelerators
- a new Electron-side settings memory module owns persistence and default merging
- `electron/preload.ts` exposes narrow read/write APIs to the renderer
- React components own settings presentation and editing flow

The renderer should not become the source of truth for active hotkeys. Its responsibility is:

- display current values
- capture proposed edits
- submit only the hotkey changes it owns, or submit a versioned patch that the main process merges with the latest stored settings
- render validation errors returned from the main process

The main process should:

- load settings on startup
- merge missing values with defaults
- validate requested changes before writing
- construct and validate the next application menu before committing persisted changes
- persist the saved result and rebuild the application menu as one observable operation

The main process must preserve settings sections that the current renderer did not edit. This prevents a stale settings modal from overwriting future `ui`, `terminal`, or other sections after the top-level `AppSettings` object grows.

## Persistence Model

Create a settings memory module adjacent to the existing Electron persistence helpers.

Responsibilities:

- define default settings
- read `settings.json`
- fall back safely when the file is missing or invalid
- write canonical JSON back to disk
- normalize partially populated or older settings documents into the current `version: 1` shape

Corrupt or missing settings must not block startup. The app should continue using defaults.

## Preload and IPC

Expose narrow preload methods:

- `getSettings(): Promise<AppSettings>`
- `saveHotkeys(hotkeys: HotkeySettings): Promise<AppSettings>`

`saveHotkeys` should merge the submitted hotkey section into the latest main-process settings document, then return the canonical saved document after validation/default normalization so the renderer always refreshes from the authoritative source.

If a future settings UI needs to edit multiple sections at once, use explicit patch semantics or optimistic revision checks rather than accepting an arbitrary full `AppSettings` object from the renderer.

No direct filesystem access should be added to the renderer.

## Menu Integration

Replace the current hard-coded accelerator constants in `buildMenu()` with values read from app settings.

Menu construction should use a single mapping layer from settings keys to menu actions so that:

- saved accelerators populate menu items
- checkbox/toggle menu behavior stays unchanged
- future additions can extend the mapping without scattering more string constants

After a successful save, the main process should rebuild and reapply the application menu immediately so changes take effect without a restart.

Menu rebuild must be planned before disk commit:

1. normalize and validate the proposed hotkeys
2. build the next menu template from the canonical settings
3. build the Electron menu object successfully
4. write canonical settings to disk
5. apply the menu with `Menu.setApplicationMenu(menu)`
6. return canonical settings to the renderer

If steps 1-3 fail, settings are not written and the active menu remains unchanged. If persistence fails, the new menu is not applied. This avoids a common partial-apply failure where `settings.json` says one shortcut is active but the running menu still uses the previous shortcut.

## In-App Settings UI

Add a lightweight settings entry point in the titlebar and render the settings experience as a renderer modal rather than a separate Electron window.

The first version should include:

- a `Settings` button or icon in the titlebar
- a modal surface with a single `Hotkeys` section
- one row per supported hotkey action

Each row should show:

- human-readable action label
- current accelerator display
- `Edit` action
- `Reset` action for that row

The modal should also offer:

- `Restore defaults`
- `Cancel`
- `Save`

This keeps the information architecture aligned with a future broader settings object without inventing empty categories today.

## Hotkey Capture Flow

Editing a hotkey should use an explicit capture mode:

1. user clicks `Edit`
2. the row enters a focused capture state
3. the next key chord is captured and normalized into Electron accelerator format
4. the proposed value is previewed in the row
5. the user saves or cancels the modal

The app should not live-apply every captured chord as the user types. Changes should apply only after explicit save.

Capture mode should support:

- modifier keys combined with a non-modifier key
- normalization to a consistent display format
- clearing an optional shortcut such as `resetLayout`

Capture mode must also prevent existing Electron menu accelerators from firing while the modal is recording a chord. For example, capturing `CmdOrCtrl+N` must not create a new session. The implementation can satisfy this by temporarily disabling OrkWorks-managed accelerators during capture, or by routing capture through main-process input handling such as `before-input-event` before normal menu command dispatch.

## Validation Rules

Validation should live in the main process so the trusted layer rejects invalid state before persisting or applying it.

The first version should reject:

- invalid Electron accelerator syntax
- duplicate shortcuts across OrkWorks-managed hotkeys
- empty values for required hotkeys

Validation errors should be action-specific so the renderer can point to the exact conflicting or invalid row.

If validation fails:

- settings are not written
- the existing menu remains unchanged
- the modal stays open
- the renderer displays the returned error

## Component and File Changes

Likely file-level changes:

- add `apps/desktop/electron/settingsMemory.ts`
- modify `apps/desktop/electron/main.ts`
- modify `apps/desktop/electron/preload.ts`
- modify `apps/desktop/src/App.tsx`
- add a renderer settings modal component under `apps/desktop/src/components/`
- update renderer typing for `window.orkworks`
- add tests for Electron settings persistence and renderer UI flows

No backend (`orkworksd`) changes are required for this feature.

## Error Handling

Settings must fail soft:

- unreadable or corrupt `settings.json` falls back to defaults
- validation failures are returned as structured UI errors
- renderer save failures do not leave partially applied menu state

The canonical sequence is:

1. renderer submits proposed settings
2. main process validates
3. main process builds the next menu object without applying it
4. main process writes settings
5. main process applies the rebuilt menu
6. main process returns canonical saved settings

If any earlier step fails, later steps do not run.

## Testing

Add automated coverage for:

- settings read/write round-trip
- missing-file fallback to defaults
- corrupt-file fallback to defaults
- default value parity with current hard-coded accelerators
- duplicate hotkey rejection
- invalid accelerator rejection
- preload IPC load/save wiring
- menu construction using saved settings
- save failure behavior where invalid menu construction or persistence errors do not partially apply settings
- capture mode suppressing existing menu accelerator execution while recording a chord
- renderer modal rendering and edit/reset/default flows

Manual verification should confirm:

- the settings modal opens from the titlebar
- editing a shortcut updates the Electron menu accelerator after save
- recording an existing shortcut such as `CmdOrCtrl+N` captures the chord without triggering the current action
- invalid or duplicate assignments are rejected with a clear error
- restoring defaults returns all shortcuts to the current shipped values
- app restart preserves saved shortcuts

## Risks

### Source of Truth Drift

If renderer capture logic and main-process validation normalize shortcuts differently, users will see confusing round-trips.

Mitigation:

- keep canonical validation/normalization in the main process
- have the renderer refresh from the saved result after every successful save

### Menu Rebuild Regression

Rebuilding the Electron menu after save could accidentally break panel checkbox state or command wiring.

Mitigation:

- keep menu action identifiers stable
- preserve the existing panel visibility notification flow
- add regression tests around menu item construction

### Settings Scope Confusion

Because the object is intentionally broad, future readers may assume more settings are implemented than actually exist.

Mitigation:

- keep `AppSettings` minimal in `version: 1`
- document that only `hotkeys` is active in the first slice

## Rollout Notes

This design intentionally stops short of implementation approval.

Current repo constraints:

- the authoritative product specs do not currently cover app settings or configurable hotkeys
- the issue board does not currently include a corresponding implementation issue
- the project ADR workflow requires recording significant architecture, stack, protocol, or boundary decisions before implementation code

Before implementation begins, the project should:

1. update the authoritative product spec to include this scope
2. write an ADR for the main-process-owned settings boundary and menu-source-of-truth decision, then update the ADR index
3. create the corresponding GitHub issue
4. then write the implementation plan and execute it under the normal TDD workflow
