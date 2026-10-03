use std::path::Path;

/// Net text-line changes in the current worktree relative to HEAD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct LineChanges {
    pub additions: usize,
    pub deletions: usize,
}

#[derive(Debug, Clone)]
pub struct GitContext {
    pub repo_root: Option<String>,
    pub branch: Option<String>,
    pub dirty: bool,
    pub changed_files: usize,
    pub line_changes: Option<LineChanges>,
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
                line_changes: None,
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
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .include_ignored(false);
    if let Ok(statuses) = repo.statuses(Some(&mut opts)) {
        changed_files = statuses.len();
        dirty = changed_files > 0;
    }

    GitContext {
        repo_root,
        branch,
        dirty,
        changed_files,
        line_changes: uncommitted_line_changes(&repo).ok(),
        is_worktree,
    }
}

/// Discover the worktree root, so subdirectories share one diff per listing.
pub fn worktree_root(cwd: &Path) -> Option<std::path::PathBuf> {
    let repo = git2::Repository::discover(cwd).ok()?;
    repo.workdir().map(Path::to_path_buf)
}

fn uncommitted_line_changes(repo: &git2::Repository) -> Result<LineChanges, git2::Error> {
    let tree = match repo.head() {
        Ok(head) => Some(head.peel_to_tree()?),
        Err(error) if error.code() == git2::ErrorCode::UnbornBranch => None,
        Err(error) => return Err(error),
    };
    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true);
    let diff = repo.diff_tree_to_workdir_with_index(tree.as_ref(), Some(&mut opts))?;
    let stats = diff.stats()?;
    Ok(LineChanges {
        additions: stats.insertions(),
        deletions: stats.deletions(),
    })
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

    fn projected_git(dir: &Path) -> serde_json::Value {
        let mut meta = crate::test_support::test_session_metadata(
            "git-session",
            "Git session",
            &dir.display().to_string(),
            "running",
            "2026-10-03T10:00:00Z",
            "2026-10-03T10:00:00Z",
        );
        meta.cwd = dir.display().to_string();
        let mut infos = vec![crate::session_types::SessionInfo::baseline_from_metadata(
            &meta,
        )];
        crate::session_projection::enrich_sessions_with_git_context(
            &mut infos,
            &Default::default(),
            detect,
        );
        serde_json::to_value(&infos[0]).unwrap()
    }

    #[test]
    fn uncommitted_lines_combine_staged_and_unstaged_changes_against_head() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        init_repo(dir);
        std::fs::write(dir.join("tracked.txt"), "one\ntwo\nthree\n").unwrap();
        git(dir, &["add", "."]);
        git(dir, &["commit", "-qm", "baseline"]);
        std::fs::write(dir.join("tracked.txt"), "one\nstaged\nthree\n").unwrap();
        git(dir, &["add", "."]);
        std::fs::write(dir.join("tracked.txt"), "one\nfinal\nthree\nextra\n").unwrap();
        let info = projected_git(dir);
        assert_eq!(info["changedFiles"], 1);
        assert_eq!(
            info["lineChanges"],
            serde_json::json!({"additions": 2, "deletions": 1})
        );
        // Undoing an indexed edit in the worktree has no net line changes.
        std::fs::write(dir.join("tracked.txt"), "one\ntwo\nthree\n").unwrap();
        assert_eq!(
            projected_git(dir)["lineChanges"],
            serde_json::json!({"additions": 0, "deletions": 0})
        );
    }

    #[test]
    fn uncommitted_lines_include_staged_deletes_and_nested_new_files_but_not_ignored_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        init_repo(dir);
        std::fs::write(dir.join("deleted.txt"), "first\nsecond\n").unwrap();
        std::fs::write(dir.join(".gitignore"), "ignored/\n").unwrap();
        git(dir, &["add", "."]);
        git(dir, &["commit", "-qm", "baseline"]);
        git(dir, &["rm", "deleted.txt"]);
        std::fs::create_dir(dir.join("new")).unwrap();
        std::fs::write(dir.join("new/a.txt"), "one\ntwo\nthree").unwrap();
        std::fs::write(dir.join("new/b.txt"), "four\n").unwrap();
        std::fs::create_dir(dir.join("ignored")).unwrap();
        std::fs::write(dir.join("ignored/large.txt"), "ignore me\n").unwrap();
        let info = projected_git(dir);
        assert_eq!(info["changedFiles"], 3);
        assert_eq!(
            info["lineChanges"],
            serde_json::json!({"additions": 4, "deletions": 2})
        );
    }

    #[test]
    fn uncommitted_lines_before_first_commit_use_empty_tree_and_skip_binary_lines() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        git(dir, &["init", "-q"]);
        std::fs::write(dir.join("staged.txt"), "first\nsecond\n").unwrap();
        git(dir, &["add", "staged.txt"]);
        std::fs::write(dir.join("new.txt"), "third\n").unwrap();
        std::fs::write(dir.join("image.bin"), b"\0\x01\n\x02\n").unwrap();
        let info = projected_git(dir);
        assert_eq!(info["changedFiles"], 3);
        assert_eq!(
            info["lineChanges"],
            serde_json::json!({"additions": 3, "deletions": 0})
        );
    }

    #[test]
    fn uncommitted_lines_clean_repo_is_zero_and_no_repo_is_unavailable() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(projected_git(tmp.path()).get("lineChanges").is_none());
        init_repo(tmp.path());
        assert_eq!(
            projected_git(tmp.path())["lineChanges"],
            serde_json::json!({"additions": 0, "deletions": 0})
        );
    }

    #[test]
    fn uncommitted_lines_are_scoped_to_each_linked_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let main = tmp.path().join("main");
        std::fs::create_dir(&main).unwrap();
        init_repo(&main);
        let wt = tmp.path().join("wt");
        git(
            &main,
            &[
                "worktree",
                "add",
                "-q",
                wt.to_str().unwrap(),
                "-b",
                "counter",
            ],
        );
        std::fs::write(wt.join("new.txt"), "one\ntwo\n").unwrap();
        assert_eq!(
            projected_git(&main)["lineChanges"],
            serde_json::json!({"additions": 0, "deletions": 0})
        );
        assert_eq!(
            projected_git(&wt)["lineChanges"],
            serde_json::json!({"additions": 2, "deletions": 0})
        );
    }

    #[test]
    fn uncommitted_lines_with_unreadable_head_are_unavailable_not_empty_tree() {
        let tmp = tempfile::tempdir().unwrap();
        init_repo(tmp.path());
        let repo = git2::Repository::open(tmp.path()).unwrap();
        let head_name = repo.head().unwrap().name().unwrap().to_string();
        std::fs::write(
            repo.path().join(head_name),
            "1111111111111111111111111111111111111111\n",
        )
        .unwrap();
        std::fs::write(tmp.path().join("new.txt"), "new\n").unwrap();
        assert!(projected_git(tmp.path()).get("lineChanges").is_none());
    }

    #[test]
    fn uncommitted_lines_share_one_diff_across_subdirectories_and_refresh_next_listing() {
        let tmp = tempfile::tempdir().unwrap();
        init_repo(tmp.path());
        let child = tmp.path().join("child");
        std::fs::create_dir(&child).unwrap();
        std::fs::write(child.join("new.txt"), "first\n").unwrap();
        let mut infos = vec![
            crate::test_support::test_session_info(
                "one",
                "One",
                tmp.path().display().to_string(),
                "running",
                "now",
            ),
            crate::test_support::test_session_info(
                "two",
                "Two",
                child.display().to_string(),
                "running",
                "now",
            ),
        ];
        let mut calls = 0;
        crate::session_projection::enrich_sessions_with_git_context(
            &mut infos,
            &Default::default(),
            |cwd| {
                calls += 1;
                detect(cwd)
            },
        );
        assert_eq!(calls, 1);
        assert_eq!(
            serde_json::to_value(&infos[0]).unwrap()["lineChanges"],
            serde_json::json!({"additions": 1, "deletions": 0})
        );
        assert_eq!(
            serde_json::to_value(&infos[1]).unwrap()["lineChanges"],
            serde_json::json!({"additions": 1, "deletions": 0})
        );
        std::fs::write(child.join("new.txt"), "first\nsecond\n").unwrap();
        crate::session_projection::enrich_sessions_with_git_context(
            &mut infos,
            &Default::default(),
            detect,
        );
        assert_eq!(
            serde_json::to_value(&infos[1]).unwrap()["lineChanges"],
            serde_json::json!({"additions": 2, "deletions": 0})
        );
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
