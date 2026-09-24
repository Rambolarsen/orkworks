use super::completion::CompletionMutationRequest;
use super::rollup::stable_rollup_id;
use super::{DismissalWatermark, Recommendation, RecommendationStatus, RecommendationType};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const ROLLUP_TRANSACTION_DIR: &str = ".rollup-transactions";
const ROLLUP_TRANSACTION_MANIFEST: &str = "manifest.json";
const ROLLUP_TRANSACTION_VERSION: u32 = 2;
const LEGACY_ROLLUP_TRANSACTION_VERSION: u32 = 1;

#[derive(Debug)]
pub(crate) enum StoreError {
    Io(io::Error),
    Json(serde_json::Error),
    InvalidTransition,
    StaleExpectedHash { id: String },
    StalePacket { id: String },
    GraphInvariant(String),
    Recovery(String),
}

impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::InvalidTransition => {
                write!(f, "recommendation is not a proposed workflow improvement")
            }
            Self::StaleExpectedHash { id } => {
                write!(f, "recommendation file changed since it was read: {id}")
            }
            Self::StalePacket { id } => {
                write!(f, "completion packet changed since it was read: {id}")
            }
            Self::GraphInvariant(message) => write!(f, "recommendation graph invariant: {message}"),
            Self::Recovery(message) => write!(f, "recommendation recovery unavailable: {message}"),
        }
    }
}

impl std::error::Error for StoreError {}

pub(crate) struct RecommendationStore {
    dir: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RollupTransactionManifest {
    version: u32,
    committed: bool,
    entries: Vec<RollupTransactionEntry>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RollupTransactionEntry {
    id: String,
    target: String,
    staged: Option<String>,
    backup: Option<String>,
    old_sha256: Option<String>,
    old_exists: bool,
    new_sha256: Option<String>,
    new_exists: bool,
}

struct Replacement {
    old: Option<Vec<u8>>,
    new: Option<Vec<u8>>,
}

struct StoredRecommendation {
    recommendation: Recommendation,
    bytes: Vec<u8>,
    hash: String,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug)]
enum FaultPoint {
    Staging,
    ManifestCommit,
    Publication(usize),
    Cleanup,
}

#[cfg(test)]
thread_local! {
    static FAULT_POINT: std::cell::RefCell<Option<FaultPoint>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn set_fault_point(point: Option<FaultPoint>) {
    FAULT_POINT.with(|fault| *fault.borrow_mut() = point);
}

#[cfg(test)]
fn take_fault_point(expected: impl FnOnce(FaultPoint) -> bool) -> Result<(), StoreError> {
    FAULT_POINT.with(|fault| {
        let mut fault = fault.borrow_mut();
        if fault.as_ref().is_some_and(|point| expected(*point)) {
            *fault = None;
            Err(StoreError::Io(io::Error::other(
                "injected rollup store failure",
            )))
        } else {
            Ok(())
        }
    })
}

impl RecommendationStore {
    pub(crate) fn open(root: PathBuf) -> Result<Self, StoreError> {
        let dir = root.join("recommendations");
        fs::create_dir_all(&dir).map_err(StoreError::Io)?;
        let store = Self { dir };
        store.migrate_legacy_rollup_filenames()?;
        store.recover_transactions()?;
        store.validate_graph()?;
        Ok(store)
    }

    pub(crate) fn list(&self) -> Result<Vec<Recommendation>, StoreError> {
        self.recover_transactions()?;
        let mut recommendations = self.read_all()?;
        validate_graph_records(&recommendations)?;
        recommendations.sort_by(|left, right| {
            left.created_at
                .cmp(&right.created_at)
                .then(left.id.cmp(&right.id))
        });
        Ok(recommendations)
    }

    pub(crate) fn list_with_hashes(
        &self,
    ) -> Result<(Vec<Recommendation>, BTreeMap<String, String>), StoreError> {
        self.recover_transactions()?;
        let mut stored = self.read_all_stored()?;
        validate_graph_records(
            &stored
                .iter()
                .map(|record| record.recommendation.clone())
                .collect::<Vec<_>>(),
        )?;
        stored.sort_by(|left, right| {
            left.recommendation
                .created_at
                .cmp(&right.recommendation.created_at)
                .then(left.recommendation.id.cmp(&right.recommendation.id))
        });
        let hashes = stored
            .iter()
            .map(|record| (record.recommendation.id.clone(), record.hash.clone()))
            .collect();
        let recommendations = stored
            .into_iter()
            .map(|record| record.recommendation)
            .collect();
        Ok((recommendations, hashes))
    }

    pub(crate) fn get(&self, id: &str) -> Result<Option<Recommendation>, StoreError> {
        self.recover_transactions()?;
        self.validate_graph()?;
        if !valid_id(id) {
            return Ok(None);
        }
        let path = self.path_for(id);
        match fs::read_to_string(path) {
            Ok(json) => {
                let recommendation = serde_json::from_str(&json).map_err(StoreError::Json)?;
                validate_recommendation(&recommendation)?;
                Ok(Some(recommendation))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(StoreError::Io(error)),
        }
    }

    pub(crate) fn put(&self, recommendation: &Recommendation) -> Result<(), StoreError> {
        self.recover_transactions()?;
        validate_recommendation(recommendation)?;
        let json = serde_json::to_vec_pretty(recommendation).map_err(StoreError::Json)?;
        let path = self.path_for(&recommendation.id);
        let temp = path.with_extension("json.tmp");
        let mut file = fs::File::create(&temp).map_err(StoreError::Io)?;
        use std::io::Write;
        file.write_all(&json).map_err(StoreError::Io)?;
        file.sync_all().map_err(StoreError::Io)?;
        drop(file);
        let target_existed = path.exists();
        crate::harness::integration::atomic_replace(&temp, &path, target_existed)
            .map_err(StoreError::Io)?;
        if let Ok(directory) = fs::File::open(&self.dir) {
            let _ = directory.sync_all();
        }
        Ok(())
    }

    pub(crate) fn apply_rollup_transaction(
        &self,
        expected: &BTreeMap<String, Option<String>>,
        parent: &Recommendation,
        members: &[Recommendation],
    ) -> Result<(), StoreError> {
        self.recover_transactions()?;
        self.validate_graph()?;
        let stored = self.read_all_stored_by_id()?;
        let current = stored
            .iter()
            .map(|(id, record)| (id.clone(), record.recommendation.clone()))
            .collect::<BTreeMap<_, _>>();
        self.verify_expected(expected, &current)?;

        let member_ids = members
            .iter()
            .map(|member| member.id.clone())
            .collect::<BTreeSet<_>>();
        if !valid_id(&parent.id)
            || parent.status != RecommendationStatus::Proposed
            || parent.rolled_up_by.is_some()
            || member_ids.is_empty()
            || member_ids.len() != members.len()
            || member_ids.contains(&parent.id)
            || member_ids.iter().any(|id| !valid_id(id))
            || parent.rollup_member_ids.iter().any(|id| !valid_id(id))
            || member_ids
                != parent
                    .rollup_member_ids
                    .iter()
                    .cloned()
                    .collect::<BTreeSet<_>>()
            || parent.rollup_member_ids.len() != member_ids.len()
        {
            return Err(StoreError::GraphInvariant(
                "rollup parent and members do not describe one complete graph".into(),
            ));
        }
        if member_ids.iter().any(|id| !current.contains_key(id)) {
            return Err(StoreError::GraphInvariant(
                "every rollup member must already exist".into(),
            ));
        }
        if let Some(existing_parent) = current.get(&parent.id) {
            if existing_parent.status != RecommendationStatus::Proposed
                || existing_parent.rollup_member_ids.is_empty()
            {
                return Err(StoreError::InvalidTransition);
            }
        }

        let mut parent_record = parent.clone();
        parent_record.rollup_member_ids = member_ids.iter().cloned().collect();
        parent_record.rollup_member_dedupe_keys = members
            .iter()
            .map(|member| (member.id.clone(), member.dedupe_key.clone()))
            .collect::<BTreeMap<_, _>>()
            .into_values()
            .collect();
        parent_record.rollup_member_dedupe_keys.sort();

        let can_reparent_from = parent_record
            .workflow_improvement
            .supersedes_recommendation_id
            .as_deref()
            .filter(|old_parent_id| {
                current
                    .get(*old_parent_id)
                    .is_some_and(|old_parent| old_parent.status == RecommendationStatus::Proposed)
            });

        let mut replacements = BTreeMap::new();
        replacements.insert(
            parent_record.id.clone(),
            serde_json::to_vec_pretty(&parent_record).map_err(StoreError::Json)?,
        );
        for member in members {
            if !member.rollup_member_ids.is_empty() {
                return Err(StoreError::GraphInvariant(format!(
                    "member {} cannot also be a rollup parent",
                    member.id
                )));
            }
            if member.recommendation_type != RecommendationType::ImproveWorkflow
                || !matches!(
                    member.status,
                    RecommendationStatus::Proposed | RecommendationStatus::RolledUp
                )
                || member
                    .rolled_up_by
                    .as_deref()
                    .is_some_and(|id| id != parent.id && Some(id) != can_reparent_from)
                || member.workflow_improvement.target_surface
                    != parent.workflow_improvement.target_surface
            {
                return Err(StoreError::InvalidTransition);
            }
            if let Some(existing) = current.get(&member.id) {
                if !matches!(
                    existing.status,
                    RecommendationStatus::Proposed | RecommendationStatus::RolledUp
                ) && existing.id != parent.id
                {
                    return Err(StoreError::InvalidTransition);
                }
                if existing
                    .rolled_up_by
                    .as_deref()
                    .is_some_and(|id| id != parent.id && Some(id) != can_reparent_from)
                {
                    return Err(StoreError::GraphInvariant(format!(
                        "member {} already belongs to another current parent",
                        member.id
                    )));
                }
            }
            let mut member_record = member.clone();
            member_record.status = RecommendationStatus::RolledUp;
            member_record.rolled_up_by = Some(parent.id.clone());
            replacements.insert(
                member_record.id.clone(),
                serde_json::to_vec_pretty(&member_record).map_err(StoreError::Json)?,
            );
        }

        if let Some(old_parent_id) = parent_record
            .workflow_improvement
            .supersedes_recommendation_id
            .as_deref()
        {
            if old_parent_id == parent_record.id {
                return Err(StoreError::GraphInvariant(
                    "a rollup cannot supersede itself".into(),
                ));
            }
            if let Some(old_parent) = current.get(old_parent_id) {
                if old_parent.status == RecommendationStatus::Proposed
                    && !old_parent.rollup_member_ids.is_empty()
                {
                    let mut superseded = old_parent.clone();
                    superseded.status = RecommendationStatus::Superseded;
                    superseded.updated_at = parent_record.updated_at.clone();
                    replacements.insert(
                        superseded.id.clone(),
                        serde_json::to_vec_pretty(&superseded).map_err(StoreError::Json)?,
                    );
                    for old_member_id in &old_parent.rollup_member_ids {
                        if member_ids.contains(old_member_id) {
                            continue;
                        }
                        let Some(old_member) = current.get(old_member_id) else {
                            return Err(StoreError::GraphInvariant(format!(
                                "superseded parent references missing member {old_member_id}"
                            )));
                        };
                        if old_member.rolled_up_by.as_deref() == Some(old_parent_id) {
                            let mut released = old_member.clone();
                            released.status = RecommendationStatus::Proposed;
                            released.rolled_up_by = None;
                            replacements.insert(
                                released.id.clone(),
                                serde_json::to_vec_pretty(&released).map_err(StoreError::Json)?,
                            );
                        }
                    }
                } else if matches!(
                    old_parent.status,
                    RecommendationStatus::Accepted
                        | RecommendationStatus::Completed
                        | RecommendationStatus::Dismissed
                        | RecommendationStatus::Expired
                        | RecommendationStatus::Failed
                ) {
                    // Terminal graphs are immutable history. A successor may
                    // point at one, but it must not release or re-parent it.
                } else if old_parent.rollup_member_ids.is_empty() {
                    return Err(StoreError::GraphInvariant(
                        "superseded recommendation is not a rollup parent".into(),
                    ));
                }
            }
        }

        let replacements: BTreeMap<String, Replacement> = replacements
            .into_iter()
            .map(|(id, new)| {
                let old = stored.get(&id).map(|record| record.bytes.clone());
                (
                    id,
                    Replacement {
                        old,
                        new: Some(new),
                    },
                )
            })
            .collect();
        let mut preview = current.clone();
        for (id, replacement) in &replacements {
            let bytes = replacement
                .new
                .as_ref()
                .expect("rollup replacements have new content");
            let recommendation = serde_json::from_slice(bytes).map_err(StoreError::Json)?;
            preview.insert(id.clone(), recommendation);
        }
        validate_graph_records(&preview.values().cloned().collect::<Vec<_>>())?;
        self.commit_replacements(expected, replacements)
    }

    /// Atomically publishes a complete recommendation graph assembled by the
    /// evaluator. The evaluator owns the in-memory projection; this boundary
    /// owns optimistic concurrency, graph validation, and durable publication.
    pub(crate) fn apply_recommendation_graph_transaction(
        &self,
        expected: &BTreeMap<String, Option<String>>,
        records: &[Recommendation],
    ) -> Result<(), StoreError> {
        self.recover_transactions()?;
        self.validate_graph()?;
        let stored = self.read_all_stored_by_id()?;
        let current = stored
            .iter()
            .map(|(id, record)| (id.clone(), record.recommendation.clone()))
            .collect::<BTreeMap<_, _>>();
        self.verify_expected(expected, &current)?;
        let next = records
            .iter()
            .cloned()
            .map(|record| (record.id.clone(), record))
            .collect::<BTreeMap<_, _>>();
        if next.len() != records.len() {
            return Err(StoreError::GraphInvariant(
                "recommendation graph contains duplicate IDs".into(),
            ));
        }
        validate_graph_records(&next.values().cloned().collect::<Vec<_>>())?;
        for (id, old) in &current {
            if !next.contains_key(id) {
                return Err(StoreError::GraphInvariant(
                    "rollup graph transaction cannot remove an existing record".into(),
                ));
            }
            if matches!(
                old.status,
                RecommendationStatus::Accepted
                    | RecommendationStatus::Completed
                    | RecommendationStatus::Dismissed
                    | RecommendationStatus::Superseded
                    | RecommendationStatus::Expired
                    | RecommendationStatus::Failed
            ) && next.get(id) != Some(old)
            {
                return Err(StoreError::InvalidTransition);
            }
        }
        let mut replacements = BTreeMap::new();
        for (id, record) in &next {
            let new = serde_json::to_vec_pretty(record).map_err(StoreError::Json)?;
            let old = stored.get(id).map(|previous| previous.bytes.clone());
            if old.as_ref() != Some(&new) {
                replacements.insert(
                    id.clone(),
                    Replacement {
                        old,
                        new: Some(new),
                    },
                );
            }
        }
        self.commit_replacements(expected, replacements)
    }

    pub(crate) fn dismiss(
        &self,
        id: &str,
        dismissed_at: String,
    ) -> Result<Option<Recommendation>, StoreError> {
        let Some(mut recommendation) = self.get(id)? else {
            return Ok(None);
        };
        if recommendation.recommendation_type != RecommendationType::ImproveWorkflow
            || !matches!(
                recommendation.status,
                RecommendationStatus::Proposed | RecommendationStatus::Executing
            )
        {
            return Err(StoreError::InvalidTransition);
        }
        let watermark = DismissalWatermark {
            dismissed_at: dismissed_at.clone(),
            dismissed_through_sequence: recommendation
                .evidence
                .iter()
                .map(|evidence| evidence.sequence)
                .max()
                .unwrap_or(0),
            observation_ids: recommendation
                .evidence
                .iter()
                .map(|evidence| evidence.observation_id.clone())
                .collect(),
            qualifying_count: recommendation.workflow_improvement.recurrence_count,
            highest_impact: recommendation.priority,
            affected_session_ids: recommendation
                .workflow_improvement
                .affected_session_ids
                .clone(),
        };
        recommendation.status = RecommendationStatus::Dismissed;
        recommendation.updated_at = dismissed_at;
        recommendation.workflow_improvement.dismissal_watermark = Some(watermark);
        if let Some(packet) = recommendation.completion_packet.as_mut() {
            packet.approval = None;
            packet.completion_idempotency_key = None;
        }
        self.put(&recommendation)?;
        Ok(Some(recommendation))
    }

    /// Reserves a `Proposed` recommendation for execution, synchronously
    /// (within the caller's single lock acquisition) and *before* any PTY
    /// write is attempted. This is the guard against two concurrent accept
    /// requests both reading `Proposed`, both writing to the same session's
    /// terminal, and only one of them losing the store transition after the
    /// fact — the second caller now sees `Executing` here and is rejected
    /// with `InvalidTransition` up front.
    pub(crate) fn begin_execution(
        &self,
        id: &str,
        target_session_id: String,
        started_at: String,
    ) -> Result<Option<Recommendation>, StoreError> {
        let Some(mut recommendation) = self.get(id)? else {
            return Ok(None);
        };
        if recommendation.recommendation_type != RecommendationType::ImproveWorkflow
            || recommendation.status != RecommendationStatus::Proposed
        {
            return Err(StoreError::InvalidTransition);
        }
        recommendation.status = RecommendationStatus::Executing;
        recommendation.target_session_id = Some(target_session_id);
        recommendation.updated_at = started_at;
        self.put(&recommendation)?;
        Ok(Some(recommendation))
    }

    pub(crate) fn begin_execution_checked(
        &self,
        id: &str,
        target_session_id: String,
        started_at: String,
        mutation: &CompletionMutationRequest,
        approver: String,
    ) -> Result<Option<Recommendation>, StoreError> {
        mutation.validate().map_err(StoreError::GraphInvariant)?;
        let Some(mut recommendation) = self.get(id)? else {
            return Ok(None);
        };
        if recommendation.recommendation_type != RecommendationType::ImproveWorkflow
            || recommendation.status != RecommendationStatus::Proposed
        {
            return Err(StoreError::InvalidTransition);
        }
        let Some(packet) = recommendation.completion_packet.as_mut() else {
            return Err(StoreError::InvalidTransition);
        };
        if packet.revision != mutation.packet_revision
            || packet.evidence_fingerprint != mutation.evidence_fingerprint
        {
            return Err(StoreError::StalePacket { id: id.into() });
        }
        packet.approval = Some(super::completion::CompletionApproval {
            approved_at: started_at.clone(),
            approver,
            revision: packet.revision,
            evidence_fingerprint: packet.evidence_fingerprint.clone(),
            action_fingerprint: packet.action_fingerprint(),
            idempotency_key: mutation.idempotency_key.clone(),
        });
        packet.completion_idempotency_key = None;
        recommendation.status = RecommendationStatus::Executing;
        recommendation.target_session_id = Some(target_session_id);
        recommendation.updated_at = started_at;
        self.put(&recommendation)?;
        Ok(Some(recommendation))
    }

    /// Finalizes a reservation after the PTY write succeeds.
    pub(crate) fn complete_execution(
        &self,
        id: &str,
        completed_at: String,
    ) -> Result<Option<Recommendation>, StoreError> {
        let Some(mut recommendation) = self.get(id)? else {
            return Ok(None);
        };
        if recommendation.recommendation_type != RecommendationType::ImproveWorkflow
            || recommendation.status != RecommendationStatus::Executing
        {
            return Err(StoreError::InvalidTransition);
        }
        recommendation.status = RecommendationStatus::Accepted;
        recommendation.updated_at = completed_at;
        self.put(&recommendation)?;
        Ok(Some(recommendation))
    }

    /// Marks an accepted recommendation complete after the agent reports that
    /// it acted on the recommendation and verified the result.
    pub(crate) fn complete_accepted(
        &self,
        id: &str,
        completed_at: String,
    ) -> Result<Option<Recommendation>, StoreError> {
        let Some(mut recommendation) = self.get(id)? else {
            return Ok(None);
        };
        if recommendation.recommendation_type != RecommendationType::ImproveWorkflow
            || recommendation.status != RecommendationStatus::Accepted
        {
            return Err(StoreError::InvalidTransition);
        }
        recommendation.status = RecommendationStatus::Completed;
        recommendation.updated_at = completed_at;
        self.put(&recommendation)?;
        Ok(Some(recommendation))
    }

    pub(crate) fn complete_accepted_checked(
        &self,
        id: &str,
        completed_at: String,
        mutation: &CompletionMutationRequest,
    ) -> Result<Option<Recommendation>, StoreError> {
        mutation.validate().map_err(StoreError::GraphInvariant)?;
        let Some(mut recommendation) = self.get(id)? else {
            return Ok(None);
        };
        let Some(packet) = recommendation.completion_packet.as_mut() else {
            return Err(StoreError::InvalidTransition);
        };
        if packet.revision != mutation.packet_revision
            || packet.evidence_fingerprint != mutation.evidence_fingerprint
        {
            return Err(StoreError::StalePacket { id: id.into() });
        }
        if recommendation.status == RecommendationStatus::Completed {
            return if packet.completion_idempotency_key.as_deref()
                == Some(mutation.idempotency_key.as_str())
            {
                Ok(Some(recommendation))
            } else {
                Err(StoreError::StalePacket { id: id.into() })
            };
        }
        if recommendation.recommendation_type != RecommendationType::ImproveWorkflow
            || recommendation.status != RecommendationStatus::Accepted
            || packet.approval.is_none()
        {
            return Err(StoreError::InvalidTransition);
        }
        packet.completion_idempotency_key = Some(mutation.idempotency_key.clone());
        recommendation.status = RecommendationStatus::Completed;
        recommendation.updated_at = completed_at;
        self.put(&recommendation)?;
        Ok(Some(recommendation))
    }

    /// Rolls a reservation back to `Proposed` after the PTY write fails, so
    /// the user can retry rather than being stuck.
    pub(crate) fn cancel_execution(
        &self,
        id: &str,
        cancelled_at: String,
    ) -> Result<Option<Recommendation>, StoreError> {
        let Some(mut recommendation) = self.get(id)? else {
            return Ok(None);
        };
        if recommendation.recommendation_type != RecommendationType::ImproveWorkflow
            || recommendation.status != RecommendationStatus::Executing
        {
            return Err(StoreError::InvalidTransition);
        }
        recommendation.status = RecommendationStatus::Proposed;
        recommendation.target_session_id = None;
        if let Some(packet) = recommendation.completion_packet.as_mut() {
            packet.approval = None;
            packet.completion_idempotency_key = None;
        }
        recommendation.updated_at = cancelled_at;
        self.put(&recommendation)?;
        Ok(Some(recommendation))
    }

    /// Replaces the evidence/action projection with a new immutable packet
    /// revision. The previous packet remains auditable through lineage, while
    /// any approval or completion capability is invalidated and the
    /// recommendation returns to the user-approvable proposed state.
    pub(crate) fn replace_completion_packet(
        &self,
        id: &str,
        expected_revision: u64,
        mut replacement: super::completion::CompletionPacket,
        updated_at: String,
    ) -> Result<Option<Recommendation>, StoreError> {
        self.recover_transactions()?;
        let stored = self.read_all_stored_by_id()?;
        let Some(current_record) = stored.get(id) else {
            return Ok(None);
        };
        let current = &current_record.recommendation;
        if current.recommendation_type != RecommendationType::ImproveWorkflow
            || !matches!(
                current.status,
                RecommendationStatus::Proposed
                    | RecommendationStatus::Executing
                    | RecommendationStatus::Accepted
            )
        {
            return Err(StoreError::InvalidTransition);
        }
        let Some(old_packet) = current.completion_packet.as_ref() else {
            return Err(StoreError::InvalidTransition);
        };
        if old_packet.revision != expected_revision
            || replacement.revision != old_packet.revision.saturating_add(1)
            || replacement.packet_id == old_packet.packet_id
        {
            return Err(StoreError::StalePacket { id: id.into() });
        }

        let mut lineage = old_packet.lineage.clone();
        lineage.push(super::completion::CompletionPacketLineage {
            packet_id: old_packet.packet_id.clone(),
            revision: old_packet.revision,
            evidence_fingerprint: old_packet.evidence_fingerprint.clone(),
            superseded_at: updated_at.clone(),
        });
        replacement.supersedes_packet_id = Some(old_packet.packet_id.clone());
        replacement.lineage = lineage;
        replacement.approval = None;
        replacement.completion_idempotency_key = None;

        let mut next = current.clone();
        next.status = RecommendationStatus::Proposed;
        next.target_session_id = None;
        next.updated_at = updated_at;
        next.completion_packet = Some(replacement);
        validate_recommendation(&next)?;
        let mut preview = stored
            .iter()
            .map(|(record_id, record)| (record_id.clone(), record.recommendation.clone()))
            .collect::<BTreeMap<_, _>>();
        preview.insert(id.into(), next.clone());
        validate_graph_records(&preview.values().cloned().collect::<Vec<_>>())?;

        let expected = BTreeMap::from([(id.into(), Some(current_record.hash.clone()))]);
        let replacements = BTreeMap::from([(
            id.into(),
            Replacement {
                old: Some(current_record.bytes.clone()),
                new: Some(serde_json::to_vec_pretty(&next).map_err(StoreError::Json)?),
            },
        )]);
        self.commit_replacements(&expected, replacements)?;
        Ok(Some(next))
    }

    /// Recover accepted/executing packet recommendations whose target session
    /// disappeared before reporting a result. Evidence remains intact and a
    /// new explicit user approval is required for a retry.
    pub(crate) fn recover_orphaned_packet_executions(
        &self,
        retained_session_ids: &HashSet<String>,
        recovered_at: String,
    ) -> Result<Vec<String>, StoreError> {
        self.recover_transactions()?;
        let stored = self.read_all_stored_by_id()?;
        let mut expected = BTreeMap::new();
        let mut replacements = BTreeMap::new();
        let mut recovered = Vec::new();
        let mut preview = stored
            .iter()
            .map(|(id, record)| (id.clone(), record.recommendation.clone()))
            .collect::<BTreeMap<_, _>>();

        for (id, record) in &stored {
            let recommendation = &record.recommendation;
            let Some(target_session_id) = recommendation.target_session_id.as_deref() else {
                continue;
            };
            if !recommendation.completion_packet.as_ref().is_some_and(|_| {
                matches!(
                    recommendation.status,
                    RecommendationStatus::Executing | RecommendationStatus::Accepted
                )
            }) || retained_session_ids.contains(target_session_id)
            {
                continue;
            }
            let mut next = recommendation.clone();
            next.status = RecommendationStatus::Proposed;
            next.target_session_id = None;
            next.updated_at = recovered_at.clone();
            if let Some(packet) = next.completion_packet.as_mut() {
                packet.approval = None;
                packet.completion_idempotency_key = None;
            }
            validate_recommendation(&next)?;
            expected.insert(id.clone(), Some(record.hash.clone()));
            replacements.insert(
                id.clone(),
                Replacement {
                    old: Some(record.bytes.clone()),
                    new: Some(serde_json::to_vec_pretty(&next).map_err(StoreError::Json)?),
                },
            );
            preview.insert(id.clone(), next);
            recovered.push(id.clone());
        }

        if recovered.is_empty() {
            return Ok(recovered);
        }
        validate_graph_records(&preview.values().cloned().collect::<Vec<_>>())?;
        self.commit_replacements(&expected, replacements)?;
        Ok(recovered)
    }

    pub(crate) fn delete_referencing_session(&self, session_id: &str) -> Result<(), StoreError> {
        self.recover_transactions()?;
        let recommendations = self.read_all()?;
        validate_graph_records(&recommendations)?;
        let mut ids = BTreeSet::new();
        for recommendation in &recommendations {
            if references_session(&recommendation, session_id) {
                ids.insert(recommendation.id.clone());
            }
        }
        self.delete_graph_records(&recommendations, ids)
    }

    pub(crate) fn scrub_orphans(
        &self,
        retained_session_ids: &HashSet<String>,
    ) -> Result<(), StoreError> {
        self.recover_transactions()?;
        let recommendations = self.read_all()?;
        validate_graph_records(&recommendations)?;
        let mut ids = BTreeSet::new();
        for recommendation in &recommendations {
            let mut referenced_session_ids = recommendation
                .source_session_ids
                .iter()
                .chain(
                    recommendation
                        .workflow_improvement
                        .affected_session_ids
                        .iter(),
                )
                .chain(
                    recommendation
                        .workflow_improvement
                        .dismissal_watermark
                        .iter()
                        .flat_map(|watermark| watermark.affected_session_ids.iter()),
                )
                .chain(
                    recommendation
                        .evidence
                        .iter()
                        .map(|evidence| &evidence.session_id),
                );
            let orphaned = referenced_session_ids.any(|id| !retained_session_ids.contains(id));
            if orphaned {
                ids.insert(recommendation.id.clone());
            }
        }
        self.delete_graph_records(&recommendations, ids)
    }

    fn path_for(&self, id: &str) -> PathBuf {
        self.dir.join(recommendation_filename(id))
    }

    fn migrate_legacy_rollup_filenames(&self) -> Result<(), StoreError> {
        for entry in fs::read_dir(&self.dir).map_err(StoreError::Io)? {
            let entry = entry.map_err(StoreError::Io)?;
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let Some(id) = name
                .strip_prefix("rollup:")
                .and_then(|name| name.strip_suffix(".json"))
            else {
                continue;
            };
            let logical_id = format!("rollup:{id}");
            if !valid_id(&logical_id) {
                continue;
            }
            let encoded = self.path_for(&logical_id);
            if encoded.exists() {
                return Err(StoreError::Recovery(format!(
                    "both legacy and encoded filenames exist for {logical_id}"
                )));
            }
            fs::rename(path, encoded).map_err(StoreError::Io)?;
        }
        Ok(())
    }

    fn read_path(&self, path: &Path) -> Result<Recommendation, StoreError> {
        let json = fs::read_to_string(path).map_err(StoreError::Io)?;
        let recommendation = serde_json::from_str(&json).map_err(StoreError::Json)?;
        validate_recommendation(&recommendation)?;
        Ok(recommendation)
    }

    fn read_stored_path(&self, path: &Path) -> Result<StoredRecommendation, StoreError> {
        let bytes = fs::read(path).map_err(StoreError::Io)?;
        let recommendation = serde_json::from_slice(&bytes).map_err(StoreError::Json)?;
        validate_recommendation(&recommendation)?;
        Ok(StoredRecommendation {
            recommendation,
            hash: hash_bytes(&bytes),
            bytes,
        })
    }

    fn read_all(&self) -> Result<Vec<Recommendation>, StoreError> {
        let mut recommendations = Vec::new();
        for entry in fs::read_dir(&self.dir).map_err(StoreError::Io)? {
            let path = entry.map_err(StoreError::Io)?.path();
            if !path.is_file()
                || path.extension().and_then(|extension| extension.to_str()) != Some("json")
            {
                continue;
            }
            recommendations.push(self.read_path(&path)?);
        }
        Ok(recommendations)
    }

    fn read_all_by_id(&self) -> Result<BTreeMap<String, Recommendation>, StoreError> {
        Ok(self
            .read_all()?
            .into_iter()
            .map(|recommendation| (recommendation.id.clone(), recommendation))
            .collect())
    }

    fn read_all_stored(&self) -> Result<Vec<StoredRecommendation>, StoreError> {
        let mut recommendations = Vec::new();
        for entry in fs::read_dir(&self.dir).map_err(StoreError::Io)? {
            let path = entry.map_err(StoreError::Io)?.path();
            if !path.is_file()
                || path.extension().and_then(|extension| extension.to_str()) != Some("json")
            {
                continue;
            }
            recommendations.push(self.read_stored_path(&path)?);
        }
        Ok(recommendations)
    }

    fn read_all_stored_by_id(&self) -> Result<BTreeMap<String, StoredRecommendation>, StoreError> {
        Ok(self
            .read_all_stored()?
            .into_iter()
            .map(|record| (record.recommendation.id.clone(), record))
            .collect())
    }

    fn validate_graph(&self) -> Result<(), StoreError> {
        validate_graph_records(&self.read_all()?)
    }

    fn verify_expected(
        &self,
        expected: &BTreeMap<String, Option<String>>,
        current: &BTreeMap<String, Recommendation>,
    ) -> Result<(), StoreError> {
        for (id, expected_hash) in expected {
            if !valid_id(id) {
                return Err(StoreError::StaleExpectedHash { id: id.clone() });
            }
            let actual = fs::read(self.path_for(id)).ok();
            match (expected_hash, actual) {
                (None, None) => {}
                (Some(expected_hash), Some(bytes)) if hash_bytes(&bytes) == *expected_hash => {}
                _ => return Err(StoreError::StaleExpectedHash { id: id.clone() }),
            }
        }
        if current
            .values()
            .any(|recommendation| !valid_id(&recommendation.id))
        {
            return Err(StoreError::GraphInvariant(
                "recommendation ID is not a safe file name".into(),
            ));
        }
        Ok(())
    }

    fn delete_graph_records(
        &self,
        recommendations: &[Recommendation],
        mut ids: BTreeSet<String>,
    ) -> Result<(), StoreError> {
        let by_id = recommendations
            .iter()
            .map(|recommendation| (recommendation.id.as_str(), recommendation))
            .collect::<BTreeMap<_, _>>();
        let mut changed = true;
        while changed {
            changed = false;
            for id in ids.clone() {
                let Some(recommendation) = by_id.get(id.as_str()) else {
                    continue;
                };
                if let Some(parent_id) = recommendation.rolled_up_by.as_deref() {
                    changed |= ids.insert(parent_id.to_string());
                }
                for member_id in &recommendation.rollup_member_ids {
                    changed |= ids.insert(member_id.clone());
                }
            }
            for recommendation in recommendations {
                if recommendation
                    .rollup_member_ids
                    .iter()
                    .any(|member_id| ids.contains(member_id))
                {
                    changed |= ids.insert(recommendation.id.clone());
                }
            }
        }
        let stored = self.read_all_stored_by_id()?;
        let expected = ids
            .iter()
            .filter_map(|id| {
                stored
                    .get(id.as_str())
                    .map(|record| (id.clone(), Some(record.hash.clone())))
            })
            .collect::<BTreeMap<_, _>>();
        let replacements = ids
            .iter()
            .filter_map(|id| {
                stored.get(id.as_str()).map(|record| {
                    (
                        id.clone(),
                        Replacement {
                            old: Some(record.bytes.clone()),
                            new: None,
                        },
                    )
                })
            })
            .collect();
        self.commit_replacements(&expected, replacements)
    }

    fn commit_replacements(
        &self,
        expected: &BTreeMap<String, Option<String>>,
        replacements: BTreeMap<String, Replacement>,
    ) -> Result<(), StoreError> {
        let current = self.read_all_by_id()?;
        self.verify_expected(expected, &current)?;
        for (id, replacement) in &replacements {
            let actual = fs::read(self.path_for(id)).ok();
            if actual != replacement.old {
                return Err(StoreError::StaleExpectedHash { id: id.clone() });
            }
        }
        if replacements.is_empty() {
            return Ok(());
        }

        let transaction_root = self
            .dir
            .join(ROLLUP_TRANSACTION_DIR)
            .join(format!("tx-{}", Uuid::new_v4()));
        fs::create_dir_all(transaction_root.join("staged")).map_err(StoreError::Io)?;
        fs::create_dir_all(transaction_root.join("backups")).map_err(StoreError::Io)?;
        let mut entries = Vec::with_capacity(replacements.len());
        #[cfg(test)]
        let mut staging_fault_pending = true;
        for (id, replacement) in replacements {
            #[cfg(test)]
            if staging_fault_pending {
                take_fault_point(|point| matches!(point, FaultPoint::Staging))?;
                staging_fault_pending = false;
            }
            let Replacement { old, new } = replacement;
            let old_sha256 = old.as_deref().map(hash_bytes);
            let old_exists = old.is_some();
            let backup = if let Some(old) = old {
                let relative = transaction_backup_path(&id);
                write_sync(&transaction_root.join(&relative), &old)?;
                Some(relative)
            } else {
                None
            };
            let new_exists = new.is_some();
            let (staged, new_sha256) = if let Some(new) = new {
                let relative = transaction_staged_path(&id);
                write_sync(&transaction_root.join(&relative), &new)?;
                (Some(relative), Some(hash_bytes(&new)))
            } else {
                (None, None)
            };
            entries.push(RollupTransactionEntry {
                id,
                target: String::new(),
                staged,
                backup,
                old_sha256,
                old_exists,
                new_sha256,
                new_exists,
            });
        }
        // The target is derived from the ID, not entry order. Rebuild it here
        // so the manifest cannot point at the wrong recommendation if the
        // map's ordering changes.
        for entry in &mut entries {
            entry.target = recommendation_filename(&entry.id);
        }

        let manifest_path = transaction_root.join(ROLLUP_TRANSACTION_MANIFEST);
        let manifest = RollupTransactionManifest {
            version: ROLLUP_TRANSACTION_VERSION,
            committed: false,
            entries,
        };
        write_manifest_atomic(&manifest_path, &manifest)?;
        sync_directory(&transaction_root)?;

        let mut committed_manifest = manifest;
        committed_manifest.committed = true;
        #[cfg(test)]
        take_fault_point(|point| matches!(point, FaultPoint::ManifestCommit))?;
        write_manifest_atomic(&manifest_path, &committed_manifest)?;
        sync_directory(&transaction_root)?;
        if let Some(parent) = transaction_root.parent() {
            sync_directory(parent)?;
        }

        #[cfg(test)]
        let mut publication_index = 0;
        for entry in &committed_manifest.entries {
            self.publish_entry(&transaction_root, entry)?;
            #[cfg(test)]
            {
                publication_index += 1;
                take_fault_point(
                    |point| matches!(point, FaultPoint::Publication(after) if after == publication_index),
                )?;
            }
        }
        sync_directory(&self.dir)?;
        if let Err(error) = self.validate_graph() {
            let rollback =
                self.rollback_transaction(&transaction_root, &committed_manifest.entries);
            if let Err(rollback_error) = rollback {
                return Err(StoreError::Recovery(format!(
                    "published graph invalid ({error}); rollback failed: {rollback_error}"
                )));
            }
            self.validate_graph().map_err(|rollback_error| {
                StoreError::Recovery(format!(
                    "published graph invalid ({error}); old graph invalid after rollback: {rollback_error}"
                ))
            })?;
            return Err(error);
        }
        #[cfg(test)]
        take_fault_point(|point| matches!(point, FaultPoint::Cleanup))?;
        fs::remove_file(&manifest_path).map_err(StoreError::Io)?;
        sync_directory(&transaction_root)?;
        fs::remove_dir_all(&transaction_root).map_err(StoreError::Io)?;
        sync_directory(&self.dir)?;
        Ok(())
    }

    fn publish_entry(
        &self,
        transaction_root: &Path,
        entry: &RollupTransactionEntry,
    ) -> Result<(), StoreError> {
        let target = self.dir.join(&entry.target);
        if !entry.new_exists {
            match fs::remove_file(&target) {
                Ok(()) => return Ok(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
                Err(error) => return Err(StoreError::Io(error)),
            }
        }
        if fs::read(&target)
            .ok()
            .is_some_and(|bytes| Some(hash_bytes(&bytes)) == entry.new_sha256)
        {
            return Ok(());
        }
        let Some(staged) = entry.staged.as_deref() else {
            return Err(StoreError::Recovery(format!(
                "transaction entry {} has no staged replacement",
                entry.id
            )));
        };
        let staged_path = transaction_root.join(staged);
        let staged_bytes = fs::read(&staged_path).map_err(StoreError::Io)?;
        if Some(hash_bytes(&staged_bytes)) != entry.new_sha256 {
            return Err(StoreError::Recovery(format!(
                "staged replacement hash mismatch for {}",
                entry.id
            )));
        }
        crate::harness::integration::atomic_replace(&staged_path, &target, target.exists())
            .map_err(StoreError::Io)
    }

    fn recover_transactions(&self) -> Result<(), StoreError> {
        let root = self.dir.join(ROLLUP_TRANSACTION_DIR);
        if !root.exists() {
            return Ok(());
        }
        let entries = fs::read_dir(&root).map_err(StoreError::Io)?;
        for entry in entries {
            let transaction_root = entry.map_err(StoreError::Io)?.path();
            if !transaction_root.is_dir() {
                continue;
            }
            if !transaction_root.join(ROLLUP_TRANSACTION_MANIFEST).exists() {
                fs::remove_dir_all(&transaction_root).map_err(StoreError::Io)?;
                continue;
            }
            self.recover_transaction(&transaction_root)?;
        }
        if fs::read_dir(&root)
            .map_err(StoreError::Io)?
            .next()
            .is_none()
        {
            fs::remove_dir(&root).map_err(StoreError::Io)?;
            sync_directory(&self.dir)?;
        }
        Ok(())
    }

    fn recover_transaction(&self, transaction_root: &Path) -> Result<(), StoreError> {
        let manifest_path = transaction_root.join(ROLLUP_TRANSACTION_MANIFEST);
        let manifest_json = fs::read_to_string(&manifest_path).map_err(|error| {
            StoreError::Recovery(format!("cannot read {}: {error}", manifest_path.display()))
        })?;
        let mut manifest: RollupTransactionManifest = serde_json::from_str(&manifest_json)
            .map_err(|error| StoreError::Recovery(format!("invalid rollup manifest: {error}")))?;
        if !matches!(
            manifest.version,
            LEGACY_ROLLUP_TRANSACTION_VERSION | ROLLUP_TRANSACTION_VERSION
        ) {
            return Err(StoreError::Recovery(format!(
                "unsupported rollup transaction version {}",
                manifest.version
            )));
        }
        if manifest.version == LEGACY_ROLLUP_TRANSACTION_VERSION {
            self.migrate_legacy_transaction_paths(transaction_root, &mut manifest.entries)?;
        }
        validate_manifest_entries(&manifest.entries)?;
        if manifest.committed {
            let publish_result = manifest
                .entries
                .iter()
                .try_for_each(|entry| self.publish_entry(transaction_root, entry));
            if publish_result.is_ok() {
                sync_directory(&self.dir)?;
                if self.validate_graph().is_ok() {
                    self.finish_transaction_cleanup(transaction_root, &manifest_path)?;
                    return Ok(());
                }
            }
        }

        self.rollback_transaction(transaction_root, &manifest.entries)?;
        self.validate_graph().map_err(|error| {
            StoreError::Recovery(format!(
                "neither old nor new recommendation graph is valid: {error}"
            ))
        })?;
        self.finish_transaction_cleanup(transaction_root, &manifest_path)
    }

    fn rollback_transaction(
        &self,
        transaction_root: &Path,
        entries: &[RollupTransactionEntry],
    ) -> Result<(), StoreError> {
        for entry in entries {
            let target = self.dir.join(&entry.target);
            if entry.old_exists {
                let Some(backup) = entry.backup.as_deref() else {
                    return Err(StoreError::Recovery(format!(
                        "transaction entry {} has no old-content backup",
                        entry.id
                    )));
                };
                let backup_path = transaction_root.join(backup);
                let bytes = fs::read(&backup_path).map_err(StoreError::Io)?;
                if Some(hash_bytes(&bytes)) != entry.old_sha256 {
                    return Err(StoreError::Recovery(format!(
                        "old-content backup hash mismatch for {}",
                        entry.id
                    )));
                }
                let restore_path = transaction_root.join(transaction_restore_path(&entry.id));
                write_sync(&restore_path, &bytes)?;
                crate::harness::integration::atomic_replace(
                    &restore_path,
                    &target,
                    target.exists(),
                )
                .map_err(StoreError::Io)?;
            } else {
                match fs::remove_file(&target) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(StoreError::Io(error)),
                }
            }
        }
        sync_directory(&self.dir)?;
        Ok(())
    }

    fn finish_transaction_cleanup(
        &self,
        transaction_root: &Path,
        manifest_path: &Path,
    ) -> Result<(), StoreError> {
        fs::remove_file(manifest_path).map_err(StoreError::Io)?;
        sync_directory(transaction_root)?;
        fs::remove_dir_all(transaction_root).map_err(StoreError::Io)?;
        sync_directory(&self.dir)?;
        Ok(())
    }

    fn migrate_legacy_transaction_paths(
        &self,
        transaction_root: &Path,
        entries: &mut [RollupTransactionEntry],
    ) -> Result<(), StoreError> {
        let mut migrated = false;
        for entry in entries {
            if !valid_id(&entry.id) {
                continue;
            }
            let legacy_target = format!("{}.json", entry.id);
            let current_target = recommendation_filename(&entry.id);
            if entry.target == legacy_target {
                migrate_transaction_path(
                    &self.dir.join(&legacy_target),
                    &self.dir.join(&current_target),
                    &entry.id,
                )?;
                entry.target = current_target;
                migrated = true;
            }
            migrate_optional_transaction_path(
                transaction_root,
                &mut entry.staged,
                &format!("staged/{}.json", entry.id),
                &transaction_staged_path(&entry.id),
                &entry.id,
                &mut migrated,
            )?;
            migrate_optional_transaction_path(
                transaction_root,
                &mut entry.backup,
                &format!("backups/{}.json", entry.id),
                &transaction_backup_path(&entry.id),
                &entry.id,
                &mut migrated,
            )?;
        }
        if migrated {
            sync_directory(&self.dir)?;
            sync_directory(transaction_root)?;
        }
        Ok(())
    }
}

fn migrate_optional_transaction_path(
    transaction_root: &Path,
    path: &mut Option<String>,
    legacy: &str,
    current: &str,
    id: &str,
    migrated: &mut bool,
) -> Result<(), StoreError> {
    if path.as_deref() != Some(legacy) {
        return Ok(());
    }
    migrate_transaction_path(
        &transaction_root.join(legacy),
        &transaction_root.join(current),
        id,
    )?;
    *path = Some(current.to_string());
    *migrated = true;
    Ok(())
}

fn migrate_transaction_path(source: &Path, destination: &Path, id: &str) -> Result<(), StoreError> {
    if source == destination || !source.exists() {
        return Ok(());
    }
    if destination.exists() {
        return Err(StoreError::Recovery(format!(
            "both legacy and encoded transaction paths exist for {id}"
        )));
    }
    fs::rename(source, destination).map_err(StoreError::Io)
}

fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn write_sync(path: &Path, bytes: &[u8]) -> Result<(), StoreError> {
    use std::io::Write;
    let mut file = fs::File::create(path).map_err(StoreError::Io)?;
    file.write_all(bytes).map_err(StoreError::Io)?;
    file.sync_all().map_err(StoreError::Io)
}

fn write_json_sync<T: Serialize>(path: &Path, value: &T) -> Result<(), StoreError> {
    let bytes = serde_json::to_vec_pretty(value).map_err(StoreError::Json)?;
    write_sync(path, &bytes)
}

fn write_manifest_atomic(
    manifest_path: &Path,
    manifest: &RollupTransactionManifest,
) -> Result<(), StoreError> {
    let temporary = manifest_path.with_extension("json.tmp");
    write_json_sync(&temporary, manifest)?;
    crate::harness::integration::atomic_replace(&temporary, manifest_path, manifest_path.exists())
        .map_err(StoreError::Io)?;
    if let Some(transaction_root) = manifest_path.parent() {
        sync_directory(transaction_root)?;
        if let Some(transaction_parent) = transaction_root.parent() {
            sync_directory(transaction_parent)?;
        }
    }
    Ok(())
}

#[cfg(windows)]
fn sync_directory(path: &Path) -> Result<(), StoreError> {
    let _ = path;
    // Rust's standard File::open does not request the Win32 directory-handle
    // semantics needed for Unix-style directory fsync. File contents are
    // still synced before publication; new-file publication uses
    // MOVEFILE_WRITE_THROUGH. ReplaceFileW has no write-through flag, so
    // Windows cannot provide equivalent directory-entry crash durability here.
    Ok(())
}

#[cfg(not(windows))]
fn sync_directory(path: &Path) -> Result<(), StoreError> {
    fs::File::open(path)
        .map_err(StoreError::Io)
        .and_then(|directory| directory.sync_all().map_err(StoreError::Io))
}

fn validate_manifest_entries(entries: &[RollupTransactionEntry]) -> Result<(), StoreError> {
    let mut ids = BTreeSet::new();
    for entry in entries {
        if !valid_id(&entry.id)
            || entry.target != recommendation_filename(&entry.id)
            || !ids.insert(entry.id.clone())
            || entry.old_exists != entry.old_sha256.is_some()
            || entry.old_exists != entry.backup.is_some()
            || entry.new_exists != entry.new_sha256.is_some()
            || entry.new_exists != entry.staged.is_some()
        {
            return Err(StoreError::Recovery(format!(
                "invalid transaction entry for {}",
                entry.id
            )));
        }
        if entry.staged.as_deref()
            != entry
                .new_exists
                .then(|| transaction_staged_path(&entry.id))
                .as_deref()
            || entry.backup.as_deref()
                != entry
                    .old_exists
                    .then(|| transaction_backup_path(&entry.id))
                    .as_deref()
        {
            return Err(StoreError::Recovery(format!(
                "invalid transaction paths for {}",
                entry.id
            )));
        }
        for relative in entry.staged.iter().chain(entry.backup.iter()) {
            let path = Path::new(relative);
            if path.is_absolute()
                || path
                    .components()
                    .any(|component| matches!(component, std::path::Component::ParentDir))
            {
                return Err(StoreError::Recovery(format!(
                    "transaction entry {} escapes its directory",
                    entry.id
                )));
            }
        }
    }
    Ok(())
}

fn recommendation_filename(id: &str) -> String {
    format!("{}.json", filename_component(id))
}

fn validate_recommendation(recommendation: &Recommendation) -> Result<(), StoreError> {
    if let Some(packet) = &recommendation.completion_packet {
        packet
            .validate(&recommendation.workspace_id)
            .map_err(StoreError::GraphInvariant)?;
        if recommendation.source_session_ids.len() != 1
            || recommendation.source_session_ids[0] != packet.source_session_id
        {
            return Err(StoreError::GraphInvariant(
                "completion packet must have one matching source session".into(),
            ));
        }
        let approval_required = matches!(
            recommendation.status,
            RecommendationStatus::Executing
                | RecommendationStatus::Accepted
                | RecommendationStatus::Completed
        );
        if packet.approval.is_some() != approval_required
            || packet.completion_idempotency_key.is_some()
                != (recommendation.status == RecommendationStatus::Completed)
        {
            return Err(StoreError::GraphInvariant(
                "completion packet approval does not match recommendation lifecycle".into(),
            ));
        }
    }
    Ok(())
}

fn transaction_staged_path(id: &str) -> String {
    format!("staged/{}.json", filename_component(id))
}

fn transaction_backup_path(id: &str) -> String {
    format!("backups/{}.json", filename_component(id))
}

fn transaction_restore_path(id: &str) -> String {
    format!("restore-{}.json", filename_component(id))
}

fn filename_component(id: &str) -> String {
    let mut encoded = String::with_capacity(id.len());
    for byte in id.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_') {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

fn validate_graph_records(records: &[Recommendation]) -> Result<(), StoreError> {
    let by_id = records
        .iter()
        .map(|recommendation| (recommendation.id.as_str(), recommendation))
        .collect::<BTreeMap<_, _>>();
    if by_id.len() != records.len() {
        return Err(StoreError::GraphInvariant(
            "duplicate recommendation IDs".into(),
        ));
    }

    let mut active_members = BTreeMap::<String, String>::new();
    for recommendation in records {
        if recommendation.rollup_member_ids.len() != recommendation.rollup_member_dedupe_keys.len()
        {
            return Err(StoreError::GraphInvariant(format!(
                "parent {} has mismatched member and dedupe-key lists",
                recommendation.id
            )));
        }
        let mut member_ids = BTreeSet::new();
        for member_id in &recommendation.rollup_member_ids {
            if !member_ids.insert(member_id) {
                return Err(StoreError::GraphInvariant(format!(
                    "parent {} lists member {} more than once",
                    recommendation.id, member_id
                )));
            }
            if !by_id.contains_key(member_id.as_str()) {
                return Err(StoreError::GraphInvariant(format!(
                    "parent {} references missing member {}",
                    recommendation.id, member_id
                )));
            }
        }
        if !member_ids.is_empty()
            && recommendation.id != stable_rollup_id(&recommendation.rollup_member_ids)
        {
            return Err(StoreError::GraphInvariant(format!(
                "parent {} does not match its stable member identity",
                recommendation.id
            )));
        }
        let mut expected_dedupe_keys = recommendation
            .rollup_member_ids
            .iter()
            .map(|member_id| {
                by_id
                    .get(member_id.as_str())
                    .expect("checked above")
                    .dedupe_key
                    .clone()
            })
            .collect::<Vec<_>>();
        expected_dedupe_keys.sort();
        let mut actual_dedupe_keys = recommendation.rollup_member_dedupe_keys.clone();
        actual_dedupe_keys.sort();
        if actual_dedupe_keys != expected_dedupe_keys {
            return Err(StoreError::GraphInvariant(format!(
                "parent {} has member dedupe keys that do not match its members",
                recommendation.id
            )));
        }
        if recommendation.rolled_up_by.is_some()
            && recommendation.status != RecommendationStatus::RolledUp
        {
            return Err(StoreError::GraphInvariant(format!(
                "non-rolled-up recommendation {} has a current parent",
                recommendation.id
            )));
        }
        if recommendation.status == RecommendationStatus::RolledUp
            && recommendation.rolled_up_by.is_none()
        {
            return Err(StoreError::GraphInvariant(format!(
                "rolled-up recommendation {} has no current parent",
                recommendation.id
            )));
        }

        let active_parent = matches!(
            recommendation.status,
            RecommendationStatus::Proposed | RecommendationStatus::Executing
        );
        if active_parent && !recommendation.rollup_member_ids.is_empty() {
            for member_id in &recommendation.rollup_member_ids {
                let member = by_id.get(member_id.as_str()).expect("checked above");
                if member.status != RecommendationStatus::RolledUp
                    || member.rolled_up_by.as_deref() != Some(recommendation.id.as_str())
                {
                    return Err(StoreError::GraphInvariant(format!(
                        "active parent {} does not own member {}",
                        recommendation.id, member_id
                    )));
                }
                if let Some(previous_parent) =
                    active_members.insert(member_id.clone(), recommendation.id.clone())
                {
                    return Err(StoreError::GraphInvariant(format!(
                        "member {} belongs to active parents {} and {}",
                        member_id, previous_parent, recommendation.id
                    )));
                }
            }
        }
    }

    for recommendation in records {
        if recommendation.status != RecommendationStatus::RolledUp {
            continue;
        }
        let parent_id = recommendation
            .rolled_up_by
            .as_deref()
            .expect("checked above");
        let Some(parent) = by_id.get(parent_id) else {
            return Err(StoreError::GraphInvariant(format!(
                "member {} points to missing parent {}",
                recommendation.id, parent_id
            )));
        };
        if parent
            .rollup_member_ids
            .iter()
            .filter(|member_id| member_id.as_str() == recommendation.id)
            .count()
            != 1
        {
            return Err(StoreError::GraphInvariant(format!(
                "parent {} does not list member {} exactly once",
                parent_id, recommendation.id
            )));
        }
    }
    Ok(())
}

fn valid_id(id: &str) -> bool {
    let legacy_id = !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'));
    let stable_rollup_id = id.strip_prefix("rollup:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    });
    legacy_id || stable_rollup_id
}

fn references_session(recommendation: &Recommendation, session_id: &str) -> bool {
    recommendation
        .source_session_ids
        .iter()
        .any(|id| id == session_id)
        || recommendation
            .evidence
            .iter()
            .any(|evidence| evidence.session_id == session_id)
        || recommendation
            .workflow_improvement
            .affected_session_ids
            .iter()
            .any(|id| id == session_id)
        || recommendation
            .workflow_improvement
            .dismissal_watermark
            .as_ref()
            .is_some_and(|watermark| {
                watermark
                    .affected_session_ids
                    .iter()
                    .any(|id| id == session_id)
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::{
        RecommendationConfidence, RecommendationType, TargetSurface, WorkflowImprovement,
        WorkflowObservationEvidence,
    };
    use crate::workflow_observations::{Impact, ObservationKind, ObservationSource};
    use sha2::{Digest, Sha256};
    use std::collections::BTreeMap;

    fn recommendation(id: &str, session_id: &str) -> Recommendation {
        let evidence = WorkflowObservationEvidence {
            observation_id: format!("observation-{id}"),
            sequence: 4,
            session_id: session_id.into(),
            kind: ObservationKind::Obstacle,
            description: "A recurring obstacle".into(),
            evidence: "The same command failed twice".into(),
            problem_area: None,
            reported_impact: Impact::High,
            source: ObservationSource::Peon,
            confidence: 0.9,
            observed_at: "2026-08-20T12:00:00Z".into(),
        };
        Recommendation {
            id: id.into(),
            workspace_id: "workspace-1".into(),
            chain_id: "chain-1".into(),
            chain_depth: 0,
            recommendation_type: RecommendationType::ImproveWorkflow,
            status: RecommendationStatus::Proposed,
            priority: Impact::High,
            title: "Improve workflow".into(),
            summary: "Make the obstacle easier to avoid".into(),
            reason: vec!["It recurred".into()],
            evidence: vec![evidence],
            repository_evidence: vec![],
            knowledge_evidence: vec![],
            source_session_ids: vec![session_id.into()],
            target_session_id: None,
            suggested_harness_id: None,
            suggested_model: None,
            suggested_working_directory: None,
            suggested_prompt: None,
            confidence: RecommendationConfidence::High,
            requires_approval: false,
            dedupe_key: "improve_workflow:v1:tooling:v1:obstacle:test".into(),
            created_at: "2026-08-20T12:00:00Z".into(),
            updated_at: "2026-08-20T12:00:00Z".into(),
            expires_at: None,
            workflow_improvement: WorkflowImprovement {
                proposed_improvement: "Remove the obstacle".into(),
                target_surface: TargetSurface::Tooling,
                observation_ids: vec!["observation-1".into()],
                recurrence_count: 1,
                affected_session_ids: vec![session_id.into()],
                impact: Impact::High,
                expected_benefit: "Less repeated failure".into(),
                supersedes_recommendation_id: None,
                dismissal_watermark: None,
            },
            completion_packet: None,
            rollup_member_ids: Vec::new(),
            rollup_member_dedupe_keys: Vec::new(),
            rollup_generation: None,
            rolled_up_by: None,
        }
    }

    fn rollup_parent(id: &str, member_ids: &[&str]) -> Recommendation {
        let mut parent = recommendation(id, "parent-session");
        parent.title = "Combined improvement".into();
        parent.summary = "Combined summary".into();
        parent.rollup_member_ids = member_ids.iter().map(|id| (*id).into()).collect();
        parent.rollup_member_ids.sort();
        parent.rollup_member_dedupe_keys = parent
            .rollup_member_ids
            .iter()
            .map(|_| parent.dedupe_key.clone())
            .collect();
        parent.evidence.clear();
        parent.source_session_ids.clear();
        parent.workflow_improvement.observation_ids.clear();
        parent.workflow_improvement.affected_session_ids.clear();
        parent.workflow_improvement.recurrence_count = 0;
        parent.workflow_improvement.supersedes_recommendation_id = None;
        parent
    }

    fn expected_hash(recommendation: &Recommendation) -> String {
        let json = serde_json::to_vec_pretty(recommendation).unwrap();
        format!("{:x}", Sha256::digest(json))
    }

    fn expected_present(recommendations: &[&Recommendation]) -> BTreeMap<String, Option<String>> {
        recommendations
            .iter()
            .map(|recommendation| {
                (
                    recommendation.id.clone(),
                    Some(expected_hash(recommendation)),
                )
            })
            .collect()
    }

    fn packet_recommendation() -> Recommendation {
        let mut recommendation = recommendation("packet-recommendation", "session-source");
        recommendation.completion_packet = Some(crate::taskmaster::completion_tests::test_packet());
        recommendation
    }

    fn packet_mutation(
        recommendation: &Recommendation,
        key: &str,
    ) -> super::super::completion::CompletionMutationRequest {
        let packet = recommendation.completion_packet.as_ref().unwrap();
        super::super::completion::CompletionMutationRequest {
            packet_revision: packet.revision,
            evidence_fingerprint: packet.evidence_fingerprint.clone(),
            idempotency_key: key.into(),
        }
    }

    #[test]
    fn persists_canonical_json_and_reloads_after_restart() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let original = recommendation("recommendation-1", "session-1");
        store.put(&original).unwrap();

        let json: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("recommendations/recommendation-1.json"))
                .unwrap(),
        )
        .unwrap();
        assert_eq!(json["type"], "improve_workflow");
        assert_eq!(json["requiresApproval"], false);
        assert!(json["targetSessionId"].is_null());
        assert_eq!(
            RecommendationStore::open(dir.path().to_path_buf())
                .unwrap()
                .get("recommendation-1")
                .unwrap(),
            Some(original)
        );
    }

    #[test]
    fn put_rejects_a_tampered_completion_packet() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let mut recommendation = packet_recommendation();
        recommendation
            .completion_packet
            .as_mut()
            .unwrap()
            .evidence_fingerprint = "forged".into();

        assert!(matches!(
            store.put(&recommendation),
            Err(StoreError::GraphInvariant(message)) if message.contains("completion packet")
        ));
        assert!(store.list().unwrap().is_empty());
    }

    #[test]
    fn packet_execution_requires_current_revision_and_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let recommendation = packet_recommendation();
        store.put(&recommendation).unwrap();
        let mutation = packet_mutation(&recommendation, "accept-1");

        let mut stale = mutation.clone();
        stale.packet_revision += 1;
        assert!(matches!(
            store.begin_execution_checked(
                "packet-recommendation",
                "session-target".into(),
                "2026-09-24T10:05:00Z".into(),
                &stale,
                "user".into(),
            ),
            Err(StoreError::StalePacket { .. })
        ));
        assert_eq!(
            store.get("packet-recommendation").unwrap().unwrap().status,
            RecommendationStatus::Proposed
        );

        store
            .begin_execution_checked(
                "packet-recommendation",
                "session-target".into(),
                "2026-09-24T10:05:00Z".into(),
                &mutation,
                "user".into(),
            )
            .unwrap();
        let accepted = store
            .complete_execution("packet-recommendation", "2026-09-24T10:06:00Z".into())
            .unwrap()
            .unwrap();
        let completion = packet_mutation(&accepted, "complete-1");
        store
            .complete_accepted_checked(
                "packet-recommendation",
                "2026-09-24T10:07:00Z".into(),
                &completion,
            )
            .unwrap();
        assert!(store
            .complete_accepted_checked(
                "packet-recommendation",
                "2026-09-24T10:08:00Z".into(),
                &completion,
            )
            .unwrap()
            .is_some());

        let mut late = completion;
        late.idempotency_key = "different-result".into();
        assert!(matches!(
            store.complete_accepted_checked(
                "packet-recommendation",
                "2026-09-24T10:09:00Z".into(),
                &late,
            ),
            Err(StoreError::StalePacket { .. })
        ));
    }

    #[test]
    fn cancelling_packet_execution_clears_approval_for_retry() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let recommendation = packet_recommendation();
        store.put(&recommendation).unwrap();
        let mutation = packet_mutation(&recommendation, "accept-cancel");
        store
            .begin_execution_checked(
                "packet-recommendation",
                "session-target".into(),
                "2026-09-24T10:05:00Z".into(),
                &mutation,
                "user".into(),
            )
            .unwrap();
        store
            .cancel_execution("packet-recommendation", "2026-09-24T10:06:00Z".into())
            .unwrap();
        let retry = store.get("packet-recommendation").unwrap().unwrap();
        assert_eq!(retry.status, RecommendationStatus::Proposed);
        assert!(retry.completion_packet.unwrap().approval.is_none());
    }

    #[test]
    fn packet_approval_must_match_recommendation_lifecycle() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let mut recommendation = packet_recommendation();
        let packet = recommendation.completion_packet.as_ref().unwrap().clone();
        recommendation.completion_packet.as_mut().unwrap().approval =
            Some(super::super::completion::CompletionApproval {
                approved_at: "2026-09-24T10:05:00Z".into(),
                approver: "user".into(),
                revision: packet.revision,
                evidence_fingerprint: packet.evidence_fingerprint.clone(),
                action_fingerprint: packet.action_fingerprint(),
                idempotency_key: "accept-1".into(),
            });
        assert!(matches!(
            store.put(&recommendation),
            Err(StoreError::GraphInvariant(message)) if message.contains("lifecycle")
        ));
    }

    #[test]
    fn replacing_packet_invalidates_approval_and_preserves_lineage() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let recommendation = packet_recommendation();
        store.put(&recommendation).unwrap();
        let mutation = packet_mutation(&recommendation, "accept-before-edit");
        store
            .begin_execution_checked(
                "packet-recommendation",
                "session-target".into(),
                "2026-09-24T10:05:00Z".into(),
                &mutation,
                "user".into(),
            )
            .unwrap();
        store
            .cancel_execution("packet-recommendation", "2026-09-24T10:06:00Z".into())
            .unwrap();

        let current = store.get("packet-recommendation").unwrap().unwrap();
        let old_packet = current.completion_packet.clone().unwrap();
        let mut replacement = old_packet.clone();
        replacement.packet_id = "packet-2".into();
        replacement
            .action
            .prompt
            .push_str(" with the updated scope");
        replacement.revision = old_packet.revision + 1;
        replacement.evidence_fingerprint = replacement.computed_evidence_fingerprint();
        replacement.readiness = replacement.derived_readiness();

        let updated = store
            .replace_completion_packet(
                "packet-recommendation",
                old_packet.revision,
                replacement,
                "2026-09-24T10:07:00Z".into(),
            )
            .unwrap()
            .unwrap();
        assert_eq!(updated.status, RecommendationStatus::Proposed);
        assert!(updated.target_session_id.is_none());
        let packet = updated.completion_packet.unwrap();
        assert!(packet.approval.is_none());
        assert_eq!(packet.completion_idempotency_key, None);
        assert_eq!(packet.supersedes_packet_id.as_deref(), Some("packet-1"));
        assert_eq!(packet.lineage.len(), 1);
        assert_eq!(packet.lineage[0].packet_id, "packet-1");

        assert!(matches!(
            store.replace_completion_packet(
                "packet-recommendation",
                old_packet.revision,
                packet,
                "2026-09-24T10:08:00Z".into(),
            ),
            Err(StoreError::StalePacket { .. })
        ));
    }

    #[test]
    fn orphaned_packet_target_is_recovered_for_user_retry() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let recommendation = packet_recommendation();
        store.put(&recommendation).unwrap();
        let mutation = packet_mutation(&recommendation, "accept-orphan");
        store
            .begin_execution_checked(
                "packet-recommendation",
                "session-missing".into(),
                "2026-09-24T10:05:00Z".into(),
                &mutation,
                "user".into(),
            )
            .unwrap();
        store
            .complete_execution("packet-recommendation", "2026-09-24T10:06:00Z".into())
            .unwrap();

        let recovered = store
            .recover_orphaned_packet_executions(
                &HashSet::from(["session-source".to_string()]),
                "2026-09-24T10:07:00Z".into(),
            )
            .unwrap();
        assert_eq!(recovered, vec!["packet-recommendation"]);
        let recommendation = store.get("packet-recommendation").unwrap().unwrap();
        assert_eq!(recommendation.status, RecommendationStatus::Proposed);
        assert!(recommendation.target_session_id.is_none());
        assert!(recommendation.completion_packet.unwrap().approval.is_none());
    }

    #[test]
    fn graph_transaction_uses_original_bytes_for_legacy_records() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let original = recommendation("legacy-recommendation", "session-1");
        let mut legacy_json = serde_json::to_value(&original).unwrap();
        let object = legacy_json.as_object_mut().unwrap();
        object.remove("rollupMemberIds");
        object.remove("rollupMemberDedupeKeys");
        object.remove("rollupGeneration");
        object.remove("rolledUpBy");
        let legacy_bytes = serde_json::to_vec_pretty(&legacy_json).unwrap();
        let path = dir
            .path()
            .join("recommendations/legacy-recommendation.json");
        std::fs::write(&path, &legacy_bytes).unwrap();

        let (records, hashes) = store.list_with_hashes().unwrap();
        assert_eq!(hashes["legacy-recommendation"], hash_bytes(&legacy_bytes));
        let mut changed = records[0].clone();
        changed.updated_at = "2026-08-21T12:00:00Z".into();
        let expected = BTreeMap::from([(
            changed.id.clone(),
            Some(hashes[changed.id.as_str()].clone()),
        )]);

        store
            .apply_recommendation_graph_transaction(&expected, &[changed])
            .unwrap();
        assert_eq!(
            store
                .get("legacy-recommendation")
                .unwrap()
                .unwrap()
                .updated_at,
            "2026-08-21T12:00:00Z"
        );
    }

    #[test]
    fn replaces_an_existing_recommendation() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let mut first = recommendation("replace-existing", "session");
        store.put(&first).unwrap();

        first.status = RecommendationStatus::Accepted;
        store.put(&first).unwrap();

        assert_eq!(
            store.get("replace-existing").unwrap().unwrap().status,
            RecommendationStatus::Accepted
        );
    }

    #[test]
    fn dismisses_in_place_with_immutable_evidence_and_watermark() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put(&recommendation("recommendation-1", "session-1"))
            .unwrap();

        let dismissed = store
            .dismiss("recommendation-1", "2026-08-21T12:00:00Z".into())
            .unwrap()
            .unwrap();

        assert_eq!(dismissed.status, RecommendationStatus::Dismissed);
        assert_eq!(dismissed.evidence[0].description, "A recurring obstacle");
        assert_eq!(
            dismissed
                .workflow_improvement
                .dismissal_watermark
                .as_ref()
                .unwrap()
                .dismissed_through_sequence,
            4
        );
    }

    #[test]
    fn begin_execution_reserves_in_place_and_sets_target_session_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put(&recommendation("recommendation-1", "session-1"))
            .unwrap();

        let reserved = store
            .begin_execution(
                "recommendation-1",
                "session-active".into(),
                "2026-08-21T12:00:00Z".into(),
            )
            .unwrap()
            .unwrap();

        assert_eq!(reserved.status, RecommendationStatus::Executing);
        assert_eq!(reserved.target_session_id, Some("session-active".into()));
        assert_eq!(reserved.updated_at, "2026-08-21T12:00:00Z");
    }

    #[test]
    fn begin_execution_rejects_non_proposed_status() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put(&recommendation("recommendation-1", "session-1"))
            .unwrap();
        store
            .begin_execution(
                "recommendation-1",
                "session-active".into(),
                "2026-08-21T12:00:00Z".into(),
            )
            .unwrap();

        // A second, concurrent accept request must be rejected here, before
        // any second PTY write could ever be attempted — this is the guard
        // that prevents duplicate terminal injection.
        let result = store.begin_execution(
            "recommendation-1",
            "session-other".into(),
            "2026-08-21T12:00:01Z".into(),
        );

        assert!(matches!(result, Err(StoreError::InvalidTransition)));
    }

    #[test]
    fn begin_execution_returns_none_for_unknown_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();

        let result = store
            .begin_execution(
                "missing",
                "session-active".into(),
                "2026-08-21T12:00:00Z".into(),
            )
            .unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn complete_execution_transitions_executing_to_accepted() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put(&recommendation("recommendation-1", "session-1"))
            .unwrap();
        store
            .begin_execution(
                "recommendation-1",
                "session-active".into(),
                "2026-08-21T12:00:00Z".into(),
            )
            .unwrap();

        let completed = store
            .complete_execution("recommendation-1", "2026-08-21T12:00:05Z".into())
            .unwrap()
            .unwrap();

        assert_eq!(completed.status, RecommendationStatus::Accepted);
        assert_eq!(completed.target_session_id, Some("session-active".into()));
        assert_eq!(completed.updated_at, "2026-08-21T12:00:05Z");
    }

    #[test]
    fn complete_execution_rejects_non_executing_status() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put(&recommendation("recommendation-1", "session-1"))
            .unwrap();

        let result = store.complete_execution("recommendation-1", "2026-08-21T12:00:00Z".into());

        assert!(matches!(result, Err(StoreError::InvalidTransition)));
    }

    #[test]
    fn complete_accepted_transitions_to_completed_and_preserves_target_session() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put(&recommendation("recommendation-1", "session-1"))
            .unwrap();
        store
            .begin_execution(
                "recommendation-1",
                "session-active".into(),
                "2026-08-21T12:00:00Z".into(),
            )
            .unwrap();
        store
            .complete_execution("recommendation-1", "2026-08-21T12:00:05Z".into())
            .unwrap();

        let completed = store
            .complete_accepted("recommendation-1", "2026-08-21T12:01:00Z".into())
            .unwrap()
            .unwrap();

        assert_eq!(completed.status, RecommendationStatus::Completed);
        assert_eq!(completed.target_session_id, Some("session-active".into()));
        assert_eq!(completed.updated_at, "2026-08-21T12:01:00Z");
    }

    #[test]
    fn complete_accepted_rejects_non_accepted_status() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put(&recommendation("recommendation-1", "session-1"))
            .unwrap();

        let result = store.complete_accepted("recommendation-1", "2026-08-21T12:00:00Z".into());

        assert!(matches!(result, Err(StoreError::InvalidTransition)));
    }

    #[test]
    fn cancel_execution_rolls_back_to_proposed_for_retry() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put(&recommendation("recommendation-1", "session-1"))
            .unwrap();
        store
            .begin_execution(
                "recommendation-1",
                "session-active".into(),
                "2026-08-21T12:00:00Z".into(),
            )
            .unwrap();

        let cancelled = store
            .cancel_execution("recommendation-1", "2026-08-21T12:00:05Z".into())
            .unwrap()
            .unwrap();

        assert_eq!(cancelled.status, RecommendationStatus::Proposed);
        assert_eq!(cancelled.target_session_id, None);
        assert_eq!(cancelled.updated_at, "2026-08-21T12:00:05Z");

        // Retrying after cancellation must succeed.
        assert!(store
            .begin_execution(
                "recommendation-1",
                "session-active".into(),
                "2026-08-21T12:00:10Z".into(),
            )
            .unwrap()
            .is_some());
    }

    #[test]
    fn dismiss_also_provides_an_escape_hatch_from_a_stuck_execution() {
        // If the sidecar crashes between begin_execution and its matching
        // complete_execution/cancel_execution, the recommendation is
        // permanently stuck at Executing (the evaluator's terminal-status
        // guard leaves anything but proposed/dismissed alone). Dismiss must
        // still work from Executing so the user always has a manual way out.
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store
            .put(&recommendation("recommendation-1", "session-1"))
            .unwrap();
        store
            .begin_execution(
                "recommendation-1",
                "session-active".into(),
                "2026-08-21T12:00:00Z".into(),
            )
            .unwrap();

        let dismissed = store
            .dismiss("recommendation-1", "2026-08-21T12:05:00Z".into())
            .unwrap()
            .unwrap();

        assert_eq!(dismissed.status, RecommendationStatus::Dismissed);
    }

    #[test]
    fn deletes_references_and_scrubs_only_missing_evidence_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store.put(&recommendation("keep", "session-1")).unwrap();
        store.put(&recommendation("delete", "session-2")).unwrap();
        store.delete_referencing_session("session-2").unwrap();
        assert!(store.get("delete").unwrap().is_none());
        assert!(store.get("keep").unwrap().is_some());

        store
            .scrub_orphans(&HashSet::from(["session-1".to_string()]))
            .unwrap();
        assert!(store.get("keep").unwrap().is_some());
    }

    #[test]
    fn applies_a_complete_parent_member_transition_and_excludes_transaction_files() {
        let parent_1_id = stable_rollup_id(&["member-a".into(), "member-b".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member_a = recommendation("member-a", "session-a");
        let member_b = recommendation("member-b", "session-b");
        store.put(&member_a).unwrap();
        store.put(&member_b).unwrap();
        let parent = rollup_parent(parent_1_id.as_str(), &["member-a", "member-b"]);
        let expected = BTreeMap::from([
            (member_a.id.clone(), Some(expected_hash(&member_a))),
            (member_b.id.clone(), Some(expected_hash(&member_b))),
            (parent.id.clone(), None),
        ]);

        store
            .apply_rollup_transaction(&expected, &parent, &[member_a, member_b])
            .unwrap();

        let persisted_parent = store.get(parent_1_id.as_str()).unwrap().unwrap();
        assert_eq!(persisted_parent.rollup_member_ids, ["member-a", "member-b"]);
        assert_eq!(
            store.get("member-a").unwrap().unwrap().status,
            RecommendationStatus::RolledUp
        );
        assert_eq!(
            store.get("member-a").unwrap().unwrap().rolled_up_by,
            Some(parent_1_id.as_str().into())
        );
        assert_eq!(store.list().unwrap().len(), 3);
        assert!(!dir
            .path()
            .join("recommendations/.rollup-transactions")
            .exists());
    }

    #[cfg_attr(windows, ignore = "legacy v1 transaction paths use ':' in filenames")]
    #[test]
    fn persists_loads_and_transacts_with_a_stable_rollup_id() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member_a = recommendation("member-a", "session-a");
        let member_b = recommendation("member-b", "session-b");
        store.put(&member_a).unwrap();
        store.put(&member_b).unwrap();
        let parent_id = stable_rollup_id(&["member-a".into(), "member-b".into()]);
        let parent = rollup_parent(&parent_id, &["member-a", "member-b"]);
        let expected = BTreeMap::from([
            (member_a.id.clone(), Some(expected_hash(&member_a))),
            (member_b.id.clone(), Some(expected_hash(&member_b))),
            (parent.id.clone(), None),
        ]);

        store
            .apply_rollup_transaction(&expected, &parent, &[member_a, member_b])
            .unwrap();

        let encoded_path = store.path_for(&parent_id);
        let legacy_path = dir
            .path()
            .join("recommendations")
            .join(format!("{parent_id}.json"));
        fs::rename(&encoded_path, &legacy_path).unwrap();
        let reopened = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let persisted = reopened.get(&parent_id).unwrap().unwrap();
        assert_eq!(persisted.id, parent_id);
        assert_eq!(persisted.rollup_member_ids, ["member-a", "member-b"]);
    }

    #[test]
    fn encodes_rollup_id_only_in_the_filesystem_filename() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        store.put(&recommendation("member-a", "session-a")).unwrap();
        store.put(&recommendation("member-b", "session-b")).unwrap();
        let parent_id = stable_rollup_id(&["member-a".into(), "member-b".into()]);
        let parent = rollup_parent(&parent_id, &["member-a", "member-b"]);

        assert!(!store
            .path_for(&parent_id)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .contains(':'));

        let expected = BTreeMap::from([
            (
                "member-a".into(),
                Some(expected_hash(&store.get("member-a").unwrap().unwrap())),
            ),
            (
                "member-b".into(),
                Some(expected_hash(&store.get("member-b").unwrap().unwrap())),
            ),
            (parent_id.clone(), None),
        ]);
        store
            .apply_rollup_transaction(
                &expected,
                &parent,
                &[
                    store.get("member-a").unwrap().unwrap(),
                    store.get("member-b").unwrap().unwrap(),
                ],
            )
            .unwrap();
        let persisted = store.get(&parent_id).unwrap().unwrap();
        assert_eq!(persisted.id, parent_id);
        assert_eq!(persisted.rollup_member_ids, ["member-a", "member-b"]);
    }

    #[test]
    fn rejects_a_stale_expected_hash_before_writing_any_graph_file() {
        let parent_id = stable_rollup_id(&["member".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member = recommendation("member", "session");
        store.put(&member).unwrap();
        let mut expected = BTreeMap::from([(member.id.clone(), Some("stale".into()))]);
        let parent = rollup_parent(parent_id.as_str(), &["member"]);

        let result = store.apply_rollup_transaction(&expected, &parent, &[member.clone()]);

        assert!(matches!(result, Err(StoreError::StaleExpectedHash { .. })));
        assert_eq!(store.get("member").unwrap(), Some(member));
        assert!(store.get(parent_id.as_str()).unwrap().is_none());
        expected.clear();
    }

    #[cfg_attr(windows, ignore = "legacy v1 transaction paths use ':' in filenames")]
    #[test]
    fn keeps_the_complete_old_graph_when_an_uncommitted_transaction_is_recovered() {
        let parent_id = stable_rollup_id(&["member".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member = recommendation("member", "session");
        store.put(&member).unwrap();
        let parent = rollup_parent(parent_id.as_str(), &["member"]);
        write_transaction_fixture(dir.path(), false, &[(&member, None)], &[&parent, &member]);

        let recovered = RecommendationStore::open(dir.path().to_path_buf()).unwrap();

        assert_eq!(recovered.get("member").unwrap(), Some(member));
        assert!(recovered.get(parent_id.as_str()).unwrap().is_none());
        assert!(!dir
            .path()
            .join("recommendations/.rollup-transactions")
            .exists());
    }

    #[cfg_attr(windows, ignore = "legacy v1 transaction paths use ':' in filenames")]
    #[test]
    fn recovers_a_legacy_rollup_transaction_with_raw_colon_paths() {
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member_a = recommendation("member-a", "session-a");
        let member_b = recommendation("member-b", "session-b");
        store.put(&member_a).unwrap();
        store.put(&member_b).unwrap();
        let parent_id = stable_rollup_id(&["member-a".into(), "member-b".into()]);
        let parent = rollup_parent(&parent_id, &["member-a", "member-b"]);
        let mut rolled_a = member_a.clone();
        rolled_a.status = RecommendationStatus::RolledUp;
        rolled_a.rolled_up_by = Some(parent_id.clone());
        let mut rolled_b = member_b.clone();
        rolled_b.status = RecommendationStatus::RolledUp;
        rolled_b.rolled_up_by = Some(parent_id.clone());

        write_transaction_fixture(
            dir.path(),
            true,
            &[(&member_a, Some(&rolled_a)), (&member_b, Some(&rolled_b))],
            &[&parent],
        );

        let recovered = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        assert_eq!(recovered.get(&parent_id).unwrap(), Some(parent));
        assert_eq!(
            recovered.get("member-a").unwrap().unwrap().status,
            RecommendationStatus::RolledUp
        );
    }

    #[cfg_attr(windows, ignore = "legacy v1 transaction paths use ':' in filenames")]
    #[test]
    fn finishes_a_committed_transaction_before_serving_reads() {
        let parent_id = stable_rollup_id(&["member".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member = recommendation("member", "session");
        store.put(&member).unwrap();
        let parent = rollup_parent(parent_id.as_str(), &["member"]);
        let mut committed_member = member.clone();
        committed_member.status = RecommendationStatus::RolledUp;
        committed_member.rolled_up_by = Some(parent.id.clone());
        write_transaction_fixture(
            dir.path(),
            true,
            &[(&member, Some(&committed_member))],
            &[&parent, &committed_member],
        );

        let recovered = RecommendationStore::open(dir.path().to_path_buf()).unwrap();

        assert_eq!(recovered.get(parent_id.as_str()).unwrap(), Some(parent));
        assert_eq!(recovered.get("member").unwrap(), Some(committed_member));
    }

    #[test]
    fn rejects_a_graph_with_a_missing_parent_or_ambiguous_active_membership() {
        let parent_a_id = stable_rollup_id(&["member".into()]);
        let parent_b_id = stable_rollup_id(&["member".into(), "member-2".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member = recommendation("member", "session");
        let mut first_parent = rollup_parent(parent_a_id.as_str(), &["member"]);
        let second_parent = rollup_parent(parent_b_id.as_str(), &["member", "member-2"]);
        store.put(&recommendation("member-2", "session-2")).unwrap();
        first_parent.status = RecommendationStatus::Proposed;
        let mut rolled_member = member;
        rolled_member.status = RecommendationStatus::RolledUp;
        rolled_member.rolled_up_by = Some(first_parent.id.clone());
        store.put(&first_parent).unwrap();
        store.put(&second_parent).unwrap();
        store.put(&rolled_member).unwrap();

        let result = store.list();

        assert!(matches!(result, Err(StoreError::GraphInvariant(_))));
    }

    #[test]
    fn rejects_a_graph_with_mismatched_member_dedupe_keys() {
        let parent_id = stable_rollup_id(&["member".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member = recommendation("member", "session");
        let mut parent = rollup_parent(parent_id.as_str(), &["member"]);
        parent.rollup_member_dedupe_keys[0] = "wrong-dedupe-key".into();
        let mut rolled_member = member;
        rolled_member.status = RecommendationStatus::RolledUp;
        rolled_member.rolled_up_by = Some(parent.id.clone());
        store.put(&parent).unwrap();
        store.put(&rolled_member).unwrap();

        assert!(matches!(store.list(), Err(StoreError::GraphInvariant(_))));
    }

    #[test]
    fn rejects_a_stable_looking_parent_id_that_does_not_match_its_members() {
        for status in [
            RecommendationStatus::Proposed,
            RecommendationStatus::Superseded,
        ] {
            let dir = tempfile::tempdir().unwrap();
            let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
            let mut member = recommendation("member", "session");
            let mut parent = rollup_parent(&format!("rollup:{}", "ab".repeat(32)), &["member"]);
            parent.status = status;
            if status == RecommendationStatus::Proposed {
                member.status = RecommendationStatus::RolledUp;
                member.rolled_up_by = Some(parent.id.clone());
            }
            store.put(&member).unwrap();
            store.put(&parent).unwrap();
            assert!(matches!(store.list(), Err(StoreError::GraphInvariant(_))));
        }
    }

    #[cfg(not(windows))]
    #[test]
    fn reports_directory_sync_failures() {
        let missing = tempfile::tempdir().unwrap().path().join("missing");
        assert!(matches!(sync_directory(&missing), Err(StoreError::Io(_))));
    }

    #[cfg(windows)]
    #[test]
    fn treats_directory_sync_as_best_effort_on_windows() {
        let missing = tempfile::tempdir().unwrap().path().join("missing");
        assert!(sync_directory(&missing).is_ok());
    }

    #[test]
    fn releases_unassigned_members_and_supersedes_a_changed_proposed_parent() {
        let parent_old_id = stable_rollup_id(&["member-a".into(), "member-b".into()]);
        let parent_new_id = stable_rollup_id(&["member-a".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member_a = recommendation("member-a", "session-a");
        let member_b = recommendation("member-b", "session-b");
        let old_parent = rollup_parent(parent_old_id.as_str(), &["member-a", "member-b"]);
        let mut old_member_a = member_a.clone();
        old_member_a.status = RecommendationStatus::RolledUp;
        old_member_a.rolled_up_by = Some(old_parent.id.clone());
        let mut old_member_b = member_b.clone();
        old_member_b.status = RecommendationStatus::RolledUp;
        old_member_b.rolled_up_by = Some(old_parent.id.clone());
        store.put(&old_parent).unwrap();
        store.put(&old_member_a).unwrap();
        store.put(&old_member_b).unwrap();
        let mut successor = rollup_parent(parent_new_id.as_str(), &["member-a"]);
        successor.workflow_improvement.supersedes_recommendation_id = Some(old_parent.id.clone());
        let expected = expected_present(&[&old_parent, &old_member_a, &old_member_b]);

        store
            .apply_rollup_transaction(&expected, &successor, &[member_a])
            .unwrap();

        assert_eq!(
            store.get(parent_old_id.as_str()).unwrap().unwrap().status,
            RecommendationStatus::Superseded
        );
        assert_eq!(
            store.get("member-a").unwrap().unwrap().rolled_up_by,
            Some(parent_new_id.as_str().into())
        );
        assert_eq!(
            store.get("member-b").unwrap().unwrap().status,
            RecommendationStatus::Proposed
        );
        assert_eq!(store.get("member-b").unwrap().unwrap().rolled_up_by, None);
    }

    #[test]
    fn session_cleanup_removes_an_entire_parent_member_graph() {
        let parent_id = stable_rollup_id(&["member".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member = recommendation("member", "session-to-remove");
        let parent = rollup_parent(parent_id.as_str(), &["member"]);
        let mut rolled_member = member;
        rolled_member.status = RecommendationStatus::RolledUp;
        rolled_member.rolled_up_by = Some(parent.id.clone());
        store.put(&parent).unwrap();
        store.put(&rolled_member).unwrap();

        store
            .delete_referencing_session("session-to-remove")
            .unwrap();

        assert!(store.get(parent_id.as_str()).unwrap().is_none());
        assert!(store.get("member").unwrap().is_none());
    }

    #[test]
    fn session_cleanup_recovers_pending_rollup_before_deleting_references() {
        let parent_id = stable_rollup_id(&["member-a".into(), "member-b".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member_a = recommendation("member-a", "session-to-remove");
        let member_b = recommendation("member-b", "session-keep");
        store.put(&member_a).unwrap();
        store.put(&member_b).unwrap();
        let parent = rollup_parent(parent_id.as_str(), &["member-a", "member-b"]);
        let expected = BTreeMap::from([
            (member_a.id.clone(), Some(expected_hash(&member_a))),
            (member_b.id.clone(), Some(expected_hash(&member_b))),
            (parent.id.clone(), None),
        ]);
        set_fault_point(Some(FaultPoint::Publication(1)));
        assert!(store
            .apply_rollup_transaction(&expected, &parent, &[member_a, member_b])
            .is_err());
        set_fault_point(None);

        store
            .delete_referencing_session("session-to-remove")
            .unwrap();

        assert!(store.get(parent_id.as_str()).unwrap().is_none());
        assert!(store.get("member-a").unwrap().is_none());
        assert!(store.get("member-b").unwrap().is_none());
    }

    #[test]
    fn session_cleanup_deletes_superseded_parents_that_still_reference_deleted_members() {
        let parent_old_id = stable_rollup_id(&["member-a".into(), "member-b".into()]);
        let parent_new_id = stable_rollup_id(&["member-a".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member_a = recommendation("member-a", "session-to-remove");
        let member_b = recommendation("member-b", "session-keep");
        let old_parent = rollup_parent(parent_old_id.as_str(), &["member-a", "member-b"]);
        let mut old_member_a = member_a.clone();
        old_member_a.status = RecommendationStatus::RolledUp;
        old_member_a.rolled_up_by = Some(old_parent.id.clone());
        let mut old_member_b = member_b.clone();
        old_member_b.status = RecommendationStatus::RolledUp;
        old_member_b.rolled_up_by = Some(old_parent.id.clone());
        store.put(&old_parent).unwrap();
        store.put(&old_member_a).unwrap();
        store.put(&old_member_b).unwrap();

        let mut successor = rollup_parent(parent_new_id.as_str(), &["member-a"]);
        successor.workflow_improvement.supersedes_recommendation_id = Some(old_parent.id.clone());
        store
            .apply_rollup_transaction(
                &expected_present(&[&old_parent, &old_member_a, &old_member_b]),
                &successor,
                &[member_a],
            )
            .unwrap();

        store
            .delete_referencing_session("session-to-remove")
            .unwrap();

        assert!(store.get(parent_old_id.as_str()).unwrap().is_none());
        assert!(store.get(parent_new_id.as_str()).unwrap().is_none());
        assert!(store.get("member-a").unwrap().is_none());
        assert!(store.get("member-b").unwrap().is_none());
    }

    #[test]
    fn orphan_cleanup_recovers_pending_rollup_before_scrubbing() {
        let parent_id = stable_rollup_id(&["member-a".into(), "member-b".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member_a = recommendation("member-a", "session-orphan");
        let member_b = recommendation("member-b", "session-keep");
        store.put(&member_a).unwrap();
        store.put(&member_b).unwrap();
        let parent = rollup_parent(parent_id.as_str(), &["member-a", "member-b"]);
        let expected = BTreeMap::from([
            (member_a.id.clone(), Some(expected_hash(&member_a))),
            (member_b.id.clone(), Some(expected_hash(&member_b))),
            (parent.id.clone(), None),
        ]);
        set_fault_point(Some(FaultPoint::Publication(1)));
        assert!(store
            .apply_rollup_transaction(&expected, &parent, &[member_a, member_b])
            .is_err());
        set_fault_point(None);

        store
            .scrub_orphans(&HashSet::from(["session-keep".to_string()]))
            .unwrap();

        assert!(store.get(parent_id.as_str()).unwrap().is_none());
        assert!(store.get("member-a").unwrap().is_none());
        assert!(store.get("member-b").unwrap().is_none());
    }

    #[test]
    fn refuses_to_reuse_a_terminal_parent_id() {
        let parent_id = stable_rollup_id(&["member".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let member = recommendation("member", "session");
        let mut terminal_parent = rollup_parent(parent_id.as_str(), &["member"]);
        terminal_parent.status = RecommendationStatus::Dismissed;
        store.put(&terminal_parent).unwrap();
        store.put(&member).unwrap();
        let successor = rollup_parent(parent_id.as_str(), &["member"]);
        let expected = expected_present(&[&terminal_parent, &member]);

        let result = store.apply_rollup_transaction(&expected, &successor, &[member.clone()]);

        assert!(matches!(result, Err(StoreError::InvalidTransition)));
        assert_eq!(
            store.get(parent_id.as_str()).unwrap(),
            Some(terminal_parent)
        );
        assert_eq!(store.get("member").unwrap(), Some(member));
    }

    #[test]
    fn rejects_missing_or_colliding_member_ids_before_staging() {
        let parent_id = stable_rollup_id(&["missing".into()]);
        let nested_parent_id = stable_rollup_id(&["existing".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let existing = recommendation("existing", "session");
        store.put(&existing).unwrap();

        let missing_parent = rollup_parent(parent_id.as_str(), &["missing"]);
        let missing = store.apply_rollup_transaction(
            &BTreeMap::from([(parent_id.as_str().into(), None)]),
            &missing_parent,
            &[recommendation("missing", "session")],
        );
        assert!(matches!(missing, Err(StoreError::GraphInvariant(_))));
        assert!(store.get(parent_id.as_str()).unwrap().is_none());
        assert!(store.get("missing").unwrap().is_none());

        let collision_parent = rollup_parent("existing", &["existing"]);
        let collision = store.apply_rollup_transaction(
            &BTreeMap::from([("existing".into(), Some(expected_hash(&existing)))]),
            &collision_parent,
            &[existing.clone()],
        );
        assert!(matches!(collision, Err(StoreError::GraphInvariant(_))));
        assert_eq!(store.get("existing").unwrap(), Some(existing.clone()));

        let mut nested_member = existing.clone();
        nested_member.rollup_member_ids = vec!["child".into()];
        nested_member.rollup_member_dedupe_keys = vec!["child-dedupe".into()];
        let nested_parent = rollup_parent(nested_parent_id.as_str(), &["existing"]);
        let nested = store.apply_rollup_transaction(
            &BTreeMap::from([("existing".into(), Some(expected_hash(&existing)))]),
            &nested_parent,
            &[nested_member],
        );
        assert!(matches!(nested, Err(StoreError::GraphInvariant(_))));
        assert!(store.get(nested_parent_id.as_str()).unwrap().is_none());
    }

    #[test]
    fn rejects_a_malformed_result_graph_without_publishing_it() {
        let parent_id = stable_rollup_id(&["member".into()]);
        let dir = tempfile::tempdir().unwrap();
        let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
        let old = recommendation("member", "session");
        store.put(&old).unwrap();
        let mut malformed_parent = rollup_parent(parent_id.as_str(), &["member"]);
        malformed_parent.status = RecommendationStatus::Proposed;
        let malformed_bytes = serde_json::to_vec_pretty(&malformed_parent).unwrap();
        let old_bytes = serde_json::to_vec_pretty(&old).unwrap();
        let result = store.commit_replacements(
            &BTreeMap::from([
                ("member".into(), Some(hash_bytes(&old_bytes))),
                (parent_id.as_str().into(), None),
            ]),
            BTreeMap::from([
                (
                    parent_id.as_str().into(),
                    Replacement {
                        old: None,
                        new: Some(malformed_bytes),
                    },
                ),
                (
                    "member".into(),
                    Replacement {
                        old: Some(old_bytes.clone()),
                        new: Some(old_bytes),
                    },
                ),
            ]),
        );

        assert!(matches!(result, Err(StoreError::GraphInvariant(_))));
        assert!(store.get(parent_id.as_str()).unwrap().is_none());
        assert_eq!(store.get("member").unwrap(), Some(old));
    }

    #[test]
    fn fault_points_recover_to_a_complete_old_or_new_graph() {
        let parent_id = stable_rollup_id(&["member-a".into(), "member-b".into()]);
        for fault in [
            FaultPoint::Staging,
            FaultPoint::ManifestCommit,
            FaultPoint::Publication(1),
            FaultPoint::Cleanup,
        ] {
            let dir = tempfile::tempdir().unwrap();
            let store = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
            let member_a = recommendation("member-a", "session-a");
            let member_b = recommendation("member-b", "session-b");
            store.put(&member_a).unwrap();
            store.put(&member_b).unwrap();
            let parent = rollup_parent(parent_id.as_str(), &["member-a", "member-b"]);
            let expected = BTreeMap::from([
                (member_a.id.clone(), Some(expected_hash(&member_a))),
                (member_b.id.clone(), Some(expected_hash(&member_b))),
                (parent.id.clone(), None),
            ]);
            set_fault_point(Some(fault));
            let result = store.apply_rollup_transaction(&expected, &parent, &[member_a, member_b]);
            set_fault_point(None);
            assert!(result.is_err());

            let recovered = RecommendationStore::open(dir.path().to_path_buf()).unwrap();
            let parent_exists = recovered.get(parent_id.as_str()).unwrap().is_some();
            let member_a = recovered.get("member-a").unwrap().unwrap();
            let member_b = recovered.get("member-b").unwrap().unwrap();
            if parent_exists {
                assert_eq!(member_a.status, RecommendationStatus::RolledUp);
                assert_eq!(member_a.rolled_up_by, Some(parent_id.as_str().into()));
                assert_eq!(member_b.status, RecommendationStatus::RolledUp);
                assert_eq!(member_b.rolled_up_by, Some(parent_id.as_str().into()));
            } else {
                assert_eq!(member_a.status, RecommendationStatus::Proposed);
                assert_eq!(member_a.rolled_up_by, None);
                assert_eq!(member_b.status, RecommendationStatus::Proposed);
                assert_eq!(member_b.rolled_up_by, None);
            }
            assert!(recovered.list().is_ok());
        }
    }

    fn write_transaction_fixture(
        root: &Path,
        committed: bool,
        old_and_new: &[(&Recommendation, Option<&Recommendation>)],
        new_records: &[&Recommendation],
    ) {
        let transaction = root.join("recommendations/.rollup-transactions/fixture");
        fs::create_dir_all(transaction.join("staged")).unwrap();
        fs::create_dir_all(transaction.join("backups")).unwrap();
        let mut entries = Vec::new();
        for (old, new) in old_and_new {
            let old_path = root.join(format!("recommendations/{}.json", old.id));
            let old_bytes = fs::read(&old_path).unwrap();
            fs::write(
                transaction.join(format!("backups/{}.json", old.id)),
                &old_bytes,
            )
            .unwrap();
            let new_record = new.unwrap_or(old);
            let staged_path = transaction.join(format!("staged/{}.json", old.id));
            fs::write(&staged_path, serde_json::to_vec_pretty(new_record).unwrap()).unwrap();
            entries.push(serde_json::json!({
                "id": old.id,
                "target": format!("{}.json", old.id),
                "staged": format!("staged/{}.json", old.id),
                "backup": format!("backups/{}.json", old.id),
                "oldSha256": format!("{:x}", Sha256::digest(&old_bytes)),
                "oldExists": true,
                "newSha256": format!("{:x}", Sha256::digest(serde_json::to_vec_pretty(new_record).unwrap())),
                "newExists": true,
            }));
        }
        for new in new_records {
            if old_and_new.iter().any(|(old, _)| old.id == new.id) {
                continue;
            }
            let staged_path = transaction.join(format!("staged/{}.json", new.id));
            fs::write(&staged_path, serde_json::to_vec_pretty(new).unwrap()).unwrap();
            entries.push(serde_json::json!({
                "id": new.id,
                "target": format!("{}.json", new.id),
                "staged": format!("staged/{}.json", new.id),
                "backup": serde_json::Value::Null,
                "oldSha256": serde_json::Value::Null,
                "oldExists": false,
                "newSha256": format!("{:x}", Sha256::digest(serde_json::to_vec_pretty(new).unwrap())),
                "newExists": true,
            }));
        }
        fs::write(
            transaction.join("manifest.json"),
            serde_json::to_vec_pretty(&serde_json::json!({
                "version": 1,
                "committed": committed,
                "entries": entries,
            }))
            .unwrap(),
        )
        .unwrap();
    }
}
