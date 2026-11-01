use super::queries::{HistoryQuery, Page, Summary};
use super::*;
use serde::Serialize;
use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Condvar, Mutex,
    },
    time::Instant,
};
use tokio::sync::{oneshot, watch};

#[derive(Serialize)]
enum Command {
    Create(serde_json::Value),
    Get(String),
    History(HistoryQuery),
    Live(Vec<String>),
    Exists(Vec<String>),
    Retention(i64, usize),
    Patch {
        id: String,
        envelope: serde_json::Value,
    },
    Organization {
        id: String,
        created: String,
        revision: String,
        choice: String,
    },
    BeginDelete {
        id: String,
        created: String,
        revision: String,
        organization_revision: String,
    },
    DeletionProgress {
        id: String,
        created: String,
        progress: u8,
        complete: bool,
    },
    #[cfg(test)]
    #[serde(skip)]
    Pause(std::sync::mpsc::Sender<()>, std::sync::mpsc::Receiver<()>),
    #[cfg(test)]
    #[serde(skip)]
    CreateAtDeadlineBoundary(
        serde_json::Value,
        std::sync::mpsc::Sender<()>,
        std::sync::mpsc::Receiver<()>,
    ),
    #[cfg(test)]
    #[serde(skip)]
    CreateAtCutoff(
        serde_json::Value,
        std::sync::mpsc::Sender<()>,
        std::sync::mpsc::Receiver<()>,
    ),
    #[cfg(test)]
    #[serde(skip)]
    CreatePaused(
        serde_json::Value,
        std::sync::mpsc::Sender<()>,
        std::sync::mpsc::Receiver<()>,
    ),
}
impl Command {
    fn is_read(&self) -> bool {
        match self {
            Self::Get(_)
            | Self::History(_)
            | Self::Live(_)
            | Self::Exists(_)
            | Self::Retention(..) => true,
            #[cfg(test)]
            Self::Pause(..) => true,
            _ => false,
        }
    }
    fn size(&self) -> Result<usize> {
        #[cfg(test)]
        match self {
            Self::Pause(..) => return Ok(0),
            Self::CreatePaused(value, ..)
            | Self::CreateAtCutoff(value, ..)
            | Self::CreateAtDeadlineBoundary(value, ..) => {
                return Ok(serde_json::to_vec(value)?.len())
            }
            _ => {}
        }
        Ok(serde_json::to_vec(self)?.len())
    }
    fn result_bound(&self) -> usize {
        match self {
            Self::Get(_) | Self::Patch { .. } => MAX_RECORD,
            Self::History(_) => 256 * 1024,
            Self::Live(ids) => ids.len().saturating_mul(8192).saturating_add(16384),
            Self::Exists(ids) => ids.iter().map(|id| id.len() + 4).sum(),
            Self::Retention(..) => 256 * 1024,
            Self::Organization { .. } => 32,
            _ => 0,
        }
    }
    fn run(self, db: &mut Database) -> Result<Reply> {
        match self {
            Self::Create(value) => {
                db.create_session(value)?;
                Ok(Reply::Unit)
            }
            Self::Get(id) => Ok(Reply::Detail(db.get(&id)?)),
            Self::History(query) => Ok(Reply::Page(db.history(query)?)),
            Self::Live(ids) => Ok(Reply::Rows(db.live(&ids)?)),
            Self::Exists(ids) => Ok(Reply::Ids(db.exists(&ids)?)),
            Self::Retention(before, limit) => Ok(Reply::Candidates(db.retention(before, limit)?)),
            Self::Patch { id, envelope } => Ok(Reply::Detail(Some(db.patch(&id, envelope)?))),
            Self::Organization {
                id,
                created,
                revision,
                choice,
            } => Ok(Reply::Revision(
                db.organize(&id, &created, &revision, &choice)?,
            )),
            Self::BeginDelete {
                id,
                created,
                revision,
                organization_revision,
            } => {
                db.begin_delete(&id, &created, &revision, &organization_revision)?;
                Ok(Reply::Unit)
            }
            Self::DeletionProgress {
                id,
                created,
                progress,
                complete,
            } => {
                db.deletion_progress(&id, &created, progress, complete)?;
                Ok(Reply::Unit)
            }
            #[cfg(test)]
            Self::Pause(entered, release) => {
                let _ = entered.send(());
                release.recv().map_err(|_| StoreError::Cancelled)?;
                Ok(Reply::Unit)
            }
            #[cfg(test)]
            Self::CreateAtDeadlineBoundary(value, ..) => {
                db.create_session(value)?;
                Ok(Reply::Unit)
            }
            #[cfg(test)]
            Self::CreateAtCutoff(value, ..) => {
                db.create_session(value)?;
                Ok(Reply::Unit)
            }
            #[cfg(test)]
            Self::CreatePaused(value, entered, release) => {
                let _ = entered.send(());
                release.recv().map_err(|_| StoreError::Cancelled)?;
                db.create_session(value)?;
                Ok(Reply::Unit)
            }
        }
    }
}
#[derive(Debug)]
enum Reply {
    Unit,
    Detail(Option<serde_json::Value>),
    Page(Page),
    Rows(Vec<Summary>),
    Ids(Vec<String>),
    Candidates(Vec<(String, String)>),
    Revision(String),
}
struct Job {
    command: Command,
    permit: Reservation,
    admitted: Instant,
    state: Arc<AtomicU8>,
    reply: oneshot::Sender<Delivered>,
}
struct Delivered {
    result: Result<Reply>,
    _permit: Reservation,
}
struct Ticket {
    receiver: oneshot::Receiver<Delivered>,
    state: Arc<AtomicU8>,
    deadline: Instant,
}
impl Drop for Ticket {
    fn drop(&mut self) {
        // Compete with the worker's exact queued-to-executing cutoff.
        // Once executing, disconnection never asserts rollback.
        let _ = self
            .state
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
    }
}
struct Reservation {
    shared: Arc<Shared>,
    bytes: usize,
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if let Ok(mut q) = self.shared.queue.lock() {
            q.bytes -= self.bytes;
        }
    }
}
async fn receive(mut ticket: Ticket) -> Result<Reply> {
    tokio::select! {
        delivered=&mut ticket.receiver=>delivered.map_err(|_|StoreError::Unavailable)?.result,
        _=tokio::time::sleep_until(ticket.deadline.into())=>{
            if ticket.state.compare_exchange(0,2,Ordering::AcqRel,Ordering::Acquire).is_ok(){return Err(StoreError::Expired);}
            // Execution has begun: retain the definitive result internally.
            // A transport timeout cannot be represented as a proven rollback.
            (&mut ticket.receiver).await.map_err(|_|StoreError::Unavailable)?.result
        }
    }
}
#[derive(Default)]
struct Queue {
    mutations: VecDeque<Job>,
    reads: VecDeque<Job>,
    bytes: usize,
    closed: bool,
}
struct Shared {
    queue: Mutex<Queue>,
    ready: Condvar,
}
struct Startup {
    shared: Arc<Shared>,
    finished: watch::Receiver<bool>,
    ready: oneshot::Receiver<Result<()>>,
    owns_worker: bool,
}
impl Drop for Startup {
    fn drop(&mut self) {
        if self.owns_worker {
            if let Ok(mut queue) = self.shared.queue.lock() {
                queue.closed = true;
                self.shared.ready.notify_one();
            }
        }
    }
}
pub(crate) struct SessionStore {
    shared: Arc<Shared>,
    finished: watch::Receiver<bool>,
}
impl SessionStore {
    pub(crate) async fn initialize(root: PathBuf, workspace: String) -> Result<Self> {
        Self::start(root, workspace, true).await
    }
    pub(crate) async fn open(root: PathBuf, workspace: String) -> Result<Self> {
        Self::start(root, workspace, false).await
    }
    async fn start(root: PathBuf, workspace: String, create: bool) -> Result<Self> {
        let mut startup = Self::launch(root, workspace, create)?;
        (&mut startup.ready)
            .await
            .map_err(|_| StoreError::Unavailable)??;
        startup.owns_worker = false;
        Ok(Self {
            shared: startup.shared.clone(),
            finished: startup.finished.clone(),
        })
    }
    fn launch(root: PathBuf, workspace: String, create: bool) -> Result<Startup> {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            ready: Condvar::new(),
        });
        let (ready_tx, ready_rx) = oneshot::channel();
        let (finished_tx, finished) = watch::channel(false);
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("session-store".into())
            .spawn(move || {
                let path = root.join("session-store").join("v1.sqlite");
                let db = if create {
                    std::fs::create_dir_all(root.join("session-store"))
                        .map_err(StoreError::from)
                        .and_then(|_| Database::create(&path, &workspace))
                } else {
                    Database::open(&path, &workspace)
                };
                match db {
                    Ok(mut db) => {
                        if ready_tx.send(Ok(())).is_ok() {
                            run_worker(&worker, &mut db);
                        }
                    }
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                    }
                }
                // Connection is dropped before maintenance/shutdown can continue.
                let _ = finished_tx.send(true);
            })?;
        Ok(Startup {
            shared,
            finished,
            ready: ready_rx,
            owns_worker: true,
        })
    }
    fn submit(&self, command: Command) -> Result<Ticket> {
        let intent = command.size()?;
        if intent > MAX_RECORD {
            return Err(StoreError::TooLarge);
        }
        let bytes = intent
            .checked_add(command.result_bound())
            .ok_or(StoreError::TooLarge)?;
        if bytes > 32 * 1024 * 1024 {
            return Err(StoreError::TooLarge);
        }
        let (reply, receiver) = oneshot::channel();
        let read = command.is_read();
        let mut queue = self
            .shared
            .queue
            .lock()
            .map_err(|_| StoreError::Unavailable)?;
        if queue.closed {
            return Err(StoreError::Unavailable);
        }
        if (read && queue.reads.len() >= 8)
            || (!read && queue.mutations.len() >= 32)
            || queue.bytes + bytes > 32 * 1024 * 1024
        {
            return Err(StoreError::QueueFull);
        }
        queue.bytes += bytes;
        let permit = Reservation {
            shared: self.shared.clone(),
            bytes,
        };
        let admitted = Instant::now();
        let state = Arc::new(AtomicU8::new(0));
        let job = Job {
            command,
            permit,
            admitted,
            state: state.clone(),
            reply,
        };
        if read {
            queue.reads.push_back(job);
        } else {
            queue.mutations.push_back(job);
        }
        self.shared.ready.notify_one();
        Ok(Ticket {
            receiver,
            state,
            deadline: admitted + Duration::from_secs(2),
        })
    }
    async fn request(&self, command: Command) -> Result<Reply> {
        receive(self.submit(command)?).await
    }
    pub(crate) async fn create(&self, value: serde_json::Value) -> Result<()> {
        match self.request(Command::Create(value)).await? {
            Reply::Unit => Ok(()),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn get(&self, id: String) -> Result<Option<serde_json::Value>> {
        match self.request(Command::Get(id)).await? {
            Reply::Detail(value) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn history(&self, query: HistoryQuery) -> Result<Page> {
        match self.request(Command::History(query)).await? {
            Reply::Page(value) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn live(&self, ids: Vec<String>) -> Result<Vec<Summary>> {
        match self.request(Command::Live(ids)).await? {
            Reply::Rows(value) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn exists(&self, ids: Vec<String>) -> Result<Vec<String>> {
        match self.request(Command::Exists(ids)).await? {
            Reply::Ids(value) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn retention(
        &self,
        before: i64,
        limit: usize,
    ) -> Result<Vec<(String, String)>> {
        match self.request(Command::Retention(before, limit)).await? {
            Reply::Candidates(value) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn patch(
        &self,
        id: String,
        envelope: serde_json::Value,
    ) -> Result<serde_json::Value> {
        match self.request(Command::Patch { id, envelope }).await? {
            Reply::Detail(Some(value)) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn organize(
        &self,
        id: String,
        created: String,
        revision: String,
        choice: String,
    ) -> Result<String> {
        match self
            .request(Command::Organization {
                id,
                created,
                revision,
                choice,
            })
            .await?
        {
            Reply::Revision(value) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn begin_delete(
        &self,
        id: String,
        created: String,
        revision: String,
        organization_revision: String,
    ) -> Result<()> {
        match self
            .request(Command::BeginDelete {
                id,
                created,
                revision,
                organization_revision,
            })
            .await?
        {
            Reply::Unit => Ok(()),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn deletion_progress(
        &self,
        id: String,
        created: String,
        progress: u8,
        complete: bool,
    ) -> Result<()> {
        match self
            .request(Command::DeletionProgress {
                id,
                created,
                progress,
                complete,
            })
            .await?
        {
            Reply::Unit => Ok(()),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn close(&self) -> Result<()> {
        {
            let mut queue = self
                .shared
                .queue
                .lock()
                .map_err(|_| StoreError::Unavailable)?;
            queue.closed = true;
            self.shared.ready.notify_one();
        }
        let mut finished = self.finished.clone();
        while !*finished.borrow_and_update() {
            finished
                .changed()
                .await
                .map_err(|_| StoreError::Unavailable)?;
        }
        Ok(())
    }
}
impl Drop for SessionStore {
    fn drop(&mut self) {
        if let Ok(mut q) = self.shared.queue.lock() {
            q.closed = true;
            self.shared.ready.notify_one();
        }
    }
}
fn run_worker(shared: &Shared, db: &mut Database) {
    let mut burst = 0;
    loop {
        let job = {
            let mut q = match shared.queue.lock() {
                Ok(q) => q,
                Err(_) => return,
            };
            while q.mutations.is_empty() && q.reads.is_empty() && !q.closed {
                q = match shared.ready.wait(q) {
                    Ok(q) => q,
                    Err(_) => return,
                };
            }
            if q.mutations.is_empty() && q.reads.is_empty() && q.closed {
                return;
            }
            if !q.mutations.is_empty() && (burst < 8 || q.reads.is_empty()) {
                burst += 1;
                q.mutations.pop_front()
            } else {
                burst = 0;
                q.reads.pop_front()
            }
        };
        let Some(job) = job else {
            continue;
        };
        let Job {
            command,
            permit,
            admitted,
            state,
            reply,
        } = job;
        // Hold the reservation through execution and unread result delivery.
        // A dropped receiver before execution proves that this job made no change.
        if !reply.is_closed() {
            #[cfg(test)]
            if let Command::CreateAtCutoff(_, entered, release) = &command {
                let _ = entered.send(());
                let _ = release.recv();
            }
            // Claim before the deadline check so a scheduler pause cannot
            // turn a stale age observation into a late execution admission.
            let result = if state
                .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
                || {
                    #[cfg(test)]
                    if let Command::CreateAtDeadlineBoundary(_, entered, release) = &command {
                        let _ = entered.send(());
                        let _ = release.recv();
                    }
                    admitted.elapsed() >= Duration::from_secs(2)
                } {
                Err(StoreError::Expired)
            } else {
                command.run(db)
            };
            let _ = reply.send(Delivered {
                result,
                _permit: permit,
            });
        }
    }
}
#[cfg(test)]
mod tests {
    use super::super::tests::document;
    use super::*;
    fn pause(store: &SessionStore) -> (Ticket, std::sync::mpsc::Sender<()>) {
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let receiver = store
            .submit(Command::Pause(entered_tx, release_rx))
            .unwrap();
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        (receiver, release_tx)
    }
    #[tokio::test]
    async fn admission_rejects_overload_without_waiting_for_stalled_storage() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (paused, release) = pause(&store);
        let mut tickets = Vec::new();
        for i in 0..32 {
            tickets.push(
                store
                    .submit(Command::Create(document(&format!("queued-{i}"))))
                    .unwrap(),
            );
        }
        assert!(matches!(
            store.submit(Command::Create(document("excess"))),
            Err(StoreError::QueueFull)
        ));
        for _ in 0..8 {
            tickets.push(store.submit(Command::Get("missing".into())).unwrap());
        }
        assert!(matches!(
            store.submit(Command::Get("excess".into())),
            Err(StoreError::QueueFull)
        ));
        drop(tickets);
        release.send(()).unwrap();
        receive(paused).await.unwrap();
        store.close().await.unwrap();
        let reopened = SessionStore::open(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        assert!(reopened.get("queued-0".into()).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn byte_admission_and_intent_bounds_apply_before_storage_execution() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (paused, release) = pause(&store);
        let mut tickets = Vec::new();
        let mut huge = document("huge");
        huge["extra"] = serde_json::json!("x".repeat(MAX_RECORD));
        assert!(matches!(
            store.submit(Command::Create(huge)),
            Err(StoreError::TooLarge)
        ));
        let mut accepted = 0;
        for i in 0..32 {
            let mut value = document(&format!("large-{i}"));
            value["extra"] = serde_json::json!("x".repeat(MAX_RECORD - 5000));
            match store.submit(Command::Patch {
                id: format!("large-{i}"),
                envelope: value,
            }) {
                Ok(ticket) => {
                    tickets.push(ticket);
                    accepted += 1;
                }
                Err(StoreError::QueueFull) => break,
                Err(e) => panic!("{e:?}"),
            }
        }
        assert!(
            accepted > 0 && accepted < 32,
            "request plus possible result memory must fit 32 MiB"
        );
        drop(tickets);
        release.send(()).unwrap();
        receive(paused).await.unwrap();
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn queue_expiry_and_cancellation_before_execution_make_no_durable_change() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (paused, release) = pause(&store);
        let cancelled = store
            .submit(Command::Create(document("cancelled")))
            .unwrap();
        drop(cancelled);
        let expired = store.submit(Command::Create(document("expired"))).unwrap();
        tokio::time::sleep(Duration::from_millis(2100)).await;
        release.send(()).unwrap();
        receive(paused).await.unwrap();
        assert!(matches!(receive(expired).await, Err(StoreError::Expired)));
        assert!(store.get("cancelled".into()).await.unwrap().is_none());
        assert!(store.get("expired".into()).await.unwrap().is_none());
    }
    #[tokio::test]
    async fn cancellation_after_execution_begins_does_not_claim_rollback() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let ticket = store
            .submit(Command::CreatePaused(
                document("committed"),
                entered_tx,
                release_rx,
            ))
            .unwrap();
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(ticket);
        release_tx.send(()).unwrap();
        store.close().await.unwrap();
        let reopened = SessionStore::open(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        assert_eq!(
            reopened.get("committed".into()).await.unwrap().unwrap()["metadataRevision"],
            "1"
        );
    }
    #[tokio::test]
    async fn queued_read_runs_after_eight_fifo_mutations() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (paused, release) = pause(&store);
        let mut tickets = Vec::new();
        let ids: Vec<_> = (0..10).map(|i| format!("id-{i}")).collect();
        for id in &ids {
            tickets.push(store.submit(Command::Create(document(id))).unwrap());
        }
        let read = store.submit(Command::Exists(ids)).unwrap();
        release.send(()).unwrap();
        receive(paused).await.unwrap();
        match receive(read).await.unwrap() {
            Reply::Ids(found) => {
                assert_eq!(found, (0..8).map(|i| format!("id-{i}")).collect::<Vec<_>>())
            }
            other => panic!("{other:?}"),
        };
        for ticket in tickets {
            receive(ticket).await.unwrap();
        }
    }
    #[tokio::test]
    async fn queued_deadline_returns_while_worker_is_still_stalled() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (paused, release) = pause(&store);
        let queued = store.submit(Command::Create(document("expired"))).unwrap();
        let result = tokio::time::timeout(Duration::from_millis(2500), receive(queued)).await;
        release.send(()).unwrap();
        receive(paused).await.unwrap();
        assert!(matches!(result, Ok(Err(StoreError::Expired))), "{result:?}");
        assert!(store.get("expired".into()).await.unwrap().is_none());
    }
    #[cfg(unix)]
    #[tokio::test]
    async fn actual_pty_input_and_output_continue_during_storage_stall() {
        use std::io::{Read, Write};
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (paused, release) = pause(&store);
        let pair = portable_pty::native_pty_system()
            .openpty(portable_pty::PtySize {
                rows: 24,
                cols: 80,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        #[cfg(unix)]
        let command = portable_pty::CommandBuilder::new("/bin/cat");
        #[cfg(windows)]
        let command = portable_pty::CommandBuilder::new("cmd.exe");
        let mut child = pair.slave.spawn_command(command).unwrap();
        drop(pair.slave);
        let mut reader = pair.master.try_clone_reader().unwrap();
        let mut writer = pair.master.take_writer().unwrap();
        let (tx, rx) = std::sync::mpsc::channel();
        let reader_thread = std::thread::spawn(move || {
            let mut buf = [0; 4096];
            let mut captured = Vec::new();
            let result = loop {
                match reader.read(&mut buf) {
                    Ok(0) => break Ok(captured),
                    Ok(n) => {
                        captured.extend_from_slice(&buf[..n]);
                        if captured.windows(17).any(|s| s == b"independent-input")
                            || captured.len() > 16384
                        {
                            break Ok(captured);
                        }
                    }
                    Err(e) => break Err(e),
                }
            };
            let _ = tx.send(result);
        });
        writer.write_all(b"independent-input\n").unwrap();
        writer.flush().unwrap();
        let observed = rx.recv_timeout(Duration::from_secs(2));
        let _ = child.kill();
        let _ = child.wait();
        drop(writer);
        drop(pair.master);
        reader_thread.join().unwrap();
        release.send(()).unwrap();
        receive(paused).await.unwrap();
        let bytes = observed.unwrap().unwrap();
        assert!(bytes.windows(17).any(|s| s == b"independent-input"));
    }
    #[tokio::test]
    async fn malformed_creation_returns_error_without_killing_the_storage_worker() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        assert!(store
            .create(serde_json::json!(["not an object"]))
            .await
            .is_err());
        store.create(document("valid-after-error")).await.unwrap();
        assert!(store
            .get("valid-after-error".into())
            .await
            .unwrap()
            .is_some());
    }
    #[tokio::test]
    async fn cancelled_startup_after_ready_closes_the_unowned_worker() {
        let dir = tempfile::tempdir().unwrap();
        let mut startup =
            SessionStore::launch(dir.path().into(), "/workspace".into(), true).unwrap();
        (&mut startup.ready).await.unwrap().unwrap();
        let mut finished = startup.finished.clone();
        // This is the suspension window after the ready send succeeds but before
        // start transfers ownership into a returned SessionStore.
        drop(startup);
        let closed = tokio::time::timeout(Duration::from_secs(1), async {
            while !*finished.borrow_and_update() {
                finished.changed().await.unwrap();
            }
        })
        .await;
        assert!(
            closed.is_ok(),
            "cancelled startup left an unowned connection/worker alive"
        );
    }
    #[tokio::test]
    async fn cancellation_between_liveness_check_and_execution_claim_cannot_commit() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let ticket = store
            .submit(Command::CreateAtCutoff(
                document("cancelled-at-cutoff"),
                entered_tx,
                release_rx,
            ))
            .unwrap();
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        drop(ticket);
        release_tx.send(()).unwrap();
        store.close().await.unwrap();
        let reopened = SessionStore::open(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        assert!(reopened
            .get("cancelled-at-cutoff".into())
            .await
            .unwrap()
            .is_none());
    }
    #[test]
    fn retention_reservation_covers_exact_json_escaped_identity_results() {
        let dir = tempfile::tempdir().unwrap();
        let mut db = Database::create(&dir.path().join("v1.sqlite"), "/workspace").unwrap();
        for i in 0..100 {
            let mut value =
                super::super::tests::closed_document(&format!("{}-{i:03}", "x".repeat(252)));
            value["createdAt"] = serde_json::json!("\u{1}".repeat(256));
            db.insert(value).unwrap();
        }
        let rows = db.retention(i64::MAX, 100).unwrap();
        assert_eq!(rows.len(), 100);
        let encoded = serde_json::to_vec(&rows).unwrap().len();
        assert!(
            Command::Retention(i64::MAX, 100).result_bound() >= encoded,
            "reservation undercounts {encoded} encoded result bytes"
        );
    }
    #[tokio::test]
    async fn worker_cannot_claim_an_expired_mutation_when_caller_timer_is_unpolled() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let mut ticket = store
            .submit(Command::CreateAtDeadlineBoundary(
                document("expired-at-claim"),
                entered_tx,
                release_rx,
            ))
            .unwrap();
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        tokio::time::sleep(Duration::from_millis(2100)).await;
        release_tx.send(()).unwrap();
        // Poll only delivery: do not run receive()'s timeout task, so the worker
        // itself must enforce expiry after a pause between its two predicates.
        let delivered = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                match ticket.receiver.try_recv() {
                    Ok(value) => break value,
                    Err(oneshot::error::TryRecvError::Empty) => tokio::task::yield_now().await,
                    Err(e) => panic!("{e:?}"),
                }
            }
        })
        .await
        .unwrap();
        assert!(matches!(delivered.result, Err(StoreError::Expired)));
        drop(delivered);
        assert!(store
            .get("expired-at-claim".into())
            .await
            .unwrap()
            .is_none());
    }
    #[tokio::test]
    async fn worker_commits_before_acknowledgement_and_reopens_without_json_mirror() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(document("one")).await.unwrap();
        assert_eq!(
            store.get("one".into()).await.unwrap().unwrap()["metadataRevision"],
            "1"
        );
        drop(store);
        // Opening the same persisted row verifies durability independently of an in-memory cache.
        let reopened = SessionStore::open(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        assert_eq!(
            reopened.get("one".into()).await.unwrap().unwrap()["id"],
            "one"
        );
        assert!(!dir.path().join("sessions").exists());
    }
}
