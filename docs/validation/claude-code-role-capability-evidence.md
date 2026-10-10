---
type: Validation Record
title: Claude Code role capability evidence
description: Version-bound static evidence and outstanding probe gates for Claude Code role profiles.
tags: [orkworks, validation, harness, agent-hierarchy]
status: draft
---

# Claude Code role capability evidence

Date: 2026-10-10

Installed CLI: `2.1.287 (Claude Code)` at `/opt/homebrew/bin/claude`, resolving to `/opt/homebrew/Caskroom/claude-code/2.1.287/claude`. The resolved executable SHA-256 is recorded below. The installed CLI version and help are the version-bound evidence in this record. Anthropic's online documentation is primary vendor material, checked 2026-10-10, but is mutable and not pinned to CLI 2.1.287; any documented behavior that the CLI help does not establish remains a lead until matched to this version and exercised.

This is a non-Copilot contribution to [#740](https://github.com/Rambolarsen/orkworks/issues/740), following the user's direction to exclude Copilot. It does not consume model inference, start agents, change user/project configuration, or authorize runtime implementation. It is a static capability assessment only.

## Decision

Claude Code 2.1.287 exposes candidate mechanisms for per-session tool lists, allow/deny rules, role agents, supplied system prompts, settings files, MCP configuration, model selection, and skill loading. This establishes that role-profile experiments are plausible. It does **not** establish complete delivery, effective permissions, sandbox enforcement, model identity, or parent-only orchestration for an OrkWorks assignment.

**All six OrkWorks role profiles remain UNVERIFIED / no-go for Claude Code 2.1.287:** orchestrator, research, implementation, review, verification, and remediation. No profile can be offered on this evidence. In particular, a `plan` label or removal of `Edit` alone does not establish read-only behavior when shell, MCP, inherited tools, or alternate configuration remain available.

## Version-bound local evidence

The following read-only commands were run from the repository worktree on 2026-10-10:

```text
claude --version
  2.1.287 (Claude Code)

realpath /opt/homebrew/bin/claude
  /opt/homebrew/Caskroom/claude-code/2.1.287/claude

shasum -a 256 /opt/homebrew/Caskroom/claude-code/2.1.287/claude
  6eab8333fe2121553100d8f40bfada384a3e989b94f947e18ba6677a6fcb41ea
```

`claude --help` was read without starting a session. It exposes these relevant options in this installed binary:

- `--agent` and `--agents` for choosing or defining agents.
- `--tools`, `--allowedTools`, and `--disallowedTools` for tool availability and rules.
- `--permission-mode`, `--permission-prompt-tool`, and `--settings` for permission and session configuration.
- `--system-prompt` and `--append-system-prompt` for caller-supplied prompt text.
- `--model`, `--fallback-model`, and `--strict-mcp-config` for model and MCP setup.
- `--disable-slash-commands` to disable skills, `--no-session-persistence`, and `--bare`; the latter explicitly skips hooks, plugins, auto-memory, and `CLAUDE.md` auto-discovery, while naming explicit context inputs that remain available.
- `--dangerously-skip-permissions` and `--allow-dangerously-skip-permissions` are present and must be excluded from any controlled profile launch.

Reproduce with `claude --version` and `claude --help`, then hash the resolved executable shown by `realpath`. The version string and binary digest identify this machine's sampled installation; they do not prove a future launch uses the same bytes. Recheck and bind the executable identity at launch. No model call or interactive session was started.

## Capability findings

| Area | Static finding | Status and consequence |
| --- | --- | --- |
| Role prompt | The installed CLI accepts `--agent` / `--agents` and caller system prompt flags. | **Candidate only.** No request-bound receipt proves the exact role prompt and mandatory repository rules reached the model. |
| Tool selection | The CLI exposes `--tools`, `--allowedTools`, and `--disallowedTools`; `--tools` can name the built-in tool set. | **Candidate only.** Effective tools across settings, environment, MCP, custom agents, and child agents were not measured. |
| Settings and policy | The CLI accepts `--settings`; current vendor docs describe managed, command-line, project-local, shared-project, and user settings, with precedence and list-merging behavior. | **Limited / exact-version mapping required.** User or organization settings may change effective rules; `--settings` cannot be assumed to replace all ambient settings. Capture loaded sources and resolved policy in a disposable probe. |
| Skill discovery and content | Installed help says `--disable-slash-commands` disables skills. Current vendor docs say normal sessions load skill descriptions, full body on invocation, while custom subagents can preload named skill bodies. | **UNVERIFIED for assignment delivery.** No exact skill-body digest or model-visible receipt was captured; omitted preloading does not prevent discovery/invocation when the `Skill` tool remains available. |
| Repository instructions | Current vendor docs describe `CLAUDE.md` auto-discovery and imported files; `--bare` explicitly disables that discovery. | **UNVERIFIED for exact inheritance and bytes.** No disposable repository hierarchy or startup-context receipt was exercised. Root and scoped instructions must be independently checked. |
| Parent-only delegation | Vendor docs say child agents inherit the main conversation's built-in and MCP tools, subject to filters; child agents can be configured to spawn more agents. | **UNVERIFIED / design risk.** OrkWorks must remove delegation capability from assignments that may not delegate, and verify inherited descendants cannot regain it. Prompt instructions are not a control. |
| Filesystem and shell | CLI exposes tool/permission configuration. Current vendor sandbox docs describe filesystem and network isolation for Bash and warn that sandbox startup fails open by default unless `sandbox.failIfUnavailable` is true. Excluded commands run outside the sandbox. | **UNVERIFIED.** No read/write denial, symlink, shell escape, temp-output, unavailable-sandbox, or excluded-command probe was run. Require fail-closed sandbox behavior where relied upon; permissions alone may not confine arbitrary shell effects. |
| MCP and connectors | CLI exposes `--strict-mcp-config`; agent definitions may carry MCP servers per current vendor docs. | **UNVERIFIED.** No effective server inventory or denial test was run. Strict configuration must be tested against inherited and inline server definitions. |
| Model binding | CLI exposes `--model` and `--fallback-model`. Current docs describe settings and environment inputs that can affect the model. | **UNVERIFIED.** No provider request/model receipt was collected; aliases, fallback, environment, or organization configuration could make a requested identity ambiguous. |
| Skill-use observation | CLI help and the reviewed docs do not provide a qualified OrkWorks request-bound proof that a specific skill body was delivered or used. | **Unknown.** Separate loaded, agent-reported, and directly observed usage. Do not treat directory discovery or a skill name in output as observed use. |
| Same-parent continuation | This static review did not find or exercise a same-parent event/wait channel. | **UNVERIFIED.** PTY liveness, background session support, hooks, or resume commands alone do not prove same-parent model-loop continuation. |

Online references checked 2026-10-10:

- [Claude Code CLI reference](https://docs.anthropic.com/en/docs/claude-code/cli-usage) — flags and commands; current and not version-pinned.
- [Settings files and precedence](https://code.claude.com/docs/en/settings) — scope, precedence, list merging, and loaded-source diagnostics; current and not version-pinned.
- [Create custom subagents](https://code.claude.com/docs/en/sub-agents) — inherited tools/MCP, permissions, and skill preloading; current and not version-pinned.
- [Extend Claude with skills](https://code.claude.com/docs/en/skills) — discovery, invocation, content lifecycle, and `allowed-tools`; current and not version-pinned.
- [Configure the sandboxed Bash tool](https://code.claude.com/docs/en/sandboxing) — filesystem/network sandbox behavior, fail-open default, and excluded commands; current and not version-pinned.
- [Authentication and access management](https://code.claude.com/docs/en/iam) — permission rules, configuration, and administrative controls; current and not version-pinned.

## Bounded follow-up required before eligibility

After the capability-evidence resource gate is reopened and an exact probe configuration is separately reviewed and authorized:

1. Use one disposable local-only child and one role (initial candidate: review with explicitly declared generated/test output). Pin the resolved CLI bytes, actual model/provider, settings sources, environment allowlist, role prompt, repository instruction bytes, and skill bytes.
2. Bind each request's effective tool inventory and prompt/skill content to a request identifier. A directory listing, startup preview, agent report, or prompt assertion is not a delivery receipt.
3. Exercise allowed and denied reads/writes on workspace, parent, home, symlink, and temporary paths; shell command effects; network egress; MCP tools; and direct or recursive child-agent spawning. Verify sandbox unavailability and excluded-command behavior fail closed.
4. Verify exact model/provider identity and that no fallback or ambient environment silently widens or changes the approved profile.
5. Test skill discovery, exact body delivery, invocation, and observable usage separately. Record `loaded`, `agent-reported`, `observed`, or `unknown` with source and coverage.
6. Only mark the one role/tool/model tuple eligible if all mandatory instruction and skill bytes, effective tools/permissions, and request identity are reproducibly established. Unsupported or ambiguous cases remain unavailable; never widen access or switch tools/models silently.

This record supplies a Claude Code evidence lead for the role-configuration work. It does not change the existing Copilot findings, the #740 resource deferral, the proposed #741 contract, the #610 implementation gate, or any runtime behavior.
