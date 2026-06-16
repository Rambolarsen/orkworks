# M5: Git Context Detection — Design

## Overview

Enhance git context detection: per-session git metadata via `git2`, worktree detection, changed file count, conflict warnings when multiple sessions share a dirty working directory, and worktree-aware recommendations.

## Architecture

### New Rust module: `crates/orkworksd/src/git.rs`

Replaces the shell-based git helpers (`git_repo_root`, `git_branch`, `git_dirty`) with a `git2`-based `GitContext` struct.

```rust
pub struct GitContext {
    pub repo_root: Option<String>,
    pub branch: Option<String>,
    pub dirty: bool,
    pub changed_files: usize,
    pub is_worktree: bool,
}
```

**Detection logic:**
- `git2::Repository::discover(path)` — open repo from cwd
- `repo.head()?.shorthand()` — branch name
- `repo.statuses(None)` — count changed files, determine dirty
- Check if `.git` is a file (worktree) vs directory (main repo)

**Conflict detection (added to `list_sessions`):**
- Group active sessions by working directory (cwd)
- If ≥ 2 sessions share same cwd and that directory is dirty → conflict warning
- Sessions in worktrees have different cwds → never conflict with each other

**Recommendation engine:**
- Simple function: given session's git context + session count in same cwd → return optional recommendation string
- Rules: worktree = good, shared dirty = warn, main repo + implementation = suggest worktree

### Changes to `crates/orkworksd/src/main.rs`

**Removed:** `git_repo_root`, `git_branch`, `git_dirty` shell helpers — replaced by `git.rs`

**Updated `SessionInfo`:** add `repo_root`, `branch`, `dirty`, `changed_files`, `is_worktree`, `conflict_warning`, `recommendation` fields

**Updated `set_workspace`:** use `git::detect()` instead of shell helpers

**Updated `list_sessions`:** detect git context per session, compute conflicts and recommendations

**Updated `create_session`:** detect git context on creation, write to metadata

### Changes to `crates/orkworksd/src/metadata.rs`

**Updated `SessionMetadata`:** add `repo_root`, `branch`, `dirty`, `changed_files`, `is_worktree` fields

### Dependencies

- `git2 = "0.19"` — libgit2 bindings (C dependency, but widely available)

### Frontend

**Updated `SessionInfo` type in api.ts:** add git context fields, `conflictWarning`, `recommendation`

**Updated RightSidebar:** show git context in session detail (branch, dirty/clean, changed file count, worktree badge)

**Updated LeftSidebar:** show conflict warning banner above sessions when applicable

**Updated WorkspaceHeader:** use git2-based context instead of shell-based

## Data Flow

```
Session created → git::detect(cwd) → write to SessionMetadata + SessionInfo
Session polled  → git::detect(cwd) → update if changed (dirty, changed_files)
Session list    → detect conflicts by grouping cwds → attach warnings
Right sidebar   → show git context for active session
Left sidebar    → show conflict banner if needed
```

## Non-goals

- Creating/deleting/merging worktrees (the app observes, doesn't manage)
- Branch management
- Automatic worktree creation
- Full `git status` display (just counts + dirty flag)

## Testing

- Unit tests for `GitContext` detection (mockable via temp git repos)
- Unit tests for conflict detection (given session lists, verify warnings)
- Unit tests for recommendations (given git context + session count, verify output)
- Serialization tests for new SessionInfo/SessionMetadata fields
