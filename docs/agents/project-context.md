---
type: Agent Guide
title: Project context
description: Product identity, repository state, CI routing, and development environments.
tags: [orkworks, repository, ci, development]
status: stable
---

# Project context

## Identity

OrkWorks is local-first mission control for AI coding sessions. Peons observe
individual sessions; Taskmaster recommends what should happen next across
harnesses, models, reviews, capacity, and Git context. OrkWorks observes and
recommends before it controls — it does not replace Claude Code, Codex,
OpenCode, Gemini CLI, or Aider.

## State of the repository

This is an APM-bootstrapped project: agent skills, hooks, and plugins are
installed through [APM](https://github.com/anthropics/apm) at the repository
root. M1 (the Electron app shell and Rust sidecar scaffold) is implemented;
subsequent milestones are tracked as GitHub issues.

## Package manager

Use **pnpm** for Node.js package-management tasks in this repository, including
`apps/desktop/` and `docs/`; do not use npm or yarn. For desktop-specific
commands and validation, read [`apps/desktop/AGENTS.md`](https://github.com/Rambolarsen/orkworks/blob/main/apps/desktop/AGENTS.md).

## CI routing

GitHub Actions has seven distinct workflow classes:

- `.github/workflows/release.yml` handles tag-driven release packaging only.
- `.github/workflows/pr-ci.yml` validates pull requests targeting `main`.
  Its focused Windows custom-inference job runs native fixture transport,
  approval-gated activation, and trust tests when Rust files, `rust-toolchain.toml`
  or the PR workflow change. It requires the targeted tests to be discoverable
  before running them, preventing a platform gate from producing an empty pass.
  Existing Linux job names and branch-protection requirements are unchanged.
- `.github/workflows/main-ci.yml` unconditionally reruns the full desktop and
  Rust test suites against `main`, without path filters, on every push to
  `main`, daily by schedule, and by manual dispatch. This exists because
  `pr-ci.yml` only triggers on `pull_request` and never re-validates `main`
  after a merge, so a bad merge — including one that bypasses branch
  protection as an administrator — could sit undetected until the next pull
  request; the daily schedule also catches drift such as flaky tests and
  dependency updates without a code change.
- `.github/workflows/docs.yml` builds the VitePress documentation site and
  deploys it to GitHub Pages when documentation or generated-fact inputs change
  on `main`, after public release changes, and daily to refresh release data.
- `.github/workflows/docs-audit.yml` checks public guides weekly and proposes
  bounded corrections through a draft PR. See [site maintenance](site-maintenance.md)
  for credentials, permissions, and the review handoff.
- `.github/workflows/quality-audit.yml` runs a weekly rotating quality audit
  from the skills in `skills/`, filing scoped issues under those skills'
  guardrails. It requires the `CLAUDE_CODE_OAUTH_TOKEN` repository secret;
  the workflow header documents the `ANTHROPIC_API_KEY` alternative.
- `.github/workflows/pr-review.yml` posts an informational automated first-pass
  review when a relevant-code pull request is opened, reopened, marked ready
  for review, or synchronized. Relevant code is scoped to changes under
  `apps/desktop/` or `crates/orkworksd/`; Claude is invoked only after the
  cumulative non-docs delta since the last completed review exceeds the
  escalation threshold (roughly more than 8 files or 500 changed lines). A
  manual workflow dispatch with a pull-request number can intentionally force
  a relevant-code review below the threshold. The workflow reuses the
  `CLAUDE_CODE_OAUTH_TOKEN` repository secret, does not run for
  documentation-only changes, is informational rather than a required check,
  cannot block a merge, and does not replace the manual `/code-review` gate.

GitHub's native Copilot code review is also enabled as a repository ruleset for
pull requests targeting `main`. It is not a workflow file, is not a required
check, and does not replace the manual `/code-review` gate.

PR CI is path-routed: desktop changes run desktop validation, Rust changes run
a blocking `cargo fmt --check` gate plus Rust tests, and non-code pull
requests receive a lightweight passing no-op code check. Every PR also builds
the documentation site and tests generated-fact filtering. `pr-ci.yml` runs a
`doc-drift` job on every pull request using the same checks as
`scripts/doc-check.sh`; this job is informational and cannot block a merge.

## Containerized development environment (optional)

The root `Containerfile` and `compose.yaml` provide an optional Podman/OCI
toolchain that can build, type-check, lint, and test both `apps/desktop` and
`crates/orkworksd` without host Node, Rust, or Electron installations. It is
an alternative to the native pnpm workflow described in
[`apps/desktop/AGENTS.md`](https://github.com/Rambolarsen/orkworks/blob/main/apps/desktop/AGENTS.md), not a replacement;
the native host workflow and release pipeline remain unchanged. Toolchain
versions are pinned in `rust-toolchain.toml`, `.nvmrc`, and `packageManager`.
GUI runs remain on the native flow (issue #80 Tier 2).

Substitute `docker compose` for `podman compose` when using Docker.

`podman compose` requires a compose provider (`podman-compose` or
`docker-compose`); a bare Podman installation does not include one. Without a
provider it fails with `looking up compose provider failed`. Install
`podman-compose` (for example, with `brew install podman-compose`) or configure
Podman to use `docker-compose`.

```bash
# Build the toolchain image
podman compose build

# Non-GUI tasks (each runs in a throwaway container)
podman compose run --rm dev bash -lc "cd apps/desktop && pnpm install"
podman compose run --rm dev bash -lc "cd apps/desktop && npx tsc --noEmit"
podman compose run --rm dev bash -lc "cd apps/desktop && node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs"
podman compose run --rm dev cargo build  --manifest-path crates/orkworksd/Cargo.toml
podman compose run --rm dev cargo clippy --manifest-path crates/orkworksd/Cargo.toml
podman compose run --rm dev cargo test   --manifest-path crates/orkworksd/Cargo.toml
```

`apps/desktop/node_modules`, `crates/orkworksd/target`, and the Cargo registry
use named volumes rather than host bind mounts because Electron and native
dependencies are platform-specific. On Windows, Podman runs in a WSL2 VM;
for bind-mounted paths on NTFS, set `git config core.autocrlf input` so
in-container shell scripts do not receive incompatible line endings.

## Windows sidecar file replacement

The Rust sidecar uses the target-specific `windows-sys`
`Win32_Storage_FileSystem` feature only on Windows: `ReplaceFileW` replaces an
expected existing configuration while preserving Windows replacement semantics,
and `MoveFileExW` publishes an expected new file without replacing an existing
one. These operations remain best-effort optimistic concurrency, not portable
compare-and-swap. It adds no dependency to Unix builds.
