//! `child-source-retained-bytes` quota rows: established only by explicit
//! fresh-preparation establishment (used="0", reserved="0"), charged by
//! checked-u64 deltas, fenced on missing/ambiguous rows, and rejected at the
//! accepted inclusive caps (8 MiB per run / 64 MiB per workspace) with the
//! complete old state preserved. Every journal command is one transaction.

use super::connection::Connection;
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
    store.journal_rows(
        super::JOURNAL_COMMAND,
        vec![super::connection::RowChange {
            table: "quota_usage",
            mode: super::connection::RowMode::Insert,
            key,
            value,
        }],
        "quota row",
    )?;
    read_used(store, owner)
}

fn owner_cap(owner: &Value) -> Result<u64> {
    match owner["kind"].as_str() {
        Some("run") => Ok(QUOTA_RUN_CAP),
        Some("workspace") => Ok(QUOTA_WORKSPACE_CAP),
        _ => Err(HierarchyStorageError("resource_scope")),
    }
}

/// Apply one signed delta with checked Counter arithmetic; refuses on a
/// missing row (fence), a nonzero `reserved` field, underflow (no-refund)
/// and an inclusive cap overflow, preserving the complete old state.
pub(crate) fn apply_delta(store: &Connection, owner: &Value, delta: i128) -> Result<Value> {
    let key = key_for(owner)?;
    let cap = owner_cap(owner)?;
    let current = read_used(store, owner)?;
    ensure(
        current["reserved"] == json!("0"),
        "reserved row state is ambiguous",
    )?;
    let used = match current.get("used").and_then(|value| value.as_str()) {
        Some(value) => value
            .parse::<u64>()
            .map_err(|_| HierarchyStorageError("quota row"))?,
        None => return Err(HierarchyStorageError("quota row")),
    };
    // Checked nonzero-allowed counter arithmetic in i128 space.
    let candidate = used as i128 + delta;
    ensure(candidate >= 0, "quota delta")?;
    let candidate = candidate as u64;
    ensure(candidate <= cap, "quota capacity")?;
    let value = json!({"reserved": "0", "used": candidate.to_string()});
    store.journal_rows(
        super::JOURNAL_COMMAND,
        vec![super::connection::RowChange {
            table: "quota_usage",
            mode: super::connection::RowMode::Replace,
            key,
            value,
        }],
        "quota row",
    )?;
    read_used(store, owner)
}

/// Read the current quota row value (used/reserved), refusing missing rows.
pub(crate) fn read_used(store: &Connection, owner: &Value) -> Result<Value> {
    let key = key_for(owner)?;
    store.read_row("quota_usage", &key, "quota row")
}
