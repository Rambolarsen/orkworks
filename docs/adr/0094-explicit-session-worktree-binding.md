---
type: Architecture Decision
title: Explicit session task-worktree binding
description: Resolve session Git context from an authenticated, durable task-worktree report.
tags: [architecture, sessions, git, worktrees]
---

# Explicit session task-worktree binding

- Status: accepted
- Date: 2026-10-25
- Deciders: OrkWorks owner

The owner approved the [design](../superpowers/specs/2026-10-25-session-worktree-location-design.md)
and [implementation plan](../superpowers/plans/2026-10-25-session-worktree-location.md)
on 2026-10-25 for [#1056](https://github.com/Rambolarsen/orkworks/issues/1056).

ADR 0032's harness/process/launch CWD chain cannot identify a task worktree when
the coding agent uses a tool-specific working directory. Add an optional durable
`worktreeBinding` resolved by the sidecar from a revisioned, authenticated
`agentMetadata.worktreePath` report. Same-repository bindings outrank that chain;
an unavailable binding fails closed instead of reverting to the launch checkout.
Launch/resume CWD and native identity remain unchanged. Git facts and Changes
share this source and disclose fallback origin. Temporary commands elsewhere do
not replace the task binding.

The narrow helper command and Codex SessionStart instructions work independently
of the gated general metadata helper commands. This does not enable the general
API-only launch marker, native observations or completion gates. Reporting is
cooperative, not automatic directory discovery. Older records retain labeled
fallback behavior; mixed-version writers are unsupported. See the linked design
for lifecycle, recovery, validation and verification contracts.
