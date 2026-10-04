---
type: Troubleshooting Guide
title: Serena MCP startup troubleshooting
description: Safe diagnostics and recovery steps for MCP startup failures caused by Serena.
tags: [serena, mcp, troubleshooting, diagnostics, tooling]
status: stable
---

# Serena MCP startup troubleshooting

Use this runbook when an agent reports an obstacle such as "MCP startup failed
due to serena" or a generic MCP startup failure that names Serena, and before
treating it as a repository defect.

## Where Serena is configured

Serena is **not declared in `apm.yml`** (`dependencies.mcp` currently has no
MCP entries), so its activation lives entirely in harness-specific,
gitignored user-local configuration:

| Harness | Location | Entry |
| ------- | -------- | ------- |
| Claude Code | `~/.claude.json` under `mcpServers.serena` (global) | `serena start-mcp-server --context=claude-code --project-from-cwd` |
| Codex | `~/.codex/config.toml` under `[mcp_servers.serena]` | `serena start-mcp-server --context=codex --project-from-cwd` |

Note: `.mcp.json` at the repository root exists but declares an empty
`mcpServers` object; it is not the source of the Serena activation above.

Do not hand-edit these files to add or remove Serena; route MCP configuration
through `apm.yml` and `apm install` (see the APM concept).

## Verify a suspected startup failure

1. Recheck the binary exists and starts:

   ```bash
   which serena && serena --version
   serena start-mcp-server --context=claude-code --project-from-cwd
   ```

   A healthy start logs `Initializing Serena MCP server`, stores the
   full run log under `~/.serena/logs/<date>/mcp_<timestamp>.txt`, and
   auto-detects the project root. It does not log an `ERROR` or `Fatal
   exception` line.

2. If step 1 reproduces a crash, read the full log file referenced in its
   second `INFO` line — the generic agent-visible phrase "MCP startup failed"
   never contains the underlying Python traceback; only the run log does.

3. Match the traceback against the known failure modes below.

## Known failure modes

### Missing `languages:` key in `.serena/project.yml` (verified, September 2026)

**Symptom (run log):** the server reaches
`Loading Serena configuration from ~/.serena/serena_config.yml`, then dies with

```
ERROR ... serena.agent:show_fatal_exception_safe:49 - Fatal exception: 'languages'
KeyError: 'languages'
  ... serena/config/serena_config.py ... _from_dict ... data["languages"]
```

**Root cause:** `.serena/project.yml` at the project root (and in sibling
worktrees) is **gitignored**, so a file hand-edited in one checkout to remove
or rename `languages:` propagates nowhere, and any worktree that
autoregisters with an old Serena-generated template instead of the current
one gets a config whose schema drifts from the installed Serena version.

**Fix:** restore the key (list the languages for which language servers
should start, e.g. `typescript`, `rust` for this repository), or delete the
stale `.serena/project.yml` and let Serena regenerate it, then re-run the
verification command in step 1.

### Serena version drift

**Symptom:** a `KeyError`, `TypeError`, or schema-mismatch traceback in the
run log after upgrading Serena (e.g. via `uv tool upgrade serena-agent`).

**Fix:** regenerate the project-local config with the installed version
(`serena config` or delete and re-autoregister `project.yml`), then verify.

## What not to do

- Do not resume, reopen, or modify another session to work around a Serena
  startup failure.
- Do not edit recommendation or observation files under `~/.orkworks/`
  directly; report verified completion through the Taskmaster API instead.
- Do not commit `.serena/` contents (including `project.yml`) to fix this;
  they are gitignored deliberately.
