use crate::http::session_handlers::{
    report_harness_session_from_local_relay, HarnessSessionReportRequest,
};
use crate::runtime::terminal_runtime::verify_workflow_report_token;
use crate::AppState;
use axum::http::StatusCode;
use serde::Deserialize;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncReadExt;

const MAX_REPORT_BYTES: u64 = 4 * 1024;
const MAX_REPORTS_PER_PASS: usize = 32;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CodexHookReportEnvelope {
    report: HarnessSessionReportRequest,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct RelayOutcome {
    pub(crate) reports_consumed: usize,
    pub(crate) reports_accepted: usize,
}

/// A private temporary directory whose path is passed only to one Codex
/// runtime. Its lifetime is tied to the PTY driver's lifetime.
pub(crate) struct CodexHookReportRelay {
    directory: tempfile::TempDir,
}

impl CodexHookReportRelay {
    pub(crate) fn new() -> io::Result<Self> {
        tempfile::Builder::new()
            .prefix("orkworks-codex-hook-reports-")
            .tempdir()
            .map(|directory| Self { directory })
    }

    pub(crate) fn mailbox_path(&self) -> &Path {
        self.directory.path()
    }

    pub(crate) async fn consume_ready(
        &self,
        state: Arc<AppState>,
        session_id: &str,
        report_token: &str,
        runtime_generation: u64,
    ) -> RelayOutcome {
        if !verify_workflow_report_token(session_id, report_token) {
            return RelayOutcome::default();
        }
        let owns_live_codex_runtime =
            state
                .sessions
                .lock()
                .unwrap()
                .get(session_id)
                .is_some_and(|handle| {
                    handle.runtime.run_generation() == runtime_generation
                        && handle.info.lifecycle == "alive"
                        && handle.info.lifecycle_phase == "active"
                        && (handle.info.harness.as_deref() == Some("codex")
                            || handle.info.harness_id.as_deref() == Some("codex"))
                });
        if !owns_live_codex_runtime {
            return RelayOutcome::default();
        }

        let mut outcome = RelayOutcome::default();
        let directory_metadata = match tokio::fs::symlink_metadata(self.mailbox_path()).await {
            Ok(metadata) if metadata.file_type().is_dir() => metadata,
            _ => return outcome,
        };
        drop(directory_metadata);

        let mut entries = match tokio::fs::read_dir(self.mailbox_path()).await {
            Ok(entries) => entries,
            Err(_) => return outcome,
        };
        let mut reports = Vec::with_capacity(MAX_REPORTS_PER_PASS);
        let mut inspected = 0;
        while inspected < MAX_REPORTS_PER_PASS {
            let entry = match entries.next_entry().await {
                Ok(Some(entry)) => entry,
                Ok(None) | Err(_) => break,
            };
            inspected += 1;
            if entry
                .file_type()
                .await
                .is_ok_and(|file_type| file_type.is_file())
                && is_report_filename(&entry.path())
            {
                reports.push(entry.path());
            }
        }
        reports.sort();

        for path in reports {
            outcome.reports_consumed += 1;
            if let Some(report) = read_report(&path).await {
                let status = report_harness_session_from_local_relay(
                    state.clone(),
                    session_id.to_string(),
                    report,
                    report_token,
                )
                .await;
                if status == StatusCode::OK {
                    outcome.reports_accepted += 1;
                }
            }
            let _ = tokio::fs::remove_file(path).await;
        }

        outcome
    }
}

fn is_report_filename(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    let Some(stem) = name.strip_suffix(".json") else {
        return false;
    };
    uuid::Uuid::parse_str(stem).is_ok_and(|id| id.simple().to_string() == stem)
}

async fn read_report(path: &PathBuf) -> Option<HarnessSessionReportRequest> {
    let metadata = tokio::fs::symlink_metadata(path).await.ok()?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_REPORT_BYTES {
        return None;
    }

    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    tokio::fs::File::open(path)
        .await
        .ok()?
        .take(MAX_REPORT_BYTES + 1)
        .read_to_end(&mut bytes)
        .await
        .ok()?;
    if bytes.len() as u64 > MAX_REPORT_BYTES {
        return None;
    }
    let report = serde_json::from_slice::<CodexHookReportEnvelope>(&bytes)
        .ok()?
        .report;
    if report.source != "codex_hook"
        || !report
            .hook_fingerprint
            .as_deref()
            .is_some_and(crate::metadata::valid_hook_fingerprint)
    {
        return None;
    }
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::is_report_filename;
    use std::path::Path;

    #[test]
    fn only_uuid_json_names_are_consumable() {
        assert!(is_report_filename(Path::new(
            "00112233445566778899aabbccddeeff.json"
        )));
        assert!(!is_report_filename(Path::new(".pending-001122.json")));
        assert!(!is_report_filename(Path::new("not-a-uuid.json")));
        assert!(!is_report_filename(Path::new(
            "00112233-4455-6677-8899-aabbccddeeff.json"
        )));
    }
}
