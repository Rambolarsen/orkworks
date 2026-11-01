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

Follow-up discovery correction, 2026-11-01: inspect Codex’s explicit
`skills/.system` root and its documented `plugins/cache/<marketplace>/<plugin>/<version>`
packages under `CODEX_HOME` (default `~/.codex`). Portable `plugin.json` uses
`skills/`; compatibility `.codex-plugin/plugin.json` uses its declared relative
skills directory. Reject escaped roots, malformed manifests and unsupported
forms with diagnostics; do not execute plugin content. Traversal has a shared
256-directory budget, in addition to the existing root and skill budgets.

[OpenAI plugin packaging](https://developers.openai.com/plugins/build/plugins)
was checked for cache layout, portable paths and compatibility manifests.
Temporary fixtures cover these layouts, system-only hidden traversal,
`CODEX_HOME`, distinct cached versions, malformed/escaped paths and limits.
A read-only inventory comparison against the same local primary workspace
changed Codex found entries from 24 to 101; Claude 82, Copilot 54 and OpenCode 75
were unchanged. These are discovered directory entries, not distinct effective
skill names or native session totals. The inventory also now reads a bounded
64 KB prefix rather than rejecting an entire skill whose instruction body
exceeds that limit. Oversized frontmatter is still rejected and detail reads
retain their separate existing limit. At that point, the APM provenance read gap
remained diagnosed. No native session was launched.

Uncovered: native activation/disabled state and precedence; Codex active cached
version selection; the Copilot plugin registry; OpenCode JSONC/global custom paths; session-specific added directories,
enterprise/remote skills and configuration overrides not inherited by the desktop.
Legacy `.codex/skills` files remain visible with unknown coding-tool discovery.

## Name grouping and source lists

Owner requested grouping by skill name with an inspectable source list on
2026-11-01. One row per exact name groups sources across Workspace and Personal;
each source retains its inventory identity, scope, paths, plugin/version and
separate instruction text. Counts distinguish names from source entries. Filters
apply to sources before grouping, without selecting an effective winner.

The Electron fixture covers one name across workspace, personal local and plugin
sources with differing descriptions, source-ID-specific detail reads, search,
combined coding-tool/scope filters, empty results, workspace closure, and layout
at 360/700/1000px. Alias coalescing and discovery remain Electron-owned and are
unchanged by this presentation adjustment.

## APM ownership read correction

Follow-up, 2026-11-01: the selected workspace lock was 379,199 bytes, exceeding
the generic 64 KiB text limit and leaving its sources without APM attribution.
The inventory now reads lock files with a dedicated 2 MiB limit and matches
recorded SKILL.md paths against discovered canonical and logical aliases within
one scope. That ownership correction did not inspect arbitrary paths from a
lock, infer plugin registration, validate deployed-file hashes, or prove current
content integrity.

Temporary fixtures cover realistic deployment-hash volume, separate unowned
copies, personal aliases, scope isolation, malformed/oversized locks, and sources
with both APM ownership and coding-tool plugin registration. The Electron UI
fixture checks package labels and package-name search. A read-only check of the
same primary workspace found 43 APM-attributed entries, including both
`context-engineering` copies owned by `addyosmani/agent-skills`, with no APM read
diagnostic. No APM operation or personal configuration mutation was performed.

## APM instruction hash comparisons

Follow-up, 2026-11-01: inventory now compares supported recorded SHA-256 hashes
against the full discovered SKILL.md file. Ownership and comparison are separate
facts. Comparison uses bounded no-follow regular-file reads, observes size and
timestamps before/after reading, and reports unavailable if a file changes during
the read. It reserves at most 1 MiB per file plus the EOF probe within a shared
16 MiB inventory budget, caching one digest per canonical file. Conflicting
baselines, missing/unsupported hashes, and read failures/limits are unavailable.

Temporary fixtures cover matching and changed instruction bodies beyond the
metadata prefix, new results after refresh, missing/unsupported hashes,
conflicting/incomplete alias baselines, per-file and aggregate limits, and
preserved APM ownership. A controlled filesystem fixture retains real handles
and modifies a temporary file after the first hash read, verifying that an
unstable read is unavailable. Electron fixtures cover collapsed change cues, all three source
results, package filtering, and narrow layout. Changed only filtering is covered
for its off-by-default state, confirmed differences, combined search/tool/scope
filters, counts, empty results, clearing, and source-specific instruction reads. Comparisons are point-in-time
observations of SKILL.md, not companion-resource verification, package
reputation/authenticity, or proof of a manual edit. No files are repaired or
installed and no session is restarted.

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

## Desktop APM configuration

Owner approved a Skills-page APM section on 2026-11-01, tracked in
[#1266](https://github.com/Rambolarsen/orkworks/issues/1266). Desktop settings
retain an optional absolute executable path; empty uses the desktop PATH.
The section displays the resolved path, detected version and qualification
status, with explicit save/check and recheck actions. No automatic APM install
or upgrade occurs. The selected executable runs as the user with existing
OrkWorks capability stripping and bounded output/time; version compatibility
is not an authenticity claim.

Temporary executable fixtures cover override priority, missing paths without
fallback, directories/relative-path rejection, version compatibility and
settings persistence. Installer fixtures check that selection changes invalidate
reviews. The Electron UI fixture exercises status, saving/resetting an override,
incompatible-version Add disabling and 360/700/1000px layout. This is source and
fixture evidence, not published-installer availability.
