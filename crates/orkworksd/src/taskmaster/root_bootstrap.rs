//! Dormant retained root-v2 source validation. No I/O, eligibility or launch authority.
#![allow(dead_code)]
mod source;
mod wire;
use serde_json::{json, Value};
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct RootBootstrapError(pub &'static str);
type Result<T> = std::result::Result<T, RootBootstrapError>;
pub(crate) struct RetainedArtifact<'a> {
    pub bytes: &'a [u8],
    pub content_digest: &'a str,
}
pub(crate) struct RootBootstrapContext<'a> {
    pub run_bytes: &'a [u8],
    pub goal: &'a str,
    pub retained_artifacts: &'a [RetainedArtifact<'a>],
}
pub(crate) struct ValidatedUnverifiedRootBootstrap {
    canonical_bytes: Vec<u8>,
    bootstrap_digest: String,
}
impl ValidatedUnverifiedRootBootstrap {
    pub(crate) fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
    pub(crate) fn bootstrap_digest(&self) -> &str {
        &self.bootstrap_digest
    }
    pub(crate) fn classification(&self) -> &'static str {
        "unverified-source-bytes-only"
    }
}

use std::collections::{BTreeMap, BTreeSet};
use wire::*;
fn run(x: &Value) -> Result<()> {
    exact(x,"schemaVersion runId workspaceId parentSessionId repositoryBindingDigest sourceWorktreeBindingDigest goal goalDigest admissionEpoch runSequence bootstrapConfigurationDigest maxParallelChildren runDefinitionDigest")?;
    ensure(x["schemaVersion"] == 1, "run version")?;
    for k in ["runId", "workspaceId", "parentSessionId"] {
        id(&x[k])?;
    }
    for k in [
        "repositoryBindingDigest",
        "sourceWorktreeBindingDigest",
        "goalDigest",
        "admissionEpoch",
        "bootstrapConfigurationDigest",
        "runDefinitionDigest",
    ] {
        digest(&x[k])?;
    }
    let seq = s(&x["runSequence"])?;
    ensure(
        !seq.starts_with('0')
            && !seq.is_empty()
            && seq.len() <= 19
            && seq.bytes().all(|b| b.is_ascii_digit())
            && seq.parse::<u64>().is_ok_and(|n| n <= i64::MAX as u64),
        "run sequence",
    )?;
    ensure(
        x["runId"] == format!("{}-{seq}", s(&x["admissionEpoch"])?),
        "run identity",
    )?;
    ensure(
        hash(s(&x["goal"])?) == digest(&x["goalDigest"])?,
        "goal binding",
    )?;
    number(&x["maxParallelChildren"], 1, 16)?;
    sealed(x, "runDefinitionDigest", "orkworks.orchestration-run.v1")
}
fn retained<'a>(
    context: &'a RootBootstrapContext<'_>,
    e: &Value,
) -> Result<BTreeMap<&'a str, &'a [u8]>> {
    let mut map = BTreeMap::new();
    for a in context.retained_artifacts {
        ensure(std::str::from_utf8(a.bytes).is_ok(), "artifact UTF-8")?;
        ensure(hash(a.bytes) == a.content_digest, "artifact digest")?;
        ensure(
            map.insert(a.content_digest, a.bytes).is_none(),
            "artifact closure",
        )?;
    }
    let refs: BTreeSet<_> = list(&e["references"], 16, 1)?
        .iter()
        .map(|r| s(&r["contentDigest"]))
        .collect::<Result<_>>()?;
    ensure(
        map.keys().copied().collect::<BTreeSet<_>>() == refs,
        "artifact closure",
    )?;
    Ok(map)
}
fn artifact(
    map: &BTreeMap<&str, &[u8]>,
    d: &Value,
    refs: &[Value],
    fixture: bool,
) -> Result<Value> {
    let key = digest(d)?;
    ensure(
        refs.iter()
            .any(|r| r["contentDigest"] == *d && (!fixture || r["kind"] == "fixture")),
        "artifact reference",
    )?;
    decode(
        map.get(key).ok_or(RootBootstrapError("artifact closure"))?,
        65536,
    )
}
fn settings(x: &Value, b: &Value, map: &BTreeMap<&str, &[u8]>) -> Result<()> {
    exact(x,"schemaVersion kind harnessId adapterId adapterVersion argv nonsecretBindings configurationSources discoveredInstructions deliveryOrder instructionMechanism")?;
    ensure(
        x["schemaVersion"] == 1 && x["kind"] == "root-startup-settings",
        "settings version",
    )?;
    for k in ["harnessId", "adapterId"] {
        id(&x[k])?;
    }
    text(&x["adapterVersion"], 512, 1)?;
    tags(
        &x["instructionMechanism"],
        &["native-agent", "startup-context"],
    )?;
    for a in list(&x["argv"], 64, 0)? {
        text(a, 2048, 0)?;
    }
    source::bindings(&x["nonsecretBindings"])?;
    let refs = list(&b["capabilityEvidence"]["references"], 16, 1)?;
    let cs = list(&x["configurationSources"], 32, 0)?;
    sorted(cs, Some("sourceIdentity"))?;
    for c in cs {
        exact(c, "sourceIdentity sourceRevision contentDigest")?;
        text(&c["sourceIdentity"], 512, 1)?;
        text(&c["sourceRevision"], 512, 1)?;
        let p = artifact(map, &c["contentDigest"], refs, false)?;
        source::projection(&p)?;
        ensure(
            p["sourceIdentity"] == c["sourceIdentity"]
                && p["sourceRevision"] == c["sourceRevision"],
            "configuration source binding",
        )?;
    }
    let discovered = list(&x["discoveredInstructions"], 32, 0)?;
    let mut seen = BTreeSet::new();
    for (i, d) in discovered.iter().enumerate() {
        exact(d, "snapshotKind snapshotId contentDigest precedence")?;
        ensure(d["precedence"] == json!(i), "discovery precedence")?;
        id(&d["snapshotId"])?;
        digest(&d["contentDigest"])?;
        let sources: Vec<&Value> = match s(&d["snapshotKind"])? {
            "rule" => list(&b["rules"], 32, 0)?.iter().collect(),
            "role-template" => vec![&b["roleTemplate"]],
            "skill" => list(&b["skills"], 16, 0)?.iter().collect(),
            "skill-resource" => list(&b["skillResources"], 32, 0)?.iter().collect(),
            _ => return Err(RootBootstrapError("automatic snapshot")),
        };
        ensure(
            sources
                .iter()
                .any(|s| s["id"] == d["snapshotId"] && s["contentDigest"] == d["contentDigest"]),
            "automatic snapshot",
        )?;
        ensure(
            seen.insert((s(&d["snapshotKind"])?, s(&d["snapshotId"])?)),
            "duplicate discovery",
        )?;
    }
    let mut supplied = 0;
    let mut automatic = Vec::new();
    for d in list(&x["deliveryOrder"], 33, 1)? {
        tags(&d["kind"], &["supplied", "automatic"])?;
        if d["kind"] == "supplied" {
            exact(d, "kind")?;
            supplied += 1;
        } else {
            exact(d, "kind precedence")?;
            automatic.push(d["precedence"].clone());
        }
    }
    ensure(
        supplied == 1
            && automatic
                == discovered
                    .iter()
                    .map(|d| d["precedence"].clone())
                    .collect::<Vec<_>>(),
        "delivery order",
    )
}
fn validate(b: &Value, r: &Value, context: &RootBootstrapContext<'_>) -> Result<()> {
    exact(b,"schemaVersion bootstrapId repositoryId repositoryBinding sourceWorktreeBinding workspaceId parentSessionId runId rootAssignmentId goalDigest roleTemplate taskCategory assignment rules requirementManifests skills skillResources rubric harness capabilityEvidence model permissions renderedInstructions renderedInstructionsDigest outputContract bootstrapConfigurationDigest")?;
    for k in [
        "bootstrapId",
        "repositoryId",
        "workspaceId",
        "parentSessionId",
        "runId",
        "rootAssignmentId",
    ] {
        id(&b[k])?;
    }
    digest(&b["goalDigest"])?;
    text(&b["taskCategory"], 512, 1)?;
    exact(&b["permissions"], "requested effective")?;
    source::output(&b["outputContract"])?;
    source::profile(&b["permissions"]["requested"], b)?;
    source::profile(&b["permissions"]["effective"], b)?;
    run(r)?;
    ensure(
        r["goal"] == context.goal && b["goalDigest"] == hash(context.goal),
        "goal binding",
    )?;
    source::repository(&b["repositoryBinding"])?;
    source::worktree(&b["sourceWorktreeBinding"], &b["repositoryBinding"])?;
    ensure(
        b["repositoryId"] == b["repositoryBinding"]["repositoryId"],
        "repository binding",
    )?;
    source::assignment(&b["assignment"], r)?;
    source::snapshots(b)?;
    let e = &b["capabilityEvidence"];
    source::evidence(e, b)?;
    let h = &b["harness"];
    exact(h,"harnessId definitionDigest adapterId adapterVersion executableIdentity toolVersion platform instructionMechanism effectiveSettingsDigest evidenceId evidenceDigest")?;
    for k in h
        .as_object()
        .ok_or(RootBootstrapError("closed fields"))?
        .keys()
    {
        ensure(h[k] == e[k], "harness binding")?;
    }
    source::executable(&h["executableIdentity"])?;
    let map = retained(context, e)?;
    let refs = list(&e["references"], 16, 1)?;
    let settings_value = artifact(&map, &h["effectiveSettingsDigest"], refs, true)?;
    settings(&settings_value, b, &map)?;
    for k in [
        "harnessId",
        "adapterId",
        "adapterVersion",
        "instructionMechanism",
    ] {
        ensure(settings_value[k] == h[k], "settings binding")?;
    }
    let model = &b["model"];
    exact(
        model,
        "schemaVersion mode modelId policyId policyDigest adapterGeneration",
    )?;
    ensure(model["schemaVersion"] == 1, "model version")?;
    id(&model["adapterGeneration"])?;
    let policy = artifact(&map, &model["policyDigest"], refs, true)?;
    source::policy(&policy)?;
    for k in ["policyId", "mode", "modelId"] {
        ensure(model[k] == policy[k], "model binding")?;
    }
    for k in ["harnessId", "adapterId", "adapterVersion"] {
        ensure(policy[k] == h[k], "policy adapter binding")?;
    }
    ensure(
        text(&b["renderedInstructions"], 262144, 1)? == wire::render(b)?,
        "root rendering",
    )?;
    ensure(
        digest(&b["renderedInstructionsDigest"])? == hash(s(&b["renderedInstructions"])?),
        "render digest",
    )?;
    sealed(
        b,
        "bootstrapConfigurationDigest",
        "orkworks.orchestrator-bootstrap.v2",
    )?;
    for k in [
        "workspaceId",
        "parentSessionId",
        "runId",
        "goalDigest",
        "bootstrapConfigurationDigest",
    ] {
        ensure(b[k] == r[k], "run bootstrap binding")?;
    }
    ensure(
        b["repositoryBinding"]["bindingDigest"] == r["repositoryBindingDigest"]
            && b["sourceWorktreeBinding"]["worktreeBindingDigest"]
                == r["sourceWorktreeBindingDigest"],
        "run source binding",
    )
}
/// Validates only the named retained, explicitly unverified root source slice.
/// Success establishes neither native identity nor eligibility, custody or authority.
pub(crate) fn validate_unverified_root_bootstrap_v2(
    bytes: &[u8],
    context: &RootBootstrapContext<'_>,
) -> Result<ValidatedUnverifiedRootBootstrap> {
    ensure(bytes.len() <= 1048576, "bootstrap bytes")?;
    ensure(
        context.run_bytes.len() <= 16384 && context.goal.len() <= 16384,
        "run bytes",
    )?;
    ensure(context.retained_artifacts.len() <= 16, "artifact count")?;
    let mut total = 0usize;
    for a in context.retained_artifacts {
        ensure(
            !a.bytes.is_empty() && a.bytes.len() <= 65536,
            "artifact bytes",
        )?;
        total = total
            .checked_add(a.bytes.len())
            .filter(|n| *n <= 1048576)
            .ok_or(RootBootstrapError("artifact bytes"))?;
    }
    let b = decode(bytes, 1048576)?;
    ensure(
        b["schemaVersion"] == 2
            && b.get("bootstrapId").is_some()
            && b.get("configurationId").is_none(),
        "root v2 envelope",
    )?;
    let r = decode(context.run_bytes, 16384)?;
    validate(&b, &r, context)?;
    Ok(ValidatedUnverifiedRootBootstrap {
        canonical_bytes: bytes.to_vec(),
        bootstrap_digest: s(&b["bootstrapConfigurationDigest"])?.into(),
    })
}

impl std::fmt::Debug for ValidatedUnverifiedRootBootstrap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidatedUnverifiedRootBootstrap")
            .field("byte_count", &self.canonical_bytes.len())
            .field("classification", &self.classification())
            .finish_non_exhaustive()
    }
}
