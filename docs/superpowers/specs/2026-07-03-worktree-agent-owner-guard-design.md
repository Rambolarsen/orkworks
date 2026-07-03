# Worktree Agent Owner Guard Design

- Date: 2026-07-03
- Status: proposed

## Summary

OrkWorks already requires agents not to work on branches they do not own, but that rule is currently policy only. This design adds a lightweight local enforcement layer based on a worktree-local marker file rather than branch naming.

The design introduces:

- an untracked `.orkworks-agent-owner` file in the worktree root
- a small repo-owned guard script that compares that marker to the active agent identity
- Git hooks for early warning and enforcement:
  - `post-checkout`: warn on mismatch
  - `pre-commit`: block on mismatch
  - `pre-push`: block on mismatch

The guard is intentionally permissive when the marker file is absent. It only blocks when a marker exists and names a different owner than the current agent.

## Problem

The repo already documents the correct rule:

- never work on branches you do not own
- use a worktree whenever the active branch belongs to someone else

In practice, agents can still accidentally start or continue work in the wrong checkout because:

- the current branch alone does not reliably identify the active owner
- multiple agents may reuse or inherit a checkout without noticing
- a policy in `AGENTS.md` is easy to miss at the moment of `git commit` or `git push`

The failure mode is straightforward: one agent commits on another agent's in-flight branch or worktree, creating ownership confusion and avoidable collisions.

## Goals

- Detect agent/worktree ownership mismatches early.
- Enforce the rule locally before commits and pushes.
- Keep ownership worktree-scoped, not branch-scoped.
- Avoid blocking checkouts that have no explicit ownership marker.
- Keep the mechanism transparent, repo-owned, and easy to inspect.

## Non-Goals

- No central ownership service or background daemon.
- No branch-name-based ownership enforcement.
- No requirement that every checkout must have a marker.
- No attempt to manage worktrees automatically.
- No attempt to identify a human user precisely when no marker exists.

## Design

### Ownership Model

Ownership is expressed by an untracked file at the worktree root:

```text
.orkworks-agent-owner
```

The file contains one canonical owner id, such as:

- `codex`
- `claude`
- `opencode`
- `copilot`
- `human`

Ownership is local to the checkout, not derived from the branch name.

### Marker Semantics

- If `.orkworks-agent-owner` does not exist, the guard allows all actions.
- If the file exists and matches the active agent id, the guard allows the action.
- If the file exists and does not match the active agent id, the guard warns or blocks depending on the hook.

This matches the desired behavior: missing marker is permissive; explicit mismatch is not.

### Active Agent Identity

The guard script resolves the current owner from environment, with this precedence:

1. `ORKWORKS_AGENT_OWNER`
2. known harness/runtime-specific env vars if the repo already has a stable way to infer them
3. fallback to `unknown`

For the first slice, explicit `ORKWORKS_AGENT_OWNER` is the primary supported path. Additional inference is optional and should not make the script opaque or brittle.

### Hook Behavior

#### `post-checkout`

- Runs after branch/worktree switches.
- If the marker is missing: no warning.
- If the marker matches: silent success.
- If the marker mismatches: print a clear warning telling the agent to stop and create or switch to the correct worktree.
- Does not block checkout completion.

This is the early detection path.

#### `pre-commit`

- If the marker is missing: allow commit.
- If the marker matches: allow commit.
- If the marker mismatches: block commit with a direct error explaining:
  - expected owner
  - actual owner
  - the required fix: move to an owned worktree or update the marker intentionally

#### `pre-push`

- Same ownership rule as `pre-commit`.
- Blocks pushes on explicit mismatch.

### Script Location

The guard should live as a repo-owned script, likely under:

- `.claude/hooks/`, or
- `scripts/`

The exact path is less important than keeping it committed, readable, and callable from Git hooks.

### Git Ignore

`.orkworks-agent-owner` must be ignored so ownership remains local and never leaks into commits.

### Starting-Work Workflow Update

`skills/starting-work/SKILL.md` should be updated so that when an agent opens a branch or worktree for coding work, it also writes or refreshes `.orkworks-agent-owner`.

Expected flow:

1. create branch or worktree
2. write `.orkworks-agent-owner`
3. continue work from that checkout

### Manual/Human Use

Humans can either:

- leave the marker absent, or
- set it to `human`

The design does not require stricter human identity handling in this slice.

## Error Handling

- Missing marker file: success
- Empty marker file: treat as invalid marker and warn/block the same as mismatch, with a message that the file is malformed
- Missing `ORKWORKS_AGENT_OWNER`: resolve as `unknown`
- Marker exists and active agent resolves to `unknown`: treat as mismatch for `pre-commit` and `pre-push`, warning for `post-checkout`

## Security And Trust Model

This is a local safety rail, not a security boundary. Any user can bypass or edit local hooks and marker files. That is acceptable because the goal is preventing accidental workflow mistakes, not defending against malicious local actors.

## Testing Expectations

Implementation should verify:

- missing marker allows checkout/commit/push
- matching marker allows checkout/commit/push
- mismatched marker warns on `post-checkout`
- mismatched marker blocks `pre-commit`
- mismatched marker blocks `pre-push`
- empty or malformed marker is treated as invalid
- `ORKWORKS_AGENT_OWNER` overrides fallback inference

## Documentation Impact

Update at minimum:

- `AGENTS.md`
- `skills/starting-work/SKILL.md`
- `.gitignore`

If hook installation/setup steps need explanation, add a short note to `README.md` or the relevant agent setup doc.

## Recommendation

Implement this as a small repo-owned script plus three lightweight Git hooks. Keep absence permissive, mismatch strict, and make the marker worktree-local and untracked.
