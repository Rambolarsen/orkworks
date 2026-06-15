# Product boundary: observe and recommend before controlling

- Status: accepted
- Deciders: OrkWorks team
- Date: 2026-06-15

## Context

OrkWorks operates alongside existing AI coding harnesses (Claude Code, Codex, OpenCode, Gemini CLI, Aider). The temptation is to build a "do everything" workflow manager, but that would compete with the user's existing tools and workflows instead of complementing them.

## Decision

OrkWorks observes and recommends before it controls. It owns terminal visibility, session overview, metadata, status detection, capacity tracking, and harness/model recommendations. It does not own git workflow, branch strategy, worktree management, merging, task decomposition, or automatic terminal input. Workflow actions may be added later as explicit opt-in conveniences.

## Consequences

- OrkWorks complements existing harnesses rather than competing with them
- Users keep their existing git/worktree workflows unchanged
- The product can focus on observability and recommendation quality
- Clear boundary prevents scope creep into workflow management
- Opt-in controls can be added later without compromising the observe-first philosophy
