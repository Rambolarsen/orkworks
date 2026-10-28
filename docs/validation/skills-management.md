---
type: Validation Evidence
title: Skill inventory and APM installation qualification
description: Source and fixture evidence for scoped skill discovery and the initial local APM install profile.
tags: [orkworks, skills, apm, desktop, validation]
status: stable
---

# Skill inventory and APM installation qualification

Date: 2026-10-28. Scope: [workspace and personal skills](../../specs/workspace-skills.md).
This record concerns source behavior, not published installer availability or
native skill loading/use. No coding-tool model session was launched.

## Discovery evidence

The registry inventories documented filesystem roots. Tests exercise temporary
workspace/personal trees, duplicate names, alias coalescing, malformed/oversized
frontmatter, configured OpenCode paths, Claude plugin registrations, and detail
reads after a file becomes a symlink. It does not establish native discovery or
precedence for every installed tool version. UI labels say Found and disclose
coverage limits instead of claiming Loaded or In use.

Primary references inspected:

- [OpenAI customization](https://learn.chatgpt.com/docs/customization/overview):
  project/user `.agents/skills` roots.
- [Claude Code skills](https://code.claude.com/docs/en/skills): personal/project
  `.claude/skills`, plugin skills and non-universal scope precedence.
- [Copilot CLI reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference):
  project, inherited, personal and custom skill directories.
- [OpenCode skills](https://opencode.ai/docs/skills): project/user compatible
  roots; project configuration paths are additionally read from root JSON.

Uncovered: native activation/disabled state and precedence; Codex/Copilot plugin
registries; OpenCode JSONC/global custom paths; session-specific added directories,
enterprise/remote skills and configuration overrides not inherited by the desktop.
Legacy `.codex/skills` files remain visible with unknown coding-tool discovery.

## Initial APM profile

Qualified source: APM 0.33.0, upstream commit
`18c4c43c924ceae890fe0f2038806690e5b2d6c8`.
Installed macOS arm64 executable SHA-256:
`657dd56be64e50b0b89c6cfa9f02d9a0ffcce8af6682d2f66810c2711b36222a`.
The runtime admits version 0.33.0; a version match is compatibility evidence,
not executable authenticity or a sandbox guarantee.

Workspace fixture: installed binary, absolute local directory containing only
SKILL.md, `--target codex --only apm --no-trust-bin`. Exit 0; actual skill bytes
appeared in `.agents/skills/skill/SKILL.md`. APM created/updated `apm.yml`,
`apm.lock.yaml`, `apm_modules/_local/skill` and `.gitignore`. The actual Electron-side installer
service also completed preview/confirmation/install against that existing
manifest with the installed binary (available preview and successful result).

Personal fixture: the same upstream version's real source CLI invoked through
Click, with `Path.home()` redirected only inside that test process to a temporary
user directory. HOME and CODEX_HOME were not changed. `--global` exited 0 and
created `.apm/apm.yml`, `.apm/apm.lock.yaml`, `.apm/apm_modules/_local/skill` and
`.agents/skills/skill/SKILL.md` under the isolated user directory. This qualifies
the source-level global routing contract; the installed binary was not run
against the developer's actual personal skill roots.

Reproduce the personal source fixture after checking out the exact APM commit:

```bash
uv run --project /path/to/apm python apps/desktop/tests/skillApmQualification.py
```

The fixture disables lifecycle execution for its disposable test only. Production
does not set that bypass: it rejects lifecycle-bearing manifests/policy and keeps
APM's policy and security checks active. No real personal skills were installed
or changed by this qualification. Test package dependencies are separate from
OrkWorks runtime dependencies.

## Preview and operation boundaries

APM 0.33.0 dry-run bootstraps `apm.yml` even when a destination has no manifest.
Confirmed by actual file inspection, not just CLI output. Therefore OrkWorks
runs previews in private temporary project directories and copies only a bounded
existing manifest when present. A source/destination fingerprint is checked
before/after preview and again after native confirmation. Production Personal
installation uses `--global`; the private project preview is not represented as
an exact personal file diff.

Admitted sources are canonical local standalone skills with a small known
resource layout. Existing manifests must target Codex only and declare only
other validated absolute local standalone skills, without lifecycle, service,
custom include or development dependencies. Broader/remote/mixed packages, Windows
installs and other APM versions remain unavailable in the UI with an explained
terminal fallback. No force, trust, compile, upgrade or session restart occurs.

The installer is not atomic or a confinement boundary. It runs as the user;
external edits/processes can race filesystem checks. Failures may leave partial
changes and require local inspection. OrkWorks serializes its own operations per
destination and does not claim to lock external editors.
