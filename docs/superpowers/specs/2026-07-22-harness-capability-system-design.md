# Harness Capability System

**Date:** 2026-07-22
**Status:** Approved in brainstorming; awaiting written-spec review

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
- Workspace-scoped status/install/uninstall/repair for supported integrations.
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
    session_signals: Option<HandlerBinding>,
    integration: Option<HandlerBinding>,
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

Only capabilities that need tool-specific protocol behavior use a
`HandlerBinding`. A binding names a handler compiled into OrkWorks and provides
validated, handler-specific configuration. Unknown handler IDs are invalid.

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
- **Workspace integration** implements status, install, uninstall, and repair.
- **Voice** projects pass-through capability metadata and never handles audio.

Shared declarative variants do not require registry lookups. Tool-specific
session-signal and integration handlers are held in typed registries, so a
signal handler cannot accidentally be used as an installer.

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

Custom definitions may use declarative capability kinds and may select known
compiled handler IDs. They cannot supply executable integration code or an
arbitrary reporter command.

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
- integration support, coverage, and current state;
- user-actionable validation diagnostics;
- whether the definition is built-in or custom.

Internal commands, absolute filesystem paths, handler IDs, and handler
configuration remain sidecar-only.

## Workspace integration lifecycle

Every resolved harness exposes the same read-only status contract. Mutation is
available only when an integration handler exists.

```rust
enum IntegrationState {
    Unsupported,
    Available,
    Installed,
    Drifted,
    Error,
}

enum IntegrationCoverage {
    Full,
    Limited,
    None,
}
```

`Drifted` means an OrkWorks-owned marker or file exists but no longer matches a
complete, valid registration. `Error` means configuration could not be read,
parsed, validated, or safely modified. Coverage is separate from state: Aider
can be installed with limited notification coverage, while generic shell has
no integration and therefore no coverage.

The generic routes are:

```text
GET    /workspace/harness-integrations/:harnessId
POST   /workspace/harness-integrations/:harnessId
DELETE /workspace/harness-integrations/:harnessId
```

The renderer submits only the harness ID. The current workspace, configuration
paths, reporter assets, and expected registration are resolved in the sidecar.
The POST operation installs an absent integration or repairs a drifted one.

### Installation guarantees

- Installation and removal are explicit user actions.
- The confirmation names the coding tool, workspace scope, signal coverage,
  and repo-relative configuration locations that will change.
- Status checks never write.
- Install reads the latest configuration, parses and validates it, merges an
  entry with a stable OrkWorks marker, and writes through temporary-file
  replacement.
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
tool:

| Harness | Workspace mechanism | Coverage |
| --- | --- | --- |
| Claude Code | Owned entries in `.claude/settings.local.json` calling the stable reporter | Full lifecycle/attention and session ID |
| Codex | Owned project lifecycle hooks beside the trusted `.codex` config layer | Full for documented hook events; no undocumented event is inferred |
| OpenCode | One owned workspace plugin subscribing to session events | Full for documented session events |
| Gemini CLI | Owned entries in workspace `.gemini/settings.json` | Full for documented lifecycle, notification, and session ID payloads |
| GitHub Copilot CLI | Owned entries in `.github/copilot/settings.local.json` | Full for documented lifecycle and notification events |
| Aider | OrkWorks-managed `--notifications-command` launch augmentation | Limited to ready-for-input notification |
| Generic shell | No integration handler | None / unsupported |

The Codex hook system supports user and project layers; this design uses the
project layer because the user selected workspace-only scope. OpenCode,
Gemini, and Copilot integrations likewise use their documented project or
workspace extension mechanisms. Aider does not expose a general lifecycle hook
API, so OrkWorks must label its notification bridge as limited. Installing the
Aider bridge persists an enabled flag in OrkWorks workspace metadata; launch
then adds the documented `--notifications-command` argument. It does not write
an Aider repository configuration file.

## User experience

Replace the Claude-only hook area in Settings with an **Integrations** row for
each active coding tool:

- **Available** — shows **Install for this workspace**.
- **Installed** — shows signal coverage and **Uninstall**.
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
- Invalid definitions and bindings return structured validation diagnostics.
- Tool configuration is parsed before any write. Invalid existing config is
  never overwritten with a fresh file.
- Installers re-read immediately before merging rather than writing a stale
  copy from an earlier status check.
- Each shared config file is updated under a sidecar-owned per-path lock.
- A file changing between final validation and atomic replacement aborts the
  operation and reports drift instead of overwriting the external edit.
- Dedicated integration files contain an OrkWorks ownership marker and schema
  version.
- Reporter payloads continue to use `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT`;
  integrations do not type into the coding-tool terminal.
- Workspace integrations never receive the sidecar-scoped secret used by the
  Electron-only plan-opening route.

## Legacy migration

Legacy `~/.orkworks/harnesses.json` arrays remain readable.

- A legacy entry matching a built-in ID is compared field-by-field with the
  current built-in and converted to a sparse override in memory.
- A legacy custom entry is converted to a complete custom definition.
- The legacy built-in ID `gh-copilot` migrates to `copilot` in harness config,
  active-harness selection, and provider preferences. A read-only alias keeps
  historical session metadata displayable without rewriting session files.
- The legacy `harness` adapter reference is translated into explicit
  declarative capability bindings matching its current behavior.
- Launch command, arguments, model prefix, default model, voice metadata,
  attention metadata, Peon configuration, and usage-limit patterns are
  preserved.
- The next successful harness save writes version 2. Migration never rewrites
  the file merely because OrkWorks started.
- Invalid legacy entries produce per-entry diagnostics and do not prevent valid
  built-ins from loading.

Session metadata continues storing the selected harness instance ID, so session
and event files require no migration.

The new Claude integration handler recognizes the existing OrkWorks
Notification-hook marker and stable reporter path. An existing installation
therefore appears installed and can be repaired or removed through the generic
flow.

## Per-harness adapter notes

These notes preserve current launch/resume behavior. Any expansion of resume or
signal behavior must be rechecked against primary documentation under the
`adding-harness` workflow before implementation.

| Harness ID | Capability/integration IDs | Launch | Exact resume | Latest fallback | Native session ID source | Approval requirement |
| --- | --- | --- | --- | --- | --- | --- |
| `claude-code` | command templates, terminal patterns, `claude-events`, `claude-workspace-hooks` | `claude` | `claude --resume {harnessSessionId}` | `claude --continue` in workspace | Claude hook JSON `session_id`, source `claude_hook`, high confidence | Explicit install/repair/uninstall |
| `opencode` | command templates, terminal patterns, `opencode-events`, `opencode-workspace-plugin` | `opencode [--model <model>]` | `opencode --session {harnessSessionId}` | `opencode --continue` in workspace | `OPENCODE_SESSION_ID` or `session.created`, deterministic source, high confidence | Explicit install/repair/uninstall |
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
- Prove an invalid override retains the valid built-in with a diagnostic.
- Prove registry replacement is atomic and provider derivation uses the same
  snapshot.

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

### API and renderer tests

- Generic status/install/uninstall routes cover available, installed, drifted,
  unsupported, limited, and error responses.
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
- [ ] Sparse overrides preserve omitted built-in fields.
- [ ] Legacy harness configuration migrates without rewriting on startup.
- [ ] Supported coding tools expose workspace-scoped status/install/uninstall;
      drift can be repaired explicitly.
- [ ] Uninstall removes only OrkWorks-owned registration.
- [ ] Aider is labeled limited and generic shell is labeled unsupported.
- [ ] No integration is installed silently or user-wide.
- [ ] The renderer never receives integration filesystem paths or arbitrary
      handler configuration.
- [ ] Authoritative specs, ADRs, issues, architecture docs, and adding-harness
      guidance are synchronized before completion.
