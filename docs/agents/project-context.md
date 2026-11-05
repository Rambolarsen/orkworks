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

### Local repository verification

Run `bash scripts/verify-repo.sh` from an owned checkout. Its read-only first
step checks the Node major from `.nvmrc`, the installed direct dependencies of
both `apps/desktop/` and `docs/`, and whether `fs-ext` loads in host Node. Select
the pinned Node version, then prepare both surfaces with
`pnpm --dir apps/desktop install --frozen-lockfile` and
`pnpm --dir docs install --frozen-lockfile`. If dependencies were last built for
Electron or another Node ABI, use the pinned host Node and run
`pnpm --dir apps/desktop rebuild fs-ext` before verification.

The prerequisite check reports failures before Rust/build/test work and does not
install or rebuild dependencies. `--dry-run` lists the existing verification
sequence without requiring dependencies. A passing prerequisite check does not
establish GUI or loopback-socket access: sandbox restrictions remain visible test
failures and require an authorized execution environment. The later native ABI
repair, builds and tests retain their existing behavior.

The PR documentation job runs the verifier's isolated prerequisite and ordering
fixtures without installing desktop dependencies or rebuilding native modules.

### Hosted checks

GitHub Actions separates validation, releases, documentation and audits into workflows:

- `.github/workflows/release.yml` preserves canonical stable-tag draft
  packaging and also runs signed nightly prereleases from an immutable `main`
  SHA at 03:23 UTC or by manual dispatch. Its privileged jobs use the protected
  `release` environment; nightly GitHub writes use a separate CI-only token.
- `.github/workflows/pr-ci.yml` validates pull requests targeting `main`.
  Its path-routed Rust job also runs the OpenCode session reporter's Node tests
  using the Node version pinned in `.nvmrc`, plus the POSIX harness reporter's
  Bash diagnostics test. The PR and main Rust jobs have a bounded 15-minute
  budget for setup, compilation, the full suite and reporter diagnostics;
  increasing that budget does not skip a validation step.
  Its focused Windows custom-inference job runs native fixture transport,
  approval-gated activation, and trust tests when Rust files, `rust-toolchain.toml`
  or the PR workflow change. It requires the targeted tests to be discoverable
  before running them, preventing a platform gate from producing an empty pass.
  The same Windows job runs the workspace update socket/native watcher tests,
  with required discovery of atomic-update and directory-recovery regressions.
  Existing Linux job names and branch-protection requirements are unchanged.
- `.github/workflows/codex-listener-proof.yml` runs the dormant connected-stream
  proof and SDK decoder tests on a macOS arm64 runner for Rust changes on PRs
  and `main`, plus manual dispatch. It requires supported-platform test discovery
  before execution; this does not qualify an installed Codex binary or enable
  native startup. [GitHub's runner reference](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)
  documents the `macos-15` arm64 label.
- `.github/workflows/main-ci.yml` validates affected desktop and Rust surfaces
  on pushes to `main`, using the same `.github/ci-paths.yml` dependency filters
  as PR CI. Push comparisons cover all commits since the pre-push SHA. Changes
  limited to unrelated documentation prose skip both product jobs; embedded knowledge,
  desktop test fixtures and documentation behavioral fixtures run Rust tests.
  Desktop also covers its external launch configuration, Taskmaster user-guide
  assertions and release-workflow contract inputs. Daily and manually dispatched runs always run
  both full suites, including the OpenCode and POSIX harness reporter tests.
  Release calls also always run both suites at the supplied immutable SHA,
  including stable-tag pushes; release validation cannot be cancelled by a
  main-push run. Main-push validation runs are not cancelled: each push owns
  its affected-surface comparison, including when a later push changes only docs.
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

The owned `.github/workflows/codeql.yml` is the active CodeQL publisher; managed
default setup is disabled. `CODEQL_ADVANCED_UPLOAD=true` enables publication;
an absent or non-`true` value disables SARIF/database publication for staging or
recovery. Publication is restricted to main and same-repository PRs; manual
runs on other branches remain non-publishing.

The workflow retains the five languages, default query suite, no-build extraction
and existing analysis categories. Four non-Rust language jobs run on every
eligible PR/main push. Rust is skipped only for a complete inventory of recognized
unrelated renderer/prose changes; fixtures, crates, unknown paths and failed diff
detection run Rust. PR comparisons use the cumulative diff; main comparisons use
immutable before/after SHAs. Only superseded PR runs are cancelled. Weekly and
manual runs always analyze all five languages, without source/query exclusions.
See the [qualification report](../validation/2026-11-02-codeql-qualification.md)
for source/coverage/findings comparisons, retained limitations and recovery.

GitHub's native Copilot code review is also enabled as a repository ruleset for
pull requests targeting `main`. It is not a workflow file, is not a required
check, and does not replace the manual `/code-review` gate.

PR CI and the macOS listener proof cancel superseded PR runs. The proof
also cancels superseded main-push runs; its platform coverage is unchanged.

PR CI is path-routed: desktop changes run desktop validation, Rust changes run
a blocking `cargo fmt --check` gate, Rust tests, and the OpenCode reporter Node
tests. Changes to Electron knowledge proof/cache/updater/transport modules and
their tests, shared knowledge fixtures, desktop dependency/install inputs, the
Node pin, or the PR workflow also run the Rust job because its authenticated
route tests execute the real Electron client. Other non-code pull requests
receive a lightweight passing no-op code check.
The full Linux Rust jobs in `pr-ci.yml` and `main-ci.yml` also install frozen
desktop dependencies with pnpm for the real Electron knowledge synchronization,
cache and authenticated-route tests. These jobs use `.nvmrc` host Node and do not
run the Electron native rebuild before Rust tests, preserving `fs-ext`'s host ABI.
The focused Windows inference filters remain unchanged.
Every PR also builds
the documentation site and tests generated-fact filtering. `pr-ci.yml` runs a
`doc-drift` job on every pull request using the same checks as
`scripts/doc-check.sh`; this job is informational and cannot block a merge.

The PR workflow runs the optional local analysis helper's standard-library
tests on Ubuntu and native Windows (Python 3.9) when its scripts or analysis
configuration change. Review diagnostics run when the review scripts or workflow
change. The Windows session-app opener tests run when the opener, its tests,
built-in harness definitions, or Node pin change. Edits to the shared routing
file or PR workflow run all these helper jobs. The local analysis tests check
snapshot/report contracts, locks and credential handling; it does not start a SonarQube stack
or replace the required product checks.

PR CI also exercises the Claude review workflow's inline failure diagnostics
with standard-library Python fixtures. Both review attempts log only bounded,
typed execution metadata; raw prompts, tool output, error text, and execution
files are not published. A numeric HTTP 429 is reported as a rate limit without
assuming a particular account limit. Diagnostics preserve the failed review
status and the existing one-retry bound.
Diagnostics run after a failed action returns; termination of the runner or
the entire job can prevent that final step from running.

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

### Local complexity analysis

An optional, free SonarQube Community Build stack runs through a separate
Podman Compose project. Use the [local analysis runbook](local-sonarqube.md)
for prerequisites, credentials and scan/report commands.
The `simplifying-repo` skill uses its verified snapshots; it is not part of
the hosted weekly audit rotation or the product runtime.

## Windows sidecar file replacement

The Rust sidecar uses the target-specific `windows-sys`
`Win32_Storage_FileSystem` feature only on Windows: `ReplaceFileW` replaces an
expected existing configuration while preserving Windows replacement semantics,
and `MoveFileExW` publishes an expected new file without replacing an existing
one. These operations remain best-effort optimistic concurrency, not portable
compare-and-swap. It adds no dependency to Unix builds.
