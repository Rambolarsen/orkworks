<script setup>
import { data } from '../.vitepress/site.data.mts'
</script>

# Bring your coding tools

OrkWorks runs the command-line coding tools you install and authenticate on
your machine. Your existing tool accounts and model-provider terms still apply.

## Built-in tools

These launch definitions are generated from the current source registry:

<ul><li v-for="tool in data.tools" :key="tool.id">{{ tool.name }}</li></ul>

An older installer may contain a different set. Built-in launch support does
not imply identical support for resume, model selection, capacity signals,
native voice, or integrations.

## Before your first session

1. Install and sign in to your chosen coding tool using its own instructions.
2. Confirm it starts from your normal terminal.
3. Open OrkWorks Settings and review the available coding tools.
4. Select the tool when creating a session in your workspace.

Official installation and sign-in guides:

- [Claude Code](https://code.claude.com/docs/en/quickstart)
- [Codex](https://github.com/openai/codex#installation-and-setup)
- [OpenCode](https://opencode.ai/docs/)
- [GitHub Copilot CLI](https://docs.github.com/en/copilot/how-tos/copilot-cli/set-up-copilot-cli/install-copilot-cli)
- [Aider](https://aider.chat/docs/install.html)
- [Antigravity CLI](https://www.antigravity.google/docs/cli/install/)

Integrations are optional and installed explicitly from Settings where
supported. They can give OrkWorks more direct session signals. Check the
reported detection, registration, and activation state; an installed
integration is not necessarily active in an already-running session.

Codex resume uses the exact conversation ID captured by its hook. If the
session-start hook report is missed, a later owned Codex hook can supply the
initial ID. OrkWorks checks that Codex still has the thread and its local
rollout saved before resuming; it will not switch to Codex's latest
conversation. A newly started Codex thread may not be resumable until Codex
has saved its rollout.
Codex CLI subagents remain part of their parent OrkWorks session and do not
appear as separate OrkWorks sessions.

### Codex session isolation

OrkWorks checks whether the installed Codex supports `--no-daemon` and uses it
for new sessions and exact resume. Each session then has its own reporting
environment. Codex 0.159.2's default shared background server can retain the
first session's environment, leaving later OrkWorks sessions without a native
ID. After updating OrkWorks, end and relaunch affected sessions. Older Codex
versions without the option keep their existing launch arguments. If the
compatibility check fails, the app shows its generic session startup error.
Check the configured executable and retry; the detailed compatibility
diagnostic is recorded in sidecar logs.

### Codex hook reports

Codex hook trust and command network access are separate. Approving OrkWorks'
hooks with `/hooks` allows them to run; it does not let their commands reach
OrkWorks' local sidecar. If Codex's `workspace-write` network access is off,
the hook cannot post attention state to `127.0.0.1`. Native session IDs use a
private local mailbox when OrkWorks supplies one, so their delivery does not
require network access.

For a loopback-only network proxy, add these settings to `~/.codex/config.toml`
and preserve any other existing settings in the same tables:

```toml
[sandbox_workspace_write]
network_access = true

[features.network_proxy]
enabled = true
domains = { "127.0.0.1" = "allow" }
```

The allow rule is for the loopback host on any port; OrkWorks chooses its local
sidecar port at runtime. Restart Codex after changing its configuration. See
the [Codex configuration reference](https://developers.openai.com/codex/config-reference/)
for the network proxy settings.

On macOS and Linux, the latest OrkWorks Codex hook result is recorded at
`~/.orkworks/hook-scripts/report-harness-event-diagnostic.json`; the Windows
reporter writes the same file. In addition to whether the native ID was parsed
and the POST result, the diagnostic keeps one redacted record each for
`PermissionRequest` and `PostToolUse`: only the top-level key names
`hook_event_name`, `model`, `permission_mode`, `turn_id`, `tool_name`,
`tool_use_id`, and `tool_response`, plus only the `hook_event_name`,
`permission_mode`, `turn_id`, `tool_name`, and `tool_use_id` scalar values.
`tool_use_id` is retained only when it is a string of at most 128 characters.
The diagnostic omits session IDs, paths, `tool_input`, token fields, all other
values, and response bodies. `PostToolUse` is capture-only
and does not change **Needs You** behavior. The new event changes Codex's hook
fingerprint, so approve the updated OrkWorks bundle once in Codex's `/hooks`
screen after updating the integration.

Use the arrow beside a tool to show or hide its details; collapsing keeps
unsaved custom-path edits. If an enabled tool needs installation or repair,
toggle it off and on, then confirm the integration update. Installation
failures appear in that tool's status, so check the message before retrying.

## Coding tools and model providers

A **coding tool** runs your interactive coding session. A **model provider**
supplies inference, including the observer you configure for Peon. These are
different settings. You can also configure custom coding tools; advanced
capabilities depend on the tool and its integration.

Native voice, where supported, belongs to the coding tool. OrkWorks does not
record or route its microphone audio.
