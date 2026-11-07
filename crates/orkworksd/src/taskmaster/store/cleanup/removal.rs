//! Owned deletion data; application freshness and protection admission remain separate.
mod journal;
use super::*;

pub(crate) struct PreparedRemoval {
    inventory: CleanupInventory,
    entries: Vec<RollupTransactionEntry>,
    uncommitted: Vec<u8>,
    committed: Vec<u8>,
}
/// Borrowed observations; copying these values does not copy the owner's authority.
pub(crate) struct RemovalTarget<'a> {
    pub(crate) id: &'a str,
    pub(crate) filename: &'a str,
    pub(crate) backup: &'a str,
    pub(crate) bytes: &'a [u8],
    pub(crate) identity: (u64, u64),
}
impl CleanupInventory {
    // Pure preparation from the bounded inventory; no path lookup or revalidation.
    pub(crate) fn prepare_session_removal(
        self,
        session_id: &str,
    ) -> Result<PreparedRemoval, StoreError> {
        let seeds = self
            .records
            .iter()
            .filter(|record| references_session(record, session_id))
            .map(|record| record.id.clone())
            .collect();
        let ids = deletion_graph_ids(&self.records, seeds);
        let mut entries = Vec::new();
        for record in &self.records {
            if !ids.contains(&record.id) {
                continue;
            }
            let leaf = self.leaves.get(&record.id).ok_or_else(|| {
                StoreError::Recovery("removal inventory lacks original leaf proof".into())
            })?;
            entries.push(RollupTransactionEntry {
                id: record.id.clone(),
                target: recommendation_filename(&record.id),
                staged: None,
                backup: Some(transaction_backup_path(&record.id)),
                old_sha256: Some(hash_bytes(&leaf.bytes)),
                old_exists: true,
                new_sha256: None,
                new_exists: false,
            });
        }
        entries.sort_by(|a, b| a.id.cmp(&b.id));
        validate_manifest_entries(&entries)?;
        let mut manifest = RollupTransactionManifest {
            version: ROLLUP_TRANSACTION_VERSION,
            committed: false,
            entries,
        };
        let uncommitted = bounded_manifest(&manifest)?;
        manifest.committed = true;
        let committed = bounded_manifest(&manifest)?;
        Ok(PreparedRemoval {
            inventory: self,
            entries: manifest.entries,
            uncommitted,
            committed,
        })
    }
}
fn bounded_manifest(manifest: &RollupTransactionManifest) -> Result<Vec<u8>, StoreError> {
    let bytes = serde_json::to_vec_pretty(manifest).map_err(StoreError::Json)?;
    if bytes.len() > 1024 * 1024 {
        return Err(StoreError::Io(io::Error::new(
            io::ErrorKind::InvalidData,
            "removal exceeds manifest byte bound",
        )));
    }
    Ok(bytes)
}
impl PreparedRemoval {
    pub(crate) fn manifest_bytes(&self, committed: bool) -> &[u8] {
        if committed {
            &self.committed
        } else {
            &self.uncommitted
        }
    }
    pub(crate) fn targets(&self) -> impl Iterator<Item = RemovalTarget<'_>> {
        self.entries.iter().map(|entry| {
            // All entries are deletion-only and validated before construction;
            // the inventory and entries are private and immutable afterwards.
            let leaf = self
                .inventory
                .leaves
                .get(&entry.id)
                .expect("prepared removal retains each original leaf");
            RemovalTarget {
                id: &entry.id,
                filename: &entry.target,
                backup: entry
                    .backup
                    .as_deref()
                    .expect("prepared deletion has a backup"),
                bytes: &leaf.bytes,
                identity: leaf.identity,
            }
        })
    }
}
