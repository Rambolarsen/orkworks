use crate::taskmaster::runtime::ContextLevel;
use crate::taskmaster::RepositoryEvidence;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const MAX_FILES: usize = 32;
const MAX_FILE_BYTES: usize = 8 * 1024;
const MAX_TOTAL_BYTES: usize = 64 * 1024;
const MAX_DEPTH: usize = 5;
const MAX_PATHS: usize = 2_000;

/// Collects only existing, regular files whose canonical target remains below
/// the canonical workspace. It deliberately reports excerpts as evidence and
/// never infers a missing policy/file from a bounded traversal.
pub(crate) fn collect_repository_facts(
    workspace: &Path,
    level: ContextLevel,
    excluded_prefixes: &[String],
    observed_at: &str,
    signals: &str,
) -> Result<Vec<RepositoryEvidence>, String> {
    if level == ContextLevel::SessionObservations {
        return Ok(Vec::new());
    }
    let root = workspace
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let repository = git2::Repository::discover(&root).ok();
    let terms = relevance_terms(signals);
    let mut paths = Vec::new();
    let mut remaining_entries = MAX_PATHS;
    collect_paths(
        &root,
        &root,
        0,
        level,
        excluded_prefixes,
        repository.as_ref(),
        &mut paths,
        &mut remaining_entries,
        &terms,
        &mut true,
        &mut TraversalCoverage::default(),
    )?;
    paths.sort_by_cached_key(|path| {
        context_priority(path.strip_prefix(&root).unwrap_or(path), &terms)
    });

    let mut evidence = Vec::with_capacity(paths.len().min(MAX_FILES));
    let mut remaining = MAX_TOTAL_BYTES;
    for path in paths.into_iter().take(MAX_FILES) {
        if remaining == 0 {
            break;
        }
        let mut bytes = Vec::new();
        open_context_file(&path)
            .map_err(|error| error.to_string())?
            .take(MAX_FILE_BYTES as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() > MAX_FILE_BYTES
            || bytes.len() > remaining
            || std::str::from_utf8(&bytes).is_err()
        {
            continue;
        }
        let lower = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
        if [
            "private_key",
            "client_secret",
            "api_key",
            "apikey",
            "access_token",
            "-----begin",
        ]
        .iter()
        .any(|marker| lower.contains(marker))
        {
            continue;
        }
        let relative = path
            .strip_prefix(&root)
            .map_err(|_| "repository fact escaped workspace")?
            .to_string_lossy()
            .replace('\\', "/");
        let excerpt = relevant_excerpt(
            std::str::from_utf8(&bytes).map_err(|error| error.to_string())?,
            &terms,
        );
        remaining = remaining.saturating_sub(bytes.len());
        evidence.push(RepositoryEvidence {
            path: relative,
            sha256: hex::encode(Sha256::digest(&bytes)),
            excerpt,
            observed_at: observed_at.to_string(),
        });
    }
    Ok(evidence)
}

/// Both analysis modes filter before bounding; persisted evidence is untouched.
pub(super) fn recent_observations(
    observations: &[crate::workflow_observations::WorkflowObservation],
    limit: usize,
) -> impl Iterator<Item = &crate::workflow_observations::WorkflowObservation> {
    observations
        .iter()
        .rev()
        .filter(|item| item.evidence.chars().any(char::is_alphanumeric))
        .take(limit)
}

pub(super) fn observation_signals<'a>(
    observations: impl Iterator<Item = &'a crate::workflow_observations::WorkflowObservation>,
) -> String {
    observations
        .map(|item| {
            format!(
                "{} {}",
                item.description.chars().take(500).collect::<String>(),
                item.evidence.chars().take(1000).collect::<String>()
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn relevance_terms(signals: &str) -> Vec<String> {
    let mut terms = std::collections::BTreeSet::new();
    for term in signals
        .to_lowercase()
        .split(|character: char| !character.is_alphanumeric())
    {
        if term.chars().count() > 3
            && !matches!(
                term,
                "this"
                    | "that"
                    | "with"
                    | "from"
                    | "have"
                    | "were"
                    | "their"
                    | "there"
                    | "could"
                    | "should"
                    | "would"
            )
        {
            terms.insert(term.to_string());
            if terms.len() == 256 {
                break;
            }
        }
    }
    terms.into_iter().collect()
}

fn relevance_score(text: &str, terms: &[String]) -> usize {
    let text = text.to_lowercase();
    terms
        .iter()
        .filter(|term| text.contains(term.as_str()))
        .count()
}

/// Return a contiguous verbatim span, retaining the bounded read's identity.
/// Score lines so a useful section after a long introduction remains visible.
pub(super) fn relevant_excerpt(text: &str, terms: &[String]) -> String {
    let mut best_start = 0;
    let mut best_score = 0;
    let mut offset = 0;
    for line in text.split_inclusive('\n') {
        let score = relevance_score(line, terms);
        if score > best_score {
            best_score = score;
            best_start = offset;
        }
        offset += line.len();
    }
    text[best_start..].chars().take(2_000).collect()
}

pub(super) fn context_priority(
    path: &Path,
    terms: &[String],
) -> (u8, std::cmp::Reverse<usize>, PathBuf) {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let historical = path.components().any(|part| {
        matches!(
            part.as_os_str().to_str(),
            Some(".superpowers" | "superpowers" | "plans" | "archive" | "archives" | "historical")
        )
    });
    let score = relevance_score(&path.to_string_lossy(), terms);
    let priority = if historical {
        5
    } else if path.components().count() == 1
        && matches!(
            name,
            "AGENTS.md"
                | "CLAUDE.md"
                | "README.md"
                | "Cargo.toml"
                | "package.json"
                | "pyproject.toml"
                | "go.mod"
        )
    {
        0
    } else if path.components().count() == 1 && matches!(name, "docs" | "doc" | "documentation") {
        // Make linked project guidance discoverable before large source/skill
        // trees exhaust the unchanged traversal allowance.
        2
    } else if matches!(name, "AGENTS.md" | "CLAUDE.md") {
        1
    } else if matches!(
        name,
        "Cargo.toml" | "package.json" | "pyproject.toml" | "go.mod"
    ) {
        2
    } else if path.starts_with("docs")
        || path.starts_with("doc")
        || path.starts_with("documentation")
    {
        2 // Preserve current documentation in the bounded catalog ahead of generic skill trees.
    } else if score > 0
        || path.starts_with(".github")
        || path.starts_with("docs/agents")
        || path.starts_with("skills")
        || path.starts_with(".agents/skills")
    {
        3
    } else {
        4
    };
    (priority, std::cmp::Reverse(score), path.to_path_buf())
}

pub(super) fn open_context_file(path: &Path) -> std::io::Result<fs::File> {
    let file = open_without_links(path)?;
    if !file.metadata()?.is_file() {
        return Err(std::io::Error::other("context file is not regular"));
    }
    Ok(file)
}

#[cfg(unix)]
fn open_without_links(path: &Path) -> std::io::Result<fs::File> {
    use std::ffi::CString;
    use std::os::fd::{AsRawFd, FromRawFd};
    use std::os::unix::ffi::OsStrExt;
    use std::path::Component;

    if !path.is_absolute() {
        return Err(std::io::Error::other("context path must be absolute"));
    }
    let mut file = fs::File::open("/")?;
    let mut parts = path.components().peekable();
    while let Some(part) = parts.next() {
        let name = match part {
            Component::RootDir => continue,
            Component::Normal(name) => CString::new(name.as_bytes())?,
            _ => return Err(std::io::Error::other("invalid context path component")),
        };
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if parts.peek().is_some() {
                libc::O_DIRECTORY
            } else {
                0
            };
        // SAFETY: file owns a live directory descriptor and name is NUL-terminated.
        // Each next component is opened relative to that descriptor without links.
        let fd = unsafe { libc::openat(file.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: openat returned a new descriptor, transferred to exactly one File.
        file = unsafe { fs::File::from_raw_fd(fd) };
    }
    Ok(file)
}

#[cfg(windows)]
fn open_without_links(path: &Path) -> std::io::Result<fs::File> {
    use std::os::windows::fs::{MetadataExt, OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_SHARE_READ,
    };
    if !path.is_absolute() {
        return Err(std::io::Error::other("context path must be absolute"));
    }
    let mut parents = Vec::new();
    let mut current = PathBuf::new();
    for part in path.components() {
        current.push(part);
        if matches!(part, std::path::Component::Prefix(_)) {
            continue;
        }
        let file = fs::OpenOptions::new()
            .read(true)
            // Deny reparse mutation and deletion/rename while descendants open.
            .share_mode(FILE_SHARE_READ)
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT | FILE_FLAG_BACKUP_SEMANTICS)
            .open(&current)?;
        if file.metadata()?.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return Err(std::io::Error::other(
                "context path contains a reparse point",
            ));
        }
        parents.push(file);
    }
    parents
        .pop()
        .ok_or_else(|| std::io::Error::other("empty context path"))
}

#[cfg(not(any(unix, windows)))]
fn open_without_links(_: &Path) -> std::io::Result<fs::File> {
    Err(std::io::Error::other(
        "safe context collection is unsupported on this platform",
    ))
}

#[derive(Default, Clone)]
pub(super) struct TraversalCoverage {
    pub complete_directories: std::collections::HashSet<PathBuf>,
    pub instruction_entries: std::collections::HashSet<PathBuf>,
}

pub(super) fn collect_paths(
    root: &Path,
    directory: &Path,
    depth: usize,
    level: ContextLevel,
    excluded: &[String],
    repository: Option<&git2::Repository>,
    output: &mut Vec<PathBuf>,
    remaining_entries: &mut usize,
    terms: &[String],
    complete: &mut bool,
    coverage: &mut TraversalCoverage,
) -> Result<(), String> {
    if depth > MAX_DEPTH || output.len() >= MAX_PATHS {
        *complete = false;
        return Ok(());
    }
    let mut reader = fs::read_dir(directory).map_err(|error| error.to_string())?;
    let mut entries = reader
        .by_ref()
        .take(*remaining_entries)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    // An exactly full budget may still have enumerated the whole directory.
    // Peek only for exhaustion; never inspect or admit the extra entry.
    if reader.next().is_none() {
        coverage
            .complete_directories
            .insert(directory.to_path_buf());
    } else {
        *complete = false;
    }
    *remaining_entries = remaining_entries.saturating_sub(entries.len());
    entries.sort_by_cached_key(|entry| {
        context_priority(
            entry.path().strip_prefix(root).unwrap_or(&entry.path()),
            terms,
        )
    });
    for entry in entries {
        let path = entry.path();
        if matches!(entry.file_name().to_str(), Some("AGENTS.md" | "CLAUDE.md")) {
            coverage.instruction_entries.insert(path.clone());
        }
        let file_type = entry.file_type().map_err(|error| error.to_string())?;
        if file_type.is_symlink() {
            continue;
        }
        let relative = match path.strip_prefix(root) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let relative_text = relative.to_string_lossy().replace('\\', "/");
        if excluded
            .iter()
            .any(|prefix| path_is_excluded(&relative_text, prefix))
            || builtin_excluded(relative)
            || repository.is_some_and(|repository| {
                repository
                    .workdir()
                    // libgit2 may return a drive path while Rust canonicalization
                    // uses a verbatim Windows prefix. Compare the same spelling.
                    .and_then(|repo_root| repo_root.canonicalize().ok())
                    .and_then(|repo_root| path.strip_prefix(repo_root).ok())
                    .map(|repo_relative| {
                        repository
                            .status_should_ignore(repo_relative)
                            .unwrap_or(true)
                    })
                    .unwrap_or(true)
            })
        {
            continue;
        }
        if file_type.is_dir() {
            collect_paths(
                root,
                &path,
                depth + 1,
                level,
                excluded,
                repository,
                output,
                remaining_entries,
                terms,
                complete,
                coverage,
            )?;
        } else if file_type.is_file() && allowed_file(relative, level) {
            // Preserve the traversed path: canonicalizing here could conceal a
            // substituted symlink into an excluded directory. Opening checks
            // every original component without following links.
            output.push(path);
        }
    }
    Ok(())
}

pub(super) fn path_is_excluded(relative: &str, configured: &str) -> bool {
    let configured = configured.replace('\\', "/");
    #[cfg(windows)]
    let (relative, configured) = (relative.to_lowercase(), configured.to_lowercase());
    let configured = configured.trim().trim_matches('/');
    !configured.is_empty()
        && (relative == configured || relative.starts_with(&format!("{configured}/")))
}

/// Applies the collector's access policy to an immutable saved snapshot.
pub(super) fn fact_is_permitted(path: &str, level: ContextLevel, excluded: &[String]) -> bool {
    level != ContextLevel::SessionObservations
        && !Path::new(path).is_absolute()
        && !Path::new(path)
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        && !excluded.iter().any(|prefix| path_is_excluded(path, prefix))
        && !builtin_excluded(Path::new(path))
        && allowed_file(Path::new(path), level)
}

fn builtin_excluded(relative: &Path) -> bool {
    if relative.components().any(|part| {
        matches!(
            part.as_os_str()
                .to_string_lossy()
                .to_ascii_lowercase()
                .as_str(),
            ".git"
                | ".orkworks"
                | "node_modules"
                | "target"
                | "dist"
                | "build"
                | ".ssh"
                | ".aws"
                | ".azure"
                | ".gcloud"
                | ".codex"
                | ".claude"
        )
    }) {
        return true;
    }
    let name = relative
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    name == ".env"
        || name.starts_with(".env.")
        || name.contains("credential")
        || name.contains("secret")
        || name.ends_with(".pem")
        || name.ends_with(".key")
        || name.ends_with(".p12")
}

fn allowed_file(path: &Path, level: ContextLevel) -> bool {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if matches!(
        name,
        "AGENTS.md"
            | "CLAUDE.md"
            | "README.md"
            | "Cargo.toml"
            | "package.json"
            | "pyproject.toml"
            | "go.mod"
            | ".gitignore"
    ) {
        return true;
    }
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();
    if extension == "md"
        || (path.starts_with(".github/workflows") && matches!(extension, "yml" | "yaml"))
    {
        return true;
    }
    level == ContextLevel::SourceCode
        && matches!(
            extension,
            "rs" | "ts" | "tsx" | "js" | "jsx" | "py" | "go" | "java" | "kt" | "cs"
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::runtime::ContextLevel;

    #[cfg(unix)]
    #[test]
    fn context_open_rejects_file_and_parent_replaced_with_symlinks() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let root = root.path().canonicalize().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("README.md"), "outside data").unwrap();
        let directory = root.join("docs");
        fs::create_dir(&directory).unwrap();
        let path = directory.join("README.md");
        fs::write(&path, "inside data").unwrap();
        assert!(open_context_file(&path).is_ok());
        fs::remove_file(&path).unwrap();
        symlink(outside.path().join("README.md"), &path).unwrap();
        assert!(open_context_file(&path).is_err());
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&directory).unwrap();
        symlink(outside.path(), &directory).unwrap();
        assert!(open_context_file(&path).is_err());
    }

    #[test]
    fn configured_exclusions_accept_native_separators_and_sensitive_directory_case() {
        assert!(path_is_excluded("private/docs/README.md", r"private\docs"));
        assert!(!path_is_excluded(
            "private/documents/README.md",
            r"private\docs"
        ));
        if cfg!(windows) {
            assert!(path_is_excluded("Private/Docs/README.md", "private/docs"));
        }
        for name in [".AWS", ".SSH", ".CODEX"] {
            assert!(builtin_excluded(&Path::new(name).join("README.md")));
        }
    }

    #[test]
    fn collector_returns_confined_document_facts_but_not_credentials_or_ignored_paths() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("README.md"),
            "Document the verification command.",
        )
        .unwrap();
        std::fs::write(directory.path().join(".env"), "TOKEN=secret").unwrap();
        std::fs::write(directory.path().join(".mcp.json"), r#"{"token":"private"}"#).unwrap();
        std::fs::write(
            directory.path().join("service-account.json"),
            r#"{"private_key":"private"}"#,
        )
        .unwrap();
        std::fs::write(
            directory.path().join("package.json"),
            r#"{"client_secret":"private"}"#,
        )
        .unwrap();
        std::fs::create_dir_all(directory.path().join("ignored")).unwrap();
        std::fs::write(directory.path().join(".gitignore"), "ignored/\n").unwrap();
        let repository = git2::Repository::init(directory.path()).unwrap();
        let facts = collect_repository_facts(
            directory.path(),
            ContextLevel::WorkflowContext,
            &[],
            "2026-09-09T00:00:00Z",
            "",
        )
        .unwrap();

        assert!(facts.iter().any(|fact| fact.path == "README.md"));
        assert!(facts.iter().all(|fact| !fact.path.ends_with(".json")));
        assert!(facts
            .iter()
            .all(|fact| fact.path != ".env" && !fact.path.starts_with("ignored/")));
        drop(repository);
    }

    #[test]
    fn current_guidance_survives_hundreds_of_historical_reports() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join(".superpowers/sdd/archive")).unwrap();
        for n in 0..200 {
            fs::write(
                root.join(format!(".superpowers/sdd/archive/{n:03}.md")),
                "old report\n".repeat(700),
            )
            .unwrap();
        }
        fs::create_dir_all(root.join("apps/desktop")).unwrap();
        fs::write(root.join("AGENTS.md"), "Current repository instructions").unwrap();
        fs::write(root.join("README.md"), "Current project overview").unwrap();
        fs::write(root.join("package.json"), "{\"name\":\"project\"}").unwrap();
        fs::write(
            root.join("apps/desktop/AGENTS.md"),
            "Current desktop guidance",
        )
        .unwrap();
        let facts =
            collect_repository_facts(root, ContextLevel::WorkflowContext, &[], "now", "").unwrap();
        for expected in [
            "AGENTS.md",
            "README.md",
            "package.json",
            "apps/desktop/AGENTS.md",
        ] {
            assert!(
                facts.iter().any(|fact| fact.path == expected),
                "missing {expected}"
            );
        }
    }

    #[test]
    fn relevant_context_reaches_later_sections_without_expanding_access() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        fs::create_dir_all(root.join("docs")).unwrap();
        for n in 0..80 {
            fs::write(
                root.join(format!("docs/a{n:03}.md")),
                "Unrelated introductory material.\n".repeat(150),
            )
            .unwrap();
        }
        let content = format!("# Guide\n\n{}\n\n## Verification\nThe verification failure requires a reproducible fixture before retrying.\n", "General introductory material.\n".repeat(100));
        fs::write(root.join("docs/verification.md"), &content).unwrap();
        fs::write(
            root.join("docs/verification-secret.md"),
            "verification private details",
        )
        .unwrap();
        fs::write(
            root.join("docs/verification-excluded.md"),
            "verification excluded details",
        )
        .unwrap();
        fs::write(
            root.join("docs/verification-sensitive.md"),
            format!("{}\napi_key=must-not-appear", content),
        )
        .unwrap();
        let facts = collect_repository_facts(
            root,
            ContextLevel::WorkflowContext,
            &["docs/verification-excluded.md".into()],
            "now",
            "verification failure",
        )
        .unwrap();
        let fact = facts
            .iter()
            .find(|fact| fact.path == "docs/verification.md")
            .expect("relevant file must survive unrelated docs");
        assert!(fact.excerpt.contains("requires a reproducible fixture"));
        assert!(
            content.contains(&fact.excerpt),
            "excerpt remains one verbatim span"
        );
        assert!(fact.excerpt.chars().count() <= 2000);
        assert_eq!(fact.sha256, hex::encode(Sha256::digest(content.as_bytes())));
        assert!(facts.iter().all(|fact| !fact.path.contains("secret")
            && !fact.path.contains("excluded")
            && !fact.path.contains("sensitive")));
    }

    #[test]
    fn nested_workspace_honors_repository_relative_ignores_and_path_exclusions() {
        let directory = tempfile::tempdir().unwrap();
        let repository = git2::Repository::init(directory.path()).unwrap();
        let nested = directory.path().join("packages/app");
        fs::create_dir_all(&nested).unwrap();
        fs::write(
            directory.path().join(".gitignore"),
            "packages/app/private.md\n",
        )
        .unwrap();
        fs::write(nested.join("private.md"), "must not read").unwrap();
        fs::write(nested.join("README.md"), "public docs").unwrap();
        fs::write(nested.join("excluded.md"), "must not read either").unwrap();
        let facts = collect_repository_facts(
            &nested,
            ContextLevel::WorkflowContext,
            &["excluded.md".into()],
            "now",
            "",
        )
        .unwrap();
        assert_eq!(
            facts
                .iter()
                .map(|fact| fact.path.as_str())
                .collect::<Vec<_>>(),
            ["README.md"]
        );
        drop(repository);
    }
}
