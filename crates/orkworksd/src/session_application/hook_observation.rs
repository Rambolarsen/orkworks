//! Legacy identity observation follow-up. The identity response remains authoritative
//! for identity only; external telemetry failures are best-effort diagnostics.
use super::*;
use crate::metadata::hook_observation::{Admission, Stamp};
use crate::runtime::session_runtime::{RuntimeCommand, RuntimeIdentity};

struct Runtime {
    identity: RuntimeIdentity,
    control: tokio::sync::mpsc::Sender<RuntimeCommand>,
    created: String,
}
/// Internal receipt authority, never a wire or persisted capability.
pub(crate) struct Receipt {
    pub(crate) stamp: Stamp,
    pub(crate) generation: u64,
}
pub(crate) struct Capture {
    workspace: RecommendationDeliveryWorkspace,
    stamp: Stamp,
    created: String,
    runtime: Option<Runtime>,
    token: Option<String>,
}
#[derive(Debug)]
pub(crate) enum Error {
    Session(SessionError),
    Busy,
    Io(std::io::Error),
    Unavailable,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Session(error) => write!(f, "{error:?}"),
            Self::Busy => f.write_str("observation file is busy"),
            Self::Io(error) => write!(f, "{error}"),
            Self::Unavailable => f.write_str("observation task did not return a definitive result"),
        }
    }
}
impl Capture {
    pub(crate) fn authenticate(&mut self, token: &str) {
        self.token = Some(token.to_owned());
    }
}

/// Read admission/task failures are distinct from a missing legacy record.
#[derive(Debug)]
pub(crate) enum TargetReadError {
    Busy,
    Unavailable,
    Conflict,
}

/// Capture original authority before the first async boundary. Only owned file
/// reads escape application guards; identity mutation keeps its existing owner.
pub(crate) async fn capture_target(
    state: &AppState,
    id: &str,
    requested: bool,
    original: Option<Receipt>,
) -> Result<(Option<String>, Option<Result<Capture, SessionError>>), TargetReadError> {
    let (workspace, reader, lease, stamp, runtime) = {
        let workspace = state.workspace.lock().unwrap();
        let Some(ws) = workspace.as_ref() else {
            return Ok((None, requested.then_some(Err(SessionError::Conflict))));
        };
        let runtime = state.sessions.lock().unwrap().get(id).map(|h| Runtime {
            identity: h.runtime.identity(),
            control: h.runtime.control_tx.clone(),
            created: h.info.created_at.clone(),
        });
        if original.as_ref().is_some_and(|receipt| {
            runtime
                .as_ref()
                .is_none_or(|runtime| runtime.identity.run_generation != receipt.generation)
        }) {
            return Err(TargetReadError::Conflict);
        }
        let stamp = requested.then(|| {
            original
                .map(|r| r.stamp)
                .unwrap_or_else(|| ws.metadata.hook_observation_stamp())
        });
        (
            RecommendationDeliveryWorkspace::capture(ws),
            ws.metadata.session_reader(),
            ws.lease.clone(),
            stamp,
            runtime,
        )
    };
    let permit = reader.try_admit().ok_or(TargetReadError::Busy)?;
    let read_owner = reader.clone();
    let read_id = id.to_owned();
    let session = tokio::task::spawn_blocking(move || {
        // Both ownership and bounded admission survive waiter cancellation.
        let _lease = lease;
        let _permit = permit;
        #[cfg(test)]
        testing::before_read(read_owner.root());
        read_owner.read(&read_id)
    })
    .await
    .map_err(|_| TargetReadError::Unavailable)?;
    {
        let selected = state.workspace.lock().unwrap();
        let Some(ws) = selected.as_ref() else {
            return Err(TargetReadError::Conflict);
        };
        if !workspace.matches(ws) || !Arc::ptr_eq(&reader, &ws.metadata.session_reader()) {
            return Err(TargetReadError::Conflict);
        }
        let sessions = state.sessions.lock().unwrap();
        let current = sessions.get(id);
        let same_runtime = match (&runtime, current) {
            (Some(old), Some(h)) => {
                old.identity == h.runtime.identity()
                    && old.control.same_channel(&h.runtime.control_tx)
                    && old.created == h.info.created_at
            }
            (None, None) => true,
            _ => false,
        };
        if !same_runtime {
            return Err(TargetReadError::Conflict);
        }
    }
    if runtime
        .as_ref()
        .zip(session.as_ref())
        .is_some_and(|(runtime, meta)| runtime.created != meta.created_at)
    {
        return Err(TargetReadError::Conflict);
    }
    let target = session.as_ref().map(|meta| meta.harness.clone());
    let capture = stamp.map(|stamp| {
        let meta = session.as_ref().ok_or(SessionError::NotFound)?;
        if meta.harness != "codex" {
            return Err(SessionError::BadRequest(
                "hook fingerprint requires a Codex session",
            ));
        }
        Ok(Capture {
            workspace,
            stamp,
            created: meta.created_at.clone(),
            runtime,
            token: None,
        })
    });
    Ok((target, capture))
}
impl SessionApplication {
    pub(crate) async fn record_codex_hook_observation(
        &self,
        id: &str,
        fingerprint: &str,
        capture: Result<Capture, SessionError>,
    ) -> Result<(), Error> {
        let capture = capture.map_err(Error::Session)?;
        if !metadata::valid_hook_fingerprint(fingerprint) {
            return Err(Error::Session(SessionError::BadRequest(
                "invalid hook fingerprint",
            )));
        }
        // This is the existing synchronous validation cutoff. Only short memory
        // claims escape it; no external observation-file wait occurs here.
        let (claim, lease) = {
            let _projection = self.state.projection_lock.lock().unwrap();
            let workspace = self.state.workspace.lock().unwrap();
            let ws = workspace
                .as_ref()
                .ok_or(Error::Session(SessionError::Conflict))?;
            if !capture.workspace.matches(ws)
                || !capture
                    .stamp
                    .belongs_to(&ws.metadata.hook_observation_owner())
            {
                return Ok(());
            }
            // Ordinary JSON identity validation retains its current owner. This
            // slice moves the external observation write, not all metadata reads.
            let meta = ws
                .metadata
                .read_session(id)
                .ok_or(Error::Session(SessionError::NotFound))?;
            if meta.created_at != capture.created || meta.harness != "codex" {
                return Ok(());
            }
            let sessions = self.state.sessions.lock().unwrap();
            let current = sessions.get(id);
            let runtime_current = match (&capture.runtime, current) {
                (Some(old), Some(h)) => {
                    old.identity == h.runtime.identity()
                        && old.control.same_channel(&h.runtime.control_tx)
                        && h.info.created_at == capture.created
                }
                (None, None) => true,
                _ => false,
            };
            if !runtime_current
                || capture.token.as_ref().is_some_and(|token| {
                    !crate::runtime::terminal_runtime::verify_workflow_report_token(id, token)
                })
            {
                return Ok(());
            }
            let lease = ws
                .lease
                .clone()
                .ok_or(Error::Session(SessionError::Conflict))?;
            match capture.stamp.try_claim() {
                Admission::Claimed(claim) => (claim, lease),
                Admission::Obsolete => return Ok(()),
                Admission::Busy => return Err(Error::Busy),
            }
        };
        let observation = metadata::CodexHookObservation {
            fingerprint: fingerprint.to_owned(),
            observed_at: iso_now(),
        };
        // Once claimed, the blocking task retains definitive completion and the
        // original workspace lease even when its HTTP waiter disappears.
        let dispatcher = tracing::dispatcher::get_default(Clone::clone);
        let session_id = id.to_owned();
        tokio::task::spawn_blocking(move || {
            tracing::dispatcher::with_default(&dispatcher, || {
                let _lease = lease;
                let result = claim.write(&observation);
                if let Err(error) = &result {
                    // This owner outlives a cancelled waiter; report the definitive
                    // failure here rather than relying on its HTTP future surviving.
                    tracing::warn!(%session_id, %error, "failed to record Codex hook observation");
                }
                result
            })
        })
        .await
        .map_err(|_| Error::Unavailable)?
        .map_err(Error::Io)
    }
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
    }
    impl Pause {
        pub(crate) async fn entered(&mut self) {
            self.entered.take().unwrap().await.unwrap();
        }
    }
    impl Drop for Pause {
        fn drop(&mut self) {
            if let Some(release) = self.release.take() {
                let _ = release.send(());
            }
        }
    }
    pub(crate) fn hold_next(root: PathBuf) -> Pause {
        let (entered_tx, entered) = oneshot::channel();
        let (release, released) = std::sync::mpsc::channel();
        assert!(GATES
            .lock()
            .unwrap()
            .insert(root, (entered_tx, released))
            .is_none());
        Pause {
            entered: Some(entered),
            release: Some(release),
        }
    }
    pub(super) fn before_read(root: &Path) {
        let gate = GATES.lock().unwrap().remove(root);
        if let Some((entered, released)) = gate {
            let _ = entered.send(());
            let _ = released.recv();
        }
    }
}
