//! Refuse the unqualified profile before inspecting or touching its path.

use super::{HierarchyStorageError, Result};
use std::path::Path;

/// Unavailable until a complete storage profile is qualified. There is no
/// production constructor or publication API behind this fence.
pub(crate) struct Connection {
    _private: (),
}

impl Connection {
    pub(crate) fn open(_path: impl AsRef<Path>, _workspace_id: &str) -> Result<Self> {
        Err(HierarchyStorageError("unqualified profile"))
    }
}
