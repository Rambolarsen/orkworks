//! Bounded, data-only coordinator records. Runtime authority is deliberately absent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

const MAX_ID: usize = 128;
const MAX_TEXT: usize = 16 * 1024;
const MAX_ITEMS: usize = 256;
const MAX_NODES: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CoordinatorError {
    Invalid(&'static str),
    HardDenied(CapabilityKind),
    Serialization(String),
}

fn bounded(value: &str, max: usize, field: &'static str) -> Result<(), CoordinatorError> {
    if value.is_empty() || value.len() > max || value.chars().any(char::is_control) {
        return Err(CoordinatorError::Invalid(field));
    }
    Ok(())
}

fn id(value: &str, field: &'static str) -> Result<(), CoordinatorError> {
    bounded(value, MAX_ID, field)?;
    if !value
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
    {
        return Err(CoordinatorError::Invalid(field));
    }
    Ok(())
}

fn strings(values: &[String], max: usize, field: &'static str) -> Result<(), CoordinatorError> {
    if values.len() > MAX_ITEMS {
        return Err(CoordinatorError::Invalid(field));
    }
    for value in values {
        bounded(value, max, field)?;
    }
    Ok(())
}

fn digest(value: &str, field: &'static str) -> Result<(), CoordinatorError> {
    if value.len() != 64 || !value.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(CoordinatorError::Invalid(field));
    }
    Ok(())
}

fn scopes(values: &[String]) -> Result<(), CoordinatorError> {
    strings(values, MAX_TEXT, "scope")?;
    for path in values {
        if path.starts_with('/')
            || path.contains('\\')
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            return Err(CoordinatorError::Invalid("noncanonical scope"));
        }
    }
    Ok(())
}

fn canonical_value(value: &Value) -> Result<Vec<u8>, CoordinatorError> {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<_> = map.keys().collect();
            keys.sort();
            let mut out = vec![b'{'];
            for (index, key) in keys.into_iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                out.extend(
                    serde_json::to_vec(key)
                        .map_err(|e| CoordinatorError::Serialization(e.to_string()))?,
                );
                out.push(b':');
                out.extend(canonical_value(&map[key])?);
            }
            out.push(b'}');
            Ok(out)
        }
        Value::Array(items) => {
            let mut out = vec![b'['];
            for (index, item) in items.iter().enumerate() {
                if index != 0 {
                    out.push(b',');
                }
                out.extend(canonical_value(item)?);
            }
            out.push(b']');
            Ok(out)
        }
        _ => serde_json::to_vec(value).map_err(|e| CoordinatorError::Serialization(e.to_string())),
    }
}

pub(crate) fn canonical_json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CoordinatorError> {
    value
        .serialize(FiniteProbe)
        .map_err(|e| CoordinatorError::Serialization(e.to_string()))?;
    let json =
        serde_json::to_value(value).map_err(|e| CoordinatorError::Serialization(e.to_string()))?;
    canonical_value(&json)
}

pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Attribution {
    Conclusive,
    Inconclusive,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct WorkspaceChangeSubject {
    pub workspace_id: String,
    pub repository_revision: String,
    pub dirty_paths: Vec<String>,
    pub attribution: Attribution,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanEvidence {
    pub version: u32,
    pub subject: String,
    pub references: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Role {
    Implementation,
    Verification,
    Review,
    Remediation,
    Custom(String),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub task: String,
    pub success_criteria: Vec<String>,
    pub prompt_context: String,
    pub prompt_context_digest: String,
    pub output_contract: String,
    pub scope: Vec<String>,
    pub role: Role,
    pub allowed_tools: Vec<String>,
    pub provider_allowlist: Vec<String>,
    pub dependencies: Vec<String>,
    pub budget_units: u64,
    pub retry_limit: u32,
    pub concurrency_class: String,
}

impl PlanNode {
    fn validate(&self) -> Result<(), CoordinatorError> {
        id(&self.id, "node id")?;
        if let Some(parent) = &self.parent_id {
            id(parent, "parent id")?;
        }
        bounded(&self.task, MAX_TEXT, "task")?;
        if self.success_criteria.is_empty() {
            return Err(CoordinatorError::Invalid("success criteria"));
        }
        strings(&self.success_criteria, MAX_TEXT, "success criteria")?;
        bounded(&self.prompt_context, MAX_TEXT, "prompt context")?;
        digest(&self.prompt_context_digest, "prompt digest")?;
        if self.prompt_context_digest != sha256_hex(self.prompt_context.as_bytes()) {
            return Err(CoordinatorError::Invalid("prompt digest"));
        }
        bounded(&self.output_contract, MAX_TEXT, "output contract")?;
        if self.scope.is_empty() {
            return Err(CoordinatorError::Invalid("scope"));
        }
        scopes(&self.scope)?;
        if self.allowed_tools.is_empty() {
            return Err(CoordinatorError::Invalid("allowed tools"));
        }
        strings(&self.allowed_tools, MAX_ID, "allowed tools")?;
        for tool in &self.allowed_tools {
            known_tool(tool)?;
        }
        if self.provider_allowlist.is_empty() {
            return Err(CoordinatorError::Invalid("provider allowlist"));
        }
        strings(&self.provider_allowlist, MAX_ID, "provider allowlist")?;
        for provider in &self.provider_allowlist {
            id(provider, "provider")?;
        }
        strings(&self.dependencies, MAX_ID, "dependencies")?;
        for dep in &self.dependencies {
            id(dep, "dependency")?;
        }
        bounded(&self.concurrency_class, MAX_ID, "concurrency class")?;
        if self.budget_units == 0 {
            return Err(CoordinatorError::Invalid("budget units"));
        }
        if let Role::Custom(name) = &self.role {
            id(name, "role")?;
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanRevision {
    pub version: u32,
    pub instance_id: String,
    pub workspace_id: String,
    pub plan_id: String,
    pub revision: u64,
    pub revocation_generation: u64,
    pub supersedes: Option<String>,
    pub subject: WorkspaceChangeSubject,
    pub evidence: PlanEvidence,
    pub nodes: Vec<PlanNode>,
    pub total_budget_units: u64,
    pub max_concurrency: u32,
    pub plan_digest: Option<String>,
    pub evidence_digest: Option<String>,
}

#[derive(Serialize)]
struct PlanDigestMaterial<'a> {
    version: u32,
    instance_id: &'a str,
    workspace_id: &'a str,
    plan_id: &'a str,
    revision: u64,
    revocation_generation: u64,
    supersedes: &'a Option<String>,
    subject: &'a WorkspaceChangeSubject,
    evidence: &'a PlanEvidence,
    nodes: &'a [PlanNode],
    total_budget_units: u64,
    max_concurrency: u32,
}

impl PlanRevision {
    pub(crate) fn compute_plan_digest(&self) -> Result<String, CoordinatorError> {
        let material = PlanDigestMaterial {
            version: self.version,
            instance_id: &self.instance_id,
            workspace_id: &self.workspace_id,
            plan_id: &self.plan_id,
            revision: self.revision,
            revocation_generation: self.revocation_generation,
            supersedes: &self.supersedes,
            subject: &self.subject,
            evidence: &self.evidence,
            nodes: &self.nodes,
            total_budget_units: self.total_budget_units,
            max_concurrency: self.max_concurrency,
        };
        Ok(sha256_hex(&canonical_json_bytes(&material)?))
    }

    pub(crate) fn compute_evidence_digest(&self) -> Result<String, CoordinatorError> {
        Ok(sha256_hex(&canonical_json_bytes(&(
            self.evidence.version,
            &self.subject,
            &self.evidence,
        ))?))
    }

    pub(crate) fn validate(&self) -> Result<(), CoordinatorError> {
        if self.version != 1 || self.evidence.version != 1 {
            return Err(CoordinatorError::Invalid("version"));
        }
        id(&self.instance_id, "instance id")?;
        id(&self.workspace_id, "workspace id")?;
        id(&self.plan_id, "plan id")?;
        if self.revision == 0 {
            return Err(CoordinatorError::Invalid("revision"));
        }
        if let Some(previous) = &self.supersedes {
            id(previous, "supersedes")?;
        }
        if self.subject.workspace_id != self.workspace_id
            || self.subject.attribution != Attribution::Conclusive
        {
            return Err(CoordinatorError::Invalid("workspace change subject"));
        }
        bounded(
            &self.subject.repository_revision,
            MAX_ID,
            "repository revision",
        )?;
        strings(&self.subject.dirty_paths, MAX_TEXT, "dirty paths")?;
        bounded(&self.evidence.subject, MAX_TEXT, "evidence subject")?;
        strings(&self.evidence.references, MAX_TEXT, "evidence references")?;
        if self.nodes.is_empty() || self.nodes.len() > MAX_NODES {
            return Err(CoordinatorError::Invalid("nodes"));
        }
        let mut ids = std::collections::HashSet::new();
        for node in &self.nodes {
            node.validate()?;
            if !ids.insert(node.id.as_str()) {
                return Err(CoordinatorError::Invalid("duplicate node"));
            }
        }
        for node in &self.nodes {
            if node.parent_id.as_deref().is_some_and(|p| !ids.contains(p))
                || node.dependencies.iter().any(|d| !ids.contains(d.as_str()))
            {
                return Err(CoordinatorError::Invalid("graph reference"));
            }
        }
        if self.total_budget_units == 0 || self.max_concurrency == 0 {
            return Err(CoordinatorError::Invalid("limits"));
        }
        let reserved = self
            .nodes
            .iter()
            .try_fold(0_u64, |sum, node| sum.checked_add(node.budget_units))
            .ok_or(CoordinatorError::Invalid("budget overflow"))?;
        if reserved > self.total_budget_units {
            return Err(CoordinatorError::Invalid("budget"));
        }
        if let Some(supplied) = &self.plan_digest {
            if supplied != &self.compute_plan_digest()? {
                return Err(CoordinatorError::Invalid("plan digest"));
            }
        }
        if let Some(supplied) = &self.evidence_digest {
            if supplied != &self.compute_evidence_digest()? {
                return Err(CoordinatorError::Invalid("evidence digest"));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanApproval {
    pub version: u32,
    pub approval_id: String,
    pub user_id: String,
    pub instance_id: String,
    pub workspace_id: String,
    pub plan_id: String,
    pub revision: u64,
    pub plan_digest: String,
    pub evidence_digest: String,
    pub approved_at: String,
    pub expires_at: String,
    pub revocation_generation: u64,
    pub supersedes: Option<String>,
}

impl PlanApproval {
    pub(crate) fn validate_against(&self, plan: &PlanRevision) -> Result<(), CoordinatorError> {
        plan.validate()?;
        if self.version != 1 {
            return Err(CoordinatorError::Invalid("approval version"));
        }
        id(&self.approval_id, "approval id")?;
        id(&self.user_id, "user id")?;
        let approved: DateTime<Utc> = self
            .approved_at
            .parse()
            .map_err(|_| CoordinatorError::Invalid("approved at"))?;
        let expires: DateTime<Utc> = self
            .expires_at
            .parse()
            .map_err(|_| CoordinatorError::Invalid("expires at"))?;
        if expires <= approved || expires <= Utc::now() {
            return Err(CoordinatorError::Invalid("approval expired"));
        }
        if self.instance_id != plan.instance_id
            || self.workspace_id != plan.workspace_id
            || self.plan_id != plan.plan_id
            || self.revision != plan.revision
            || self.plan_digest != plan.compute_plan_digest()?
            || self.evidence_digest != plan.compute_evidence_digest()?
            || self.revocation_generation != plan.revocation_generation
            || self.supersedes != plan.supersedes
        {
            return Err(CoordinatorError::Invalid("approval binding"));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PlanStatus {
    Draft,
    Proposed,
    Active,
    ReadyForUserReview,
    Paused,
    RecoveryRequired,
    Completed,
    Failed,
    Cancelled,
    Expired,
    Revoked,
}

impl PlanStatus {
    pub(crate) fn can_activate(
        self,
        approval: &PlanApproval,
        plan: &PlanRevision,
        current_generation: u64,
    ) -> Result<(), CoordinatorError> {
        if self != Self::Proposed || approval.revocation_generation != current_generation {
            return Err(CoordinatorError::Invalid("activation state"));
        }
        approval.validate_against(plan)
    }
    pub(crate) fn allows_transition(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Proposed)
                | (
                    Self::Proposed,
                    Self::Active | Self::Paused | Self::RecoveryRequired
                )
                | (
                    Self::Active,
                    Self::ReadyForUserReview
                        | Self::Paused
                        | Self::RecoveryRequired
                        | Self::Completed
                        | Self::Failed
                        | Self::Cancelled
                        | Self::Expired
                        | Self::Revoked
                )
                | (
                    Self::ReadyForUserReview,
                    Self::Active | Self::Failed | Self::Cancelled | Self::Expired | Self::Revoked
                )
                | (
                    Self::Paused,
                    Self::Active | Self::Cancelled | Self::Expired | Self::Revoked
                )
        )
    }
    pub(crate) fn can_launch(self) -> bool {
        self == Self::Active
    }
    pub(crate) fn is_terminal_success(self) -> bool {
        self == Self::Completed
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityKind {
    ReadFile,
    WriteFile,
    RunBoundedCommand,
    GitRead,
    GitMutation,
    Credentials,
    PermissionChange,
    DestructiveCommand,
    MergeApproval,
    ProviderSubstitution,
    ScopeExpansion,
    BudgetExpansion,
    RetryExpansion,
    ConcurrencyExpansion,
}

impl CapabilityKind {
    pub(crate) fn is_hard_denied(self) -> bool {
        !matches!(
            self,
            Self::ReadFile | Self::WriteFile | Self::RunBoundedCommand | Self::GitRead
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct CapabilityRequest {
    pub version: u32,
    pub node_id: String,
    pub contract_reference: String,
    pub current_capability_digest: String,
    pub kind: CapabilityKind,
    pub scope: Vec<String>,
    pub tool_id: String,
    pub expected_cost: u64,
    pub reason: String,
    pub evidence: Vec<String>,
}

fn known_tool(tool: &str) -> Result<(), CoordinatorError> {
    id(tool, "tool id")?;
    if !matches!(
        tool,
        "read_file" | "write_file" | "run_bounded_command" | "git_read"
    ) {
        return Err(CoordinatorError::Invalid("unknown tool"));
    }
    Ok(())
}

impl CapabilityRequest {
    pub(crate) fn validate(&self) -> Result<(), CoordinatorError> {
        if self.version != 1 {
            return Err(CoordinatorError::Invalid("request version"));
        }
        id(&self.node_id, "node id")?;
        id(&self.contract_reference, "contract reference")?;
        digest(&self.current_capability_digest, "capability digest")?;
        if self.scope.is_empty() {
            return Err(CoordinatorError::Invalid("scope"));
        }
        scopes(&self.scope)?;
        known_tool(&self.tool_id)?;
        if self.expected_cost == 0 {
            return Err(CoordinatorError::Invalid("expected cost"));
        }
        bounded(&self.reason, MAX_TEXT, "reason")?;
        strings(&self.evidence, MAX_TEXT, "evidence")?;
        if self.kind.is_hard_denied() {
            return Err(CoordinatorError::HardDenied(self.kind));
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CapabilityDecision {
    RequiresParentReview,
}

impl CapabilityDecision {
    pub(crate) fn direct(kind: CapabilityKind, _role: Role) -> Result<Self, CoordinatorError> {
        if kind.is_hard_denied() {
            Err(CoordinatorError::HardDenied(kind))
        } else {
            Ok(Self::RequiresParentReview)
        }
    }
    pub(crate) fn from_request(request: &CapabilityRequest) -> Result<Self, CoordinatorError> {
        request.validate()?;
        Ok(Self::RequiresParentReview)
    }
}

// Traverses a Serialize value before JSON conversion, which otherwise turns NaN/inf into null.
struct FiniteProbe;

#[derive(Debug)]
struct ProbeError(String);
impl std::fmt::Display for ProbeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ProbeError {}
impl serde::ser::Error for ProbeError {
    fn custom<T: std::fmt::Display>(message: T) -> Self {
        Self(message.to_string())
    }
}

impl serde::Serializer for FiniteProbe {
    type Ok = ();
    type Error = ProbeError;
    type SerializeSeq = Self;
    type SerializeTuple = Self;
    type SerializeTupleStruct = Self;
    type SerializeTupleVariant = Self;
    type SerializeMap = Self;
    type SerializeStruct = Self;
    type SerializeStructVariant = Self;
    fn serialize_bool(self, _: bool) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_i8(self, _: i8) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_i16(self, _: i16) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_i32(self, _: i32) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_i64(self, _: i64) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_u8(self, _: u8) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_u16(self, _: u16) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_u32(self, _: u32) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_u64(self, _: u64) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_f32(self, v: f32) -> Result<(), ProbeError> {
        if v.is_finite() {
            Ok(())
        } else {
            Err(ProbeError("non-finite number".into()))
        }
    }
    fn serialize_f64(self, v: f64) -> Result<(), ProbeError> {
        if v.is_finite() {
            Ok(())
        } else {
            Err(ProbeError("non-finite number".into()))
        }
    }
    fn serialize_char(self, _: char) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_str(self, _: &str) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_bytes(self, _: &[u8]) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_none(self) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_some<T: ?Sized + Serialize>(self, value: &T) -> Result<(), ProbeError> {
        value.serialize(self)
    }
    fn serialize_unit(self) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_unit_struct(self, _: &'static str) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
    ) -> Result<(), ProbeError> {
        Ok(())
    }
    fn serialize_newtype_struct<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        value: &T,
    ) -> Result<(), ProbeError> {
        value.serialize(self)
    }
    fn serialize_newtype_variant<T: ?Sized + Serialize>(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        value: &T,
    ) -> Result<(), ProbeError> {
        value.serialize(self)
    }
    fn serialize_seq(self, _: Option<usize>) -> Result<Self, ProbeError> {
        Ok(self)
    }
    fn serialize_tuple(self, _: usize) -> Result<Self, ProbeError> {
        Ok(self)
    }
    fn serialize_tuple_struct(self, _: &'static str, _: usize) -> Result<Self, ProbeError> {
        Ok(self)
    }
    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, ProbeError> {
        Ok(self)
    }
    fn serialize_map(self, _: Option<usize>) -> Result<Self, ProbeError> {
        Ok(self)
    }
    fn serialize_struct(self, _: &'static str, _: usize) -> Result<Self, ProbeError> {
        Ok(self)
    }
    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        _: &'static str,
        _: usize,
    ) -> Result<Self, ProbeError> {
        Ok(self)
    }
}

impl serde::ser::SerializeSeq for FiniteProbe {
    type Ok = ();
    type Error = ProbeError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), ProbeError> {
        value.serialize(FiniteProbe)
    }
    fn end(self) -> Result<(), ProbeError> {
        Ok(())
    }
}
impl serde::ser::SerializeTuple for FiniteProbe {
    type Ok = ();
    type Error = ProbeError;
    fn serialize_element<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), ProbeError> {
        value.serialize(FiniteProbe)
    }
    fn end(self) -> Result<(), ProbeError> {
        Ok(())
    }
}
impl serde::ser::SerializeTupleStruct for FiniteProbe {
    type Ok = ();
    type Error = ProbeError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), ProbeError> {
        value.serialize(FiniteProbe)
    }
    fn end(self) -> Result<(), ProbeError> {
        Ok(())
    }
}
impl serde::ser::SerializeTupleVariant for FiniteProbe {
    type Ok = ();
    type Error = ProbeError;
    fn serialize_field<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), ProbeError> {
        value.serialize(FiniteProbe)
    }
    fn end(self) -> Result<(), ProbeError> {
        Ok(())
    }
}
impl serde::ser::SerializeMap for FiniteProbe {
    type Ok = ();
    type Error = ProbeError;
    fn serialize_key<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), ProbeError> {
        value.serialize(FiniteProbe)
    }
    fn serialize_value<T: ?Sized + Serialize>(&mut self, value: &T) -> Result<(), ProbeError> {
        value.serialize(FiniteProbe)
    }
    fn end(self) -> Result<(), ProbeError> {
        Ok(())
    }
}
impl serde::ser::SerializeStruct for FiniteProbe {
    type Ok = ();
    type Error = ProbeError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), ProbeError> {
        value.serialize(FiniteProbe)
    }
    fn end(self) -> Result<(), ProbeError> {
        Ok(())
    }
}
impl serde::ser::SerializeStructVariant for FiniteProbe {
    type Ok = ();
    type Error = ProbeError;
    fn serialize_field<T: ?Sized + Serialize>(
        &mut self,
        _: &'static str,
        value: &T,
    ) -> Result<(), ProbeError> {
        value.serialize(FiniteProbe)
    }
    fn end(self) -> Result<(), ProbeError> {
        Ok(())
    }
}
