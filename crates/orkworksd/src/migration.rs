use std::path::Path;

pub fn migrate_if_needed(workspace_path: &Path, global_dir: &Path) {
    let old_dir = workspace_path.join(".orkworks");
    let old_workspace_json = old_dir.join("workspace.json");

    if !old_workspace_json.exists() {
        return;
    }

    let sessions_dir = global_dir.join("sessions");
    if sessions_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&sessions_dir) {
            if entries.count() > 0 {
                return;
            }
        }
    }

    let _ = std::fs::create_dir_all(&sessions_dir);

    for subdir in &["sessions", "events", "capacity"] {
        let old_sub = old_dir.join(subdir);
        let new_sub = global_dir.join(subdir);
        if old_sub.is_dir() {
            let _ = std::fs::create_dir_all(&new_sub);
            if let Ok(entries) = std::fs::read_dir(&old_sub) {
                for entry in entries.flatten() {
                    let dest = new_sub.join(entry.file_name());
                    let _ = std::fs::copy(entry.path(), &dest);
                }
            }
        }
    }

    let old_workspace = old_dir.join("workspace.json");
    if old_workspace.exists() {
        let _ = std::fs::copy(&old_workspace, global_dir.join("workspace.json"));
    }
}
