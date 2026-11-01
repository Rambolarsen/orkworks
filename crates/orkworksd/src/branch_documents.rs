//! Branch-scoped document identity and bounded, read-only discovery.
use crate::metadata::{PlanReference, PlanSource};
use git2::{Oid, Repository};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    path::Path,
    time::{Duration, Instant, SystemTime},
};

const ROOTS: [&str; 3] = ["docs/superpowers/specs", "docs/superpowers/plans", "specs"];
const ENTRY_LIMIT: usize = 10_000;
const DOCUMENT_LIMIT: usize = 500;
const TEXT_LIMIT: usize = 4 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DocumentKind {
    Design,
    Implementation,
}
impl DocumentKind {
    pub(crate) const ALL: [Self; 2] = [Self::Design, Self::Implementation];
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Identity {
    pub common_dir: String,
    pub branch: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Document {
    pub kind: DocumentKind,
    pub reference: PlanReference,
    pub title: String,
    pub establishing_head: Option<String>,
    pub pinned: bool,
    pub observed_at: String,
}

#[derive(Clone, Debug)]
pub(crate) struct Scan {
    pub identity: Identity,
    pub documents: Vec<Document>,
    pub notice: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Step {
    pub kind: DocumentKind,
    pub available: bool,
    pub title: Option<String>,
    pub file_name: Option<String>,
    pub source: Option<PlanSource>,
    pub pinned: bool,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Projection {
    pub branch: Option<String>,
    pub shared_sessions: usize,
    pub steps: Vec<Step>,
    pub notice: Option<String>,
}

pub(crate) fn validate_document(id: &Identity, doc: &Document) -> Result<(), String> {
    let root = doc
        .reference
        .worktree_root
        .as_deref()
        .ok_or("Document anchor unavailable.")?;
    if identity(Path::new(root))? != *id {
        return Err("Document belongs to another branch.".into());
    }
    if !doc.pinned {
        if let Some(head) = &doc.establishing_head {
            let repo = Repository::open(root).map_err(|e| e.to_string())?;
            let old = Oid::from_str(head).map_err(|_| "Invalid document history.")?;
            let now = repo
                .head()
                .ok()
                .and_then(|h| h.target())
                .ok_or("Branch history unavailable.")?;
            if now != old && !repo.graph_descendant_of(now, old).unwrap_or(false) {
                return Err("Document history changed; rediscovery required.".into());
            }
        }
    }
    read_document(Path::new(root), &doc.reference.relative_path).map(|_| ())
}

pub(crate) fn resolve(
    store: &crate::branch_documents_store::Store,
    root: &Path,
    fresh: bool,
) -> Result<(Identity, Vec<Document>, Option<String>), String> {
    let scan = store.scan(root, fresh)?;
    let documents = store.reconcile(&scan)?;
    Ok((scan.identity, documents, scan.notice))
}

pub(crate) fn projection(store: &crate::branch_documents_store::Store, root: &Path) -> Projection {
    let resolved = resolve(store, root, false);
    let (id, documents, notice) = match resolved {
        Ok(result) => (Some(result.0), result.1, result.2),
        Err(error) => (None, Vec::new(), Some(error)),
    };
    let steps = DocumentKind::ALL
        .into_iter()
        .map(|kind| {
            let selected = documents.iter().find(|d| d.kind == kind);
            Step {
                kind,
                available: selected.is_some_and(|d| {
                    id.as_ref()
                        .is_some_and(|id| validate_document(id, d).is_ok())
                }),
                title: selected.map(|d| d.title.clone()),
                file_name: selected.and_then(|d| {
                    Path::new(&d.reference.relative_path)
                        .file_name()?
                        .to_str()
                        .map(str::to_owned)
                }),
                source: selected.map(|d| d.reference.source),
                pinned: selected.is_some_and(|d| d.pinned),
            }
        })
        .collect();
    Projection {
        branch: id.map(|id| id.branch.trim_start_matches("refs/heads/").into()),
        shared_sessions: 1,
        steps,
        notice,
    }
}

/// Bind fresh accepted reporting evidence to the reference's actual branch,
/// never to a session's launch directory or an unrelated sibling checkout.
pub(crate) fn publish(
    store: &crate::branch_documents_store::Store,
    reference: &PlanReference,
) -> Result<(), String> {
    if kind_for_path(&reference.relative_path).is_none() {
        return Ok(());
    }
    let root = Path::new(
        reference
            .worktree_root
            .as_deref()
            .ok_or("Document anchor unavailable.")?,
    );
    // Detached/non-Git legacy references retain their session-local behavior.
    let Ok(id) = identity(root) else {
        return Ok(());
    };
    let doc = document(
        root,
        &reference.relative_path,
        reference.source,
        reference.source == PlanSource::UserSelected,
    )?;
    store.report(&id, doc)
}

pub(crate) fn identity(cwd: &Path) -> Result<Identity, String> {
    let repo = Repository::discover(cwd).map_err(|_| "Git worktree unavailable.")?;
    if repo.is_bare() || repo.head_detached().unwrap_or(false) {
        return Err("No shared branch at detached HEAD.".into());
    }
    let branch = repo
        .find_reference("HEAD")
        .ok()
        .and_then(|r| r.symbolic_target().map(str::to_owned))
        .filter(|r| r.starts_with("refs/heads/"))
        .ok_or("Local branch unavailable.")?;
    let common =
        crate::session_location::canonical_git_path(repo.commondir()).map_err(str::to_owned)?;
    Ok(Identity {
        common_dir: common
            .to_str()
            .ok_or("Unsupported repository path.")?
            .into(),
        branch,
    })
}

pub(crate) fn kind_for_path(path: &str) -> Option<DocumentKind> {
    let path = Path::new(path);
    if crate::git_changes::safe_path(path).is_err()
        || path.to_str()?.chars().any(char::is_control)
        || !path.extension()?.eq_ignore_ascii_case("md")
    {
        return None;
    }
    if path.starts_with(ROOTS[1]) {
        Some(DocumentKind::Implementation)
    } else if path.starts_with(ROOTS[0]) || path.starts_with(ROOTS[2]) {
        Some(DocumentKind::Design)
    } else {
        None
    }
}
fn eligible(path: &Path) -> bool {
    path.to_str().and_then(kind_for_path).is_some()
        && !path.file_name().and_then(|n| n.to_str()).is_some_and(|n| {
            n.eq_ignore_ascii_case("readme.md") || n.eq_ignore_ascii_case("index.md")
        })
}

// Git's index stat cache avoids opening every inherited document just to
// discover that it is unchanged. A mismatched/unknown stat always gets read.
fn index_matches(root: &Path, path: &Path, index: &git2::Index, head: Option<Oid>) -> bool {
    let Some(entry) = index.get_path(path, 0) else {
        return false;
    };
    if Some(entry.id) != head || ![0o100644, 0o100755].contains(&entry.mode) {
        return false;
    }
    let Ok(meta) = std::fs::symlink_metadata(root.join(path)) else {
        return false;
    };
    if !meta.is_file() || meta.len() != u64::from(entry.file_size) {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        meta.mtime() == i64::from(entry.mtime.seconds())
            && meta.mtime_nsec() == i64::from(entry.mtime.nanoseconds())
            && meta.ctime() == i64::from(entry.ctime.seconds())
            && meta.ctime_nsec() == i64::from(entry.ctime.nanoseconds())
    }
    #[cfg(not(unix))]
    {
        let matches = |time: std::io::Result<SystemTime>, expected: git2::IndexTime| {
            time.ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .is_some_and(|t| {
                    t.as_secs() == expected.seconds() as u64
                        && t.subsec_nanos() == expected.nanoseconds()
                })
        };
        matches(meta.modified(), entry.mtime) && matches(meta.created(), entry.ctime)
    }
}

pub(crate) fn read_document(root: &Path, relative: &str) -> Result<String, String> {
    kind_for_path(relative).ok_or("Unsupported document path.")?;
    let bytes = crate::git_changes_fs::read_regular(root, Path::new(relative))
        .map_err(|_| "Document is unavailable or exceeds 1 MiB.")?;
    String::from_utf8(bytes).map_err(|_| "Document is not UTF-8.".into())
}

pub(crate) fn document(
    root: &Path,
    relative: &str,
    source: PlanSource,
    pinned: bool,
) -> Result<Document, String> {
    let kind = kind_for_path(relative).ok_or("Unsupported document path.")?;
    let content = read_document(root, relative)?;
    let repo = Repository::discover(root).map_err(|_| "Repository unavailable.")?;
    Ok(from_content(
        root,
        relative,
        kind,
        source,
        pinned,
        repo.head().ok().and_then(|h| h.target()),
        &content,
    ))
}

fn from_content(
    root: &Path,
    relative: &str,
    kind: DocumentKind,
    source: PlanSource,
    pinned: bool,
    head: Option<Oid>,
    content: &str,
) -> Document {
    let title = content
        .lines()
        .find_map(|line| line.strip_prefix("# "))
        .unwrap_or_else(|| {
            Path::new(relative)
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("Document")
        })
        .chars()
        .filter(|c| !c.is_control())
        .take(200)
        .collect();
    Document {
        kind,
        title,
        reference: PlanReference {
            worktree_root: Some(root.to_string_lossy().into_owned()),
            relative_path: relative.into(),
            source,
        },
        establishing_head: head.map(|id| id.to_string()),
        pinned,
        observed_at: chrono::Utc::now().to_rfc3339(),
    }
}

struct Budget {
    start: Instant,
    entries: usize,
}
impl Budget {
    fn visit(&mut self) -> Result<(), String> {
        self.entries += 1;
        if self.entries > ENTRY_LIMIT || self.start.elapsed() > Duration::from_secs(5) {
            Err("Document discovery limit reached; latest documents unavailable.".into())
        } else {
            Ok(())
        }
    }
}

fn tree_documents(
    repo: &Repository,
    tree: Option<&git2::Tree<'_>>,
    budget: &mut Budget,
) -> Result<BTreeMap<String, Oid>, String> {
    fn walk(
        repo: &Repository,
        tree: &git2::Tree<'_>,
        prefix: &Path,
        out: &mut BTreeMap<String, Oid>,
        budget: &mut Budget,
    ) -> Result<(), String> {
        for entry in tree {
            budget.visit()?;
            let name = entry.name().ok_or("Unsupported document path.")?;
            let path = prefix.join(name);
            if entry.kind() == Some(git2::ObjectType::Tree) {
                walk(
                    repo,
                    &repo.find_tree(entry.id()).map_err(|e| e.to_string())?,
                    &path,
                    out,
                    budget,
                )?;
            } else if [0o100644, 0o100755].contains(&entry.filemode()) && eligible(&path) {
                if out.len() >= DOCUMENT_LIMIT {
                    return Err("Document discovery limit reached.".into());
                }
                out.insert(
                    path.to_str().ok_or("Unsupported document path.")?.into(),
                    entry.id(),
                );
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    if let Some(tree) = tree {
        for root in ROOTS {
            if let Ok(entry) = tree.get_path(Path::new(root)) {
                if entry.kind() == Some(git2::ObjectType::Tree) {
                    walk(
                        repo,
                        &repo.find_tree(entry.id()).map_err(|e| e.to_string())?,
                        Path::new(root),
                        &mut out,
                        budget,
                    )?;
                }
            }
        }
    }
    Ok(out)
}

pub(crate) fn discover(cwd: &Path) -> Result<Scan, String> {
    let identity = identity(cwd)?;
    let repo = Repository::discover(cwd).map_err(|e| e.to_string())?;
    let root = repo
        .workdir()
        .ok_or("No worktree.")?
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let head = repo.head().ok().and_then(|h| h.target());
    let mut budget = Budget {
        start: Instant::now(),
        entries: 0,
    };
    let head_tree = head
        .map(|id| repo.find_commit(id).and_then(|c| c.tree()))
        .transpose()
        .map_err(|e| e.to_string())?;
    let tracked = tree_documents(&repo, head_tree.as_ref(), &mut budget)?;
    let mut notice = None;
    let mut committed = Vec::new();
    if let Some(head) = head {
        let history = (|| -> Result<Vec<String>, String> {
            let bases = crate::git_changes::bases(&root)?;
            if bases.limited {
                return Err("Base branch inspection was incomplete.".into());
            }
            let base_ref = bases
                .default_ref
                .ok_or("No default base available for committed document discovery.")?;
            let base = repo
                .find_reference(&base_ref)
                .and_then(|r| r.peel_to_commit())
                .map_err(|e| e.to_string())?
                .id();
            let bases = repo
                .merge_bases(head, base)
                .map_err(|_| "Branch history unavailable; no common ancestor.")?;
            if bases.len() != 1 {
                return Err("Branch history has ambiguous merge bases.".into());
            }
            let stop = bases[0];
            let base_tree = repo
                .find_commit(stop)
                .and_then(|c| c.tree())
                .map_err(|e| e.to_string())?;
            let mut remaining = tracked
                .iter()
                .filter(|(path, id)| {
                    base_tree
                        .get_path(Path::new(path.as_str()))
                        .map(|e| e.id() != **id)
                        .unwrap_or(true)
                })
                .map(|(path, _)| path.clone())
                .collect::<std::collections::BTreeSet<_>>();
            let mut out = Vec::new();
            let mut next = head;
            while next != stop && !remaining.is_empty() {
                budget.visit()?;
                let commit = repo.find_commit(next).map_err(|e| e.to_string())?;
                let current = commit.tree().map_err(|e| e.to_string())?;
                let parent = commit.parent(0).map_err(|_| "Branch history incomplete.")?;
                let previous = parent.tree().map_err(|e| e.to_string())?;
                let changed: Vec<_> = remaining
                    .iter()
                    .filter(|path| {
                        current
                            .get_path(Path::new(path.as_str()))
                            .ok()
                            .map(|e| e.id())
                            != previous
                                .get_path(Path::new(path.as_str()))
                                .ok()
                                .map(|e| e.id())
                    })
                    .cloned()
                    .collect();
                for path in changed {
                    budget.visit()?;
                    remaining.remove(&path);
                    out.push(path);
                }
                next = parent.id();
            }
            Ok(out)
        })();
        match history {
            Ok(paths) => committed = paths,
            Err(e) => notice = Some(e),
        }
    }
    // Only the private repository sees ignore controls; source controls are read
    // through anchored no-follow handles by the existing worktree walker.
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let scratch = Repository::init(temp.path()).map_err(|e| e.to_string())?;
    let excludes = temp.path().join("excludes");
    std::fs::write(&excludes, "").map_err(|e| e.to_string())?;
    scratch
        .config()
        .and_then(|mut c| c.set_str("core.excludesfile", excludes.to_str().unwrap_or("")))
        .map_err(|e| e.to_string())?;
    for (key, filename, default_name) in [
        ("core.excludesfile", "global-ignore", "ignore"),
        ("core.attributesfile", "global-attributes", "attributes"),
    ] {
        let configured = repo
            .config()
            .ok()
            .and_then(|c| c.get_path(key).ok())
            .or_else(|| {
                std::env::var_os("XDG_CONFIG_HOME")
                    .map(std::path::PathBuf::from)
                    .or_else(|| dirs::home_dir().map(|home| home.join(".config")))
                    .map(|base| base.join("git").join(default_name))
            });
        let data = if let Some(path) = configured {
            crate::git_changes_fs::metadata_file(&if path.is_absolute() {
                path
            } else {
                root.join(path)
            })
            .map_err(|e| e.to_string())?
            .unwrap_or_default()
        } else {
            Vec::new()
        };
        let path = temp.path().join(filename);
        std::fs::write(&path, data).map_err(|e| e.to_string())?;
        scratch
            .config()
            .and_then(|mut c| {
                c.set_str(
                    key,
                    path.to_str()
                        .ok_or_else(|| git2::Error::from_str("Unsupported control path"))?,
                )
            })
            .map_err(|e| e.to_string())?;
    }
    if let Some(data) = crate::git_changes_fs::metadata_file(&repo.commondir().join("info/exclude"))
        .map_err(|e| e.to_string())?
    {
        std::fs::write(scratch.path().join("info/exclude"), data).map_err(|e| e.to_string())?;
    }
    let index = repo.index().map_err(|e| e.to_string())?;
    let indexed: std::collections::HashSet<_> = index
        .iter()
        .filter_map(|e| String::from_utf8(e.path).ok())
        .collect();
    let conflicted: std::collections::HashSet<_> = index
        .iter()
        .filter(|e| e.flags & 0x3000 != 0)
        .filter_map(|e| String::from_utf8(e.path).ok())
        .collect();
    let mut dirty = Vec::new();
    let mut available = HashMap::new();
    let bytes = std::cell::Cell::new(0usize);
    let entries = std::cell::Cell::new(budget.entries);
    let failure = std::cell::RefCell::new(None);
    let walked = crate::git_changes_fs::walk(
        &root,
        |entry| {
            if ![0o100644, 0o100755].contains(&entry.mode) || !eligible(&entry.path) {
                return true;
            }
            let Some(path) = entry.path.to_str() else {
                return true;
            };
            if conflicted.contains(path) {
                *failure.borrow_mut() =
                    Some("Document has unresolved Git conflicts; latest unavailable.".into());
                return false;
            }
            if !indexed.contains(path) && scratch.status_should_ignore(&entry.path).unwrap_or(true)
            {
                return true;
            }
            let data = match entry.content {
                Ok(Some(data)) => data,
                _ => {
                    *failure.borrow_mut() =
                        Some("Document unreadable or oversized; latest unavailable.".into());
                    return false;
                }
            };
            bytes.set(bytes.get() + data.len());
            if bytes.get() > TEXT_LIMIT || available.len() >= DOCUMENT_LIMIT {
                *failure.borrow_mut() = Some("Document discovery limit reached.".into());
                return false;
            }
            let id = Oid::hash_object(git2::ObjectType::Blob, &data).ok();
            if tracked.get(path).copied() != id {
                let modified = std::fs::symlink_metadata(root.join(path))
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                dirty.push((modified, path.to_owned()));
            }
            available.insert(path.to_owned(), data);
            true
        },
        |path, dir| {
            entries.set(entries.get() + 1);
            if entries.get() > ENTRY_LIMIT || budget.start.elapsed() > Duration::from_secs(5) {
                *failure.borrow_mut() = Some("Document discovery limit reached.".into());
                return false;
            }
            if dir {
                ROOTS
                    .iter()
                    .any(|r| Path::new(r).starts_with(path) || path.starts_with(r))
            } else {
                eligible(path)
                    && (path
                        .to_str()
                        .is_some_and(|p| committed.iter().any(|candidate| candidate == p))
                        || !index_matches(
                            &root,
                            path,
                            &index,
                            path.to_str().and_then(|p| tracked.get(p).copied()),
                        ))
            }
        },
        |path, _| {
            let path = path.to_str().ok_or("Unsupported document path.")?;
            let indexed = index.get_path(Path::new(path), 0).map(|e| e.id);
            Ok((
                None,
                crate::git_changes_eol::policy(
                    &scratch,
                    &repo,
                    path,
                    indexed,
                    tracked.get(path).copied(),
                )?,
            ))
        },
        |prefix, attrs, ignore| {
            for (name, data) in [(".gitattributes", attrs), (".gitignore", ignore)] {
                if let Some(data) = data {
                    let path = temp.path().join(prefix).join(name);
                    std::fs::create_dir_all(path.parent().ok_or("Invalid control path.")?)
                        .map_err(|e| e.to_string())?;
                    std::fs::write(path, data).map_err(|e| e.to_string())?;
                }
            }
            Ok(())
        },
    );
    if let Err(e) = walked {
        *failure.borrow_mut() = Some(e.to_string());
    }
    if let Some(error) = failure.into_inner() {
        return Ok(Scan {
            identity,
            documents: Vec::new(),
            notice: Some(error),
        });
    }
    dirty.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    let mut documents = Vec::new();
    for (path, source) in dirty
        .into_iter()
        .map(|(_, p)| (p, PlanSource::GitWorktree))
        .chain(committed.into_iter().map(|p| (p, PlanSource::GitHistory)))
    {
        let Some(kind) = kind_for_path(&path) else {
            continue;
        };
        if documents.iter().any(|d: &Document| d.kind == kind) || !available.contains_key(&path) {
            continue;
        }
        let content = std::str::from_utf8(available.get(&path).ok_or("Document unavailable.")?)
            .map_err(|_| "Document is not UTF-8.")?;
        documents.push(from_content(
            &root, &path, kind, source, false, head, content,
        ));
    }
    documents.sort_by_key(|d| if d.kind == DocumentKind::Design { 0 } else { 1 });
    if identity != self::identity(&root)? || repo.head().ok().and_then(|h| h.target()) != head {
        return Err("Branch changed during document discovery; refresh.".into());
    }
    Ok(Scan {
        identity,
        documents,
        notice,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};

    fn write(root: &Path, path: &str, content: &str) {
        fs::create_dir_all(root.join(path).parent().unwrap()).unwrap();
        fs::write(root.join(path), content).unwrap();
    }
    fn commit(repo: &git2::Repository) {
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let parent = repo.head().ok().map(|h| h.peel_to_commit().unwrap());
        let sig = git2::Signature::now("test", "test@example.com").unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "change",
            &tree,
            &parent.iter().collect::<Vec<_>>(),
        )
        .unwrap();
    }
    fn fixture() -> (tempfile::TempDir, git2::Repository) {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        repo.set_head("refs/heads/main").unwrap();
        write(dir.path(), "specs/inherited.md", "# Inherited\n");
        commit(&repo);
        repo.branch(
            "task",
            &repo.head().unwrap().peel_to_commit().unwrap(),
            false,
        )
        .unwrap();
        repo.set_head("refs/heads/task").unwrap();
        (dir, repo)
    }

    #[test]
    fn recovers_both_committed_documents_without_attaching_inherited_specs() {
        let (dir, repo) = fixture();
        write(
            dir.path(),
            "docs/superpowers/specs/design.md",
            "# Task design\n",
        );
        write(
            dir.path(),
            "docs/superpowers/plans/plan.md",
            "# Task plan\n",
        );
        commit(&repo);
        let scan = discover(dir.path()).unwrap();
        assert!(scan.notice.is_none(), "{:?}", scan.notice);
        assert_eq!(scan.documents[0].title, "Task design");
        assert_eq!(scan.documents[1].title, "Task plan");
        assert!(scan
            .documents
            .iter()
            .all(|d| d.reference.relative_path != "specs/inherited.md"));
        assert_eq!(scan.identity.branch, "refs/heads/task");
    }

    #[test]
    fn dirty_documents_supersede_commits_and_ignore_indexes_and_ignored_files() {
        let (dir, repo) = fixture();
        write(dir.path(), "docs/superpowers/plans/old.md", "# Old plan\n");
        commit(&repo);
        write(
            dir.path(),
            ".gitignore",
            "docs/superpowers/plans/ignored.md\n",
        );
        write(
            dir.path(),
            "docs/superpowers/plans/new.md",
            "# Current plan\n",
        );
        write(
            dir.path(),
            "docs/superpowers/plans/ignored.md",
            "# Ignore me\n",
        );
        write(dir.path(), "specs/README.md", "# Not a spec\n");
        let scan = discover(dir.path()).unwrap();
        assert!(scan.notice.is_none(), "{:?}", scan.notice);
        assert_eq!(scan.documents.len(), 1);
        assert_eq!(scan.documents[0].title, "Current plan");
        assert_eq!(scan.documents[0].kind, DocumentKind::Implementation);
    }

    #[test]
    fn latest_commit_order_handles_reused_branches_and_deleted_documents() {
        let (dir, repo) = fixture();
        write(dir.path(), "specs/first.md", "# First work\n");
        commit(&repo);
        write(dir.path(), "specs/second.md", "# Second work\n");
        commit(&repo);
        let scan = discover(dir.path()).unwrap();
        assert_eq!(scan.documents[0].title, "Second work");
        fs::remove_file(dir.path().join("specs/second.md")).unwrap();
        let scan = discover(dir.path()).unwrap();
        assert_eq!(scan.documents[0].title, "First work");
    }

    #[test]
    fn branch_identity_is_shared_with_subdirectories_but_not_other_repositories() {
        let (dir, _) = fixture();
        fs::create_dir(dir.path().join("child")).unwrap();
        let a = identity(dir.path()).unwrap();
        assert_eq!(a, identity(&dir.path().join("child")).unwrap());
        let (other, _) = fixture();
        assert_ne!(a, identity(other.path()).unwrap());
    }

    #[test]
    fn session_projection_shares_both_slots_only_within_the_same_repository_branch() {
        let (dir, _) = fixture();
        write(dir.path(), "specs/current.md", "# Shared design\n");
        write(
            dir.path(),
            "docs/superpowers/plans/current.md",
            "# Shared plan\n",
        );
        let (other, _) = fixture();
        let state = crate::test_support::test_app_state_with_workspace(dir.path());
        let infos = [
            ("one", dir.path()),
            ("two", dir.path()),
            ("other", other.path()),
        ]
        .into_iter()
        .map(|(id, root)| {
            let mut meta = crate::test_support::test_session_metadata(
                id,
                id,
                root.to_str().unwrap(),
                "ended",
                "now",
                "now",
            );
            meta.cwd = root.to_string_lossy().into_owned();
            crate::session_types::SessionInfo::baseline_from_metadata(&meta)
        })
        .collect();
        let projected =
            crate::session_projection::SessionProjection::new(state).enrich_workspace(infos);
        for info in &projected[..2] {
            let docs = info.plan_documents.as_ref().unwrap();
            assert_eq!(docs.shared_sessions, 2);
            assert_eq!(docs.steps.len(), 2);
            assert!(docs.steps.iter().all(|step| step.available));
            assert_eq!(info.has_openable_plan, Some(true));
        }
        assert_eq!(
            projected[2]
                .plan_documents
                .as_ref()
                .unwrap()
                .shared_sessions,
            1
        );
    }

    #[test]
    fn clean_crlf_checkouts_do_not_promote_inherited_specs_to_current_work() {
        let (dir, repo) = fixture();
        repo.config()
            .unwrap()
            .set_bool("core.autocrlf", true)
            .unwrap();
        write(dir.path(), "specs/inherited.md", "# Inherited\r\n");
        let scan = discover(dir.path()).unwrap();
        assert!(scan.documents.is_empty(), "{:?}", scan.documents);
    }

    #[test]
    fn large_unchanged_inherited_history_does_not_hide_a_small_current_plan() {
        let (dir, repo) = fixture();
        repo.set_head("refs/heads/main").unwrap();
        for i in 0..16 {
            write(
                dir.path(),
                &format!("specs/old-{i}.md"),
                &"old\n".repeat(80_000),
            );
        }
        commit(&repo);
        let tip = repo.head().unwrap().target().unwrap();
        repo.find_reference("refs/heads/task")
            .unwrap()
            .set_target(tip, "test")
            .unwrap();
        repo.set_head("refs/heads/task").unwrap();
        write(
            dir.path(),
            "docs/superpowers/plans/current.md",
            "# Current plan\n",
        );
        let scan = discover(dir.path()).unwrap();
        assert!(scan.notice.is_none(), "{:?}", scan.notice);
        assert_eq!(scan.documents.len(), 1);
        assert_eq!(scan.documents[0].title, "Current plan");
    }

    #[test]
    fn refreshing_the_same_path_updates_its_title_and_commit_evidence() {
        let (dir, repo) = fixture();
        write(dir.path(), "specs/current.md", "# Before\n");
        commit(&repo);
        let storage = tempfile::tempdir().unwrap();
        let store = crate::branch_documents_store::Store::new(storage.path());
        let (_, first, _) = resolve(&store, dir.path(), true).unwrap();
        assert_eq!(first[0].title, "Before");
        write(dir.path(), "specs/current.md", "# After\n");
        commit(&repo);
        let (_, second, _) = resolve(&store, dir.path(), true).unwrap();
        assert_eq!(second[0].title, "After");
        assert_ne!(first[0].establishing_head, second[0].establishing_head);
    }

    #[test]
    fn discovery_limits_do_not_publish_a_false_latest_document() {
        let (dir, _) = fixture();
        for i in 0..501 {
            write(dir.path(), &format!("specs/new-{i}.md"), "# Design\n");
        }
        let scan = discover(dir.path()).unwrap();
        assert!(scan.notice.is_some());
        assert!(scan.documents.is_empty());
    }

    #[test]
    fn unborn_branches_discover_documents_and_detached_heads_do_not_invent_branches() {
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        repo.set_head("refs/heads/task").unwrap();
        write(dir.path(), "specs/new.md", "# New design\n");
        let scan = discover(dir.path()).unwrap();
        assert_eq!(scan.documents[0].title, "New design");
        commit(&repo);
        repo.set_head_detached(repo.head().unwrap().target().unwrap())
            .unwrap();
        assert!(identity(dir.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn document_reads_reject_symlink_ancestors_leaves_and_fifos() {
        use std::os::unix::fs::symlink;
        let (dir, _) = fixture();
        let outside = tempfile::tempdir().unwrap();
        write(outside.path(), "foreign.md", "private");
        symlink(outside.path(), dir.path().join("specs/link")).unwrap();
        symlink(
            outside.path().join("foreign.md"),
            dir.path().join("specs/leaf.md"),
        )
        .unwrap();
        assert!(read_document(dir.path(), "specs/link/foreign.md").is_err());
        assert!(read_document(dir.path(), "specs/leaf.md").is_err());
        let fifo =
            std::ffi::CString::new(dir.path().join("specs/fifo.md").to_str().unwrap()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(fifo.as_ptr(), 0o600) }, 0);
        assert!(read_document(dir.path(), "specs/fifo.md").is_err());
        assert!(read_document(dir.path(), "specs/../inherited.md").is_err());
    }
}
