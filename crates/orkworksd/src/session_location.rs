//! Explicit task-worktree identity, independent of launch/resume CWD.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorktreeBinding {
    pub root: String,
    pub common_dir: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum GitLocationSource {
    Agent,
    Harness,
    Process,
    Launch,
}

impl WorktreeBinding {
    pub fn resolve(launch: &str, path: &str) -> Result<Self, &'static str> {
        if path.len() > 4096
            || path.chars().any(char::is_control)
            || !Path::new(path).is_absolute()
            || !Path::new(path).is_dir()
        {
            return Err("invalid worktree path");
        }
        let launch =
            git2::Repository::discover(launch).map_err(|_| "launch repository unavailable")?;
        let repo = git2::Repository::discover(path).map_err(|_| "worktree unavailable")?;
        let root = canonical_git_path(repo.workdir().ok_or("bare repository")?)?;
        let common = canonical_git_path(repo.commondir())?;
        if launch.is_bare() || canonical_git_path(launch.commondir()).ok().as_ref() != Some(&common)
        {
            return Err("worktree belongs to another repository");
        }
        Ok(Self {
            root: root.to_str().ok_or("unsupported path")?.into(),
            common_dir: common.to_str().ok_or("unsupported path")?.into(),
        })
    }

    pub fn validate(&self) -> Result<PathBuf, &'static str> {
        let root = canonical_git_path(Path::new(&self.root))
            .map_err(|_| "Reported task worktree unavailable.")?;
        let repo =
            git2::Repository::open(&root).map_err(|_| "Reported task worktree unavailable.")?;
        if repo
            .workdir()
            .and_then(|p| canonical_git_path(p).ok())
            .as_ref()
            != Some(&root)
            || canonical_git_path(repo.commondir()).ok().as_deref()
                != Some(Path::new(&self.common_dir))
        {
            return Err("Reported task worktree unavailable; repository changed.");
        }
        root.canonicalize()
            .map_err(|_| "Reported task worktree unavailable.")
    }
}

// libgit2 expects ordinary Windows paths, while canonicalize returns verbatim paths.
pub(crate) fn canonical_git_path(path: &Path) -> Result<PathBuf, &'static str> {
    let path = path.canonicalize().map_err(|_| "directory unavailable")?;
    let text = path.to_str().ok_or("unsupported path")?;
    Ok(PathBuf::from(normalize_windows_path(text)))
}

pub(crate) fn normalize_windows_path(text: &str) -> String {
    if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(text).to_owned()
    }
}
