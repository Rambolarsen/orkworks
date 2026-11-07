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
    if let Some(dir) = &binding.records {
        if entry(dir, JOURNAL)?.is_some() {
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
    let parent = mkdir(dir, JOURNAL)?;
    let name = basename(&format!("tx-{}", uuid::Uuid::new_v4()))?;
    let directory = mkdir(&parent, &name)?;
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
    recover_expected(binding, Some(&removal.inventory.leaves))
}

pub(super) fn recover(binding: &Binding) -> io::Result<()> {
    recover_expected(binding, None)
}
fn recover_expected(
    binding: &Binding,
    expected: Option<&BTreeMap<String, CapturedLeaf>>,
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
    let id = text
        .strip_prefix("tx-")
        .ok_or_else(|| invalid("invalid transaction name"))?;
    if uuid::Uuid::parse_str(id)
        .map(|u| u.to_string())
        .ok()
        .as_deref()
        != Some(id)
    {
        return Err(invalid("noncanonical transaction identity"));
    }
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
    let parsed: Option<RollupTransactionManifest> = manifest_leaf
        .as_ref()
        .map(|leaf| {
            crate::harness::definition::parse_strict_json(&leaf.bytes, LIMIT)
                .map_err(|_| invalid("invalid deletion manifest"))
        })
        .transpose()?;
    if let Some(m) = &parsed {
        if m.version != 2
            || m.entries.len() > 1024
            || m.entries.iter().any(|e| {
                !e.old_exists || e.new_exists || e.staged.is_some() || e.new_sha256.is_some()
            })
        {
            return Err(invalid("unsupported deletion manifest"));
        }
        validate_manifest_entries(&m.entries).map_err(|e| invalid(&e.to_string()))?;
    }
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
                backups.insert(e.id.clone(), (leaf, Some(record)));
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
                backups.insert(id.clone(), (leaf, None));
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
        let bytes = bounded_manifest(&next).map_err(|e| invalid(&e.to_string()))?;
        if !bytes.starts_with(&temp.bytes) {
            return Err(invalid("temporary manifest differs from expected write"));
        }
    }
    // Validate the entire graph and every target before the first effect.
    if committed {
        let mut old: BTreeMap<String, Recommendation> =
            live.iter().map(|r| (r.id.clone(), r.clone())).collect();
        if !all_absent {
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
            graph(&old.into_values().collect::<Vec<_>>())?;
        }
        let remaining: Vec<_> = live
            .iter()
            .filter(|r| !selected.values().any(|e| e.id == r.id))
            .cloned()
            .collect();
        graph(&remaining)?;
    } else {
        graph(&live)?;
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
            for (leaf, _) in backups.values() {
                leaf.verify(directory)?;
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
    // Known leaf removals only. Manifest remains until all backup cleanup is durable.
    if let Some(directory) = &tx.backups {
        for (leaf, _) in backups.values() {
            tx.current(binding)?;
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
    if let Some(temp) = temp {
        tx.current(binding)?;
        temp.verify(&tx.directory)?;
        remove(&tx.directory, TEMP, &temp.file, false)?;
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
