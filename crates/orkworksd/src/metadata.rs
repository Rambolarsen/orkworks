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
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Event {
    #[serde(rename = "type")]
    pub event_type: String,
    pub timestamp: String,
    pub status: String,
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

        if let Some(ref status) = inf.status {
            meta.status = status.clone();
        }
        if let Some(ref phase) = inf.phase {
            meta.phase = phase.clone();
        }
        meta.metadata_source = "peon".into();
        meta.metadata_confidence = inf.confidence;
        meta.last_activity = timestamp.to_string();

        self.write_session(&meta);

        self.append_event(id, &Event {
            event_type: "peon.inference".into(),
            timestamp: timestamp.to_string(),
            status: inf.status.clone().unwrap_or_else(|| meta.status.clone()),
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
            created_at: "now".into(),
            last_activity: "now".into(),
            metadata_source: "process".into(),
            metadata_confidence: 1.0,
            repo_root: Some("/tmp".into()),
            branch: Some("main".into()),
            dirty: Some(false),
            changed_files: Some(0),
            is_worktree: Some(false),
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
        });
        store.append_event("test-2", &Event {
            event_type: "session.status".into(),
            timestamp: "later".into(),
            status: "running".into(),
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
        });
        store.append_event("test-3", &Event {
            event_type: "session.status".into(),
            timestamp: "t2".into(),
            status: "running".into(),
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
}
