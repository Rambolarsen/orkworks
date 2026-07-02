# APM-Managed MCP Configuration Design

- Date: 2026-06-30
- Status: proposed

## Summary

This repo should manage MCP server configuration through `apm.yml` instead of hand-maintained client-specific config files.

The first slice should include:

- `apm.yml` as the canonical source of truth for MCP servers in this repo
- repo-committed MCP definitions for the GitHub server and an IDE/code-navigation server, if the chosen IDE server is portable across the target clients
- continued use of `.github/copilot-instructions.md` as a minimal pointer to `AGENTS.md`, not a second place to maintain MCP details
- environment-based or user-local auth for MCP servers, never committed secrets

The first slice should not include:

- duplicate per-client MCP definitions when APM can generate them
- hardcoded tokens, API keys, or user-specific machine paths in repo config
- a parallel non-APM source of truth for the same MCP servers
- client-specific exceptions unless portability actually fails for a required client

## Problem

The repo already standardizes agent dependencies through APM (`apm.yml` plus `apm install`) but MCP configuration has not been declared there yet. Managing MCP servers separately per client would create drift between Copilot/Codex/OpenCode-style clients and would bypass the dependency-management pattern the repo already uses for skills and plugins.

The repo also already simplified `.github/copilot-instructions.md` to point at `AGENTS.md`, so adding MCP details there would reintroduce a second maintenance surface without helping cross-client distribution.

## Design Goals

- Keep one repo-owned source of truth for MCP configuration.
- Reuse the existing APM workflow rather than introducing a parallel convention.
- Support the clients this repo already targets in `apm.yml` (`claude`, `codex`, `copilot`, `opencode`) as far as APM can distribute the same MCP definitions.
- Avoid committing secrets or user-local configuration.
- Keep the configuration portable and easy to audit.

## Proposed Design

### Architecture

`apm.yml` becomes the canonical manifest for MCP servers in this repo under `dependencies.mcp`.

Human guidance remains split from machine configuration:

- `AGENTS.md` stays the authoritative human-readable workflow/conventions document
- `.github/copilot-instructions.md` stays a tiny pointer to `AGENTS.md`
- `apm.yml` owns MCP server declarations and distribution to supported clients

This keeps the repo's agent setup model consistent: skills, plugins, and MCP servers are all declared through APM.

### Components And Data Flow

1. A developer adds or updates MCP entries in `apm.yml`.
2. `apm install` resolves those MCP server declarations and writes the client-specific wiring for supported local clients.
3. Clients consume the generated MCP configuration locally.
4. Repo documentation continues to describe workflow and conventions in `AGENTS.md`, without duplicating server wiring instructions.

The first target servers should be:

- **GitHub MCP server** — strong fit because this repo's workflow depends on GitHub issues, PRs, and release automation
- **IDE/code-navigation MCP server** — only if the selected server is portable enough across the APM-managed clients you care about; otherwise it should be deferred rather than forcing config drift

### Portability Rule

Portability is more important than checking every possible box.

If one desired MCP server cannot be expressed cleanly through APM for the clients this repo actually uses, the repo should:

1. keep `apm.yml` as the canonical source for the portable subset
2. document the exception explicitly
3. avoid forking the full configuration model into separate per-client files unless there is no practical alternative

## Error Handling And Operational Rules

- Secrets must not be committed into `apm.yml`; use environment variables, local auth flows, or user-local client configuration.
- If `apm install` cannot materialize a server for one required client, treat that as a portability/design issue to resolve before adding more manual config.
- Prefer omitting a marginal server over adding a second competing configuration system.
- Keep generated client config out of hand-maintained repo docs unless a client-specific exception becomes unavoidable.

## Non-Goals

- Rewriting `AGENTS.md` to explain MCP server internals
- Expanding `.github/copilot-instructions.md` into a second source of truth
- Supporting every imaginable MCP server in the first slice
- Client-specific customization beyond what APM needs to generate compatible local config

## Testing And Validation

Implementation should verify:

- `apm.yml` contains the intended `dependencies.mcp` entries
- `apm install` succeeds with the new MCP definitions
- the generated local client config includes the expected servers for the supported clients
- `.github/copilot-instructions.md` remains the minimal pointer to `AGENTS.md`
- no secrets or machine-specific paths are introduced into committed files

## Open Questions

- Which IDE/code-navigation MCP server is the best cross-client fit for this repo's actual developer environments?
- Whether the first implementation should land only the GitHub MCP server and leave IDE/code-navigation for a follow-up once portability is confirmed
