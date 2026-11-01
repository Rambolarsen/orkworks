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

pub(crate) fn subject(
    state: &crate::AppState,
    id: &str,
) -> Result<(PathBuf, crate::session_location::GitLocationSource), String> {
    if id.is_empty()
        || id.len() > 256
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
    {
        return Err("Invalid session ID.".into());
    }
    let info = {
        let ws = state.workspace.lock().unwrap();
        let ws = ws.as_ref().ok_or("Workspace unavailable.")?;
        let live = state.sessions.lock().unwrap();
        live.get(id)
            .map(|h| h.info.clone())
            .or_else(|| {
                ws.metadata
                    .read_session(id)
                    .map(|m| crate::session_types::SessionInfo::baseline_from_metadata(&m))
            })
            .ok_or("Session unavailable.")?
    };
    if let Some(binding) = &info.worktree_binding {
        return binding
            .validate()
            .map(|root| (root, crate::session_location::GitLocationSource::Agent))
            .map_err(str::to_owned);
    }
    let cwds = crate::session_view::resolve_effective_locations(
        &[info],
        &state.peon.reported_cwd.read().unwrap(),
        &state.session_pids.lock().unwrap(),
        crate::procfs::live_cwds,
    );
    let (cwd, source) = cwds.get(id).ok_or("Session directory unavailable.")?;
    git_changes_root(std::path::Path::new(cwd)).map(|root| (root, *source))
}
fn git_changes_root(cwd: &std::path::Path) -> Result<PathBuf, String> {
    crate::git::worktree_root(cwd)
        .ok_or("No available Git worktree for this session.")?
        .canonicalize()
        .map_err(|_| "Worktree unavailable.".into())
}
