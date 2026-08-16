use std::path::{Component, Path, PathBuf};

use crate::metadata::PlanReference;

fn git_common_dir(path: &Path) -> Result<PathBuf, String> {
    let repo = git2::Repository::discover(path).map_err(|error| error.to_string())?;
    let git_dir = repo.path().canonicalize().map_err(|error| error.to_string())?;
    if repo.is_worktree() {
        let worktrees = git_dir.parent().ok_or("linked worktree has no parent")?;
        if worktrees.file_name().is_some_and(|name| name == "worktrees") {
            return worktrees.parent().map(Path::to_path_buf).ok_or_else(|| "linked worktree has no common git directory".into());
        }
    }
    Ok(git_dir)
}

fn is_supported_plan_path(relative: &Path) -> bool {
    relative.starts_with("docs/superpowers/plans")
        || relative.starts_with("docs/superpowers/specs")
        || relative.starts_with("specs")
}

pub(crate) fn resolve_printed_plan_path(launch_root: &Path, printed_path: &str) -> Result<(PathBuf, String), String> {
    if printed_path.chars().any(char::is_control) { return Err("plan path must not contain control characters".into()); }
    let printed = Path::new(printed_path);
    let launch_root = launch_root.canonicalize().map_err(|error| error.to_string())?;
    let candidate = if printed.is_absolute() { printed.to_path_buf() } else {
        if printed.components().any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_))) { return Err("relative plan path must not escape launch worktree".into()); }
        launch_root.join(printed)
    }.canonicalize().map_err(|error| error.to_string())?;
    let candidate_repo = git_common_dir(&candidate)?;
    if candidate_repo != git_common_dir(&launch_root)? { return Err("plan path is outside the session repository worktree family".into()); }
    let root = git2::Repository::discover(&candidate).map_err(|error| error.to_string())?.workdir().ok_or("plan repository is bare")?.canonicalize().map_err(|error| error.to_string())?;
    let relative = candidate.strip_prefix(&root).map_err(|_| "plan path is outside its worktree".to_string())?;
    if !candidate.is_file() || !candidate.extension().is_some_and(|extension| extension.eq_ignore_ascii_case("md")) || !is_supported_plan_path(relative) { return Err("plan path is not a supported plan or specification".into()); }
    Ok((root, relative.to_str().ok_or("plan path is not valid UTF-8")?.to_owned()))
}

/// Verbs that indicate a line is reporting a file the agent just wrote,
/// rather than merely mentioning or quoting an existing path (e.g. a `grep`
/// hit, an error message, or prose referencing someone else's plan).
const WRITE_SIGNALS: [&str; 12] = [
    "wrote", "write", "writes", "writing", "written", "created", "create",
    "creates", "creating", "saved", "save", "saves",
];

/// Maximum token index at which a write-signal verb counts as the
/// "sentence-initial authorship report" shape: `Wrote <path>`,
/// `Created <path>`, `Plan written to <path>`, `Spec written and
/// committed: <path>` — the verb sits at the first or second token of
/// the line.
const WRITE_VERB_LINE_PREFIX: usize = 1;
/// Maximum gap between a sentence-initial write-signal verb and the path
/// token that follows it (e.g. the `and committed:` clause between
/// `written` and the path in `Spec written and committed: <path>`).
const WRITE_VERB_PROXIMITY: usize = 3;
/// Maximum gap allowed between a write-signal verb and the path when the
/// verb is *not* sentence-initial: the verb must sit immediately adjacent
/// to the path, so `A model writes <path>` still scores, while
/// `the spec writes <…long body…> <path>` does not.
const WRITE_VERB_ADJACENCY: usize = 1;

fn trim_plan_token(word: &str) -> &str {
    word.trim_matches(|ch: char| matches!(ch, '`' | '\'' | '"' | '(' | ')' | '[' | ']' | '{' | '}' | ',' | '.' | ':'))
}

/// Returns the first Markdown path printed by an agent that sits under one
/// of the narrow plan locations OrkWorks recognizes, on a line that
/// *reports* having written the file. A write-signal verb qualifies only
/// if it sits at the start of the line (within `WRITE_VERB_LINE_PREFIX`
/// tokens) and the path follows within `WRITE_VERB_PROXIMITY` tokens of it
/// — covering `Wrote <path>`, `Created <path>`, `Plan written to <path>`,
/// and `Spec written and committed: <path>` — or if the write-signal verb
/// is immediately adjacent to the path anywhere on the line (within
/// `WRITE_VERB_ADJACENCY` tokens). A write-signal verb anywhere else on
/// the line is *not* treated as authorship, so prose that merely discusses
/// a plan path while its sentence also happens to contain
/// "writes"/"writing"/"saved" no longer attaches the path as
/// `source: terminal_fallback` authorship, honoring the spec's
/// "intentionally favors missing the card over misattributing an unrelated
/// document" contract. Validation against the actual workspace happens
/// before the value is persisted or served; this fallback also yields to
/// hook-reported `planPath` (ADR 0037/0038) and to user-approved
/// terminal-link selections.
pub(crate) fn printed_plan_path(output: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }
        // Pre-compute the token positions of write-signal verbs on this
        // line so the proximity / adjacency gates can scan the line once
        // and rank paths against authorship-shaped verbs, catching prose
        // (where the verb is far from the path) without re-tokenizing.
        let write_verb_positions: Vec<usize> = tokens
            .iter()
            .enumerate()
            .filter_map(|(i, tok)| {
                WRITE_SIGNALS
                    .contains(&trim_plan_token(tok).to_ascii_lowercase().as_str())
                    .then_some(i)
            })
            .collect();
        if write_verb_positions.is_empty() {
            return None;
        }
        tokens.iter().enumerate().find_map(|(i, word)| {
            let path = trim_plan_token(word);
            if Path::new(path)
                .components()
                .any(|component| matches!(component, Component::ParentDir | Component::RootDir | Component::Prefix(_)))
            {
                return None;
            }
            if !(path.starts_with("docs/superpowers/plans/")
                || path.starts_with("docs/superpowers/specs/")
                || path.starts_with("specs/"))
                || !path.ends_with(".md")
                || path.chars().any(char::is_control)
            {
                return None;
            }
            let verb_precedes_within_proximity = write_verb_positions
                .iter()
                .any(|&vp| vp <= WRITE_VERB_LINE_PREFIX && vp < i && i - vp <= WRITE_VERB_PROXIMITY);
            let verb_adjacent_to_path = write_verb_positions
                .iter()
                .any(|&vp| vp < i && i - vp <= WRITE_VERB_ADJACENCY);
            (verb_precedes_within_proximity || verb_adjacent_to_path)
                .then_some(path.to_owned())
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

/// Resolves a persisted plan reference. New references retain the worktree
/// that produced the path; older string-only references deliberately retain
/// the historical workspace-root behavior.
pub(crate) fn resolve_openable_plan_reference(
    session_workspace_root: &Path,
    reference: &PlanReference,
) -> Result<PathBuf, String> {
    let anchor = match reference.worktree_root.as_deref() {
        Some(root) => Path::new(root),
        None => return resolve_openable_plan(session_workspace_root, &reference.relative_path),
    };
    if reference.relative_path.chars().any(char::is_control) {
        return Err("plan path must not contain control characters".into());
    }
    let relative = Path::new(&reference.relative_path);
    if relative.is_absolute()
        || relative.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err("plan path must be relative to its worktree".into());
    }

    let anchor = anchor.canonicalize().map_err(|error| error.to_string())?;
    let anchor_repo = git2::Repository::discover(&anchor).map_err(|error| error.to_string())?;
    let worktree_root = anchor_repo
        .workdir()
        .ok_or("plan repository is bare")?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if anchor != worktree_root {
        return Err("plan worktree anchor is not a repository root".into());
    }
    if git_common_dir(&worktree_root)? != git_common_dir(session_workspace_root)? {
        return Err("plan worktree is outside the session repository family".into());
    }

    let candidate = worktree_root
        .join(relative)
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if !candidate.starts_with(&worktree_root)
        || !candidate.is_file()
        || !candidate
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        || !is_supported_plan_path(relative)
    {
        return Err("plan path is not an openable worktree Markdown file".into());
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
    let reported = Path::new(reported_path);
    // Reject lexical escape vectors on relative inputs. ParentDir, RootDir,
    // or drive Prefix components would let a relative hook input diverge
    // from the workspace anchor or escalate across drives before
    // canonicalization runs.
    if !reported.is_absolute() {
        for component in reported.components() {
            if matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            ) {
                return Err("relative plan path must not escape the workspace".into());
            }
        }
    }
    let workspace = workspace_root
        .canonicalize()
        .map_err(|error| error.to_string())?;
    // Anchor relative inputs against the canonical workspace so they do
    // not silently resolve against orkworksd's process CWD.
    let lexical_anchored = if reported.is_absolute() {
        reported.to_path_buf()
    } else {
        workspace.join(reported)
    };
    let candidate = lexical_anchored
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let relative = candidate
        .strip_prefix(&workspace)
        .map_err(|_| "plan path is outside the workspace".to_string())?;
    if !candidate.is_file()
        || !candidate
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        || !is_supported_plan_path(relative)
    {
        return Err("plan path is not a supported plan or specification".into());
    }
    // Reject in-workspace symlink pivots into an allowed root. For
    // absolute inputs, strip the lexical anchored path against the
    // uncanonical workspace_root (the syntactic path the workspace was
    // opened with) so legitimate symlinked workspace prefixes — such as
    // macOS `/var` -> `/private/var` — keep working. For relative
    // inputs, lexical_anchored is workspace-canonical.join(reported), so
    // strip against the canonicalized workspace. When the lexical strip
    // succeeds, the result must equal the canonical relative form; a
    // mismatch means a symlink crossed between roots and the session did
    // not directly author the recorded plan/spec.
    let lexical_relative = if reported.is_absolute() {
        lexical_anchored.strip_prefix(workspace_root).ok()
    } else {
        lexical_anchored.strip_prefix(&workspace).ok()
    };
    if let Some(lexical_relative) = lexical_relative {
        if lexical_relative != relative {
            return Err("plan path crosses a symlink into an allowed root".into());
        }
    }
    relative
        .to_str()
        .map(str::to_owned)
        .ok_or_else(|| "plan path is not valid UTF-8".into())
}

#[cfg(test)]
mod tests {
    use super::{normalize_reported_plan_path, printed_plan_path, resolve_openable_plan, resolve_openable_plan_reference, resolve_printed_plan_path};
    use crate::metadata::{PlanReference, PlanSource};
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
    fn anchors_a_workspace_relative_hook_path_against_the_workspace_root() {
        let workspace = tempfile::tempdir().unwrap();
        let plan_dir = workspace.path().join("docs/superpowers/plans");
        fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("relative.md");
        fs::write(&plan, "# plan").unwrap();

        // A relative hook input must resolve against the workspace, not
        // orkworksd's process CWD. Switch CWD to a sibling of the workspace
        // so a CWD-anchored resolve would land outside the workspace.
        let sibling = tempfile::tempdir().unwrap();
        std::env::set_current_dir(sibling.path()).unwrap();
        assert_eq!(
            normalize_reported_plan_path(
                workspace.path(),
                "docs/superpowers/plans/relative.md"
            )
            .unwrap(),
            "docs/superpowers/plans/relative.md"
        );
    }

    #[test]
    fn rejects_a_relative_hook_path_with_parent_dir_components() {
        let workspace = tempfile::tempdir().unwrap();
        let plan_dir = workspace.path().join("docs/superpowers/plans");
        fs::create_dir_all(&plan_dir).unwrap();
        fs::write(plan_dir.join("plan.md"), "# plan").unwrap();

        let outside = tempfile::tempdir().unwrap();
        let outside_plan = outside.path().join("outside.md");
        fs::write(&outside_plan, "# outside").unwrap();

        // Even if the canonicalized target lands inside the workspace,
        // lexical escape via `..` must be rejected up front.
        assert!(normalize_reported_plan_path(
            workspace.path(),
            "../outside.md"
        )
        .is_err());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_an_inbound_symlink_from_a_non_allowed_root_into_an_allowed_root() {
        use std::os::unix::fs::symlink;

        let workspace = tempfile::tempdir().unwrap();
        let plan_dir = workspace.path().join("docs/superpowers/plans");
        fs::create_dir_all(&plan_dir).unwrap();
        let target = plan_dir.join("real.md");
        fs::write(&target, "# plan").unwrap();

        let notes_dir = workspace.path().join("notes");
        fs::create_dir_all(&notes_dir).unwrap();
        // A session edited `notes/current.md`, but the file is a symlink
        // into the allowed plan root; without the lexical check, the
        // canonical form would be recorded as if the session had authored
        // `docs/superpowers/plans/real.md` directly.
        symlink(&target, notes_dir.join("current.md")).unwrap();

        assert!(normalize_reported_plan_path(
            workspace.path(),
            notes_dir.join("current.md").to_str().unwrap()
        )
        .is_err());
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
    fn resolves_relative_terminal_link_against_launch_worktree() {
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        let plan_dir = workspace.path().join("specs");
        fs::create_dir_all(&plan_dir).unwrap();
        fs::write(plan_dir.join("plan.md"), "# plan").unwrap();
        let (root, relative) = resolve_printed_plan_path(workspace.path(), "specs/plan.md").unwrap();
        assert_eq!(root, workspace.path().canonicalize().unwrap());
        assert_eq!(relative, "specs/plan.md");
    }

    #[test]
    fn resolves_an_anchored_plan_reference_from_its_recorded_worktree() {
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        let plan_dir = workspace.path().join("specs");
        fs::create_dir_all(&plan_dir).unwrap();
        let plan = plan_dir.join("selected.md");
        fs::write(&plan, "# plan").unwrap();

        let reference = PlanReference {
            worktree_root: Some(workspace.path().to_string_lossy().into_owned()),
            relative_path: "specs/selected.md".into(),
            source: PlanSource::UserSelected,
        };
        assert_eq!(
            resolve_openable_plan_reference(workspace.path(), &reference).unwrap(),
            plan.canonicalize().unwrap()
        );

        let escaped = PlanReference {
            relative_path: "../selected.md".into(),
            ..reference.clone()
        };
        assert!(resolve_openable_plan_reference(workspace.path(), &escaped).is_err());

        fs::write(workspace.path().join("notes.md"), "# notes").unwrap();
        let tampered = PlanReference {
            relative_path: "notes.md".into(),
            ..reference
        };
        assert!(resolve_openable_plan_reference(workspace.path(), &tampered).is_err());
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
    fn ignores_prose_that_quotes_a_write_verb_away_from_a_plan_path() {
        // Real false positive reported in the wild: a session reading the
        // session-plan-review spec aloud while its prose also contains a
        // write signal ("writes"/"writing"/"saved" etc., e.g. while quoting
        // the spec body that says "writes the file") must not be tagged as
        // having authored the cited plan. The bare anywhere-on-line
        // write-signal rule used to misattribute, attaching
        // `source: terminal_fallback` to sessions that were merely
        // *discussing* a plan path; the detector now requires the write
        // signal to sit close to the path (proximity/adjacency), so prose
        // far from the path is treated as background context, not authorship.
        assert_eq!(
            printed_plan_path(
                "The spec writes that OrkWorks recognizes a path at `specs/session-plan-review.md`."
            ),
            None
        );
        assert_eq!(
            printed_plan_path("We finished writing again about specs/some-other-doc.md"),
            None
        );
        assert_eq!(
            printed_plan_path(
                "Saved the previous draft earlier; specs/legacy.md remains open"
            ),
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
