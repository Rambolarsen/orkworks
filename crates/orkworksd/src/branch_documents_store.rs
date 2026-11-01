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
    generation: std::sync::atomic::AtomicU64,
    #[cfg(test)]
    pub(crate) before_scan: Mutex<Option<Box<dyn FnOnce() + Send>>>,
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
            generation: std::sync::atomic::AtomicU64::new(0),
            #[cfg(test)]
            before_scan: Mutex::new(None),
        }
    }
    pub(crate) fn scan(&self, root: &Path, fresh: bool) -> Result<Scan, String> {
        let generation = self.generation.load(std::sync::atomic::Ordering::Acquire);
        #[cfg(test)]
        {
            let hook = self.before_scan.lock().unwrap().take();
            if let Some(hook) = hook {
                hook();
            }
        }
        let root = crate::git::worktree_root(root)
            .ok_or("Worktree unavailable.")?
            .canonicalize()
            .map_err(|_| "Worktree unavailable.")?;
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
                    if *key == id && *oid == head && scan.generation == Some(generation) {
                        return Ok(scan.clone());
                    }
                }
            }
        }
        let mut result = crate::branch_documents::discover(&root)?;
        result.generation = Some(generation);
        let mut scans = self
            .scans
            .lock()
            .map_err(|_| "Document cache unavailable.")?;
        if scans.len() >= 128 {
            scans.clear();
        }
        if generation == self.generation.load(std::sync::atomic::Ordering::Acquire) {
            scans.insert(root, (now, id, head, result.clone()));
        }
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
        // Invalidates every in-flight scan before publication, including scans
        // which have already left the cache but have not reconciled yet.
        self.generation
            .fetch_update(
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
                |n| n.checked_add(1),
            )
            .map_err(|_| "Document generation exhausted; restart the sidecar.")?;
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
    pub(crate) fn report(&self, id: &Identity, mut document: Document) -> Result<(), String> {
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
        if document.reference.source == crate::metadata::PlanSource::TerminalFallback {
            if let Some(old) = current.iter().find(|old| {
                old.kind == document.kind
                    && old.reference.worktree_root == document.reference.worktree_root
                    && old.reference.relative_path == document.reference.relative_path
                    && matches!(
                        old.reference.source,
                        crate::metadata::PlanSource::AgentReported
                            | crate::metadata::PlanSource::HookReported
                    )
            }) {
                document.reference.source = old.reference.source;
                document.observed_at = old.observed_at.clone();
            }
        }
        current.retain(|d| d.kind != document.kind);
        current.push(document);
        self.save(id, current)
    }
    pub(crate) fn reconcile(&self, scan: &Scan) -> Result<Vec<Document>, String> {
        let lock = self.lock(&scan.identity);
        let _guard = lock.lock().map_err(|_| "Branch selection unavailable.")?;
        let mut current = self.load(&scan.identity)?;
        if scan.generation.is_some_and(|generation| {
            generation != self.generation.load(std::sync::atomic::Ordering::Acquire)
        }) {
            return Ok(current);
        }
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
    pub(crate) fn use_latest(
        &self,
        id: &Identity,
        kind: DocumentKind,
        selection_id: Option<&str>,
    ) -> Result<(), String> {
        self.scans
            .lock()
            .map_err(|_| "Document cache unavailable.")?
            .clear();
        let lock = self.lock(id);
        let _guard = lock.lock().map_err(|_| "Branch selection unavailable.")?;
        let mut current = self.load(id)?;
        if let Some(expected) = selection_id {
            if !current
                .iter()
                .any(|doc| doc.kind == kind && doc.current_selection_id(Some(id)) == expected)
            {
                return Err("Document selection changed; refresh.".into());
            }
        }
        current.retain(|d| d.kind != kind);
        self.save(id, current)
    }

    pub(crate) fn with_current_document(
        &self,
        id: &Identity,
        expected: &Document,
        send: &mut dyn FnMut() -> Result<(), ()>,
    ) -> Result<(), ()> {
        let lock = self.lock(id);
        let _guard = lock.lock().map_err(|_| ())?;
        let documents = self.load(id).map_err(|_| ())?;
        let current = documents
            .iter()
            .find(|d| d.kind == expected.kind)
            .ok_or(())?;
        if current.reference.worktree_root != expected.reference.worktree_root
            || current.reference.relative_path != expected.reference.relative_path
        {
            return Err(());
        }
        crate::branch_documents::validate_document(id, current).map_err(|_| ())?;
        let content = crate::branch_documents::read_document(
            Path::new(current.reference.worktree_root.as_deref().ok_or(())?),
            &current.reference.relative_path,
        )
        .map_err(|_| ())?;
        if crate::branch_documents::content_hash(&content) != expected.content_hash {
            return Err(());
        }
        send()
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
            content_hash: String::new(),
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
                generation: None,
                identity: id.clone(),
                documents: vec![doc("docs/superpowers/plans/latest.md", false)],
                notice: None,
            })
            .unwrap();
        assert!(result[0].pinned);
        assert!(result[0].reference.relative_path.ends_with("pinned.md"));
        other_session
            .use_latest(&id, DocumentKind::Implementation, None)
            .unwrap();
        let result = store
            .reconcile(&Scan {
                generation: None,
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
                generation: None,
                identity: id,
                documents: vec![],
                notice: Some("Limit".into()),
            })
            .unwrap();
        assert!(result[0].reference.relative_path.ends_with("known.md"));
    }
}
