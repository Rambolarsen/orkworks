//! Bounded, read-only Git comparisons. Only private temporary repositories are written.
use git2::{Oid, Repository};
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
    let repo = Repository::discover(cwd).map_err(err)?;
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
    let head = match repo.head() {
        Ok(h) => Some(h.peel_to_commit().map_err(err)?),
        Err(e) if e.code() == git2::ErrorCode::UnbornBranch => None,
        Err(e) => return Err(err(e)),
    };
    let head_id = head.as_ref().map(|h| h.id());
    let head_tree = head.as_ref().map(|h| h.tree()).transpose().map_err(err)?;
    let mut base_oid = None;
    let mut baseline_id = head_id;
    let baseline = if comparison == Comparison::Branch {
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
        base_oid = Some(base);
        let common = repo.merge_bases(h, base).map_err(|_| {
            "No available common ancestor; fetch history in your coding tool.".to_owned()
        })?;
        if common.len() != 1 {
            return Err("Multiple merge bases; comparison unavailable.".into());
        }
        baseline_id = Some(common[0]);
        Some(
            repo.find_commit(common[0])
                .and_then(|c| c.tree())
                .map_err(err)?,
        )
    } else {
        head_tree.clone()
    };
    let old = tree_files(&repo, baseline.as_ref(), start)?;
    let target = if comparison == Comparison::Branch {
        tree_files(&repo, head_tree.as_ref(), start)?
    } else {
        BTreeMap::new()
    };
    let temp = tempfile::tempdir().map_err(err)?;
    let scratch = Repository::init(temp.path()).map_err(err)?;
    // libgit2 resolves only sanitized control inputs in this private worktree.
    // Git administrative/global config remains the selected repository's normal
    // metadata authority, copied with bounded nonblocking leaf reads.
    let config = repo.config().map_err(err)?;
    for (key, name) in [
        ("core.attributesfile", "global-attributes"),
        ("core.excludesfile", "global-excludes"),
    ] {
        let original = config.get_path(key).ok().or_else(|| {
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
        });
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
    let mut before = git2::Index::new().map_err(err)?;
    let mut after = git2::Index::new().map_err(err)?;
    let mut total = 0usize;
    let mut partial = false;
    let mut notices = Vec::new();
    let visited = std::cell::RefCell::new(HashSet::new());
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
    let mut seen_modules = HashSet::new();
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
            let data = blob(&repo, *id, *mode)?;
            control_size += data.len();
            if control_size > 4 * 1024 * 1024 {
                return Err("Git control index budget exceeded.".into());
            }
            put(&scratch, &mut controls, path, &data, *mode)?;
        }
    }
    controls.write().map_err(err)?;
    let effective_mode = |path: &str, mode: i32| {
        if [0o100644, 0o100755].contains(&mode) {
            if let Some((_, indexed, _)) = stage0.get(path) {
                if (literal_symlinks && *indexed == 0o120000)
                    || (!trust_filemode && [0o100644, 0o100755].contains(indexed))
                {
                    return *indexed;
                }
            }
        }
        mode
    };
    let mut compare = |path: &str,
                       new: Result<Option<(Vec<u8>, i32)>, String>|
     -> Result<(), String> {
        if start.elapsed() > Duration::from_secs(5) {
            return Err("Capture budget exceeded; comparison unavailable.".into());
        }
        visited.borrow_mut().insert(path.to_owned());
        let old_content = old
            .get(path)
            .map(|(id, m)| blob(&repo, *id, *m).map(|b| (b, *m)))
            .transpose();
        let pair = old_content.and_then(|old| {
            new.map(|mut new| {
                if comparison == Comparison::Uncommitted {
                    if let Some((_, mode)) = &mut new {
                        *mode = effective_mode(path, *mode);
                    }
                }
                (old, new)
            })
        });
        let (old, new) = match pair {
            Ok(p) => p,
            Err(n) => {
                partial = true;
                notices.push((path.to_owned(), n));
                return Ok(());
            }
        };
        if conflicts.contains(path) {
            partial = true;
            notices.push((path.into(), "Conflict; patch unavailable.".into()));
            return Ok(());
        }
        if old == new {
            return Ok(());
        }
        if before.len() + after.len() >= 1000
            || total + old.as_ref().map_or(0, |x| x.0.len()) + new.as_ref().map_or(0, |x| x.0.len())
                > TOTAL_LIMIT
        {
            partial = true;
            return Ok(());
        }
        for (content, index) in [(old, &mut before), (new, &mut after)] {
            if let Some((data, mut mode)) = content {
                total += data.len();
                if mode == 0o160000 {
                    mode = 0o100644
                }
                put(&scratch, index, path, &data, mode)?;
            }
        }
        Ok(())
    };
    if comparison == Comparison::Branch {
        let paths: HashSet<_> = old.keys().chain(target.keys()).cloned().collect();
        for p in paths {
            if start.elapsed() > Duration::from_secs(5) {
                return Err("Capture budget exceeded; comparison unavailable.".into());
            }
            if old.get(&p) == target.get(&p) {
                continue;
            }
            let new = target
                .get(&p)
                .map(|(id, m)| blob(&repo, *id, *m).map(|b| (b, *m)))
                .transpose();
            compare(&p, new)?;
        }
    } else {
        let count = std::cell::Cell::new(0usize);
        let exceeded = std::cell::Cell::new(false);
        let failure = std::cell::RefCell::new(None);
        crate::git_changes_fs::walk(
            &worktree,
            |entry| {
                count.set(count.get() + 1);
                if count.get() > CANDIDATES || start.elapsed() > Duration::from_secs(5) {
                    exceeded.set(true);
                    return false;
                }
                let Some(p) = entry.path.to_str() else {
                    exceeded.set(true);
                    return true;
                };
                if gitlinks.contains(p) {
                    seen_modules.insert(p.to_owned());
                }
                if !old.contains_key(p)
                    && !tracked.contains(p)
                    && scratch.status_should_ignore(&entry.path).unwrap_or(true)
                {
                    return true;
                }
                if matches!(entry.content, Ok(None)) {
                    visited.borrow_mut().insert(p.to_owned());
                    return true;
                }
                if let Err(e) = compare(p, entry.content.map(|b| b.map(|b| (b, entry.mode)))) {
                    *failure.borrow_mut() = Some(e);
                    return false;
                }
                true
            },
            |path, is_dir| {
                count.set(count.get() + 1);
                if count.get() > CANDIDATES || start.elapsed() > Duration::from_secs(5) {
                    exceeded.set(true);
                    return false;
                }
                let Some(p) = path.to_str() else {
                    exceeded.set(true);
                    return false;
                };
                if is_dir && gitlinks.contains(p) {
                    return false;
                }
                if is_dir {
                    if let Err(e) = std::fs::create_dir_all(temp.path().join(path)) {
                        *failure.borrow_mut() = Some(e.to_string());
                        return false;
                    }
                }
                let prefix = format!("{p}/");
                old.contains_key(p)
                    || tracked.contains(p)
                    || tracked.iter().any(|k| k.starts_with(&prefix))
                    || old.keys().any(|k| k.starts_with(&prefix))
                    || !scratch.status_should_ignore(path).unwrap_or(true)
            },
            |path, mode| {
                let path = path.to_str().ok_or("Unsupported path.")?;
                let mode = effective_mode(path, mode);
                let expected = old
                    .get(path)
                    .filter(|(_, m)| !conflicts.contains(path) && *m == mode)
                    .map(|(id, _)| *id);
                let eol = crate::git_changes_eol::policy(
                    &scratch,
                    &repo,
                    path,
                    stage0.get(path).map(|e| e.0),
                    old.get(path).map(|e| e.0),
                )?;
                Ok((expected, eol))
            },
            |prefix, attrs, ignore| {
                for (name, data) in [(".gitattributes", attrs), (".gitignore", ignore)] {
                    if let Some(data) = data {
                        let directory = temp.path().join(prefix);
                        std::fs::create_dir_all(&directory).map_err(err)?;
                        std::fs::write(directory.join(name), data).map_err(err)?;
                    }
                }
                Ok(())
            },
        )
        .map_err(err)?;
        if let Some(e) = failure.into_inner() {
            return Err(e);
        }
        for path in &gitlinks {
            if seen_modules.contains(path) {
                continue;
            }
            let directory = worktree.join(path);
            if !directory.exists() && stage0.get(path).is_some_and(|e| e.2) {
                continue;
            }
            let new = if !directory.exists() {
                Ok(None)
            } else if directory
                .canonicalize()
                .is_ok_and(|p| p.starts_with(&worktree))
            {
                repo.find_submodule(path)
                    .and_then(|module| module.open())
                    .and_then(|sub| sub.head().and_then(|h| h.peel_to_commit()).map(|c| c.id()))
                    .map(|id| Some((format!("Subproject commit {id}\n").into_bytes(), 0o160000)))
                    .map_err(|_| {
                        "Submodule commit unavailable; inspect its registered session.".into()
                    })
            } else {
                Err("Submodule directory outside this worktree; unavailable.".into())
            };
            compare(path, new)?;
        }
        if !exceeded.get() {
            let remaining: Vec<String> = old
                .keys()
                .chain(tracked.iter())
                .filter(|p| !visited.borrow().contains(*p))
                .cloned()
                .collect::<HashSet<_>>()
                .into_iter()
                .collect();
            for path in remaining {
                if let Some((id, mode, true)) = stage0.get(&path) {
                    if old.get(&path) == Some(&(*id, *mode)) {
                        continue;
                    }
                    compare(
                        &path,
                        blob(&repo, *id, *mode).map(|data| Some((data, *mode))),
                    )?;
                } else {
                    compare(&path, Ok(None))?;
                }
            }
        } else {
            drop(compare);
            partial = true;
        }
    }
    let before = before.write_tree_to(&scratch).map_err(err)?;
    let after = after.write_tree_to(&scratch).map_err(err)?;
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
    let mut files = Vec::new();
    let mut patches = HashMap::new();
    let mut output = 0;
    for (i, delta) in diff.deltas().enumerate() {
        if files.len() >= 500 || start.elapsed() > Duration::from_secs(5) {
            partial = true;
            break;
        }
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
        let mut notice = None;
        let mut additions = None;
        let mut deletions = None;
        let mut text = String::new();
        if let Some(mut patch) = git2::Patch::from_diff(&diff, i).map_err(err)? {
            if patch.delta().old_file().is_binary() || patch.delta().new_file().is_binary() {
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
                continue;
            }
            let bytes = patch.to_buf().map_err(err)?;
            if bytes.len() > PATCH_LIMIT || output + bytes.len() > TOTAL_LIMIT {
                notice = Some("Patch exceeds display budget.".into());
                partial = true
            } else if std::str::from_utf8(&bytes).is_err()
                || bytes.split(|b| *b == b'\n').any(|l| l.len() > 65536)
            {
                notice = Some("Unsupported text encoding or line too long.".into());
                partial = true
            } else {
                let (_, a, d) = patch.line_stats().map_err(err)?;
                additions = Some(a);
                deletions = Some(d);
                text = String::from_utf8(bytes.to_vec()).map_err(err)?;
                output += text.len();
            }
        } else {
            notice = Some("Binary file; no text patch.".into());
        }
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
    }
    for (path, n) in notices
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
    Ok(Capture {
        snapshot_id: uuid::Uuid::new_v4().to_string(),
        session_id: String::new(),
        worktree: worktree.display().to_string(),
        branch: repo
            .head()
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
            }),
        comparison,
        base_ref: base_ref.map(str::to_owned),
        base_oid: base_oid.map(|x| x.to_string()),
        baseline_oid: baseline_id.map(|x| x.to_string()),
        head_oid: head_id.map(|x| x.to_string()),
        captured_at: chrono::Utc::now().to_rfc3339(),
        partial,
        notice: partial
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
