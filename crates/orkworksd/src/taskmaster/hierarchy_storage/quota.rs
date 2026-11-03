//! `child-source-retained-bytes` quota rows: established only by explicit
//! fresh-preparation establishment (used="0", reserved="0"), charged by
//! checked-u64 deltas, fenced on missing/ambiguous rows, and rejected at the
//! accepted inclusive caps (8 MiB per run / 64 MiB per workspace) with the
//! complete old state preserved. Every command is one transaction.

use super::connection::{CommandBuild, Connection, RowChange, RowMode};
use super::rows::{quota_key, RETAINED_BYTES_RESOURCE};
use super::{ensure, HierarchyStorageError, Result};
use serde_json::{json, Value};

/// Accepted inclusive byte caps for the retained source pool.
pub(crate) const QUOTA_RUN_CAP: u64 = 8 * 1024 * 1024;
pub(crate) const QUOTA_WORKSPACE_CAP: u64 = 64 * 1024 * 1024;

fn key_for(owner: &Value) -> Result<Value> {
    let parsed = quota_key(&json!({"owner": owner, "resource": RETAINED_BYTES_RESOURCE}))?;
    let kind = &parsed["owner"]["kind"];
    ensure(kind == "run" || kind == "workspace", "resource_scope")?;
    Ok(json!({"owner": parsed["owner"], "resource": parsed["resource"]}))
}

/// Establish the fresh-preparation row at used="0"/reserved="0". Rejected if
/// the row exists (never rewritten) or the store is unavailable.
pub(crate) fn establish(store: &Connection, owner: &Value) -> Result<Value> {
    establish_resource(store, owner, RETAINED_BYTES_RESOURCE)?;
    read_used(store, owner)
}

/// Resource-scoped establishment surface; only the retained-bytes resource
/// is admitted by this kernel.
pub(crate) fn establish_resource(
    store: &Connection,
    owner: &Value,
    resource: &str,
) -> Result<Value> {
    ensure(resource == RETAINED_BYTES_RESOURCE, "resource")?;
    let key = key_for(owner)?;
    let value = json!({"reserved": "0", "used": "0"});
    store.journal_command(super::JOURNAL_COMMAND, "quota row", |transaction| {
        use rusqlite::OptionalExtension;
        let existing: Option<Vec<u8>> = transaction
            .query_row(
                "SELECT value FROM quota_usage WHERE key = ?1",
                rusqlite::params![super::canonical(&key)?],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| HierarchyStorageError("physical database"))?;
        if existing.is_some() {
            return Err(HierarchyStorageError("quota row"));
        }
        Ok(CommandBuild::Rows(vec![RowChange {
            table: "quota_usage",
            mode: RowMode::InsertStrict,
            key,
            value,
        }]))
    })?;
    read_used(store, owner)
}

pub(crate) fn owner_cap_for(owner: &Value) -> Result<u64> {
    match owner["kind"].as_str() {
        Some("run") => Ok(QUOTA_RUN_CAP),
        Some("workspace") => Ok(QUOTA_WORKSPACE_CAP),
        _ => Err(HierarchyStorageError("resource_scope")),
    }
}

/// Apply one signed delta with checked Counter arithmetic inside one
/// transaction; refuses on a missing row (fence), a nonzero `reserved`
/// field, underflow (no-refund) and an inclusive cap overflow, preserving
/// the complete old state. An exact no-delta command is a no-op.
pub(crate) fn apply_delta(store: &Connection, owner: &Value, delta: i128) -> Result<Value> {
    let key = key_for(owner)?;
    let cap = owner_cap_for(owner)?;
    store.journal_command(super::JOURNAL_COMMAND, "quota row", |transaction| {
        use rusqlite::OptionalExtension;
        let existing: Option<Vec<u8>> = transaction
            .query_row(
                "SELECT value FROM quota_usage WHERE key = ?1",
                rusqlite::params![super::canonical(&key)?],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| HierarchyStorageError("physical database"))?;
        let stored: Value = existing
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .ok_or(HierarchyStorageError("quota row"))?;
        ensure(
            stored["reserved"] == json!("0"),
            "reserved row state is ambiguous",
        )?;
        let used = stored["used"]
            .as_str()
            .and_then(|value| value.parse::<u64>().ok())
            .ok_or(HierarchyStorageError("quota row"))?;
        // Checked nonzero-allowed counter arithmetic in i128 space; the
        // cap comparison stays in i128 so no cast can truncate.
        let candidate = used as i128 + delta;
        ensure(candidate >= 0, "quota delta")?;
        ensure(candidate <= i128::from(cap), "quota capacity")?;
        let candidate = candidate as u64;
        if candidate == used {
            return Ok(CommandBuild::NoOp);
        }
        let value = json!({"reserved": "0", "used": candidate.to_string()});
        Ok(CommandBuild::Rows(vec![RowChange {
            table: "quota_usage",
            mode: RowMode::Replace,
            key,
            value,
        }]))
    })?;
    read_used(store, owner)
}

/// Read the current quota row value (used/reserved), refusing missing rows.
pub(crate) fn read_used(store: &Connection, owner: &Value) -> Result<Value> {
    let key = key_for(owner)?;
    store
        .find_row("quota_usage", &key)?
        .ok_or(HierarchyStorageError("quota row"))
}
