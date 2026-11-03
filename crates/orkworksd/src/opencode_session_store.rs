//! Read-only native availability for OpenCode exact resume; never creates a store.
use crate::harness::CommandSpec;
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::io::AsyncReadExt;

const UNVERIFIABLE: &str = "Cannot verify this exact OpenCode conversation. Check the local OpenCode installation and saved session; no replacement was started.";
const MISSING: &str =
    "OpenCode has no saved local conversation for this exact ID; no replacement was started.";

pub(crate) async fn verify_exact(command: &CommandSpec, id: &str) -> Result<(), &'static str> {
    // The native path command cannot attest remote/custom resume semantics.
    if command.args != ["--standalone", "--session", id] {
        return Err(UNVERIFIABLE);
    }
    let path = selected_path(command).await?;
    let id = id.to_owned();
    match tokio::task::spawn_blocking(move || saved_session_at(&path, &id)).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => Err(MISSING),
        _ => Err(UNVERIFIABLE),
    }
}

async fn selected_path(command: &CommandSpec) -> Result<PathBuf, &'static str> {
    let tool =
        crate::harness::detect::probe_installed_tool(&command.program).ok_or(UNVERIFIABLE)?;
    let mut probe = tokio::process::Command::new(tool.executable);
    // Keep native path selection (OPENCODE_DB/XDG) but strip session authority.
    probe
        .env_clear()
        .envs(std::env::vars_os().filter(|(key, _)| {
            key.to_str()
                .map(|key| {
                    let key = key.to_ascii_uppercase();
                    crate::runtime::terminal_runtime::should_forward_terminal_env(&key)
                        && !key.starts_with("ORKWORKS_")
                })
                .unwrap_or(true)
        }));
    probe
        .args(["debug", "paths", "db"])
        .current_dir(&command.cwd)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    let read = async {
        let mut child = probe.spawn().map_err(|_| UNVERIFIABLE)?;
        let mut out = Vec::new();
        let mut err = Vec::new();
        let mut stdout = child.stdout.take().ok_or(UNVERIFIABLE)?.take(8193);
        let mut stderr = child.stderr.take().ok_or(UNVERIFIABLE)?.take(8193);
        let (a, b) = tokio::join!(stdout.read_to_end(&mut out), stderr.read_to_end(&mut err));
        a.map_err(|_| UNVERIFIABLE)?;
        b.map_err(|_| UNVERIFIABLE)?;
        if out.len() > 8192 || err.len() > 8192 {
            return Err(UNVERIFIABLE);
        }
        if !child.wait().await.map_err(|_| UNVERIFIABLE)?.success() {
            return Err(UNVERIFIABLE);
        }
        let path = std::str::from_utf8(&out).map_err(|_| UNVERIFIABLE)?.trim();
        if path.chars().any(char::is_control) || !Path::new(path).is_absolute() {
            return Err(UNVERIFIABLE);
        }
        Ok(PathBuf::from(path))
    };
    tokio::time::timeout(Duration::from_secs(3), read)
        .await
        .map_err(|_| UNVERIFIABLE)?
}

fn saved_session_at(path: &Path, id: &str) -> Result<bool, ()> {
    let db = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|_| ())?;
    db.busy_timeout(Duration::from_millis(100))
        .map_err(|_| ())?;
    let table: Option<String> = db
        .query_row(
            "SELECT type FROM sqlite_master WHERE name = 'session_v2'",
            [],
            |r| r.get(0),
        )
        .optional()
        .map_err(|_| ())?;
    if table.as_deref() != Some("table") {
        return Err(());
    }
    let mut columns = db
        .prepare("PRAGMA table_info(session_v2)")
        .map_err(|_| ())?;
    let mut rows = columns.query([]).map_err(|_| ())?;
    let mut indexed_id = false;
    while let Some(row) = rows.next().map_err(|_| ())? {
        indexed_id |= row.get::<_, String>(1).map_err(|_| ())? == "id"
            && row
                .get::<_, String>(2)
                .map_err(|_| ())?
                .eq_ignore_ascii_case("TEXT")
            && row.get::<_, u32>(5).map_err(|_| ())? == 1;
    }
    if !indexed_id {
        return Err(());
    }
    db.query_row(
        "SELECT EXISTS(SELECT 1 FROM session_v2 WHERE id = ?1)",
        [id],
        |r| r.get(0),
    )
    .map_err(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn exact_identity_requires_the_v2_native_record() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("opencode.db");
        let db = Connection::open(&path).unwrap();
        db.execute_batch("CREATE TABLE session_v2 (id TEXT PRIMARY KEY); CREATE TABLE session (id TEXT PRIMARY KEY); INSERT INTO session_v2 VALUES ('ses_saved'); INSERT INTO session VALUES ('ses_legacy');").unwrap();
        assert_eq!(saved_session_at(&path, "ses_saved"), Ok(true));
        assert_eq!(saved_session_at(&path, "ses_missing"), Ok(false));
        assert_eq!(saved_session_at(&path, "ses_legacy"), Ok(false));
        db.execute("DELETE FROM session_v2 WHERE id = 'ses_saved'", [])
            .unwrap();
        assert_eq!(saved_session_at(&path, "ses_saved"), Ok(false));
    }

    #[test]
    fn unavailable_or_unknown_store_is_not_created_or_claimed_missing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("absent.db");
        assert_eq!(saved_session_at(&path, "ses_saved"), Err(()));
        assert!(!path.exists());
        std::fs::write(&path, "corrupt database").unwrap();
        assert_eq!(saved_session_at(&path, "ses_saved"), Err(()));
        std::fs::remove_file(&path).unwrap();
        Connection::open(&path)
            .unwrap()
            .execute_batch("CREATE TABLE session (id TEXT PRIMARY KEY)")
            .unwrap();
        assert_eq!(saved_session_at(&path, "ses_saved"), Err(()));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn exact_probe_checks_the_selected_store_without_launching_a_session() {
        let dir = tempfile::tempdir().unwrap();
        let executable = dir.path().join("opencode-fixture");
        let path = dir.path().join("opencode-fixture.db");
        Connection::open(&path).unwrap().execute_batch("CREATE TABLE session_v2 (id TEXT PRIMARY KEY); INSERT INTO session_v2 VALUES ('ses_saved')").unwrap();
        std::fs::write(&executable, r#"#!/bin/sh
[ "$1" = debug ] && [ "$2" = paths ] && [ "$3" = db ] || exit 80
for key in ORKWORKS_SESSION_ID ORKWORKS_PORT ORKWORKS_REPORT_TOKEN ORKWORKS_PROMPT_HOOK_GENERATION; do
  if printenv "$key" >/dev/null; then exit 81; fi
done
printf '%s\n' "$0.db"
"#).unwrap();
        crate::test_support::make_test_executable(&executable);
        let mut command = CommandSpec {
            program: executable.display().to_string(),
            cwd: dir.path().display().to_string(),
            args: vec![
                "--standalone".into(),
                "--session".into(),
                "ses_saved".into(),
            ],
        };
        assert_eq!(verify_exact(&command, "ses_saved").await, Ok(()));
        command.args[2] = "ses_missing".into();
        assert_eq!(verify_exact(&command, "ses_missing").await, Err(MISSING));
        command.args.push("--server=remote".into());
        assert_eq!(
            verify_exact(&command, "ses_missing").await,
            Err(UNVERIFIABLE)
        );
        std::fs::write(&executable, "#!/bin/sh\nprintf ':memory:\\n'\n").unwrap();
        command.args.pop();
        assert_eq!(
            verify_exact(&command, "ses_missing").await,
            Err(UNVERIFIABLE)
        );
    }

    #[test]
    fn wal_availability_preserves_native_conversation_data() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("native.db");
        let writer = Connection::open(&path).unwrap();
        writer.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE session_v2 (id TEXT PRIMARY KEY); INSERT INTO session_v2 VALUES ('ses_wal');").unwrap();
        let before = std::fs::read(&path).unwrap();
        assert_eq!(saved_session_at(&path, "ses_wal"), Ok(true));
        assert_eq!(before, std::fs::read(&path).unwrap());
        let count: u32 = writer
            .query_row("SELECT count(*) FROM session_v2", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
