use std::path::{Component, Path, PathBuf};

/// Returns the first Markdown path printed by an agent that is in one of the
/// narrow plan locations OrkWorks recognizes. Validation against the actual
/// workspace happens before the value is persisted or served.
pub(crate) fn printed_plan_path(output: &str) -> Option<String> {
    output.split_whitespace().find_map(|word| {
        let path = word.trim_matches(|ch: char| matches!(ch, '`' | '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | '.' | ':'));
        if Path::new(path)
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
        {
            return None;
        }
        (path.starts_with("docs/superpowers/plans/") || path.starts_with("specs/"))
            .then_some(path)
            .filter(|path| path.ends_with(".md") && !path.chars().any(char::is_control))
            .map(str::to_owned)
    })
}

pub(crate) fn resolve_openable_plan(
    workspace_root: &Path,
    relative_path: &str,
) -> Result<PathBuf, String> {
    if relative_path.chars().any(char::is_control) {
        return Err("plan path must not contain control characters".into());
    }
    let relative = Path::new(relative_path);
    if relative.is_absolute() {
        return Err("plan path must be workspace-relative".into());
    }
    let workspace = workspace_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let candidate = workspace
        .join(relative)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !candidate.starts_with(&workspace)
        || !candidate.is_file()
        || !candidate
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return Err("plan path is not an openable workspace Markdown file".into());
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    use super::{printed_plan_path, resolve_openable_plan};
    use std::fs;

    #[test]
    fn accepts_workspace_relative_markdown_only() {
        let workspace = tempfile::tempdir().unwrap();
        fs::create_dir(workspace.path().join("docs")).unwrap();
        fs::write(workspace.path().join("docs/plan.MD"), "# plan").unwrap();
        fs::create_dir(workspace.path().join("docs\nignored")).unwrap();
        fs::write(workspace.path().join("docs\nignored/plan.MD"), "# injected").unwrap();
        fs::write(workspace.path().join("docs/notes.txt"), "notes").unwrap();

        assert!(resolve_openable_plan(workspace.path(), "docs/plan.MD").is_ok());
        assert!(resolve_openable_plan(
            workspace.path(),
            workspace.path().join("docs/plan.MD").to_str().unwrap()
        )
        .is_err());
        assert!(resolve_openable_plan(workspace.path(), "../outside.md").is_err());
        assert!(resolve_openable_plan(workspace.path(), "docs/missing.md").is_err());
        assert!(resolve_openable_plan(workspace.path(), "docs/notes.txt").is_err());
        assert!(resolve_openable_plan(workspace.path(), "docs").is_err());
        assert!(resolve_openable_plan(workspace.path(), "docs\nignored/plan.MD").is_err());
    }

    #[test]
    fn finds_a_printed_plan_under_an_allowed_root() {
        assert_eq!(
            printed_plan_path("Plan written to `docs/superpowers/plans/session-review.md`."),
            Some("docs/superpowers/plans/session-review.md".into())
        );
        assert_eq!(
            printed_plan_path("See specs/session-plan-review.md before continuing."),
            Some("specs/session-plan-review.md".into())
        );
    }

    #[test]
    fn ignores_printed_markdown_outside_plan_roots() {
        assert_eq!(printed_plan_path("Read docs/readme.md"), None);
        assert_eq!(printed_plan_path("Read ../specs/escape.md"), None);
        assert_eq!(printed_plan_path("Read docs/superpowers/plans/not-a-plan.txt"), None);
        assert_eq!(printed_plan_path("Read specs/../README.md"), None);
        assert_eq!(printed_plan_path("Read docs/superpowers/plans/../../README.md"), None);
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
