use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const COMPLETION_PACKET_SCHEMA_VERSION: u32 = 1;
pub(crate) const COMPLETION_EVIDENCE_VERSION: u32 = 1;
const MAX_PACKET_ID_CHARS: usize = 128;
const MAX_SCOPE_CHARS: usize = 512;
const MAX_PATHS: usize = 512;
const MAX_LINEAGE: usize = 16;

fn valid_text(value: &str, max: usize) -> bool {
    !value.is_empty() && value.chars().count() <= max && value.chars().all(|ch| !ch.is_control())
}

fn valid_fingerprint(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PacketAttribution {
    Unambiguous,
    Inconclusive,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PacketReadiness {
    VerificationNeeded,
    ReviewReady,
    FindingsNeedFix,
    ReadyForUserReview,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompletionVerificationResult {
    Passed,
    Failed,
    NotRun,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompletionReviewOutcome {
    NoFindings,
    Findings,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompletionActionRole {
    Review,
    Verification,
    Fix,
    UserReview,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CompletionEvidenceIssueKind {
    Missing,
    Conflict,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionEvidenceProvenance {
    pub source_session_id: String,
    pub workspace_id: String,
    pub workspace_snapshot_id: String,
    pub observed_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionChangeSubject {
    pub workspace_id: String,
    pub scope: String,
    pub snapshot_id: String,
    pub attribution: PacketAttribution,
    pub changed_paths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionVerification {
    pub command: String,
    pub result: CompletionVerificationResult,
    pub applicable_revision: String,
    pub observed_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionReview {
    pub reviewer_session_id: String,
    pub reviewer_identity: String,
    pub independent: bool,
    pub outcome: CompletionReviewOutcome,
    pub applicable_revision: String,
    pub observed_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionEvidenceIssue {
    pub kind: CompletionEvidenceIssueKind,
    pub detail: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionAction {
    pub prompt: String,
    pub model: Option<String>,
    pub scope: String,
    pub role: CompletionActionRole,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionApproval {
    pub approved_at: String,
    pub approver: String,
    pub revision: u64,
    pub evidence_fingerprint: String,
    pub action_fingerprint: String,
    pub idempotency_key: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionPacketLineage {
    pub packet_id: String,
    pub revision: u64,
    pub evidence_fingerprint: String,
    pub superseded_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionPacket {
    pub schema_version: u32,
    pub evidence_version: u32,
    pub packet_id: String,
    pub source_session_id: String,
    pub observed_at: String,
    pub provenance: CompletionEvidenceProvenance,
    pub subject: CompletionChangeSubject,
    pub verification: Option<CompletionVerification>,
    pub review: Option<CompletionReview>,
    pub missing_evidence: Vec<CompletionEvidenceIssue>,
    pub conflicting_evidence: Vec<CompletionEvidenceIssue>,
    pub action: CompletionAction,
    pub readiness: PacketReadiness,
    pub revision: u64,
    pub evidence_fingerprint: String,
    pub approval: Option<CompletionApproval>,
    pub supersedes_packet_id: Option<String>,
    pub lineage: Vec<CompletionPacketLineage>,
    pub completion_idempotency_key: Option<String>,
}

impl CompletionPacket {
    pub(crate) fn computed_evidence_fingerprint(&self) -> String {
        #[derive(Serialize)]
        struct Evidence<'a> {
            evidence_version: u32,
            source_session_id: &'a str,
            observed_at: &'a str,
            provenance: &'a CompletionEvidenceProvenance,
            subject: &'a CompletionChangeSubject,
            verification: &'a Option<CompletionVerification>,
            review: &'a Option<CompletionReview>,
            missing_evidence: &'a [CompletionEvidenceIssue],
            conflicting_evidence: &'a [CompletionEvidenceIssue],
        }
        let bytes = serde_json::to_vec(&Evidence {
            evidence_version: self.evidence_version,
            source_session_id: &self.source_session_id,
            observed_at: &self.observed_at,
            provenance: &self.provenance,
            subject: &self.subject,
            verification: &self.verification,
            review: &self.review,
            missing_evidence: &self.missing_evidence,
            conflicting_evidence: &self.conflicting_evidence,
        })
        .expect("completion evidence is serializable");
        hex::encode(Sha256::digest(bytes))
    }

    pub(crate) fn action_fingerprint(&self) -> String {
        let bytes = serde_json::to_vec(&self.action).expect("completion action is serializable");
        hex::encode(Sha256::digest(bytes))
    }

    pub(crate) fn derived_readiness(&self) -> PacketReadiness {
        if self.subject.attribution == PacketAttribution::Inconclusive
            || !self.missing_evidence.is_empty()
            || !self.conflicting_evidence.is_empty()
            || !self.verification.as_ref().is_some_and(|verification| {
                verification.result == CompletionVerificationResult::Passed
                    && verification.applicable_revision == self.subject.snapshot_id
            })
        {
            return PacketReadiness::VerificationNeeded;
        }
        match self.review.as_ref() {
            None => PacketReadiness::ReviewReady,
            Some(review)
                if review.outcome == CompletionReviewOutcome::Findings
                    && review.applicable_revision == self.subject.snapshot_id =>
            {
                PacketReadiness::FindingsNeedFix
            }
            Some(review)
                if review.outcome == CompletionReviewOutcome::NoFindings
                    && review.independent
                    && review.reviewer_session_id != self.source_session_id
                    && review.applicable_revision == self.subject.snapshot_id =>
            {
                PacketReadiness::ReadyForUserReview
            }
            Some(_) => PacketReadiness::VerificationNeeded,
        }
    }

    pub(crate) fn validate(&self, workspace_id: &str) -> Result<(), String> {
        if self.schema_version != COMPLETION_PACKET_SCHEMA_VERSION {
            return Err("unsupported completion packet schema version".into());
        }
        if self.evidence_version != COMPLETION_EVIDENCE_VERSION {
            return Err("unsupported completion evidence version".into());
        }
        for (name, value, max) in [
            ("packet_id", self.packet_id.as_str(), MAX_PACKET_ID_CHARS),
            ("source_session_id", self.source_session_id.as_str(), 256),
            ("observed_at", self.observed_at.as_str(), 128),
            (
                "provenance.source_session_id",
                self.provenance.source_session_id.as_str(),
                256,
            ),
            (
                "provenance.workspace_id",
                self.provenance.workspace_id.as_str(),
                256,
            ),
            (
                "provenance.workspace_snapshot_id",
                self.provenance.workspace_snapshot_id.as_str(),
                256,
            ),
            (
                "provenance.observed_at",
                self.provenance.observed_at.as_str(),
                128,
            ),
            (
                "subject.workspace_id",
                self.subject.workspace_id.as_str(),
                256,
            ),
            (
                "subject.scope",
                self.subject.scope.as_str(),
                MAX_SCOPE_CHARS,
            ),
            (
                "subject.snapshot_id",
                self.subject.snapshot_id.as_str(),
                256,
            ),
            ("action.prompt", self.action.prompt.as_str(), 8_192),
            ("action.scope", self.action.scope.as_str(), MAX_SCOPE_CHARS),
        ] {
            if !valid_text(value, max) {
                return Err(format!("completion packet field {name} is invalid"));
            }
        }
        if self
            .action
            .model
            .as_ref()
            .is_some_and(|model| !valid_text(model, 256))
        {
            return Err("completion packet action model is invalid".into());
        }
        if self.source_session_id != self.provenance.source_session_id {
            return Err("completion packet provenance source session does not match".into());
        }
        if self.provenance.workspace_id != workspace_id
            || self.subject.workspace_id != workspace_id
            || self.provenance.workspace_snapshot_id != self.subject.snapshot_id
        {
            return Err("completion packet is not scoped to this workspace snapshot".into());
        }
        if self.subject.changed_paths.len() > MAX_PATHS
            || self
                .subject
                .changed_paths
                .iter()
                .any(|path| !valid_text(path, 1_024))
        {
            return Err("completion packet changed paths are invalid".into());
        }
        if self.revision == 0 || self.evidence_fingerprint != self.computed_evidence_fingerprint() {
            return Err("completion packet evidence fingerprint is invalid".into());
        }
        if self.readiness != self.derived_readiness() {
            return Err("completion packet readiness is not derived from its evidence".into());
        }
        if let Some(verification) = &self.verification {
            if !valid_text(&verification.command, 4_096)
                || verification.applicable_revision != self.subject.snapshot_id
                || !valid_text(&verification.observed_at, 128)
            {
                return Err("completion packet verification evidence is invalid".into());
            }
        }
        if let Some(review) = &self.review {
            if !valid_text(&review.reviewer_session_id, 256)
                || !valid_text(&review.reviewer_identity, 256)
                || review.applicable_revision != self.subject.snapshot_id
                || !valid_text(&review.observed_at, 128)
            {
                return Err("completion packet review evidence is invalid".into());
            }
        }
        if self
            .supersedes_packet_id
            .as_ref()
            .is_some_and(|id| !valid_text(id, MAX_PACKET_ID_CHARS) || id == &self.packet_id)
        {
            return Err("completion packet supersession identity is invalid".into());
        }
        if self.lineage.len() > MAX_LINEAGE {
            return Err("completion packet lineage is too long".into());
        }
        let mut previous_revision = 0;
        for entry in &self.lineage {
            if !valid_text(&entry.packet_id, MAX_PACKET_ID_CHARS)
                || entry.revision == 0
                || entry.revision <= previous_revision
                || !valid_fingerprint(&entry.evidence_fingerprint)
                || !valid_text(&entry.superseded_at, 128)
            {
                return Err("completion packet lineage is invalid".into());
            }
            previous_revision = entry.revision;
        }
        if self
            .lineage
            .last()
            .is_some_and(|entry| entry.revision >= self.revision)
            || self.supersedes_packet_id.is_some() != !self.lineage.is_empty()
        {
            return Err("completion packet supersession lineage is invalid".into());
        }
        if let Some(approval) = &self.approval {
            if approval.revision != self.revision
                || approval.evidence_fingerprint != self.evidence_fingerprint
                || approval.action_fingerprint != self.action_fingerprint()
                || approval.approved_at.is_empty()
                || !valid_text(&approval.approved_at, 128)
                || !valid_text(&approval.approver, 256)
                || !valid_text(&approval.idempotency_key, 128)
                || approval.idempotency_key.chars().any(char::is_whitespace)
            {
                return Err("completion packet approval is stale or malformed".into());
            }
        }
        if self
            .completion_idempotency_key
            .as_ref()
            .is_some_and(|key| !valid_text(key, 128) || key.chars().any(char::is_whitespace))
        {
            return Err("completion packet idempotency key is invalid".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompletionMutationRequest {
    pub packet_revision: u64,
    pub evidence_fingerprint: String,
    pub idempotency_key: String,
}

impl CompletionMutationRequest {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.packet_revision == 0
            || !valid_fingerprint(&self.evidence_fingerprint)
            || !valid_text(&self.idempotency_key, 128)
            || self.idempotency_key.chars().any(char::is_whitespace)
        {
            return Err("completion mutation request is invalid".into());
        }
        Ok(())
    }
}
