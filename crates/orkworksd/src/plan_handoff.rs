use std::path::{Path, PathBuf};

pub(crate) fn resolve_openable_plan(workspace_root: &Path, relative_path: &str) -> Result<PathBuf, String> {
    let relative = Path::new(relative_path);
    if relative.is_absolute() {
        return Err("plan path must be workspace-relative".into());
    }
    let workspace = workspace_root.canonicalize().map_err(|error| error.to_string())?;
    let candidate = workspace.join(relative).canonicalize().map_err(|error| error.to_string())?;
    if !candidate.starts_with(&workspace) || !candidate.is_file()
        || !candidate.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("md")) {
        return Err("plan path is not an openable workspace Markdown file".into());
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::resolve_openable_plan;
    use std::fs;

    #[test]
    fn accepts_workspace_relative_markdown_only() {
        let workspace = tempfile::tempdir().unwrap();
        fs::create_dir(workspace.path().join("docs")).unwrap();
        fs::write(workspace.path().join("docs/plan.MD"), "# plan").unwrap();
        fs::write(workspace.path().join("docs/notes.txt"), "notes").unwrap();

        assert!(resolve_openable_plan(workspace.path(), "docs/plan.MD").is_ok());
        assert!(resolve_openable_plan(workspace.path(), workspace.path().join("docs/plan.MD").to_str().unwrap()).is_err());
        assert!(resolve_openable_plan(workspace.path(), "../outside.md").is_err());
        assert!(resolve_openable_plan(workspace.path(), "docs/missing.md").is_err());
        assert!(resolve_openable_plan(workspace.path(), "docs/notes.txt").is_err());
        assert!(resolve_openable_plan(workspace.path(), "docs").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_workspace_symlink_that_escapes_the_workspace() {
        use std::os::unix::fs::symlink;

        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(workspace.path().join("docs")).unwrap();
        let outside_plan = outside.path().join("outside.md");
        fs::write(&outside_plan, "# outside").unwrap();
        symlink(&outside_plan, workspace.path().join("docs/escaped.md")).unwrap();

        assert!(resolve_openable_plan(workspace.path(), "docs/escaped.md").is_err());
    }
}
