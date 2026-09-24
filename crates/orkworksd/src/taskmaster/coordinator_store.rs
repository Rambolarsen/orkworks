//! Durable coordinator definitions and approval state. No runtime authority lives here.

use super::coordinator::{CoordinatorError, PlanApproval, PlanRevision, PlanStatus};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MAX_RECORD_BYTES: usize = 2 * 1024 * 1024 + 8192;

#[derive(Debug)]
pub(crate) enum CoordinatorStoreError {
    Io(io::Error),
    Invalid(String),
    Stale,
}

impl std::fmt::Display for CoordinatorStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Invalid(reason) => write!(f, "invalid coordinator record: {reason}"),
            Self::Stale => write!(f, "stale or conflicting coordinator record"),
        }
    }
}

impl std::error::Error for CoordinatorStoreError {}

impl From<io::Error> for CoordinatorStoreError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<CoordinatorError> for CoordinatorStoreError {
    fn from(error: CoordinatorError) -> Self {
        Self::Invalid(format!("{error:?}"))
    }
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub(crate) struct StoredPlan {
    pub plan: PlanRevision,
    pub status: PlanStatus,
    pub approval: Option<PlanApproval>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DiskRecord {
    plan: serde_json::Value,
    status: PlanStatus,
    approval: Option<PlanApproval>,
}

pub(crate) struct CoordinatorStore {
    dir: PathBuf,
    mutation: Mutex<()>,
}

impl CoordinatorStore {
    pub(crate) fn open(root: PathBuf) -> Result<Self, CoordinatorStoreError> {
        let dir = root.join("coordinator/plans");
        fs::create_dir_all(&dir)?;
        let store = Self {
            dir,
            mutation: Mutex::new(()),
        };
        store.recover()?;
        Ok(store)
    }

    pub(crate) fn put_proposed(&self, plan: &PlanRevision) -> Result<(), CoordinatorStoreError> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| CoordinatorStoreError::Stale)?;
        self.recover_inner()?;
        plan.validate()?;
        let (id, digest) = identity(plan)?;
        let records = self.read_all()?;
        let latest = records
            .iter()
            .filter(|record| {
                identity(&record.plan)
                    .ok()
                    .is_some_and(|(other, _)| other == id)
            })
            .map(|record| record.plan.revision())
            .max();
        if let Some(current) = records.iter().find(|record| {
            identity(&record.plan)
                .ok()
                .is_some_and(|(other, _)| other == id)
                && record.plan.revision() == plan.revision()
        }) {
            return if identity(&current.plan)?.1 == digest
                && current.plan == *plan
                && current.status == PlanStatus::Proposed
            {
                Ok(())
            } else {
                Err(CoordinatorStoreError::Stale)
            };
        }
        if latest.is_some_and(|revision| plan.revision() <= revision) {
            return Err(CoordinatorStoreError::Stale);
        }
        let record = StoredPlan {
            plan: plan.clone(),
            status: PlanStatus::Proposed,
            approval: None,
        };
        self.publish(&id, plan.revision(), None, &record)
    }

    pub(crate) fn get(
        &self,
        plan_id: &str,
        revision: u64,
    ) -> Result<Option<StoredPlan>, CoordinatorStoreError> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| CoordinatorStoreError::Stale)?;
        self.recover_inner()?;
        safe_id(plan_id)?;
        if revision == 0 {
            return Err(CoordinatorStoreError::Invalid("revision".into()));
        }
        self.read_at(plan_id, revision)
    }

    pub(crate) fn activate(
        &self,
        approval: &PlanApproval,
    ) -> Result<StoredPlan, CoordinatorStoreError> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| CoordinatorStoreError::Stale)?;
        self.recover_inner()?;
        safe_id(&approval.plan_id)?;
        let records = self.read_all()?;
        if records
            .iter()
            .filter(|record| {
                identity(&record.plan)
                    .ok()
                    .is_some_and(|(id, _)| id == approval.plan_id)
            })
            .any(|record| record.plan.revision() > approval.revision)
        {
            return Err(CoordinatorStoreError::Stale);
        }
        let mut record = self
            .read_at(&approval.plan_id, approval.revision)?
            .ok_or(CoordinatorStoreError::Stale)?;
        if record.status != PlanStatus::Proposed
            || approval.plan_digest != identity(&record.plan)?.1
        {
            return Err(CoordinatorStoreError::Stale);
        }
        record
            .status
            .can_activate(approval, &record.plan, approval.revocation_generation)
            .map_err(|_| CoordinatorStoreError::Stale)?;
        let previous = record.clone();
        record.status = PlanStatus::Active;
        record.approval = Some(approval.clone());
        self.publish(
            &approval.plan_id,
            approval.revision,
            Some(&previous),
            &record,
        )?;
        Ok(record)
    }

    pub(crate) fn transition(
        &self,
        plan_id: &str,
        revision: u64,
        expected_digest: &str,
        next: PlanStatus,
    ) -> Result<StoredPlan, CoordinatorStoreError> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| CoordinatorStoreError::Stale)?;
        self.recover_inner()?;
        safe_id(plan_id)?;
        let mut record = self
            .read_at(plan_id, revision)?
            .ok_or(CoordinatorStoreError::Stale)?;
        if identity(&record.plan)?.1 != expected_digest || !record.status.allows_transition(next) {
            return Err(CoordinatorStoreError::Stale);
        }
        let previous = record.clone();
        record.status = next;
        self.publish(plan_id, revision, Some(&previous), &record)?;
        Ok(record)
    }

    pub(crate) fn recover(&self) -> Result<(), CoordinatorStoreError> {
        let _guard = self
            .mutation
            .lock()
            .map_err(|_| CoordinatorStoreError::Stale)?;
        self.recover_inner()
    }

    fn recover_inner(&self) -> Result<(), CoordinatorStoreError> {
        self.read_all()?;
        for plan_dir in fs::read_dir(&self.dir)? {
            let plan_dir = plan_dir?;
            if !plan_dir.file_type()?.is_dir() {
                continue;
            }
            for entry in fs::read_dir(plan_dir.path())? {
                let entry = entry?;
                if entry.file_name().to_string_lossy().ends_with(".json.tmp") {
                    if !entry.file_type()?.is_file() {
                        return Err(CoordinatorStoreError::Invalid("temporary file type".into()));
                    }
                    fs::remove_file(entry.path())?;
                    sync_dir(&plan_dir.path())?;
                }
            }
        }
        Ok(())
    }

    fn read_all(&self) -> Result<Vec<StoredPlan>, CoordinatorStoreError> {
        let mut records = Vec::new();
        for plan_dir in fs::read_dir(&self.dir)? {
            let plan_dir = plan_dir?;
            if !plan_dir.file_type()?.is_dir() {
                return Err(CoordinatorStoreError::Invalid("plan directory type".into()));
            }
            let id = plan_dir
                .file_name()
                .into_string()
                .map_err(|_| CoordinatorStoreError::Invalid("plan directory name".into()))?;
            safe_id(&id)?;
            for entry in fs::read_dir(plan_dir.path())? {
                let entry = entry?;
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| CoordinatorStoreError::Invalid("revision filename".into()))?;
                if name.ends_with(".json.tmp") {
                    continue;
                }
                let revision = name
                    .strip_suffix(".json")
                    .and_then(|value| value.parse::<u64>().ok())
                    .ok_or_else(|| CoordinatorStoreError::Invalid("revision filename".into()))?;
                if name != format!("{revision}.json")
                    || revision == 0
                    || !entry.file_type()?.is_file()
                {
                    return Err(CoordinatorStoreError::Invalid("revision file type".into()));
                }
                records.push(read_record(&entry.path(), &id, revision)?);
            }
        }
        Ok(records)
    }

    fn read_at(
        &self,
        id: &str,
        revision: u64,
    ) -> Result<Option<StoredPlan>, CoordinatorStoreError> {
        let path = self.path(id, revision);
        match fs::metadata(&path) {
            Ok(_) => read_record(&path, id, revision).map(Some),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn path(&self, id: &str, revision: u64) -> PathBuf {
        self.dir.join(id).join(format!("{revision}.json"))
    }

    fn publish(
        &self,
        id: &str,
        revision: u64,
        expected: Option<&StoredPlan>,
        record: &StoredPlan,
    ) -> Result<(), CoordinatorStoreError> {
        let path = self.path(id, revision);
        let parent = path
            .parent()
            .ok_or_else(|| CoordinatorStoreError::Invalid("record path".into()))?;
        fs::create_dir_all(parent)?;
        let bytes = serde_json::to_vec(record)
            .map_err(|error| CoordinatorStoreError::Invalid(error.to_string()))?;
        if bytes.len() > MAX_RECORD_BYTES {
            return Err(CoordinatorStoreError::Invalid("record size".into()));
        }
        // A stale snapshot cannot replace an externally changed record.
        if self.read_at(id, revision)?.as_ref() != expected {
            return Err(CoordinatorStoreError::Stale);
        }
        let temp = path.with_extension("json.tmp");
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temp)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        crate::harness::integration::atomic_replace(&temp, &path, path.exists())?;
        sync_dir(parent)?;
        Ok(())
    }
}

fn read_record(path: &Path, id: &str, revision: u64) -> Result<StoredPlan, CoordinatorStoreError> {
    if !fs::symlink_metadata(path)?.file_type().is_file() {
        return Err(CoordinatorStoreError::Invalid("record file type".into()));
    }
    let file = File::open(path)?;
    if file.metadata()?.len() > MAX_RECORD_BYTES as u64 {
        return Err(CoordinatorStoreError::Invalid("record size".into()));
    }
    let mut bytes = Vec::new();
    file.take(MAX_RECORD_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_RECORD_BYTES {
        return Err(CoordinatorStoreError::Invalid("record size".into()));
    }
    let disk: DiskRecord = serde_json::from_slice(&bytes)
        .map_err(|error| CoordinatorStoreError::Invalid(error.to_string()))?;
    let plan_bytes = serde_json::to_vec(&disk.plan)
        .map_err(|error| CoordinatorStoreError::Invalid(error.to_string()))?;
    let plan = PlanRevision::from_persisted_bytes(&plan_bytes)?;
    let (stored_id, _) = identity(&plan)?;
    if stored_id != id || plan.revision() != revision {
        return Err(CoordinatorStoreError::Invalid(
            "record path mismatch".into(),
        ));
    }
    if matches!(disk.status, PlanStatus::Draft)
        || (disk.status == PlanStatus::Proposed && disk.approval.is_some())
    {
        return Err(CoordinatorStoreError::Invalid("approval state".into()));
    }
    if let Some(approval) = &disk.approval {
        let approved_at = approval
            .approved_at
            .parse()
            .map_err(|_| CoordinatorStoreError::Invalid("approval timestamp".into()))?;
        approval.validate_against_at(&plan, approved_at)?;
    } else if matches!(
        disk.status,
        PlanStatus::Active
            | PlanStatus::ReadyForUserReview
            | PlanStatus::Completed
            | PlanStatus::Failed
    ) {
        return Err(CoordinatorStoreError::Invalid("missing approval".into()));
    }
    Ok(StoredPlan {
        plan,
        status: disk.status,
        approval: disk.approval,
    })
}

fn identity(plan: &PlanRevision) -> Result<(String, String), CoordinatorStoreError> {
    let value = serde_json::to_value(plan)
        .map_err(|error| CoordinatorStoreError::Invalid(error.to_string()))?;
    let id = value["plan_id"]
        .as_str()
        .ok_or_else(|| CoordinatorStoreError::Invalid("plan id".into()))?;
    safe_id(id)?;
    Ok((id.to_owned(), plan.compute_plan_digest()?))
}

fn safe_id(id: &str) -> Result<(), CoordinatorStoreError> {
    if id.is_empty()
        || id.len() > 128
        || !id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
        || id == "."
        || id == ".."
    {
        return Err(CoordinatorStoreError::Invalid("plan id".into()));
    }
    Ok(())
}

fn sync_dir(path: &Path) -> Result<(), CoordinatorStoreError> {
    if let Ok(dir) = File::open(path) {
        dir.sync_all()?;
    }
    Ok(())
}
