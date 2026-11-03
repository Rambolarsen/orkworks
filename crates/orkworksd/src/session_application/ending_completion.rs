//! Ordered legacy finalization and honest cross-file partial results.
//! SQL routing and shared-lock JSON scheduling remain separately gated.
use super::*;
use crate::runtime::session_runtime::{RuntimeCommand, RuntimeIdentity};
use std::io;

#[derive(Debug, Default)]
pub(crate) enum WriteResult {
    #[default]
    NotAttempted,
    Absent,
    Written,
    Failed(io::Error),
}
#[derive(Debug, Default)]
pub(crate) struct Completion {
    pub(crate) replay: WriteResult,
    pub(crate) metadata: WriteResult,
    pub(crate) event: WriteResult,
    pub(crate) published: bool,
}
impl Completion {
    fn log_failures(&self, id: &str) {
        for (stage, result) in [
            ("replay", &self.replay),
            ("metadata", &self.metadata),
            ("event", &self.event),
        ] {
            if let WriteResult::Failed(error) = result {
                tracing::warn!(session_id = id, stage, %error,
                    metadata_committed = matches!(self.metadata, WriteResult::Written),
                    live_published = self.published, "session ending persistence is incomplete");
            }
        }
    }
}
struct Workspace {
    identity: RecommendationDeliveryWorkspace,
    reader: Arc<metadata::session_reader::SessionReader>,
}
struct Context {
    workspace: Option<Workspace>,
    runtime: RuntimeIdentity,
    control: tokio::sync::mpsc::Sender<RuntimeCommand>,
    created: String,
    harness: Option<String>,
    harness_id: Option<String>,
}
impl Context {
    fn capture(ws: Option<&WorkspaceState>, h: &SessionHandle) -> Self {
        Self {
            workspace: ws.map(|ws| Workspace {
                identity: RecommendationDeliveryWorkspace::capture(ws),
                reader: ws.metadata.session_reader(),
            }),
            runtime: h.runtime.identity(),
            control: h.runtime.control_tx.clone(),
            created: h.info.created_at.clone(),
            harness: h.info.harness.clone(),
            harness_id: h.info.harness_id.clone(),
        }
    }
    fn workspace_matches(&self, ws: Option<&WorkspaceState>) -> bool {
        match (&self.workspace, ws) {
            (None, None) => true,
            (Some(old), Some(ws)) => {
                old.identity.matches(ws) && Arc::ptr_eq(&old.reader, &ws.metadata.session_reader())
            }
            _ => false,
        }
    }
    fn runtime_matches(&self, id: &str, h: &SessionHandle) -> bool {
        h.info.id == id
            && h.runtime.identity() == self.runtime
            && self.control.same_channel(&h.runtime.control_tx)
            && h.info.created_at == self.created
            && h.info.harness == self.harness
            && h.info.harness_id == self.harness_id
            && h.info.lifecycle_phase == "ending"
    }
    fn record_matches(&self, meta: &metadata::SessionMetadata) -> bool {
        meta.created_at == self.created
            && self
                .harness
                .as_deref()
                .is_none_or(|kind| kind == meta.harness)
    }
}
impl SessionApplication {
    fn capture_ending_context(
        &self,
        id: &str,
        generation: crate::runtime::session_runtime::RuntimeGeneration,
    ) -> Option<Context> {
        let workspace = self.state.workspace.lock().unwrap();
        let sessions = self.state.sessions.lock().unwrap();
        let h = sessions.get(id).filter(|h| {
            h.info.id == id
                && h.runtime.run_generation() == generation
                && h.info.lifecycle_phase == "ending"
        })?;
        Some(Context::capture(workspace.as_ref(), h))
    }
    /// The internal finalizer owns this blocking job through completion even if
    /// its waiter disappears. A JoinError leaves completion unknown, not rolled back.
    pub(crate) async fn finish_session_ending(
        &self,
        id: String,
        generation: crate::runtime::session_runtime::RuntimeGeneration,
        final_snapshot: metadata::ObservedStatusSnapshotMetadata,
        fallback_terminal_status: String,
    ) -> Result<Completion, tokio::task::JoinError> {
        let Some(context) = self.capture_ending_context(&id, generation) else {
            return Ok(Completion::default());
        };
        let state = self.state.clone();
        #[cfg(test)]
        testing::dispatched(&state);
        tokio::task::spawn_blocking(move || {
            let result = SessionApplication::new(state).complete_session_ending_captured(
                &id,
                context,
                final_snapshot,
                &fallback_terminal_status,
            );
            result.log_failures(&id);
            result
        })
        .await
    }
    /// Compatibility callers receive live publication, never an event-durability promise.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn complete_session_ending(
        &self,
        id: &str,
        generation: crate::runtime::session_runtime::RuntimeGeneration,
        final_snapshot: metadata::ObservedStatusSnapshotMetadata,
        fallback_terminal_status: &str,
    ) -> bool {
        let result = self.complete_session_ending_result(
            id,
            generation,
            final_snapshot,
            fallback_terminal_status,
        );
        result.log_failures(id);
        result.published
    }
    pub(crate) fn complete_session_ending_result(
        &self,
        id: &str,
        generation: crate::runtime::session_runtime::RuntimeGeneration,
        final_snapshot: metadata::ObservedStatusSnapshotMetadata,
        fallback_terminal_status: &str,
    ) -> Completion {
        let Some(context) = self.capture_ending_context(id, generation) else {
            return Completion::default();
        };
        self.complete_session_ending_captured(id, context, final_snapshot, fallback_terminal_status)
    }
    fn complete_session_ending_captured(
        &self,
        id: &str,
        context: Context,
        final_snapshot: metadata::ObservedStatusSnapshotMetadata,
        fallback_terminal_status: &str,
    ) -> Completion {
        let mut outcome = Completion::default();
        #[cfg(test)]
        testing::checkpoint(&self.state, testing::Stage::BeforeHistory);
        let now = iso_now();
        let mut final_status = fallback_terminal_status.to_owned();
        {
            // Preserve the existing grid writer's gate across the compound
            // sequence. No deferred resize can land between grid and ended record.
            let _grid = TERMINAL_SIZE_WRITE_LOCK.lock().unwrap();
            let workspace = self.state.workspace.lock().unwrap();
            if !context.workspace_matches(workspace.as_ref()) {
                return outcome;
            }
            let size = {
                let sessions = self.state.sessions.lock().unwrap();
                let Some(h) = sessions.get(id).filter(|h| context.runtime_matches(id, h)) else {
                    return outcome;
                };
                (h.runtime.last_cols, h.runtime.last_rows)
            };
            if let Some(ws) = workspace.as_ref() {
                // Validate the record's creation before any replay-file effect.
                let present = match ws.metadata.try_read_revisioned_session(id) {
                    Ok(Some(record)) if context.record_matches(&record.metadata) => true,
                    Ok(Some(_)) => return outcome,
                    Ok(None) => {
                        outcome.metadata = WriteResult::Absent;
                        false
                    }
                    Err(error) => {
                        outcome.metadata = WriteResult::Failed(error);
                        return outcome;
                    }
                };
                let grid = ws
                    .metadata
                    .try_write_terminal_size(id, size.0, size.1)
                    .and_then(|()| {
                        (ws.metadata.read_terminal_size(id) == Some(size))
                            .then_some(())
                            .ok_or_else(|| {
                                io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "final terminal grid could not be confirmed",
                                )
                            })
                    });
                outcome.replay = match grid {
                    Ok(()) => WriteResult::Written,
                    Err(error) => WriteResult::Failed(error),
                };
                if present && matches!(outcome.replay, WriteResult::Failed(_)) {
                    return outcome;
                }
                if present {
                    #[cfg(test)]
                    testing::checkpoint(&self.state, testing::Stage::BeforeMetadata);
                    let stored =
                        ws.metadata
                            .mutate_session_optional(id, WriteScope::ATTENTION, |meta| {
                                if !context.record_matches(meta) {
                                    return Ok(false);
                                }
                                Ok(metadata::lifecycle_mutations::LifecycleMutation::Finalize {
                                    snapshot: final_snapshot.clone(),
                                    fallback_status: fallback_terminal_status.into(),
                                    timestamp: now.clone(),
                                }
                                .apply(meta)
                                .is_some())
                            });
                    match stored {
                        Ok(Some(record)) => {
                            outcome.metadata = WriteResult::Written;
                            final_status = record.metadata.status;
                            #[cfg(test)]
                            testing::checkpoint(&self.state, testing::Stage::BeforeEvent);
                            outcome.event = match ws.metadata.try_append_event(
                                id,
                                &metadata::Event {
                                    event_type: "session.status".into(),
                                    timestamp: now.clone(),
                                    status: final_status.clone(),
                                    observed_status: final_snapshot.value.clone(),
                                    confidence: final_snapshot.confidence,
                                    summary: None,
                                    source: None,
                                    recommendation_id: None,
                                },
                            ) {
                                Ok(()) => WriteResult::Written,
                                Err(error) => WriteResult::Failed(error),
                            };
                        }
                        Ok(None) => return outcome,
                        Err(SessionTransactionError::Io(error))
                            if error.kind() == io::ErrorKind::NotFound =>
                        {
                            outcome.metadata = WriteResult::Absent;
                        }
                        Err(SessionTransactionError::Io(error)) => {
                            outcome.metadata = WriteResult::Failed(error);
                            return outcome;
                        }
                        Err(error) => {
                            outcome.metadata = WriteResult::Failed(io::Error::other(error));
                            return outcome;
                        }
                    }
                }
            }
        }
        #[cfg(test)]
        testing::checkpoint(&self.state, testing::Stage::BeforePublish);
        let workspace = self.state.workspace.lock().unwrap();
        if !context.workspace_matches(workspace.as_ref()) {
            return outcome;
        }
        let mut sessions = self.state.sessions.lock().unwrap();
        if let Some(h) = sessions
            .get_mut(id)
            .filter(|h| context.runtime_matches(id, h))
        {
            h.runtime.attention_owner.accepted_write();
            h.info.status = final_status.clone();
            h.info.lifecycle_phase = "ended".into();
            h.info.lifecycle = "dead".into();
            h.info.attention = None;
            h.info.connectivity = Some(connectivity_for_status(&final_status).into());
            h.info.terminal_outcome = terminal_outcome_for_status(&final_status);
            h.info.observed_status = None;
            h.info.final_observed_status = final_snapshot.value;
            h.info.last_activity_at = Some(now);
            h.resume_in_progress = false;
            outcome.published = true;
        }
        outcome
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::super::*;
    use std::sync::LazyLock;
    use tokio::sync::oneshot;
    #[derive(Clone, Copy, Hash, Eq, PartialEq)]
    pub(crate) enum Stage {
        BeforeHistory,
        BeforeMetadata,
        BeforeEvent,
        BeforePublish,
    }
    type Key = (usize, Stage);
    type Gate = (oneshot::Sender<()>, std::sync::mpsc::Receiver<()>);
    static GATES: LazyLock<Mutex<std::collections::HashMap<Key, Gate>>> =
        LazyLock::new(|| Mutex::new(Default::default()));
    pub(crate) struct Pause {
        key: Key,
        entered: Option<oneshot::Receiver<()>>,
        release: Option<std::sync::mpsc::Sender<()>>,
    }
    impl Pause {
        pub(crate) async fn entered(&mut self) {
            self.entered.take().unwrap().await.unwrap();
        }
    }
    impl Drop for Pause {
        fn drop(&mut self) {
            GATES.lock().unwrap().remove(&self.key);
            if let Some(tx) = self.release.take() {
                let _ = tx.send(());
            }
        }
    }
    pub(crate) fn hold(state: &Arc<AppState>, stage: Stage) -> Pause {
        let key = (Arc::as_ptr(state) as usize, stage);
        let (tx, entered) = oneshot::channel();
        let (release, rx) = std::sync::mpsc::channel();
        assert!(GATES.lock().unwrap().insert(key, (tx, rx)).is_none());
        Pause {
            key,
            entered: Some(entered),
            release: Some(release),
        }
    }
    static DISPATCHES: LazyLock<Mutex<std::collections::HashMap<usize, oneshot::Sender<()>>>> =
        LazyLock::new(|| Mutex::new(Default::default()));
    pub(crate) struct DispatchNotice {
        key: usize,
        entered: Option<oneshot::Receiver<()>>,
    }
    impl DispatchNotice {
        pub(crate) async fn entered(&mut self) {
            self.entered.take().unwrap().await.unwrap();
        }
    }
    impl Drop for DispatchNotice {
        fn drop(&mut self) {
            DISPATCHES.lock().unwrap().remove(&self.key);
        }
    }
    pub(crate) fn notice_dispatch(state: &Arc<AppState>) -> DispatchNotice {
        let key = Arc::as_ptr(state) as usize;
        let (tx, rx) = oneshot::channel();
        assert!(DISPATCHES.lock().unwrap().insert(key, tx).is_none());
        DispatchNotice {
            key,
            entered: Some(rx),
        }
    }
    pub(crate) fn dispatched(state: &Arc<AppState>) {
        if let Some(tx) = DISPATCHES
            .lock()
            .unwrap()
            .remove(&(Arc::as_ptr(state) as usize))
        {
            let _ = tx.send(());
        }
    }
    pub(crate) fn checkpoint(state: &Arc<AppState>, stage: Stage) {
        let gate = GATES
            .lock()
            .unwrap()
            .remove(&(Arc::as_ptr(state) as usize, stage));
        if let Some((tx, rx)) = gate {
            let _ = tx.send(());
            let _ = rx.recv();
        }
    }
}
