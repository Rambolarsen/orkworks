# Scoped Agent Instructions Design

## Goal

Reduce irrelevant agent context while preserving the repository's existing safety, verification, and product constraints.

## Problem

The root `AGENTS.md` is the repository's authoritative entry point, but it currently contains universal workflow rules alongside detailed Electron/renderer and Rust-sidecar rules. Agents working in one subsystem must load instructions for the other, increasing context cost and the risk of applying a rule outside its intended boundary.

## Design

Keep `AGENTS.md` as the authoritative, discoverable repository entry point for every configured harness. It retains universal rules and all cross-boundary invariants: product scope and specs, issue/PR workflow, skill requirements, documentation currency checks, terminology, Electron/renderer boundaries, and metadata-protocol constraints.

Do not move a rule from the root file until its delivery is verified for every configured harness. The validation record must identify each target's native path-scoping mechanism, the exact file it loads, and a probe that shows an agent receives the rule for a task in that path. A root link is not sufficient evidence of delivery.

The initial change therefore adds the [harness-instruction coverage record](../../agents/harness-instruction-coverage.md) and makes this promotion rule explicit in `AGENTS.md`. It keeps the existing instructions at root. A later, evidence-backed change may add scoped files and move only implementation-local detail:

- `apps/desktop/AGENTS.md` may own desktop-local validation commands and implementation conventions.
- `crates/orkworksd/AGENTS.md` may own Rust-local validation commands and module layout.

Cross-cutting contracts remain at root. Any future root directory map must name both scoped files and require cross-cutting work to read both.

## Non-goals

- No new task board, memory store, workflow tool, dependency, or instruction-file duplication.
- No changes to product behavior, source code, CI, APM configuration, or authoritative specs.
- No duplication of root instructions into scoped files.

## Validation

- Confirm the coverage record accounts for Claude, Codex, Copilot, and OpenCode, including their native scoping mechanism and an evidence status.
- Check that the root remains sufficient for root-level and cross-cutting work, including its protocol and Electron/renderer constraints.
- Run the existing documentation and worktree currency checks.

## Rollback

Revert the coverage-record and root-policy additions if they prove misleading. Do not move instructions to scoped files until coverage is established; if a later move causes missed rules, restore the affected rules to root.
