//! Explicit format routing; no SQL identity is reconstructed from journal bytes.
use super::*;
use crate::session_store::SqlDeletionIdentity;

#[derive(Clone, Copy)]
pub(super) enum Kind<'a> {
    Legacy,
    Sql(&'a SqlDeletionIdentity),
}
impl Kind<'_> {
    pub(super) fn name(self) -> String {
        match self {
            Self::Legacy => format!("tx-{}", uuid::Uuid::new_v4()),
            Self::Sql(identity) => identity.journal_name(),
        }
    }
    pub(super) fn check_name(self, text: &str) -> io::Result<()> {
        match self {
            Self::Sql(identity) if text == identity.journal_name() => Ok(()),
            Self::Sql(_) => Err(invalid("pending journal belongs to another SQL deletion")),
            Self::Legacy => {
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
                Ok(())
            }
        }
    }
    pub(super) fn parse(self, bytes: &[u8], name: &str) -> io::Result<RollupTransactionManifest> {
        match self {
            Self::Sql(identity) => {
                let decoded = sql_codec::decode(bytes, identity, name)
                    .map_err(|e| invalid(&e.to_string()))?;
                Ok(RollupTransactionManifest {
                    version: 3,
                    committed: decoded.committed,
                    entries: decoded.entries,
                })
            }
            Self::Legacy => {
                let m: RollupTransactionManifest =
                    crate::harness::definition::parse_strict_json(bytes, LIMIT)
                        .map_err(|_| invalid("invalid deletion manifest"))?;
                if m.version != 2
                    || m.entries.len() > 1024
                    || m.entries.iter().any(|e| {
                        !e.old_exists
                            || e.new_exists
                            || e.staged.is_some()
                            || e.new_sha256.is_some()
                    })
                {
                    return Err(invalid("unsupported deletion manifest"));
                }
                validate_manifest_entries(&m.entries).map_err(|e| invalid(&e.to_string()))?;
                Ok(m)
            }
        }
    }
    pub(super) fn encode(self, manifest: &RollupTransactionManifest) -> io::Result<Vec<u8>> {
        match self {
            Self::Legacy => bounded_manifest(manifest),
            Self::Sql(identity) => {
                sql_codec::encode(identity, &manifest.entries, manifest.committed)
            }
        }
        .map_err(|e| invalid(&e.to_string()))
    }
    pub(super) fn selection(
        self,
        records: &[Recommendation],
        entries: &BTreeMap<&str, &RollupTransactionEntry>,
        require_complete: bool,
    ) -> io::Result<()> {
        if let Self::Sql(identity) = self {
            let seeds = records
                .iter()
                .filter(|r| references_session(r, identity.session_id()))
                .map(|r| r.id.clone())
                .collect();
            let expected = deletion_graph_ids(records, seeds);
            let selected: BTreeSet<_> = entries.values().map(|e| e.id.clone()).collect();
            // Committed recovery may observe extra independent references from
            // before producer fencing; the retained fresh pass removes them.
            if !selected.is_subset(&expected) || (require_complete && expected != selected) {
                return Err(invalid(
                    "SQL journal targets do not match the session graph",
                ));
            }
        }
        Ok(())
    }
}
