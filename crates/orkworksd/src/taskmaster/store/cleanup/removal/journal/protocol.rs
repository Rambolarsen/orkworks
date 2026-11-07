use super::*;
use crate::taskmaster::store::cleanup::unix::{self, entry, identity, invalid, names, open};
use std::{
    ffi::{CStr, CString},
    io::{Read, Write},
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::fs::{MetadataExt, OpenOptionsExt},
    },
};
mod files;
use files::*;
mod kind;
use kind::Kind;
const JOURNAL: &CStr = c".rollup-transactions";
const MANIFEST: &CStr = c"manifest.json";
const TEMP: &CStr = c"manifest.json.tmp";
const LIMIT: usize = 1024 * 1024;
const TOTAL: usize = 32 * LIMIT;

fn graph(records: &[Recommendation]) -> io::Result<()> {
    validate_graph_records(records).map_err(|e| invalid(&e.to_string()))
}
fn records(binding: &Binding) -> io::Result<(Vec<Recommendation>, BTreeMap<String, CapturedLeaf>)> {
    unix::read_records(binding, true).map_err(|e| invalid(&e.to_string()))
}
fn current(binding: &Binding) -> io::Result<()> {
    unix::current(binding)?;
    let root_path = binding
        .store
        .dir
        .parent()
        .ok_or_else(|| invalid("missing metadata root"))?;
    // A renamed ancestor bridged by a symlink preserves inode identity but would
    // allow a new canonical path to acquire a different path-keyed operation owner.
    if fs::canonicalize(root_path)?.as_path() != root_path {
        return Err(invalid("canonical metadata root changed after binding"));
    }
    let root = fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(root_path)?;
    if identity(&root)? != identity(&binding.root)? {
        return Err(invalid("metadata root changed after binding"));
    }
    Ok(())
}
fn sync_records(binding: &Binding) -> io::Result<()> {
    current(binding)?;
    if let Some(dir) = &binding.records {
        sync(dir)?;
    }
    sync(&binding.root)
}
struct Transaction {
    parent: fs::File,
    name: CString,
    directory: fs::File,
    backups: Option<fs::File>,
    staged: Option<fs::File>,
}
impl Transaction {
    fn current(&self, binding: &Binding) -> io::Result<()> {
        current(binding)?;
        let dir = binding
            .records
            .as_ref()
            .ok_or_else(|| invalid("missing records directory"))?;
        check(dir, JOURNAL, &self.parent)?;
        check(&self.parent, &self.name, &self.directory)?;
        for (name, child) in [(c"backups", &self.backups), (c"staged", &self.staged)] {
            match child {
                Some(file) => check(&self.directory, name, file)?,
                None if entry(&self.directory, name)?.is_some() => {
                    return Err(invalid("journal directory appeared"))
                }
                None => (),
            }
        }
        Ok(())
    }
    fn sync(&self, binding: &Binding) -> io::Result<()> {
        self.current(binding)?;
        sync(&self.directory)?;
        sync(&self.parent)?;
        sync_records(binding)
    }
}
fn optional_directory(parent: &fs::File, name: &CStr) -> io::Result<Option<fs::File>> {
    match open(parent, name, true) {
        Ok(file) => {
            check(parent, name, &file)?;
            Ok(Some(file))
        }
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}
fn manifest(
    tx: &Transaction,
    bytes: &[u8],
    previous: Option<&Leaf>,
    binding: &Binding,
) -> io::Result<Leaf> {
    tx.current(binding)?;
    let temp = write(&tx.directory, TEMP, bytes)?;
    tx.current(binding)?;
    temp.verify(&tx.directory)?;
    match previous {
        Some(leaf) => leaf.verify(&tx.directory)?,
        None if entry(&tx.directory, MANIFEST)?.is_some() => {
            return Err(invalid("manifest appeared"))
        }
        None => (),
    }
    // SAFETY: live pinned directory descriptor and fixed, single basenames.
    if unsafe {
        libc::renameat(
            tx.directory.as_raw_fd(),
            TEMP.as_ptr(),
            tx.directory.as_raw_fd(),
            MANIFEST.as_ptr(),
        )
    } < 0
    {
        return Err(io::Error::last_os_error());
    }
    checkpoint()?;
    tx.sync(binding)?;
    read(&tx.directory, MANIFEST)
}
pub(super) fn publish(removal: &PreparedRemoval) -> io::Result<()> {
    publish_kind(removal, Kind::Legacy)
}
pub(super) fn publish_sql(
    removal: &PreparedRemoval,
    identity: &crate::session_store::SqlDeletionIdentity,
) -> io::Result<()> {
    publish_kind(removal, Kind::Sql(identity))
}
fn publish_kind(removal: &PreparedRemoval, kind: Kind<'_>) -> io::Result<()> {
    let binding = &removal.inventory._binding.0;
    current(binding)?;
    let (current_records, leaves) = records(binding)?;
    graph(&current_records)?;
    if leaves.len() != removal.inventory.leaves.len()
        || leaves.iter().any(|(id, leaf)| {
            !removal
                .inventory
                .leaves
                .get(id)
                .is_some_and(|old| old.identity == leaf.identity && old.bytes == leaf.bytes)
        })
    {
        return Err(invalid(
            "recommendation inventory changed before publication",
        ));
    }
    let existing_parent = match &binding.records {
        Some(dir) => optional_directory(dir, JOURNAL)?,
        None => None,
    };
    if let Some(parent) = &existing_parent {
        if !names(parent)?.is_empty() {
            return Err(invalid("recommendation recovery required"));
        }
    }
    if removal.entries.is_empty() {
        return sync_records(binding);
    }
    let dir = binding
        .records
        .as_ref()
        .ok_or_else(|| invalid("missing records directory"))?;
    current(binding)?;
    let parent = match existing_parent {
        Some(parent) => parent,
        None => mkdir(dir, JOURNAL)?,
    };
    let name = basename(&kind.name())?;
    let directory = mkdir(&parent, &name)?;
    if matches!(kind, Kind::Sql(_)) {
        // Establish the bound transaction name durably before any backup write.
        sync(&directory)?;
        sync(&parent)?;
        sync_records(binding)?;
    }
    let backups = Some(mkdir(&directory, c"backups")?);
    let staged = Some(mkdir(&directory, c"staged")?);
    let tx = Transaction {
        parent,
        name,
        directory,
        backups,
        staged,
    };
    for target in removal.targets() {
        tx.current(binding)?;
        write(
            tx.backups.as_ref().expect("new backups directory"),
            &basename(target.filename)?,
            target.bytes,
        )?;
    }
    sync(tx.backups.as_ref().expect("new backups directory"))?;
    sync(tx.staged.as_ref().expect("new staged directory"))?;
    tx.sync(binding)?;
    let uncommitted = manifest(&tx, removal.manifest_bytes(false), None, binding)?;
    manifest(
        &tx,
        removal.manifest_bytes(true),
        Some(&uncommitted),
        binding,
    )?;
    // Recovery performs complete content/graph validation before its first unlink.
    // The same held operation permit spans publication, recovery and the receipt.
    recover_expected(binding, Some(&removal.inventory.leaves), kind)
}

pub(super) fn recover(binding: &Binding) -> io::Result<()> {
    recover_expected(binding, None, Kind::Legacy)
}
pub(super) fn recover_sql(
    binding: &Binding,
    identity: &crate::session_store::SqlDeletionIdentity,
) -> io::Result<()> {
    recover_expected(binding, None, Kind::Sql(identity))
}
fn recover_expected(
    binding: &Binding,
    expected: Option<&BTreeMap<String, CapturedLeaf>>,
    kind: Kind<'_>,
) -> io::Result<()> {
    current(binding)?;
    let (live, leaves) = records(binding)?;
    if let Some(expected) = expected {
        if leaves.len() != expected.len()
            || leaves.iter().any(|(id, leaf)| {
                !expected
                    .get(id)
                    .is_some_and(|old| old.identity == leaf.identity && old.bytes == leaf.bytes)
            })
        {
            return Err(invalid(
                "captured inventory changed during journal publication",
            ));
        }
    }
    let Some(dir) = &binding.records else {
        return sync_records(binding);
    };
    let Some(parent) = optional_directory(dir, JOURNAL)? else {
        graph(&live)?;
        return sync_records(binding);
    };
    let pending = names(&parent)?;
    if pending.is_empty() {
        graph(&live)?;
        current(binding)?;
        remove(dir, JOURNAL, &parent, true)?;
        return sync_records(binding);
    }
    if pending.len() != 1 {
        return Err(invalid("multiple pending recommendation journals"));
    }
    let name = pending.into_iter().next().expect("one transaction");
    let text = name
        .to_str()
        .map_err(|_| invalid("invalid transaction name"))?;
    kind.check_name(text)?;
    let directory = open(&parent, &name, true)?;
    let mut tx = Transaction {
        backups: optional_directory(&directory, c"backups")?,
        staged: optional_directory(&directory, c"staged")?,
        parent,
        name,
        directory,
    };
    tx.current(binding)?;
    let entries = names(&tx.directory)?;
    if entries
        .iter()
        .any(|n| ![MANIFEST, TEMP, c"backups", c"staged"].contains(&n.as_c_str()))
    {
        return Err(invalid("unknown journal entry"));
    }
    if let Some(staged) = &tx.staged {
        if !names(staged)?.is_empty() {
            return Err(invalid("deletion journal contains staged replacements"));
        }
    }
    let manifest_leaf = optional_read(&tx.directory, MANIFEST)?;
    let temp = optional_read(&tx.directory, TEMP)?;
    let text = tx
        .name
        .to_str()
        .map_err(|_| invalid("invalid transaction name"))?;
    let parsed: Option<RollupTransactionManifest> = manifest_leaf
        .as_ref()
        .map(|leaf| kind.parse(&leaf.bytes, text))
        .transpose()?;
    let committed = parsed.as_ref().is_some_and(|m| m.committed);
    let selected: BTreeMap<&str, &RollupTransactionEntry> = parsed
        .as_ref()
        .map(|m| m.entries.iter().map(|e| (e.target.as_str(), e)).collect())
        .unwrap_or_default();
    let all_absent = committed && selected.values().all(|e| !leaves.contains_key(&e.id));
    let mut backups = BTreeMap::new();
    let mut total = 0usize;
    if let Some(directory) = &tx.backups {
        for name in names(directory)? {
            let leaf = read(directory, &name)?;
            total = total
                .checked_add(leaf.bytes.len())
                .filter(|n| *n <= TOTAL)
                .ok_or_else(|| invalid("backup aggregate bound exceeded"))?;
            let filename = name
                .to_str()
                .map_err(|_| invalid("invalid backup filename"))?;
            if committed {
                let e = selected
                    .get(filename)
                    .ok_or_else(|| invalid("unknown backup"))?;
                if e.old_sha256.as_deref() != Some(hash_bytes(&leaf.bytes).as_str()) {
                    return Err(invalid("backup hash mismatch"));
                }
                let record = parse_record(&leaf.bytes, &e.id)?;
                backups.insert(e.id.clone(), (capture_backup(leaf)?, Some(record)));
            } else {
                let matching = leaves
                    .iter()
                    .find(|(id, _)| recommendation_filename(id) == filename);
                let Some((id, original)) = matching else {
                    return Err(invalid("draft backup has no original"));
                };
                if parsed.is_some() && !selected.contains_key(filename) {
                    return Err(invalid("unknown uncommitted backup"));
                }
                if !original.bytes.starts_with(&leaf.bytes) {
                    return Err(invalid("draft backup differs from original"));
                }
                backups.insert(id.clone(), (capture_backup(leaf)?, None));
            }
        }
    }
    if let Some(temp) = &temp {
        if committed {
            return Err(invalid(
                "committed journal has an unexpected temporary manifest",
            ));
        }
        let next = if let Some(manifest) = &parsed {
            let mut next = manifest.clone();
            next.committed = true;
            next
        } else {
            // Before the first manifest rename, the complete backup set determines
            // the sole manifest this writer could have been staging.
            if backups.is_empty() {
                return Err(invalid("temporary manifest has no completed backup"));
            }
            let mut entries = Vec::new();
            for (id, (backup, _)) in &backups {
                if leaves[id].bytes != backup.bytes {
                    return Err(invalid("temporary manifest precedes completed backups"));
                }
                entries.push(RollupTransactionEntry {
                    id: id.clone(),
                    target: recommendation_filename(id),
                    staged: None,
                    backup: Some(transaction_backup_path(id)),
                    old_sha256: Some(hash_bytes(&backup.bytes)),
                    old_exists: true,
                    new_sha256: None,
                    new_exists: false,
                });
            }
            RollupTransactionManifest {
                version: 2,
                committed: false,
                entries,
            }
        };
        if parsed.is_none() {
            // A short first prefix cannot prove backup completeness. SQL's
            // selected job identifies the full expected graph before discard.
            let expected = next
                .entries
                .iter()
                .map(|e| (e.target.as_str(), e))
                .collect();
            kind.selection(&live, &expected, true)?;
        }
        let bytes = kind.encode(&next)?;
        if !bytes.starts_with(&temp.bytes) {
            return Err(invalid("temporary manifest differs from expected write"));
        }
    }
    // Validate the entire graph and every target before the first effect.
    if committed {
        let mut old: BTreeMap<String, Recommendation> =
            live.iter().map(|r| (r.id.clone(), r.clone())).collect();
        let complete_backups = selected.values().all(|e| backups.contains_key(&e.id));
        // Complete SQL evidence must still be validated when the targets are
        // already gone. Missing backups are allowed only after all targets are
        // absent, so interrupted backup cleanup remains resumable.
        if !all_absent || (matches!(kind, Kind::Sql(_)) && complete_backups) {
            for e in selected.values() {
                let (_, record) = backups
                    .get(&e.id)
                    .ok_or_else(|| invalid("missing committed backup"))?;
                if let Some(leaf) = leaves.get(&e.id) {
                    if e.old_sha256.as_deref() != Some(hash_bytes(&leaf.bytes).as_str()) {
                        return Err(invalid("target changed since journal capture"));
                    }
                }
                old.insert(
                    e.id.clone(),
                    record.as_ref().expect("parsed committed backup").clone(),
                );
            }
            if old.len() > 1024
                || old
                    .values()
                    .map(|r| {
                        leaves
                            .get(&r.id)
                            .map(|l| l.bytes.len())
                            .unwrap_or_else(|| backups[&r.id].0.bytes.len())
                    })
                    .sum::<usize>()
                    > TOTAL
            {
                return Err(invalid("reconstructed graph exceeds bounds"));
            }
            let old = old.into_values().collect::<Vec<_>>();
            graph(&old)?;
            // An empty journal must prove there is no selected work. A
            // nonempty committed pass may precede additional independent refs.
            kind.selection(&old, &selected, selected.is_empty())?;
        }
        let remaining: Vec<_> = live
            .iter()
            .filter(|r| !selected.values().any(|e| e.id == r.id))
            .cloned()
            .collect();
        graph(&remaining)?;
    } else {
        graph(&live)?;
        if parsed.is_some() {
            kind.selection(&live, &selected, true)?;
        }
        for e in selected.values() {
            if !leaves
                .get(&e.id)
                .is_some_and(|l| e.old_sha256.as_deref() == Some(hash_bytes(&l.bytes).as_str()))
            {
                return Err(invalid("uncommitted target changed"));
            }
        }
    }
    if committed {
        // Repeat durability barrier even after a prior crash following manifest rename.
        if let Some(directory) = &tx.backups {
            for (id, (captured, _)) in &backups {
                let leaf = reopen_backup(directory, id, captured)?;
                sync(&leaf.file)?;
            }
            sync(directory)?;
        }
        tx.sync(binding)?;
        for e in selected.values() {
            if let Some(captured) = leaves.get(&e.id) {
                tx.current(binding)?;
                if let Some(manifest) = &manifest_leaf {
                    manifest.verify(&tx.directory)?;
                }
                let leaf = read(dir, &basename(&e.target)?)?;
                if identity(&leaf.file)? != captured.identity || leaf.bytes != captured.bytes {
                    return Err(invalid("target changed during recovery"));
                }
                leaf.verify(dir)?;
                remove(dir, &leaf.name, &leaf.file, false)?;
            }
        }
    }
    sync_records(binding)?;
    graph(&records(binding)?.0)?;
    // The first draft temp needs the complete backup set for validation. Its
    // removal must be durable before any of that evidence can be discarded.
    if let Some(temp) = temp {
        tx.current(binding)?;
        temp.verify(&tx.directory)?;
        remove(&tx.directory, TEMP, &temp.file, false)?;
        sync(&tx.directory)?;
    }
    // Known leaf removals only. Manifest remains until all backup cleanup is durable.
    if let Some(directory) = &tx.backups {
        for (id, (captured, _)) in &backups {
            tx.current(binding)?;
            let leaf = reopen_backup(directory, id, captured)?;
            leaf.verify(directory)?;
            remove(directory, &leaf.name, &leaf.file, false)?;
        }
        sync(directory)?;
    }
    for name in [c"backups", c"staged"] {
        tx.current(binding)?;
        let child = if name == c"backups" {
            tx.backups.take()
        } else {
            tx.staged.take()
        };
        if let Some(file) = child {
            remove(&tx.directory, name, &file, true)?;
            sync(&tx.directory)?;
        }
    }
    if let Some(manifest) = manifest_leaf {
        tx.current(binding)?;
        manifest.verify(&tx.directory)?;
        remove(&tx.directory, MANIFEST, &manifest.file, false)?;
    }
    tx.sync(binding)?;
    remove(&tx.parent, &tx.name, &tx.directory, true)?;
    sync(&tx.parent)?;
    current(binding)?;
    remove(dir, JOURNAL, &tx.parent, true)?;
    sync_records(binding)
}
fn parse_record(bytes: &[u8], id: &str) -> io::Result<Recommendation> {
    let r: Recommendation = crate::harness::definition::parse_strict_json(bytes, LIMIT)
        .map_err(|_| invalid("invalid backup JSON"))?;
    validate_recommendation(&r).map_err(|e| invalid(&e.to_string()))?;
    if r.id != id {
        return Err(invalid("backup identity mismatch"));
    }
    Ok(r)
}

// Inventory bytes and observed identities are bounded; open descriptors are
// constant in the number of records. Every later effect reopens one leaf and
// checks both its current inode and bytes against the observation.
fn capture_backup(leaf: Leaf) -> io::Result<CapturedLeaf> {
    Ok(CapturedLeaf {
        identity: identity(&leaf.file)?,
        bytes: leaf.bytes,
    })
}
fn reopen_backup(directory: &fs::File, id: &str, captured: &CapturedLeaf) -> io::Result<Leaf> {
    let leaf = read(directory, &basename(&recommendation_filename(id))?)?;
    if identity(&leaf.file)? != captured.identity || leaf.bytes != captured.bytes {
        return Err(invalid("backup changed during recovery"));
    }
    Ok(leaf)
}
