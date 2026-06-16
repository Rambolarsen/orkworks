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

    let repo_root = repo.workdir().map(|p| p.display().to_string());

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
    fn detect_in_git_repo_has_repo_root_and_branch() {
        let ctx = detect(&std::env::current_dir().unwrap());
        assert!(ctx.repo_root.is_some());
        assert!(ctx.branch.is_some());
    }

    #[test]
    fn dirty_repo_has_changed_files() {
        let ctx = detect(&std::env::current_dir().unwrap());
        if ctx.dirty {
            assert!(ctx.changed_files > 0);
        }
    }
}
