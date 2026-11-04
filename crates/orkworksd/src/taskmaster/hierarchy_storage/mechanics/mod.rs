#![allow(dead_code)]
//! Isolated, unqualified storage mechanics retained only for unit tests.
//!
//! The 17-family catalog, generic journal and contiguous proof chain do not
//! satisfy the accepted format-4 profile. These fixtures confer no production
//! admission or source-owned publication authority; see #1356 and #1176.
use serde_json::Value;

pub(crate) mod connection;
pub(crate) mod publication;
pub(crate) mod quota;
pub(crate) mod reload;
pub(crate) mod rows;
pub(crate) mod schema;
#[cfg(test)]
mod tests;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct HierarchyStorageError(pub &'static str);

// Test-only journal seam (cfg(test)); production builds have no arm.
// Thread-local so parallel tests never arm each other.
#[cfg(test)]
thread_local!(pub(crate) static TEST_SEAM: std::sync::atomic::AtomicBool =
    const { std::sync::atomic::AtomicBool::new(false) });
type Result<T> = std::result::Result<T, HierarchyStorageError>;

/// Canonical byte domain for change manifests (format 4).
pub(super) const CHANGE_DOMAIN: &str = "orkworks.hierarchy-storage-change.v4";
/// Canonical byte domain for logical row commitments (format 4).
pub(super) const ROW_DOMAIN: &str = "orkworks.hierarchy-storage-row.v4";
/// Non-producer command kind for the storage-owner journal primitives.
pub(super) const JOURNAL_COMMAND: &str = "hierarchy-storage-journal";
/// Closed kind for fresh-store initialization only.
pub(super) const INITIALIZE_COMMAND: &str = "initialize-store";

pub(super) const STORE_SCHEMA_VERSION: i64 = 4;
/// 4 KiB page size is an accepted scope-1 configuration requirement.
pub(super) const PAGE_SIZE: i64 = 4096;
/// 1 GiB main file cap through the accepted page default.
pub(super) const MAX_PAGE_COUNT: i64 = 262144;

pub(super) fn ensure(ok: bool, reason: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(HierarchyStorageError(reason))
    }
}

/// Canonical JSON serialization; callers byte-compare against stored bytes.
pub(super) fn canonical(v: &Value) -> Result<Vec<u8>> {
    crate::taskmaster::coordinator::canonical_json_bytes(v)
        .map_err(|_| HierarchyStorageError("canonical wire"))
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    crate::taskmaster::coordinator::sha256_hex(bytes)
}

/// Domain digest over canonical bytes with the trailing LF separator.
pub(super) fn domain_digest(domain: &str, canonical_bytes: &[u8]) -> String {
    let mut bytes = domain.as_bytes().to_vec();
    bytes.push(b'\n');
    bytes.extend_from_slice(canonical_bytes);
    sha256_hex(&bytes)
}
