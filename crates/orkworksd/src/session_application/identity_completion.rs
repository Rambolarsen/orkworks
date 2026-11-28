//! Identity publication and label reservation consume one transaction result.
//! Neither follow-up rereads session JSON or adopts a replacement runtime.
use super::*;
use crate::codex_session_store::{DeferredLabelRefresh, LabelRefreshSchedule};
use crate::runtime::session_runtime::{RuntimeCommand, RuntimeIdentity};

struct Runtime {
    identity: RuntimeIdentity,
    control: tokio::sync::mpsc::Sender<RuntimeCommand>,
    created: String,
    harness: Option<String>,
    harness_id: Option<String>,
}
pub(super) struct Context {
    state: Weak<AppState>,
    workspace: RecommendationDeliveryWorkspace,
    reader: Arc<metadata::session_reader::SessionReader>,
    runtime: Option<Runtime>,
    token: Option<String>,
}
impl Context {
    pub(super) fn capture(
        state: &Arc<AppState>,
        ws: &WorkspaceState,
        id: &str,
        authenticated: bool,
        token: Option<&str>,
    ) -> Self {
        let runtime = state.sessions.lock().unwrap().get(id).map(|h| Runtime {
            identity: h.runtime.identity(),
            control: h.runtime.control_tx.clone(),
            created: h.info.created_at.clone(),
            harness: h.info.harness.clone(),
            harness_id: h.info.harness_id.clone(),
        });
        Self {
            state: Arc::downgrade(state),
            workspace: RecommendationDeliveryWorkspace::capture(ws),
            reader: ws.metadata.session_reader(),
            runtime,
            token: if authenticated {
                token.map(str::to_owned)
            } else {
                None
            },
        }
    }
    fn current_workspace(&self, state: &Arc<AppState>, ws: &WorkspaceState) -> bool {
        self.state
            .upgrade()
            .is_some_and(|old| Arc::ptr_eq(&old, state))
            && self.workspace.matches(ws)
            && Arc::ptr_eq(&self.reader, &ws.metadata.session_reader())
    }
    fn current_runtime(
        &self,
        h: &SessionHandle,
        identity: &metadata::HarnessSessionIdentity,
    ) -> bool {
        self.runtime.as_ref().is_some_and(|old| {
            old.identity == h.runtime.identity()
                && old.control.same_channel(&h.runtime.control_tx)
                && old.created == h.info.created_at
                && old.harness == h.info.harness
                && old.harness_id == h.info.harness_id
        }) && identity.created_at == h.info.created_at
            && identity.id == h.info.id
            && h.info
                .harness
                .as_deref()
                .is_none_or(|kind| kind == identity.harness.as_str())
    }
}

pub(crate) struct Completion {
    pub(crate) result: metadata::HarnessSessionMergeResult,
    identity: Option<metadata::HarnessSessionIdentity>,
    context: Option<Context>,
}
impl Completion {
    pub(super) fn new(merge: metadata::HarnessSessionMerge, context: Option<Context>) -> Self {
        Self {
            result: merge.result,
            identity: merge.identity,
            context,
        }
    }
    /// Caller holds the selected workspace guard; this performs memory work only.
    pub(super) fn publish(&self, state: &Arc<AppState>, ws: &WorkspaceState, id: &str) {
        if !matches!(
            self.result,
            metadata::HarnessSessionMergeResult::Accepted
                | metadata::HarnessSessionMergeResult::IgnoredUnchanged
        ) {
            return;
        }
        let (Some(context), Some(identity)) = (&self.context, &self.identity) else {
            return;
        };
        if !context.current_workspace(state, ws) || identity.id != id {
            return;
        }
        let Some(resume) = identity.resume.as_ref() else {
            return;
        };
        let mut sessions = state.sessions.lock().unwrap();
        let Some(h) = sessions
            .get_mut(id)
            .filter(|h| context.current_runtime(h, identity))
        else {
            return;
        };
        if h.info
            .resume
            .as_ref()
            .and_then(|r| r.harness_session_id.as_ref())
            != resume.harness_session_id.as_ref()
        {
            if let Some(tracker) = h.runtime.native_approval.as_mut() {
                tracker.identity_changed();
            }
        }
        h.info.resume = Some(resume.clone());
    }
    pub(crate) fn reserve_codex_label_refresh(
        &self,
        id: &str,
        native_id: &str,
    ) -> Option<(RuntimeIdentity, LabelRefreshSchedule)> {
        self.reserve_codex_label_refresh_with(id, native_id, None)
    }

    pub(crate) fn claim_deferred_codex_label_refresh(
        &self,
        id: &str,
        native_id: &str,
        deferred: &DeferredLabelRefresh,
    ) -> Option<u64> {
        let (_, LabelRefreshSchedule::Immediate(epoch)) =
            self.reserve_codex_label_refresh_with(id, native_id, Some(deferred))?
        else {
            return None;
        };
        Some(epoch)
    }

    fn reserve_codex_label_refresh_with(
        &self,
        id: &str,
        native_id: &str,
        deferred: Option<&DeferredLabelRefresh>,
    ) -> Option<(RuntimeIdentity, LabelRefreshSchedule)> {
        let context = self.context.as_ref()?;
        let state = context.state.upgrade()?;
        #[cfg(test)]
        testing::before_publication(&context.reader.root().join("label-reservation"));
        crate::codex_session_store::with_label_refresh_reservation(
            id,
            native_id,
            std::time::Duration::from_secs(5),
            deferred,
            |reserve| {
                let workspace = state.workspace.lock().unwrap();
                let ws = workspace.as_ref()?;
                let sessions = state.sessions.lock().unwrap();
                let h = sessions.get(id)?;
                if !self.can_refresh_codex_label(&state, ws, h, native_id)
                    || !crate::codex_session_store::accept_native_label_identity(id, native_id)
                {
                    return None;
                }
                Some((h.runtime.identity(), reserve()?))
            },
        )
    }

    pub(crate) fn can_lookup_codex_label(
        &self,
        state: &Arc<AppState>,
        id: &str,
        native_id: &str,
        epoch: u64,
    ) -> bool {
        crate::codex_session_store::with_label_refresh_generation(id, epoch, || {
            if crate::codex_session_store::native_label_refresh_is_blocked(id, native_id) {
                return false;
            }
            let workspace = state.workspace.lock().unwrap();
            let Some(ws) = workspace.as_ref() else {
                return false;
            };
            let sessions = state.sessions.lock().unwrap();
            sessions
                .get(id)
                .is_some_and(|h| self.can_refresh_codex_label(state, ws, h, native_id))
        })
        .unwrap_or(false)
    }

    pub(super) fn can_refresh_codex_label(
        &self,
        state: &Arc<AppState>,
        ws: &WorkspaceState,
        h: &SessionHandle,
        native_id: &str,
    ) -> bool {
        let (Some(context), Some(identity)) = (&self.context, &self.identity) else {
            return false;
        };
        should_schedule_codex_label_refresh(self.result, identity.label_source)
            && identity.harness == "codex"
            && identity
                .resume
                .as_ref()
                .and_then(|r| r.harness_session_id.as_deref())
                == Some(native_id)
            && context.current_workspace(state, ws)
            && context.current_runtime(h, identity)
            && h.info.lifecycle_phase == "active"
            && h.info.harness.as_deref() == Some("codex")
            && h.info
                .resume
                .as_ref()
                .and_then(|r| r.harness_session_id.as_deref())
                == Some(native_id)
            && context.token.as_deref().is_some_and(|token| {
                crate::runtime::terminal_runtime::verify_workflow_report_token(&identity.id, token)
            })
    }
}
pub(crate) fn should_schedule_codex_label_refresh(
    result: metadata::HarnessSessionMergeResult,
    label_source: metadata::LabelSource,
) -> bool {
    matches!(
        result,
        metadata::HarnessSessionMergeResult::Accepted
            | metadata::HarnessSessionMergeResult::IgnoredUnchanged
    ) && label_source.is_automatic()
}

#[cfg(test)]
pub(crate) mod testing {
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::{LazyLock, Mutex};
    use tokio::sync::oneshot;
    type Gate = (oneshot::Sender<()>, std::sync::mpsc::Receiver<()>);
    static GATES: LazyLock<Mutex<HashMap<PathBuf, Gate>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    pub(crate) struct Pause {
        entered: Option<oneshot::Receiver<()>>,
        release: Option<std::sync::mpsc::Sender<()>>,
        key: PathBuf,
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
    pub(crate) fn hold(root: PathBuf) -> Pause {
        let (tx, entered) = oneshot::channel();
        let (release, rx) = std::sync::mpsc::channel();
        assert!(GATES
            .lock()
            .unwrap()
            .insert(root.clone(), (tx, rx))
            .is_none());
        Pause {
            entered: Some(entered),
            release: Some(release),
            key: root,
        }
    }
    pub(crate) fn before_publication(root: &Path) {
        let held = GATES.lock().unwrap().remove(root);
        if let Some((entered, release)) = held {
            let _ = entered.send(());
            let _ = release.recv();
        }
    }
}
