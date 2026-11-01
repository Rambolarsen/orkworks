//! Additive branch selections under the owning workspace's single-writer lease.
use crate::branch_documents::{Document, DocumentKind, Identity, Scan};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, Weak},
};

pub(crate) struct Store {
    root: PathBuf,
    locks: Mutex<HashMap<String, Weak<Mutex<()>>>>,
    scans: Mutex<HashMap<PathBuf, (std::time::Instant, Identity, Option<git2::Oid>, Scan)>>,
}
#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    version: u32,
    identity: Identity,
    documents: Vec<Document>,
}
impl Store {
    pub(crate) fn new(root: &Path) -> Self {
        Self {
            root: root.join("branch-documents"),
            locks: Mutex::new(HashMap::new()),
            scans: Mutex::new(HashMap::new()),
        }
    }
    pub(crate) fn scan(&self, root: &Path, fresh: bool) -> Result<Scan, String> {
        let root = root.canonicalize().map_err(|_| "Worktree unavailable.")?;
        let id = crate::branch_documents::identity(&root)?;
        let repo = git2::Repository::open(&root).map_err(|_| "Worktree unavailable.")?;
        let head = repo.head().ok().and_then(|h| h.target());
        let now = std::time::Instant::now();
        {
            let mut scans = self
                .scans
                .lock()
                .map_err(|_| "Document cache unavailable.")?;
            scans.retain(|_, (at, _, _, _)| {
                now.duration_since(*at) < std::time::Duration::from_secs(5)
            });
            if !fresh {
                if let Some((_, key, oid, scan)) = scans.get(&root) {
                    if *key == id && *oid == head {
                        return Ok(scan.clone());
                    }
                }
            }
        }
        let result = crate::branch_documents::discover(&root)?;
        let mut scans = self
            .scans
            .lock()
            .map_err(|_| "Document cache unavailable.")?;
        if scans.len() >= 128 {
            scans.clear();
        }
        scans.insert(root, (now, id, head, result.clone()));
        Ok(result)
    }
    fn key(id: &Identity) -> String {
        let mut hash = Sha256::new();
        for field in [&id.common_dir, &id.branch] {
            hash.update((field.len() as u64).to_be_bytes());
            hash.update(field.as_bytes());
        }
        hex::encode(hash.finalize())
    }
    fn path(&self, id: &Identity) -> PathBuf {
        self.root.join(format!("{}.json", Self::key(id)))
    }
    fn lock(&self, id: &Identity) -> Arc<Mutex<()>> {
        let mut locks = self.locks.lock().expect("Branch selection lock poisoned");
        locks.retain(|_, lock| lock.strong_count() > 0);
        let key = Self::key(id);
        if let Some(lock) = locks.get(&key).and_then(Weak::upgrade) {
            return lock;
        }
        let lock = Arc::new(Mutex::new(()));
        locks.insert(key, Arc::downgrade(&lock));
        lock
    }
    pub(crate) fn load(&self, id: &Identity) -> Result<Vec<Document>, String> {
        let data = match crate::git_changes_fs::metadata_file(&self.path(id)) {
            Ok(Some(data)) => data,
            Ok(None) => return Ok(Vec::new()),
            Err(_) => return Err("Branch document record unreadable.".into()),
        };
        let record: Record = serde_json::from_slice(&data)
            .map_err(|_| "Branch document record malformed; preserved without changes.")?;
        if record.version != 1
            || record.identity != *id
            || record.documents.len() > 2
            || DocumentKind::ALL
                .iter()
                .any(|kind| record.documents.iter().filter(|d| d.kind == *kind).count() > 1)
            || record.documents.iter().any(|d| {
                crate::branch_documents::kind_for_path(&d.reference.relative_path) != Some(d.kind)
            })
        {
            return Err("Unsupported branch document record; preserved without changes.".into());
        }
        Ok(record.documents)
    }
    fn save(&self, id: &Identity, documents: Vec<Document>) -> Result<(), String> {
        std::fs::create_dir_all(&self.root)
            .map_err(|_| "Could not create branch document storage.")?;
        let text = serde_json::to_string(&Record {
            version: 1,
            identity: id.clone(),
            documents,
        })
        .map_err(|e| e.to_string())?;
        crate::metadata::write_atomic(&self.path(id), &text)
            .map_err(|_| "Could not save branch documents.".into())
    }
    pub(crate) fn report(&self, id: &Identity, document: Document) -> Result<(), String> {
        self.scans
            .lock()
            .map_err(|_| "Document cache unavailable.")?
            .clear();
        let lock = self.lock(id);
        let _guard = lock.lock().map_err(|_| "Branch selection unavailable.")?;
        let mut current = self.load(id)?;
        if current
            .iter()
            .any(|d| d.kind == document.kind && d.pinned && !document.pinned)
        {
            return Ok(());
        }
        current.retain(|d| d.kind != document.kind);
        current.push(document);
        self.save(id, current)
    }
    pub(crate) fn reconcile(&self, scan: &Scan) -> Result<Vec<Document>, String> {
        let lock = self.lock(&scan.identity);
        let _guard = lock.lock().map_err(|_| "Branch selection unavailable.")?;
        let mut current = self.load(&scan.identity)?;
        let before = current.clone();
        for candidate in &scan.documents {
            if current.iter().any(|d| d.kind == candidate.kind && d.pinned) {
                continue;
            }
            if scan.notice.is_some()
                && candidate.reference.source != crate::metadata::PlanSource::GitWorktree
            {
                continue;
            }
            if let Some(old) = current.iter().find(|d| d.kind == candidate.kind) {
                // Preserve stronger provenance for the very same document and
                // avoid rewriting identical selections on every polling pass.
                if old.reference.worktree_root == candidate.reference.worktree_root
                    && old.reference.relative_path == candidate.reference.relative_path
                {
                    let mut refreshed = candidate.clone();
                    if matches!(
                        old.reference.source,
                        crate::metadata::PlanSource::AgentReported
                            | crate::metadata::PlanSource::HookReported
                            | crate::metadata::PlanSource::TerminalFallback
                    ) {
                        refreshed.reference.source = old.reference.source;
                    }
                    refreshed.observed_at = old.observed_at.clone();
                    if *old == refreshed {
                        continue;
                    }
                    current.retain(|d| d.kind != candidate.kind);
                    current.push(refreshed);
                    continue;
                }
            }
            current.retain(|d| d.kind != candidate.kind);
            current.push(candidate.clone());
        }
        current.sort_by_key(|d| if d.kind == DocumentKind::Design { 0 } else { 1 });
        if current != before {
            self.save(&scan.identity, current.clone())?;
        }
        Ok(current)
    }
    pub(crate) fn use_latest(&self, id: &Identity, kind: DocumentKind) -> Result<(), String> {
        self.scans
            .lock()
            .map_err(|_| "Document cache unavailable.")?
            .clear();
        let lock = self.lock(id);
        let _guard = lock.lock().map_err(|_| "Branch selection unavailable.")?;
        let mut current = self.load(id)?;
        current.retain(|d| d.kind != kind);
        self.save(id, current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::branch_documents::{Document, DocumentKind, Identity, Scan};
    use crate::metadata::{PlanReference, PlanSource};
    fn identity(branch: &str) -> Identity {
        Identity {
            common_dir: "/repo/.git".into(),
            branch: format!("refs/heads/{branch}"),
        }
    }
    fn doc(path: &str, pinned: bool) -> Document {
        Document {
            kind: DocumentKind::Implementation,
            reference: PlanReference {
                worktree_root: Some("/repo".into()),
                relative_path: path.into(),
                source: if pinned {
                    PlanSource::UserSelected
                } else {
                    PlanSource::GitHistory
                },
            },
            title: path.into(),
            establishing_head: None,
            pinned,
            observed_at: "2026-11-01T06:00:00Z".into(),
        }
    }

    #[test]
    fn explicit_selection_survives_restart_and_different_session_discovery() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let id = identity("task");
        store
            .report(&id, doc("docs/superpowers/plans/pinned.md", true))
            .unwrap();
        let other_session = Store::new(dir.path());
        let result = other_session
            .reconcile(&Scan {
                identity: id.clone(),
                documents: vec![doc("docs/superpowers/plans/latest.md", false)],
                notice: None,
            })
            .unwrap();
        assert!(result[0].pinned);
        assert!(result[0].reference.relative_path.ends_with("pinned.md"));
        other_session
            .use_latest(&id, DocumentKind::Implementation)
            .unwrap();
        let result = store
            .reconcile(&Scan {
                identity: id,
                documents: vec![doc("docs/superpowers/plans/latest.md", false)],
                notice: None,
            })
            .unwrap();
        assert!(!result[0].pinned);
        assert!(result[0].reference.relative_path.ends_with("latest.md"));
    }

    #[test]
    fn branch_switch_and_corrupt_future_records_do_not_overwrite_other_work() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let a = identity("a");
        let b = identity("b");
        store
            .report(&a, doc("docs/superpowers/plans/a.md", true))
            .unwrap();
        assert!(store.load(&b).unwrap().is_empty());
        let path = store.path(&a);
        std::fs::write(&path, b"{\"version\":999}").unwrap();
        assert!(store
            .report(&a, doc("docs/superpowers/plans/b.md", true))
            .is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"{\"version\":999}");
    }

    #[test]
    fn incomplete_scan_preserves_established_automatic_document() {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::new(dir.path());
        let id = identity("task");
        store
            .report(&id, doc("docs/superpowers/plans/known.md", false))
            .unwrap();
        let result = store
            .reconcile(&Scan {
                identity: id,
                documents: vec![],
                notice: Some("Limit".into()),
            })
            .unwrap();
        assert!(result[0].reference.relative_path.ends_with("known.md"));
    }
}
