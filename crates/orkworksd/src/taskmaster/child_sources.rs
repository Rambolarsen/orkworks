//! Dormant child-source byte validation; no I/O, publication or eligibility.
#![allow(dead_code)]
mod configuration;
mod records;
mod reports;
mod wire;
use super::root_bootstrap::RootBootstrapError as ByteError;
use serde_json::Value;
type ByteResult<T> = std::result::Result<T, ByteError>;
use wire::*;
pub(super) struct ParsedBundle {
    configuration: Value,
    evidence: Vec<Value>,
    plan: Value,
    request: Value,
    approval: Value,
    receipt: Value,
    reservation: Value,
    allocation: Value,
    attachment: Value,
    source: Value,
}
impl ParsedBundle {
    fn decode(b: &ChildSourceBundle<'_>) -> ByteResult<Self> {
        ensure(b.evidence_objects.len() <= 16, "evidence count")?;
        Ok(Self {
            configuration: decode(b.configuration, 1048576)?,
            evidence: b
                .evidence_objects
                .iter()
                .map(|b| decode(b, 2097152))
                .collect::<ByteResult<_>>()?,
            plan: decode(b.plan, 65536)?,
            request: decode(b.request, 16384)?,
            approval: decode(b.approval, 16384)?,
            receipt: decode(b.receipt, 4096)?,
            reservation: decode(b.reservation, 16384)?,
            allocation: decode(b.allocation, 16384)?,
            attachment: decode(b.attachment, 16384)?,
            source: decode(b.source, 2097152)?,
        })
    }
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ChildSourceError(pub &'static str);
pub(crate) struct ChildSourceBundle<'a> {
    pub configuration: &'a [u8],
    pub evidence_objects: &'a [&'a [u8]],
    pub plan: &'a [u8],
    pub request: &'a [u8],
    pub approval: &'a [u8],
    pub receipt: &'a [u8],
    pub reservation: &'a [u8],
    pub allocation: &'a [u8],
    pub attachment: &'a [u8],
    pub source: &'a [u8],
}
pub(crate) struct ChildSourceValidationInput<'a> {
    pub run: &'a [u8],
    pub parent_input: &'a [u8],
    pub worker: ChildSourceBundle<'a>,
    pub reviewer: ChildSourceBundle<'a>,
    pub snapshot: &'a [u8],
    pub report_context: &'a Value,
}
pub(crate) struct ValidatedUnverifiedChildSource<'a> {
    configuration: &'a [u8],
    source: &'a [u8],
    configuration_digest: String,
    source_digest: String,
}
impl ValidatedUnverifiedChildSource<'_> {
    pub(crate) fn configuration_bytes(&self) -> &[u8] {
        self.configuration
    }
    pub(crate) fn source_bytes(&self) -> &[u8] {
        self.source
    }
    pub(crate) fn configuration_digest(&self) -> &str {
        &self.configuration_digest
    }
    pub(crate) fn source_digest(&self) -> &str {
        &self.source_digest
    }
}
pub(crate) struct ValidatedUnverifiedChildSources<'a> {
    worker: ValidatedUnverifiedChildSource<'a>,
    reviewer: ValidatedUnverifiedChildSource<'a>,
}
impl ValidatedUnverifiedChildSources<'_> {
    pub(crate) fn worker(&self) -> &ValidatedUnverifiedChildSource<'_> {
        &self.worker
    }
    pub(crate) fn reviewer(&self) -> &ValidatedUnverifiedChildSource<'_> {
        &self.reviewer
    }
    pub(crate) fn classification(&self) -> &'static str {
        "unverified-source-bytes-only"
    }
}
/// Checks only supplied retained source bytes; success grants no authority.
pub(crate) fn validate_unverified_child_sources_v1<'a>(
    input: &ChildSourceValidationInput<'a>,
) -> Result<ValidatedUnverifiedChildSources<'a>, ChildSourceError> {
    validate(input).map_err(|error| ChildSourceError(error.0))
}
fn validate<'a>(
    input: &ChildSourceValidationInput<'a>,
) -> ByteResult<ValidatedUnverifiedChildSources<'a>> {
    let run = decode(input.run, 16384)?;
    super::root_bootstrap::run(&run)?;
    let initial = decode(input.parent_input, 32768)?;
    same(
        &initial,
        &serde_json::json!({"schemaVersion":1,"workspaceId":run["workspaceId"],"runId":run["runId"],"preparationRevision":1,"inputRevision":1,"goalDigest":run["goalDigest"],"bootstrapConfigurationDigest":run["bootstrapConfigurationDigest"],"decision":null,"records":[]}),
        "initial input",
    )?;
    let worker = ParsedBundle::decode(&input.worker)?;
    let reviewer = ParsedBundle::decode(&input.reviewer)?;
    let wc = configuration::validate(&worker, &run, None)?;
    records::bundle(&worker, &run, &initial, None, &wc)?;
    let snapshot = decode(input.snapshot, 262144)?;
    records::snapshot(&snapshot, &worker, &reviewer, &run, input.report_context)?;
    let rc = configuration::validate(&reviewer, &run, Some(&snapshot))?;
    records::bundle(&reviewer, &run, &initial, Some(&snapshot), &rc)?;
    for key in [
        "taskId",
        "reservationId",
        "childSessionId",
        "configurationId",
        "configurationDigest",
    ] {
        ensure(
            worker.source["assignmentIdentity"][key] != reviewer.source["assignmentIdentity"][key],
            "reviewer independence",
        )?;
    }
    ensure(
        worker.attachment["attachmentId"] != reviewer.attachment["attachmentId"],
        "attachment independence",
    )?;
    for key in ["canonicalRoot", "rootObject", "privateGitDirectoryObject"] {
        ensure(
            worker.allocation["allocatedWorktree"][key]
                != reviewer.allocation["allocatedWorktree"][key],
            "allocation independence",
        )?;
    }
    ensure(
        worker.allocation["group"]["branchName"] != reviewer.allocation["group"]["branchName"],
        "branch independence",
    )?;
    let validated = |raw: &ChildSourceBundle<'a>,
                     parsed: &ParsedBundle|
     -> ByteResult<ValidatedUnverifiedChildSource<'a>> {
        Ok(ValidatedUnverifiedChildSource {
            configuration: raw.configuration,
            source: raw.source,
            configuration_digest: s(&parsed.configuration["configurationDigest"])?.into(),
            source_digest: domain("orkworks.child-assignment-source.v1", &parsed.source)?,
        })
    };
    Ok(ValidatedUnverifiedChildSources {
        worker: validated(&input.worker, &worker)?,
        reviewer: validated(&input.reviewer, &reviewer)?,
    })
}
