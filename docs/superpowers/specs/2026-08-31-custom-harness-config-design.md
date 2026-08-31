# Custom Harness Configuration

- Status: approved for implementation planning
- Deciders: OrkWorks maintainers
- Date: 2026-08-31

## Context

OrkWorks already resolves embedded declarative harness definitions, sparse
built-in overrides, and complete custom definitions into one registry. The
sidecar also persists user harnesses in ~/.orkworks/harnesses.json and
projects harness Peon capabilities into the model-provider registry.

The missing product surface is a safe, understandable Settings workflow for
using that configuration model. The immediate use case is running two
Copilot-compatible commands independently:

- the normal Copilot CLI using copilot
- a local variant using copilot-local

The custom variant must preserve Copilot-compatible behavior while remaining
independent: the user must be able to disable the normal Copilot harness and
continue using only the local variant.

## Goals

- Let users duplicate a built-in harness into an independent custom harness.
- Let users inspect complete effective harness configurations.
- Let users edit complete custom definitions as JSON.
- Let users edit built-in behavior as sparse JSON overrides.
- Validate configuration before saving and again in the sidecar.
- Keep existing enable/disable, detection, command-path, and integration
  controls in the Coding tools Settings section.
- Reuse supported integration adapters without making the source harness an
  owner or prerequisite.
- Preserve a clean extension path for future declarative harnesses and
  harness-backed model providers.

## Non-goals

- A general plugin system for arbitrary Rust or hook code.
- User-defined integration protocols or arbitrary hook scripts.
- A linked extends relationship between a custom harness and its source.
- A new standalone model-provider editor in this slice.
- Automatic installation of hooks when a definition is created.
- Changes to OrkWorks' single-active-session context model.

## Core concepts

### Built-ins, overrides, and custom definitions

The registry has three user-visible origins:

1. Built-in is the shipped definition with no user override.
2. Built-in override is the shipped definition resolved with a sparse user
   patch.
3. Custom is a complete independent definition stored in the user document.

Built-in definitions are never mutated. A built-in override supplies only the
fields the user changed; unspecified fields continue to come from the shipped
definition. Resetting a built-in removes its override.

Duplicating a harness creates a complete custom snapshot. It does not retain a
base-harness relationship, so future changes to the source built-in do not
silently alter the custom tool. The custom ID is immutable after creation; the
display name remains editable.

For the immediate use case, duplicating Copilot creates a definition with
relevant fields such as these; unchanged fields are retained in the complete
custom snapshot:

~~~json
{
  "id": "copilot-local",
  "name": "Copilot Local",
  "launch": {
    "kind": "command-template",
    "command": "copilot-local",
    "args": ["--model", "{model}"],
    "modelPrefix": null
  },
  "integration": { "kind": "copilot" },
  "sessionSignals": { "kind": "copilot" }
}
~~~

The duplicate copies the source's other launch, resume, model, Peon,
capacity, voice, and supported capability fields unless the user changes
them.

### Harnesses, integrations, and providers are separate

These identities must not be conflated:

| Concern | Example |
| --- | --- |
| Harness/session launch | copilot-local |
| Workspace integration adapter | copilot |
| Peon model provider | copilot-local |
| Shipped default definition | copilot |

The custom harness gets its own session and Peon-provider identity. A copied
integration binding points to the existing, closed Copilot adapter. A custom
harness without an integration binding has no hook lifecycle.

Harness-backed providers continue to be projected from the resolved harness
registry. Therefore a copied harness with a peon capability becomes an
independent provider entry with its own model selection and capacity state,
even when it shares an integration adapter with another harness.

## Integration and hook lifecycle

An integration binding identifies a reusable, code-owned adapter. The adapter
owns the supported hook schema, target path, marker, reporter behavior, and
workspace mutation rules. The harness definition only references the adapter;
it does not own the installed hook file.

Integration status and mutations are grouped by:

- workspace
- integration adapter
- adapter target

They are not grouped by source harness or by whether a harness was duplicated.

For each save of active coding tools, OrkWorks computes the set of adapters
referenced by active harnesses:

- If at least one active harness uses the Copilot adapter, the shared Copilot
  hook remains installed.
- Disabling normal Copilot while copilot-local remains active leaves the
  Copilot hook installed.
- Disabling copilot-local while normal Copilot remains active also leaves it
  installed.
- The hook is eligible for removal only when no active harness uses the
  adapter.
- A shared install, repair, or uninstall operation is confirmed once per
  adapter, not once per harness row.

Every harness row that references the adapter shows the same underlying
status, with a clarification such as “Used by Copilot Local” or “Shared with
GitHub Copilot CLI.” A custom harness may opt out by setting its integration
binding to null; if it is active, the UI explains that it no longer uses
Copilot hook behavior.

Declaring a known adapter does not prove that a renamed or wrapped executable
supports the adapter's protocol. The editor therefore warns that the custom
command must remain compatible with the selected adapter. The existing
explicit integration confirmation continues to name the workspace file and
warn when OrkWorks-owned executable hook code will be installed.

## Settings experience

The existing Coding tools section remains the primary view. Its current
enable/disable toggles, detection status, command-path controls, integration
status, confirmation flow, and Save action remain available.

Each harness row additionally exposes:

- View config for the effective resolved definition.
- Duplicate for built-ins and custom harnesses.
- Edit override and Reset to default for built-ins.
- Edit JSON and Delete for custom harnesses.

The JSON editor is an in-place detail view within the same Settings section.
It does not replace the coding-tool list or remove its lifecycle controls.
A clear Back action returns to the list.

The editor identifies the configuration mode:

- Override JSON for a built-in sparse patch.
- Configuration JSON for a complete custom definition.

For both modes, a read-only effective-configuration preview shows the result
after built-ins, overrides, and custom values are resolved. The preview also
identifies inherited fields and the selected integration adapter.

The explanatory copy is part of the feature, not an incidental tooltip:

- Built-in override: “Only these fields are customized. Unspecified fields
  continue using the built-in defaults. Future built-in improvements will
  apply automatically.”
- Custom duplicate: “This is an independent copy. Future changes to the
  source harness will not modify it.”
- Shared integration: “This tool uses the Copilot integration. The shared
  hook remains installed while any active compatible tool uses it.”

Duplicate starts from the resolved source definition, proposes a unique
slugged ID and name, and opens the complete custom JSON for review before
saving. The duplicate operation itself does not install hooks.

## JSON validation and persistence

The editor uses strict JSON without comments. Client-side validation provides
fast feedback while typing:

- parse errors include line and column information
- required fields and field types are checked
- enum values and capability shapes are checked
- IDs are checked for syntax, collisions, and reserved built-in IDs
- command templates are checked for supported placeholders
- capability combinations and integration references are checked

An unavailable executable is a warning rather than a save failure. This lets
users configure a command before installing it or while using an environment
where detection is temporarily unavailable.

The sidecar is authoritative. It revalidates the complete request or sparse
patch, resolves it against the current registry, and rejects invalid
definitions before publishing them. Writes use the existing revision-aware
atomic persistence path. A concurrent edit returns a retryable conflict
instead of overwriting another user's configuration.

The sidecar response used by Settings must include enough metadata to
distinguish the resolved definition from its origin and stored override. The
resolved definition remains the source of truth for launch and runtime
consumers; the stored sparse patch is exposed only for the built-in override
editor.

The existing harness routes remain the conceptual API:

- list resolved harnesses with origin and override metadata
- create a complete custom definition
- update a built-in sparse patch
- replace a complete custom definition
- delete a custom definition or built-in override

No renderer process receives authority to mutate workspace hook files.
Electron main continues to mediate hook-mutating operations and confirmation,
while the sidecar owns registry validation, persistence, and integration
mutation execution.

## Future extensibility

### New declarative harnesses

A new command-based harness that fits the existing capability variants can be
added as a custom definition without a Rust adapter. Its definition can
declare launch arguments, model handling, resume templates, Peon invocation,
capacity patterns, label resets, and any existing compatible integration.

A future first-class built-in tool is a new embedded definition plus focused
tests. Built-in consumers continue to read the same resolved registry.

### New protocol-specific behavior

If a tool uses a hook schema, session protocol, or capability not represented
by the existing closed variants, it requires a reviewed compiled adapter.
JSON configuration may reference an existing adapter but cannot introduce
new executable integration code or authority-bearing hook behavior.

### Model providers

The existing separation remains:

- harness-backed Peon providers are projected from harness definitions
- standalone local inference providers such as Ollama remain provider
  definitions with their own settings and verification
- provider selection and model settings remain in the Model providers
  Settings section

This harness feature does not require merging provider and harness documents.
It only ensures that custom harness Peon definitions are projected through
the same provider path as built-ins.

## Error handling

The UI must keep the user's draft visible when validation or persistence
fails. Errors are associated with the relevant JSON field when possible;
sidecar diagnostics are displayed without replacing the draft.

Integration-related errors distinguish among:

- invalid or unsupported adapter reference
- a missing or undetected executable warning
- an invalid or drifted workspace hook file
- a user cancellation of the hook confirmation
- a revision conflict in harness configuration
- a workspace change during an integration operation

Saving a definition and changing active tools are separate operations. A
successful definition save does not silently enable the tool or install its
integration. The existing explicit Save tools flow and confirmation remain
the point where active-tool and shared-hook mutations occur.

## Testing strategy

### Sidecar and registry

- Resolve built-in, overridden, and custom definitions with correct origins.
- Duplicate Copilot into copilot-local as a complete independent snapshot.
- Reject reserved-ID collisions and malformed custom definitions.
- Preserve unspecified built-in fields when applying sparse overrides.
- Reset an override and recover the shipped definition.
- Project a custom harness Peon capability into an independent provider entry.
- Preserve independent model and capacity state for copilot and copilot-local.
- Group integration status and mutations by adapter and target.
- Keep the shared Copilot hook when only the child is active.
- Remove it only after the last active compatible harness is disabled.
- Reject unsupported adapter references and incompatible capability shapes.
- Exercise revision conflicts and atomic write failures.

### Desktop and Settings

- Preserve existing active-tool toggles, detection, command-path controls,
  hook confirmation, and Save behavior.
- Display the correct origin badge and editor mode.
- Keep the effective preview synchronized with a valid draft.
- Block saving invalid JSON while preserving the draft and diagnostics.
- Show an unavailable command as a warning.
- Explain copied integration behavior and independent custom ownership.
- Allow the original Copilot to be disabled while copilot-local remains
  active.
- Avoid issuing duplicate install/uninstall operations for a shared adapter.

### Compatibility

- Existing v2 harness documents continue to load unchanged.
- Existing v1 legacy migration remains intact.
- Existing built-in overrides continue to use sparse patches.
- Existing provider settings continue to accept projected harness-backed
  provider IDs.
- Existing workspace hook files and unrelated configuration remain
  preserved by the integration transaction rules.
