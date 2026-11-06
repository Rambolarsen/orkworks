//! Bounded, read-only Git comparisons. Only private temporary repositories are written.
use git2::{Oid, Repository, Tree};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Component, Path},
    time::{Duration, Instant},
};
pub const INPUT_LIMIT: usize = 1024 * 1024;
const PATCH_LIMIT: usize = 256 * 1024;
const TOTAL_LIMIT: usize = 4 * 1024 * 1024;
const CANDIDATES: usize = 10_000;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Comparison {
    Uncommitted,
    Branch,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bases {
    pub refs: Vec<String>,
    pub default_ref: Option<String>,
    pub limited: bool,
    pub unavailable_reason: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFile {
    pub id: String,
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub additions: Option<usize>,
    pub deletions: Option<usize>,
    pub notice: Option<String>,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Capture {
    pub snapshot_id: String,
    pub session_id: String,
    pub worktree: String,
    pub git_context_source: Option<crate::session_location::GitLocationSource>,
    pub branch: String,
    pub comparison: Comparison,
    pub base_ref: Option<String>,
    pub base_oid: Option<String>,
    pub baseline_oid: Option<String>,
    pub head_oid: Option<String>,
    pub captured_at: String,
    pub partial: bool,
    pub notice: Option<String>,
    pub files: Vec<ChangedFile>,
    #[serde(skip)]
    pub patches: HashMap<String, String>,
    #[serde(skip)]
    pub created: Option<Instant>,
}
fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub fn safe_path(path: &Path) -> Result<(), String> {
    if path.as_os_str().len() > 4096
        || path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, Component::Normal(_)))
        || path.components().any(|c| {
            c.as_os_str().to_str().is_none_or(|s| {
                s.eq_ignore_ascii_case(".git") || s.contains('\\') || s.contains(':')
            })
        })
    {
        Err("Unsafe or unsupported Git path.".into())
    } else {
        Ok(())
    }
}
fn open(cwd: &Path) -> Result<Repository, String> {
    let path = crate::session_location::canonical_git_path(cwd).map_err(err)?;
    let repo = Repository::discover(path).map_err(err)?;
    if repo.is_bare() {
        return Err("Bare repository has no worktree.".into());
    }
    Ok(repo)
}
pub fn bases(cwd: &Path) -> Result<Bases, String> {
    let repo = open(cwd)?;
    let start = Instant::now();
    let mut refs = Vec::new();
    let mut limited = false;
    for entry in repo.references().map_err(err)? {
        if refs.len() == 500 || start.elapsed() > Duration::from_secs(5) {
            limited = true;
            break;
        }
        let entry = entry.map_err(err)?;
        if !(entry.is_branch() || entry.is_remote()) || entry.symbolic_target().is_some() {
            continue;
        }
        if let Some(name) = entry.name() {
            if name.len() <= 4096 && entry.peel_to_commit().is_ok() {
                refs.push(name.to_owned())
            }
        }
    }
    refs.sort();
    let current = repo.head().ok().and_then(|h| h.name().map(str::to_owned));
    let remote = repo
        .find_reference("refs/remotes/origin/HEAD")
        .ok()
        .and_then(|r| r.symbolic_target().map(str::to_owned));
    let default_ref = remote
        .into_iter()
        .chain(["refs/heads/main".into(), "refs/heads/master".into()])
        .find(|r| Some(r) != current.as_ref() && refs.contains(r));
    let unavailable_reason = if repo.head().is_err() {
        Some("No branch history before the first commit, or HEAD is unavailable.".into())
    } else if refs.is_empty() {
        Some("No base branches available locally; fetch history in your coding tool.".into())
    } else {
        None
    };
    Ok(Bases {
        refs,
        default_ref,
        limited,
        unavailable_reason,
    })
}
type TreeFiles = BTreeMap<String, (Oid, i32)>;
fn tree_files(
    repo: &Repository,
    tree: Option<&git2::Tree<'_>>,
    start: Instant,
) -> Result<TreeFiles, String> {
    let mut out = BTreeMap::new();
    if let Some(tree) = tree {
        let mut failure = None;
        tree.walk(git2::TreeWalkMode::PreOrder, |prefix, e| {
            if out.len() >= CANDIDATES || start.elapsed() > Duration::from_secs(5) {
                failure = Some("Repository exceeds inspection traversal budget.");
                return git2::TreeWalkResult::Abort;
            }
            let Some(name) = e.name() else {
                failure = Some("Non-UTF-8 path; comparison unavailable.");
                return git2::TreeWalkResult::Abort;
            };
            if e.kind() != Some(git2::ObjectType::Tree) {
                let p = format!("{prefix}{name}");
                if safe_path(Path::new(&p)).is_err() {
                    failure = Some("Unsafe Git path; comparison unavailable.");
                    return git2::TreeWalkResult::Abort;
                }
                out.insert(p, (e.id(), e.filemode()));
            }
            git2::TreeWalkResult::Ok
        })
        .map_err(|e| failure.unwrap_or(e.message()).to_owned())?;
    }
    let _ = repo;
    Ok(out)
}
fn blob(repo: &Repository, id: Oid, mode: i32) -> Result<Vec<u8>, String> {
    if mode == 0o160000 {
        return Ok(format!("Subproject commit {id}\n").into_bytes());
    }
    if repo.odb().map_err(err)?.read_header(id).map_err(err)?.0 > INPUT_LIMIT {
        return Err("File exceeds 1 MiB; patch unavailable.".into());
    }
    let b = repo.find_blob(id).map_err(err)?;
    if b.size() > INPUT_LIMIT {
        return Err("File exceeds 1 MiB; patch unavailable.".into());
    }
    Ok(b.content().to_vec())
}
fn put(
    repo: &Repository,
    index: &mut git2::Index,
    path: &str,
    data: &[u8],
    mode: i32,
) -> Result<(), String> {
    let id = repo.blob(data).map_err(err)?;
    index
        .add(&git2::IndexEntry {
            ctime: git2::IndexTime::new(0, 0),
            mtime: git2::IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: mode as u32,
            uid: 0,
            gid: 0,
            file_size: data.len() as u32,
            id,
            flags: 0,
            flags_extended: 0,
            path: path.as_bytes().to_vec(),
        })
        .map_err(err)
}
fn resolve_head(repo: &Repository) -> Result<(Option<Oid>, Option<Tree<'_>>), String> {
    let head = match repo.head() {
        Ok(h) => Some(h.peel_to_commit().map_err(err)?),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
        Err(e) => return Err(err(e)),
    };
    let head_tree = head.as_ref().map(|h| h.tree()).transpose().map_err(err)?;
    Ok((head.map(|h| h.id()), head_tree))
}

/// Comparison start references: the baseline tree and resolved base/baseline ids.
type Baseline<'a> = (Option<Tree<'a>>, Option<Oid>, Option<Oid>);
/// Resolve the comparison baseline tree, merge base and baseline identifier.
fn select_baseline<'a>(
    repo: &'a Repository,
    cwd: &Path,
    comparison: Comparison,
    base_ref: Option<&str>,
    head_id: Option<Oid>,
    head_tree: Option<&Tree<'a>>,
) -> Result<Baseline<'a>, String> {
    if comparison != Comparison::Branch {
        return Ok((head_tree.cloned(), None, head_id));
    }
    let h = head_id.ok_or("No branch history before the first commit.")?;
    let name = base_ref.ok_or("Choose a base branch.")?;
    if !bases(cwd)?.refs.iter().any(|r| r == name) {
        return Err("Base branch is unavailable; choose an existing local ref.".into());
    }
    let base = repo
        .find_reference(name)
        .and_then(|r| r.peel_to_commit())
        .map_err(err)?
        .id();
    let common = repo.merge_bases(h, base).map_err(|_| {
        "No available common ancestor; fetch history in your coding tool.".to_owned()
    })?;
    if common.len() != 1 {
        return Err("Multiple merge bases; comparison unavailable.".into());
    }
    let baseline = repo
        .find_commit(common[0])
        .and_then(|c| c.tree())
        .map_err(err)?;
    Ok((Some(baseline), Some(base), Some(common[0])))
}

/// Directory holding the global attributes or excludes file; None when unset.
fn global_control_path(config: &git2::Config, key: &str) -> Option<std::path::PathBuf> {
    config.get_path(key).ok().or_else(|| {
        std::env::var_os("XDG_CONFIG_HOME")
            .map(std::path::PathBuf::from)
            .or_else(|| dirs::home_dir().map(|h| h.join(".config")))
            .map(|d| {
                d.join("git").join(if key == "core.attributesfile" {
                    "attributes"
                } else {
                    "ignore"
                })
            })
    })
}

/// Copy the selected repository's attributes and excludes into the scratch
/// worktree. libgit2 resolves only sanitized control inputs in this private
/// worktree. Git administrative/global config remains the selected
/// repository's normal metadata authority, copied with bounded nonblocking
/// leaf reads.
fn mirror_control_inputs(
    repo: &Repository,
    scratch: &Repository,
    worktree: &Path,
    config: &git2::Config,
) -> Result<(), String> {
    // libgit2 resolves only sanitized control inputs in this private worktree.
    // Git administrative/global config remains the selected repository's normal
    // metadata authority, copied with bounded nonblocking leaf reads.
    for (key, name) in [
        ("core.attributesfile", "global-attributes"),
        ("core.excludesfile", "global-excludes"),
    ] {
        let original = global_control_path(config, key);
        let data = if let Some(path) = original {
            crate::git_changes_fs::metadata_file(&if path.is_absolute() {
                path
            } else {
                worktree.join(path)
            })
            .map_err(err)?
            .unwrap_or_default()
        } else {
            Vec::new()
        };
        let destination = scratch.path().join(name);
        std::fs::write(&destination, data).map_err(err)?;
        scratch
            .config()
            .map_err(err)?
            .set_str(key, &destination.display().to_string())
            .map_err(err)?;
    }
    for name in ["attributes", "exclude"] {
        if let Some(data) =
            crate::git_changes_fs::metadata_file(&repo.commondir().join("info").join(name))
                .map_err(err)?
        {
            std::fs::write(scratch.path().join("info").join(name), data).map_err(err)?;
        }
    }
    Ok(())
}

/// Immutable comparison inputs: the selected repository, its baseline tree
/// files and the index-derived membership sets used by every comparison path.
struct CaptureSets<'a> {
    repo: &'a Repository,
    scratch: &'a Repository,
    worktree: &'a Path,
    start: Instant,
    comparison: Comparison,
    old: BTreeMap<String, (Oid, i32)>,
    tracked: HashSet<String>,
    gitlinks: HashSet<String>,
    conflicts: HashSet<String>,
    stage0: HashMap<String, (Oid, i32, bool)>,
    trust_filemode: bool,
    literal_symlinks: bool,
}

impl<'a> CaptureSets<'a> {
    fn new(
        repo: &'a Repository,
        scratch: &'a Repository,
        worktree: &'a Path,
        comparison: Comparison,
        start: Instant,
        old: BTreeMap<String, (Oid, i32)>,
    ) -> Result<Self, String> {
        let index = if comparison == Comparison::Uncommitted {
            Some(repo.index().map_err(err)?)
        } else {
            None
        };
        if index.as_ref().is_some_and(|index| index.len() > CANDIDATES) {
            return Err("Git index exceeds inspection budget.".into());
        }
        let tracked: HashSet<String> = index
            .iter()
            .flat_map(|index| index.iter())
            .map(|e| {
                String::from_utf8(e.path)
                    .map_err(|_| "Non-UTF-8 index path; comparison unavailable.".to_owned())
            })
            .collect::<Result<_, _>>()?;
        let gitlinks: HashSet<String> = old
            .iter()
            .filter(|(_, (_, mode))| *mode == 0o160000)
            .map(|(p, _)| p.clone())
            .chain(
                index
                    .iter()
                    .flat_map(|index| index.iter())
                    .filter(|e| e.mode == 0o160000)
                    .filter_map(|e| String::from_utf8(e.path).ok()),
            )
            .collect();
        let conflicts: HashSet<String> = if comparison == Comparison::Uncommitted {
            repo.index()
                .map_err(err)?
                .iter()
                .filter(|e| e.flags & 0x3000 != 0)
                .filter_map(|e| String::from_utf8(e.path).ok())
                .collect()
        } else {
            HashSet::new()
        };
        let trust_filemode = repo
            .config()
            .ok()
            .and_then(|c| c.get_bool("core.filemode").ok())
            .unwrap_or(cfg!(unix));
        let literal_symlinks = repo
            .config()
            .ok()
            .and_then(|c| c.get_bool("core.symlinks").ok())
            == Some(false);
        // With core.symlinks=false the index defines which ordinary files are
        // link-text checkouts. HEAD must not hide a staged conversion or replacement.
        let stage0: HashMap<String, (Oid, i32, bool)> = index
            .iter()
            .flat_map(|index| index.iter())
            .filter(|e| e.flags & 0x3000 == 0)
            .filter_map(|e| {
                String::from_utf8(e.path)
                    .ok()
                    .map(|path| (path, (e.id, e.mode as i32, e.flags_extended & 0x4000 != 0)))
            })
            .collect();
        let mut controls = scratch.index().map_err(err)?;
        let mut control_size = 0usize;
        for (path, (id, mode, _)) in &stage0 {
            if matches!(
                std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str()),
                Some(".gitattributes" | ".gitignore")
            ) {
                if ![0o100644, 0o100755].contains(mode) {
                    return Err("Unsupported indexed Git control file.".into());
                }
                let data = blob(repo, *id, *mode)?;
                control_size += data.len();
                if control_size > 4 * 1024 * 1024 {
                    return Err("Git control index budget exceeded.".into());
                }
                put(scratch, &mut controls, path, &data, *mode)?;
            }
        }
        controls.write().map_err(err)?;
        Ok(Self {
            repo,
            scratch,
            worktree,
            start,
            comparison,
            old,
            tracked,
            gitlinks,
            conflicts,
            stage0,
            trust_filemode,
            literal_symlinks,
        })
    }

    /// Resolve the effective file mode: uncommitted comparisons honor
    /// filemode trust and literal-symlink configuration from the index.
    pub fn effective_mode(&self, path: &str, mode: i32) -> i32 {
        if [0o100644, 0o100755].contains(&mode) {
            if let Some((_, indexed, _)) = self.stage0.get(path) {
                if (self.literal_symlinks && *indexed == 0o120000)
                    || (!self.trust_filemode && [0o100644, 0o100755].contains(indexed))
                {
                    return *indexed;
                }
            }
        }
        mode
    }
}

/// Mutable comparison accumulators: the staged before/after indices, budget
/// bookkeeping and the per-path notices and visit records.
struct Staging<'a> {
    sets: &'a CaptureSets<'a>,
    before: git2::Index,
    after: git2::Index,
    total: usize,
    partial: bool,
    notices: Vec<(String, String)>,
    visited: HashSet<String>,
}

impl Staging<'_> {
    fn new<'a>(sets: &'a CaptureSets<'a>) -> Result<Staging<'a>, String> {
        Ok(Staging {
            sets,
            before: git2::Index::new().map_err(err)?,
            after: git2::Index::new().map_err(err)?,
            total: 0,
            partial: false,
            notices: Vec::new(),
            visited: HashSet::new(),
        })
    }

    /// Old/new content pair for a path with the uncommitted mode adjustment,
    /// or the read error that makes the path unavailable.
    fn compared_pair(
        &self,
        path: &str,
        new: Result<Option<(Vec<u8>, i32)>, String>,
    ) -> Result<(Option<(Vec<u8>, i32)>, Option<(Vec<u8>, i32)>), String> {
        let sets = self.sets;
        let old_content = sets
            .old
            .get(path)
            .map(|(id, m)| blob(sets.repo, *id, *m).map(|b| (b, *m)))
            .transpose();
        old_content.and_then(|old| {
            new.map(|mut new| {
                if sets.comparison == Comparison::Uncommitted {
                    if let Some((_, mode)) = &mut new {
                        *mode = sets.effective_mode(path, *mode);
                    }
                }
                (old, new)
            })
        })
    }

    /// Record one compared path: budget-checked staging of old/new blob pairs.
    fn compare(
        &mut self,
        path: &str,
        new: Result<Option<(Vec<u8>, i32)>, String>,
    ) -> Result<(), String> {
        let sets = self.sets;
        if sets.start.elapsed() > Duration::from_secs(5) {
            return Err("Capture budget exceeded; comparison unavailable.".into());
        }
        self.visited.insert(path.to_owned());
        let (old, new) = match self.compared_pair(path, new) {
            Ok(p) => p,
            Err(n) => {
                self.partial = true;
                self.notices.push((path.to_owned(), n));
                return Ok(());
            }
        };
        if sets.conflicts.contains(path) {
            self.partial = true;
            self.notices
                .push((path.into(), "Conflict; patch unavailable.".into()));
            return Ok(());
        }
        if old == new {
            return Ok(());
        }
        if self.before.len() + self.after.len() >= 1000
            || self.total
                + old.as_ref().map_or(0, |x| x.0.len())
                + new.as_ref().map_or(0, |x| x.0.len())
                > TOTAL_LIMIT
        {
            self.partial = true;
            return Ok(());
        }
        for (content, index) in [(old, &mut self.before), (new, &mut self.after)] {
            if let Some((data, mut mode)) = content {
                self.total += data.len();
                if mode == 0o160000 {
                    mode = 0o100644
                }
                put(sets.scratch, index, path, &data, mode)?;
            }
        }
        Ok(())
    }
}

/// Compare two resolved trees: every differing path enters the staging indices.
fn compare_branch(
    sets: &CaptureSets<'_>,
    staging: &mut Staging<'_>,
    target: &BTreeMap<String, (Oid, i32)>,
) -> Result<(), String> {
    let paths: HashSet<_> = sets.old.keys().chain(target.keys()).cloned().collect();
    for p in paths {
        if sets.start.elapsed() > Duration::from_secs(5) {
            return Err("Capture budget exceeded; comparison unavailable.".into());
        }
        if sets.old.get(&p) == target.get(&p) {
            continue;
        }
        let new = target
            .get(&p)
            .map(|(id, m)| blob(sets.repo, *id, *m).map(|b| (b, *m)))
            .transpose();
        staging.compare(&p, new)?;
    }
    Ok(())
}

/// Per-walk budget cells: candidate counting, exceeded marking and the first
/// callback failure, shared across the walk callbacks.
struct WalkCells {
    count: std::cell::Cell<usize>,
    exceeded: std::cell::Cell<bool>,
    failure: std::cell::RefCell<Option<String>>,
}

impl WalkCells {
    fn new() -> Self {
        Self {
            count: std::cell::Cell::new(0usize),
            exceeded: std::cell::Cell::new(false),
            failure: std::cell::RefCell::new(None),
        }
    }

    /// Advance the candidate budget and mark it exceeded past its limits.
    fn over(&self, start: Instant) -> bool {
        self.count.set(self.count.get() + 1);
        if self.count.get() <= CANDIDATES && start.elapsed() <= Duration::from_secs(5) {
            return false;
        }
        self.exceeded.set(true);
        true
    }
}

/// File-callback logic: budgeted comparisons of one worktree leaf.
fn visit_worktree_entry(
    sets: &CaptureSets<'_>,
    staging: &mut Staging<'_>,
    cells: &WalkCells,
    seen_modules: &mut HashSet<String>,
    entry: crate::git_changes_fs::Entry,
) -> bool {
    if cells.over(sets.start) {
        return false;
    }
    let Some(p) = entry.path.to_str() else {
        cells.exceeded.set(true);
        return true;
    };
    if sets.gitlinks.contains(p) {
        seen_modules.insert(p.to_owned());
    }
    if !sets.old.contains_key(p)
        && !sets.tracked.contains(p)
        && sets
            .scratch
            .status_should_ignore(&entry.path)
            .unwrap_or(true)
    {
        return true;
    }
    if matches!(entry.content, Ok(None)) {
        staging.visited.insert(p.to_owned());
        return true;
    }
    if let Err(e) = staging.compare(p, entry.content.map(|b| b.map(|b| (b, entry.mode)))) {
        *cells.failure.borrow_mut() = Some(e);
        return false;
    }
    true
}

/// Directory-callback logic: budgeted descent decision with control mirroring.
fn descend_worktree(
    sets: &CaptureSets<'_>,
    cells: &WalkCells,
    temp: &tempfile::TempDir,
    path: &Path,
    is_dir: bool,
) -> bool {
    if cells.over(sets.start) {
        return false;
    }
    let Some(p) = path.to_str() else {
        cells.exceeded.set(true);
        return false;
    };
    if is_dir && sets.gitlinks.contains(p) {
        return false;
    }
    if is_dir {
        if let Err(e) = std::fs::create_dir_all(temp.path().join(path)) {
            *cells.failure.borrow_mut() = Some(e.to_string());
            return false;
        }
    }
    let prefix = format!("{p}/");
    sets.old.contains_key(p)
        || sets.tracked.contains(p)
        || sets.tracked.iter().any(|k| k.starts_with(&prefix))
        || sets.old.keys().any(|k| k.starts_with(&prefix))
        || !sets.scratch.status_should_ignore(path).unwrap_or(true)
}

/// Read-mode-callback logic: captured expected blob and line-end policy.
fn expect_worktree_mode(
    sets: &CaptureSets<'_>,
    path: &Path,
    mode: i32,
) -> Result<(Option<Oid>, crate::git_changes_eol::Eol), String> {
    let path = path.to_str().ok_or("Unsupported path.")?;
    let mode = sets.effective_mode(path, mode);
    let expected = sets
        .old
        .get(path)
        .filter(|(_, m)| !sets.conflicts.contains(path) && *m == mode)
        .map(|(id, _)| *id);
    let eol = crate::git_changes_eol::policy(
        sets.scratch,
        sets.repo,
        path,
        sets.stage0.get(path).map(|e| e.0),
        sets.old.get(path).map(|e| e.0),
    )?;
    Ok((expected, eol))
}

/// Control-callback logic: mirror directory attributes and ignore files.
fn prepare_worktree_controls(
    temp: &tempfile::TempDir,
    prefix: &Path,
    attrs: Option<Vec<u8>>,
    ignore: Option<Vec<u8>>,
) -> Result<(), String> {
    for (name, data) in [(".gitattributes", attrs), (".gitignore", ignore)] {
        if let Some(data) = data {
            let directory = temp.path().join(prefix);
            std::fs::create_dir_all(&directory).map_err(err)?;
            std::fs::write(directory.join(name), data).map_err(err)?;
        }
    }
    Ok(())
}

/// Compare gitlink worktree states, including absent or escaped modules.
fn compare_submodule_states(
    sets: &CaptureSets<'_>,
    staging: &mut Staging<'_>,
    seen_modules: &HashSet<String>,
) -> Result<(), String> {
    for path in &sets.gitlinks {
        if seen_modules.contains(path) {
            continue;
        }
        let directory = sets.worktree.join(path);
        if !directory.exists() && sets.stage0.get(path).is_some_and(|e| e.2) {
            continue;
        }
        let new = if !directory.exists() {
            Ok(None)
        } else if directory
            .canonicalize()
            .is_ok_and(|p| p.starts_with(sets.worktree))
        {
            sets.repo
                .find_submodule(path)
                .and_then(|module| module.open())
                .and_then(|sub| sub.head().and_then(|h| h.peel_to_commit()).map(|c| c.id()))
                .map(|id| Some((format!("Subproject commit {id}\n").into_bytes(), 0o160000)))
                .map_err(|_| "Submodule commit unavailable; inspect its registered session.".into())
        } else {
            Err("Submodule directory outside this worktree; unavailable.".into())
        };
        staging.compare(path, new)?;
    }
    Ok(())
}

/// Compare baseline and tracked paths the walk could not show, including late
/// skip-worktree index entries.
fn compare_remaining_staged(
    sets: &CaptureSets<'_>,
    staging: &mut Staging<'_>,
) -> Result<(), String> {
    let remaining: Vec<String> = sets
        .old
        .keys()
        .chain(sets.tracked.iter())
        .filter(|p| !staging.visited.contains(*p))
        .cloned()
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    for path in remaining {
        if let Some((id, mode, true)) = sets.stage0.get(&path) {
            if sets.old.get(&path) == Some(&(*id, *mode)) {
                continue;
            }
            staging.compare(
                &path,
                blob(sets.repo, *id, *mode).map(|data| Some((data, *mode))),
            )?;
        } else {
            staging.compare(&path, Ok(None))?;
        }
    }
    Ok(())
}

/// Inspect the live worktree against the baseline and staged index state,
/// including submodule pointers and late staged additions, honoring the
/// walk/existence budgets by marking the capture partial.
fn inspect_uncommitted(
    sets: &CaptureSets<'_>,
    staging: &mut Staging<'_>,
    temp: &tempfile::TempDir,
) -> Result<(), String> {
    let mut seen_modules: HashSet<String> = HashSet::new();
    let cells = WalkCells::new();
    crate::git_changes_fs::walk(
        sets.worktree,
        |entry| visit_worktree_entry(sets, staging, &cells, &mut seen_modules, entry),
        |path, is_dir| descend_worktree(sets, &cells, temp, path, is_dir),
        |path, mode| expect_worktree_mode(sets, path, mode),
        |prefix, attrs, ignore| prepare_worktree_controls(temp, prefix, attrs, ignore),
    )
    .map_err(err)?;
    if let Some(e) = cells.failure.into_inner() {
        return Err(e);
    }
    compare_submodule_states(sets, staging, &seen_modules)?;
    if cells.exceeded.get() {
        staging.partial = true;
        return Ok(());
    }
    compare_remaining_staged(sets, staging)?;
    Ok(())
}

/// Pair of split-side delta indices for one type-change conversion path.
fn typechange_indices(split: &git2::Diff<'_>, path: &str) -> Result<Vec<usize>, String> {
    let indices: Vec<_> = split
        .deltas()
        .enumerate()
        .filter(|(_, d)| d.new_file().path().or(d.old_file().path()) == Some(Path::new(path)))
        .map(|(index, _)| index)
        .collect();
    if indices.len() != 2 {
        return Err("Type change patch unavailable.".into());
    }
    Ok(indices)
}

/// Net line change totals of a type conversion, as Git reports them.
fn typechange_net_lines(
    scratch: &Repository,
    delta: &git2::DiffDelta,
    path: &str,
) -> Result<(usize, usize), String> {
    let old_blob = scratch.find_blob(delta.old_file().id()).map_err(err)?;
    let new_blob = scratch.find_blob(delta.new_file().id()).map_err(err)?;
    let mut opts = git2::DiffOptions::new();
    // Classification already used the captured attributes above.
    opts.force_text(true);
    let net = git2::Patch::from_blobs(
        &old_blob,
        Some(Path::new(path)),
        &new_blob,
        Some(Path::new(path)),
        Some(&mut opts),
    )
    .map_err(err)?;
    let (_, a, d) = net.line_stats().map_err(err)?;
    Ok((a, d))
}

/// One rendered patch side with its line totals, or the display notice that
/// blocks it (budget or encoding failure, not a fetch failure).
type SidePatch = Result<(usize, usize, String), String>;

/// Budget and encoding check for one rendered patch side.
fn side_patch_bytes(
    patch: &mut git2::Patch<'_>,
    text_size: usize,
    output_size: usize,
) -> Result<SidePatch, String> {
    let bytes = patch.to_buf().map_err(err)?;
    if text_size + bytes.len() > PATCH_LIMIT || output_size + text_size + bytes.len() > TOTAL_LIMIT
    {
        return Ok(Err("Patch exceeds display budget.".into()));
    }
    if std::str::from_utf8(&bytes).is_err() || bytes.split(|b| *b == b'\n').any(|l| l.len() > 65536)
    {
        return Ok(Err("Unsupported text encoding or line too long.".into()));
    }
    let (_, a, d) = patch.line_stats().map_err(err)?;
    Ok(Ok((
        a,
        d,
        std::str::from_utf8(&bytes).map_err(err)?.to_owned(),
    )))
}

/// Text and recorded line totals for one delta's non-binary patch sides,
/// or the display notice that empties them.
fn delta_patch_content(
    staging: &mut Staging<'_>,
    mut file_patches: Vec<Option<git2::Patch<'_>>>,
    delta: &git2::DiffDelta,
    path: &str,
    output: &mut usize,
) -> Result<(Option<(usize, usize)>, Option<String>, String), String> {
    let mut added = 0usize;
    let mut deleted = 0usize;
    let mut text = String::new();
    let mut notice = None;
    if file_patches.iter().any(Option::is_none) {
        notice = Some("Binary file; no text patch.".into());
    } else {
        for mut patch in file_patches.into_iter().flatten() {
            let text_size = text.len();
            match side_patch_bytes(&mut patch, text_size, *output)? {
                Ok((a, d, chunk)) => {
                    added += a;
                    deleted += d;
                    text.push_str(&chunk);
                }
                Err(n) => {
                    notice = Some(n);
                    staging.partial = true;
                    break;
                }
            }
        }
        if notice.is_none() {
            if delta.status() == git2::Delta::Typechange {
                // Git displays replacement hunks for a type conversion, but
                // its line totals describe the net blob-content change.
                let (a, d) = typechange_net_lines(staging.sets.scratch, delta, path)?;
                added = a;
                deleted = d;
            }
            *output += text.len();
        } else {
            // Never retain or expose one half of a conversion as a complete patch.
            text = String::new();
        }
    }
    Ok((notice.is_none().then_some((added, deleted)), notice, text))
}

/// One diff delta's summary, patch text and budget outcome.
#[allow(clippy::too_many_arguments)]
fn render_delta(
    staging: &mut Staging<'_>,
    diff: &git2::Diff<'_>,
    typechange_diff: Option<&git2::Diff<'_>>,
    delta_index: usize,
    delta: &git2::DiffDelta,
    files: &mut Vec<ChangedFile>,
    patches: &mut HashMap<String, String>,
    output: &mut usize,
) -> Result<(), String> {
    let path = delta
        .new_file()
        .path()
        .or(delta.old_file().path())
        .and_then(Path::to_str)
        .ok_or("Unsupported path.")?
        .to_owned();
    let old_path = delta
        .old_file()
        .path()
        .and_then(Path::to_str)
        .filter(|p| *p != path)
        .map(str::to_owned);
    let (patch_diff, indices) = if delta.status() == git2::Delta::Typechange {
        let split = typechange_diff.ok_or("Type change patch unavailable.")?;
        (split, typechange_indices(split, &path)?)
    } else {
        (diff, vec![delta_index])
    };
    let file_patches = indices
        .into_iter()
        .map(|index| git2::Patch::from_diff(patch_diff, index).map_err(err))
        .collect::<Result<Vec<_>, _>>()?;
    if file_patches
        .iter()
        .flatten()
        .any(|patch| patch.delta().old_file().is_binary() || patch.delta().new_file().is_binary())
    {
        let id = uuid::Uuid::new_v4().to_string();
        files.push(ChangedFile {
            id,
            path,
            old_path,
            status: "Binary".into(),
            additions: None,
            deletions: None,
            notice: Some("Binary file; no text patch.".into()),
        });
        return Ok(());
    }
    let (totals, notice, text) = delta_patch_content(staging, file_patches, delta, &path, output)?;
    let (additions, deletions) = match totals {
        Some((a, d)) => (Some(a), Some(d)),
        None => (None, None),
    };
    let id = uuid::Uuid::new_v4().to_string();
    patches.insert(id.clone(), text);
    let status = match delta.status() {
        git2::Delta::Added => "New",
        git2::Delta::Deleted => "Deleted",
        git2::Delta::Renamed => "Renamed",
        git2::Delta::Typechange => "Type changed",
        _ => "Modified",
    }
    .into();
    files.push(ChangedFile {
        id,
        path,
        old_path,
        status,
        additions,
        deletions,
        notice,
    });
    Ok(())
}

/// Write the captured trees and render patch text and per-file summaries,
/// honoring the file-count, patch-size and aggregate output budgets.
fn render_changes(
    staging: &mut Staging<'_>,
) -> Result<(Vec<ChangedFile>, HashMap<String, String>), String> {
    let start = staging.sets.start;
    let scratch = staging.sets.scratch;
    let before = staging.before.write_tree_to(scratch).map_err(err)?;
    let after = staging.after.write_tree_to(scratch).map_err(err)?;
    let mut opts = git2::DiffOptions::new();
    opts.context_lines(3)
        .skip_binary_check(false)
        .include_typechange(true);
    let mut diff = scratch
        .diff_tree_to_tree(
            Some(&scratch.find_tree(before).map_err(err)?),
            Some(&scratch.find_tree(after).map_err(err)?),
            Some(&mut opts),
        )
        .map_err(err)?;
    let mut find = git2::DiffFindOptions::new();
    find.renames(true).rename_limit(500);
    diff.find_similar(Some(&mut find)).map_err(err)?;
    // libgit2 omits content from combined Typechange deltas. Keep those deltas
    // for file identity/rename detection, but render their deletion and addition
    // from the same captured trees, without another source-file read.
    let typechange_diff = if diff.deltas().any(|d| d.status() == git2::Delta::Typechange) {
        let mut opts = git2::DiffOptions::new();
        opts.context_lines(3).include_typechange(false);
        Some(
            scratch
                .diff_tree_to_tree(
                    Some(&scratch.find_tree(before).map_err(err)?),
                    Some(&scratch.find_tree(after).map_err(err)?),
                    Some(&mut opts),
                )
                .map_err(err)?,
        )
    } else {
        None
    };
    let mut files = Vec::new();
    let mut patches = HashMap::new();
    let mut output = 0;
    for (i, delta) in diff.deltas().enumerate() {
        if files.len() >= 500 || start.elapsed() > Duration::from_secs(5) {
            staging.partial = true;
            break;
        }
        render_delta(
            staging,
            &diff,
            typechange_diff.as_ref(),
            i,
            &delta,
            &mut files,
            &mut patches,
            &mut output,
        )?;
    }
    for (path, n) in std::mem::take(&mut staging.notices)
        .into_iter()
        .take(500usize.saturating_sub(files.len()))
    {
        files.push(ChangedFile {
            id: uuid::Uuid::new_v4().to_string(),
            path,
            old_path: None,
            status: "Unavailable".into(),
            additions: None,
            deletions: None,
            notice: Some(n),
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok((files, patches))
}

/// Detect any reference churn under the capture and fail fast.
fn verify_refs_unchanged(
    repo: &Repository,
    head_id: Option<Oid>,
    base_ref: Option<&str>,
    base_oid: Option<Oid>,
) -> Result<(), String> {
    if repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_commit().ok())
        .map(|h| h.id())
        != head_id
        || base_ref.is_some_and(|r| {
            repo.find_reference(r)
                .and_then(|r| r.peel_to_commit())
                .ok()
                .map(|c| c.id())
                != base_oid
        })
    {
        return Err("Git baseline changed during capture; refresh.".into());
    }
    Ok(())
}

/// Branch name for the capture header; detached and unborn heads are labeled.
fn branch_label(repo: &Repository, head_id: Option<Oid>) -> String {
    repo.head()
        .ok()
        .and_then(|h| {
            if h.is_branch() {
                h.shorthand().map(str::to_owned)
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            head_id
                .map(|h| format!("Detached HEAD {}", &h.to_string()[..8]))
                .unwrap_or("Unborn branch".into())
        })
}

pub fn capture(
    cwd: &Path,
    comparison: Comparison,
    base_ref: Option<&str>,
) -> Result<Capture, String> {
    let start = Instant::now();
    let repo = open(cwd)?;
    let worktree = repo
        .workdir()
        .ok_or("No worktree.")?
        .canonicalize()
        .map_err(err)?;
    let (head_id, head_tree) = resolve_head(&repo)?;
    let (baseline, base_oid, baseline_id) = select_baseline(
        &repo,
        cwd,
        comparison,
        base_ref,
        head_id,
        head_tree.as_ref(),
    )?;
    let old = tree_files(&repo, baseline.as_ref(), start)?;
    let target = if comparison == Comparison::Branch {
        tree_files(&repo, head_tree.as_ref(), start)?
    } else {
        BTreeMap::new()
    };
    let temp = tempfile::tempdir().map_err(err)?;
    let scratch = Repository::init(temp.path()).map_err(err)?;
    mirror_control_inputs(&repo, &scratch, &worktree, &repo.config().map_err(err)?)?;
    let sets = CaptureSets::new(&repo, &scratch, &worktree, comparison, start, old)?;
    let mut staging = Staging::new(&sets)?;
    if comparison == Comparison::Branch {
        compare_branch(&sets, &mut staging, &target)?;
    } else {
        inspect_uncommitted(&sets, &mut staging, &temp)?;
    }
    let (files, patches) = render_changes(&mut staging)?;
    verify_refs_unchanged(&repo, head_id, base_ref, base_oid)?;
    Ok(Capture {
        snapshot_id: uuid::Uuid::new_v4().to_string(),
        session_id: String::new(),
        git_context_source: None,
        worktree: worktree.display().to_string(),
        branch: branch_label(&repo, head_id),
        comparison,
        base_ref: base_ref.map(str::to_owned),
        base_oid: base_oid.map(|x| x.to_string()),
        baseline_oid: baseline_id.map(|x| x.to_string()),
        head_oid: head_id.map(|x| x.to_string()),
        captured_at: chrono::Utc::now().to_rfc3339(),
        partial: staging.partial,
        notice: staging
            .partial
            .then(|| "Partial comparison; some content or totals are unavailable.".into()),
        files,
        patches,
        created: Some(Instant::now()),
    })
}

#[derive(Default)]
pub struct CaptureStore {
    captures: Vec<Capture>,
}
impl CaptureStore {
    pub fn insert(&mut self, capture: Capture) {
        self.captures.retain(|c| {
            c.created
                .is_some_and(|t| t.elapsed() < Duration::from_secs(300))
        });
        while self.captures.len() >= 2 {
            self.captures.remove(0);
        }
        self.captures.push(capture);
    }
    pub fn find(&self, session: &str, root: &Path, id: &str) -> Result<&Capture, String> {
        self.captures
            .iter()
            .find(|c| {
                c.session_id == session
                    && Path::new(&c.worktree) == root
                    && c.snapshot_id == id
                    && c.created
                        .is_some_and(|t| t.elapsed() < Duration::from_secs(300))
            })
            .ok_or("Inspection expired or source changed; refresh.".into())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    fn git(dir: &Path, args: &[&str]) {
        let out = std::process::Command::new("git")
            .args(["-c", "user.name=t", "-c", "user.email=t@t"])
            .args(args)
            .current_dir(dir)
            .output()
            .unwrap();
        assert!(out.status.success(), "{out:?}");
    }
    fn init() -> tempfile::TempDir {
        let d = tempfile::tempdir().unwrap();
        git(d.path(), &["init", "-qb", "main"]);
        std::fs::write(d.path().join("a.txt"), "one\ntwo\n").unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "base"]);
        d
    }
    #[test]
    fn comparisons_separate_commits_from_dirty_worktree_and_use_merge_base() {
        let d = init();
        git(d.path(), &["checkout", "-qb", "feature"]);
        std::fs::write(d.path().join("committed.txt"), "branch\n").unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "feature"]);
        git(d.path(), &["checkout", "main"]);
        std::fs::write(d.path().join("base-only.txt"), "base advanced\n").unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "advance"]);
        git(d.path(), &["checkout", "feature"]);
        std::fs::write(d.path().join("a.txt"), "staged\n").unwrap();
        git(d.path(), &["add", "."]);
        std::fs::write(d.path().join("a.txt"), "final\n").unwrap();
        std::fs::write(d.path().join("new.txt"), "new\n").unwrap();
        let a = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert_eq!(
            a.files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(),
            ["a.txt", "new.txt"]
        );
        assert!(a.patches.values().any(|p| p.contains("+final")));
        assert!(!a.patches.values().any(|p| p.contains("+staged")));
        let b = capture(d.path(), Comparison::Branch, Some("refs/heads/main")).unwrap();
        assert_eq!(b.files.len(), 1);
        assert_eq!(b.files[0].path, "committed.txt");
        assert_ne!(b.baseline_oid, b.base_oid);
    }
    #[test]
    fn unborn_ignored_binary_rename_and_no_repository_are_truthful() {
        let d = tempfile::tempdir().unwrap();
        assert!(capture(d.path(), Comparison::Uncommitted, None).is_err());
        git(d.path(), &["init", "-qb", "main"]);
        std::fs::write(d.path().join(".gitignore"), "ignored\n").unwrap();
        std::fs::write(d.path().join("ignored"), "secret").unwrap();
        std::fs::write(d.path().join("binary"), b"a\0b").unwrap();
        let a = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert_eq!(a.files.len(), 2);
        assert!(a
            .files
            .iter()
            .any(|f| f.notice.as_deref() == Some("Binary file; no text patch.")));
        assert!(capture(d.path(), Comparison::Branch, Some("refs/heads/main")).is_err());
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "base"]);
        std::fs::rename(d.path().join(".gitignore"), d.path().join("renamed")).unwrap();
        let a = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(a
            .files
            .iter()
            .any(|f| f.status == "Renamed" && f.additions == Some(0)));
    }
    #[test]
    fn conflicts_and_large_inputs_remain_explicitly_unavailable() {
        let d = init();
        std::fs::write(d.path().join("huge.txt"), vec![b'x'; INPUT_LIMIT + 1]).unwrap();
        let a = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(a.partial);
        assert!(a
            .files
            .iter()
            .any(|f| f.path == "huge.txt" && f.additions.is_none() && f.notice.is_some()));
        std::fs::remove_file(d.path().join("huge.txt")).unwrap();
        git(d.path(), &["checkout", "-qb", "feature"]);
        std::fs::write(d.path().join("a.txt"), "feature\n").unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "feature"]);
        git(d.path(), &["checkout", "main"]);
        std::fs::write(d.path().join("a.txt"), "main\n").unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "main"]);
        let out = std::process::Command::new("git")
            .args([
                "-c",
                "user.name=t",
                "-c",
                "user.email=t@t",
                "merge",
                "feature",
            ])
            .current_dir(d.path())
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(
            Repository::open(d.path())
                .unwrap()
                .index()
                .unwrap()
                .has_conflicts(),
            "Expected an actual merge conflict, got stdout={} stderr={}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        // Even a file that currently matches HEAD remains an unmerged index entry.
        std::fs::write(d.path().join("a.txt"), "main\n").unwrap();
        let a = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(a
            .files
            .iter()
            .any(|f| f.path == "a.txt"
                && f.notice.as_deref() == Some("Conflict; patch unavailable.")));
    }
    #[cfg(unix)]
    #[test]
    fn fifo_and_nested_symlink_do_not_read_targets_or_block() {
        use std::{ffi::CString, os::unix::fs::symlink};
        let d = init();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "OUTSIDE SECRET").unwrap();
        symlink(outside.path(), d.path().join("escape")).unwrap();
        let fifo = CString::new(d.path().join("pipe").as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        let a = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(a
            .files
            .iter()
            .any(|f| f.path == "pipe" && f.notice.is_some()));
        assert!(!a.files.iter().any(|f| f.path.contains("secret")));
    }
    #[cfg(unix)]
    #[test]
    fn leaf_replaced_with_symlink_during_read_fails_closed() {
        use std::os::unix::fs::symlink;
        let d = init();
        let outside = tempfile::tempdir().unwrap();
        let secret = outside.path().join("secret");
        std::fs::write(&secret, "OUTSIDE SECRET").unwrap();
        let mut results = Vec::new();
        crate::git_changes_fs::walk(
            &d.path().canonicalize().unwrap(),
            |entry| {
                results.push(entry);
                true
            },
            |path, _| {
                if path == Path::new("a.txt") {
                    std::fs::remove_file(d.path().join(path)).unwrap();
                    symlink(&secret, d.path().join(path)).unwrap();
                }
                true
            },
            |_, _| Ok((None, crate::git_changes_eol::Eol::Raw)),
            |_, _, _| Ok(()),
        )
        .unwrap();
        assert!(results
            .iter()
            .any(|r| r.path == Path::new("a.txt") && r.content.is_err()));
    }
    #[test]
    fn forced_staged_additions_are_included_even_when_ignored() {
        let d = init();
        std::fs::create_dir(d.path().join("ignored")).unwrap();
        std::fs::write(d.path().join(".gitignore"), "ignored/\n").unwrap();
        std::fs::write(d.path().join("ignored/new.txt"), "staged content\n").unwrap();
        git(d.path(), &["add", "-f", "ignored/new.txt"]);
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(c.files.iter().any(|f| f.path == "ignored/new.txt"));
    }
    #[test]
    fn deleted_binary_files_obey_the_aggregate_capture_budget() {
        let d = init();
        for i in 0..6 {
            std::fs::write(d.path().join(format!("{i}.bin")), vec![0; 900_000]).unwrap();
        }
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "binary baseline"]);
        for i in 0..6 {
            std::fs::remove_file(d.path().join(format!("{i}.bin"))).unwrap();
        }
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(
            c.partial,
            "deletion inputs over the capture budget must be partial"
        );
    }
    #[test]
    fn unchanged_oversized_files_are_not_reported_as_deltas() {
        let d = init();
        std::fs::write(d.path().join("large.bin"), vec![0; INPUT_LIMIT + 1]).unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "large baseline"]);
        let a = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(!a.partial);
        assert!(a.files.is_empty());
        let b = capture(d.path(), Comparison::Branch, Some("refs/heads/main")).unwrap();
        assert!(!b.partial);
        assert!(b.files.is_empty());
    }
    #[cfg(unix)]
    #[test]
    fn unsupported_untracked_names_cannot_be_reported_as_clean() {
        #[cfg(target_os = "linux")]
        use std::os::unix::ffi::OsStringExt;
        let d = init();
        std::fs::write(d.path().join("bad:path"), "content").unwrap();
        assert!(capture(d.path(), Comparison::Uncommitted, None).is_err());
        std::fs::remove_file(d.path().join("bad:path")).unwrap();
        #[cfg(target_os = "linux")]
        {
            std::fs::write(
                d.path().join(std::ffi::OsString::from_vec(vec![0xff])),
                "content",
            )
            .unwrap();
            assert!(capture(d.path(), Comparison::Uncommitted, None).is_err());
        }
    }
    fn stage_file_mode(dir: &Path, mode: &str, content: &[u8]) {
        let repo = Repository::open(dir).unwrap();
        let id = repo.blob(content).unwrap();
        git(
            dir,
            &["update-index", "--cacheinfo", &format!("{mode},{id},a.txt")],
        );
        std::fs::write(dir.join("a.txt"), content).unwrap();
    }

    #[test]
    fn file_symlink_conversions_include_content_in_both_comparisons() {
        for reverse in [false, true] {
            for comparison in [Comparison::Uncommitted, Comparison::Branch] {
                let d = init();
                git(d.path(), &["config", "core.symlinks", "false"]);
                if reverse {
                    stage_file_mode(d.path(), "120000", b"somewhere");
                    git(d.path(), &["commit", "-qm", "link baseline"]);
                }
                git(d.path(), &["checkout", "-qb", "feature"]);
                let (mode, content): (&str, &[u8]) = if reverse {
                    ("100644", b"one\ntwo\n")
                } else {
                    ("120000", b"somewhere")
                };
                stage_file_mode(d.path(), mode, content);
                let base = if comparison == Comparison::Branch {
                    git(d.path(), &["commit", "-qm", "convert type"]);
                    Some("refs/heads/main")
                } else {
                    None
                };
                let c = capture(d.path(), comparison, base).unwrap();
                assert!(!c.partial);
                assert_eq!(c.files.len(), 1);
                let file = &c.files[0];
                assert_eq!(file.status, "Type changed");
                assert_eq!(
                    (file.additions, file.deletions),
                    if reverse {
                        (Some(2), Some(1))
                    } else {
                        (Some(1), Some(2))
                    }
                );
                assert!(file.notice.is_none());
                let patch = &c.patches[&file.id];
                for line in if reverse {
                    ["-somewhere", "+one", "+two"]
                } else {
                    ["+somewhere", "-one", "-two"]
                } {
                    assert!(patch.lines().any(|l| l == line), "missing {line}: {patch}");
                }
                assert!(patch.contains(if reverse {
                    "deleted file mode 120000"
                } else {
                    "deleted file mode 100644"
                }));
                assert!(patch.contains(if reverse {
                    "new file mode 100644"
                } else {
                    "new file mode 120000"
                }));
            }
        }
    }

    #[test]
    fn type_conversion_with_identical_bytes_keeps_zero_net_totals_and_full_patch() {
        let d = init();
        git(d.path(), &["config", "core.symlinks", "false"]);
        stage_file_mode(d.path(), "100644", b"somewhere");
        git(d.path(), &["commit", "-qm", "same content baseline"]);
        stage_file_mode(d.path(), "120000", b"somewhere");
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(!c.partial);
        assert_eq!(c.files.len(), 1);
        let file = &c.files[0];
        assert_eq!((file.additions, file.deletions), (Some(0), Some(0)));
        let patch = &c.patches[&file.id];
        assert!(patch.lines().any(|line| line == "-somewhere"));
        assert!(patch.lines().any(|line| line == "+somewhere"));
    }

    #[test]
    fn type_conversion_counts_only_net_changed_lines() {
        let d = init();
        git(d.path(), &["config", "core.symlinks", "false"]);
        stage_file_mode(d.path(), "120000", b"one\n");
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(!c.partial);
        assert_eq!(c.files.len(), 1);
        let file = &c.files[0];
        assert_eq!((file.additions, file.deletions), (Some(0), Some(1)));
        let patch = &c.patches[&file.id];
        for line in ["-one", "-two", "+one"] {
            assert!(patch.lines().any(|l| l == line));
        }
    }

    #[test]
    fn binary_type_conversion_keeps_content_and_totals_unavailable() {
        let d = init();
        git(d.path(), &["config", "core.symlinks", "false"]);
        stage_file_mode(d.path(), "100644", b"binary\0content");
        git(d.path(), &["commit", "-qm", "binary baseline"]);
        stage_file_mode(d.path(), "120000", b"somewhere");
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert_eq!(c.files.len(), 1);
        let file = &c.files[0];
        assert_eq!((file.additions, file.deletions), (None, None));
        assert_eq!(file.notice.as_deref(), Some("Binary file; no text patch."));
        assert!(c.patches.get(&file.id).is_none_or(String::is_empty));
    }

    #[test]
    fn type_conversion_applies_patch_budget_to_both_sides_together() {
        let d = init();
        git(d.path(), &["config", "core.symlinks", "false"]);
        // Each rendered side is below 256 KiB; their combined patch is above it.
        stage_file_mode(d.path(), "100644", "old\n".repeat(28_000).as_bytes());
        git(d.path(), &["commit", "-qm", "large text baseline"]);
        stage_file_mode(d.path(), "120000", "new\n".repeat(28_000).as_bytes());
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(c.partial);
        assert_eq!(c.files.len(), 1);
        let file = &c.files[0];
        assert_eq!((file.additions, file.deletions), (None, None));
        assert_eq!(
            file.notice.as_deref(),
            Some("Patch exceeds display budget.")
        );
        assert!(c.patches[&file.id].is_empty());
    }

    #[test]
    fn branch_mode_changes_ignore_worktree_filemode_configuration() {
        let d = init();
        git(d.path(), &["checkout", "-qb", "feature"]);
        git(d.path(), &["update-index", "--chmod=+x", "a.txt"]);
        git(d.path(), &["commit", "-qm", "executable"]);
        git(d.path(), &["config", "core.filemode", "false"]);
        let c = capture(d.path(), Comparison::Branch, Some("refs/heads/main")).unwrap();
        assert_eq!(c.files.len(), 1);
        assert_eq!(c.files[0].path, "a.txt");
        assert_eq!(
            (c.files[0].additions, c.files[0].deletions),
            (Some(0), Some(0))
        );
        let patch = &c.patches[&c.files[0].id];
        assert!(patch.contains("old mode 100644"));
        assert!(patch.contains("new mode 100755"));
    }
    #[test]
    fn non_symlink_checkouts_preserve_git_link_semantics() {
        let d = init();
        let repo = Repository::open(d.path()).unwrap();
        let id = repo.blob(b"target.txt").unwrap();
        git(
            d.path(),
            &[
                "update-index",
                "--add",
                "--cacheinfo",
                &format!("120000,{id},link"),
            ],
        );
        git(d.path(), &["commit", "-qm", "link"]);
        git(d.path(), &["config", "core.symlinks", "false"]);
        std::fs::write(d.path().join("link"), "target.txt").unwrap();
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(!c.partial);
        assert!(c.files.is_empty());
        git(d.path(), &["rm", "--cached", "link"]);
        git(d.path(), &["add", "link"]);
        let converted = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert_eq!(converted.files.len(), 1);
        assert_eq!(converted.files[0].path, "link");
        git(d.path(), &["rm", "--cached", "link"]);
        let untracked = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert_eq!(untracked.files.len(), 1);
        assert_eq!(untracked.files[0].path, "link");
    }
    #[test]
    fn crlf_checkout_normalization_matches_git_for_config_and_attributes() {
        for attributes in [None, Some("*.txt text eol=crlf\n"), Some("*.txt -text\n")] {
            let d = init();
            if let Some(attrs) = attributes {
                std::fs::write(d.path().join(".gitattributes"), attrs).unwrap();
                git(d.path(), &["add", "."]);
                git(d.path(), &["commit", "-qm", "attributes"]);
            }
            git(d.path(), &["config", "core.autocrlf", "true"]);
            std::fs::write(d.path().join("a.txt"), "one\r\ntwo\r\n").unwrap();
            let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
            if attributes.is_some_and(|a| a.contains("-text")) {
                assert_eq!(c.files.len(), 1);
            } else {
                assert!(!c.partial);
                assert!(
                    c.files.is_empty(),
                    "{:?}",
                    c.files.iter().map(|f| &f.path).collect::<Vec<_>>()
                );
            }
        }
        let d = init();
        let text = "x\n".repeat(INPUT_LIMIT / 2 + 1);
        std::fs::write(d.path().join("large.txt"), &text).unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "large"]);
        git(d.path(), &["config", "core.autocrlf", "true"]);
        std::fs::write(d.path().join("large.txt"), text.replace('\n', "\r\n")).unwrap();
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(!c.partial);
        assert!(c.files.is_empty());
    }
    #[test]
    fn sparse_index_files_preserve_clean_and_staged_content() {
        let d = init();
        git(d.path(), &["update-index", "--skip-worktree", "a.txt"]);
        std::fs::remove_file(d.path().join("a.txt")).unwrap();
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(!c.partial);
        assert!(c.files.is_empty());
        let repo = Repository::open(d.path()).unwrap();
        let id = repo.blob(b"staged\n").unwrap();
        git(
            d.path(),
            &["update-index", "--cacheinfo", &format!("100644,{id},a.txt")],
        );
        git(d.path(), &["update-index", "--skip-worktree", "a.txt"]);
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert_eq!(c.files.len(), 1);
        assert!(c.patches.values().any(|p| p.contains("+staged")));
    }
    #[test]
    fn staged_executable_modes_are_taken_from_index_when_filemode_is_untrusted() {
        let d = init();
        git(d.path(), &["config", "core.filemode", "false"]);
        git(d.path(), &["update-index", "--chmod=+x", "a.txt"]);
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert_eq!(c.files.len(), 1);
        assert!(c.patches.values().any(|p| p.contains("new mode 100755")));
        std::fs::write(d.path().join("new.txt"), "new\n").unwrap();
        git(d.path(), &["add", "new.txt"]);
        git(d.path(), &["update-index", "--chmod=+x", "new.txt"]);
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(c
            .patches
            .values()
            .any(|p| p.contains("new file mode 100755")));
    }
    #[test]
    fn forced_text_normalization_cannot_hide_large_raw_equal_changes() {
        let d = init();
        std::fs::write(d.path().join("large.txt"), "abc\r\n".repeat(230000)).unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "raw CRLF"]);
        std::fs::write(d.path().join(".gitattributes"), "*.txt text\n").unwrap();
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(c.partial);
        assert!(c.files.iter().any(|f| f.path == "large.txt"));
    }
    #[cfg(unix)]
    #[test]
    fn git_control_fifos_fail_closed_without_hanging() {
        const CHILD: &str = "ORKWORKS_CONTROL_FIFO_TEST";
        if std::env::var_os(CHILD).is_some() {
            use std::ffi::CString;
            let d = init();
            for name in [".gitattributes", ".gitignore"] {
                let path = d.path().join(name);
                let c = CString::new(path.as_os_str().as_encoded_bytes()).unwrap();
                assert_eq!(unsafe { libc::mkfifo(c.as_ptr(), 0o600) }, 0);
                assert!(capture(d.path(), Comparison::Uncommitted, None).is_err());
                std::fs::remove_file(path).unwrap();
            }
            return;
        }
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "git_changes::tests::git_control_fifos_fail_closed_without_hanging",
            ])
            .env(CHILD, "1")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            if Instant::now() > deadline {
                let _ = child.kill();
                let _ = child.wait();
                panic!("Git control FIFO blocked capture")
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    #[test]
    fn index_attributes_survive_missing_worktree_control_and_large_auto_preserves_crlf() {
        let d = init();
        std::fs::write(d.path().join(".gitattributes"), "*.txt text eol=crlf\n").unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "attrs"]);
        std::fs::write(d.path().join("a.txt"), "one\r\ntwo\r\n").unwrap();
        std::fs::remove_file(d.path().join(".gitattributes")).unwrap();
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert_eq!(c.files.len(), 1);
        assert_eq!(c.files[0].path, ".gitattributes");
        let d = init();
        std::fs::write(d.path().join("large.txt"), "abc\r\n".repeat(230000)).unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "raw"]);
        git(d.path(), &["config", "core.autocrlf", "true"]);
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(!c.partial);
        assert!(c.files.is_empty());
    }
    #[cfg(unix)]
    #[test]
    fn ignored_directories_are_skipped_before_reading_control_files() {
        use std::ffi::CString;
        let d = init();
        std::fs::write(d.path().join(".gitignore"), "ignored/\n").unwrap();
        git(d.path(), &["add", "."]);
        git(d.path(), &["commit", "-qm", "ignore"]);
        std::fs::create_dir(d.path().join("ignored")).unwrap();
        let path = CString::new(
            d.path()
                .join("ignored/.gitattributes")
                .as_os_str()
                .as_encoded_bytes(),
        )
        .unwrap();
        assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
        let c = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(!c.partial);
        assert!(c.files.is_empty());
    }
    #[test]
    fn snapshots_reject_foreign_subjects_and_expired_handles() {
        let d = init();
        let mut a = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        a.session_id = "one".into();
        let id = a.snapshot_id.clone();
        let mut store = CaptureStore::default();
        store.insert(a);
        assert!(store
            .find("two", &d.path().canonicalize().unwrap(), &id)
            .is_err());
        assert!(store
            .find("one", &d.path().canonicalize().unwrap(), &id)
            .is_ok());
        store.captures[0].created = Some(Instant::now() - Duration::from_secs(301));
        assert!(store
            .find("one", &d.path().canonicalize().unwrap(), &id)
            .is_err());
    }
    #[test]
    fn base_enumeration_prefers_main_and_rejects_revision_expressions() {
        let d = init();
        git(d.path(), &["checkout", "-qb", "feature"]);
        let b = bases(d.path()).unwrap();
        assert_eq!(b.default_ref.as_deref(), Some("refs/heads/main"));
        assert!(capture(d.path(), Comparison::Branch, Some("main~1")).is_err());
        git(d.path(), &["checkout", "--detach"]);
        assert!(capture(d.path(), Comparison::Branch, Some("refs/heads/main")).is_ok());
    }
    #[cfg(unix)]
    #[test]
    fn symlinks_are_link_text_and_special_files_are_never_opened() {
        use std::os::unix::fs::symlink;
        let d = init();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("secret"), "MUST NOT APPEAR").unwrap();
        symlink(outside.path(), d.path().join("escape")).unwrap();
        let a = capture(d.path(), Comparison::Uncommitted, None).unwrap();
        assert!(a.files.iter().any(|f| f.path == "escape"));
        assert!(!a.patches.values().any(|p| p.contains("MUST NOT APPEAR")));
        assert!(safe_path(Path::new("../escape")).is_err());
        assert!(safe_path(Path::new(".git/config")).is_err());
    }
}
