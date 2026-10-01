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

    // `Repository::is_worktree` checks real worktree linkage; a `.git` file
    // alone is not proof, since submodule checkouts have one too.
    let is_worktree = repo.is_worktree();

    let mut changed_files = 0;
    let mut dirty = false;

    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true).include_ignored(false);
    if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
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

    fn git(dir: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(["-c", "protocol.file.allow=always"])
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(out.status.success(), "git {args:?}: {out:?}");
    }

    fn init_repo(dir: &Path) {
        git(dir, &["init", "-q"]);
        git(dir, &["commit", "-q", "--allow-empty", "-m", "init"]);
    }

    #[test]
    fn linked_worktree_is_detected_as_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let main = tmp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        init_repo(&main);
        let wt = tmp.path().join("wt");
        git(
            &main,
            &["worktree", "add", "-q", wt.to_str().unwrap(), "-b", "wt"],
        );

        assert!(detect(&wt).is_worktree);
        assert!(!detect(&main).is_worktree);
    }

    #[test]
    fn submodule_checkout_is_not_a_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("sub_repo");
        let main = tmp.path().join("main_repo");
        std::fs::create_dir(&sub).unwrap();
        std::fs::create_dir(&main).unwrap();
        init_repo(&sub);
        init_repo(&main);
        git(
            &main,
            &["submodule", "add", "-q", sub.to_str().unwrap(), "sub"],
        );
        assert!(main.join("sub/.git").is_file());

        assert!(!detect(&main.join("sub")).is_worktree);
    }
}
