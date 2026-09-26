use rusqlite::{Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

static LABEL_REFRESH_GENERATIONS: LazyLock<Mutex<std::collections::HashMap<String, u64>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
static LABEL_REFRESH_GATES: LazyLock<Mutex<std::collections::HashMap<String, Arc<Mutex<()>>>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));
#[derive(Clone)]
struct BlockedNativeLabelRefresh {
    native_session_id: String,
}
static BLOCKED_NATIVE_LABEL_IDS: LazyLock<
    Mutex<std::collections::HashMap<String, BlockedNativeLabelRefresh>>,
> = LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

pub(crate) fn reserve_label_refresh_generation(session_id: &str) -> u64 {
    let gate = label_refresh_gate(session_id);
    let _gate = gate.lock().unwrap();
    let mut generations = LABEL_REFRESH_GENERATIONS.lock().unwrap();
    let generation = generations.entry(session_id.to_owned()).or_insert(0);
    *generation = generation.saturating_add(1);
    *generation
}

pub(crate) fn invalidate_label_refresh_generation(session_id: &str) {
    let _ = reserve_label_refresh_generation(session_id);
}

pub(crate) fn clear_label_refresh_generation(session_id: &str) {
    let gate = label_refresh_gate(session_id);
    let _gate = gate.lock().unwrap();
    LABEL_REFRESH_GENERATIONS.lock().unwrap().remove(session_id);
}

pub(crate) fn block_native_label_refresh(session_id: &str, native_session_id: &str) {
    BLOCKED_NATIVE_LABEL_IDS.lock().unwrap().insert(
        session_id.to_owned(),
        BlockedNativeLabelRefresh {
            native_session_id: native_session_id.to_owned(),
        },
    );
}

pub(crate) fn clear_native_label_refresh_block(session_id: &str) {
    BLOCKED_NATIVE_LABEL_IDS.lock().unwrap().remove(session_id);
}

pub(crate) fn native_label_refresh_is_blocked(session_id: &str, native_session_id: &str) -> bool {
    BLOCKED_NATIVE_LABEL_IDS
        .lock()
        .unwrap()
        .get(session_id)
        .is_some_and(|blocked| blocked.native_session_id == native_session_id)
}

pub(crate) fn native_identity_reset_is_authorized(
    session_id: &str,
    native_session_id: &str,
) -> bool {
    let blocked = BLOCKED_NATIVE_LABEL_IDS.lock().unwrap();
    let Some(blocked) = blocked.get(session_id) else {
        return false;
    };
    blocked.native_session_id == native_session_id
}

pub(crate) fn accept_native_label_identity(session_id: &str, native_session_id: &str) -> bool {
    let mut blocked = BLOCKED_NATIVE_LABEL_IDS.lock().unwrap();
    match blocked.get(session_id) {
        Some(previous) if previous.native_session_id == native_session_id => false,
        Some(_) => {
            blocked.remove(session_id);
            true
        }
        None => true,
    }
}

fn label_refresh_gate(session_id: &str) -> Arc<Mutex<()>> {
    LABEL_REFRESH_GATES
        .lock()
        .unwrap()
        .entry(session_id.to_owned())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

pub(crate) fn with_label_refresh_generation<T>(
    session_id: &str,
    expected_generation: u64,
    operation: impl FnOnce() -> T,
) -> Option<T> {
    let gate = label_refresh_gate(session_id);
    let _gate = gate.lock().unwrap();
    let matches = LABEL_REFRESH_GENERATIONS
        .lock()
        .unwrap()
        .get(session_id)
        .copied()
        .unwrap_or(0)
        == expected_generation;
    matches.then(operation)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CodexLabelField {
    Name,
    Title,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CodexLabelCandidate {
    pub(crate) text: String,
    pub(crate) field: CodexLabelField,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CodexStoreError {
    Unavailable,
}

pub(crate) fn lookup_label_candidate(
    native_session_id: &str,
) -> Result<Option<CodexLabelCandidate>, CodexStoreError> {
    let path = codex_store_path().ok_or(CodexStoreError::Unavailable)?;
    lookup_label_candidate_at(&path, native_session_id)
}

pub(crate) fn has_saved_session(native_session_id: &str) -> bool {
    let Some(path) = codex_store_path() else {
        return false;
    };
    is_saved_thread_at(&path, native_session_id)
}

pub(crate) fn saved_sessions(native_session_ids: &[String]) -> std::collections::HashSet<String> {
    let Some(path) = codex_store_path() else {
        return std::collections::HashSet::new();
    };
    saved_sessions_at(&path, native_session_ids)
}

fn saved_sessions_at(
    path: &Path,
    native_session_ids: &[String],
) -> std::collections::HashSet<String> {
    if native_session_ids.is_empty() {
        return std::collections::HashSet::new();
    }
    let Ok(connection) =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        return std::collections::HashSet::new();
    };
    if connection.busy_timeout(Duration::from_millis(100)).is_err() {
        return std::collections::HashSet::new();
    }
    let mut saved = std::collections::HashSet::new();
    for chunk in native_session_ids.chunks(500) {
        let placeholders = std::iter::repeat_n("?", chunk.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!("SELECT id, rollout_path FROM threads WHERE id IN ({placeholders})");
        let Ok(mut statement) = connection.prepare(&sql) else {
            return std::collections::HashSet::new();
        };
        let Ok(rows) = statement.query_map(rusqlite::params_from_iter(chunk), |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
        }) else {
            return std::collections::HashSet::new();
        };
        for row in rows {
            let Ok((native_session_id, rollout_path)) = row else {
                return std::collections::HashSet::new();
            };
            if rollout_path.is_some_and(|path| Path::new(&path).is_file()) {
                saved.insert(native_session_id);
            }
        }
    }
    saved
}

fn is_saved_thread_at(path: &Path, native_session_id: &str) -> bool {
    if native_session_id.is_empty() {
        return false;
    }
    let Ok(connection) =
        Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
    else {
        return false;
    };
    if connection.busy_timeout(Duration::from_millis(100)).is_err() {
        return false;
    }
    let rollout_path = connection
        .query_row(
            "SELECT rollout_path FROM threads WHERE id = ?1 LIMIT 1",
            [native_session_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .ok()
        .flatten()
        .flatten();
    rollout_path.is_some_and(|rollout_path| Path::new(&rollout_path).is_file())
}

fn lookup_label_candidate_at(
    path: &Path,
    native_session_id: &str,
) -> Result<Option<CodexLabelCandidate>, CodexStoreError> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| CodexStoreError::Unavailable)?;
    connection
        .busy_timeout(Duration::from_millis(100))
        .map_err(|_| CodexStoreError::Unavailable)?;

    let has_name_column = {
        let mut statement = connection
            .prepare("PRAGMA table_info(threads)")
            .map_err(|_| CodexStoreError::Unavailable)?;
        let mut rows = statement
            .query([])
            .map_err(|_| CodexStoreError::Unavailable)?;
        let mut found = false;
        while let Some(row) = rows.next().map_err(|_| CodexStoreError::Unavailable)? {
            found |= row
                .get::<_, String>(1)
                .map_err(|_| CodexStoreError::Unavailable)?
                == "name";
        }
        found
    };

    let row = if has_name_column {
        connection
            .query_row(
                "SELECT name, title FROM threads WHERE id = ?1 LIMIT 1",
                [native_session_id],
                |row| {
                    Ok((
                        row.get::<_, Option<String>>(0)?,
                        row.get::<_, Option<String>>(1)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| CodexStoreError::Unavailable)?
    } else {
        connection
            .query_row(
                "SELECT title FROM threads WHERE id = ?1 LIMIT 1",
                [native_session_id],
                |row| Ok((None, row.get::<_, Option<String>>(0)?)),
            )
            .optional()
            .map_err(|_| CodexStoreError::Unavailable)?
    };

    let Some((name, title)) = row else {
        return Ok(None);
    };

    if let Some(text) = normalize_label(name) {
        return Ok(Some(CodexLabelCandidate {
            text,
            field: CodexLabelField::Name,
        }));
    }
    Ok(normalize_label(title).map(|text| CodexLabelCandidate {
        text,
        field: CodexLabelField::Title,
    }))
}

fn codex_store_path() -> Option<PathBuf> {
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|path| path.join(".codex")))?;
    Some(home.join("state_5.sqlite"))
}

fn normalize_label(value: Option<String>) -> Option<String> {
    let value = value?;
    if value.chars().any(char::is_control) {
        return None;
    }
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    let bounded: String = normalized.chars().take(100).collect();
    (!bounded.is_empty()).then_some(bounded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn fixture() -> tempfile::NamedTempFile {
        let file = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    name TEXT,
                    title TEXT
                );",
            )
            .unwrap();
        file
    }

    #[test]
    fn prefers_saved_name_over_generated_title() {
        let file = fixture();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute(
                "INSERT INTO threads (id, name, title) VALUES (?1, ?2, ?3)",
                ("native-1", "Saved session name", "Generated title"),
            )
            .unwrap();

        assert_eq!(
            lookup_label_candidate_at(file.path(), "native-1").unwrap(),
            Some(CodexLabelCandidate {
                text: "Saved session name".into(),
                field: CodexLabelField::Name,
            })
        );
    }

    #[test]
    fn falls_back_to_title_when_name_is_blank() {
        let file = fixture();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute(
                "INSERT INTO threads (id, name, title) VALUES (?1, ?2, ?3)",
                ("native-2", "  ", "Generated title"),
            )
            .unwrap();

        assert_eq!(
            lookup_label_candidate_at(file.path(), "native-2").unwrap(),
            Some(CodexLabelCandidate {
                text: "Generated title".into(),
                field: CodexLabelField::Title,
            })
        );
    }

    #[test]
    fn does_not_match_a_different_native_session_id() {
        let file = fixture();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute(
                "INSERT INTO threads (id, name, title) VALUES (?1, ?2, ?3)",
                ("native-3", "Other session", "Other title"),
            )
            .unwrap();

        assert_eq!(
            lookup_label_candidate_at(file.path(), "native-4").unwrap(),
            None
        );
    }

    #[test]
    fn saved_thread_requires_a_matching_row_and_existing_rollout_file() {
        let file = fixture();
        let rollout = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute_batch("ALTER TABLE threads ADD COLUMN rollout_path TEXT;")
            .unwrap();
        connection
            .execute(
                "INSERT INTO threads (id, rollout_path) VALUES (?1, ?2)",
                ("native-saved", rollout.path().to_string_lossy().as_ref()),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO threads (id, rollout_path) VALUES (?1, ?2)",
                ("native-no-rollout", "/missing/rollout.jsonl"),
            )
            .unwrap();
        drop(connection);

        assert!(is_saved_thread_at(file.path(), "native-saved"));
        assert!(!is_saved_thread_at(file.path(), "native-missing"));
        assert!(!is_saved_thread_at(file.path(), "native-no-rollout"));

        std::fs::remove_file(rollout.path()).unwrap();
        assert!(!is_saved_thread_at(file.path(), "native-saved"));
    }

    #[test]
    fn saved_sessions_batches_ids_on_one_read_connection() {
        let file = fixture();
        let rollout = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute_batch("ALTER TABLE threads ADD COLUMN rollout_path TEXT;")
            .unwrap();
        connection
            .execute(
                "INSERT INTO threads (id, rollout_path) VALUES (?1, ?2)",
                ("native-saved", rollout.path().to_string_lossy().as_ref()),
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO threads (id, rollout_path) VALUES (?1, ?2)",
                ("native-no-rollout", "/missing/rollout.jsonl"),
            )
            .unwrap();
        drop(connection);

        let saved = saved_sessions_at(
            file.path(),
            &[
                "native-saved".into(),
                "native-missing".into(),
                "native-no-rollout".into(),
            ],
        );

        assert_eq!(
            saved,
            std::collections::HashSet::from(["native-saved".into()])
        );
    }

    #[test]
    fn rejects_control_text_and_bounds_display_text() {
        let file = fixture();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute(
                "INSERT INTO threads (id, name, title) VALUES (?1, ?2, ?3)",
                ("native-5", "has\ncontrol", "unused"),
            )
            .unwrap();
        assert_eq!(
            lookup_label_candidate_at(file.path(), "native-5").unwrap(),
            Some(CodexLabelCandidate {
                text: "unused".into(),
                field: CodexLabelField::Title,
            })
        );

        connection
            .execute(
                "UPDATE threads SET name = NULL, title = ?2 WHERE id = ?1",
                ("native-5", "x".repeat(200)),
            )
            .unwrap();
        let candidate = lookup_label_candidate_at(file.path(), "native-5")
            .unwrap()
            .unwrap();
        assert_eq!(candidate.field, CodexLabelField::Title);
        assert_eq!(candidate.text.chars().count(), 100);
    }

    #[test]
    fn unsupported_schema_is_reported_as_unavailable() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute("CREATE TABLE not_threads (id TEXT)", [])
            .unwrap();

        assert!(matches!(
            lookup_label_candidate_at(file.path(), "native-6"),
            Err(CodexStoreError::Unavailable)
        ));
    }

    #[test]
    fn reads_title_when_the_optional_name_column_is_absent() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let connection = Connection::open(file.path()).unwrap();
        connection
            .execute_batch(
                "CREATE TABLE threads (
                    id TEXT PRIMARY KEY,
                    title TEXT
                );
                INSERT INTO threads (id, title) VALUES ('native-title-only', 'Title only');",
            )
            .unwrap();

        assert_eq!(
            lookup_label_candidate_at(file.path(), "native-title-only").unwrap(),
            Some(CodexLabelCandidate {
                text: "Title only".into(),
                field: CodexLabelField::Title,
            })
        );
    }

    #[test]
    fn reset_blocks_the_previous_native_identity_until_a_new_one_arrives() {
        let session_id = "native-label-reset-block-test";
        let old_native_id = "old-native-id";
        let new_native_id = "new-native-id";
        clear_native_label_refresh_block(session_id);

        block_native_label_refresh(session_id, old_native_id);
        assert!(native_label_refresh_is_blocked(session_id, old_native_id));
        assert!(!accept_native_label_identity(session_id, old_native_id));
        assert!(accept_native_label_identity(session_id, new_native_id));
        assert!(!native_label_refresh_is_blocked(session_id, old_native_id));
        clear_native_label_refresh_block(session_id);
    }

    #[test]
    fn reset_authorization_requires_the_recorded_native_identity() {
        let session_id = "native-label-reset-process-test";
        clear_native_label_refresh_block(session_id);
        block_native_label_refresh(session_id, "old-native-id");

        assert!(native_identity_reset_is_authorized(
            session_id,
            "old-native-id"
        ));
        assert!(!native_identity_reset_is_authorized(
            session_id,
            "different-old-id"
        ));
        assert!(!native_identity_reset_is_authorized(
            "other-session",
            "old-native-id"
        ));

        clear_native_label_refresh_block(session_id);
    }
}
