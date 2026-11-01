use super::internal_mutations::InternalMutationResult;
use super::queries::{HistoryQuery, Page, Summary};
use super::*;
use crate::metadata::{
    internal_mutations::InternalMutation, session_transactions::MetadataRevision,
};
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
    Internal {
        id: String,
        created: String,
        expected: Option<MetadataRevision>,
        proof: Option<super::AttentionProof>,
        intent: Box<InternalMutation>,
    },
    Get(String),
    OwnedAttention {
        id: String,
        created: String,
    },
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
            | Self::OwnedAttention { .. }
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
            // Canonical record plus JSON-escaped creation/session proof and decision.
            Self::Internal { .. } | Self::OwnedAttention { .. } => MAX_RECORD + 4096,
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
            Self::Internal {
                id,
                created,
                expected,
                proof,
                intent,
            } => Ok(Reply::Internal(db.mutate_internal_owned(
                &id,
                &created,
                expected.as_ref(),
                proof.as_ref(),
                &intent,
            )?)),
            Self::OwnedAttention { id, created } => Ok(Reply::OwnedAttention(
                db.read_owned_attention(&id, &created)?,
            )),
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
    Internal(InternalMutationResult),
    OwnedAttention(Option<(serde_json::Value, super::AttentionProof)>),
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
    execution: Option<oneshot::Sender<ExecutionRequest>>,
    resources: Option<RuntimeWriteResources>,
}
struct Delivered {
    result: Result<Reply>,
    _permit: Reservation,
    resources: Option<RuntimeWriteResources>,
}
struct Ticket {
    shared: std::sync::Weak<Shared>,
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
        if self.state.load(Ordering::Acquire) == 2 {
            if let Some(shared) = self.shared.upgrade() {
                shared.retire_cancelled(&self.state);
            }
        }
    }
}
struct PreparedJob {
    shared: Arc<Shared>,
    job: Job,
    ticket: Ticket,
}
pub(crate) struct MutationCancellation {
    state: Arc<AtomicU8>,
}
impl Drop for MutationCancellation {
    fn drop(&mut self) {
        let _ = self
            .state
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire);
    }
}
pub(crate) struct ExecutionGrant {
    decision: Option<std::sync::mpsc::Sender<ExecutionDecision>>,
}
impl ExecutionGrant {
    /// Only call after releasing the application currentness guards.
    pub(crate) fn execute(mut self) -> Result<()> {
        self.decision
            .take()
            .ok_or(StoreError::Unavailable)?
            .send(ExecutionDecision::Run)
            .map_err(|_| StoreError::Unavailable)
    }
}
impl Drop for ExecutionGrant {
    fn drop(&mut self) {
        if let Some(decision) = self.decision.take() {
            let _ = decision.send(ExecutionDecision::Reject);
        }
    }
}
pub(crate) struct ExecutionRequest {
    decision: Option<std::sync::mpsc::Sender<ExecutionDecision>>,
    state: Arc<AtomicU8>,
    deadline: Instant,
}
enum ExecutionDecision {
    Run,
    Reject,
    Expired,
}
impl ExecutionRequest {
    /// The application calls this under its short currentness guards. This
    /// competes with Ticket cancellation at the same execution cutoff.
    pub(crate) fn grant(mut self) -> Result<ExecutionGrant> {
        if self
            .state
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
            || Instant::now() >= self.deadline
        {
            if let Some(decision) = self.decision.take() {
                let _ = decision.send(ExecutionDecision::Expired);
            }
            return Err(StoreError::Expired);
        }
        Ok(ExecutionGrant {
            decision: Some(self.decision.take().ok_or(StoreError::Unavailable)?),
        })
    }
}
impl Drop for ExecutionRequest {
    fn drop(&mut self) {
        if let Some(decision) = self.decision.take() {
            let _ = decision.send(ExecutionDecision::Reject);
        }
    }
}
impl PreparedJob {
    fn enqueue(self, gate: Option<oneshot::Sender<ExecutionRequest>>) -> Result<Ticket> {
        let Self {
            shared,
            mut job,
            ticket,
        } = self;
        let mut queue = shared.queue.lock().map_err(|_| StoreError::Unavailable)?;
        if queue.closed {
            return Err(StoreError::Unavailable);
        }
        if Instant::now() >= ticket.deadline {
            return Err(StoreError::Expired);
        }
        job.execution = gate;
        if job.command.is_read() {
            queue.reads.push_back(job);
        } else {
            queue.mutations.push_back(job);
        }
        shared.ready.notify_one();
        Ok(ticket)
    }
}
struct Reservation {
    shared: Arc<Shared>,
    bytes: usize,
    read: bool,
    pending: bool,
}
impl Reservation {
    fn executing(&mut self) {
        if self.pending {
            if let Ok(mut q) = self.shared.queue.lock() {
                if self.read {
                    q.pending_reads -= 1;
                } else {
                    q.pending_mutations -= 1;
                }
                self.pending = false;
            }
        }
    }
}
impl Drop for Reservation {
    fn drop(&mut self) {
        if let Ok(mut q) = self.shared.queue.lock() {
            q.bytes -= self.bytes;
            if self.pending {
                if self.read {
                    q.pending_reads -= 1;
                } else {
                    q.pending_mutations -= 1;
                }
            }
        }
    }
}
async fn receive_delivery(mut ticket: Ticket) -> Result<Delivered> {
    tokio::select! {
        delivered=&mut ticket.receiver=>delivered.map_err(|_|StoreError::Unavailable),
        _=tokio::time::sleep_until(ticket.deadline.into())=>{
            if ticket.state.compare_exchange(0,2,Ordering::AcqRel,Ordering::Acquire).is_ok(){return Err(StoreError::Expired);}
            // Execution has begun: retain the definitive result internally.
            (&mut ticket.receiver).await.map_err(|_|StoreError::Unavailable)
        }
    }
}
async fn receive(ticket: Ticket) -> Result<Reply> {
    receive_delivery(ticket).await?.result
}
#[derive(Default)]
struct Queue {
    mutations: VecDeque<Job>,
    reads: VecDeque<Job>,
    bytes: usize,
    pending_mutations: usize,
    pending_reads: usize,
    closed: bool,
}
struct Shared {
    queue: Mutex<Queue>,
    ready: Condvar,
    lanes: Mutex<std::collections::HashMap<String, std::sync::Weak<tokio::sync::Mutex<()>>>>,
    #[cfg(test)]
    after_claim: Mutex<Option<(String, oneshot::Sender<()>, std::sync::mpsc::Receiver<()>)>>,
}
impl Shared {
    fn retire_cancelled(&self, state: &Arc<AtomicU8>) {
        // The queued-to-executing CAS has already rejected execution. Remove
        // only this exact ticket; a dequeued/executing owner keeps its resources.
        let retired = self.queue.lock().ok().and_then(|mut queue| {
            if let Some(index) = queue
                .mutations
                .iter()
                .position(|job| Arc::ptr_eq(&job.state, state))
            {
                queue.mutations.remove(index)
            } else if let Some(index) = queue
                .reads
                .iter()
                .position(|job| Arc::ptr_eq(&job.state, state))
            {
                queue.reads.remove(index)
            } else {
                None
            }
        });
        // Reservation::drop reacquires queue; never drop a job under its mutex.
        drop(retired);
    }
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
pub(crate) struct PreparedInternalMutation {
    prepared: PreparedJob,
    id: String,
}
pub(crate) struct InternalMutationTicket {
    ticket: Ticket,
}
pub(crate) struct RuntimeWriteResources {
    _lane: tokio::sync::OwnedMutexGuard<()>,
    _lease: Option<Arc<crate::workspace_runtime::WorkspaceLease>>,
}
impl RuntimeWriteResources {
    pub(crate) fn new(
        lane: tokio::sync::OwnedMutexGuard<()>,
        lease: Option<Arc<crate::workspace_runtime::WorkspaceLease>>,
    ) -> Self {
        Self {
            _lane: lane,
            _lease: lease,
        }
    }
}
pub(crate) struct InternalCompletion {
    pub(crate) outcome: InternalMutationResult,
    _resources: Option<RuntimeWriteResources>,
    _permit: Reservation,
}
impl PreparedInternalMutation {
    pub(crate) fn cancellation(&self) -> MutationCancellation {
        MutationCancellation {
            state: self.prepared.ticket.state.clone(),
        }
    }

    pub(crate) fn lane(&self) -> Result<Arc<tokio::sync::Mutex<()>>> {
        let mut lanes = self
            .prepared
            .shared
            .lanes
            .lock()
            .map_err(|_| StoreError::Unavailable)?;
        lanes.retain(|_, lane| lane.strong_count() > 0);
        if let Some(lane) = lanes.get(&self.id).and_then(std::sync::Weak::upgrade) {
            return Ok(lane);
        }
        let lane = Arc::new(tokio::sync::Mutex::new(()));
        lanes.insert(self.id.clone(), Arc::downgrade(&lane));
        Ok(lane)
    }
    pub(crate) fn deadline(&self) -> Instant {
        self.prepared.ticket.deadline
    }
    pub(crate) fn enqueue(
        mut self,
        resources: RuntimeWriteResources,
    ) -> Result<(InternalMutationTicket, oneshot::Receiver<ExecutionRequest>)> {
        let (ready, request) = oneshot::channel();
        self.prepared.job.resources = Some(resources);
        let ticket = self.prepared.enqueue(Some(ready))?;
        Ok((InternalMutationTicket { ticket }, request))
    }
}
impl InternalMutationTicket {
    pub(crate) async fn finish(self) -> Result<InternalCompletion> {
        let Delivered {
            result,
            _permit,
            resources,
        } = receive_delivery(self.ticket).await?;
        match result? {
            Reply::Internal(outcome) => Ok(InternalCompletion {
                outcome,
                _resources: resources,
                _permit,
            }),
            _ => Err(StoreError::Unavailable),
        }
    }
}
pub(crate) struct SessionStore {
    shared: Arc<Shared>,
    finished: watch::Receiver<bool>,
    root: PathBuf,
    workspace: String,
}
impl SessionStore {
    pub(crate) async fn initialize(root: PathBuf, workspace: String) -> Result<Self> {
        Self::start(root, workspace, true).await
    }
    pub(crate) async fn open(root: PathBuf, workspace: String) -> Result<Self> {
        Self::start(root, workspace, false).await
    }
    async fn start(root: PathBuf, workspace: String, create: bool) -> Result<Self> {
        let mut startup = Self::launch(root.clone(), workspace.clone(), create)?;
        (&mut startup.ready)
            .await
            .map_err(|_| StoreError::Unavailable)??;
        startup.owns_worker = false;
        Ok(Self {
            shared: startup.shared.clone(),
            finished: startup.finished.clone(),
            root,
            workspace,
        })
    }
    fn launch(root: PathBuf, workspace: String, create: bool) -> Result<Startup> {
        let shared = Arc::new(Shared {
            queue: Mutex::new(Queue::default()),
            ready: Condvar::new(),
            lanes: Mutex::new(Default::default()),
            #[cfg(test)]
            after_claim: Mutex::new(None),
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
    fn prepare(&self, command: Command) -> Result<PreparedJob> {
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
        if (read && queue.pending_reads >= 8)
            || (!read && queue.pending_mutations >= 32)
            || queue.bytes + bytes > 32 * 1024 * 1024
        {
            return Err(StoreError::QueueFull);
        }
        queue.bytes += bytes;
        if read {
            queue.pending_reads += 1;
        } else {
            queue.pending_mutations += 1;
        }
        drop(queue);
        let permit = Reservation {
            shared: self.shared.clone(),
            bytes,
            read,
            pending: true,
        };
        let admitted = Instant::now();
        let state = Arc::new(AtomicU8::new(0));
        let job = Job {
            command,
            permit,
            admitted,
            state: state.clone(),
            reply,
            execution: None,
            resources: None,
        };
        Ok(PreparedJob {
            shared: self.shared.clone(),
            job,
            ticket: Ticket {
                shared: Arc::downgrade(&self.shared),
                receiver,
                state,
                deadline: admitted + Duration::from_secs(2),
            },
        })
    }
    fn submit(&self, command: Command) -> Result<Ticket> {
        self.prepare(command)?.enqueue(None)
    }
    async fn request(&self, command: Command) -> Result<Reply> {
        receive(self.submit(command)?).await
    }
    pub(crate) fn location(&self) -> (&std::path::Path, &str) {
        (&self.root, &self.workspace)
    }
    pub(crate) fn prepare_internal(
        &self,
        id: String,
        created: String,
        intent: InternalMutation,
    ) -> Result<PreparedInternalMutation> {
        self.prepare_internal_owned(id, created, None, None, intent)
    }
    pub(crate) fn prepare_internal_owned(
        &self,
        id: String,
        created: String,
        expected: Option<MetadataRevision>,
        proof: Option<super::AttentionProof>,
        intent: InternalMutation,
    ) -> Result<PreparedInternalMutation> {
        if id.is_empty()
            || id.len() > 256
            || created.is_empty()
            || created.len() > 256
            || !id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-_".contains(&c))
        {
            return Err(StoreError::Invalid("invalid creation identity".into()));
        }
        let prepared = self.prepare(Command::Internal {
            id: id.clone(),
            created,
            expected,
            proof,
            intent: Box::new(intent),
        })?;
        Ok(PreparedInternalMutation { prepared, id })
    }
    pub(crate) async fn mutate_internal(
        &self,
        id: String,
        created: String,
        expected: Option<MetadataRevision>,
        intent: InternalMutation,
    ) -> Result<InternalMutationResult> {
        match self
            .request(Command::Internal {
                id,
                created,
                expected,
                proof: None,
                intent: Box::new(intent),
            })
            .await?
        {
            Reply::Internal(value) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn read_owned_attention(
        &self,
        id: String,
        created: String,
    ) -> Result<Option<(serde_json::Value, super::AttentionProof)>> {
        match self
            .request(Command::OwnedAttention { id, created })
            .await?
        {
            Reply::OwnedAttention(value) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
    }
    pub(crate) async fn mutate_owned_attention(
        &self,
        id: String,
        created: String,
        proof: super::AttentionProof,
        intent: InternalMutation,
    ) -> Result<InternalMutationResult> {
        match self
            .request(Command::Internal {
                id,
                created,
                expected: None,
                proof: Some(proof),
                intent: Box::new(intent),
            })
            .await?
        {
            Reply::Internal(value) => Ok(value),
            _ => Err(StoreError::Unavailable),
        }
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
fn execute_command(_shared: &Shared, command: Command, db: &mut Database) -> Result<Reply> {
    #[cfg(test)]
    {
        let gate = {
            let mut gate = _shared.after_claim.lock().unwrap();
            if matches!(&command, Command::Internal { id, .. } if gate.as_ref().is_some_and(|(expected, ..)| expected == id))
            {
                gate.take()
            } else {
                None
            }
        };
        if let Some((_, entered, release)) = gate {
            let _ = entered.send(());
            let _ = release.recv();
        }
    }
    command.run(db)
}
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    pub(crate) struct Pause {
        release: Option<std::sync::mpsc::Sender<()>>,
        _ticket: Ticket,
    }
    impl Drop for Pause {
        fn drop(&mut self) {
            if let Some(release) = self.release.take() {
                let _ = release.send(());
            }
        }
    }
    pub(crate) fn pause(store: &SessionStore) -> Pause {
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let ticket = store
            .submit(Command::Pause(entered_tx, release_rx))
            .unwrap();
        entered_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        Pause {
            release: Some(release_tx),
            _ticket: ticket,
        }
    }
    pub(crate) async fn wait_for_internal(store: &SessionStore, id: &str) {
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                let queued = store.shared.queue.lock().unwrap().mutations.iter()
                    .any(|job| matches!(&job.command, Command::Internal { id: queued, .. } if queued == id));
                if queued { return; }
                tokio::task::yield_now().await;
            }
        }).await.expect("application mutation did not enter the worker queue");
    }
    pub(crate) struct ClaimPause {
        entered: Option<oneshot::Receiver<()>>,
        release: Option<std::sync::mpsc::Sender<()>>,
    }
    impl ClaimPause {
        pub(crate) fn started(&mut self) -> bool {
            matches!(self.entered.as_mut().unwrap().try_recv(), Ok(()))
        }
        pub(crate) async fn entered(&mut self) {
            self.entered.take().unwrap().await.unwrap();
        }
    }
    impl Drop for ClaimPause {
        fn drop(&mut self) {
            if let Some(release) = self.release.take() {
                let _ = release.send(());
            }
        }
    }
    pub(crate) fn hold_next_internal(store: &SessionStore, id: &str) -> ClaimPause {
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        *store.shared.after_claim.lock().unwrap() = Some((id.into(), entered_tx, release_rx));
        ClaimPause {
            entered: Some(entered_rx),
            release: Some(release_tx),
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
            mut permit,
            admitted,
            state,
            reply,
            execution,
            resources,
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
            let granted = execution.map(|ready| {
                let deadline = admitted + Duration::from_secs(2);
                let remaining = deadline
                    .checked_duration_since(Instant::now())
                    .ok_or(StoreError::Expired)?;
                let (decision, received) = std::sync::mpsc::channel();
                let request = ExecutionRequest {
                    decision: Some(decision),
                    state: state.clone(),
                    deadline,
                };
                ready.send(request).map_err(|_| StoreError::Expired)?;
                let decision = match received.recv_timeout(remaining) {
                    Ok(decision) => decision,
                    // A successful grant claimed execution under application
                    // guards; a descheduled sender is no longer queued work.
                    Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                        if state.load(Ordering::Acquire) == 1 =>
                    {
                        received.recv().map_err(|_| StoreError::Unavailable)?
                    }
                    Err(_) => return Err(StoreError::Expired),
                };
                match decision {
                    ExecutionDecision::Run if state.load(Ordering::Acquire) == 1 => Ok(()),
                    ExecutionDecision::Reject => Err(StoreError::Conflict),
                    _ => Err(StoreError::Expired),
                }
            });
            let result = if let Some(granted) = granted {
                granted.and_then(|()| {
                    permit.executing();
                    execute_command(shared, command, db)
                })
            } else if state
                .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
                || {
                    #[cfg(test)]
                    if let Command::CreateAtDeadlineBoundary(_, entered, release) = &command {
                        let _ = entered.send(());
                        let _ = release.recv();
                    }
                    admitted.elapsed() >= Duration::from_secs(2)
                }
            {
                Err(StoreError::Expired)
            } else {
                permit.executing();
                execute_command(shared, command, db)
            };
            permit.executing(); // rejected/expired jobs also leave the pending set
            let _ = reply.send(Delivered {
                result,
                _permit: permit,
                resources,
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
    async fn application_claim_does_not_start_sql_before_guards_are_released() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(document("after-release")).await.unwrap();
        let prepared = store
            .prepare_internal(
                "after-release".into(),
                "2026-01-01T00:00:00Z".into(),
                InternalMutation::UserTitle {
                    title: Some("after release".into()),
                },
            )
            .unwrap();
        let lane = prepared.lane().unwrap().lock_owned().await;
        let (ticket, ready) = prepared
            .enqueue(RuntimeWriteResources::new(lane, None))
            .unwrap();
        let mut started = testing::hold_next_internal(&store, "after-release");
        let claim = ready.await.unwrap().grant().unwrap();
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert!(
            !started.started(),
            "SQL started before the application released its guards"
        );
        claim.execute().unwrap();
        tokio::time::timeout(Duration::from_secs(2), started.entered())
            .await
            .unwrap();
        drop(started);
        assert!(ticket.finish().await.unwrap().outcome.document.is_some());
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn caller_cancellation_competes_with_grant_while_ticket_remains_owned() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let prepared = store
            .prepare(Command::Create(document("caller-cancel")))
            .unwrap();
        let cancellation = MutationCancellation {
            state: prepared.ticket.state.clone(),
        };
        let (ready, request) = oneshot::channel();
        let ticket = prepared.enqueue(Some(ready)).unwrap();
        let request = request.await.unwrap();
        drop(cancellation);
        assert!(request.grant().is_err());
        assert!(matches!(receive(ticket).await, Err(StoreError::Expired)));
        assert!(store.get("caller-cancel".into()).await.unwrap().is_none());
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn a_job_awaiting_grant_still_consumes_its_pending_slot() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (ready, request) = oneshot::channel();
        let ticket = store
            .prepare(Command::Create(document("waiting-grant")))
            .unwrap()
            .enqueue(Some(ready))
            .unwrap();
        let request = request.await.unwrap();
        let mut prepared = Vec::new();
        for i in 0..31 {
            prepared.push(
                store
                    .prepare(Command::Create(document(&format!("reserved-{i}"))))
                    .unwrap(),
            );
        }
        assert!(matches!(
            store.prepare(Command::Create(document("one-too-many"))),
            Err(StoreError::QueueFull)
        ));
        drop(prepared);
        request.grant().unwrap().execute().unwrap();
        receive(ticket).await.unwrap();
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn prepared_writes_share_one_lane_per_session_for_the_same_worker() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let first = store
            .prepare_internal(
                "same".into(),
                "created".into(),
                InternalMutation::UserTitle {
                    title: Some("one".into()),
                },
            )
            .unwrap();
        let second = store
            .prepare_internal(
                "same".into(),
                "created".into(),
                InternalMutation::UserTitle {
                    title: Some("two".into()),
                },
            )
            .unwrap();
        let other = store
            .prepare_internal(
                "other".into(),
                "created".into(),
                InternalMutation::UserTitle {
                    title: Some("other".into()),
                },
            )
            .unwrap();
        let held = first.lane().unwrap().lock_owned().await;
        assert!(second.lane().unwrap().try_lock_owned().is_err());
        assert!(other.lane().unwrap().try_lock_owned().is_ok());
        drop(held);
        assert!(second.lane().unwrap().try_lock_owned().is_ok());
        drop((first, second, other));
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn queued_cancellation_frees_admission_before_a_stalled_worker_resumes() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let paused = testing::pause(&store);
        let mut tickets = Vec::new();
        for i in 0..32 {
            tickets.push(
                store
                    .submit(Command::Create(document(&format!("queued-{i}"))))
                    .unwrap(),
            );
        }
        assert!(matches!(
            store.submit(Command::Create(document("over-capacity"))),
            Err(StoreError::QueueFull)
        ));
        drop(tickets.pop());
        let replacement = store.submit(Command::Create(document("replacement")));
        let admitted = replacement.is_ok();
        drop(replacement);
        drop(tickets);
        drop(paused);
        assert!(
            admitted,
            "cancelled queued work retained admission behind the stalled worker"
        );
        assert!(store.get("queued-31".into()).await.unwrap().is_none());
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn prepared_jobs_reserve_queue_slots_and_release_them_when_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut prepared = Vec::new();
        for i in 0..32 {
            prepared.push(
                store
                    .prepare(Command::Create(document(&format!("prepared-{i}"))))
                    .unwrap(),
            );
        }
        assert!(matches!(
            store.submit(Command::Create(document("over-capacity"))),
            Err(StoreError::QueueFull)
        ));
        assert!(store.get("prepared-0".into()).await.unwrap().is_none());
        drop(prepared);
        store.create(document("after-drop")).await.unwrap();
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn prepared_internal_results_consume_the_existing_total_byte_budget() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let mut prepared = Vec::new();
        loop {
            let command = Command::Internal {
                id: "budget".into(),
                created: "created".into(),
                expected: None,
                proof: None,
                intent: Box::new(InternalMutation::UserTitle {
                    title: Some("title".into()),
                }),
            };
            match store.prepare(command) {
                Ok(job) => prepared.push(job),
                Err(StoreError::QueueFull) => break,
                Err(error) => panic!("{error:?}"),
            }
        }
        assert!(
            prepared.len() < 32,
            "result reservations must exhaust bytes before slots"
        );
        assert!(!prepared.is_empty());
        drop(prepared);
        store.create(document("budget-released")).await.unwrap();
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn prepared_deadline_includes_time_before_enqueue() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let prepared = store
            .prepare(Command::Create(document("expired-before-queue")))
            .unwrap();
        tokio::time::sleep(Duration::from_millis(2050)).await;
        assert!(matches!(prepared.enqueue(None), Err(StoreError::Expired)));
        assert!(store
            .get("expired-before-queue".into())
            .await
            .unwrap()
            .is_none());
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn denied_execution_grant_never_changes_the_record() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let prepared = store.prepare(Command::Create(document("denied"))).unwrap();
        let (ready, request) = oneshot::channel();
        let ticket = prepared.enqueue(Some(ready)).unwrap();
        drop(request.await.unwrap());
        assert!(matches!(receive(ticket).await, Err(StoreError::Conflict)));
        assert!(store.get("denied".into()).await.unwrap().is_none());
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn cancellation_can_win_the_same_claim_as_execution_grant() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let prepared = store
            .prepare(Command::Create(document("cancel-before-grant")))
            .unwrap();
        let (ready, request) = oneshot::channel();
        let ticket = prepared.enqueue(Some(ready)).unwrap();
        let request = request.await.unwrap();
        drop(ticket);
        assert!(request.grant().is_err());
        assert!(store
            .get("cancel-before-grant".into())
            .await
            .unwrap()
            .is_none());
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn granted_execution_is_not_reported_as_queued_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let (entered_tx, entered_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let prepared = store
            .prepare(Command::CreatePaused(
                document("granted"),
                entered_tx,
                release_rx,
            ))
            .unwrap();
        let (ready, request) = oneshot::channel();
        let ticket = prepared.enqueue(Some(ready)).unwrap();
        request.await.unwrap().grant().unwrap().execute().unwrap();
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let receipt = tokio::spawn(receive(ticket));
        tokio::time::sleep(Duration::from_millis(2050)).await;
        assert!(
            !receipt.is_finished(),
            "an executing grant must retain the real result"
        );
        release_tx.send(()).unwrap();
        assert!(matches!(receipt.await.unwrap(), Ok(Reply::Unit)));
        assert!(store.get("granted".into()).await.unwrap().is_some());
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn late_application_grant_cannot_execute_an_expired_preparation() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let prepared = store
            .prepare(Command::Create(document("late-grant")))
            .unwrap();
        let (ready, request) = oneshot::channel();
        let ticket = prepared.enqueue(Some(ready)).unwrap();
        let request = request.await.unwrap();
        tokio::time::sleep(Duration::from_millis(2050)).await;
        assert!(request.grant().is_err());
        assert!(matches!(receive(ticket).await, Err(StoreError::Expired)));
        assert!(store.get("late-grant".into()).await.unwrap().is_none());
        store.close().await.unwrap();
    }
    #[tokio::test]
    async fn close_rejects_unqueued_preparations_without_late_writes() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        let prepared = store
            .prepare(Command::Create(document("closed-before-enqueue")))
            .unwrap();
        store.close().await.unwrap();
        assert!(matches!(
            prepared.enqueue(None),
            Err(StoreError::Unavailable)
        ));
        let reopened = SessionStore::open(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        assert!(reopened
            .get("closed-before-enqueue".into())
            .await
            .unwrap()
            .is_none());
        reopened.close().await.unwrap();
    }
    #[tokio::test]
    async fn executing_worker_retains_lane_and_lease_after_its_result_owner_is_dropped() {
        let dir = tempfile::tempdir().unwrap();
        let store = SessionStore::initialize(dir.path().into(), "/workspace".into())
            .await
            .unwrap();
        store.create(document("retained")).await.unwrap();
        let prepared = store
            .prepare_internal(
                "retained".into(),
                "2026-01-01T00:00:00Z".into(),
                InternalMutation::UserTitle {
                    title: Some("retained title".into()),
                },
            )
            .unwrap();
        let lane = prepared.lane().unwrap();
        let held = lane.clone().lock_owned().await;
        let lease =
            Arc::new(crate::workspace_runtime::WorkspaceLease::acquire(dir.path()).unwrap());
        let resources = RuntimeWriteResources::new(held, Some(lease));
        let mut claimed = testing::hold_next_internal(&store, "retained");
        let (ticket, ready) = prepared.enqueue(resources).unwrap();
        ready.await.unwrap().grant().unwrap().execute().unwrap();
        tokio::time::timeout(Duration::from_secs(2), claimed.entered())
            .await
            .unwrap();
        drop(ticket);
        assert!(lane.clone().try_lock_owned().is_err());
        assert!(crate::workspace_runtime::WorkspaceLease::acquire(dir.path()).is_err());
        drop(claimed);
        assert_eq!(
            store.get("retained".into()).await.unwrap().unwrap()["label"],
            "retained title"
        );
        assert!(lane.try_lock_owned().is_ok());
        assert!(crate::workspace_runtime::WorkspaceLease::acquire(dir.path()).is_ok());
        store.close().await.unwrap();
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
