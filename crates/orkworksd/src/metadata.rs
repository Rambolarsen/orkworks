use crate::harness::{ResumeMemory, ResumeState, ResumeStrategy};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::warn;

pub const TERMINAL_OUTPUT_MAX_LINES: usize = 10_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub label: String,
    pub workspace: String,
    pub task: String,
    #[serde(rename = "harnessId", alias = "harness", default)]
    pub harness: String,
    #[serde(rename = "modelId", alias = "model", default)]
    pub model: String,
    pub cwd: String,
    pub status: String,
    pub phase: String,
    #[serde(rename = "observedStatus")]
    pub observed_status: Option<String>,
    pub summary: Option<String>,
    #[serde(rename = "nextAction")]
    pub next_action: Option<String>,
    #[serde(rename = "needsUserInput")]
    pub needs_user_input: Option<bool>,
    #[serde(rename = "detectedQuestion")]
    pub detected_question: Option<String>,
    #[serde(rename = "suggestedOptions")]
    pub suggested_options: Option<Vec<String>>,
    #[serde(rename = "blockerDescription")]
    pub blocker_description: Option<String>,
    #[serde(rename = "failedCommand")]
    pub failed_command: Option<String>,
    #[serde(rename = "failedTest")]
    pub failed_test: Option<String>,
    #[serde(rename = "capacityHints")]
    pub capacity_hints: Option<Vec<String>>,
    #[serde(rename = "peonLastInference")]
    pub peon_last_inference: Option<String>,
    #[serde(rename = "modelProviderId", alias = "providerId", skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(rename = "modelProviderLabel", alias = "providerLabel", skip_serializing_if = "Option::is_none")]
    pub provider_label: Option<String>,
    #[serde(rename = "providerModel", skip_serializing_if = "Option::is_none")]
    pub provider_model: Option<String>,
    #[serde(rename = "providerState", skip_serializing_if = "Option::is_none")]
    pub provider_state: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "lastActivity")]
    pub last_activity: String,
    #[serde(rename = "metadataSource")]
    pub metadata_source: String,
    #[serde(rename = "metadataConfidence")]
    pub metadata_confidence: f64,
    #[serde(rename = "repoRoot")]
    pub repo_root: Option<String>,
    pub branch: Option<String>,
    pub dirty: Option<bool>,
    #[serde(rename = "changedFiles")]
    pub changed_files: Option<usize>,
    #[serde(rename = "isWorktree")]
    pub is_worktree: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resume: Option<ResumeMemory>,
    #[serde(rename = "resumedFrom", skip_serializing_if = "Option::is_none")]
    pub resumed_from: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: String,
    pub status: String,
    #[serde(rename = "observedStatus")]
    pub observed_status: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkspaceMemory {
    #[serde(rename = "lastActiveSessionId", skip_serializing_if = "Option::is_none")]
    pub last_active_session_id: Option<String>,
    #[serde(rename = "lastActiveAt", skip_serializing_if = "Option::is_none")]
    pub last_active_at: Option<String>,
    #[serde(rename = "activeHarnessIds", default, skip_serializing_if = "Vec::is_empty")]
    pub active_harness_ids: Vec<String>,
}

pub struct MetadataStore {
    root: PathBuf,
}

impl MetadataStore {
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
        }
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    pub fn events_dir(&self) -> PathBuf {
        self.root.join("events")
    }

    pub fn workspace_memory_path(&self) -> PathBuf {
        self.root.join("workspace.json")
    }

    pub fn read_workspace_memory(&self) -> Option<WorkspaceMemory> {
        let data = fs::read_to_string(self.workspace_memory_path()).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn write_workspace_memory(&self, memory: &WorkspaceMemory) {
        if let Err(e) = fs::create_dir_all(&self.root) {
            warn!("failed to create metadata root {:?}: {e}", self.root);
            return;
        }
        let path = self.workspace_memory_path();
        match serde_json::to_string_pretty(memory) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    warn!("failed to write workspace memory {:?}: {e}", path);
                }
            }
            Err(e) => warn!("failed to serialize workspace memory: {e}"),
        }
    }

    pub fn read_all_sessions(&self) -> Vec<SessionMetadata> {
        let dir = self.sessions_dir();
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(_) => return vec![],
        };
        let mut sessions: Vec<SessionMetadata> = entries
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("json"))
            .filter_map(|entry| fs::read_to_string(entry.path()).ok())
            .filter_map(|data| serde_json::from_str::<SessionMetadata>(&data).ok())
            .collect();
        sessions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        sessions
    }

    pub fn write_session(&self, meta: &SessionMetadata) {
        let dir = self.sessions_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            warn!("failed to create sessions dir {:?}: {e}", dir);
            return;
        }
        let path = dir.join(format!("{}.json", meta.id));
        match serde_json::to_string_pretty(meta) {
            Ok(json) => {
                if let Err(e) = fs::write(&path, json) {
                    warn!("failed to write session {}: {e}", meta.id);
                }
            }
            Err(e) => warn!("failed to serialize session {}: {e}", meta.id),
        }
    }

    pub fn delete_session(&self, id: &str) -> std::io::Result<()> {
        let path = self.sessions_dir().join(format!("{}.json", id));
        match fs::remove_file(&path) {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }

    pub fn delete_events(&self, id: &str) -> std::io::Result<()> {
        let ndjson_path = self.events_dir().join(format!("{}.ndjson", id));
        let terminal_path = self.terminal_output_path(id);

        if let Err(e) = fs::remove_file(&ndjson_path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e);
            }
        }
        if let Err(e) = fs::remove_file(&terminal_path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(e);
            }
        }
        Ok(())
    }

    pub fn clear_last_active_session_if_matches(&self, id: &str) -> std::io::Result<()> {
        let Some(mut memory) = self.read_workspace_memory() else {
            return Ok(());
        };
        if memory.last_active_session_id.as_deref() == Some(id) {
            memory.last_active_session_id = None;
            memory.last_active_at = None;
            self.write_workspace_memory(&memory);
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn read_events(&self, id: &str) -> Vec<Event> {
        let path = self.events_dir().join(format!("{}.ndjson", id));
        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return vec![],
        };
        data.lines()
            .filter_map(|line| serde_json::from_str::<Event>(line).ok())
            .collect()
    }

    pub fn read_session(&self, id: &str) -> Option<SessionMetadata> {
        let path = self.sessions_dir().join(format!("{}.json", id));
        let data = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn session_modified_secs_ago(&self, id: &str) -> Option<u64> {
        let path = self.sessions_dir().join(format!("{}.json", id));
        let modified = fs::metadata(path).ok()?.modified().ok()?;
        modified.elapsed().ok().map(|elapsed| elapsed.as_secs())
    }

    pub fn append_event(&self, id: &str, event: &Event) {
        let dir = self.events_dir();
        if let Err(e) = fs::create_dir_all(&dir) {
            warn!("failed to create events dir {:?}: {e}", dir);
            return;
        }
        let path = dir.join(format!("{}.ndjson", id));
        match serde_json::to_string(event) {
            Ok(json) => match fs::OpenOptions::new().create(true).append(true).open(&path) {
                Ok(mut file) => {
                    if let Err(e) = writeln!(file, "{json}") {
                        warn!("failed to write event to {id}: {e}");
                    }
                }
                Err(e) => warn!("failed to open event file for {id}: {e}"),
            },
            Err(e) => warn!("failed to serialize event for {id}: {e}"),
        }
    }

    pub fn persist_provider_context(
        &self,
        id: &str,
        provider: &crate::providers::ProviderObservation,
    ) {
        let mut meta = match self.read_session(id) {
            Some(m) => m,
            None => return,
        };
        meta.provider_id = Some(provider.provider_id.clone());
        meta.provider_label = Some(provider.provider_label.clone());
        meta.provider_model = provider.provider_model.clone();
        meta.provider_state = Some(provider.provider_state.clone());
        self.write_session(&meta);
    }

    pub fn merge_peon_inference(
        &self,
        id: &str,
        inf: &crate::peon::PeonInference,
        timestamp: &str,
        provider: Option<&crate::providers::ProviderObservation>,
    ) {
        let mut meta = match self.read_session(id) {
            Some(m) => m,
            None => return,
        };

        meta.observed_status = inf.observed_status.clone().or(meta.observed_status);
        if let Some(ref phase) = inf.phase {
            meta.phase = phase.clone();
        }
        meta.summary = inf.summary.clone().or(meta.summary);
        meta.next_action = inf.next_action.clone().or(meta.next_action);
        meta.needs_user_input = inf.needs_user_input.or(meta.needs_user_input);
        meta.detected_question = inf.detected_question.clone().or(meta.detected_question);
        meta.suggested_options = inf.suggested_options.clone().or(meta.suggested_options);
        meta.blocker_description = inf.blocker_description.clone().or(meta.blocker_description);
        meta.failed_command = inf.failed_command.clone().or(meta.failed_command);
        meta.failed_test = inf.failed_test.clone().or(meta.failed_test);
        meta.capacity_hints = inf.capacity_hints.clone().or(meta.capacity_hints);

        let had_harness = !meta.harness.is_empty();
        let had_model = !meta.model.is_empty();
        if let Some(ref h) = inf.detected_harness {
            if meta.harness.is_empty() {
                meta.harness = h.clone();
            }
        }
        if let Some(ref m) = inf.detected_model {
            if meta.model.is_empty() {
                meta.model = m.clone();
            }
        }
        let now_has_harness = !meta.harness.is_empty();
        let now_has_model = !meta.model.is_empty();
        if now_has_harness && (!had_harness || (!had_model && now_has_model)) {
            meta.label = if now_has_model {
                format!("{} ({})", meta.harness, meta.model)
            } else {
                meta.harness.clone()
            };
        }

        if let Some(ref sid) = inf.harness_session_id {
            if !sid.is_empty() && sid.len() >= 3 && !sid.contains(char::is_whitespace) {
                let mut resume = meta.resume.take().unwrap_or_else(|| ResumeMemory {
                    state: ResumeState::Available,
                    preferred_strategy: ResumeStrategy::None,
                    harness_session_id: None,
                    latest_fallback: true,
                    last_seen_at: None,
                });
                resume.harness_session_id = Some(sid.clone());
                resume.last_seen_at = Some(timestamp.to_string());
                if resume.preferred_strategy == ResumeStrategy::None {
                    resume.preferred_strategy = ResumeStrategy::Exact;
                }
                meta.resume = Some(resume);
            }
        }

        meta.peon_last_inference = Some(timestamp.to_string());
        meta.metadata_source = "peon".into();
        meta.metadata_confidence = inf.confidence;

        if let Some(p) = provider {
            meta.provider_id = Some(p.provider_id.clone());
            meta.provider_label = Some(p.provider_label.clone());
            meta.provider_model = p.provider_model.clone();
            meta.provider_state = Some(p.provider_state.clone());
        }

        self.write_session(&meta);

        self.append_event(id, &Event {
            event_type: "peon.inference".into(),
            timestamp: timestamp.to_string(),
            status: meta.status.clone(),
            observed_status: inf.observed_status.clone(),
            confidence: Some(inf.confidence),
        });
    }

    fn terminal_output_path(&self, id: &str) -> PathBuf {
        self.events_dir().join(format!("{}.terminal", id))
    }

    pub fn append_terminal_output_lines(&self, id: &str, lines: &[String]) {
        if lines.is_empty() {
            return;
        }
        if let Err(e) = fs::create_dir_all(&self.events_dir()) {
            warn!("failed to create events dir for terminal output: {e}");
            return;
        }
        let path = self.terminal_output_path(id);
        let mut file = match fs::OpenOptions::new().create(true).append(true).open(&path) {
            Ok(f) => f,
            Err(e) => {
                warn!("failed to open terminal output file for {id}: {e}");
                return;
            }
        };
        for line in lines {
            if let Err(e) = writeln!(file, "{line}") {
                warn!("failed to append terminal output for {id}: {e}");
                return;
            }
        }
        // Inline trim when the file exceeds 1.5x max_lines to prevent unbounded growth
        // during long-running sessions. Only check approximate size via metadata.
        let len_hint = file.metadata().map(|m| m.len()).unwrap_or(0);
        drop(file);
        // Rough estimate: 100 bytes per line, so 1.5x MAX_LINES ≈ 150 * MAX_LINES bytes
        if len_hint > (TERMINAL_OUTPUT_MAX_LINES as u64 * 150) {
            let _ = self.trim_terminal_output(id, TERMINAL_OUTPUT_MAX_LINES);
        }
    }

    pub fn read_terminal_output(&self, id: &str, max_lines: usize) -> Vec<String> {
        let path = self.terminal_output_path(id);
        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return Vec::new(),
        };
        let all: Vec<&str> = data.lines().collect();
        let start = if all.len() > max_lines { all.len() - max_lines } else { 0 };
        all[start..].iter().map(|s| s.to_string()).collect()
    }

    pub fn delete_terminal_output(&self, id: &str) {
        let path = self.terminal_output_path(id);
        if let Err(e) = fs::remove_file(&path) {
            if e.kind() != std::io::ErrorKind::NotFound {
                warn!("failed to delete terminal output for {id}: {e}");
            }
        }
    }

    pub fn trim_terminal_output(&self, id: &str, max_lines: usize) {
        let path = self.terminal_output_path(id);
        let data = match fs::read_to_string(&path) {
            Ok(d) => d,
            Err(_) => return,
        };
        let all: Vec<&str> = data.lines().collect();
        if all.len() <= max_lines {
            return;
        }
        let start = all.len() - max_lines;
        match fs::write(&path, all[start..].join("\n") + "\n") {
            Ok(_) => {}
            Err(e) => warn!("failed to trim terminal output for {id}: {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_and_read_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = SessionMetadata {
            id: "test-1".into(),
            label: "Test".into(),
            workspace: "/tmp".into(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: "/tmp".into(),
            status: "running".into(),
            phase: "implementation".into(),
            observed_status: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: Some("/tmp".into()),
            branch: Some("main".into()),
            dirty: Some(false),
            changed_files: Some(0),
            is_worktree: Some(false),
            resume: None,
            resumed_from: None,
        };
        store.write_session(&meta);
        let read = store.read_session("test-1").unwrap();
        assert_eq!(read.status, "running");
    }

    #[test]
    fn append_and_read_events() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.append_event("test-2", &Event {
            event_type: "session.created".into(),
            timestamp: "now".into(),
            status: "creating".into(),
            observed_status: None,
            confidence: None,
        });
        store.append_event("test-2", &Event {
            event_type: "session.status".into(),
            timestamp: "later".into(),
            status: "running".into(),
            observed_status: None,
            confidence: None,
        });
        let path = store.events_dir().join("test-2.ndjson");
        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(contents.lines().count(), 2);
    }

    #[test]
    fn read_events_returns_deserialized_events() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.append_event("test-3", &Event {
            event_type: "session.created".into(),
            timestamp: "t1".into(),
            status: "creating".into(),
            observed_status: None,
            confidence: None,
        });
        store.append_event("test-3", &Event {
            event_type: "session.status".into(),
            timestamp: "t2".into(),
            status: "running".into(),
            observed_status: None,
            confidence: None,
        });
        let events = store.read_events("test-3");
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].event_type, "session.created");
        assert_eq!(events[1].status, "running");
    }

    #[test]
    fn read_events_returns_empty_for_missing_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let events = store.read_events("nonexistent");
        assert!(events.is_empty());
    }

    #[test]
    fn merge_peon_inference_renames_session_when_harness_detected() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&SessionMetadata {
            id: "rename-test".into(),
            label: "Session abc12345".into(),
            workspace: "/tmp".into(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: "/tmp".into(),
            status: "running".into(),
            phase: "".into(),
            observed_status: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            resume: None,
            resumed_from: None,
        });

        // First inference: harness detected, no model
        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None, summary: None, next_action: None,
            needs_user_input: None, detected_question: None, suggested_options: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.8,
            detected_harness: Some("claude-code".into()),
            detected_model: None,
            harness_session_id: None,
        };
        store.merge_peon_inference("rename-test", &inf, "t1", None);
        let meta = store.read_session("rename-test").unwrap();
        assert_eq!(meta.label, "claude-code");
        assert_eq!(meta.harness, "claude-code");
        assert_eq!(meta.model, "");

        // Second inference: model also detected, label updates to include it
        let inf2 = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None, summary: None, next_action: None,
            needs_user_input: None, detected_question: None, suggested_options: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.9,
            detected_harness: Some("claude-code".into()),
            detected_model: Some("claude-sonnet-4-5".into()),
            harness_session_id: None,
        };
        store.merge_peon_inference("rename-test", &inf2, "t2", None);
        let meta2 = store.read_session("rename-test").unwrap();
        assert_eq!(meta2.label, "claude-code (claude-sonnet-4-5)");
        assert_eq!(meta2.harness, "claude-code");
        assert_eq!(meta2.model, "claude-sonnet-4-5");

        // Third inference: same harness/model again — label should NOT be re-overwritten
        store.merge_peon_inference("rename-test", &inf2, "t3", None);
        let meta3 = store.read_session("rename-test").unwrap();
        assert_eq!(meta3.label, "claude-code (claude-sonnet-4-5)");
    }

    #[test]
    fn merge_peon_inference_preserves_lifecycle_status_and_writes_observer_status() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&SessionMetadata {
            id: "test-peon-observer".into(),
            label: "Test".into(),
            workspace: "/tmp".into(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: "/tmp".into(),
            status: "running".into(),
            phase: "".into(),
            observed_status: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            resume: None,
            resumed_from: None,
        });

        let inf = crate::peon::PeonInference {
            observed_status: Some("waiting_for_input".into()),
            phase: Some("review".into()),
            summary: Some("Needs a decision".into()),
            next_action: Some("Pick an option".into()),
            needs_user_input: Some(true),
            detected_question: Some("Proceed?".into()),
            suggested_options: Some(vec!["yes".into(), "no".into()]),
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 0.82,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
        };

        store.merge_peon_inference("test-peon-observer", &inf, "later", None);

        let meta = store.read_session("test-peon-observer").unwrap();
        assert_eq!(meta.status, "running");

        let path = store.sessions_dir().join("test-peon-observer.json");
        let raw: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
        assert_eq!(raw["observedStatus"], "waiting_for_input");
        assert_eq!(raw["summary"], "Needs a decision");
        assert_eq!(raw["needsUserInput"], true);
        assert_eq!(raw["peonLastInference"], "later");
    }

    fn test_metadata(id: &str) -> SessionMetadata {
        SessionMetadata {
            id: id.into(),
            label: "Test".into(),
            workspace: "/tmp".into(),
            task: "".into(),
            harness: "".into(),
            model: "".into(),
            cwd: "/tmp".into(),
            status: "running".into(),
            phase: "".into(),
            observed_status: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            peon_last_inference: None,
            provider_id: None,
            provider_label: None,
            provider_model: None,
            provider_state: None,
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: None,
            branch: None,
            dirty: None,
            changed_files: None,
            is_worktree: None,
            resume: None,
            resumed_from: None,
        }
    }

    #[test]
    fn write_and_read_workspace_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());

        store.write_workspace_memory(&WorkspaceMemory {
            last_active_session_id: Some("session-1".into()),
            last_active_at: Some("2026-06-17T12:00:00Z".into()),
            active_harness_ids: vec![],
        });

        let memory = store.read_workspace_memory().unwrap();
        assert_eq!(memory.last_active_session_id.as_deref(), Some("session-1"));
        assert_eq!(memory.last_active_at.as_deref(), Some("2026-06-17T12:00:00Z"));
    }

    #[test]
    fn read_all_sessions_includes_resume_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let mut meta = test_metadata("remembered");
        meta.resume = Some(crate::harness::ResumeMemory {
            state: crate::harness::ResumeState::Available,
            preferred_strategy: crate::harness::ResumeStrategy::Exact,
            harness_session_id: Some("sess-abc".into()),
            latest_fallback: true,
            last_seen_at: Some("2026-06-17T12:00:00Z".into()),
        });
        store.write_session(&meta);

        let all = store.read_all_sessions();

        assert_eq!(all.len(), 1);
        assert_eq!(
            all[0].resume.as_ref().and_then(|r| r.harness_session_id.as_deref()),
            Some("sess-abc"),
        );
    }

    #[test]
    fn peon_inference_writes_harness_session_id_to_resume_memory() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = test_metadata("session-id-test");
        store.write_session(&meta);

        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None, summary: None, next_action: None,
            needs_user_input: None, detected_question: None, suggested_options: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.9,
            detected_harness: Some("claude-code".into()),
            detected_model: Some("claude-sonnet-4-5".into()),
            harness_session_id: Some("sess-abc123".into()),
        };
        store.merge_peon_inference("session-id-test", &inf, "2026-06-20T12:00:00Z", None);

        let updated = store.read_session("session-id-test").unwrap();
        let resume = updated.resume.unwrap();
        assert_eq!(resume.state, ResumeState::Available);
        assert_eq!(resume.preferred_strategy, ResumeStrategy::Exact);
        assert_eq!(resume.harness_session_id.as_deref(), Some("sess-abc123"));
        assert_eq!(resume.last_seen_at.as_deref(), Some("2026-06-20T12:00:00Z"));
        assert!(resume.latest_fallback);
    }

    #[test]
    fn peon_inference_ignores_empty_harness_session_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = test_metadata("empty-sid-test");
        store.write_session(&meta);

        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None, summary: None, next_action: None,
            needs_user_input: None, detected_question: None, suggested_options: None,
            blocker_description: None, failed_command: None, failed_test: None,
            capacity_hints: None, confidence: 0.9,
            detected_harness: None,
            detected_model: None,
            harness_session_id: Some("".into()),
        };
        store.merge_peon_inference("empty-sid-test", &inf, "2026-06-20T12:00:00Z", None);

        let updated = store.read_session("empty-sid-test").unwrap();
        assert!(updated.resume.is_none());
    }

    #[test]
    fn peon_inference_rejects_invalid_harness_session_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());

        // Too short
        {
            let meta = test_metadata("short-sid");
            store.write_session(&meta);
            let inf = crate::peon::PeonInference {
                observed_status: Some("working".into()),
                phase: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None, confidence: 0.9,
                detected_harness: None,
                detected_model: None,
                harness_session_id: Some("ab".into()),
            };
            store.merge_peon_inference("short-sid", &inf, "2026-06-20T12:00:00Z", None);
            assert!(store.read_session("short-sid").unwrap().resume.is_none());
        }

        // Contains whitespace
        {
            let meta = test_metadata("whitespace-sid");
            store.write_session(&meta);
            let inf = crate::peon::PeonInference {
                observed_status: Some("working".into()),
                phase: None, summary: None, next_action: None,
                needs_user_input: None, detected_question: None, suggested_options: None,
                blocker_description: None, failed_command: None, failed_test: None,
                capacity_hints: None, confidence: 0.9,
                detected_harness: None,
                detected_model: None,
                harness_session_id: Some("not an id".into()),
            };
            store.merge_peon_inference("whitespace-sid", &inf, "2026-06-20T12:00:00Z", None);
            assert!(store.read_session("whitespace-sid").unwrap().resume.is_none());
        }
    }

    #[test]
    fn terminal_output_round_trip_and_trim() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());

        let lines: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        store.append_terminal_output_lines("test-session", &lines);

        let read = store.read_terminal_output("test-session", 50);
        assert_eq!(read.len(), 50);
        assert_eq!(read[0], "line 50");
        assert_eq!(read[49], "line 99");

        // Write more lines, trigger inline trim
        let more: Vec<String> = (100..200).map(|i| format!("line {}", i)).collect();
        store.append_terminal_output_lines("test-session", &more);

        // trim to 50
        store.trim_terminal_output("test-session", 50);
        let after_trim = store.read_terminal_output("test-session", 100);
        assert_eq!(after_trim.len(), 50);
        assert_eq!(after_trim[0], "line 150");
        assert_eq!(after_trim[49], "line 199");

        // Delete and verify
        store.delete_terminal_output("test-session");
        let after_delete = store.read_terminal_output("test-session", 100);
        assert!(after_delete.is_empty());
    }

    #[test]
    fn terminal_output_empty_session_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let lines = store.read_terminal_output("nonexistent", 50);
        assert!(lines.is_empty());
    }

    #[test]
    fn delete_session_removes_json_file() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let meta = test_metadata("delete-me");
        store.write_session(&meta);
        assert!(store.read_session("delete-me").is_some());

        store.delete_session("delete-me").unwrap();
        assert!(store.read_session("delete-me").is_none());
    }

    #[test]
    fn delete_session_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        // Should not error if file doesn't exist
        assert!(store.delete_session("nonexistent").is_ok());
    }

    #[test]
    fn delete_events_removes_ndjson_and_terminal() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.append_event("del-test", &Event {
            event_type: "session.created".into(),
            timestamp: "t1".into(),
            status: "creating".into(),
            observed_status: None,
            confidence: None,
        });
        store.append_terminal_output_lines("del-test", &["line 1".into(), "line 2".into()]);

        let ndjson_path = store.events_dir().join("del-test.ndjson");
        let terminal_path = store.events_dir().join("del-test.terminal");
        assert!(ndjson_path.exists());
        assert!(terminal_path.exists());

        store.delete_events("del-test").unwrap();

        assert!(!ndjson_path.exists());
        assert!(!terminal_path.exists());
    }

    #[test]
    fn delete_events_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        assert!(store.delete_events("nonexistent").is_ok());
    }

    #[test]
    fn merge_peon_inference_persists_provider_context() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        store.write_session(&test_metadata("provider-context"));

        let inf = crate::peon::PeonInference {
            observed_status: Some("working".into()),
            phase: None,
            summary: Some("still working".into()),
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 0.9,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
        };

        let provider = crate::providers::ProviderObservation {
            provider_id: "claude-code".into(),
            provider_label: "Claude Code".into(),
            provider_model: Some("sonnet".into()),
            provider_state: "healthy".into(),
        };

        store.merge_peon_inference("provider-context", &inf, "later", Some(&provider));

        let meta = store.read_session("provider-context").unwrap();
        assert_eq!(meta.provider_id.as_deref(), Some("claude-code"));
        assert_eq!(meta.provider_label.as_deref(), Some("Claude Code"));
        assert_eq!(meta.provider_model.as_deref(), Some("sonnet"));
        assert_eq!(meta.provider_state.as_deref(), Some("healthy"));
    }

    #[test]
    fn read_session_accepts_canonical_terminology_fields() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        std::fs::create_dir_all(store.sessions_dir()).unwrap();

        let raw = serde_json::json!({
            "id": "canonical-fields",
            "label": "Canonical Fields",
            "workspace": "/tmp",
            "task": "",
            "harnessId": "opencode",
            "modelId": "deepseek/deepseek-reasoner",
            "cwd": "/tmp",
            "status": "running",
            "phase": "",
            "modelProviderId": "openrouter",
            "createdAt": "now",
            "lastActivity": "now",
            "metadataSource": "process",
            "metadataConfidence": 1.0
        });

        std::fs::write(
            store.sessions_dir().join("canonical-fields.json"),
            serde_json::to_string_pretty(&raw).unwrap(),
        ).unwrap();

        let meta = store.read_session("canonical-fields").unwrap();
        assert_eq!(meta.harness, "opencode");
        assert_eq!(meta.model, "deepseek/deepseek-reasoner");
        assert_eq!(meta.provider_id.as_deref(), Some("openrouter"));
    }
}
