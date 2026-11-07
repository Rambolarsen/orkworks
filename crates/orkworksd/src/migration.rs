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

    if let Err(error) = std::fs::create_dir_all(&sessions_dir) {
        tracing::warn!(path = %sessions_dir.display(), %error, "failed to create legacy migration directory");
    }

    for subdir in &["sessions", "events", "capacity"] {
        let old_sub = old_dir.join(subdir);
        let new_sub = global_dir.join(subdir);
        if old_sub.is_dir() {
            if let Err(error) = std::fs::create_dir_all(&new_sub) {
                tracing::warn!(path = %new_sub.display(), %error, "failed to create legacy migration directory");
            }
            if let Ok(entries) = std::fs::read_dir(&old_sub) {
                for entry in entries.flatten() {
                    let dest = new_sub.join(entry.file_name());
                    let source = entry.path();
                    if let Err(error) = std::fs::copy(&source, &dest) {
                        tracing::warn!(source = %source.display(), destination = %dest.display(), %error, "failed to copy legacy migration file");
                    }
                }
            }
        }
    }

    let old_workspace = old_dir.join("workspace.json");
    if old_workspace.exists() {
        let destination = global_dir.join("workspace.json");
        if let Err(error) = std::fs::copy(&old_workspace, &destination) {
            tracing::warn!(source = %old_workspace.display(), destination = %destination.display(), %error, "failed to copy legacy workspace metadata");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::sync::{Arc, Mutex};

    #[derive(Clone, Default)]
    struct LogCapture(Arc<Mutex<Vec<u8>>>);

    impl Write for LogCapture {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogCapture {
        type Writer = Self;

        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn migrate_with_logs(workspace: &Path, global: &Path) -> String {
        let capture = LogCapture::default();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .without_time()
            .with_writer(capture.clone())
            .finish();
        tracing::subscriber::with_default(subscriber, || migrate_if_needed(workspace, global));
        let bytes = capture.0.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    fn legacy_workspace(root: &Path) -> std::path::PathBuf {
        let workspace = root.join("workspace");
        let legacy = workspace.join(".orkworks");
        for subdir in ["sessions", "events", "capacity"] {
            std::fs::create_dir_all(legacy.join(subdir)).unwrap();
            std::fs::write(legacy.join(subdir).join("record.json"), subdir).unwrap();
        }
        std::fs::write(legacy.join("workspace.json"), "legacy workspace").unwrap();
        workspace
    }

    #[test]
    fn migration_copies_all_legacy_records_without_removing_sources() {
        let root = tempfile::tempdir().unwrap();
        let workspace = legacy_workspace(root.path());
        let global = root.path().join("global");

        assert!(migrate_with_logs(&workspace, &global).is_empty());

        for subdir in ["sessions", "events", "capacity"] {
            assert_eq!(
                std::fs::read_to_string(global.join(subdir).join("record.json")).unwrap(),
                subdir
            );
            assert_eq!(
                std::fs::read_to_string(
                    workspace.join(".orkworks").join(subdir).join("record.json")
                )
                .unwrap(),
                subdir
            );
        }
        assert_eq!(
            std::fs::read_to_string(global.join("workspace.json")).unwrap(),
            "legacy workspace"
        );
        assert_eq!(
            std::fs::read_to_string(workspace.join(".orkworks/workspace.json")).unwrap(),
            "legacy workspace"
        );
    }

    #[test]
    fn migration_skips_all_copying_when_global_sessions_already_has_entries() {
        let root = tempfile::tempdir().unwrap();
        let workspace = legacy_workspace(root.path());
        let global = root.path().join("global");
        std::fs::create_dir_all(global.join("sessions")).unwrap();
        std::fs::write(global.join("sessions/existing.json"), "existing session").unwrap();
        std::fs::write(global.join("workspace.json"), "existing workspace").unwrap();

        assert!(migrate_with_logs(&workspace, &global).is_empty());

        assert_eq!(
            std::fs::read_to_string(global.join("sessions/existing.json")).unwrap(),
            "existing session"
        );
        assert_eq!(
            std::fs::read_to_string(global.join("workspace.json")).unwrap(),
            "existing workspace"
        );
        assert!(!global.join("sessions/record.json").exists());
        assert!(!global.join("events").exists());
        assert!(!global.join("capacity").exists());
    }

    #[test]
    fn migration_logs_record_copy_failure_and_continues_other_records() {
        let root = tempfile::tempdir().unwrap();
        let workspace = legacy_workspace(root.path());
        let global = root.path().join("global");
        let destination = global.join("events/record.json");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(workspace.join(".orkworks/events/other.json"), "other event").unwrap();

        let logs = migrate_with_logs(&workspace, &global);

        assert!(logs.contains("WARN"), "{logs}");
        assert!(
            logs.contains("failed to copy legacy migration file"),
            "{logs}"
        );
        assert!(
            logs.contains(
                &workspace
                    .join(".orkworks/events/record.json")
                    .display()
                    .to_string()
            ),
            "{logs}"
        );
        assert!(logs.contains(&destination.display().to_string()), "{logs}");
        assert!(logs.contains("error="), "{logs}");
        assert_eq!(
            std::fs::read_to_string(global.join("events/other.json")).unwrap(),
            "other event"
        );
        assert_eq!(
            std::fs::read_to_string(global.join("capacity/record.json")).unwrap(),
            "capacity"
        );
        assert_eq!(
            std::fs::read_to_string(workspace.join(".orkworks/events/record.json")).unwrap(),
            "events"
        );
    }

    #[test]
    fn migration_logs_directory_creation_failure() {
        let root = tempfile::tempdir().unwrap();
        let workspace = legacy_workspace(root.path());
        let global = root.path().join("global");
        std::fs::create_dir_all(&global).unwrap();
        let destination = global.join("events");
        std::fs::write(&destination, "not a directory").unwrap();

        let logs = migrate_with_logs(&workspace, &global);

        assert!(logs.contains("WARN"), "{logs}");
        assert!(
            logs.contains("failed to create legacy migration directory"),
            "{logs}"
        );
        assert!(logs.contains(&destination.display().to_string()), "{logs}");
        assert!(logs.contains("error="), "{logs}");
        assert_eq!(
            std::fs::read_to_string(&destination).unwrap(),
            "not a directory"
        );
        assert_eq!(
            std::fs::read_to_string(global.join("capacity/record.json")).unwrap(),
            "capacity"
        );
    }

    #[test]
    fn migration_logs_workspace_copy_failure() {
        let root = tempfile::tempdir().unwrap();
        let workspace = legacy_workspace(root.path());
        let global = root.path().join("global");
        let destination = global.join("workspace.json");
        std::fs::create_dir_all(&destination).unwrap();

        let logs = migrate_with_logs(&workspace, &global);

        assert!(logs.contains("WARN"), "{logs}");
        assert!(
            logs.contains("failed to copy legacy workspace metadata"),
            "{logs}"
        );
        assert!(
            logs.contains(
                &workspace
                    .join(".orkworks/workspace.json")
                    .display()
                    .to_string()
            ),
            "{logs}"
        );
        assert!(logs.contains(&destination.display().to_string()), "{logs}");
        assert!(logs.contains("error="), "{logs}");
        assert!(destination.is_dir());
    }
}
