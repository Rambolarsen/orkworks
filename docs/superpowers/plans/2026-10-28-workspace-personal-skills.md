---
type: Implementation Plan
title: Workspace and personal skills
description: Deliver scoped skill discovery and qualified APM additions through desktop settings.
tags: [orkworks, skills, apm, desktop]
status: stable
---

# Workspace and personal skills

Spec: [approved scope](../../../specs/workspace-skills.md). Tracks
[#1118](https://github.com/Rambolarsen/orkworks/issues/1118), PR #1119.
Owner approved implementation on 2026-10-28. Execute inline with TDD; one final
fresh code review. No repeated design gate within this scope.

## Boundaries and sequence

- [x] Record ADR 0097 for Electron-main discovery/process ownership. Qualify APM
  0.33.0 against disposable pure-skill local sources in workspace and personal
  scope. Unknown remote graph/mixed-content packages get an explained terminal
  fallback. No production personal directory is touched by tests.
- [x] Add `electron/skillInventory.ts` and `electron/skillTypes.ts`: bounded YAML
  frontmatter discovery, scope/path provenance, aliases, malformed-root
  diagnostics and known coding-tool roots. No precedence inference. Duplicate
  data-only IPC contracts in `src/skillTypes.ts`; never cross the import boundary.
  Tests in `tests/skillInventory.test.ts` exercise real temporary directories,
  both scopes, ancestor roots, aliases/cycles, malformed/oversized metadata and
  plugin locations/configuration coverage diagnostics.
- [x] Add `electron/skillInstall.ts`: fixed-argv APM runner, version qualification,
  source/manifest safety checks, expiry/input-bound previews, per-destination
  exclusion, explicit confirmation and partial-failure reporting. Tests in
  `tests/skillInstall.test.ts` use filesystem fixtures and controlled process
  results; an optional real-APM fixture verifies actual deployed files and no
  automatic executable work. Remote previews stay unavailable until complete.
- [x] Wire narrow inventory/detail/pick-source/preview/install IPC operations in
  main and preload and declarations in `src/orkworksWindow.d.ts`. Main owns the
  workspace identity and derives personal paths. Revalidate after confirmation;
  no renderer-supplied arbitrary directory or flags. Strip inherited OrkWorks
  credentials and policy-bypass environment values from the APM child.
- [x] Add `src/components/SkillsSettings.tsx` to Settings navigation, with scope
  groups, search/tool filter, read-only details, discovery gaps, explicit add
  scope/target and reviewed confirmation flow. Test actual rendered behavior
  with existing esbuild/Electron fixtures, including narrow layout, duplicate
  names, empty/no-workspace states and error recovery.
- [x] Update canonical docs/spec evidence and issue/PR scope. Run desktop type
  checks, relevant tests, visual QA, `scripts/verify-repo.sh`, docs checks,
  doc-check and worktree-check. Complete `/code-review low` against the code diff
  before merge. Follow the existing PR lifecycle and only remove owned worktrees.

## Review focus and bounded choices

Discovery is an inventory of known roots, not native-session loading proof.
Expose unsupported config, plugin and session-specific override coverage.
Use a 64 KiB SKILL.md bound, 2,000 directories and depth 8 per root; report limits.
YAML parsing uses the existing js-yaml package promoted to a production dependency.
Qualified Add begins with one supported target/profile; complete means a
representative skill adds successfully in both scopes. Existing broader manifests
may be rejected instead of applying unrelated dependencies or lifecycle actions.

Review: Ready. Dependencies 2 (desktop only); blast radius 2 (new settings section);
state changes 3 (explicit APM writes); reversibility 3 (partial changes reported,
manual review); uncertainty 3 (qualified local profile, remote fallback). Total 13.
APM qualification precedes enabling mutation; unsupported profiles cannot pass
preview. No migration/new database or orchestration eligibility is introduced.

## Execution rulings

- APM 0.33 dry-run writes a bootstrap manifest. Preview therefore runs in a
  private temporary project directory; real destinations remain untouched until
  confirmation. Personal routing was separately qualified through the real
  upstream source CLI with a process-local temporary home resolver.
- Qualified mutation begins with Codex-targeted local standalone skills only.
  Remote/mixed sources, broad existing manifests, Windows and other versions
  retain explained terminal fallback, as allowed by the approved design.
- The initial shell used Node 26. Repository verification uses a temporary
  Node 22.23.4 toolchain and a worktree-local fs-ext rebuild for the pinned ABI.

## Verification and review

Full `scripts/verify-repo.sh` passed with Node 22.23.4: Rust formatting/build,
2,296 Rust unit tests and integration suites, 13 reporter tests, native ABI
repair, 1,602 desktop tests, desktop build, docs build/116 contract tests, diff
and documentation/worktree checks. Final UI renderer regression and typecheck
also passed after the review follow-up.

Completed `/code-review medium` at code head
`78095f85a06e292f142230515abdb2fe9ba5a27b`; the size/process boundary justified
medium. Correctness review approved; its optional stale-status finding was fixed
with an observed-red/green delayed-completion renderer test and re-reviewed.
Independent security review found no material issue in the unchanged backend
code at `4c2dca6647af7ad7d402156a1b834e1a199e929d`. Reviewers did not independently
rerun tests/build. PR checks and merge remain the final integration step.
