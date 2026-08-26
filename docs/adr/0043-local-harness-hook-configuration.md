# Harness hook configuration is local; doc checks use a committed shared source

- Status: accepted
- Deciders: Lars-Erik, Codex
- Date: 2026-08-26

## Context

Harness hook configuration contains machine-specific commands, paths, and
activation state. Committing it causes cross-environment drift and can make
OrkWorks refuse or overwrite a configuration created on another machine.
The repository also needs one consistent documentation-currency check that
can be used by multiple coding tools and CI.

## Decision

- Keep the harness-neutral documentation checker at `scripts/doc-check.sh`.
  Claude Code and CI invoke that committed source directly; other harnesses
  may use generated local adapters.
- Ignore workspace-local harness hook configuration and generated adapters:
  Claude `settings.local.json`, Copilot `settings.local.json`, Gemini
  settings, and the entire `.codex/` directory. OrkWorks may install its
  reporter entries there without creating cross-environment Git changes.
- Keep project-level shared configuration such as `.claude/settings.json`
  tracked when it contains repository-owned tooling configuration.

## Consequences

Each environment must generate or install its local harness hooks after a
checkout. The committed doc-check source remains available to CI and every
harness adapter. The prior Codex-specific decision to support a tracked
`.codex/hooks.json` is superseded because cross-environment portability is the
more important constraint.
