use sha2::{Digest, Sha256};
use std::path::PathBuf;

pub(crate) fn iso_now() -> String {
    chrono::Utc::now().to_rfc3339()
}

pub(crate) fn workspace_hash(path: &std::path::Path) -> String {
    let canonical = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = Sha256::new();
    hasher.update(canonical.to_string_lossy().as_bytes());
    let result = hasher.finalize();
    hex::encode(&result[..8])
}

pub(crate) fn orkworks_global_dir(workspace_path: &std::path::Path) -> Option<PathBuf> {
    dirs::home_dir().map(|h| {
        h.join(".orkworks")
            .join("workspaces")
            .join(workspace_hash(workspace_path))
    })
}
