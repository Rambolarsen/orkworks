use crate::harness::ResumeMemory;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::warn;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    pub id: String,
    pub label: String,
    pub workspace: String,
    pub task: String,
    pub harness: String,
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

    pub fn merge_peon_inference(&self, id: &str, inf: &crate::peon::PeonInference, timestamp: &str) {
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

        meta.peon_last_inference = Some(timestamp.to_string());
        meta.metadata_source = "peon".into();
        meta.metadata_confidence = inf.confidence;

        self.write_session(&meta);

        self.append_event(id, &Event {
            event_type: "peon.inference".into(),
            timestamp: timestamp.to_string(),
            status: meta.status.clone(),
            observed_status: inf.observed_status.clone(),
            confidence: Some(inf.confidence),
        });
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
        };
        store.merge_peon_inference("rename-test", &inf, "t1");
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
        };
        store.merge_peon_inference("rename-test", &inf2, "t2");
        let meta2 = store.read_session("rename-test").unwrap();
        assert_eq!(meta2.label, "claude-code (claude-sonnet-4-5)");
        assert_eq!(meta2.harness, "claude-code");
        assert_eq!(meta2.model, "claude-sonnet-4-5");

        // Third inference: same harness/model again — label should NOT be re-overwritten
        store.merge_peon_inference("rename-test", &inf2, "t3");
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
        };

        store.merge_peon_inference("test-peon-observer", &inf, "later");

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
}
