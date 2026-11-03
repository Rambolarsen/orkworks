//! Open-time revalidation (never repairs): epoch/workspace consistency,
//! commit-chain continuity, retained proofs, every retained row's shape and
//! digest, and the retained-byte quota recompute. Any missing/ambiguous/
//! corrupt state fences writes and reports unavailable; nothing is
//! truncated, migrated, moved or recomputed from "latest" rows.

use super::rows;
use super::{canonical, ensure, HierarchyStorageError, Result, JOURNAL_COMMAND};
use serde_json::{json, Value};

/// Closed command kinds this format-4 catalog knows. Producer kinds beyond
/// the child-source slice appear only as DDL and never in journalable rows.
const CLOSED_COMMAND_KINDS: [&str; 10] = [
    "initialize-store",
    JOURNAL_COMMAND,
    "child-source-plan-approve",
    "child-source-propose",
    "child-source-allocate",
    "child-source-attach",
    "child-source-reserve",
    "evaluation-artifact-read-work",
    "evaluation-write",
    "report-publication",
];

/// The logical row tables this verification walks (excluding the header and
/// the proof table itself).
const ROW_TABLES: [&str; 15] = [
    "artifact_read_debits",
    "candidate_work",
    "child_allocations",
    "child_attachments",
    "child_plan_approvals",
    "child_reservations",
    "dependencies",
    "evaluation_artifact_streams",
    "heads",
    "objects",
    "payloads",
    "pending_material",
    "quota_usage",
    "receipt_slots",
    "summary_state",
];

/// Full chain + row verification against a live (read-only) connection.
/// Errors fence the store with their static reason; nothing is repaired.
pub(super) fn verify_chain(
    database: &rusqlite::Connection,
    workspace: &str,
    epoch: &str,
) -> Result<()> {
    // 1. Retained proofs, ordered by sequence with no gaps.
    let mut statement = database
        .prepare("SELECT sequence, body, change_digest FROM commit_proof ORDER BY sequence")
        .map_err(|_| HierarchyStorageError("physical database"))?;
    let mut proofs: Vec<(i64, Vec<u8>, String)> = Vec::new();
    {
        let mut query = statement
            .query([])
            .map_err(|_| HierarchyStorageError("physical database"))?;
        while let Some(row) = query
            .next()
            .map_err(|_| HierarchyStorageError("physical database"))?
        {
            proofs.push((
                row.get(0)
                    .map_err(|_| HierarchyStorageError("physical database"))?,
                row.get(1)
                    .map_err(|_| HierarchyStorageError("physical database"))?,
                row.get(2)
                    .map_err(|_| HierarchyStorageError("physical database"))?,
            ));
        }
    }
    drop(statement);
    ensure(!proofs.is_empty(), "chain")?;
    for (index, (sequence, _, _)) in proofs.iter().enumerate() {
        ensure(
            u64::try_from(*sequence).map_err(|_| HierarchyStorageError("exhausted"))?
                == index as u64 + 1,
            "chain",
        )?;
    }
    // Manifest envelopes: canonical bytes; exact closure; per-proof digest
    // equations; closed command kinds; predecessor linkage; changes sorted
    // uniquely by UTF-8 table name then canonical key bytes.
    let mut manifests: Vec<(u64, Value, String)> = Vec::with_capacity(proofs.len());
    for (index, (sequence, body, change_digest)) in proofs.iter().enumerate() {
        let manifest: Value =
            serde_json::from_slice(body).map_err(|_| HierarchyStorageError("chain"))?;
        ensure(canonical(&manifest)? == *body, "canonical wire")?;
        exact_manifest(&manifest, *sequence, workspace, epoch)?;
        let expected = super::domain_digest(super::CHANGE_DOMAIN, body);
        ensure(expected == *change_digest, "chain")?;
        if index > 0 {
            // Predecessor == the canonical prior CommitRef.
            let (previous_sequence, previous, _previous_digest) = &manifests[index - 1];
            let prior_commit = json!({
                "changeDigest": super::domain_digest(super::CHANGE_DOMAIN, super::canonical(previous)?.as_slice()),
                "commitSequence": previous_sequence.to_string(),
                "storageEpoch": epoch,
            });
            ensure(
                canonical(&manifest["predecessor"])? == canonical(&prior_commit)?,
                "chain",
            )?;
        }
        let entries = manifest["changes"]
            .as_array()
            .ok_or(HierarchyStorageError("chain"))?;
        let mut ordering: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
        for entry in entries {
            exact_change_entry(entry)?;
            ordering.push((
                entry["table"]
                    .as_str()
                    .ok_or(HierarchyStorageError("chain"))?
                    .as_bytes()
                    .to_vec(),
                canonical(&entry["key"])?,
            ));
        }
        let mut sorted = ordering.clone();
        sorted.sort();
        sorted.dedup();
        ensure(ordering == sorted, "chain")?;
        manifests.push((u64::try_from(*sequence).unwrap_or(0), manifest, expected));
    }
    // 2. Header/latest resolves to the final retained proof by epoch,
    //    sequence and the recomputed manifest digest.
    let (last_sequence, last_body, last_digest) = proofs.last().expect("non-empty proofs");
    let latest_bytes: Vec<u8> = database
        .query_row(
            "SELECT latest FROM store_header WHERE singleton = 1",
            [],
            |row| row.get(0),
        )
        .map_err(|_| HierarchyStorageError("chain"))?;
    let latest_value: Value =
        serde_json::from_slice(&latest_bytes).map_err(|_| HierarchyStorageError("header"))?;
    ensure(canonical(&latest_value)? == latest_bytes, "canonical wire")?;
    let latest = rows::commit_ref(&latest_value)?;
    ensure(latest.storage_epoch == epoch, "epoch")?;
    ensure(
        latest.sequence == u64::try_from(*last_sequence).unwrap_or(1),
        "chain",
    )?;
    ensure(
        latest.change_digest == *last_digest
            && latest.change_digest == super::domain_digest(super::CHANGE_DOMAIN, last_body),
        "chain",
    )?;
    // 3. Every retained row re-derives its own commitment against the
    //    manifest whose sequence its lastCommit names. Proof digests travel
    //    with the manifests so the stored lastCommit changeDigest is checked
    //    against the proof it names.
    for table in ROW_TABLES {
        reverify_rows(database, table, epoch, &manifests)?;
    }
    // 4. Missing committed state: every change entry across the chain must
    //    still have its stored row (this kernel never deletes rows), so a
    //    deleted dependency/quota/row after journaling fences the store.
    for (_sequence, manifest, _digest) in &manifests {
        for entry in manifest["changes"]
            .as_array()
            .ok_or(HierarchyStorageError("chain"))?
        {
            let table = entry["table"]
                .as_str()
                .ok_or(HierarchyStorageError("chain"))?;
            let key = canonical(&entry["key"])?;
            let exists: bool = database
                .query_row(
                    &format!("SELECT EXISTS(SELECT 1 FROM {table} WHERE key = ?1)"),
                    rusqlite::params![key],
                    |row| row.get(0),
                )
                .map_err(|_| HierarchyStorageError("physical database"))?;
            ensure(exists, "rows")?;
        }
    }
    // 5. Retained-byte quota recompute for the admitted resource.
    reverify_quotas(database)?;
    Ok(())
}

fn exact_manifest(manifest: &Value, sequence: i64, workspace: &str, epoch: &str) -> Result<()> {
    rows::exact(
        manifest,
        "changes commandKind commitSequence predecessor schemaVersion storageEpoch workspaceId",
    )?;
    ensure(
        manifest["schemaVersion"] == json!(super::STORE_SCHEMA_VERSION),
        "chain",
    )?;
    let kind = manifest["commandKind"]
        .as_str()
        .ok_or(HierarchyStorageError("chain"))?;
    ensure(CLOSED_COMMAND_KINDS.contains(&kind), "chain")?;
    ensure(manifest["storageEpoch"] == json!(epoch), "epoch")?;
    ensure(manifest["workspaceId"] == json!(workspace), "workspace")?;
    ensure(
        manifest["commitSequence"] == json!(sequence.to_string()),
        "chain",
    )?;
    if sequence == 1 {
        ensure(manifest["predecessor"] == Value::Null, "chain")?;
        ensure(manifest["changes"] == json!([]), "chain")?;
        return Ok(());
    }
    ensure(
        !manifest["changes"]
            .as_array()
            .ok_or(HierarchyStorageError("chain"))?
            .is_empty(),
        "chain",
    )
}

fn exact_change_entry(entry: &Value) -> Result<()> {
    rows::exact(entry, "afterDigest beforeDigest key table")?;
    ensure(entry["afterDigest"].is_string(), "chain")?;
    ensure(
        entry["beforeDigest"].is_null() || entry["beforeDigest"].is_string(),
        "chain",
    )?;
    Ok(())
}

/// Re-derive every stored row of one table against its named proof's change
/// entry: canonical key/value bytes, the row-domain commitment, and the
/// stored lastCommit CommitRef resolving to that proof's digest.
fn reverify_rows(
    database: &rusqlite::Connection,
    table: &str,
    epoch: &str,
    manifests: &[(u64, Value, String)],
) -> Result<()> {
    // Proof lookup by sequence: change entries for (table, canonical key).
    let entry_for = |sequence: u64, key_bytes: &[u8]| -> Result<Value> {
        let index = manifests
            .binary_search_by(|probe| probe.0.cmp(&sequence))
            .map_err(|_| HierarchyStorageError("chain"))?;
        let entry_list = manifests[index].1["changes"]
            .as_array()
            .ok_or(HierarchyStorageError("chain"))?;
        let matching = entry_list
            .iter()
            .find(|entry| {
                entry["table"] == json!(table)
                    && canonical(&entry["key"]).map(|bytes| bytes == key_bytes) == Ok(true)
            })
            .cloned()
            .ok_or(HierarchyStorageError("rows"))?;
        Ok(matching)
    };
    // Digest cross-check helper: the proof at `sequence` must own the
    // stored `lastCommit` changeDigest byte-for-byte.
    let proof_digest = |sequence: u64| -> Result<&String> {
        let index = manifests
            .binary_search_by(|probe| probe.0.cmp(&sequence))
            .map_err(|_| HierarchyStorageError("chain"))?;
        Ok(&manifests[index].2)
    };
    let mut statement = database
        .prepare(&format!(
            "SELECT key, value, last_commit, sequence FROM {table}",
        ))
        .map_err(|_| HierarchyStorageError("physical database"))?;
    let mut query = statement
        .query([])
        .map_err(|_| HierarchyStorageError("physical database"))?;
    while let Some(row) = query
        .next()
        .map_err(|_| HierarchyStorageError("physical database"))?
    {
        let key_bytes: Vec<u8> = row
            .get(0)
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let value_bytes: Vec<u8> = row
            .get(1)
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let commit_bytes: Vec<u8> = row
            .get(2)
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let sequence: i64 = row
            .get(3)
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let sequence = u64::try_from(sequence).map_err(|_| HierarchyStorageError("chain"))?;
        let value: Value =
            serde_json::from_slice(&value_bytes).map_err(|_| HierarchyStorageError("rows"))?;
        ensure(canonical(&value)? == value_bytes, "canonical wire")?;
        let key: Value =
            serde_json::from_slice(&key_bytes).map_err(|_| HierarchyStorageError("rows"))?;
        ensure(canonical(&key)? == key_bytes, "canonical wire")?;
        let body = rows::without_field(&value, "lastCommit");
        // The stored lastCommit resolves to the retained proof it names.
        let commit_value: Value =
            serde_json::from_slice(&commit_bytes).map_err(|_| HierarchyStorageError("rows"))?;
        ensure(canonical(&commit_value)? == commit_bytes, "canonical wire")?;
        let commit = rows::commit_ref(&commit_value)?;
        ensure(commit.storage_epoch == epoch, "epoch")?;
        ensure(commit.sequence == sequence, "chain")?;
        ensure(commit.change_digest == *proof_digest(sequence)?, "rows")?;
        // The embedded lastCommit link inside the value must match the
        // verified column bytes exactly.
        ensure(canonical(&value["lastCommit"])? == commit_bytes, "rows")?;
        // The row's digest inside that proof's manifest of the changed set.
        let entry: Value = entry_for(sequence, &key_bytes)?;
        let recomputed = rows::row_digest(table, &key, &body)?;
        ensure(entry["afterDigest"] == json!(recomputed), "rows")?;
    }
    Ok(())
}

/// Recompute the `child-source-retained-bytes` used counters over the
/// admitted child rows: a mismatch is ambiguity that fences, never a repair.
fn reverify_quotas(database: &rusqlite::Connection) -> Result<()> {
    let mut totals: Vec<(Value, u64)> = Vec::new();
    // Walk the admitted child rows; derive (workspace, run) owners from the
    // rows' own keys (bodies where the key is a scalar).
    for section in CHILD_ROWS {
        collect_section(database, section, &mut totals)?;
    }
    let mut statement = database
        .prepare("SELECT key, value FROM quota_usage")
        .map_err(|_| HierarchyStorageError("physical database"))?;
    {
        let mut query = statement
            .query([])
            .map_err(|_| HierarchyStorageError("physical database"))?;
        while let Some(row) = query
            .next()
            .map_err(|_| HierarchyStorageError("physical database"))?
        {
            let key_bytes: Vec<u8> = row
                .get(0)
                .map_err(|_| HierarchyStorageError("physical database"))?;
            let value_bytes: Vec<u8> = row
                .get(1)
                .map_err(|_| HierarchyStorageError("physical database"))?;
            let key: Value =
                serde_json::from_slice(&key_bytes).map_err(|_| HierarchyStorageError("rows"))?;
            let value: Value =
                serde_json::from_slice(&value_bytes).map_err(|_| HierarchyStorageError("rows"))?;
            ensure(rows::quota_key(&key).is_ok(), "rows")?;
            let used = value["used"]
                .as_str()
                .and_then(|text| text.parse::<u64>().ok())
                .ok_or(HierarchyStorageError("quota row"))?;
            ensure(value["reserved"] == json!("0"), "quota row")?;
            let owner = &key["owner"];
            let owner_bytes = canonical(owner)?;
            let recomputed = totals
                .iter()
                .filter(|(scope, _)| canonical(scope) == Ok(owner_bytes.clone()))
                .map(|(_, bytes)| *bytes)
                .sum::<u64>();
            ensure(recomputed == used, "quota")?;
        }
    }
    drop(statement);
    Ok(())
}

const CHILD_ROWS: [&str; 5] = [
    "child_allocations",
    "child_attachments",
    "child_plan_approvals",
    "child_reservations",
    "objects",
];

fn collect_section(
    database: &rusqlite::Connection,
    table: &str,
    totals: &mut Vec<(Value, u64)>,
) -> Result<()> {
    let mut statement = database
        .prepare(&format!("SELECT key, value FROM {table}"))
        .map_err(|_| HierarchyStorageError("physical database"))?;
    let mut query = statement
        .query([])
        .map_err(|_| HierarchyStorageError("physical database"))?;
    while let Some(row) = query
        .next()
        .map_err(|_| HierarchyStorageError("physical database"))?
    {
        let key_bytes: Vec<u8> = row
            .get(0)
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let value_bytes: Vec<u8> = row
            .get(1)
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let key: Value =
            serde_json::from_slice(&key_bytes).map_err(|_| HierarchyStorageError("rows"))?;
        let value: Value =
            serde_json::from_slice(&value_bytes).map_err(|_| HierarchyStorageError("rows"))?;
        let body = rows::without_field(&value, "lastCommit");
        for holder in super::rows::owner_keys(table, &key, &value)? {
            let charge = key_bytes.len() as u64 + canonical(&body)?.len() as u64;
            let mut hold = false;
            for (scope, used_ref) in totals.iter_mut() {
                if canonical(scope)? == canonical(&holder)? {
                    *used_ref += charge;
                    hold = true;
                }
            }
            if !hold {
                totals.push((holder, charge));
            }
        }
    }
    // statement and query are dropped together below.

    Ok(())
}
