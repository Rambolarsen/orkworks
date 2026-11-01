//! Strict, field-scoped agent mutation of the canonical session record.

use super::session_transactions::{
    MetadataRevision, RevisionedSession, SessionTransactionError, WriteScope,
};
use super::{
    canonical_attention, source_priority, MetadataStore, PlanReference, PlanSource, SessionMetadata,
};
use crate::plan_handoff::resolve_printed_plan_path;
use crate::workspace_runtime::iso_now;
use serde::{Deserialize, Deserializer};
use std::io;
use std::path::Path;

#[derive(Debug)]
enum PatchSemanticError {
    Conflict,
    BadRequest,
    Unauthorized,
}

impl std::fmt::Display for PatchSemanticError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict => formatter.write_str("agent metadata conflict"),
            Self::BadRequest => formatter.write_str("invalid agent metadata"),
            Self::Unauthorized => formatter.write_str("session is not live"),
        }
    }
}

impl std::error::Error for PatchSemanticError {}

fn semantic_error(error: PatchSemanticError) -> io::Error {
    io::Error::other(error)
}

fn require_live(meta: &SessionMetadata) -> io::Result<()> {
    if meta.lifecycle == "alive" && meta.lifecycle_phase == "active" && meta.status == "running" {
        Ok(())
    } else {
        Err(semantic_error(PatchSemanticError::Unauthorized))
    }
}

#[derive(Debug, Deserialize)]
#[serde(
    rename_all = "camelCase",
    deny_unknown_fields,
    try_from = "RawEnvelope"
)]
pub(crate) struct AgentPatchEnvelope {
    pub(crate) metadata_revision: MetadataRevision,
    pub(crate) patch: AgentPatch,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RawEnvelope {
    metadata_revision: MetadataRevision,
    patch: AgentPatch,
}

impl TryFrom<RawEnvelope> for AgentPatchEnvelope {
    type Error = &'static str;

    fn try_from(value: RawEnvelope) -> Result<Self, Self::Error> {
        value.patch.validate()?;
        Ok(Self {
            metadata_revision: value.metadata_revision,
            patch: value.patch,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AgentPatch {
    #[serde(default, deserialize_with = "present_object")]
    attention_state: Option<AttentionState>,
    #[serde(default, deserialize_with = "present_object")]
    agent_metadata: Option<AgentMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AttentionState {
    #[serde(deserialize_with = "nullable")]
    observed_status: Option<String>,
    #[serde(deserialize_with = "nullable")]
    needs_user_input: Option<bool>,
    #[serde(deserialize_with = "nullable")]
    detected_question: Option<String>,
    #[serde(deserialize_with = "nullable")]
    suggested_options: Option<Vec<String>>,
}

fn nullable<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::<T>::deserialize(deserializer)
}

fn present_object<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

#[derive(Debug, Default)]
enum FieldUpdate<T> {
    #[default]
    Omitted,
    Clear,
    Set(T),
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for FieldUpdate<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match Option::<T>::deserialize(deserializer)? {
            Some(value) => Self::Set(value),
            None => Self::Clear,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct AgentMetadata {
    #[serde(default, deserialize_with = "present_object")]
    work_fields: Option<WorkFields>,
    #[serde(default)]
    summary: FieldUpdate<String>,
    #[serde(default)]
    plan_path: FieldUpdate<String>,
    #[serde(default)]
    worktree_path: FieldUpdate<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct WorkFields {
    task: String,
    #[serde(deserialize_with = "nullable")]
    next_action: Option<String>,
    work_phase: String,
    #[serde(deserialize_with = "nullable")]
    blocker_description: Option<String>,
    #[serde(deserialize_with = "nullable")]
    failed_command: Option<String>,
    #[serde(deserialize_with = "nullable")]
    failed_test: Option<String>,
}

impl AttentionState {
    fn validate(&self) -> Result<(), &'static str> {
        let status = self.observed_status.as_deref();
        if !matches!(
            status,
            None | Some(
                "waiting_for_input"
                    | "blocked"
                    | "failed"
                    | "capped"
                    | "working"
                    | "idle"
                    | "stale"
                    | "done"
            )
        ) {
            return Err("invalid observed status");
        }
        let input = self.needs_user_input == Some(true);
        if status == Some("waiting_for_input") && !input {
            return Err("waiting status requires user input");
        }
        if input
            && !matches!(
                status,
                Some("waiting_for_input" | "blocked" | "failed" | "capped")
            )
        {
            return Err("status cannot require user input");
        }
        if let Some(question) = &self.detected_question {
            if !valid_text(question) || !input {
                return Err("question requires user input");
            }
        }
        if let Some(options) = &self.suggested_options {
            if options.iter().any(|option| !valid_text(option)) {
                return Err("invalid suggested option");
            }
            if !options.is_empty() && self.detected_question.is_none() {
                return Err("options require a question");
            }
        }
        if !input
            && (self.detected_question.is_some()
                || self
                    .suggested_options
                    .as_ref()
                    .is_some_and(|options| !options.is_empty()))
        {
            return Err("prompt fields require user input");
        }
        Ok(())
    }
}

fn valid_text(value: &str) -> bool {
    !value.trim().is_empty() && !value.chars().any(char::is_control)
}

impl AgentPatch {
    fn validate(&self) -> Result<(), &'static str> {
        if self.attention_state.is_none() && self.agent_metadata.is_none() {
            return Err("empty patch");
        }
        if let Some(attention) = &self.attention_state {
            attention.validate()?;
        }
        if let Some(agent) = &self.agent_metadata {
            if agent.work_fields.is_none()
                && matches!(agent.summary, FieldUpdate::Omitted)
                && matches!(agent.plan_path, FieldUpdate::Omitted)
                && matches!(agent.worktree_path, FieldUpdate::Omitted)
            {
                return Err("empty agent metadata");
            }
            if let FieldUpdate::Set(summary) = &agent.summary {
                if !valid_text(summary) {
                    return Err("invalid summary");
                }
            }
            if let FieldUpdate::Set(path) = &agent.plan_path {
                if !valid_text(path) || Path::new(path).is_absolute() || path.starts_with('~') {
                    return Err("invalid plan path");
                }
            }
            if let Some(work) = &agent.work_fields {
                if !valid_text(&work.task)
                    || !matches!(
                        work.work_phase.as_str(),
                        "ideation" | "implementation" | "review" | "debugging" | "unknown"
                    )
                {
                    return Err("invalid work fields");
                }
                if [
                    work.next_action.as_deref(),
                    work.blocker_description.as_deref(),
                    work.failed_command.as_deref(),
                    work.failed_test.as_deref(),
                ]
                .into_iter()
                .flatten()
                .any(|value| !valid_text(value))
                {
                    return Err("invalid work field text");
                }
            }
        }
        Ok(())
    }

    pub(crate) fn scope(&self) -> WriteScope {
        WriteScope {
            attention: self.attention_state.is_some(),
            work: self
                .agent_metadata
                .as_ref()
                .is_some_and(|agent| agent.work_fields.is_some()),
        }
    }

    pub(crate) fn updates_summary(&self) -> bool {
        self.agent_metadata
            .as_ref()
            .is_some_and(|agent| !matches!(agent.summary, FieldUpdate::Omitted))
    }

    fn validate_current(
        &self,
        meta: &SessionMetadata,
        check_source: bool,
    ) -> io::Result<Option<PlanReference>> {
        if check_source && self.attention_state.is_some() {
            let source = meta
                .provenance
                .attention_source
                .as_deref()
                .unwrap_or(&meta.metadata_source);
            if !source_priority::can_overwrite("agent", source, None) {
                return Err(semantic_error(PatchSemanticError::Conflict));
            }
        }
        let Some(agent) = &self.agent_metadata else {
            return Ok(None);
        };
        if check_source
            && agent.work_fields.is_some()
            && !source_priority::can_overwrite("agent", &meta.metadata_source, None)
        {
            return Err(semantic_error(PatchSemanticError::Conflict));
        }
        if check_source
            && !matches!(agent.summary, FieldUpdate::Omitted)
            && !source_priority::can_overwrite(
                "agent",
                meta.provenance
                    .summary_source
                    .as_deref()
                    .unwrap_or("unknown"),
                None,
            )
        {
            return Err(semantic_error(PatchSemanticError::Conflict));
        }
        if let FieldUpdate::Set(path) = &agent.worktree_path {
            crate::session_location::WorktreeBinding::resolve(&meta.cwd, path)
                .map_err(|_| semantic_error(PatchSemanticError::BadRequest))?;
        }
        if matches!(agent.plan_path, FieldUpdate::Omitted) {
            return Ok(None);
        }
        if meta
            .plan_path
            .as_ref()
            .is_some_and(|plan| plan.source == PlanSource::UserSelected)
        {
            return Err(semantic_error(PatchSemanticError::Conflict));
        }
        let FieldUpdate::Set(path) = &agent.plan_path else {
            return Ok(None);
        };
        let (root, relative_path) = resolve_printed_plan_path(Path::new(&meta.cwd), path)
            .map_err(|_| semantic_error(PatchSemanticError::BadRequest))?;
        Ok(Some(PlanReference {
            worktree_root: Some(root.to_string_lossy().into_owned()),
            relative_path,
            source: PlanSource::AgentReported,
        }))
    }

    pub(crate) fn apply_checked(&self, meta: &mut SessionMetadata) -> Result<(), AgentPatchError> {
        self.apply(meta)
            .map_err(|error| map_error(SessionTransactionError::Io(error)))
    }

    fn apply(&self, meta: &mut SessionMetadata) -> io::Result<()> {
        require_live(meta)?;
        let plan = self.validate_current(meta, true)?;
        let binding = match self.agent_metadata.as_ref().map(|a| &a.worktree_path) {
            Some(FieldUpdate::Set(path)) => Some(
                crate::session_location::WorktreeBinding::resolve(&meta.cwd, path)
                    .map_err(|_| semantic_error(PatchSemanticError::BadRequest))?,
            ),
            _ => None,
        };
        if let Some(attention) = &self.attention_state {
            meta.observed_status = attention.observed_status.clone();
            meta.needs_user_input = attention.needs_user_input;
            meta.detected_question = attention.detected_question.clone();
            meta.suggested_options = attention.suggested_options.clone();
            meta.attention = canonical_attention(attention.observed_status.as_deref());
            let cleared = attention.observed_status.is_none();
            meta.provenance.attention_source =
                Some(if cleared { "unknown" } else { "agent" }.into());
            meta.provenance.attention_confidence = Some(if cleared { 0.0 } else { 1.0 });
            meta.provenance.attention_origin = Some("direct_agent".into());
        }
        if let Some(agent) = &self.agent_metadata {
            if !matches!(agent.worktree_path, FieldUpdate::Omitted) {
                meta.worktree_binding = binding;
            }
            if let Some(work) = &agent.work_fields {
                meta.task = work.task.clone();
                meta.next_action = work.next_action.clone();
                meta.work_phase = work.work_phase.clone();
                meta.blocker_description = work.blocker_description.clone();
                meta.failed_command = work.failed_command.clone();
                meta.failed_test = work.failed_test.clone();
                meta.metadata_source = "agent".into();
                meta.metadata_confidence = 1.0;
            }
            match &agent.summary {
                FieldUpdate::Omitted => {}
                FieldUpdate::Clear => {
                    meta.summary = None;
                    meta.provenance.summary_source = None;
                    meta.provenance.summary_confidence = None;
                    meta.provenance.summary_observed_at = None;
                }
                FieldUpdate::Set(value) => {
                    meta.summary = Some(value.clone());
                    meta.provenance.summary_source = Some("agent".into());
                    meta.provenance.summary_confidence = Some(1.0);
                    meta.provenance.summary_observed_at = Some(iso_now());
                }
            }
            match &agent.plan_path {
                FieldUpdate::Omitted => {}
                FieldUpdate::Clear => meta.plan_path = None,
                FieldUpdate::Set(_) => meta.plan_path = plan,
            }
        }
        meta.last_activity = iso_now();
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) enum AgentPatchError {
    Conflict,
    BadRequest,
    Unauthorized,
    Io(io::Error),
}

fn map_error(error: SessionTransactionError) -> AgentPatchError {
    match error {
        SessionTransactionError::Conflict => AgentPatchError::Conflict,
        SessionTransactionError::Io(error) => match error
            .get_ref()
            .and_then(|inner| inner.downcast_ref::<PatchSemanticError>())
        {
            Some(PatchSemanticError::Conflict) => AgentPatchError::Conflict,
            Some(PatchSemanticError::BadRequest) => AgentPatchError::BadRequest,
            Some(PatchSemanticError::Unauthorized) => AgentPatchError::Unauthorized,
            None => AgentPatchError::Io(error),
        },
    }
}

impl MetadataStore {
    pub(crate) fn validate_agent_patch(
        &self,
        id: &str,
        expected: &MetadataRevision,
        patch: &AgentPatch,
    ) -> Result<MetadataRevision, AgentPatchError> {
        self.with_session_write_lock(id, || {
            let record = self
                .try_read_revisioned_session(id)
                .map_err(AgentPatchError::Io)?
                .ok_or_else(|| {
                    AgentPatchError::Io(io::Error::new(
                        io::ErrorKind::NotFound,
                        "session record missing",
                    ))
                })?;
            if &record.revision != expected {
                return Err(AgentPatchError::Conflict);
            }
            require_live(&record.metadata)
                .map_err(|error| map_error(SessionTransactionError::Io(error)))?;
            patch
                .validate_current(&record.metadata, false)
                .map_err(|error| map_error(SessionTransactionError::Io(error)))?;
            Ok(record.revision)
        })
    }

    pub(crate) fn apply_agent_patch(
        &self,
        id: &str,
        expected: &MetadataRevision,
        patch: &AgentPatch,
    ) -> Result<RevisionedSession, AgentPatchError> {
        self.try_update_session_scoped(id, expected, patch.scope(), |meta| patch.apply(meta))
            .map_err(map_error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::test_session_metadata;
    use serde_json::json;
    use std::sync::{mpsc, Arc};
    use std::time::Duration;

    fn live_meta(id: &str, workspace: &str) -> SessionMetadata {
        let mut meta = test_session_metadata(id, "test", workspace, "running", "before", "before");
        meta.lifecycle = "alive".into();
        meta.lifecycle_phase = "active".into();
        meta.connectivity = "online".into();
        meta.terminal_outcome = None;
        meta.final_observed_status_snapshot = None;
        meta.cwd = workspace.into();
        meta
    }

    fn parse(value: serde_json::Value) -> Result<AgentPatchEnvelope, serde_json::Error> {
        serde_json::from_value(value)
    }

    #[test]
    fn task_worktree_report_is_revisioned_canonical_and_preserved_by_legacy_writes() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("main");
        let b = tmp.path().join("feature");
        let repo = git2::Repository::init(&a).unwrap();
        let tree = repo
            .find_tree(repo.index().unwrap().write_tree().unwrap())
            .unwrap();
        let sig = git2::Signature::now("test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "base", &tree, &[])
            .unwrap();
        repo.worktree("feature", &b, None).unwrap();
        std::fs::create_dir(b.join("src")).unwrap();
        let store = MetadataStore::new(&tmp.path().join("state"));
        let original = live_meta("worktree-report", a.to_str().unwrap());
        store.write_session(&original);
        let before = store
            .try_read_revisioned_session(&original.id)
            .unwrap()
            .unwrap();
        let set = parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{"worktreePath":b.join("src")}}})).expect("worktree patch must be accepted").patch;
        let bound = store
            .apply_agent_patch(&original.id, &before.revision, &set)
            .unwrap();
        let json = serde_json::to_value(&bound.metadata).unwrap();
        assert_eq!(
            Path::new(json["worktreeBinding"]["root"].as_str().unwrap())
                .canonicalize()
                .unwrap(),
            b.canonicalize().unwrap()
        );
        assert_eq!(bound.metadata.cwd, original.cwd);
        assert_eq!(
            bound.metadata.provenance.attention_updated_at,
            before.metadata.provenance.attention_updated_at
        );
        assert!(matches!(
            store.apply_agent_patch(&original.id, &before.revision, &set),
            Err(AgentPatchError::Conflict)
        ));
        store.write_session(&original); // a stale legacy writer must retain the newer binding
        let after = store
            .try_read_revisioned_session(&original.id)
            .unwrap()
            .unwrap();
        let summary = parse(
            json!({"metadataRevision":"0","patch":{"agentMetadata":{"summary":"separate update"}}}),
        )
        .unwrap()
        .patch;
        let after = store
            .apply_agent_patch(&original.id, &after.revision, &summary)
            .unwrap();
        assert_eq!(
            serde_json::to_value(&after.metadata).unwrap()["worktreeBinding"],
            json["worktreeBinding"]
        );
        let unrelated = tempfile::tempdir().unwrap();
        git2::Repository::init(unrelated.path()).unwrap();
        let bad = parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{"worktreePath":unrelated.path(),"summary":"must not land"}}})).unwrap().patch;
        assert!(matches!(
            store.apply_agent_patch(&original.id, &after.revision, &bad),
            Err(AgentPatchError::BadRequest)
        ));
        assert_eq!(
            store
                .try_read_revisioned_session(&original.id)
                .unwrap()
                .unwrap()
                .revision,
            after.revision
        );
        let clear =
            parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{"worktreePath":null}}}))
                .unwrap()
                .patch;
        let cleared = store
            .apply_agent_patch(&original.id, &after.revision, &clear)
            .unwrap();
        store.write_session(&bound.metadata);
        assert!(store
            .read_session(&original.id)
            .unwrap()
            .worktree_binding
            .is_none());
        assert!(serde_json::to_value(&cleared.metadata)
            .unwrap()
            .get("worktreeBinding")
            .is_none_or(|v| v.is_null()));
    }

    fn owned_native_wait(
        store: &MetadataStore,
        id: &str,
    ) -> (SessionMetadata, super::super::SessionWriteToken) {
        let mut meta = live_meta(id, "/tmp");
        meta.observed_status = Some("waiting_for_input".into());
        meta.attention = Some("needs_you".into());
        meta.needs_user_input = Some(true);
        meta.detected_question = Some("Native prompt".into());
        meta.suggested_options = Some(vec!["Continue".into()]);
        meta.provenance.attention_source = Some("agent".into());
        meta.provenance.attention_origin = Some("native_codex".into());
        meta.provenance.attention_confidence = Some(1.0);
        let owner = store.try_write_session_owned(&meta).unwrap().unwrap();
        (meta, owner)
    }

    fn native_clear(mut meta: SessionMetadata) -> SessionMetadata {
        meta.observed_status = Some("working".into());
        meta.attention = Some("working".into());
        meta.needs_user_input = Some(false);
        meta.detected_question = None;
        meta.suggested_options = Some(vec![]);
        meta
    }

    fn attention_patch() -> AgentPatch {
        parse(json!({"metadataRevision":"0","patch":{"attentionState":{
            "observedStatus":"blocked","needsUserInput":true,
            "detectedQuestion":"Agent prompt","suggestedOptions":[]}}}))
        .unwrap()
        .patch
    }

    fn work_patch() -> AgentPatch {
        parse(
            json!({"metadataRevision":"0","patch":{"agentMetadata":{"workFields":{
            "task":"Latest API work","nextAction":"Review patch","workPhase":"implementation",
            "blockerDescription":null,"failedCommand":null,"failedTest":null}}}}),
        )
        .unwrap()
        .patch
    }

    #[test]
    fn native_clear_first_rejects_stale_agent_patch_then_accepts_reread() {
        let dir = tempfile::tempdir().unwrap();
        let store = Arc::new(MetadataStore::new(dir.path()));
        let id = "agent-patch-native-clear-first";
        let (native, owner) = owned_native_wait(&store, id);
        let before = store.try_read_revisioned_session(id).unwrap().unwrap();
        let (at_final_check_tx, at_final_check_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let clear_store = Arc::clone(&store);
        let clear = std::thread::spawn(move || {
            clear_store.try_write_session_if_owned_when(&native_clear(native), &owner, || {
                at_final_check_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
                true
            })
        });
        at_final_check_rx
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        let (attempt_tx, attempt_rx) = mpsc::channel();
        let writer_store = Arc::clone(&store);
        let patch = attention_patch();
        let writer = std::thread::spawn(move || {
            attempt_tx.send(()).unwrap();
            writer_store.apply_agent_patch(id, &before.revision, &patch)
        });
        attempt_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let lane = store.session_write_lock(id);
        assert!(lane.try_lock().is_err());
        release_tx.send(()).unwrap();
        assert!(clear.join().unwrap().unwrap());
        assert!(matches!(
            writer.join().unwrap(),
            Err(AgentPatchError::Conflict)
        ));

        let after_clear = store.try_read_revisioned_session(id).unwrap().unwrap();
        assert_eq!(
            after_clear.metadata.observed_status.as_deref(),
            Some("working")
        );
        assert_eq!(after_clear.metadata.detected_question, None);
        assert_eq!(
            after_clear.metadata.provenance.attention_origin.as_deref(),
            Some("native_codex")
        );
        let retried = store
            .apply_agent_patch(id, &after_clear.revision, &attention_patch())
            .unwrap();
        assert_eq!(retried.metadata.observed_status.as_deref(), Some("blocked"));
        assert_eq!(
            retried.metadata.detected_question.as_deref(),
            Some("Agent prompt")
        );
        assert_eq!(
            retried.metadata.provenance.attention_origin.as_deref(),
            Some("direct_agent")
        );
    }

    #[test]
    fn attention_patch_first_invalidates_old_native_clear_owner() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let id = "agent-patch-before-native-clear";
        let (native, owner) = owned_native_wait(&store, id);
        let before = store.try_read_revisioned_session(id).unwrap().unwrap();
        let patched = store
            .apply_agent_patch(id, &before.revision, &attention_patch())
            .unwrap();
        assert!(!store
            .try_write_session_if_owned(&native_clear(native), &owner)
            .unwrap());
        let current = store.try_read_revisioned_session(id).unwrap().unwrap();
        assert_eq!(current.revision, patched.revision);
        assert_eq!(current.metadata.observed_status.as_deref(), Some("blocked"));
        assert_eq!(
            current.metadata.detected_question.as_deref(),
            Some("Agent prompt")
        );
        assert_eq!(
            current.metadata.provenance.attention_origin.as_deref(),
            Some("direct_agent")
        );
    }

    #[test]
    fn work_patch_first_keeps_native_clear_owner_and_latest_work() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let id = "agent-work-before-native-clear";
        let (native, owner) = owned_native_wait(&store, id);
        let before = store.try_read_revisioned_session(id).unwrap().unwrap();
        let patched = store
            .apply_agent_patch(id, &before.revision, &work_patch())
            .unwrap();
        assert_eq!(patched.metadata.task, "Latest API work");
        assert!(store
            .try_write_session_if_owned(&native_clear(native), &owner)
            .unwrap());
        let current = store.try_read_revisioned_session(id).unwrap().unwrap();
        assert_eq!(current.metadata.observed_status.as_deref(), Some("working"));
        assert_eq!(current.metadata.detected_question, None);
        assert_eq!(current.metadata.task, "Latest API work");
        assert_eq!(
            current.metadata.next_action.as_deref(),
            Some("Review patch")
        );
        assert_eq!(current.metadata.metadata_source, "agent");
        assert_eq!(
            current.metadata.provenance.attention_origin.as_deref(),
            Some("native_codex")
        );
    }

    #[test]
    fn attention_requires_all_fields_and_rejects_protected_fields() {
        let base = json!({"metadataRevision":"1","patch":{"attentionState":{
            "observedStatus":"waiting_for_input","needsUserInput":true,
            "detectedQuestion":null,"suggestedOptions":[]}}});
        assert!(parse(base.clone()).is_ok());
        let mut missing = base.clone();
        missing["patch"]["attentionState"]
            .as_object_mut()
            .unwrap()
            .remove("detectedQuestion");
        assert!(parse(missing).is_err());
        let mut protected = base;
        protected["patch"]["attentionState"]["attentionSource"] = json!("user");
        assert!(parse(protected).is_err());
    }

    #[test]
    fn prompt_tuple_rules_cover_each_status() {
        for status in ["blocked", "failed", "capped", "waiting_for_input"] {
            let tuple = AttentionState {
                observed_status: Some(status.into()),
                needs_user_input: Some(true),
                detected_question: None,
                suggested_options: Some(vec![]),
            };
            assert!(tuple.validate().is_ok(), "{status}");
        }
        for status in ["working", "idle", "stale", "done"] {
            let tuple = AttentionState {
                observed_status: Some(status.into()),
                needs_user_input: Some(true),
                detected_question: None,
                suggested_options: Some(vec![]),
            };
            assert!(tuple.validate().is_err(), "{status}");
        }
        let tuple = AttentionState {
            observed_status: None,
            needs_user_input: Some(false),
            detected_question: Some("still here".into()),
            suggested_options: None,
        };
        assert!(tuple.validate().is_err());
        let waiting_without_input = AttentionState {
            observed_status: Some("waiting_for_input".into()),
            needs_user_input: Some(false),
            detected_question: None,
            suggested_options: None,
        };
        assert!(waiting_without_input.validate().is_err());
        let options_without_question = AttentionState {
            observed_status: Some("blocked".into()),
            needs_user_input: Some(true),
            detected_question: None,
            suggested_options: Some(vec!["Continue".into()]),
        };
        assert!(options_without_question.validate().is_err());
        let whitespace_question = AttentionState {
            observed_status: Some("blocked".into()),
            needs_user_input: Some(true),
            detected_question: Some("  ".into()),
            suggested_options: None,
        };
        assert!(whitespace_question.validate().is_err());
        let clear = AttentionState {
            observed_status: None,
            needs_user_input: Some(false),
            detected_question: None,
            suggested_options: Some(vec![]),
        };
        assert!(clear.validate().is_ok());
    }

    #[test]
    fn agent_groups_reject_empty_partial_and_whitespace() {
        assert!(parse(json!({"metadataRevision":"0","patch":{}})).is_err());
        assert!(parse(json!({"metadataRevision":"0","patch":{"agentMetadata":null}})).is_err());
        assert!(parse(json!({"metadataRevision":"0","patch":{"attentionState":null}})).is_err());
        assert!(parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{}}})).is_err());
        assert!(parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{"workFields":null,"summary":"x"}}})).is_err());
        assert!(parse(
            json!({"metadataRevision":"0","patch":{"agentMetadata":{"workFields":{"task":"x"}}}})
        )
        .is_err());
        assert!(
            parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{"summary":"   "}}}))
                .is_err()
        );
    }

    #[test]
    fn summary_only_keeps_work_and_attention_authority() {
        let mut meta = live_meta("patch-summary", "/tmp");
        meta.metadata_source = "user".into();
        meta.provenance.attention_source = Some("peon".into());
        let patch =
            parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{"summary":"Done"}}}))
                .unwrap()
                .patch;
        assert_eq!(patch.scope().attention, false);
        assert_eq!(patch.scope().work, false);
        patch.apply(&mut meta).unwrap();
        assert_eq!(meta.summary.as_deref(), Some("Done"));
        assert_eq!(meta.provenance.summary_source.as_deref(), Some("agent"));
        assert_eq!(meta.metadata_source, "user");
        assert_eq!(meta.provenance.attention_source.as_deref(), Some("peon"));
    }

    #[test]
    fn explicit_null_summary_clears_content_and_all_provenance() {
        let mut meta = live_meta("agent-patch-summary-clear", "/tmp");
        meta.summary = Some("prior".into());
        meta.provenance.summary_source = Some("agent".into());
        meta.provenance.summary_confidence = Some(1.0);
        meta.provenance.summary_observed_at = Some("2026-01-01T00:00:00Z".into());
        let patch =
            parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{"summary":null}}}))
                .unwrap()
                .patch;
        patch.apply(&mut meta).unwrap();
        assert_eq!(meta.summary, None);
        assert_eq!(meta.provenance.summary_source, None);
        assert_eq!(meta.provenance.summary_confidence, None);
        assert_eq!(meta.provenance.summary_observed_at, None);
    }

    #[test]
    fn identical_attention_write_advances_revision_and_clear_relinquishes_source() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let id = "agent-patch-identical";
        store.write_session(&live_meta(id, "/tmp"));
        let first = store.try_read_revisioned_session(id).unwrap().unwrap();
        let set = parse(json!({"metadataRevision":"0","patch":{"attentionState":{
            "observedStatus":"working","needsUserInput":false,"detectedQuestion":null,"suggestedOptions":[]}}})).unwrap().patch;
        let second = store.apply_agent_patch(id, &first.revision, &set).unwrap();
        let third = store.apply_agent_patch(id, &second.revision, &set).unwrap();
        assert_ne!(third.revision, second.revision);
        assert_eq!(
            third.metadata.provenance.attention_source.as_deref(),
            Some("agent")
        );
        let clear = parse(json!({"metadataRevision":"0","patch":{"attentionState":{
            "observedStatus":null,"needsUserInput":false,"detectedQuestion":null,"suggestedOptions":[]}}})).unwrap().patch;
        let fourth = store
            .apply_agent_patch(id, &third.revision, &clear)
            .unwrap();
        assert_eq!(fourth.metadata.attention, None);
        assert_eq!(
            fourth.metadata.provenance.attention_source.as_deref(),
            Some("unknown")
        );
        assert!(source_priority::can_overwrite("peon", "unknown", Some(0)));
    }

    #[test]
    fn user_selected_plan_conflicts_without_applying_summary() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let id = "agent-patch-user-plan";
        let mut meta = live_meta(id, "/tmp");
        meta.plan_path = Some(PlanReference {
            worktree_root: None,
            relative_path: "specs/user.md".into(),
            source: PlanSource::UserSelected,
        });
        store.write_session(&meta);
        let first = store.try_read_revisioned_session(id).unwrap().unwrap();
        let patch = parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{"summary":"new", "planPath":null}}})).unwrap().patch;
        assert!(matches!(
            store.apply_agent_patch(id, &first.revision, &patch),
            Err(AgentPatchError::Conflict)
        ));
        let after = store.try_read_revisioned_session(id).unwrap().unwrap();
        assert_eq!(after.revision, first.revision);
        assert_eq!(after.metadata.summary, first.metadata.summary);
        assert_eq!(
            after.metadata.plan_path.as_ref().unwrap().source,
            PlanSource::UserSelected
        );
    }

    #[test]
    fn user_owned_attention_conflicts_with_combined_patch_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let id = "agent-patch-user-attention";
        let mut meta = live_meta(id, "/tmp");
        meta.provenance.attention_source = Some("user".into());
        store.write_session(&meta);
        let before = store.try_read_revisioned_session(id).unwrap().unwrap();
        let patch = parse(json!({"metadataRevision":"0","patch":{
            "attentionState":{"observedStatus":"working","needsUserInput":false,"detectedQuestion":null,"suggestedOptions":[]},
            "agentMetadata":{"summary":"must not land"}}})).unwrap().patch;
        assert!(matches!(
            store.apply_agent_patch(id, &before.revision, &patch),
            Err(AgentPatchError::Conflict)
        ));
        let after = store.try_read_revisioned_session(id).unwrap().unwrap();
        assert_eq!(after.revision, before.revision);
        assert_eq!(after.metadata.summary, before.metadata.summary);
        assert_eq!(
            after.metadata.provenance.attention_source.as_deref(),
            Some("user")
        );
    }

    #[test]
    fn summary_only_write_preserves_scope_timestamps() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let id = "agent-patch-summary-scope";
        let mut meta = live_meta(id, "/tmp");
        meta.provenance.attention_updated_at = Some("2026-01-01T00:00:00Z".into());
        meta.provenance.work_metadata_updated_at = Some("2026-01-02T00:00:00Z".into());
        store.write_session(&meta);
        let before = store.try_read_revisioned_session(id).unwrap().unwrap();
        let patch =
            parse(json!({"metadataRevision":"0","patch":{"agentMetadata":{"summary":"Done"}}}))
                .unwrap()
                .patch;
        let after = store
            .apply_agent_patch(id, &before.revision, &patch)
            .unwrap();
        assert_ne!(after.revision, before.revision);
        assert_eq!(
            after.metadata.provenance.attention_updated_at,
            before.metadata.provenance.attention_updated_at
        );
        assert_eq!(
            after.metadata.provenance.work_metadata_updated_at,
            before.metadata.provenance.work_metadata_updated_at
        );
    }

    #[test]
    fn complete_work_snapshot_updates_work_only() {
        let dir = tempfile::tempdir().unwrap();
        let store = MetadataStore::new(dir.path());
        let id = "agent-patch-work-snapshot";
        let mut meta = live_meta(id, "/tmp");
        meta.provenance.attention_source = Some("peon".into());
        meta.provenance.attention_updated_at = Some("2026-01-01T00:00:00Z".into());
        store.write_session(&meta);
        let before = store.try_read_revisioned_session(id).unwrap().unwrap();
        let patch = parse(
            json!({"metadataRevision":"0","patch":{"agentMetadata":{"workFields":{
            "task":"Implement API","nextAction":"Run tests","workPhase":"implementation",
            "blockerDescription":null,"failedCommand":null,"failedTest":null}}}}),
        )
        .unwrap()
        .patch;
        let after = store
            .apply_agent_patch(id, &before.revision, &patch)
            .unwrap();
        assert_eq!(after.metadata.task, "Implement API");
        assert_eq!(after.metadata.next_action.as_deref(), Some("Run tests"));
        assert_eq!(after.metadata.metadata_source, "agent");
        assert_eq!(
            after.metadata.provenance.attention_source.as_deref(),
            Some("peon")
        );
        assert_eq!(
            after.metadata.provenance.attention_updated_at,
            before.metadata.provenance.attention_updated_at
        );
        assert_ne!(
            after.metadata.provenance.work_metadata_updated_at,
            before.metadata.provenance.work_metadata_updated_at
        );
    }

    #[test]
    fn filesystem_permission_error_remains_server_error() {
        let error = SessionTransactionError::Io(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "disk denied",
        ));
        assert!(matches!(map_error(error), AgentPatchError::Io(_)));
    }

    #[test]
    fn plan_path_resolves_inside_session_repository_with_sidecar_source() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        std::fs::create_dir(dir.path().join("specs")).unwrap();
        std::fs::write(dir.path().join("specs/plan.md"), "# Plan\n").unwrap();
        let store = MetadataStore::new(&dir.path().join("metadata"));
        let id = "agent-patch-plan-path";
        store.write_session(&live_meta(id, &dir.path().display().to_string()));
        let before = store.try_read_revisioned_session(id).unwrap().unwrap();
        let patch = parse(
            json!({"metadataRevision":"0","patch":{"agentMetadata":{"planPath":"specs/plan.md"}}}),
        )
        .unwrap()
        .patch;
        let after = store
            .apply_agent_patch(id, &before.revision, &patch)
            .unwrap();
        let plan = after.metadata.plan_path.unwrap();
        assert_eq!(plan.relative_path, "specs/plan.md");
        assert_eq!(plan.source, PlanSource::AgentReported);
        assert_eq!(
            plan.worktree_root.as_deref(),
            dir.path().canonicalize().unwrap().to_str()
        );
    }
}
