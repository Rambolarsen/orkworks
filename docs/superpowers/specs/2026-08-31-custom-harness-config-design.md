# Custom Harness Configuration

- Status: approved for implementation planning
- Deciders: OrkWorks maintainers
- Date: 2026-08-31
- Review follow-up: 2026-08-31

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
- Make concurrent edits, deletion, shared integration reconciliation, and
  provider lifecycle explicit and observable.

## Non-goals

- A general plugin system for arbitrary Rust or hook code.
- User-defined integration protocols or arbitrary hook scripts.
- A linked extends relationship between a custom harness and its source.
- A new standalone model-provider editor in this slice.
- Automatic installation of hooks when a definition is created.
- Changes to OrkWorks' single-active-session context model.
- User-authored selection of compiled signal handlers, reporters, hook paths,
  or integration implementations. The only exception is the controlled,
  server-created compatibility profile described below.

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
  }
}
~~~

The duplicate copies the source's other launch, resume, model, Peon,
capacity, voice, and supported capability fields unless the user changes
them. Integration and native signal bindings are not editable JSON fields;
they are controlled by the duplicate operation's read-only compatibility
profile.

### Code-owned compatibility profiles

Compiled integrations and native session-signal handlers are a trust boundary.
The editable harness document must not be able to name a Rust handler,
reporter script, hook path, or arbitrary integration kind. In particular:

- Custom JSON cannot set `integration` or `sessionSignals`.
- Built-in sparse overrides cannot change those code-owned bindings.
- The sidecar exposes profile metadata and the effective binding as
  read-only values in Settings; the JSON editor cannot add or edit them.
- A duplicate request is a sidecar operation, not a renderer-side
  copy-and-create. The sidecar resolves the source and returns a snapshot plus
  a proposed ID/name for the editor. On the final create, the sidecar uses the
  source ID and expected document revision to attach the source's allowlisted
  compatibility profile, if any, to the new custom record. A custom source
  therefore carries the same profile and hooks; it does not create a new
  adapter binding.
- A custom replacement preserves its existing profile but cannot add, remove,
  or change it through JSON. Removing a profile is an explicit Settings action
  that performs integration reconciliation and confirmation when needed.
- A profile is resolved only through a code-owned allowlist. The profile maps
  to the existing adapter and native signal contract; it never carries
  executable user input.

Profile assignments are persisted separately from editable definitions in a
sidecar-owned map keyed by immutable custom harness ID, for example
`compatibilityProfiles: { "copilot-local": "copilot" }`. The profile map is
not accepted in custom JSON and is not editable by a generic replace request.
The user document advances from version 2 to version 3 for this metadata; the
v2-to-v3 migration creates an empty map and preserves all existing overrides
and custom definitions. Deleting a custom definition removes its profile map
entry. Unknown profile IDs fail closed during load and are surfaced as an
unavailable integration rather than guessed or executed.

At runtime, registry resolution combines the editable definition with this
sidecar-owned profile map and the compiled allowlist to produce the effective
native-signal and integration bindings. Those derived fields are never written
back into the editable custom definition.

The first user-created profile assignment is `copilot`, created when
duplicating the built-in Copilot definition (and carried forward when that
custom tool is duplicated). It gives `copilot-local` the same supported
Copilot event behavior while keeping its harness ID, launch command, provider
ID, and settings independent. A custom definition created from scratch has no
profile. Before implementation, the authoritative MVP/ADR language that says
custom definitions cannot select compiled bindings is clarified to distinguish
this server-created profile metadata from arbitrary user-authored selection;
the prohibition on arbitrary selection remains in force.

The effective configuration view shows the profile and its derived bindings
separately from the editable definition, for example:

~~~text
Compatibility profile: copilot (read-only)
Derived integration: copilot adapter (read-only)
Derived session signals: copilot (read-only)
~~~

### Harnesses, integrations, and providers are separate

These identities must not be conflated:

| Concern | Example |
| --- | --- |
| Harness/session launch | copilot-local |
| Workspace integration adapter | copilot |
| Peon model provider | copilot-local |
| Shipped default definition | copilot |

The custom harness gets its own session and Peon-provider identity. A copied
compatibility profile points to the existing, closed Copilot adapter. A custom
harness without a profile has no native signal or hook lifecycle.

Harness-backed providers continue to be projected from the resolved harness
registry. Therefore a copied harness with a peon capability becomes an
independent provider entry with its own model selection and capacity state,
even when it shares an integration adapter with another harness.

## Integration and hook lifecycle

An integration adapter is identified by a code-owned `adapterId` and
`targetId`. The target resolves to a workspace path inside the adapter; users
cannot supply a path. The adapter owns the supported hook schema, marker,
reporter behavior, executable assets, and workspace mutation rules. The
harness record only contributes a compatibility profile and never owns the
installed hook file.

The desired integration state is a map of `{ adapterId, targetId }` keys to
the active harness IDs that consume each key. Status and mutations are
computed once per key, then projected back onto the harness rows. This makes
shared ownership explicit and avoids treating a duplicate as a separate hook
installation.

For each save of active coding tools, OrkWorks:

1. Captures the active harness IDs and resolves them against one registry
   snapshot. The request includes the workspace active-selection revision and
   harness document revision; either mismatch aborts before hook mutation. A
   missing or retired ID is rejected and cannot be silently enabled.
2. Groups the selected harnesses by their resolved adapter and target.
3. Reads status once per adapter/target key and plans idempotent installs,
   repairs, or uninstalls. A status read or mutation never runs once per row.
4. Shows one confirmation per adapter/target group, including the resolved
   workspace path and all consuming harness names.
5. Publishes the requested active-harness selection, then executes the
   confirmed integration plan through Electron main and the sidecar.
6. Returns an outcome for every key and maps it to every consuming row.

Active-harness persistence and workspace hook mutation are not one atomic
transaction: hook files are external workspace state. If one mutation fails,
the selected active IDs remain the user's choice, the affected rows show an
action-needed error, and the next Save retries idempotently. A failed
uninstall is reported as leftover integration state; it is not hidden by
removing the row. A workspace identity change aborts the remaining plan and
requires a reload before retrying.

The resulting behavior includes:

- If at least one active harness uses the Copilot adapter, the shared Copilot
  hook remains installed.
- Disabling normal Copilot while copilot-local remains active leaves the
  Copilot hook installed.
- Disabling copilot-local while normal Copilot remains active also leaves it
  installed.
- The hook is eligible for removal only when no active harness uses the
  adapter.
- A shared install, repair, or uninstall operation is confirmed once per
  adapter/target key, not once per harness row.

Every harness row that references a profile shows the same underlying status,
with a clarification such as “Used by Copilot Local” or “Shared with GitHub
Copilot CLI.” The integration Settings surface also shows the adapter/target
row and its consumer list. Removing a profile is an explicit action; if the
custom harness is active, the UI explains that native signals and hook
behavior will stop after reconciliation. Profile removal publishes the
profile change even if an external hook uninstall fails; the leftover hook is
reported as cleanup-needed and retried through the normal integration flow.

Declaring a profile does not prove that a renamed or wrapped executable
supports the adapter's protocol. The editor therefore warns that the custom
command must remain compatible with the selected profile. The existing
explicit integration confirmation continues to name the workspace file and
warn when OrkWorks-owned executable hook code will be installed.

### Deletion and stale references

Custom definitions are global, while active harness IDs are stored per
workspace. To avoid deleting a tool out from under the current workspace:

- Delete is rejected when the custom ID is active in the current workspace;
  the error tells the user to disable it and save first.
- A delete is allowed when the custom ID is inactive in the current workspace.
  Other workspaces may temporarily contain a stale active ID because they are
  not concurrently loaded by this sidecar.
- On every workspace load and active-harness save, missing custom IDs are
  normalized out of the active set before integration reconciliation. Any
  resulting unreferenced owned adapter is then eligible for uninstall.
- Historical sessions are not rewritten. They retain their harness ID and
  any captured display snapshot; a deleted definition cannot be used for new
  sessions.
- The delete response reports global definition deletion only. It does not
  claim that other workspaces were cleaned; the next load/save in those
  workspaces reports stale-ID removal and any resulting integration cleanup.

## Settings experience

The existing Coding tools section remains the primary view. Its current
enable/disable toggles, detection status, command-path controls, integration
status, confirmation flow, and Save action remain available.

Each harness row additionally exposes:

- View config for the effective resolved definition.
- Duplicate for built-ins and custom harnesses. The sidecar performs the
  duplication so any compatibility profile is assigned by the allowlist.
- Edit override and Reset to default for built-ins.
- Edit JSON and Delete for custom harnesses.
- Remove compatibility profile for custom harnesses that no longer need the
  source tool's native signals or integration.

The JSON editor is an in-place detail view within the same Settings section.
It does not replace the coding-tool list or remove its lifecycle controls.
A clear Back action returns to the list.

The editor identifies the configuration mode:

- Override JSON for a built-in sparse patch.
- Configuration JSON for a complete custom definition.

For both modes, a read-only effective-configuration preview shows the result
after built-ins, overrides, and custom values are resolved. The preview also
identifies inherited fields, the compatibility profile, and any derived
integration adapter. Code-owned profile and binding fields are visibly
read-only rather than appearing to be editable JSON.

The explanatory copy is part of the feature, not an incidental tooltip:

- Built-in override: “Only these fields are customized. Unspecified fields
  continue using the built-in defaults. Future built-in improvements will
  apply automatically.”
- Custom duplicate: “This is an independent copy. Future changes to the
  source harness will not modify it.”
- Shared integration: “This tool uses the Copilot integration. The shared
  hook remains installed while any active compatible tool uses it.”
- Compatibility profile: “This profile is code-owned. It supplies the
  supported Copilot signals and integration; the command and harness settings
  remain independently editable.”

Duplicate starts from the resolved source definition, proposes a unique
slugged ID and name, and asks the sidecar for a duplicate snapshot. The
complete editable JSON then opens for review. On Save, the sidecar creates the
custom record from that definition and derives the profile from the source
ID; the renderer never submits a profile value. The duplicate operation does
not install hooks or persist a record until the user saves.

The existing command-path control remains available for built-ins and custom
rows. For a custom row it edits the custom launch command through the same
complete-definition editor or a field-scoped read-modify-write operation; it
must not send a built-in patch or replace unrelated custom fields. A command
edit uses the current document revision and reports a conflict instead of
silently clobbering a concurrent JSON edit.

## JSON validation and persistence

The editor uses a versioned, strict JSON schema without comments or trailing
commas. Renderer and sidecar validation use the same canonical schema and
conformance fixtures; the sidecar remains authoritative. The accepted
document rules are explicit:

- The root is an object. Unknown keys are rejected rather than ignored.
- Duplicate object keys are rejected. The client and sidecar must not rely on
  last-key-wins parser behavior.
- A complete custom definition requires `id`, `name`, and `launch`; optional
  capability fields use `null` only where the schema declares absence.
- A sparse built-in patch distinguishes omission (preserve the current value)
  from `null` (remove an optional capability). Arrays replace; nested objects
  merge according to the documented patch schema; changing a tagged capability
  kind replaces that capability.
- IDs use the existing lowercase kebab-case grammar, must not collide with a
  built-in or another custom ID, and are immutable after creation.
- Command templates may contain only `{model}`, `{cwd}`, `{repoRoot}`, and
  `{harnessSessionId}`. Braces must form one of those complete tokens; unknown
  or malformed placeholders are errors.
- `integration`, `sessionSignals`, compatibility profiles, hook paths,
  reporter commands, and executable assets are outside the user-editable
  schema. Attempts to submit them are field-specific validation errors.
- Capability combinations are validated together, including model argument
  requirements, Peon command requirements, resume templates, and profile
  compatibility.
- The sidecar rejects a request larger than 256 KiB before parsing the
  document. Diagnostics include a JSON field path and stable error code when
  applicable.

Client-side validation provides fast feedback while typing and reports parse
errors with line and column information. It may additionally warn about an
undetected executable, but it must not downgrade a sidecar validation error.

An unavailable executable is a warning rather than a save failure. This lets
users configure a command before installing it or while using an environment
where detection is temporarily unavailable.

The sidecar is authoritative. It revalidates the complete request or sparse
patch, resolves it against the current registry, and rejects invalid
definitions before publishing them. Writes use the existing revision-aware
atomic persistence path plus an explicit document revision protocol:

- `GET /harnesses` returns an opaque `documentRevision` alongside resolved
  harnesses, origins, stored override metadata, and read-only profiles.
- Create, duplicate, replace, built-in override, reset, and delete requests
  all include the revision the client read as `expectedRevision`.
- The sidecar compares that revision with the current document under the same
  write lock used for the mutation. A mismatch returns HTTP 409 with code
  `harness_config_revision_changed`, the current revision, and no mutation.
- Successful mutations return the new revision. The renderer refreshes its
  snapshot before retrying and never merges a stale draft automatically.
- Field-scoped controls such as command-path editing use the same
  read-modify-write protocol and preserve unrelated fields.

This protocol prevents two Settings windows or a Settings window and a CLI
from silently overwriting one another. An atomic write failure leaves the
previous document and revision published.

The sidecar response used by Settings must include enough metadata to
distinguish the resolved definition from its origin and stored override. The
resolved definition remains the source of truth for launch and runtime
consumers; the stored sparse patch is exposed only for the built-in override
editor.

The conceptual harness API is:

- `GET /harnesses` — list resolved harnesses plus `documentRevision`, origin,
  stored override, and read-only compatibility metadata.
- `POST /harnesses` — create a complete custom definition with
  `expectedRevision` and no profile, or with a server-validated
  `duplicateSourceId` from the duplicate preview.
- `POST /harnesses/:sourceId/duplicate` — return a resolved independent
  snapshot, proposed ID/name, and the document revision to use for the final
  create; it does not persist or install anything.
- `PUT /harnesses/:id` — replace a custom definition or update a built-in
  sparse patch with `expectedRevision`; profile metadata is preserved and not
  accepted from the JSON body.
- `POST /harnesses/:id/remove-profile` — explicitly remove a custom profile,
  reconciling active integrations before reporting completion.
- `DELETE /harnesses/:id` — delete a custom definition or built-in override
  with `expectedRevision`, subject to the active-workspace deletion rules.

No renderer process receives authority to mutate workspace hook files.
Electron main continues to mediate hook-mutating operations and confirmation,
while the sidecar owns registry validation, persistence, and integration
mutation execution.

## Future extensibility

### New declarative harnesses

A new command-based harness that fits the existing capability variants can be
added as a custom definition without a Rust adapter. Its definition can
declare launch arguments, model handling, resume templates, Peon invocation,
capacity patterns, label resets, and other existing declarative capabilities.
It has no native signal or workspace integration unless a reviewed duplicate
operation assigns an existing allowlisted compatibility profile.

A future first-class built-in tool is a new embedded definition plus focused
tests. Built-in consumers continue to read the same resolved registry.

### New protocol-specific behavior

If a tool uses a hook schema, session protocol, or capability not represented
by the existing closed variants, it requires a reviewed compiled adapter.
Only a server-created, allowlisted compatibility profile may reference an
existing adapter. JSON configuration cannot introduce executable integration
code or authority-bearing hook behavior, and cannot select an adapter directly.

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

Provider lifecycle is nevertheless part of this feature. A projected provider
uses the immutable harness ID as its provider ID, so renaming a custom harness
does not lose settings. When the resolved registry gains a custom harness with
a Peon capability, provider settings normalization adds one entry with the
same defaults used for a built-in harness: enabled by the existing default
policy, fallback order after existing entries, no selected model override,
and no capacity override. The new provider appears in Model providers
Settings with its origin and harness name.

When a custom harness is edited, its provider ID and existing enabled state,
fallback order, selected model, and capacity state are preserved. Model
listing and launch/Peon command changes are reflected after the next
verification or refresh. Removing the Peon capability removes the projected
provider after an explicit warning; any current provider selection is cleared
and the UI reports that the provider became unavailable rather than silently
redirecting to another provider. Deleting a custom harness removes its
projected provider and associated provider-setting entry, while historical
session records retain their captured provider ID and display it as historical
if the definition is gone.

Provider normalization is deterministic and tested for add, edit, capability
removal, delete, and restart. It does not infer that two harnesses sharing an
integration profile share provider state.

### Compatibility matrix

The sidecar must make the relationship between editable capabilities and
code-owned behavior explicit:

| Profile | Native session signals | Workspace integration | Editable command/config behavior |
| --- | --- | --- | --- |
| none | none | none | Launch, resume, models, Peon, capacity, voice, and other supported declarative capabilities |
| `copilot` | Copilot contract | Copilot adapter and its code-owned target | Command and declarative capabilities are independent; compatibility is warned, not assumed |
| future allowlisted profile | Profile-specific compiled contract | Profile-specific adapter/target | Only the profile's documented declarative fields are editable |

The Copilot profile routes session events using the custom harness identity;
sharing the adapter does not merge the `copilot` and `copilot-local` sessions
or providers. The profile assignment is hard-checked when created by the
Copilot duplicate operation; later edits that make the command or capabilities
look unlike the source produce a compatibility warning, not an implicit
profile change. Removing or changing a profile is never an incidental
consequence of editing JSON.

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
- a custom definition that is active in the current workspace and therefore
  cannot be deleted
- a partial adapter plan failure with the affected adapter/target and consumer
  harnesses named explicitly
- a stale custom harness ID removed during workspace normalization

Saving a definition and changing active tools are separate operations. A
successful definition save does not silently enable the tool or install its
integration. The existing explicit Save tools flow and confirmation remain
the point where active-tool and shared-hook mutations occur.

## Testing strategy

### Sidecar and registry

- Resolve built-in, overridden, and custom definitions with correct origins.
- Duplicate Copilot through the sidecar into copilot-local as a complete
  independent snapshot with a read-only `copilot` profile.
- Create a custom definition from scratch with no profile or compiled binding.
- Reject custom JSON and built-in patches that attempt to select
  `integration`, `sessionSignals`, hook paths, or reporter commands.
- Reject reserved-ID collisions and malformed custom definitions.
- Preserve unspecified built-in fields when applying sparse overrides.
- Reset an override and recover the shipped definition.
- Project a custom harness Peon capability into an independent provider entry.
- Preserve independent model and capacity state for copilot and copilot-local.
- Preserve provider settings across custom edits and renames; add and remove
  projected provider entries deterministically.
- Group integration status and mutations by adapter and target, with explicit
  consumer harness IDs.
- Keep the shared Copilot hook when only the child is active.
- Remove it only after the last active compatible harness is disabled.
- Reject incompatible profile assignments and capability shapes.
- Exercise missing-ID normalization, active-delete rejection, historical
  session retention, and deferred integration cleanup.
- Exercise duplicate-key, unknown-field, placeholder, size-limit, and
  field-path validation errors.
- Exercise create/update/delete revision conflicts, including two concurrent
  read-modify-write clients, and atomic write failures.
- Exercise partial shared-adapter failures, retry behavior, and workspace
  identity changes during a plan.

### Desktop and Settings

- Preserve existing active-tool toggles, detection, command-path controls,
  hook confirmation, and Save behavior.
- Display the correct origin badge and editor mode.
- Keep the effective preview synchronized with a valid draft.
- Block saving invalid JSON while preserving the draft and diagnostics.
- Show an unavailable command as a warning.
- Explain copied integration behavior and independent custom ownership.
- Show read-only compatibility profile and derived binding metadata.
- Keep custom command-path edits from replacing unrelated JSON fields.
- Explain active-delete rejection, stale-ID cleanup, and partial integration
  outcomes.
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
- Existing built-in profile bindings remain code-owned and unaffected by
  editable JSON patches.
