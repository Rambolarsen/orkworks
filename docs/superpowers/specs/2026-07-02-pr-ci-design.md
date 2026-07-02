# PR CI Design

- Date: 2026-07-02
- Status: proposed

## Summary

Add a pull-request CI workflow for normal development changes without expanding the existing release pipeline. The workflow should validate only the surfaces touched by the PR: desktop changes run desktop validation, Rust changes run Rust validation, and docs-only or unrelated changes get a lightweight passing no-op job so PRs always show one clear status signal.

## Problem

The repo currently has no PR CI. The only GitHub Actions workflow is the tag-triggered release pipeline in `.github/workflows/release.yml`, so ordinary PRs show no checks at all. That leaves reviewers without automated validation for desktop or Rust changes and makes it unclear whether “no checks” means “nothing is configured” or “something failed to start.”

## Goals

- Add CI for pull requests targeting `main`
- Run desktop validation only when `apps/desktop/**` changes
- Run Rust validation only when `crates/orkworksd/**` changes
- Run both validations when both surfaces change
- Show one lightweight passing check when a PR does not touch either code surface
- Keep release packaging isolated in the existing release workflow

## Non-Goals

- Replacing or expanding the release workflow
- Cross-platform packaging or artifact creation on every PR
- Running Rust checks for desktop-only changes
- Running desktop checks for Rust-only changes
- Adding broad repo-wide linting unrelated to the touched surfaces

## Proposed Design

### Workflow structure

Create a new workflow file dedicated to PR validation, triggered on:

```yaml
on:
  pull_request:
    branches:
      - main
```

The workflow should contain four jobs:

1. `changes`
2. `desktop`
3. `rust`
4. `noop`

The `changes` job is the routing step. It determines whether the PR touches:

- `apps/desktop/**`
- `crates/orkworksd/**`
- neither of those surfaces

It should publish boolean outputs that downstream jobs can consume, such as:

- `desktop_changed`
- `rust_changed`
- `relevant_code_changed`

### Change detection

Use a standard changed-files/path-filter action rather than hand-rolled shell diff parsing. The workflow only needs path classification, not semantic diff analysis.

Expected behavior:

- PR touches only `apps/desktop/**`:
  - `desktop` runs
  - `rust` skips
  - `noop` skips
- PR touches only `crates/orkworksd/**`:
  - `rust` runs
  - `desktop` skips
  - `noop` skips
- PR touches both:
  - `desktop` runs
  - `rust` runs
  - `noop` skips
- PR touches docs/specs/other files only:
  - `desktop` skips
  - `rust` skips
  - `noop` runs and passes

### Desktop job

The desktop job should provide stronger validation, not just the targeted local checks used for one branch.

It should:

1. check out the repo
2. install Node 22
3. install pnpm
4. run `pnpm install --frozen-lockfile` in `apps/desktop`
5. run `npx tsc --noEmit`
6. run the full desktop test suite:
   - `node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs`
7. run `pnpm build`

This gives:

- TypeScript type safety
- renderer/electron/unit-style test coverage
- bundling/build validation

### Rust job

The Rust job should stay lightweight and focused:

1. check out the repo
2. install stable Rust
3. optionally use cargo caching
4. run:

```bash
cargo test --manifest-path crates/orkworksd/Cargo.toml
```

This matches the repo’s documented Rust verification path and keeps the PR signal aligned with actual development commands.

### No-op job

When the PR does not touch either `apps/desktop/**` or `crates/orkworksd/**`, run a tiny success job.

Its purpose is not validation depth; it is status clarity. Reviewers should see one explicit green check instead of an empty PR checks section.

The job should:

- print a short message such as `No desktop or Rust changes detected; skipping code validation.`
- exit successfully

### Separation from release pipeline

Keep the existing release workflow unchanged. PR CI should not:

- build release artifacts
- package Electron apps
- publish releases
- depend on tag/version checks

That logic remains in `.github/workflows/release.yml`.

## Operational Notes

- The workflow should use branch protection-friendly job names so required checks can be configured later without churn.
- `desktop`, `rust`, and `noop` should each be individually understandable in the GitHub UI.
- The `changes` job can remain visible even if it is only routing; that makes skipped downstream jobs easier to explain.

## Validation Plan

Implementation should verify:

- a desktop-only PR runs `desktop` and skips `rust`/`noop`
- a Rust-only PR runs `rust` and skips `desktop`/`noop`
- a mixed PR runs both `desktop` and `rust`
- a docs-only PR runs only `noop`
- the release workflow still triggers only on `v*` tag pushes

## Risks

- Path filters that are too narrow may skip real validation for moved or renamed files
- Path filters that are too broad may waste CI time
- The full desktop build/test job will be slower than targeted local verification, but that is intentional for PR quality

## Recommendation

Implement one new PR CI workflow with path-based routing and per-surface jobs. This is the lowest-complexity design that gives useful signal for real code changes while avoiding unnecessary CI cost on unrelated PRs.
