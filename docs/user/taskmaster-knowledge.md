# Recommendation knowledge and background analysis

Open **Settings → Recommendations** to choose a Taskmaster model provider and
model. This selection is independent of Peon. Without a configured supported
model, the existing deterministic recommendations remain available.

Background inference supports Ollama, Codex, and Claude Code. CLI profiles require
Codex 0.153.4 or later in the 0.x series, or Claude Code 2.1.236 or later in the
2.x series. Each call checks the installed version before sending context;
unrecognized versions and failed checks do not run inference. Other providers
are shown as unavailable; they never fall back to Peon's selection.
CLI providers reuse existing logins; no separate API credentials are required.
Custom executable wrappers and custom Codex user-configured backends are not
supported by these profiles.

Administrator-managed CLI policies remain active. They may run required hooks,
add instructions/context, or control request routing. Taskmaster requests only
recommendations and disables optional tools/integrations where supported; it does
not bypass administrator controls. The context settings below limit OrkWorks'
own collection, not additional effects of the CLI's managed policy. A policy
conflict fails the request without retrying with weaker permissions.
Taskmaster has its own capability list, independent of Peon. Providers need a
verified inference-only transport before they
can be used for unattended analysis; ordinary coding-tool launch commands do
not satisfy that requirement.

Model suggestions are static; enter your provider's model ID directly when it
is not listed. Opening Recommendations settings does not run model-discovery
commands. Model IDs are preserved unchanged and must be nonempty, at most
256 UTF-8 bytes, and contain no control characters. Reasoning effort must be
supported by the selected adapter; choose **Provider default** to remove an
unsupported setting. Ollama currently does not support this effort setting.

Background discovery is read-only. Choose **Session observations**, **Workflow
context**, or **Relevant source code** to control what may be sent to the model
provider. Workflow context includes selected instructions, documentation,
manifests, and CI configuration. Add relative paths to exclude files or folders.
Ignored files, credential files, and files outside the workspace are excluded;
additional terminal replay is not collected.

Use **All workspaces** for defaults or **This workspace** for overrides. The
default limit is eight AI evaluations per UTC day across the app and at least
one hour between evaluations of a workspace. Failed calls count. This limits
usage, not the amount a provider may bill. Saving a narrower context invalidates
pending results that used the previous context.

Shared reference knowledge ships with the application and updates automatically
every six hours while the app is running. Updates are verified before use; an
offline or failed update keeps the cached version. You can disable knowledge
updates independently of AI analysis. Settings shows version, last successful
check, and remaining daily evaluations.

Recommendation evidence distinguishes session observations, repository snapshots,
and the knowledge pages that informed a suggestion. Experimental guidance remains
labeled. Knowledge does not override repository instructions or your decisions.
An updated knowledge page alone does not revive a dismissed recommendation.

**Fix with AI** still requires your click and sends a scoped prompt into your
active session. Discovery never edits files or starts sessions. Decisions and
verified completion remain local; the app does not upload lessons to the brain.

See [the design contract](../../specs/taskmaster-knowledge.md) for limits and
distribution details.

## Custom inference executable approvals

Custom inference definitions appear in the Taskmaster provider picker without
requiring Peon support. An approved executable can run in the background when
Taskmaster analysis is enabled and that provider is selected. Calls remain
subject to context settings, exclusions, intervals, and usage budgets. Selecting
a provider does not grant approval; temporarily unavailable saved selections
are retained.

Add an optional `inference` capability in a coding tool's JSON editor. It does
not require Peon support. For example:

```json
{
  "inference": {
    "kind": "command",
    "command": "/absolute/path/to/your-wrapper",
    "args": ["--model", "{model}"],
    "input": "stdin",
    "output": "result-json-v1",
    "timeoutSecs": 60
  }
}
```

This is a capability fragment, not a complete coding-tool definition. The
wrapper must implement the [custom adapter protocol](../superpowers/specs/2026-09-10-custom-inference-adapters-design.md).
Windows requires a native executable or an explicitly configured native
interpreter/wrapper; scripts do not get an implicit shell invocation.

Under **Settings → Recommendations → Custom inference executables**, inspect
the command, resolved path, arguments, input mode, and timeout. **Review and
approve executable** opens a native warning dialog, with Cancel selected by
default. Importing JSON or saving recommendation settings never grants trust.
Definitions or executable resolution changed since inspection require a refresh
and a new review. **Revoke approval** works even if the executable is unavailable.

Approval is global across workspaces and includes updates to that installed
executable, its dependencies, and configuration. It is not a sandbox or a safety
certification. When invoked, the executable receives
permitted context and existing CLI credential access and may run configured hooks
or plugins. Administrator policy still applies; no separate API credentials are
required. Revocation prevents new calls and discards pending output, but a
running process may continue until exit or timeout, and side effects cannot be
undone.
