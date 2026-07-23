# Harness integration contracts

This evidence register is the implementation gate for compiled harness signal
and integration bindings. It was retrieved on 2026-07-23. A row marked
**limited** or **unsupported** must not be promoted until its missing exact
fixture and version/feature evidence are added beside the binding.

| Harness | Primary evidence | Local configuration / event contract | Status and no-op rule |
| --- | --- | --- | --- |
| Claude Code | [Hooks reference](https://code.claude.com/docs/en/hooks) | `.claude/settings.local.json` is local/non-shareable. The documented `hooks` object contains event matcher groups and command hooks; common payload includes string `session_id`, `cwd`, and `hook_event_name`. | Feature-probed. Install only owned local entries; unknown event fields are ignored. Coverage remains limited until version-pinned fixtures cover selected events. |
| Codex | [Codex hooks](https://learn.chatgpt.com/docs/hooks) | No stable primary schema or local-only target was retrievable for this contract review. | Unsupported for installation; `OPENCODE_SESSION_ID` is not applicable. Do not infer a hook payload or edit `.codex` files. |
| OpenCode | [Plugins](https://dev.opencode.ai/docs/plugins/) | Plugin configuration is documented, but the reviewed page does not pin a stable workspace plugin event payload type or establish `.opencode/plugins/orkworks.js` as an eligible ignored/untracked target. | Limited / activation unknown. `OPENCODE_SESSION_ID` remains the only high-confidence session-ID source. `session.created` is disabled; no installer writes a plugin file. |
| Gemini CLI | [Hooks reference](https://geminicli.com/docs/hooks/reference/) | `settings.json` has a `hooks` object; command hook input includes string `session_id`, `cwd`, `hook_event_name`, and ISO `timestamp`. The documented lifecycle events include `BeforeAgent`, `AfterAgent`, `SessionStart`, `SessionEnd`, and `Notification`. | Feature-probed. Never edit a tracked/shareable settings file. Without an eligible local target and version-pinned fixtures, status is limited / activation unknown. |
| GitHub Copilot CLI | [Hooks reference](https://docs.github.com/en/copilot/reference/hooks-reference) | `.github/copilot/settings.local.json` supports inline `hooks`; command hooks use versioned JSON configuration. Documented payload includes `sessionId` string and numeric `timestamp`; `notification` reports `agent_idle` and `permission_prompt`. | Feature-probed. Install only owned local entries. Unsupported event/payload variants are a no-op until exact fixtures and version evidence pass. |
| Aider | [Notifications](https://aider.chat/docs/usage/notifications.html) | `--notifications-command` runs a configured command when Aider is waiting for input; it provides no native session ID or lifecycle schema. | Limited. The workspace-owned enablement flag may augment launch with the stable reporter; no repository Aider config is edited. |
| Generic shell | No deterministic extension point | None. | Unsupported; all integration mutation requests are no-ops with a conflict response. |

Decision rule: primary schema + reproducible fixture + version/tag evidence is
verified; primary schema + fixture without version evidence is feature-probed;
a documented event without stable payload schema is limited with unknown
activation; and a target that is not local-only or already ignored/untracked is
unsupported for installation.
