//! Pure SQL-bound v3 deletion encoding. No file effects or recovery authority.
#![allow(dead_code)] // Captured application routing remains a separately qualified slice.
use super::*;
use crate::session_store::SqlDeletionIdentity;
use serde::{Deserialize, Serialize};

const MANIFEST_BYTES: usize = 1024 * 1024;
const ENTRY_LIMIT: usize = 1024;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireManifest {
    version: u32,
    committed: bool,
    // Compared as a complete JSON value against the SQL-minted identity. This
    // rejects missing/unknown fields and wrong types without deserializing a proof.
    sql_deletion: serde_json::Value,
    entries: Vec<WireEntry>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WireEntry {
    id: String,
    target: String,
    #[serde(deserialize_with = "Option::deserialize")]
    staged: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    backup: Option<String>,
    #[serde(deserialize_with = "Option::deserialize")]
    old_sha256: Option<String>,
    old_exists: bool,
    #[serde(deserialize_with = "Option::deserialize")]
    new_sha256: Option<String>,
    new_exists: bool,
}
impl From<WireEntry> for RollupTransactionEntry {
    fn from(entry: WireEntry) -> Self {
        Self {
            id: entry.id,
            target: entry.target,
            staged: entry.staged,
            backup: entry.backup,
            old_sha256: entry.old_sha256,
            old_exists: entry.old_exists,
            new_sha256: entry.new_sha256,
            new_exists: entry.new_exists,
        }
    }
}

pub(super) struct DecodedRemoval {
    pub(super) committed: bool,
    pub(super) entries: Vec<RollupTransactionEntry>,
}
fn invalid_manifest() -> StoreError {
    StoreError::Recovery("invalid SQL-bound deletion manifest".into())
}
fn validate(entries: &[RollupTransactionEntry]) -> Result<(), StoreError> {
    if entries.len() > ENTRY_LIMIT {
        return Err(invalid_manifest());
    }
    validate_manifest_entries(entries)?;
    for entry in entries {
        if !entry.old_exists
            || entry.new_exists
            || entry.staged.is_some()
            || entry.new_sha256.is_some()
            || !entry.old_sha256.as_ref().is_some_and(|hash| {
                hash.len() == 64
                    && hash
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            })
        {
            return Err(invalid_manifest());
        }
    }
    Ok(())
}
pub(super) fn encode(
    identity: &SqlDeletionIdentity,
    entries: &[RollupTransactionEntry],
    committed: bool,
) -> Result<Vec<u8>, StoreError> {
    validate(entries)?;
    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Manifest<'a> {
        version: u32,
        committed: bool,
        sql_deletion: &'a SqlDeletionIdentity,
        entries: &'a [RollupTransactionEntry],
    }
    let bytes = serde_json::to_vec_pretty(&Manifest {
        version: 3,
        committed,
        sql_deletion: identity,
        entries,
    })
    .map_err(StoreError::Json)?;
    if bytes.len() > MANIFEST_BYTES {
        return Err(invalid_manifest());
    }
    Ok(bytes)
}
pub(super) fn decode(
    bytes: &[u8],
    expected: &SqlDeletionIdentity,
    basename: &str,
) -> Result<DecodedRemoval, StoreError> {
    if basename != expected.journal_name() {
        return Err(invalid_manifest());
    }
    let wire: WireManifest = crate::harness::definition::parse_strict_json(bytes, MANIFEST_BYTES)
        .map_err(|_| invalid_manifest())?;
    if wire.version != 3
        || wire.sql_deletion != serde_json::to_value(expected).map_err(StoreError::Json)?
    {
        return Err(invalid_manifest());
    }
    let entries: Vec<_> = wire.entries.into_iter().map(Into::into).collect();
    validate(&entries)?;
    Ok(DecodedRemoval {
        committed: wire.committed,
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_store::{SessionStore, SqlDeletionIdentity};
    use serde_json::{json, Value};

    async fn selected() -> SqlDeletionIdentity {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut meta = crate::test_support::test_session_metadata(
            "pending",
            "Before",
            "/workspace",
            "ended",
            "2026-01-01T00:00:00Z",
            "2026-01-02T00:00:00Z",
        );
        meta.lifecycle = "dead".into();
        meta.lifecycle_phase = "ended".into();
        store
            .create(serde_json::to_value(meta).unwrap())
            .await
            .unwrap();
        store
            .begin_delete(
                "pending".into(),
                "2026-01-01T00:00:00Z".into(),
                "1".into(),
                "0".into(),
            )
            .await
            .unwrap();
        let identity = store
            .deletion_record("pending".into(), "2026-01-01T00:00:00Z".into())
            .await
            .unwrap()
            .unwrap()
            .identity;
        store.close().await.unwrap();
        identity
    }
    fn entry(id: &str) -> RollupTransactionEntry {
        RollupTransactionEntry {
            id: id.into(),
            target: recommendation_filename(id),
            staged: None,
            backup: Some(transaction_backup_path(id)),
            old_sha256: Some("a".repeat(64)),
            old_exists: true,
            new_sha256: None,
            new_exists: false,
        }
    }
    fn value(identity: &SqlDeletionIdentity) -> Value {
        serde_json::from_slice(&encode(identity, &[entry("rec-1")], false).unwrap()).unwrap()
    }
    fn denied(identity: &SqlDeletionIdentity, value: &Value) {
        assert!(
            decode(
                &serde_json::to_vec(value).unwrap(),
                identity,
                &identity.journal_name()
            )
            .is_err(),
            "{value}"
        );
    }

    #[tokio::test]
    async fn sql_codec_roundtrips_only_commit_state_and_preserves_entry_bytes() {
        let identity = selected().await;
        let mut values = Vec::new();
        for committed in [false, true] {
            let bytes = encode(&identity, &[entry("rec-1")], committed).unwrap();
            let decoded = decode(&bytes, &identity, &identity.journal_name()).unwrap();
            assert_eq!(decoded.committed, committed);
            assert_eq!(decoded.entries[0].target, "rec-1.json");
            assert_eq!(
                decoded.entries[0].backup.as_deref(),
                Some("backups/rec-1.json")
            );
            assert_eq!(
                decoded.entries[0].old_sha256.as_deref(),
                Some("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
            );
            let mut wire: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(wire["version"], 3);
            assert_eq!(
                wire["sqlDeletion"],
                serde_json::to_value(&identity).unwrap()
            );
            wire["committed"] = json!(false);
            values.push(wire);
        }
        assert_eq!(values[0], values[1]);
    }

    #[tokio::test]
    async fn sql_codec_denies_foreign_identity_and_noncanonical_basename() {
        let identity = selected().await;
        let bytes = encode(&identity, &[entry("rec-1")], true).unwrap();
        let foreign = selected().await;
        assert!(decode(&bytes, &foreign, &foreign.journal_name()).is_err());
        for name in [
            "tx-12345678-1234-4234-8234-123456789abc".to_string(),
            identity.journal_name().to_uppercase(),
            format!("{}/", identity.journal_name()),
            format!("{}0", identity.journal_name()),
        ] {
            assert!(decode(&bytes, &identity, &name).is_err());
        }
        for key in ["databaseEpoch", "workspaceSha256", "sessionId", "createdAt"] {
            let mut v = value(&identity);
            v["sqlDeletion"][key] = json!("foreign");
            denied(&identity, &v);
        }
    }

    #[tokio::test]
    async fn sql_codec_refuses_unknown_missing_duplicate_and_wrongly_typed_fields() {
        let identity = selected().await;
        for location in [0, 1, 2] {
            let mut v = value(&identity);
            let object = match location {
                0 => v.as_object_mut().unwrap(),
                1 => v["sqlDeletion"].as_object_mut().unwrap(),
                _ => v["entries"][0].as_object_mut().unwrap(),
            };
            object.insert("future".into(), json!(true));
            denied(&identity, &v);
        }
        for key in [
            "staged",
            "backup",
            "oldSha256",
            "newSha256",
            "oldExists",
            "newExists",
            "id",
            "target",
        ] {
            let mut v = value(&identity);
            v["entries"][0].as_object_mut().unwrap().remove(key);
            denied(&identity, &v);
        }
        for (key, bad) in [
            ("version", json!(2)),
            ("version", json!(4)),
            ("committed", json!("true")),
            ("sqlDeletion", json!(null)),
            ("entries", json!({})),
        ] {
            let mut v = value(&identity);
            v[key] = bad;
            denied(&identity, &v);
        }
        let original = serde_json::to_string(&value(&identity)).unwrap();
        for duplicate in [
            original.replacen("{", "{\"version\":3,", 1),
            original.replace("\"sessionId\":", "\"sessionId\":\"pending\",\"sessionId\":"),
            original.replace("\"oldExists\":", "\"oldExists\":true,\"oldExists\":"),
        ] {
            assert!(decode(duplicate.as_bytes(), &identity, &identity.journal_name()).is_err());
        }
        assert!(decode(
            format!("{original} {{}}").as_bytes(),
            &identity,
            &identity.journal_name()
        )
        .is_err());
    }

    #[tokio::test]
    async fn sql_codec_rejects_replacements_escaping_paths_duplicate_ids_and_bad_hashes() {
        let identity = selected().await;
        for (key, bad) in [
            ("oldExists", json!(false)),
            ("newExists", json!(true)),
            ("staged", json!("staged/rec-1.json")),
            ("newSha256", json!("b".repeat(64))),
            ("target", json!("../rec-1.json")),
            ("backup", json!("../rec-1.json")),
            ("oldSha256", json!("A".repeat(64))),
            ("oldSha256", json!("a".repeat(63))),
            ("oldSha256", json!("g".repeat(64))),
        ] {
            let mut v = value(&identity);
            v["entries"][0][key] = bad;
            denied(&identity, &v);
        }
        let duplicate = entry("rec-1");
        assert!(encode(&identity, &[entry("rec-1"), duplicate], false).is_err());
        let mut replacement = entry("rec-1");
        replacement.new_exists = true;
        replacement.new_sha256 = Some("b".repeat(64));
        replacement.staged = Some("staged/rec-1.json".into());
        assert!(encode(&identity, &[replacement], false).is_err());
    }

    #[tokio::test]
    async fn sql_codec_enforces_exact_manifest_byte_and_entry_caps() {
        let identity = selected().await;
        let entries: Vec<_> = (0..1024).map(|i| entry(&format!("r{i}"))).collect();
        let bytes = encode(&identity, &entries, true).unwrap();
        assert_eq!(
            decode(&bytes, &identity, &identity.journal_name())
                .unwrap()
                .entries
                .len(),
            1024
        );
        let mut over = entries;
        over.push(entry("extra"));
        assert!(encode(&identity, &over, false).is_err());
        let mut v = value(&identity);
        v["entries"] = serde_json::to_value(&over).unwrap();
        denied(&identity, &v);
        // Paths alone fit the existing ID/path rules, but their serialized
        // envelope exceeds the manifest byte cap even with only one entry.
        assert!(encode(&identity, &[entry(&"x".repeat(400_000))], false).is_err());
        let mut exact = encode(&identity, &[], false).unwrap();
        exact.resize(1024 * 1024, b' ');
        assert!(decode(&exact, &identity, &identity.journal_name()).is_ok());
        exact.push(b' ');
        assert!(decode(&exact, &identity, &identity.journal_name()).is_err());
    }
}
