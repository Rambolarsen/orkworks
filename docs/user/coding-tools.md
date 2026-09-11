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

## Coding tools and model providers

A **coding tool** runs your interactive coding session. A **model provider**
supplies inference, including the observer you configure for Peon. These are
different settings. You can also configure custom coding tools; advanced
capabilities depend on the tool and its integration.

Native voice, where supported, belongs to the coding tool. OrkWorks does not
record or route its microphone audio.
