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
) -> Result<Vec<RepositoryEvidence>, String> {
    if level == ContextLevel::SessionObservations {
        return Ok(Vec::new());
    }
    let root = workspace
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let repository = git2::Repository::discover(&root).ok();
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
    )?;
    paths.sort();

    let mut evidence = Vec::with_capacity(paths.len().min(MAX_FILES));
    let mut remaining = MAX_TOTAL_BYTES;
    for path in paths.into_iter().take(MAX_FILES) {
        if remaining == 0 {
            break;
        }
        let metadata = fs::metadata(&path).map_err(|error| error.to_string())?;
        if !metadata.is_file() {
            continue;
        }
        let mut bytes = Vec::new();
        fs::File::open(&path)
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
        let excerpt: String = String::from_utf8_lossy(&bytes)
            .chars()
            .take(2_000)
            .collect();
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

fn collect_paths(
    root: &Path,
    directory: &Path,
    depth: usize,
    level: ContextLevel,
    excluded: &[String],
    repository: Option<&git2::Repository>,
    output: &mut Vec<PathBuf>,
    remaining_entries: &mut usize,
) -> Result<(), String> {
    if depth > MAX_DEPTH || output.len() >= MAX_PATHS {
        return Ok(());
    }
    let mut entries = fs::read_dir(directory)
        .map_err(|error| error.to_string())?
        .take(*remaining_entries)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    *remaining_entries = remaining_entries.saturating_sub(entries.len());
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let path = entry.path();
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
            )?;
        } else if file_type.is_file() && allowed_file(relative, level) {
            let canonical = match path.canonicalize() {
                Ok(value) if value.starts_with(root) => value,
                _ => continue,
            };
            output.push(canonical);
        }
    }
    Ok(())
}

fn path_is_excluded(relative: &str, configured: &str) -> bool {
    let configured = configured.trim().trim_matches('/');
    !configured.is_empty()
        && (relative == configured || relative.starts_with(&format!("{configured}/")))
}

fn builtin_excluded(relative: &Path) -> bool {
    if relative.components().any(|part| {
        matches!(
            part.as_os_str().to_str(),
            Some(
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
