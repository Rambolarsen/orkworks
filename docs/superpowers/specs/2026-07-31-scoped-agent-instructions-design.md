# Scoped Agent Instructions Design

## Goal

Reduce irrelevant agent context while preserving the repository's existing safety, verification, and product constraints.

## Problem

The root `AGENTS.md` is the repository's authoritative entry point, but it currently contains universal workflow rules alongside detailed Electron/renderer and Rust-sidecar rules. Agents working in one subsystem must load instructions for the other, increasing context cost and the risk of applying a rule outside its intended boundary.

## Design

Keep `AGENTS.md` as the concise, discoverable repository entry point. It will retain rules that apply everywhere: product scope and specs, issue/PR workflow, skill requirements, documentation currency checks, terminology, and the directory map.

Move detailed implementation rules into the closest applicable instruction file:

- `apps/desktop/AGENTS.md` owns Electron main/renderer boundaries, duplicated IPC-contract ownership, and desktop validation commands.
- `crates/orkworksd/AGENTS.md` owns Rust-sidecar module and metadata-protocol guidance, plus Rust validation commands.

The root file links to both scoped files. Scoped files do not repeat repo-wide rules; they add only constraints that apply to work below their directory.

## Non-goals

- No new task board, memory store, workflow tool, or dependency.
- No changes to product behavior, source code, CI, APM configuration, or authoritative specs.
- No duplication of root instructions into scoped files.

## Validation

- Confirm each instruction file is discoverable from the root and contains only its applicable rules.
- Check that the root still contains the required repository-wide workflow and product constraints.
- Run the existing documentation and worktree currency checks.

## Rollback

Delete the two scoped instruction files and restore the moved sections to the root file if a supported harness cannot discover nested `AGENTS.md` files or the split causes repeated missed rules.
