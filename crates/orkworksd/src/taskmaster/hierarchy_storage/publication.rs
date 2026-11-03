//! Non-source-valid storage primitives: journal one retained child-source
//! object, one child metadata row or one provenance dependency edge — one
//! transaction per command with its complete change manifest (including the
//! quota charges for the retained-byte pool) and proof entry. This unit
//! publishes NO accepted source-owned transition: the attachment transition
//! needs its exact accepted effect set (initial summary state, the five
//! fresh assignment quota rows, retry/interrupted dispositions) which this
//! mechanism does not supply, so attachment publication stays gated.

use super::connection::{CommandBuild, Connection, RowChange, RowMode};
use super::quota;
use super::rows::{
    self, AdmittedMeta, MetaKind, ObjectKind, SourceContext, CHILD_SOURCE_PROVENANCE,
};
use super::{canonical, ensure, HierarchyStorageError, Result, JOURNAL_COMMAND};
use serde_json::{json, Value};

/// The outcome of a journal command: the CommitRef its rows (and proof) link
/// to. Exact retries return the unchanged current commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct JournalOutcome {
    commit: rows::CommitRef,
}

impl JournalOutcome {
    pub(crate) fn commit(&self) -> rows::CommitRef {
        self.commit.clone()
    }
}

/// Journal one retained child-source object row together with its
/// retained-byte pool charges (run + workspace owners) in ONE transaction:
/// immutable on conflict; an exact retry of identical bytes is a no-op that
/// leaves the committed chain untouched.
pub(crate) fn journal_object(
    store: &Connection,
    kind: ObjectKind,
    context: &SourceContext,
    bytes: &[u8],
) -> Result<JournalOutcome> {
    let admitted = rows::admit_source_object(kind, context, bytes)?;
    let value = serde_json::from_slice::<Value>(&admitted.bytes)
        .map_err(|_| HierarchyStorageError("canonical wire"))?;
    let body = json!({"body": value, "digest": admitted.digest});
    let commit = store.journal_command(JOURNAL_COMMAND, "mutable object", |transaction| {
        build_row_with_charges(transaction, "objects", admitted.key.clone(), body.clone())
    })?;
    Ok(JournalOutcome {
        commit: commit.commit,
    })
}

/// Journal one `child_*` metadata row; the approvals envelope carries the
/// source digest and receipt; reservation/allocation/attachment rows are
/// body-only. Exact retries are no-ops.
pub(crate) fn journal_child_metadata(
    store: &Connection,
    kind: MetaKind,
    admitted: &AdmittedMeta,
    receipt: Option<&Value>,
) -> Result<JournalOutcome> {
    let mut value = admitted.value_base.clone();
    match kind {
        MetaKind::PlanApproval => {
            value["digest"] = json!(admitted.digest);
            let receipt = receipt.ok_or(HierarchyStorageError("receipt"))?;
            value["receipt"] = receipt.clone();
        }
        _ => {
            ensure(receipt.is_none(), "unexpected receipt")?;
        }
    }
    // Envelope/cap validation against the longest possible CommitRef so the
    // actual write can never exceed the complete-pair wrap cap.
    let synthetic_commit = json!({
        "changeDigest": "0".repeat(64),
        "commitSequence": u64::MAX.to_string(),
        "storageEpoch": "0".repeat(64),
    });
    let mut full = value.clone();
    full["lastCommit"] = synthetic_commit;
    rows::check_metadata_row(kind, &admitted.key, &full)?;
    let commit = store.journal_command(JOURNAL_COMMAND, "metadata row", |transaction| {
        build_row_with_charges(
            transaction,
            kind.table_name(),
            admitted.key.clone(),
            value.clone(),
        )
    })?;
    Ok(JournalOutcome {
        commit: commit.commit,
    })
}

/// Shared single-transaction build: detects the exact-retry no-op, then
/// returns the row plus its run/workspace retained-byte charge rows.
fn build_row_with_charges(
    transaction: &rusqlite::Transaction<'_>,
    table: &'static str,
    key: Value,
    value: Value,
) -> Result<CommandBuild> {
    use rusqlite::OptionalExtension;
    let key_bytes = canonical(&key)?;
    let existing: Option<Vec<u8>> = transaction
        .query_row(
            &format!("SELECT value FROM {table} WHERE key = ?1"),
            rusqlite::params![key_bytes],
            |row| row.get::<_, Vec<u8>>(0),
        )
        .optional()
        .map_err(|_| HierarchyStorageError("physical database"))?;
    if let Some(bytes) = existing {
        let stored: Value = serde_json::from_slice(&bytes)
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let stored_body = rows::without_field(&stored, "lastCommit");
        if canonical(&stored_body)? == canonical(&value)? {
            return Ok(CommandBuild::NoOp);
        }
        let reason = if table == "objects" {
            "mutable object"
        } else {
            "metadata row"
        };
        return Err(HierarchyStorageError(reason));
    }
    // Quota charge rows for the same transaction; missing or ambiguous quota
    // rows fence the whole command, and the accepted caps are inclusive.
    let charge = (canonical(&key)?.len() + canonical(&value)?.len()) as i128;
    let mut changes = vec![RowChange {
        table,
        mode: RowMode::InsertStrict,
        key: key.clone(),
        value: value.clone(),
    }];
    for holder in rows::owner_keys(table, &key, &value)? {
        let quota_key = quota_key_for(&holder)?;
        let stored: Value = transaction
            .query_row(
                "SELECT value FROM quota_usage WHERE key = ?1",
                rusqlite::params![canonical(&quota_key)?],
                |row| row.get::<_, Vec<u8>>(0),
            )
            .optional()
            .map_err(|_| HierarchyStorageError("physical database"))?
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .ok_or(HierarchyStorageError("quota row"))?;
        ensure(
            stored["reserved"] == json!("0"),
            "reserved row state is ambiguous",
        )?;
        let used = stored["used"]
            .as_str()
            .and_then(|used| used.parse::<u64>().ok())
            .ok_or(HierarchyStorageError("quota row"))?;
        // Checked nonzero-allowed counter arithmetic in i128 space; the cap
        // comparison stays in i128 so no cast can truncate.
        let candidate = used as i128 + charge;
        ensure(candidate >= 0, "quota delta")?;
        let cap = quota::owner_cap_for(&holder)?;
        ensure(candidate <= i128::from(cap), "quota capacity")?;
        let candidate = candidate as u64;
        let new_value = json!({"reserved": "0", "used": candidate.to_string()});
        changes.push(RowChange {
            table: "quota_usage",
            mode: RowMode::Replace,
            key: quota_key,
            value: new_value,
        });
    }
    Ok(CommandBuild::Rows(changes))
}

fn quota_key_for(owner: &Value) -> Result<Value> {
    Ok(json!({"owner": owner, "resource": super::rows::RETAINED_BYTES_RESOURCE}))
}

/// Provenance dependency edge: `child-source-provenance` only; both endpoints
/// must resolve to retained object rows with the cited digests.
pub(crate) fn journal_dependency(
    store: &Connection,
    consumer: &Value,
    target: &Value,
) -> Result<JournalOutcome> {
    let preflight = {
        for endpoint in [consumer, target] {
            rows::object_ref(endpoint)?;
            let key = &endpoint["key"];
            let stored = store
                .find_row("objects", key)?
                .ok_or(HierarchyStorageError("join"))?;
            let digest = stored["digest"]
                .as_str()
                .ok_or(HierarchyStorageError("join"))?;
            ensure(endpoint["digest"] == json!(digest), "join")?;
        }
        json!({
            "consumer": consumer,
            "purpose": CHILD_SOURCE_PROVENANCE,
            "target": target,
        })
    };
    let commit = store.journal_command(JOURNAL_COMMAND, "dependency", |transaction| {
        use rusqlite::OptionalExtension;
        let key_bytes = canonical(&preflight)?;
        let existing: Option<Vec<u8>> = transaction
            .query_row(
                "SELECT value FROM dependencies WHERE key = ?1",
                rusqlite::params![key_bytes],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| HierarchyStorageError("physical database"))?;
        if let Some(bytes) = existing {
            let stored: Value = serde_json::from_slice(&bytes)
                .map_err(|_| HierarchyStorageError("physical database"))?;
            if canonical(&rows::without_field(&stored, "lastCommit"))? == canonical(&json!({}))? {
                return Ok(CommandBuild::NoOp);
            }
        }
        Ok(CommandBuild::Rows(vec![RowChange {
            table: "dependencies",
            mode: RowMode::InsertStrict,
            key: preflight,
            value: json!({}),
        }]))
    })?;
    Ok(JournalOutcome {
        commit: commit.commit,
    })
}
