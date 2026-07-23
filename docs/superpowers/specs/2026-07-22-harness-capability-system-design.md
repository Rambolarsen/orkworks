# Harness Capability System

**Date:** 2026-07-22
**Status:** Approved in brainstorming; revised after written-spec review

## Goal

Make coding-tool integrations easier to extend without adding a runtime plugin
platform. A straightforward coding tool should require one declarative
definition. Tool-specific code should be limited to capabilities that cannot be
expressed safely as data.

Migrate every current built-in to the new model and give each coding tool a
consistent, workspace-scoped integration lifecycle: status, install, and
uninstall. Tools without a safe integration mechanism report that explicitly.

## Current problem

The current system is split across several partially overlapping models:

- `HarnessConfig` owns the live launch command, user-visible identity, voice
  metadata, and optional Peon provider configuration.
- `HarnessAdapter` owns resume templates, capability booleans, and usage-limit
  patterns.
- `resolve_session_launch()` builds commands directly from `HarnessConfig`, so
  the launch template in `HarnessAdapter` is dead and has already drifted.
- Built-in attention and session-ID integrations are implemented through
  one-off code paths rather than a shared lifecycle.
- Disk entries replace complete built-in `HarnessConfig` values, so omitted
  fields silently fall back to Rust defaults instead of preserving the built-in
  value.
- Boolean capability declarations can claim support without identifying the
  implementation that provides it.

The July 3 harness-registry unification correctly co-located Peon provider
configuration with harness instances. This design preserves that result while
replacing the remaining split between live config and code-only adapters.

## Scope

In scope:

- One resolved harness definition used by launch, resume, model discovery,
  Peon, capacity detection, session signals, voice projection, and workspace
  integration management.
- Declarative built-ins embedded in the sidecar binary.
- Sparse user overrides and declarative custom harnesses.
- Small, compiled Rust handlers for tool-specific capabilities.
- Workspace-scoped status/install/uninstall for supported integrations. Install
  is also the idempotent reconcile operation for drifted registrations.
- Migration of Claude Code, OpenCode, Codex, Gemini CLI, Aider, GitHub Copilot
  CLI, and generic shell in one change. The interactive `copilot` CLI replaces
  the legacy one-shot `gh copilot suggest` built-in.
- Migration of legacy `~/.orkworks/harnesses.json` data.
- Replacement of the Claude-specific hook Settings flow and HTTP routes with a
  generic harness-integration flow.

Out of scope:

- Loading native libraries, WebAssembly modules, or third-party executable
  plugins at runtime.
- Downloading integration code from a marketplace.
- Arbitrary user-supplied hook commands. User definitions may still configure
  the coding-tool launch command, as they can today.
- Installing or uninstalling coding tools themselves.
- User-wide integration installation.
- Inventing signals, resume behavior, or hook support that a coding tool does
  not document.
- Changing OrkWorks' observer-first control boundary.

## Decision

Use a declarative harness definition composed from typed capability bindings.
Shared behavior is implemented by built-in declarative capability kinds.
Behavior that must interpret a tool-specific protocol is implemented by a
small compiled handler selected by a stable ID.

The sidecar builds one resolved registry and all consumers use it. Runtime
services must not switch on harness IDs such as `claude-code` or `gemini`.

## Architecture

```text
embedded built-in definitions     user overrides/custom definitions
             |                                  |
             +------------ merge ---------------+
                              |
                     validate and resolve
                              |
                  ResolvedHarnessRegistry
                    /    /    |    \     \
              launch resume Peon capacity integration
                              |
                  compiled handler registry
```

### Harness definition

The contract is:

```rust
struct HarnessDefinition {
    id: String,
    name: String,
    launch: LaunchCapability,
    default_model: Option<String>,
    resume: Option<ResumeCapability>,
    models: Option<ModelCapability>,
    peon: Option<PeonCapability>,
    capacity: Option<CapacityCapability>,
    session_signals: Option<SessionSignalBinding>,
    integration: Option<IntegrationBinding>,
    voice: Option<VoiceCapability>,
}
```

Capability presence means support. Capability absence means unsupported. The
stored definition does not contain independent `supports_*` booleans.

Declarative capability kinds are closed, serde-tagged enums. Examples include:

- `command-template` for launch and resume commands;
- `static` or `command` for model discovery;
- `terminal-patterns` for capacity detection;
- the existing declarative Peon invocation configuration;
- declarative native-voice pass-through metadata.

Only capabilities that need tool-specific protocol behavior use a compiled
binding. These bindings are closed, serde-tagged Rust enums rather than generic
handler-name strings. Each variant owns its validated data and declares the
exact normalized signal kinds it can emit: native session ID, attention,
lifecycle, model, and context usage. Integration variants likewise own their
reporter choice and authority-bearing paths. User configuration cannot select
an arbitrary handler, reporter, or filesystem location, and a custom harness
may use a compiled variant only when that variant's code-owned compatibility
predicate accepts the harness definition.

Built-in definitions live in one versioned JSON resource compiled into the
sidecar with `include_str!`. This keeps adding a simple built-in to one data
entry while retaining a single distributable binary. A test parses and
validates the resource before release.

### Capability handlers

Do not introduce one large adapter trait. Use narrow contracts so a coding tool
implements only the exceptional behavior it needs:

- **Launch** builds the initial `CommandSpec`.
- **Resume** reports available strategies and builds the selected command.
- **Models** returns configured or discovered models.
- **Peon** describes headless inference invocation.
- **Capacity** classifies supported terminal or probe signals.
- **Session signals** translates a tool event into normalized OrkWorks metadata.
- **Workspace integration** implements status, idempotent install/reconcile,
  and uninstall.
- **Voice** projects pass-through capability metadata and never handles audio.

Shared declarative variants do not require registry lookups. Tool-specific
session-signal and integration handlers are exhaustive matches over the closed
binding enums. This prevents a signal handler from being used as an installer
and prevents definitions from claiming signal kinds that the compiled
implementation does not expose.

### Resolved registry

Startup constructs an immutable `ResolvedHarnessRegistry`:

1. Parse the embedded built-ins.
2. Load the versioned user configuration.
3. Apply sparse overrides to matching built-ins.
4. Append complete custom definitions.
5. Resolve named handler bindings.
6. Validate commands, placeholders, capability combinations, and handler
   configuration.
7. Publish the new registry atomically.

Harness CRUD rebuilds a candidate registry and swaps it into application state
only after the complete candidate validates. Provider/Peon definitions are
derived from the same snapshot, avoiding stale provider state after a harness
edit.

Durable CRUD is one transaction under a configuration lock and revision
check: load the latest supported v1 or v2 bytes, migrate v1 in memory when
needed, build and validate the complete candidate, write that exact v2 document
through same-directory temporary-file replacement, and only then publish the
persisted registry snapshot and its derived providers. A write or replacement
failure leaves both the live registry and the previous file unchanged. The
save path returns errors; it never treats a failed durable write as success.

An invalid custom definition is excluded and reported. An invalid override
does not disable the corresponding built-in; OrkWorks retains the valid
built-in and exposes the rejected override as a diagnostic.

## User configuration

Replace the legacy top-level array with a versioned document:

```json
{
  "version": 2,
  "overrides": {
    "opencode": {
      "launch": {
        "command": "/opt/opencode"
      }
    }
  },
  "custom": [
    {
      "id": "company-cli",
      "name": "Company CLI",
      "launch": {
        "kind": "command-template",
        "command": "company-ai",
        "args": ["--model", "{model}"]
      }
    }
  ]
}
```

Overrides use dedicated patch structs; they are not deserialized as complete
definitions with defaults. Omitted fields preserve the built-in value. `null`
is accepted only for optional capabilities and explicitly removes that
capability.

Patch semantics are deterministic:

- the map key is the immutable built-in ID; an override cannot change it;
- scalar and object fields merge recursively field by field;
- arrays replace the complete built-in array rather than append or merge;
- changing a tagged enum's `kind` replaces that complete capability value and
  requires all fields for the new variant;
- `null` is legal only at an optional capability boundary, never for required
  fields or nested scalar values;
- custom IDs must be unique and must not collide with a built-in ID, override
  key, compatibility alias, or another custom definition.

Custom definitions may use declarative capability kinds. They cannot opt into
compiled signal or integration bindings unless the binding variant explicitly
permits custom definitions and its code-owned compatibility check succeeds.
Authority-bearing built-in bindings do not permit this. Custom definitions
cannot supply executable integration code, integration paths, or an arbitrary
reporter command. Round-trip fixtures cover every merge rule and collision.

## Runtime data flow

### Launch and resume

`POST /sessions` resolves the requested harness instance once, then asks its
launch capability to build the command. The dead `HarnessAdapter` launch path
and the direct `HarnessConfig` launch renderer are removed.

Resume reads the same definition, derives available resume strategies from the
configured resume capability and captured memory, and builds exactly the
strategy selected by the request. No fallback is advertised unless it is
present in the definition and documented for that tool.

### Peon, models, and capacity

Peon provider definitions, model discovery, and capacity detection are derived
from the resolved harness snapshot. A harness without Peon configuration is
not an inference provider. A harness without capacity configuration does not
receive capacity classification from an unrelated tool's patterns.

### Public projection

The renderer receives a public harness descriptor containing:

- ID and display name;
- default model and model choices where available;
- derived capability names;
- integration support, coverage, tool detection, registration, ownership, and
  effective activation;
- user-actionable validation diagnostics;
- whether the definition is built-in or custom.

Internal commands, absolute filesystem paths, handler IDs, and handler
configuration remain sidecar-only.

## Workspace integration lifecycle

Every resolved harness exposes the same read-only status contract. Mutation is
available only when an integration handler exists. Status keeps independent
axes so registration is never presented as proof that the tool is installed or
that the integration will execute.

```rust
enum IntegrationRegistration {
    Unsupported,
    Absent,
    Installed,
    Drifted,
    Error,
}

enum IntegrationOwnership {
    None,
    OrkWorks,
    Ambiguous,
}

enum IntegrationActivation {
    Active,
    NeedsTrust,
    Disabled,
    Unknown,
    NotApplicable,
}

enum IntegrationCoverage {
    Full,
    Limited,
    None,
}
```

The status also reports `enabled` from OrkWorks workspace metadata and
`tool_detected` from executable/version probing. `Drifted` means an
OrkWorks-owned marker or file exists but no longer matches a complete, valid
registration. `Error` means configuration could not be read, parsed, validated,
or safely modified. `Activation` describes whether the coding tool will
currently execute the registration, including trust and tool-level disablement;
unknown must remain unknown. Typed diagnostics explain the safe next action.
Coverage is separate from all of these axes: Aider can be registered and
enabled with limited notification coverage, while generic shell has no
integration and therefore no coverage.

`Full` means the handler has verified contracts for that tool's selected
OrkWorks signal set: native session ID when exposed, attention-set,
attention-clear, and lifecycle end when exposed. It does not imply support for
every upstream hook event. `Limited` means at least one useful selected signal
is deterministic but one or more selected signals are unavailable or
unverified. `None` means no deterministic integration signal is available.

The generic routes are:

```text
GET    /workspace/harness-integrations/:harnessId
POST   /workspace/harness-integrations/:harnessId
DELETE /workspace/harness-integrations/:harnessId
```

The renderer submits only the harness ID. The current workspace, configuration
paths, reporter assets, and expected registration are resolved in the sidecar.
The POST operation installs an absent integration or repairs a drifted one.
POST and DELETE require the per-sidecar secret already established by ADR 0025,
or a separately derived mutation-scoped token. The token remains in Electron
main: preload exposes only a harness-ID mutation *intent*, not a direct sidecar
mutation. Electron main obtains the sanitized confirmation descriptor, owns and
displays the native confirmation, and attaches mutation authority only after
the user accepts. Cancellation never calls the route. The renderer never
receives the token, and terminal child processes never inherit it. A compromised
renderer may trigger a confirmation prompt but cannot approve or silently
perform the write. Direct unauthenticated or incorrectly authenticated mutation
requests are rejected. GET remains read-only and does not require mutation
authority.

### Installation guarantees

- Installation and removal are explicit user actions.
- The confirmation names the coding tool, workspace scope, signal coverage,
  and repo-relative configuration locations that will change.
- Status checks never write.
- Install reads the latest configuration, parses and validates it, merges an
  entry with a stable OrkWorks marker, and writes through temporary-file
  replacement.
- Before any read or write, the handler canonicalizes the workspace and the
  target's nearest existing ancestor. It rejects symlink, junction, or reparse
  point traversal that escapes the workspace. The temporary file is created in
  the validated target directory, and containment plus file type are
  revalidated immediately before replacement. A workspace change invalidates
  the operation rather than redirecting it.
- OrkWorks prefers a tool's documented local-only workspace file. It never
  silently edits a tracked or otherwise shareable project file. When a tool has
  no local-only mechanism, installation is allowed only into a dedicated,
  already ignored and untracked OrkWorks-owned file after the confirmation
  explicitly warns that executable integration code is being added. If those
  conditions cannot be proved, status is limited or unsupported. OrkWorks does
  not edit `.gitignore` on the user's behalf.
- Repeated install is idempotent.
- Reporter assets are copied to stable paths under
  `~/.orkworks/hook-scripts/` before registration. Packaged or development
  source paths are never installed.
- A reporter copied before a failed registration is harmless and may remain.

### Uninstallation guarantees

- Uninstall removes only OrkWorks-owned entries or dedicated files.
- Unrelated user and coding-tool configuration is preserved.
- Parent directories and shared reporter assets are not deleted.
- Repeated uninstall is idempotent.
- If ownership is ambiguous, the handler reports drift/error and does not
  remove data silently.
- Shared reporter assets remain because another workspace may still use them.

### Built-in integration mapping

The implementation uses the current documented native extension point for each
tool, subject to the local-only/ignored-file policy above:

| Harness | Workspace mechanism | Coverage |
| --- | --- | --- |
| Claude Code | Owned entries in `.claude/settings.local.json` calling the stable reporter | Full for the verified selected signal contract |
| Codex | Owned project lifecycle hooks beside the trusted `.codex` config layer, only in a proven local-only or already ignored dedicated file | Full for verified documented hook events; activation may require trust |
| OpenCode | One owned workspace plugin in `.opencode/plugins/`, only when the dedicated file is already ignored and untracked | Limited until the required event payload schema is pinned to a primary type definition |
| Gemini CLI | Owned entries in workspace `.gemini/settings.json` only when the selected file is documented local-only or is already ignored and untracked | Full only when the supported-version and verified-payload gates pass; otherwise limited/unknown |
| GitHub Copilot CLI | Owned entries in `.github/copilot/settings.local.json` | Full for the verified selected lifecycle/attention/session-ID contract |
| Aider | OrkWorks-managed `--notifications-command` launch augmentation | Limited to ready-for-input notification |
| Generic shell | No integration handler | None / unsupported |

The Codex hook system supports user and project layers; this design considers
only the project layer because the user selected workspace-only scope, but it
still enforces the non-shared-file policy and reports `NeedsTrust` until the
exact hook hash is trusted. OpenCode and Gemini project mechanisms can contain
executable or shared configuration and are not assumed personal merely because
they are workspace-scoped. Copilot's documented local settings file satisfies
the local-only policy. Aider does not expose a general lifecycle hook API, so
OrkWorks labels its notification bridge limited. Installing the Aider bridge
persists an enabled flag in OrkWorks workspace metadata; launch then adds the
documented `--notifications-command` argument. It does not write an Aider
repository configuration file.

## User experience

Replace the Claude-only hook area in Settings with an **Integrations** row for
each active coding tool. The row renders the independent detection,
registration, ownership, activation, and coverage axes rather than collapsing
them into one badge:

- **Available** (supported + absent) — shows **Install for this workspace**.
- **Installed + active** — shows signal coverage and **Uninstall**.
- **Installed + needs trust/disabled/unknown** — names the prerequisite and
  does not claim that signals are working.
- **Drifted** — explains the mismatch and shows **Repair** or **Uninstall**
  when ownership is still unambiguous.
- **Error** — shows a concise, non-destructive error and no unsafe action.
- **Unsupported** — remains visible with an explanation; no action button.
- **Limited** coverage — shown alongside the state, not disguised as full
  integration.

Install, repair, and uninstall each require confirmation. Settings refreshes
status after every operation and does not optimistically claim success.

## Failure handling and safety

- Unknown harness IDs return not found.
- Unsupported integration mutation returns conflict.
- Missing or incorrect mutation authority returns unauthorized before handler
  lookup or filesystem access.
- Invalid definitions and bindings return structured validation diagnostics.
- Tool configuration is parsed before any write. Invalid existing config is
  never overwritten with a fresh file.
- Installers re-read immediately before merging rather than writing a stale
  copy from an earlier status check.
- Each shared config file is updated under a sidecar-owned per-path lock.
- External-edit protection is best-effort optimistic concurrency: capture a
  revision/hash, merge from the latest read, perform a final hash/stat check,
  keep a recoverable backup where the target format permits it, and atomically
  replace. A sidecar lock coordinates OrkWorks writers but cannot eliminate the
  residual cross-process race between the final check and rename on every
  supported platform. A detected change aborts and reports drift; the UI and
  docs do not promise platform-independent compare-and-swap semantics.
- Canonical containment and no-follow validation is repeated immediately
  before every replacement; symlinks, Windows junctions/reparse points, and
  workspace switches cannot redirect a mutation outside the selected
  workspace.
- Dedicated integration files contain an OrkWorks ownership marker and schema
  version.
- Reporter payloads continue to use `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT`;
  integrations do not type into the coding-tool terminal.
- Reporter processes and coding-tool terminals never receive the sidecar-scoped
  mutation secret. Only Electron main uses it to authorize explicit lifecycle
  mutations.

## Legacy migration

Legacy `~/.orkworks/harnesses.json` arrays remain readable.

- The binary embeds fingerprints/fixtures for every built-in snapshot shipped
  by a migratable schema version. A legacy entry matching a known stock
  snapshot becomes no override, so it inherits the new built-in. Differences
  from that matching historical snapshot become a sparse override. If no known
  snapshot matches, migration conservatively freezes the entry's explicit
  values as an override and emits a diagnostic rather than guessing which
  fields were customized.
- A legacy custom entry is converted to a complete custom definition.
- The legacy built-in ID `gh-copilot` migrates to `copilot` across one
  revision-checked transaction covering harness config, workspace active
  selection, and provider preferences. A read-only alias keeps historical
  session metadata displayable without rewriting session files.
- An exact known stock `gh-copilot` snapshot becomes the new interactive
  `copilot` built-in with no command override. A customized legacy entry is
  preserved as an explicit override only when it is compatible with the
  interactive CLI; otherwise it becomes a uniquely named custom definition
  retaining its original `gh copilot suggest` command and receives an
  actionable diagnostic. Existing `copilot` custom-ID collisions abort that
  entry's migration rather than overwrite it.
- The legacy `harness` adapter reference is translated into explicit
  declarative capability bindings matching its current behavior.
- Except for the intentionally replaced command in a recognized stock
  `gh-copilot` snapshot, customized launch command, arguments, model prefix,
  default model, voice metadata, attention metadata, Peon configuration, and
  usage-limit patterns are preserved.
- The next successful harness save writes version 2. Migration never rewrites
  the file merely because OrkWorks started.
- A multi-file preference migration records a migration revision and is
  restartable: already-applied components are recognized, conflicting partial
  state produces a diagnostic, and no component is published live until its
  durable write succeeds. Fixtures cover stock, customized, conflicting,
  malformed, partially applied, and interrupted states.
- Invalid legacy entries produce per-entry diagnostics and do not prevent valid
  built-ins from loading.

Session metadata continues storing the selected harness instance ID, so session
and event files require no migration.

The new Claude integration handler recognizes the existing OrkWorks
Notification-hook marker and stable reporter path. An existing installation
therefore appears installed and can be repaired or removed through the generic
flow.

## Signal contract and version gate

Every compiled signal binding carries a code-owned contract manifest. For each
supported event it pins the exact upstream event name, configuration fragment,
payload selector and type, normalized metadata write, source, confidence,
clear/staleness rule, activation prerequisite, and minimum supported CLI
version. The version floor must come from an upstream release/tag or a fixture
captured from that version; it is not guessed from the latest documentation.
When the executable is missing, older than the floor, disabled, untrusted, or
has an unverified payload schema, detection/activation says so and coverage is
downgraded. A built-in cannot claim a signal or `Full` coverage without a
passing contract fixture.

The initial normalized mapping is deliberately conservative:

| Harness | Installed config shape | Verified event and payload | Normalized write | Clear/staleness | Activation prerequisite |
| --- | --- | --- | --- | --- | --- |
| Claude Code | Owned command entries in `.claude/settings.local.json` | `SessionStart`, `UserPromptSubmit`, `Notification`, and `SessionEnd`; common `session_id`, with notification subtype and end reason accepted only from the pinned schemas | Session ID to resume memory; verified permission/idle notification to `observed_status = waiting_for_input`, `attention = needs_you`; `SessionEnd` records terminal lifecycle outcome; source `claude_hook`, confidence `1.0` | `UserPromptSubmit` clears attention; event timestamps reject late writes; session end is terminal | Supported version and enabled local hook |
| Codex | Owned command entries in a dedicated eligible `.codex/hooks.json` | `SessionStart`, `UserPromptSubmit`, `PermissionRequest`, and `Stop`; common string `session_id`, exact event-specific fields from the published hook schema | Session ID to resume memory; permission request or completed turn to waiting/needs-you; prompt submit clears it; source `codex_hook`, confidence `1.0` | Clear on `UserPromptSubmit`; stale events older than the stored capture timestamp are ignored | Supported version, trusted project layer, trusted exact hook hash, hook enabled |
| OpenCode | Dedicated owned `.opencode/plugins/orkworks.js` | Event names `session.created`, `session.idle`, `session.error`, `session.status`, `permission.asked`, and `permission.replied` are documented, but payload fields must be pinned to the upstream TypeScript event types before use | Only fields proven by the pinned type fixture are written; until then coverage remains limited and activation `Unknown` | Contract fixture defines ID correlation and clear order; late events cannot overwrite newer metadata | Supported version, eligible ignored/untracked plugin file, verified payload type |
| Gemini CLI | Owned command entries in an eligible `.gemini/settings.json` `hooks` object | `SessionStart`, `BeforeAgent`, `AfterAgent`, `Notification`, and `SessionEnd`; common string `session_id`, `hook_event_name`, `cwd`, and ISO timestamp from the published schema | Session ID to resume memory; `BeforeAgent` clears attention; verified `AfterAgent`/notification subtype sets waiting/needs-you; `SessionEnd` updates lifecycle outcome; source `gemini_hook`, confidence `1.0` | Ignore older timestamps; clear on `BeforeAgent`; session end is terminal | Supported version, trusted workspace where required, hook enabled, eligible local file |
| GitHub Copilot CLI | Owned command entries in `.github/copilot/settings.local.json` | `sessionStart`, `userPromptSubmitted`, `agentStop`, `sessionEnd`; camelCase payload uses string `sessionId` and numeric timestamp, with the documented PascalCase/snake-case form accepted only as a separately tested variant | Session ID to resume memory; prompt submit clears attention; agent stop sets waiting/needs-you; session end records terminal outcome; source `copilot_hook`, confidence `1.0` | Event timestamp ordering; clear on submitted prompt; session end is terminal | Supported interactive `copilot` version and enabled local hook |
| Aider | OrkWorks workspace flag causing `--notifications-command <stable reporter>` launch augmentation | Documented notification command invocation; no native session ID or general lifecycle payload | Ready notification sets waiting/needs-you; source `aider_notification`, confidence `0.9` | Clear on subsequent terminal user input; otherwise expires under the normal attention-staleness policy | Supported notification option and OrkWorks workspace flag enabled |
| Generic shell | None | None | None | Not applicable | Unsupported |

The exact JSON fragments and vendor payload fixtures live beside each compiled
binding and are copied into the authoritative implementation spec/ADR evidence
before code is enabled. Reporter input outside its OrkWorks session environment
is a successful no-op. A payload with a missing/invalid session correlation,
unknown event, wrong type, or stale timestamp is rejected or ignored without
mutating metadata. Existing metadata source precedence remains authoritative;
native hook writes do not allow a late lower-priority event to overwrite newer
user or agent metadata.

## Per-harness adapter notes

These notes preserve current launch/resume behavior. Any expansion of resume or
signal behavior must be rechecked against primary documentation under the
`adding-harness` workflow before implementation.

| Harness ID | Capability/integration IDs | Launch | Exact resume | Latest fallback | Native session ID source | Approval requirement |
| --- | --- | --- | --- | --- | --- | --- |
| `claude-code` | command templates, terminal patterns, `claude-events`, `claude-workspace-hooks` | `claude` | `claude --resume {harnessSessionId}` | `claude --continue` in workspace | Claude hook JSON `session_id`, source `claude_hook`, high confidence | Explicit install/repair/uninstall |
| `opencode` | command templates, terminal patterns, `opencode-events`, `opencode-workspace-plugin` | `opencode [--model <model>]` | `opencode --session {harnessSessionId}` | `opencode --continue` in workspace | `OPENCODE_SESSION_ID`, deterministic source, high confidence; `session.created` remains disabled until its upstream payload type fixture is pinned | Explicit install/repair/uninstall |
| `codex` | command template, terminal patterns, `codex-events`, `codex-workspace-hooks` | `codex` | Not configured in this change | None | documented hook `session_id`, source `codex_hook`, high confidence | Explicit install/repair/uninstall; interactive probes remain user-triggered |
| `gemini` | command template, `gemini-events`, `gemini-workspace-hooks` | `gemini` | Not configured in this change | None | documented hook `session_id`, source `gemini_hook`, high confidence | Explicit install/repair/uninstall |
| `aider` | command template, `aider-notification-bridge` | `aider [--model <model>]` | Not configured | None | None | Explicit limited integration enable/disable |
| `copilot` | command template, `copilot-events`, `copilot-workspace-hooks` | `copilot` | Not configured in this change | None | documented hook `sessionId`/`session_id`, source `copilot_hook`, high confidence | Explicit install/repair/uninstall |
| `generic-shell` | command template only | user's interactive shell | Not configured | None | None | Integration unavailable |

Test updates span `harness.rs`/its replacement modules,
`harness_registry.rs`, provider derivation, session handler launch/resume tests,
new integration-handler fixtures, HTTP route tests, `apps/desktop/tests/api.test.ts`,
`apps/desktop/tests/newSessionDialogState.test.ts`, and new Settings integration
state tests.

## Testing strategy

### Definition and registry tests

- Parse and validate the complete embedded built-in resource.
- Table-test every built-in's expected capability set.
- Reject unknown handler IDs, malformed placeholders, invalid command shapes,
  and contradictory capability configuration.
- Prove sparse overrides preserve every omitted built-in field.
- Prove nested object merge, array replacement, tagged-variant replacement,
  legal/illegal `null`, immutable IDs, and all collision rules round-trip.
- Migrate every known shipped built-in snapshot without freezing stock values;
  unknown historical snapshots freeze conservatively with a diagnostic.
- Prove an invalid override retains the valid built-in with a diagnostic.
- Prove registry replacement is atomic and provider derivation uses the same
  snapshot.
- Inject persistence and replacement failures and prove disk plus live registry
  remain on the same previous snapshot.

### Command and runtime tests

- Pin launch command rendering for every built-in, with and without a model.
- Pin every currently supported exact and latest resume strategy.
- Prove no resume strategy is advertised without configuration and memory.
- Keep Peon provider, model discovery, capacity detection, and usage-limit
  classification tests passing through the resolved registry.

### Integration handler tests

Each supported handler has fixtures for:

- absent configuration;
- unrelated existing configuration;
- already-installed configuration;
- partial or drifted OrkWorks registration;
- malformed existing configuration;
- install twice;
- uninstall twice;
- install followed by uninstall;
- preservation of unrelated configuration;
- refusal to remove ambiguous user-edited data;
- POSIX and Windows command/path rendering where the tool supports both.
- file and directory symlink escapes, Windows junction/reparse-point escapes,
  and a workspace switch immediately before replacement;
- tracked/shareable project config refusal and eligible ignored/untracked
  dedicated-file installation;
- an external edit injected between final revision check and replacement,
  including the documented residual-race behavior;
- tool missing/unsupported-version, registered-but-disabled, needs-trust,
  unknown activation, and owned/ambiguous registration states;
- every exact event/payload fixture, timestamp ordering, signal clear rule,
  unsupported event no-op, and invocation outside an OrkWorks session.

### API and renderer tests

- Generic status/install/uninstall routes cover available, installed, drifted,
  unsupported, limited, and error responses.
- Mutation routes reject missing/incorrect authority before filesystem access;
  GET remains read-only.
- Electron-main confirmation acceptance is the only path that attaches mutation
  authority; cancellation and direct preload/renderer attempts perform no
  filesystem mutation.
- API projections preserve independent enabled, tool-detected, registration,
  ownership, activation/trust, coverage, and diagnostic fields.
- Persistence failure returns an error and never projects an unpersisted state.
- The renderer sends only a harness ID.
- Settings shows the correct action and confirmation for each state.
- UI state refreshes from the returned status after a mutation.
- Existing session, terminal, Peon, capacity, and remembered-session tests
  remain green.

## Documentation and issue tracking

This design changes the harness architecture and expands deterministic
integration installation beyond Claude Code. Before implementation code:

1. Update `specs/orkworks-mvp.md` to define the composable capability model and
   generic, explicit workspace integration lifecycle.
2. Update `specs/native-harness-voice-support.md` only to replace obsolete
   config examples; voice remains pass-through.
3. Add an ADR for the resolved definition plus compiled capability-handler
   boundary and update `docs/adr/README.md`.
4. Create a scoped implementation issue. For #23, #71, #103-#108, #180,
   #187, and #188, explicitly record whether the new issue fully supersedes its
   acceptance criteria or merely links it as remaining follow-up work.
5. Update `docs/agents/architecture.md`, `AGENTS.md`, `README.md`, and the
   `skills/adding-harness/` file-layout guidance when implementation lands.

## Primary integration references

Checked on 2026-07-22:

- [Claude Code hooks reference](https://code.claude.com/docs/en/hooks)
- [Codex hooks](https://learn.chatgpt.com/docs/hooks)
- [OpenCode plugins](https://opencode.ai/docs/plugins/)
- [Gemini CLI hooks reference](https://geminicli.com/docs/hooks/reference/)
- [GitHub Copilot hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference)
- [Aider notifications](https://aider.chat/docs/usage/notifications.html)

## Acceptance criteria

- [ ] Every built-in is represented by one declarative harness definition.
- [ ] The interactive GitHub Copilot CLI replaces `gh copilot suggest`, with a
      `gh-copilot` compatibility migration for persisted configuration and
      historical sessions.
- [ ] Launch, resume, Peon, capacity, signals, and integration consumers use one
      resolved registry.
- [ ] The duplicate `HarnessAdapter` launch path and capability booleans are
      removed.
- [ ] Adding a simple built-in requires one definition entry and tests, with no
      new Rust behavior.
- [ ] Tool-specific behavior is isolated behind narrow compiled handlers.
- [ ] Compiled bindings expose exact signal kinds and code-owned compatibility;
      user definitions cannot select reporters or authority-bearing paths.
- [ ] Sparse overrides preserve omitted built-in fields.
- [ ] Durable harness CRUD publishes only the exact successfully persisted
      registry snapshot.
- [ ] Legacy harness configuration migrates without rewriting on startup.
- [ ] Supported coding tools expose workspace-scoped status/install/uninstall;
      drift can be repaired explicitly.
- [ ] Integration status keeps enabled, tool-detected, registration, ownership,
      activation/trust, and coverage axes independent.
- [ ] Uninstall removes only OrkWorks-owned registration.
- [ ] Aider is labeled limited and generic shell is labeled unsupported.
- [ ] No integration is installed silently or user-wide.
- [ ] POST/DELETE require Electron-main-held mutation authority and reject direct
      unauthenticated requests before filesystem access.
- [ ] Install/uninstall cannot escape the canonical workspace through symlinks,
      junctions, reparse points, or a workspace switch, and never silently
      modifies tracked/shareable project configuration.
- [ ] The renderer never receives integration filesystem paths or arbitrary
      handler configuration.
- [ ] Authoritative specs, ADRs, issues, architecture docs, and adding-harness
      guidance are synchronized before completion.
