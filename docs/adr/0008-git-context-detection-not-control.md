# Git context detection first, not workflow control

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

Many AI coding tasks involve Git repositories, branches, and worktrees. OrkWorks could try to manage all of these, but that would place it in the critical path of the user's development workflow. A lighter approach is to detect and display Git context, and use it to inform recommendations.

## Decision

OrkWorks will detect Git context per session (repo root, branch, dirty/clean state, changed file count, worktree status) and display it in the UI. It will warn when multiple active sessions share the same dirty working directory and recommend worktree isolation where appropriate. It will not create, delete, merge, rebase, reset, or clean up worktrees in the MVP.

## Consequences

- Users get situational awareness without ceding workflow control
- Recommendations are informed by real repo state
- Warning about dirty shared workspaces prevents common agent-collision bugs
- Detecting Git context is read-only and safe to run automatically
- Worktree management can be added later as an opt-in convenience
