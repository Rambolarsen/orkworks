//! Workspace-owned external observation file. SQL never treats this as session metadata.
use super::CodexHookObservation;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
struct State {
    epoch: Arc<()>,
    busy: bool,
}
pub(crate) struct Store {
    root: PathBuf,
    state: Mutex<State>,
    available: Condvar,
}
#[derive(Clone)]
pub(crate) struct Stamp {
    owner: Arc<Store>,
    epoch: Arc<()>,
}
pub(crate) enum Admission {
    Claimed(Claim),
    Busy,
    Obsolete,
}
/// Owned logical exclusion, so application guards can be dropped before file I/O.
pub(crate) struct Claim {
    owner: Arc<Store>,
}
impl Store {
    pub(crate) fn new(root: &Path) -> Arc<Self> {
        Arc::new(Self {
            root: root.to_owned(),
            state: Mutex::new(State {
                epoch: Arc::new(()),
                busy: false,
            }),
            available: Condvar::new(),
        })
    }
    pub(crate) fn stamp(self: &Arc<Self>) -> Stamp {
        Stamp {
            owner: self.clone(),
            epoch: self.state.lock().unwrap().epoch.clone(),
        }
    }
    fn claim_legacy(self: &Arc<Self>, clear: bool) -> Claim {
        let mut state = self.state.lock().unwrap();
        if clear {
            state.epoch = Arc::new(());
        }
        while state.busy {
            state = self.available.wait(state).unwrap();
        }
        // Also revoke captures made while an earlier claimed writer was draining.
        if clear {
            state.epoch = Arc::new(());
        }
        state.busy = true;
        Claim {
            owner: self.clone(),
        }
    }
    pub(crate) fn write(
        self: &Arc<Self>,
        observation: &CodexHookObservation,
    ) -> std::io::Result<()> {
        self.claim_legacy(false).write(observation)
    }
    pub(crate) fn clear(self: &Arc<Self>) -> std::io::Result<()> {
        let _claim = self.claim_legacy(true);
        match std::fs::remove_file(self.root.join("codex-hook-observation.json")) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    }
}
impl std::fmt::Debug for Stamp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HookObservationStamp")
    }
}
impl Stamp {
    pub(crate) fn belongs_to(&self, owner: &Arc<Store>) -> bool {
        Arc::ptr_eq(&self.owner, owner)
    }

    /// Fail fast while another external effect owns this workspace file.
    pub(crate) fn try_claim(self) -> Admission {
        let mut state = self.owner.state.lock().unwrap();
        if !Arc::ptr_eq(&state.epoch, &self.epoch) {
            return Admission::Obsolete;
        }
        if state.busy {
            return Admission::Busy;
        }
        state.busy = true;
        drop(state);
        Admission::Claimed(Claim { owner: self.owner })
    }
}
impl Claim {
    pub(crate) fn write(&self, observation: &CodexHookObservation) -> std::io::Result<()> {
        if !super::valid_hook_fingerprint(&observation.fingerprint) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid hook fingerprint",
            ));
        }
        #[cfg(test)]
        let _finished = testing::before_write(&self.owner.root);
        std::fs::create_dir_all(&self.owner.root)?;
        let json = serde_json::to_string_pretty(observation)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        super::write_atomic(&self.owner.root.join("codex-hook-observation.json"), &json)
    }
}
impl Drop for Claim {
    fn drop(&mut self) {
        self.owner.state.lock().unwrap().busy = false;
        self.owner.available.notify_all();
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use std::collections::HashMap;
    use std::sync::LazyLock;
    use tokio::sync::oneshot;
    type Gate = (
        oneshot::Sender<()>,
        std::sync::mpsc::Receiver<()>,
        oneshot::Sender<()>,
    );
    static GATES: LazyLock<Mutex<HashMap<PathBuf, Gate>>> =
        LazyLock::new(|| Mutex::new(HashMap::new()));
    pub(crate) struct Pause {
        entered: Option<oneshot::Receiver<()>>,
        finished: Option<oneshot::Receiver<()>>,
        release: Option<std::sync::mpsc::Sender<()>>,
    }
    impl Pause {
        pub(crate) async fn entered(&mut self) {
            self.entered.take().unwrap().await.unwrap();
        }
        pub(crate) async fn finish(mut self) {
            let _ = self.release.take().unwrap().send(());
            self.finished.take().unwrap().await.unwrap();
        }
    }
    impl Drop for Pause {
        fn drop(&mut self) {
            if let Some(release) = self.release.take() {
                let _ = release.send(());
            }
        }
    }
    pub(crate) struct Finished(Option<oneshot::Sender<()>>);
    impl Drop for Finished {
        fn drop(&mut self) {
            if let Some(sent) = self.0.take() {
                let _ = sent.send(());
            }
        }
    }
    pub(crate) fn hold_next_write(root: PathBuf) -> Pause {
        let (entered_tx, entered) = oneshot::channel();
        let (release, release_rx) = std::sync::mpsc::channel();
        let (finished_tx, finished) = oneshot::channel();
        assert!(GATES
            .lock()
            .unwrap()
            .insert(root, (entered_tx, release_rx, finished_tx))
            .is_none());
        Pause {
            entered: Some(entered),
            finished: Some(finished),
            release: Some(release),
        }
    }
    pub(super) fn before_write(root: &Path) -> Finished {
        let gate = GATES.lock().unwrap().remove(root);
        if let Some((entered, release, finished)) = gate {
            let _ = entered.send(());
            let _ = release.recv();
            Finished(Some(finished))
        } else {
            Finished(None)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sql_trust_clear_revokes_captured_work_and_follows_an_already_claimed_write() {
        let dir = tempfile::tempdir().unwrap();
        let owner = Store::new(dir.path());
        let old = owner.stamp();
        let stamp = owner.stamp();
        let Admission::Claimed(claim) = stamp.try_claim() else {
            panic!("expected claim");
        };
        let clearing = owner.clone();
        let pending = std::thread::spawn(move || clearing.clear().unwrap());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while Arc::ptr_eq(&old.epoch, &owner.state.lock().unwrap().epoch) {
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(matches!(old.try_claim(), Admission::Obsolete));
        assert!(matches!(owner.stamp().try_claim(), Admission::Busy));
        claim
            .write(&CodexHookObservation {
                fingerprint: "a".repeat(64),
                observed_at: "2026-01-01T00:00:00Z".into(),
            })
            .unwrap();
        drop(claim);
        pending.join().unwrap();
        assert!(!dir.path().join("codex-hook-observation.json").exists());
        assert!(matches!(owner.stamp().try_claim(), Admission::Claimed(_)));
    }
}
