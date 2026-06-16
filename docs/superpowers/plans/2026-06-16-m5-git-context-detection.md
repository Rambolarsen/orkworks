# M5: Git Context Detection — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace shell-based git helpers with `git2` crate, add per-session git context (branch, dirty, changed files, worktree detection), conflict warnings, and recommendations.

**Architecture:** New `crates/orkworksd/src/git.rs` module with `git2`-based `GitContext` detection. Updated `SessionInfo`/`SessionMetadata` with git fields. Conflict detection groups active sessions by cwd and flags shared dirty directories. Frontend shows git context in right sidebar detail panel and conflict warnings in left sidebar.

**Tech Stack:** Rust (git2, serde), React/TypeScript

---

## File Structure

| Action | Path | Responsibility |
|--------|------|---------------|
| Create | `crates/orkworksd/src/git.rs` | Git2-based context detection |
| Modify | `crates/orkworksd/src/main.rs` | Replace shell helpers, wire git into session lifecycle |
| Modify | `crates/orkworksd/src/metadata.rs` | Add git fields to SessionMetadata |
| Modify | `crates/orkworksd/Cargo.toml` | Add git2 dependency |
| Modify | `apps/desktop/src/api.ts` | Add git fields to SessionInfo |
| Modify | `apps/desktop/src/components/RightSidebar.tsx` | Show git context in detail |
| Modify | `apps/desktop/src/components/LeftSidebar.tsx` | Show conflict warning banner |
| Modify | `apps/desktop/src/App.css` | Conflict warning styles |

---

### Task 1: Add git2 dependency

**Files:**
- Modify: `crates/orkworksd/Cargo.toml`

- [ ] **Step 1: Add git2 to dependencies**

In `crates/orkworksd/Cargo.toml`, add after the `notify` line:
```toml
git2 = "0.19"
```

- [ ] **Step 2: Build to verify**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml`
Expected: compiles without errors (may download/build libgit2)

- [ ] **Step 3: Commit**

```bash
git add crates/orkworksd/Cargo.toml crates/orkworksd/Cargo.lock
git commit -m "chore: add git2 crate for git context detection"
```

---

### Task 2: Create git.rs module

**Files:**
- Create: `crates/orkworksd/src/git.rs`
- Modify: `crates/orkworksd/src/main.rs` — add `mod git;`

- [ ] **Step 1: Add mod declaration in main.rs**

After `mod watcher;` add:
```rust
mod git;
```

- [ ] **Step 2: Write git.rs**

```rust
use std::path::Path;

#[derive(Debug, Clone)]
pub struct GitContext {
    pub repo_root: Option<String>,
    pub branch: Option<String>,
    pub dirty: bool,
    pub changed_files: usize,
    pub is_worktree: bool,
}

pub fn detect(cwd: &Path) -> GitContext {
    let repo = match git2::Repository::discover(cwd) {
        Ok(r) => r,
        Err(_) => {
            return GitContext {
                repo_root: None,
                branch: None,
                dirty: false,
                changed_files: 0,
                is_worktree: false,
            };
        }
    };

    let repo_root = repo
        .workdir()
        .map(|p| p.display().to_string());

    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()));

    let is_worktree = repo
        .workdir()
        .map(|w| w.join(".git").is_file())
        .unwrap_or(false);

    let mut changed_files = 0;
    let mut dirty = false;

    if let Ok(statuses) = repo.statuses(None) {
        changed_files = statuses.len();
        dirty = changed_files > 0;
    }

    GitContext {
        repo_root,
        branch,
        dirty,
        changed_files,
        is_worktree,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_no_repo_returns_empty_context() {
        let dir = tempfile::tempdir().unwrap();
        let ctx = detect(dir.path());
        assert!(ctx.repo_root.is_none());
        assert!(ctx.branch.is_none());
        assert!(!ctx.dirty);
        assert_eq!(ctx.changed_files, 0);
        assert!(!ctx.is_worktree);
    }

    #[test]
    fn detect_in_git_repo() {
        // Assumes CWD is in this repo
        let ctx = detect(&std::env::current_dir().unwrap());
        assert!(ctx.repo_root.is_some());
        assert!(ctx.branch.is_some());
        assert!(ctx.changed_files > 0 || !ctx.dirty);
    }

    #[test]
    fn dirty_repo_has_changed_files() {
        let ctx = detect(&std::env::current_dir().unwrap());
        // After all our commits, there may be dirty files
        // Just verify the fields are present and consistent
        if ctx.dirty {
            assert!(ctx.changed_files > 0);
        }
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: tests pass (20 existing + 3 new git tests)

- [ ] **Step 4: Commit**

```bash
git add crates/orkworksd/src/git.rs crates/orkworksd/src/main.rs
git commit -m "feat: add git2-based git context detection module"
```

---

### Task 3: Wire git.rs into main.rs — session lifecycle

**Files:**
- Modify: `crates/orkworksd/src/main.rs`

Read the current file first. You need to:
1. Remove old shell-based `git_repo_root`, `git_branch`, `git_dirty` functions
2. Update `SessionInfo` with git fields
3. Update `set_workspace` to use `git::detect`
4. Update `create_session` to detect git context on creation
5. Update `list_sessions` to detect conflicts and attach warnings/recommendations
6. Update `set_session_status` to refresh git context on status changes

- [ ] **Step 1: Remove old shell-based git helpers**

Delete functions: `git_repo_root`, `git_branch`, `git_dirty` (search for them in main.rs and remove all three).

- [ ] **Step 2: Update SessionInfo struct**

Replace the `SessionInfo` struct:
```rust
#[derive(Clone, Debug, Serialize)]
struct SessionInfo {
    id: String,
    label: String,
    status: String,
    cwd: String,
    created_at: String,
    #[serde(rename = "metadataSource")]
    metadata_source: Option<String>,
    #[serde(rename = "metadataConfidence")]
    metadata_confidence: Option<f64>,
    #[serde(rename = "repoRoot")]
    repo_root: Option<String>,
    branch: Option<String>,
    dirty: Option<bool>,
    #[serde(rename = "changedFiles")]
    changed_files: Option<usize>,
    #[serde(rename = "isWorktree")]
    is_worktree: Option<bool>,
    #[serde(rename = "conflictWarning")]
    conflict_warning: Option<String>,
    recommendation: Option<String>,
}
```

- [ ] **Step 3: Update set_workspace to use git::detect**

Replace the git context detection in `set_workspace`:
```rust
let git_ctx = git::detect(&ws_path);
let repo_root = git_ctx.repo_root;
let branch = git_ctx.branch;
let dirty = Some(git_ctx.dirty);
```

- [ ] **Step 4: Update create_session to include git context**

After the metadata write block, add git context to the SessionInfo creation. In `create_session`, update the `SessionInfo` construction to include git fields:

After the session insert, before `Json(info)`:
```rust
let git_ctx = git::detect(&PathBuf::from(&info.cwd));
let mut info = info;
info.repo_root = git_ctx.repo_root;
info.branch = git_ctx.branch;
info.dirty = Some(git_ctx.dirty);
info.changed_files = Some(git_ctx.changed_files);
info.is_worktree = Some(git_ctx.is_worktree);
```

- [ ] **Step 5: Add conflict detection and recommendations to list_sessions**

Add helper functions before `list_sessions`:
```rust
fn detect_conflicts(sessions: &[SessionInfo]) -> Vec<(String, String)> {
    use std::collections::HashMap;
    let mut cwd_groups: HashMap<&str, Vec<&SessionInfo>> = HashMap::new();
    for s in sessions {
        if s.status == "running" || s.status == "creating" {
            cwd_groups.entry(&s.cwd).or_default().push(s);
        }
    }
    let mut warnings = Vec::new();
    for (cwd, group) in &cwd_groups {
        if group.len() >= 2 {
            if let Some(s) = group.first() {
                if s.dirty.unwrap_or(false) {
                    warnings.push((
                        group[0].id.clone(),
                        format!("{} sessions in this dirty workspace", group.len()),
                    ));
                }
            }
        }
    }
    warnings
}

fn recommendation(ctx: &git::GitContext, session_count_in_cwd: usize) -> Option<String> {
    if ctx.is_worktree {
        return Some("Running in a separate worktree. Good isolation.".into());
    }
    if session_count_in_cwd >= 2 && ctx.dirty {
        return Some("Multiple sessions in the same dirty workspace. Consider separate worktrees.".into());
    }
    if !ctx.is_worktree && ctx.dirty && ctx.branch.as_deref() != Some("main") {
        return Some("Implementing outside main. A worktree may be safer.".into());
    }
    None
}
```

Then update `list_sessions` to compute conflicts and recommendations, and map them into the SessionInfo:

After computing `infos`, before `Json(infos)`:
```rust
let conflict_warnings = detect_conflicts(&infos);
let mut cwd_counts: HashMap<&str, usize> = HashMap::new();
for info in &infos {
    *cwd_counts.entry(&info.cwd).or_default() += 1;
}
for info in &mut infos {
    let ctx = git::detect(&PathBuf::from(&info.cwd));
    info.repo_root = ctx.repo_root;
    info.branch = ctx.branch;
    info.dirty = Some(ctx.dirty);
    info.changed_files = Some(ctx.changed_files);
    info.is_worktree = Some(ctx.is_worktree);
    info.conflict_warning = conflict_warnings.iter()
        .find(|(id, _)| id == &info.id)
        .map(|(_, w)| w.clone());
    info.recommendation = recommendation(&ctx, cwd_counts.get(info.cwd.as_str()).copied().unwrap_or(1));
}
```

Add `use std::collections::HashMap;` to imports.

- [ ] **Step 6: Build and test**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml && cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: builds clean, all tests pass (~23 tests)

- [ ] **Step 7: Add conflict detection test**

In the existing `mod tests` block, add:
```rust
#[test]
fn conflict_detection_two_active_sessions_same_dirty_cwd() {
    let mut sessions = vec![
        SessionInfo {
            id: "a".into(), label: "A".into(), status: "running".into(),
            cwd: "/tmp/shared".into(), created_at: "now".into(),
            metadata_source: None, metadata_confidence: None,
            repo_root: None, branch: None, dirty: Some(true),
            changed_files: Some(3), is_worktree: Some(false),
            conflict_warning: None, recommendation: None,
        },
        SessionInfo {
            id: "b".into(), label: "B".into(), status: "running".into(),
            cwd: "/tmp/shared".into(), created_at: "now".into(),
            metadata_source: None, metadata_confidence: None,
            repo_root: None, branch: None, dirty: Some(true),
            changed_files: Some(5), is_worktree: Some(false),
            conflict_warning: None, recommendation: None,
        },
    ];
    let warnings = detect_conflicts(&sessions);
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].1.contains("2 sessions"));
}
```

Move `detect_conflicts` and `recommendation` functions above the test module (they need to be in scope).

- [ ] **Step 8: Commit**

```bash
git add crates/orkworksd/src/main.rs
git commit -m "feat: wire git2 context into sessions, add conflict detection and recommendations"
```

---

### Task 4: Add git fields to SessionMetadata

**Files:**
- Modify: `crates/orkworksd/src/metadata.rs`

- [ ] **Step 1: Add git fields to SessionMetadata**

Add these fields to `SessionMetadata`:
```rust
#[serde(rename = "repoRoot")]
pub repo_root: Option<String>,
pub branch: Option<String>,
pub dirty: Option<bool>,
#[serde(rename = "changedFiles")]
pub changed_files: Option<usize>,
#[serde(rename = "isWorktree")]
pub is_worktree: Option<bool>,
```

- [ ] **Step 2: Update create_session metadata write in main.rs**

In `create_session`, when writing metadata, include git fields:
```rust
let git_ctx = git::detect(&ws_path);
ws.metadata.write_session(&metadata::SessionMetadata {
    // ... existing fields ...
    repo_root: git_ctx.repo_root,
    branch: git_ctx.branch,
    dirty: Some(git_ctx.dirty),
    changed_files: Some(git_ctx.changed_files),
    is_worktree: Some(git_ctx.is_worktree),
});
```

- [ ] **Step 3: Update tests for new fields**

In `metadata.rs` tests, update `SessionMetadata` construction to include:
```rust
repo_root: Some("/tmp".into()),
branch: Some("main".into()),
dirty: Some(false),
changed_files: Some(0),
is_worktree: Some(false),
```

- [ ] **Step 4: Build and test**

Run: `cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add crates/orkworksd/src/metadata.rs crates/orkworksd/src/main.rs
git commit -m "feat: add git fields to session metadata"
```

---

### Task 5: Frontend — update API types

**Files:**
- Modify: `apps/desktop/src/api.ts`

- [ ] **Step 1: Add git fields to SessionInfo**

```typescript
export interface SessionInfo {
  id: string;
  label: string;
  status: string;
  cwd: string;
  created_at: string;
  metadataSource?: string;
  metadataConfidence?: number;
  repoRoot?: string;
  branch?: string;
  dirty?: boolean;
  changedFiles?: number;
  isWorktree?: boolean;
  conflictWarning?: string;
  recommendation?: string;
}
```

- [ ] **Step 2: TypeCheck**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/api.ts
git commit -m "feat: add git context fields to SessionInfo type"
```

---

### Task 6: Frontend — show git context in right sidebar detail

**Files:**
- Modify: `apps/desktop/src/components/RightSidebar.tsx`

- [ ] **Step 1: Add git context section to RightSidebar**

After the "Directory" section (after the closing `</div>` of `session-detail-section`), add:

```tsx
{active.branch && (
  <div className="session-detail-section">
    <div className="session-detail-label">Git</div>
    <div className="session-detail-value">
      {active.branch}
      {active.isWorktree && (
        <span style={{ color: "#4ec94e", marginLeft: 6, fontSize: 10 }}>worktree</span>
      )}
    </div>
    <div style={{ display: "flex", gap: 8, marginTop: 2, fontSize: 10 }}>
      <span style={{ color: active.dirty ? "#d4d44e" : "#4ec94e" }}>
        {active.dirty ? "dirty" : "clean"}
      </span>
      {active.changedFiles !== undefined && active.changedFiles > 0 && (
        <span style={{ color: "#858585" }}>{active.changedFiles} files changed</span>
      )}
    </div>
  </div>
)}
{active.conflictWarning && (
  <div className="session-detail-section">
    <div className="conflict-warning">&#x26A0; {active.conflictWarning}</div>
  </div>
)}
{active.recommendation && (
  <div className="session-detail-section">
    <div className="recommendation-text">{active.recommendation}</div>
  </div>
)}
```

- [ ] **Step 2: TypeCheck**

Run: `cd apps/desktop && npx tsc --noEmit`
Expected: no errors

- [ ] **Step 3: Commit**

```bash
git add apps/desktop/src/components/RightSidebar.tsx
git commit -m "feat: show git context in session detail panel"
```

---

### Task 7: Frontend — CSS for conflict warnings and recommendations

**Files:**
- Modify: `apps/desktop/src/App.css`

- [ ] **Step 1: Add styles**

Append to `apps/desktop/src/App.css`:

```css
.conflict-warning {
  background: #3a2a1a;
  border: 1px solid #5a3a1a;
  border-radius: 4px;
  padding: 6px 8px;
  font-size: 10px;
  color: #d4d44e;
  line-height: 1.4;
}

.recommendation-text {
  font-size: 11px;
  color: #858585;
  font-style: italic;
  line-height: 1.4;
}
```

- [ ] **Step 2: Commit**

```bash
git add apps/desktop/src/App.css
git commit -m "feat: add conflict warning and recommendation styles"
```

---

### Task 8: Integration test

**Files:**
- None (verification only)

- [ ] **Step 1: Full build**

Run: `cargo build --manifest-path crates/orkworksd/Cargo.toml && cargo test --manifest-path crates/orkworksd/Cargo.toml`
Expected: builds, all tests pass

- [ ] **Step 2: TypeCheck frontend**

Run: `cd apps/desktop && npx tsc --noEmit && node --test tests/*.test.ts`
Expected: no errors, all tests pass

- [ ] **Step 3: Commit and push**

```bash
git add -A && git commit -m "feat: M5 git context detection — integration complete" && git push
```
