use rusqlite::{Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::Duration;

static LABEL_REFRESH_GENERATIONS: LazyLock<Mutex<std::collections::HashMap<String, u64>>> =
    LazyLock::new(|| Mutex::new(std::collections::HashMap::new()));

pub(crate) fn reserve_label_refresh_generation(session_id: &str) -> u64 {
    let mut generations = LABEL_REFRESH_GENERATIONS.lock().unwrap();
    let generation = generations.entry(session_id.to_owned()).or_insert(0);
    *generation = generation.saturating_add(1);
    *generation
}

pub(crate) fn invalidate_label_refresh_generation(session_id: &str) {
    let _ = reserve_label_refresh_generation(session_id);
}

pub(crate) fn clear_label_refresh_generation(session_id: &str) {
    LABEL_REFRESH_GENERATIONS.lock().unwrap().remove(session_id);
}

pub(crate) fn hold_label_refresh_generation(
    session_id: &str,
    expected_generation: u64,
) -> Option<MutexGuard<'static, std::collections::HashMap<String, u64>>> {
    let generations = LABEL_REFRESH_GENERATIONS.lock().unwrap();
    (generations.get(session_id).copied().unwrap_or(0) == expected_generation)
        .then_some(generations)
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

fn lookup_label_candidate_at(
    path: &Path,
    native_session_id: &str,
) -> Result<Option<CodexLabelCandidate>, CodexStoreError> {
    let connection = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|_| CodexStoreError::Unavailable)?;
    connection
        .busy_timeout(Duration::from_millis(100))
        .map_err(|_| CodexStoreError::Unavailable)?;

    let row = connection
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
        .map_err(|_| CodexStoreError::Unavailable)?;

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
}
