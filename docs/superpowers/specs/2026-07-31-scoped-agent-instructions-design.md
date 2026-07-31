# Scoped Agent Instructions Design

## Goal

Reduce irrelevant agent context while preserving the repository's existing safety, verification, and product constraints.

## Problem

The root `AGENTS.md` is the repository's authoritative entry point, but it currently contains universal workflow rules alongside detailed Electron/renderer and Rust-sidecar rules. Agents working in one subsystem must load instructions for the other, increasing context cost and the risk of applying a rule outside its intended boundary.

## Design

Keep `AGENTS.md` as the concise, authoritative entry point for every configured harness. It retains universal rules and cross-component invariants: product scope and specs, issue/PR workflow, skill requirements, documentation currency checks, terminology, Electron/renderer boundaries, and metadata-protocol constraints.

Do not move a rule from the root file until its delivery is verified for every configured harness. The initial validation record documents the current root entry points and an evidence status; it does not claim a native path-scoping mechanism or exact scoped file. Before any future rule move, extend each target's record with the proposed native mechanism, exact local file, and successful probe evidence showing an agent receives the rule for a task in that path. A root link is not sufficient evidence of delivery.

The [harness-instruction coverage record](../../agents/harness-instruction-coverage.md) now establishes the delivery mechanisms needed for a scoped split:

- Codex and Copilot load nested `AGENTS.md` files.
- Claude loads path-scoped `.claude/rules/*.md` rules when it reads a matching file.
- OpenCode follows the root router, which explicitly directs it to read the relevant scoped `AGENTS.md` on demand; its `instructions` list is session-wide and is not used for this split.

This change creates `apps/desktop/AGENTS.md` and `crates/orkworksd/AGENTS.md` as the canonical home for local commands and implementation conventions. The root file names both and requires agents to read the applicable file before changing that subsystem. Claude path rules import the canonical scoped files rather than duplicate them.

Cross-cutting contracts remain at root. Work spanning both subsystems must read both scoped files.

## Non-goals

- No new task board, memory store, workflow tool, dependency, or instruction-file duplication.
- No changes to product behavior, source code, CI, APM configuration, or authoritative specs.
- No duplication of root instructions into scoped files.

## Validation

- Confirm each harness reaches the relevant canonical scoped file through its documented or observed mechanism.
- Check that the root remains sufficient for root-level and cross-cutting work, including its protocol and Electron/renderer constraints, and that it routes subsystem work to the right file.
- Run the existing documentation and worktree currency checks.

## Rollback

Restore a moved rule to root if a supported harness misses it, and retain the coverage record as the debugging evidence.
