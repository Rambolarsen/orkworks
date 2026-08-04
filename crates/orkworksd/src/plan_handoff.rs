use std::path::{Component, Path, PathBuf};

/// Verbs that indicate a line is reporting a file the agent just wrote,
/// rather than merely mentioning or quoting an existing path (e.g. a `grep`
/// hit, an error message, or prose referencing someone else's plan).
const WRITE_SIGNALS: [&str; 12] = [
    "wrote", "write", "writes", "writing", "written", "created", "create",
    "creates", "creating", "saved", "save", "saves",
];

fn has_write_signal(line: &str) -> bool {
    line.split(|ch: char| !ch.is_ascii_alphanumeric())
        .any(|word| WRITE_SIGNALS.contains(&word.to_ascii_lowercase().as_str()))
}

/// Returns the first Markdown path printed by an agent that is in one of the
/// narrow plan locations OrkWorks recognizes, on a line that also reports
/// having written the file. Validation against the actual workspace happens
/// before the value is persisted or served.
pub(crate) fn printed_plan_path(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        if !has_write_signal(line) {
            return None;
        }
        line.split_whitespace().find_map(|word| {
            let path = word.trim_matches(|ch: char| matches!(ch, '`' | '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | '.' | ':'));
            if Path::new(path)
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
            {
                return None;
            }
            (path.starts_with("docs/superpowers/plans/")
                || path.starts_with("docs/superpowers/specs/")
                || path.starts_with("specs/"))
                .then_some(path)
                .filter(|path| path.ends_with(".md") && !path.chars().any(char::is_control))
                .map(str::to_owned)
        })
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

pub(crate) fn normalize_reported_plan_path(
    workspace_root: &Path,
    reported_path: &str,
) -> Result<String, String> {
    if reported_path.chars().any(char::is_control) {
        return Err("plan path must not contain control characters".into());
    }
    let workspace = workspace_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let candidate = Path::new(reported_path)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let relative = candidate
        .strip_prefix(&workspace)
        .map_err(|_| "plan path is outside the workspace".to_string())?;
    if !candidate.is_file()
        || !candidate
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        || !(relative.starts_with("docs/superpowers/plans")
            || relative.starts_with("docs/superpowers/specs")
            || relative.starts_with("specs"))
    {
        return Err("plan path is not a supported plan or specification".into());
    }
    relative
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| "plan path is not valid UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::{normalize_reported_plan_path, printed_plan_path, resolve_openable_plan};
    use std::fs;

    #[test]
    fn accepts_workspace_relative_markdown_only() {
        let workspace = tempfile::tempdir().unwrap();
        fs::create_dir(workspace.path().join("docs")).unwrap();
        fs::write(workspace.path().join("docs/plan.MD"), "# plan").unwrap();
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
    fn normalizes_a_hook_reported_absolute_plan_path() {
        let workspace = tempfile::tempdir().unwrap();
        let plan_dir = workspace.path().join("docs/superpowers/plans");
        fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("session.md");
        fs::write(&plan, "# plan").unwrap();

        assert_eq!(
            normalize_reported_plan_path(workspace.path(), plan.to_str().unwrap()).unwrap(),
            "docs/superpowers/plans/session.md"
        );
    }

    #[test]
    fn finds_a_printed_plan_under_an_allowed_root() {
        assert_eq!(
            printed_plan_path("Plan written to `docs/superpowers/plans/session-review.md`."),
            Some("docs/superpowers/plans/session-review.md".into())
        );
        assert_eq!(
            printed_plan_path("Wrote docs/superpowers/plans/session-review.md"),
            Some("docs/superpowers/plans/session-review.md".into())
        );
        assert_eq!(
            printed_plan_path("Created specs/new-feature.md"),
            Some("specs/new-feature.md".into())
        );
        assert_eq!(
            printed_plan_path(
                "Spec written and committed: docs/superpowers/specs/2026-08-03-recorded-terminal-size-cue-design.md."
            ),
            Some("docs/superpowers/specs/2026-08-03-recorded-terminal-size-cue-design.md".into())
        );
    }

    #[test]
    fn ignores_an_incidental_mention_with_no_write_signal() {
        // A session reading or referencing someone else's plan/spec must not
        // be mistaken for having authored it (the root cause of a real false
        // positive: OrkWorks itself discussing specs/session-plan-review.md).
        assert_eq!(
            printed_plan_path("See specs/session-plan-review.md before continuing."),
            None
        );
        assert_eq!(
            printed_plan_path("grep hit: docs/superpowers/plans/unrelated.md:12:some text"),
            None
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
