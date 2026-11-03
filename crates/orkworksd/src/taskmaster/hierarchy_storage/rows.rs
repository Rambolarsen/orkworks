//! Canonical row admission for the accepted child-source slice: strict
//! canonical JSON bytes, the closed child-source `ObjectKey` registry,
//! checked Counter TEXT and CommitRefs, per-kind body caps, and record
//! validators that reconstruct each row's key through explicit source joins.
//! Noncanonical, unknown, oversized or digest-mismatched state is rejected
//! before persistence; nothing is normalized on read.

use super::{canonical, ensure, sha256_hex, HierarchyStorageError, Result};
use serde_json::{json, Value};

/// Accepted child-source object kinds; anything else is outside the closed
/// registry and rejected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ObjectKind {
    Plan,
    Configuration,
    ReviewInput,
    Evidence,
    AssignmentSource,
}

impl ObjectKind {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            ObjectKind::Plan => "child-source-plan",
            ObjectKind::Configuration => "child-source-configuration",
            ObjectKind::ReviewInput => "child-review-input",
            ObjectKind::Evidence => "child-source-evidence",
            ObjectKind::AssignmentSource => "child-assignment-source",
        }
    }

    /// Per-kind body cap: plan ≤64 KiB, configuration ≤1 MiB, input snapshot
    /// ≤256 KiB, evidence wrapper ≤96 KiB; the retained provenance source
    /// obeys the 2 MiB row ceiling.
    fn max_bytes(self) -> usize {
        match self {
            ObjectKind::Plan => 64 * 1024,
            ObjectKind::Configuration => 1024 * 1024,
            ObjectKind::ReviewInput => 256 * 1024,
            ObjectKind::Evidence => 96 * 1024,
            ObjectKind::AssignmentSource => 2 * 1024 * 1024,
        }
    }
}

/// A validated child-source object body with its reconstructed canonical
/// `ObjectKey` and its owning contract's source digest.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdmittedObject {
    pub key: Value,
    pub digest: String,
    pub body: Value,
}

// --- canonical wire and closed-field helpers.

fn integers(x: &Value) -> bool {
    match x {
        Value::Number(n) => {
            n.as_i64()
                .is_some_and(|n| (-9007199254740991..=9007199254740991).contains(&n))
                || n.as_u64().is_some_and(|n| n <= 9007199254740991)
        }
        Value::Array(xs) => xs.iter().all(integers),
        Value::Object(m) => m.values().all(integers),
        _ => true,
    }
}

/// Strict decode: byte equality with a canonical re-serialization rejects
/// duplicate keys, stray whitespace, unpaired surrogates, floats and
/// noncanonical wire order; `serde_json` handles UTF-8 validity.
pub(super) fn decode(bytes: &[u8], limit: usize) -> Result<Value> {
    ensure(!bytes.is_empty() && bytes.len() <= limit, "bytes")?;
    let x: Value =
        serde_json::from_slice(bytes).map_err(|_| HierarchyStorageError("canonical wire"))?;
    ensure(integers(&x) && canonical(&x)? == bytes, "canonical wire")?;
    Ok(x)
}

fn exact(x: &Value, fields: &str) -> Result<()> {
    let map = x
        .as_object()
        .ok_or(HierarchyStorageError("closed fields"))?;
    ensure(
        map.len() == fields.split_whitespace().count()
            && fields.split_whitespace().all(|k| map.contains_key(k)),
        "closed fields",
    )
}

fn s(x: &Value) -> Result<&str> {
    x.as_str().ok_or(HierarchyStorageError("string"))
}

fn text_min(x: &Value, min: usize, max: usize) -> Result<&str> {
    let v = s(x)?;
    ensure((min..=max).contains(&v.len()), "text bound")?;
    Ok(v)
}

fn text(x: &Value, max: usize) -> Result<&str> {
    text_min(x, 1, max)
}

fn id(x: &Value) -> Result<&str> {
    let v = text(x, 128)?;
    ensure(
        v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b)),
        "id",
    )?;
    Ok(v)
}

fn digest(x: &Value) -> Result<&str> {
    let v = s(x)?;
    ensure(
        v.len() == 64
            && v.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "digest",
    )?;
    Ok(v)
}

fn integer(x: &Value, min: u64, max: u64) -> Result<u64> {
    let n = x.as_u64().ok_or(HierarchyStorageError("integer"))?;
    ensure((min..=max).contains(&n), "integer bound")?;
    Ok(n)
}

fn list(x: &Value, max: usize, min: usize) -> Result<&[Value]> {
    let a = x.as_array().ok_or(HierarchyStorageError("array"))?;
    ensure((min..=max).contains(&a.len()), "array bound")?;
    Ok(a)
}

fn sorted_strings<'a, T>(values: T) -> Result<()>
where
    T: IntoIterator<Item = &'a str>,
{
    let mut prev: Option<&str> = None;
    for next in values {
        ensure(prev.is_none_or(|p| p < next), "sorted unique set")?;
        prev = Some(next);
    }
    Ok(())
}

fn tags<'v>(x: &'v Value, choices: &[&str]) -> Result<&'v str> {
    let v = s(x)?;
    ensure(choices.contains(&v), "tag")?;
    Ok(v)
}

fn valid_hex63(value: &str) -> bool {
    value
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn sha(value: &str) -> Result<String> {
    Ok(sha256_hex(value.as_bytes()))
}

/// Sealed per-record digest: the source domain over the canonical body minus
/// the named digest field.
fn sealed(x: &Value, field: &str, domain: &str) -> Result<()> {
    let expected = digest(&x[field])?;
    let mut body = x.clone();
    body.as_object_mut()
        .ok_or(HierarchyStorageError("closed fields"))?
        .remove(field);
    let mut bytes = domain.as_bytes().to_vec();
    bytes.push(b'\n');
    bytes.extend(canonical(&body)?);
    ensure(sha256_hex(&bytes) == expected, "sealed digest")
}

// --- Counter TEXT, CommitRef, run identity.

/// Canonical ASCII decimal TEXT Counter for u64, checked in Rust: `"0"` or a
/// nonzero digit followed by up to 19 digits, at most `18446744073709551615`.
pub(crate) fn counter(x: &Value, positive: bool) -> Result<u64> {
    let text_value = s(x).map_err(|_| HierarchyStorageError("counter"))?;
    let ok = (if positive {
        !text_value.starts_with('0')
    } else {
        true
    }) && !text_value.is_empty()
        && text_value.len() <= 20
        && text_value.bytes().all(|b| b.is_ascii_digit())
        && text_value.parse::<u64>().is_ok();
    ensure(ok, "counter")?;
    ensure(!positive || text_value != "0", "positive counter")?;
    text_value
        .parse::<u64>()
        .map_err(|_| HierarchyStorageError("counter"))
}

/// `CommitRef` is exactly `{storageEpoch, commitSequence, changeDigest}`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommitRef {
    pub storage_epoch: String,
    pub sequence: u64,
    pub change_digest: String,
}

pub(crate) fn commit_ref(value: &Value) -> Result<CommitRef> {
    exact(value, "changeDigest commitSequence storageEpoch")?;
    Ok(CommitRef {
        storage_epoch: digest(&value["storageEpoch"])?.to_string(),
        sequence: counter(&value["commitSequence"], true)?,
        change_digest: digest(&value["changeDigest"])?.to_string(),
    })
}

/// Canonical CommitRef serialization (sorted keys, Counter as TEXT).
pub(crate) fn commit_ref_value(commit: &CommitRef) -> Value {
    json!({
        "changeDigest": commit.change_digest,
        "commitSequence": commit.sequence.to_string(),
        "storageEpoch": commit.storage_epoch,
    })
}

/// `runId` is `admissionEpoch` + `-` + positive canonical run sequence.
fn run_id(x: &Value) -> Result<&str> {
    let v = text(x, 128)?;
    let (epoch, sequence) = v.split_once('-').ok_or(HierarchyStorageError("run id"))?;
    ensure(epoch.len() == 64 && valid_hex63(epoch), "run id")?;
    ensure(
        !sequence.is_empty()
            && sequence.len() <= 19
            && !sequence.starts_with('0')
            && sequence.bytes().all(|b| b.is_ascii_digit()),
        "run id",
    )?;
    Ok(v)
}

fn valid_workspace(v: &str) -> bool {
    (1..=128).contains(&v.len())
        && v.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

/// TaskKey is exactly `{workspaceId, runId, planId, planRevision, taskId}`.
pub(crate) fn task_key(x: &Value) -> Result<Value> {
    exact(x, "planId planRevision runId taskId workspaceId")?;
    ensure(valid_workspace(s(&x["workspaceId"])?), "workspace")?;
    run_id(&x["runId"])?;
    id(&x["planId"])?;
    integer(&x["planRevision"], 1, 64)?;
    id(&x["taskId"])?;
    Ok(x.clone())
}

/// PlanRef is exactly `{workspaceId, runId, planId, planRevision,
/// planDefinitionDigest}`.
fn plan_ref(x: &Value) -> Result<Value> {
    exact(
        x,
        "planDefinitionDigest planId planRevision runId workspaceId",
    )?;
    ensure(valid_workspace(s(&x["workspaceId"])?), "workspace")?;
    run_id(&x["runId"])?;
    id(&x["planId"])?;
    integer(&x["planRevision"], 1, 64)?;
    digest(&x["planDefinitionDigest"])?;
    Ok(x.clone())
}

fn configuration_ref(x: &Value) -> Result<Value> {
    exact(x, "configurationDigest configurationId task")?;
    task_key(&x["task"])?;
    id(&x["configurationId"])?;
    digest(&x["configurationDigest"])?;
    Ok(x.clone())
}

fn approval_ref(x: &Value) -> Result<Value> {
    exact(x, "approvalDigest approvalId plan")?;
    plan_ref(&x["plan"])?;
    id(&x["approvalId"])?;
    digest(&x["approvalDigest"])?;
    Ok(x.clone())
}

fn reservation_ref(x: &Value) -> Result<Value> {
    exact(x, "reservationDigest reservationId task")?;
    task_key(&x["task"])?;
    id(&x["reservationId"])?;
    digest(&x["reservationDigest"])?;
    Ok(x.clone())
}

fn allocation_ref(x: &Value) -> Result<Value> {
    exact(x, "allocationDigest allocationId reservation")?;
    reservation_ref(&x["reservation"])?;
    id(&x["allocationId"])?;
    digest(&x["allocationDigest"])?;
    Ok(x.clone())
}

fn input_ref(x: &Value) -> Result<Value> {
    exact(x, "inputDigest task")?;
    task_key(&x["task"])?;
    digest(&x["inputDigest"])?;
    Ok(x.clone())
}

/// Quota owner sub-shapes: the closed ledger owner codecs only.
pub(crate) fn owner_value(x: &Value) -> Result<Value> {
    match tags(&x["kind"], &["assignment", "run", "workspace"])? {
        "assignment" => {
            exact(x, "assignmentIdentity kind")?;
            child_identity(&x["assignmentIdentity"])?;
        }
        "run" => {
            exact(x, "kind runId workspaceId")?;
            ensure(valid_workspace(s(&x["workspaceId"])?), "workspace")?;
            run_id(&x["runId"])?;
        }
        _ => {
            exact(x, "kind workspaceId")?;
            ensure(valid_workspace(s(&x["workspaceId"])?), "workspace")?;
        }
    }
    Ok(x.clone())
}

/// The only resource admitted by this kernel's producer primitives.
pub(crate) const RETAINED_BYTES_RESOURCE: &str = "child-source-retained-bytes";

/// `quota_usage` key is exactly `{owner, resource}` with the closed owner
/// codec and this kernel's admitted resource.
pub(crate) fn quota_key(key: &Value) -> Result<Value> {
    exact(key, "owner resource")?;
    owner_value(&key["owner"])?;
    ensure(s(&key["resource"])? == RETAINED_BYTES_RESOURCE, "resource")?;
    Ok(key.clone())
}

/// Child assignment identity: `{kind:"child", workspaceId, runId, planId,
/// planRevision, taskId, taskVersion, reservationId, parentSessionId,
/// childSessionId, configurationId, configurationDigest}`.
pub(crate) fn child_identity(x: &Value) -> Result<Value> {
    exact(
        x,
        "childSessionId configurationDigest configurationId kind parentSessionId planId planRevision reservationId runId taskId taskVersion workspaceId",
    )?;
    ensure(x["kind"] == "child", "child identity")?;
    for field in [
        "childSessionId",
        "configurationId",
        "parentSessionId",
        "planId",
        "reservationId",
        "taskId",
        "workspaceId",
    ] {
        id(&x[field])?;
    }
    ensure(valid_workspace(s(&x["workspaceId"])?), "workspace")?;
    run_id(&x["runId"])?;
    integer(&x["planRevision"], 1, 64)?;
    integer(&x["taskVersion"], 1, 2147483647)?;
    digest(&x["configurationDigest"])?;
    Ok(x.clone())
}

// --- bounded paths and root-owned binding descriptors.

/// Strict padded canonical base64 decode within `[min, max]` decoded bytes.
fn base64_decoded(value: &str, min: usize, max: usize) -> Result<Vec<u8>> {
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(value.as_bytes())
        .map_err(|_| HierarchyStorageError("canonical base64"))?;
    ensure((min..=max).contains(&bytes.len()), "base64 bound")?;
    ensure(
        base64::engine::general_purpose::STANDARD.encode(&bytes) == value,
        "canonical base64",
    )?;
    Ok(bytes)
}

fn native_path(x: &Value) -> Result<()> {
    exact(x, "base64 encoding")?;
    let encoding = tags(&x["encoding"], &["unix-bytes", "windows-utf16le"])?;
    let bytes = base64_decoded(s(&x["base64"])?, 1, 4096)?;
    if encoding == "unix-bytes" {
        ensure(bytes[0] == b'/' && !bytes.contains(&0), "native path")?;
    } else {
        ensure(
            bytes.len() % 2 == 0
                && bytes
                    .chunks_exact(2)
                    .all(|pair| pair[0] != 0 || pair[1] != 0),
            "native path",
        )?;
    }
    Ok(())
}

fn directory(x: &Value) -> Result<()> {
    exact(x, "fileId mechanismVersion platform volumeId")?;
    for name in ["fileId", "platform", "volumeId"] {
        let value = s(&x[name])?;
        ensure(
            !value.is_empty() && value.len() <= 128,
            "opaque native object",
        )?;
        ensure(
            value.bytes().all(|b| (b' '..=b'~').contains(&b)),
            "opaque native object",
        )?;
    }
    integer(&x["mechanismVersion"], 1, 9007199254740991)?;
    Ok(())
}

fn repository_binding(x: &Value) -> Result<()> {
    exact(x, "bindingDigest canonicalCommonDirectory commonDirectoryObject registryEpoch repositoryId schemaVersion")?;
    ensure(x["schemaVersion"] == 1, "repository version")?;
    let epoch = s(&x["registryEpoch"])?;
    ensure(epoch.len() == 32 && valid_hex63(epoch), "registry epoch")?;
    let repository = s(&x["repositoryId"])?;
    let identity = repository
        .strip_prefix("repo-")
        .ok_or(HierarchyStorageError("repository id"))?;
    ensure(
        identity.len() == 32 && valid_hex63(identity),
        "repository id",
    )?;
    native_path(&x["canonicalCommonDirectory"])?;
    directory(&x["commonDirectoryObject"])?;
    ensure(canonical(x)?.len() <= 8192, "repository 8 KiB")?;
    sealed(x, "bindingDigest", "orkworks.repository-binding.v1")
}

fn worktree_binding(x: &Value) -> Result<()> {
    exact(x, "canonicalPrivateGitDirectory canonicalRoot kind privateGitDirectoryObject repositoryBinding rootObject worktreeBindingDigest")?;
    tags(&x["kind"], &["primary", "linked"])?;
    native_path(&x["canonicalRoot"])?;
    native_path(&x["canonicalPrivateGitDirectory"])?;
    directory(&x["rootObject"])?;
    directory(&x["privateGitDirectoryObject"])?;
    sealed(x, "worktreeBindingDigest", "orkworks.worktree-binding.v1")
}

fn source_path(x: &Value, max: usize) -> Result<()> {
    exact(x, "base64 encoding")?;
    let encoding = tags(&x["encoding"], &["unix-bytes", "windows-utf16le"])?;
    let bytes = base64_decoded(s(&x["base64"])?, 1, max)?;
    if encoding == "unix-bytes" {
        ensure(bytes[0] == b'/' && !bytes.contains(&0), "source path")?;
    } else {
        ensure(
            bytes.len() % 2 == 0
                && bytes
                    .chunks_exact(2)
                    .all(|pair| pair[0] != 0 || pair[1] != 0),
            "source path",
        )?;
    }
    Ok(())
}

/// Provenance descriptor: `local` adds the exact `sourcePath`; a repository
/// file source must be local.
fn provenance(x: &Value) -> Result<()> {
    let kind = tags(&x["kind"], &["local", "nonlocal"])?;
    match kind {
        "local" => exact(
            x,
            "kind schemaVersion sourceIdentity sourceKind sourcePath sourceRevision",
        )?,
        _ => exact(
            x,
            "kind schemaVersion sourceIdentity sourceKind sourceRevision",
        )?,
    }
    ensure(x["schemaVersion"] == 1, "provenance version")?;
    tags(
        &x["sourceKind"],
        &["repository", "bundled", "user-approved"],
    )?;
    text(&x["sourceIdentity"], 512)?;
    text(&x["sourceRevision"], 512)?;
    if kind == "local" {
        source_path(&x["sourcePath"], 2732)?;
    } else {
        ensure(
            s(&x["sourceKind"])? != "repository",
            "repository file source must be local",
        )?;
    }
    Ok(())
}

/// UTC Gregorian timestamp `YYYY-MM-DDTHH:MM:SSZ`, not year 0000.
fn utc_timestamp(x: &Value) -> Result<()> {
    let value = text(x, 20)?;
    ensure(value.len() == 20, "UTC timestamp")?;
    let bytes = value.as_bytes();
    let shape = bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4] == b'-'
        && bytes[5..7].iter().all(u8::is_ascii_digit)
        && bytes[7] == b'-'
        && bytes[8..10].iter().all(u8::is_ascii_digit)
        && bytes[10] == b'T'
        && bytes[11..13].iter().all(u8::is_ascii_digit)
        && bytes[13] == b':'
        && bytes[14..16].iter().all(u8::is_ascii_digit)
        && bytes[16] == b':'
        && bytes[17..19].iter().all(u8::is_ascii_digit)
        && bytes[19] == b'Z'
        && !value.starts_with("0000");
    ensure(shape && month_day_valid(value), "UTC timestamp")
}

fn month_day_valid(value: &str) -> bool {
    use chrono::NaiveDate;
    NaiveDate::parse_from_str(&value[..10], "%Y-%m-%d").is_ok()
        && format!("{}T{}Z", &value[..10], &value[11..19]) == value
}

// --- child instruction snapshots and role/rule/skill closures.

fn content_digest_binding(x: &Value, content_key: &str, digest_key: &str) -> Result<()> {
    let content = s(&x[content_key])?;
    ensure(
        digest(&x[digest_key])? == sha256_hex(content.as_bytes()),
        "content digest",
    )
}

fn relative_text_path(x: &Value, max: usize) -> Result<()> {
    let value = text(x, max)?;
    ensure(
        !value.contains('\\') && !value.contains('\0'),
        "relative path",
    )?;
    ensure(
        value
            .split('/')
            .all(|part| !part.is_empty() && part != "." && part != ".."),
        "relative path components",
    )?;
    Ok(())
}

fn role_template(r: &Value) -> Result<()> {
    exact(
        r,
        "contentDigest id instructions instructionsDigest provenance role version",
    )?;
    id(&r["id"])?;
    integer(&r["version"], 1, 2147483647)?;
    tags(&r["role"], &["verification", "review"])?;
    text(&r["instructions"], 262144)?;
    ensure(
        digest(&r["instructionsDigest"])? == sha256_hex(s(&r["instructions"])?.as_bytes()),
        "role instructions digest",
    )?;
    provenance(&r["provenance"])?;
    // contentDigest covers the role body minus the contentDigest property.
    let expected = digest(&r["contentDigest"])?;
    let mut role_body = r.clone();
    role_body
        .as_object_mut()
        .ok_or(HierarchyStorageError("closed fields"))?
        .remove("contentDigest");
    ensure(
        sha256_hex(&canonical(&role_body)?) == expected,
        "role content digest",
    )
}

fn rules_skills_resources(c: &Value, task_id: &str) -> Result<()> {
    let rules = list(&c["rules"], 32, 0)?;
    let resources = list(&c["skillResources"], 32, 0)?;
    ensure(
        rules.len() + resources.len() <= 32,
        "combined rule/resource cap",
    )?;
    let skills = list(&c["skills"], 16, 0)?;
    for rule in rules {
        id(&rule["id"])?;
    }
    for skill in skills {
        id(&skill["id"])?;
    }
    for resource in resources {
        id(&resource["id"])?;
    }
    // Sorted-unique requirement: no equal ids within each of the three sets.
    let mut ids = Vec::new();
    for rule in rules {
        ids.push((s(&rule["id"])?, 0usize));
    }
    for skill in skills {
        ids.push((s(&skill["id"])?, 1usize));
    }
    for resource in resources {
        ids.push((s(&resource["id"])?, 2usize));
    }
    ids.sort();
    for window in ids.windows(2) {
        ensure(
            !(window[0] == window[1] && window[0].1 == window[1].1),
            "unique snapshot IDs",
        )?;
    }
    for (index, rule) in rules.iter().enumerate() {
        exact(
            rule,
            "authority content contentDigest id precedence provenance sourcePath",
        )?;
        text(&rule["sourcePath"], 2048)?;
        relative_text_path(&rule["sourcePath"], 2048)?;
        ensure(rule["precedence"] == index as u64, "rule precedence")?;
        ensure(rule["authority"] == "binding", "rule authority")?;
        provenance(&rule["provenance"])?;
        content_digest_binding(rule, "content", "contentDigest")?;
    }
    for skill in skills {
        exact(skill, "content contentDigest id provenance requirement requirementSources resourceIds selectionReason version")?;
        text(&skill["version"], 128)?;
        tags(&skill["requirement"], &["mandatory", "optional"])?;
        text(&skill["selectionReason"], 2048)?;
        let owned = list(&skill["resourceIds"], 32, 0)?;
        for resource_id in owned {
            id(resource_id)?;
        }
        sorted_strings(owned.iter().filter_map(|v| v.as_str()))?;
        provenance(&skill["provenance"])?;
        content_digest_binding(skill, "content", "contentDigest")?;
    }
    for resource in resources {
        exact(
            resource,
            "content contentDigest id provenance skillId sourceReference",
        )?;
        id(&resource["skillId"])?;
        text(&resource["sourceReference"], 2048)?;
        ensure(
            skills.iter().any(|skill| {
                skill["id"] == resource["skillId"]
                    && skill["resourceIds"].as_array().is_some_and(|owned| {
                        owned.iter().any(|owned_id| owned_id == &resource["id"])
                    })
            }),
            "resource ownership",
        )?;
        provenance(&resource["provenance"])?;
        content_digest_binding(resource, "content", "contentDigest")?;
    }
    for skill in skills {
        let closed = list(&skill["resourceIds"], 32, 0)?
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>();
        let actual = resources
            .iter()
            .filter(|r| r["skillId"] == skill["id"])
            .filter_map(|r| r["id"].as_str())
            .collect::<Vec<_>>();
        ensure(closed == actual, "resource closure")?;
    }
    // Requirement manifests: closed coverage over rules + role template.
    let role = &c["roleTemplate"];
    let mut roles_sources: Vec<&Value> = Vec::with_capacity(rules.len() + 1);
    roles_sources.extend(rules.iter());
    roles_sources.push(role);
    let manifests = list(&c["requirementManifests"], 33, 0)?;
    ensure(manifests.len() == roles_sources.len(), "manifest coverage")?;
    let mut covered = Vec::new();
    for source in &roles_sources {
        covered.push(s(&source["id"])?);
    }
    covered.sort();
    for window in covered.windows(2) {
        ensure(window[0] < window[1], "unique snapshot IDs")?;
    }
    for (index, manifest) in manifests.iter().enumerate() {
        exact(
            manifest,
            "applicabilityReason requiredSkillIds sourceDigest sourceId sourceLocations taskId",
        )?;
        ensure(manifest["taskId"] == task_id, "manifest task")?;
        text(&manifest["applicabilityReason"], 2048)?;
        let required = list(&manifest["requiredSkillIds"], 16, 0)?;
        for required_id in required {
            id(required_id)?;
        }
        sorted_strings(required.iter().filter_map(|v| v.as_str()))?;
        let locations = list(&manifest["sourceLocations"], 32, 1)?;
        for location in locations {
            text(location, 512)?;
        }
        sorted_strings(locations.iter().filter_map(|v| v.as_str()))?;
        ensure(
            manifest["sourceDigest"] == roles_sources[index]["contentDigest"]
                && manifest["sourceId"] == roles_sources[index]["id"],
            "manifest source",
        )?;
    }
    let mandatory_union: std::collections::BTreeSet<&str> = manifests
        .iter()
        .filter_map(|m| m["requiredSkillIds"].as_array())
        .flat_map(|values| values.iter().filter_map(|v| v.as_str()))
        .collect();
    let mandatory: Vec<&str> = skills
        .iter()
        .filter_map(|skill| {
            if skill["requirement"] == "mandatory" {
                skill["id"].as_str()
            } else {
                None
            }
        })
        .collect();
    let mandatory_set: std::collections::BTreeSet<&str> = mandatory.iter().copied().collect();
    ensure(mandatory_set == mandatory_union, "mandatory skill union")?;
    Ok(())
}

fn rubric(x: &Value) -> Result<()> {
    exact(x, "dimensions evaluatorRole id version")?;
    id(&x["id"])?;
    integer(&x["version"], 1, 2147483647)?;
    tags(
        &x["evaluatorRole"],
        &[
            "orchestrator",
            "research",
            "implementation",
            "review",
            "verification",
            "remediation",
        ],
    )?;
    for dimension in list(&x["dimensions"], 16, 1)? {
        exact(dimension, "description id")?;
        id(&dimension["id"])?;
        text(&dimension["description"], 2048)?;
    }
    Ok(())
}

fn criteria(xs: &Value) -> Result<()> {
    let entries = list(xs, 32, 1)?;
    let mut names = Vec::new();
    for entry in entries {
        exact(entry, "description id requirement")?;
        id(&entry["id"])?;
        text(&entry["description"], 2048)?;
        tags(&entry["requirement"], &["required", "optional"])?;
        names.push(s(&entry["id"])?.to_string());
    }
    ensure(
        names.len()
            == names
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
        "unique criterion IDs",
    )?;
    ensure(
        entries
            .iter()
            .any(|entry| entry["requirement"] == "required"),
        "required criterion",
    )
}

fn child_profile(x: &Value) -> Result<()> {
    exact(x, "accessChanges commands connectors coordinationActions externalSources nativeDelegation readPaths tools writePaths")?;
    // Child access lists stay empty in this slice; the closed tool set is
    // verified by the component checker against the selected role.
    for name in [
        "readPaths",
        "writePaths",
        "commands",
        "externalSources",
        "connectors",
    ] {
        ensure(x[name] == json!([]), "no live child access")?;
    }
    ensure(x["nativeDelegation"] == "denied", "native delegation")?;
    ensure(x["accessChanges"] == "request-only", "request-only access")?;
    list(&x["coordinationActions"], 16, 0)?;
    let tools = list(&x["tools"], 64, 0)?;
    for tool in tools {
        id(tool)?;
    }
    Ok(())
}

fn child_assignment(x: &Value) -> Result<()> {
    exact(x, "baseCommit criteria dependencies description expectedOutput inputs label schemaVersion worktreeGroupId worktreeScope")?;
    ensure(x["schemaVersion"] == 1, "assignment version")?;
    text(&x["label"], 512)?;
    text(&x["description"], 4096)?;
    text(&x["expectedOutput"], 2048)?;
    ensure(x["dependencies"] == json!([]), "child dependencies")?;
    exact(&x["worktreeScope"], "readPaths writePaths")?;
    ensure(
        x["worktreeScope"]["readPaths"] == json!([])
            && x["worktreeScope"]["writePaths"] == json!([]),
        "child worktree scope",
    )?;
    id(&x["worktreeGroupId"])?;
    let base = s(&x["baseCommit"])?;
    ensure(
        (base.len() == 40 || base.len() == 64) && valid_hex63(base),
        "Git object ID syntax only",
    )?;
    for input in list(&x["inputs"], 32, 0)? {
        exact(input, "content contentDigest id kind")?;
        ensure(input["kind"] == "inline", "child input kind")?;
        id(&input["id"])?;
        let content = s(&input["content"])?;
        ensure(
            !content.is_empty() && content.len() <= 4096,
            "input content",
        )?;
        ensure(
            digest(&input["contentDigest"])? == sha256_hex(content.as_bytes()),
            "input content digest",
        )?;
    }
    criteria(&x["criteria"])
}

fn branch_name(x: &Value) -> Result<&str> {
    let name = text(x, 256)?;
    let forbidden_control = name
        .bytes()
        .any(|b| b <= b'\x20' || b == b'\x7f' || b"~^:?*[\\".contains(&b));
    ensure(
        name != "main"
            && name != "HEAD"
            && name != "@"
            && !name.starts_with('-')
            && !forbidden_control
            && !name.contains("..")
            && !name.contains("@{"),
        "Git ref syntax only",
    )?;
    ensure(
        name.split('/').all(|part| {
            !part.is_empty()
                && !part.starts_with('.')
                && !part.ends_with('.')
                && !part.ends_with(".lock")
        }),
        "Git ref components",
    )?;
    Ok(name)
}

fn bindings_list(xs: &Value) -> Result<()> {
    let entries = list(xs, 32, 0)?;
    let mut names = Vec::new();
    for entry in entries {
        exact(entry, "name value")?;
        let name = s(&entry["name"])?;
        let first = name
            .chars()
            .next()
            .ok_or(HierarchyStorageError("binding name"))?;
        let ok = (1..=128).contains(&name.len())
            && (first.is_ascii_alphabetic() || first == '_')
            && name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b));
        ensure(ok, "binding name")?;
        ensure(s(&entry["value"])?.len() <= 2048, "binding value")?;
        names.push(name.to_string());
    }
    ensure(
        names.len()
            == names
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
        "sorted unique bindings",
    )?;
    ensure(names.windows(2).all(|w| w[0] < w[1]), "binding order")
}

fn executable_identity(x: &Value) -> Result<()> {
    ensure(canonical(x)?.len() <= 65536, "executable identity 64 KiB")?;
    let wrapped = tags(&x["kind"], &["direct", "wrapped"])? == "wrapped";
    match wrapped {
        true => exact(x, "components executable kind schemaVersion")?,
        false => exact(x, "executable kind schemaVersion")?,
    }
    ensure(x["schemaVersion"] == 1, "executable version")?;
    exact(&x["executable"], "contentDigest path")?;
    source_path(&x["executable"]["path"], 2732)?;
    digest(&x["executable"]["contentDigest"])?;
    if wrapped {
        let components = list(&x["components"], 64, 1)?;
        let mut fingerprints = Vec::new();
        for component in components {
            exact(component, "contentDigest kind path")?;
            tags(
                &component["kind"],
                &["wrapper", "interpreter", "package-manifest", "package-file"],
            )?;
            source_path(&component["path"], 2732)?;
            digest(&component["contentDigest"])?;
            fingerprints.push(canonical(
                &json!({"kind": component["kind"], "path": component["path"]}),
            )?);
        }
        let mut unique = fingerprints.clone();
        unique.sort();
        unique.dedup();
        ensure(unique.len() == fingerprints.len(), "unique components")?;
    }
    Ok(())
}

fn platform(x: &Value) -> Result<()> {
    exact(x, "architecture os osVersion")?;
    tags(&x["os"], &["macos", "linux", "windows"])?;
    tags(&x["architecture"], &["arm64", "x64"])?;
    text(&x["osVersion"], 512)?;
    Ok(())
}

/// Capability evidence record bound to the configuration's permissions and
/// its selected role; the `unverified` slice decision is part of the shape.
fn capability_evidence(x: &Value, permissions: &Value, role: &str) -> Result<()> {
    ensure(canonical(x)?.len() <= 65536, "capability evidence 64 KiB")?;
    exact(
        x,
        "adapterId adapterVersion assessedAt assessorIdentity checks decision definitionDigest effectiveProfileDigest effectiveSettingsDigest evidenceDigest evidenceId executableIdentity harnessId instructionMechanism platform references requestedProfileDigest role schemaVersion toolVersion",
    )?;
    ensure(x["schemaVersion"] == 1, "evidence version")?;
    ensure(x["decision"] == "unverified", "unverified slice")?;
    ensure(x["role"] == role, "evidence role")?;
    for name in ["evidenceId", "harnessId", "adapterId"] {
        id(&x[name])?;
    }
    for name in [
        "definitionDigest",
        "effectiveProfileDigest",
        "effectiveSettingsDigest",
        "requestedProfileDigest",
    ] {
        digest(&x[name])?;
    }
    for name in ["adapterVersion", "toolVersion", "assessorIdentity"] {
        text(&x[name], 512)?;
    }
    tags(
        &x["instructionMechanism"],
        &["native-agent", "startup-context"],
    )?;
    platform(&x["platform"])?;
    utc_timestamp(&x["assessedAt"])?;
    executable_identity(&x["executableIdentity"])?;
    let references = list(&x["references"], 16, 1)?;
    for reference in references {
        exact(reference, "contentDigest id kind sourceReference")?;
        id(&reference["id"])?;
        tags(&reference["kind"], &["primary-reference", "fixture"])?;
        text(&reference["sourceReference"], 2048)?;
        digest(&reference["contentDigest"])?;
    }
    let checks = list(&x["checks"], 10, 10)?;
    const SURFACES: [&str; 10] = [
        "commands",
        "connectors",
        "delegation",
        "filesystem",
        "instructions",
        "model-policy",
        "network",
        "skills",
        "startup",
        "tools",
    ];
    let mut seen = Vec::new();
    for check in checks {
        exact(check, "reason referenceIds result surface")?;
        seen.push(s(&check["surface"])?);
    }
    seen.sort_unstable();
    ensure(seen.as_slice() == SURFACES, "ten surfaces")?;
    let mut unverified = false;
    for check in checks {
        tags(
            &check["result"],
            &["passed", "failed", "unverified", "not-applicable"],
        )?;
        text(&check["reason"], 2048)?;
        let reference_ids = list(&check["referenceIds"], 16, 1)?;
        let mut resolved_ok = true;
        let mut primary = false;
        let mut fixture = false;
        for reference_id in reference_ids {
            id(reference_id)?;
            match references.iter().find(|r| r["id"] == *reference_id) {
                Some(reference) => {
                    if s(&reference["kind"])? == "primary-reference" {
                        primary = true;
                    } else {
                        fixture = true;
                    }
                }
                None => resolved_ok = false,
            }
        }
        ensure(resolved_ok, "evidence reference")?;
        if check["result"] == "unverified" {
            unverified = true;
        }
        if check["result"] == "passed" || check["result"] == "not-applicable" {
            ensure(primary && fixture, "source and fixture evidence")?;
        }
    }
    ensure(unverified, "unverified check required")?;
    let requested_digest = sha256_hex(&canonical(&permissions["requested"])?);
    let effective_digest = sha256_hex(&canonical(&permissions["effective"])?);
    ensure(
        x["requestedProfileDigest"] == json!(requested_digest)
            && x["effectiveProfileDigest"] == json!(effective_digest),
        "profile digest binding",
    )?;
    sealed(x, "evidenceDigest", "orkworks.capability-evidence.v1")
}

/// Child startup settings: structural closure plus the discovered-instruction
/// binding into the owning configuration's snapshots. Artifact-content joins
/// resolve against retained evidence rows at journal time.
fn startup_settings(
    x: &Value,
    configuration: &Value,
    harness: &Value,
    snapshot_input: Option<&Value>,
) -> Result<()> {
    ensure(canonical(x)?.len() <= 65536, "settings 64 KiB")?;
    exact(
        x,
        "adapterId adapterVersion argv configurationSources deliveryOrder discoveredInstructions harnessId instructionMechanism kind nonsecretBindings schemaVersion",
    )?;
    ensure(x["schemaVersion"] == 1, "settings version")?;
    ensure(x["kind"] == "child-startup-settings", "settings kind")?;
    ensure(
        x["instructionMechanism"] == "startup-context",
        "child instruction mechanism",
    )?;
    for name in [
        "harnessId",
        "adapterId",
        "adapterVersion",
        "instructionMechanism",
    ] {
        ensure(x[name] == harness[name], "settings adapter binding")?;
    }
    for value in list(&x["argv"], 64, 0)? {
        text_min(value, 0, 2048)?;
    }
    bindings_list(&x["nonsecretBindings"])?;
    let sources = list(&x["configurationSources"], 32, 0)?;
    for source in sources {
        exact(source, "contentDigest sourceIdentity sourceRevision")?;
        text(&source["sourceIdentity"], 512)?;
        text(&source["sourceRevision"], 512)?;
        digest(&source["contentDigest"])?;
    }
    sorted_strings(sources.iter().filter_map(|v| v["sourceIdentity"].as_str()))?;
    let discovered = list(&x["discoveredInstructions"], 32, 0)?;
    let mut seen = Vec::new();
    for (index, discovery) in discovered.iter().enumerate() {
        exact(
            discovery,
            "contentDigest precedence snapshotId snapshotKind",
        )?;
        ensure(
            discovery["precedence"] == index as u64,
            "discovery precedence",
        )?;
        id(&discovery["snapshotId"])?;
        digest(&discovery["contentDigest"])?;
        let kind = tags(
            &discovery["snapshotKind"],
            &["rule", "role-template", "skill", "skill-resource"],
        )?;
        let snapshots: &[Value] = match kind {
            "rule" => configuration["rules"].as_array().unwrap_or(&EMPTY_ARRAY),
            "role-template" => std::slice::from_ref(&configuration["roleTemplate"]),
            "skill" => configuration["skills"].as_array().unwrap_or(&EMPTY_ARRAY),
            _ => configuration["skillResources"]
                .as_array()
                .unwrap_or(&EMPTY_ARRAY),
        };
        ensure(
            snapshots.iter().any(|snapshot| {
                snapshot["id"] == discovery["snapshotId"]
                    && snapshot["contentDigest"] == discovery["contentDigest"]
            }),
            "automatic snapshot binding",
        )?;
        seen.push(format!("{}:{}", kind, s(&discovery["snapshotId"])?));
    }
    seen.sort();
    seen.dedup();
    ensure(seen.len() == discovered.len(), "unique discovery")?;
    // Delivery order: one supplied payload first; the reviewer appends exactly
    // one retained-input block immediately after it, then automatic entries in
    // original precedence order.
    let delivery = list(
        &x["deliveryOrder"],
        if snapshot_input.is_some() { 34 } else { 33 },
        if snapshot_input.is_some() { 2 } else { 1 },
    )?;
    let mut supplied = 0;
    let mut retained_inputs = 0;
    let mut automatic: Vec<u64> = Vec::new();
    for (index, entry) in delivery.iter().enumerate() {
        let kind = tags(&entry["kind"], &["automatic", "review-input", "supplied"])?;
        match kind {
            "supplied" => {
                exact(entry, "kind")?;
                supplied += 1;
            }
            "automatic" => {
                exact(entry, "kind precedence")?;
                automatic.push(entry["precedence"].as_u64().unwrap_or(u64::MAX));
            }
            _ => {
                exact(entry, "inputDigest kind")?;
                let digests_to_input = snapshot_input
                    .and_then(|inputs| inputs["inputDigest"].as_str())
                    .is_some_and(|digest| entry["inputDigest"] == json!(digest));
                ensure(digests_to_input, "reviewer retained-input digest")?;
                ensure(
                    index > 0 && delivery[index - 1]["kind"] == "supplied",
                    "review data immediately follows supplied",
                )?;
                retained_inputs += 1;
            }
        }
    }
    ensure(supplied == 1, "one supplied payload")?;
    ensure(
        retained_inputs == usize::from(snapshot_input.is_some()),
        "reviewer retained-input count",
    )?;
    let expected_automatic: Vec<u64> = discovered
        .iter()
        .filter_map(|d| d["precedence"].as_u64())
        .collect();
    ensure(automatic == expected_automatic, "automatic order")?;
    Ok(())
}

static EMPTY_ARRAY: Vec<Value> = Vec::new();

fn output_contract(x: &Value, review_input: &Value) -> Result<()> {
    exact(
        x,
        "declarations maxRehashBytes maxSnapshotBytes schemaVersion",
    )?;
    ensure(x["schemaVersion"] == 1, "output version")?;
    integer(&x["maxSnapshotBytes"], 0, 16_777_216)?;
    integer(&x["maxRehashBytes"], 0, 67_108_864)?;
    let declarations = list(&x["declarations"], 1, 0)?;
    if declarations.is_empty() {
        ensure(
            x["maxSnapshotBytes"] == 0 && x["maxRehashBytes"] == 0,
            "empty output caps",
        )?;
    } else {
        let declaration = &declarations[0];
        exact(declaration, "id kind maxBytes source")?;
        id(&declaration["id"])?;
        ensure(
            declaration["kind"] == "assignment_evaluation",
            "child evaluation declaration",
        )?;
        exact(&declaration["source"], "kind")?;
        ensure(
            declaration["source"]["kind"] == "server-artifact",
            "declared server artifact",
        )?;
        integer(&declaration["maxBytes"], 1, 524_288)?;
        ensure(
            review_input["evaluationOutputDeclarationId"] == declaration["id"],
            "declared evaluation id",
        )?;
        ensure(
            declaration["maxBytes"].as_u64().unwrap_or(0)
                <= x["maxSnapshotBytes"].as_u64().unwrap_or(0),
            "output aggregate capacity",
        )?;
        ensure(
            x["maxRehashBytes"].as_u64().unwrap_or(0)
                >= 2 * x["maxSnapshotBytes"].as_u64().unwrap_or(1),
            "output rehash capacity",
        )?;
    }
    Ok(())
}

fn review_input(x: &Value) -> Result<()> {
    if x.is_null() {
        return Ok(());
    }
    exact(x, "evaluationOutputDeclarationId evidenceSources kind maxReadBytes outputDeclarationIds schemaVersion target")?;
    ensure(x["schemaVersion"] == 1, "review input version")?;
    ensure(x["kind"] == "accepted-child-result", "review input kind")?;
    exact(
        &x["target"],
        "assignmentIdentity resultDigest resultRevision",
    )?;
    child_identity(&x["target"]["assignmentIdentity"])?;
    integer(&x["target"]["resultRevision"], 1, 32)?;
    digest(&x["target"]["resultDigest"])?;
    ensure(x["outputDeclarationIds"] == json!([]), "review output ids")?;
    for pair in list(&x["evidenceSources"], 113, 0)? {
        exact(pair, "report source")?;
    }
    id(&x["evaluationOutputDeclarationId"])?;
    integer(&x["maxReadBytes"], 1, 262_144)?;
    Ok(())
}

// --- child-source record validators and key reconstruction.

/// Store/journal context for key reconstruction. The store owns one
/// workspace; the joined run id comes from the plan being journaled or
/// from a retained peer row's frozen identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct SourceContext<'a> {
    pub workspace: &'a str,
    pub run_id: &'a str,
}

fn git_object_id(x: &Value) -> Result<&str> {
    let value = text(x, 64)?;
    ensure(
        (value.len() == 40 || value.len() == 64) && valid_hex63(value),
        "Git object ID syntax only",
    )?;
    Ok(value)
}

fn plan_group(group: &Value) -> Result<()> {
    exact(group, "absolutePath baseCommit branchName groupId")?;
    id(&group["groupId"])?;
    branch_name(&group["branchName"])?;
    native_path(&group["absolutePath"])?;
    git_object_id(&group["baseCommit"])?;
    Ok(())
}

/// `ChildSourcePlanV1`; the key's workspace comes from the store context and
/// `runId` from the parent plan binding.
fn plan_identity(p: &Value, workspace: &str) -> Result<Value> {
    exact(
        p,
        "batches kind maxParallelChildren parentPlanBinding planDefinitionDigest scopeText schemaVersion task",
    )?;
    ensure(p["schemaVersion"] == 1, "plan version")?;
    ensure(p["kind"] == "single-task-child-source", "plan kind")?;
    let binding = &p["parentPlanBinding"];
    exact(
        binding,
        "bootstrapConfigurationDigest inputDigest parentSessionId planId planRevision repositoryBindingDigest runDefinitionDigest runId sourceWorktreeBindingDigest preparationRevision",
    )?;
    run_id(&binding["runId"])?;
    id(&binding["parentSessionId"])?;
    id(&binding["planId"])?;
    integer(&binding["planRevision"], 1, 64)?;
    for name in [
        "bootstrapConfigurationDigest",
        "inputDigest",
        "repositoryBindingDigest",
        "runDefinitionDigest",
        "sourceWorktreeBindingDigest",
    ] {
        digest(&binding[name])?;
    }
    integer(&binding["preparationRevision"], 1, 64)?;
    let task = &p["task"];
    exact(task, "configuration group initialPrompt input taskId")?;
    id(&task["taskId"])?;
    exact(
        &task["configuration"],
        "configurationDigest configurationId",
    )?;
    id(&task["configuration"]["configurationId"])?;
    digest(&task["configuration"]["configurationDigest"])?;
    if task["input"].is_null() {
        // worker plan: no retained review input reference
    } else {
        input_ref(&task["input"])?;
    }
    text(&task["initialPrompt"], 4096)?;
    plan_group(&task["group"])?;
    let batches = list(&p["batches"], 1, 1)?;
    let batch = list(&batches[0], 1, 1)?;
    ensure(batch[0] == p["task"]["taskId"], "single-task batch")?;
    integer(&p["maxParallelChildren"], 1, 1)?;
    text(&p["scopeText"], 4096)?;
    sealed(p, "planDefinitionDigest", "orkworks.child-source-plan.v1")?;
    Ok(json!({
        "workspaceId": workspace,
        "runId": binding["runId"],
        "planId": binding["planId"],
        "planRevision": binding["planRevision"],
        "planDefinitionDigest": p["planDefinitionDigest"],
    }))
}

/// Complete child v3 configuration (`OutputAssignmentConfiguration`); the
/// key's `TaskKey` carries the store workspace and the joined run id.
fn configuration_identity(c: &Value, context: &SourceContext) -> Result<Value> {
    exact(
        c,
        "assignment assignmentKind capabilityEvidence configurationDigest configurationId harness model outputContract parentSessionId permissions planId planRevision renderedInstructions renderedInstructionsDigest repositoryBinding repositoryId requirementManifests roleTemplate rubric rules schemaVersion skillResources skills sourceWorktreeBinding taskCategory taskId workspaceId reviewInput",
    )?;
    ensure(c["schemaVersion"] == 3, "configuration version")?;
    ensure(c["assignmentKind"] == "child", "child configuration")?;
    for name in [
        "configurationId",
        "repositoryId",
        "parentSessionId",
        "planId",
        "taskId",
    ] {
        id(&c[name])?;
    }
    ensure(valid_workspace(s(&c["workspaceId"])?), "workspace")?;
    integer(&c["planRevision"], 1, 64)?;
    text(&c["taskCategory"], 512)?;
    repository_binding(&c["repositoryBinding"])?;
    let worktree = &c["sourceWorktreeBinding"];
    ensure(
        canonical(&worktree["repositoryBinding"])? == canonical(&c["repositoryBinding"])?,
        "worktree repository binding",
    )?;
    worktree_binding(worktree)?;
    let role = s(&c["roleTemplate"]["role"])?;
    role_template(&c["roleTemplate"])?;
    rules_skills_resources(c, s(&c["taskId"])?)?;
    rubric(&c["rubric"])?;
    child_assignment(&c["assignment"])?;
    child_profile(&c["permissions"]["requested"])?;
    child_profile(&c["permissions"]["effective"])?;
    review_input(&c["reviewInput"])?;
    output_contract(&c["outputContract"], &c["reviewInput"])?;
    let harness = &c["harness"];
    exact(
        harness,
        "adapterId adapterVersion definitionDigest effectiveSettingsDigest evidenceDigest evidenceId executableIdentity harnessId instructionMechanism platform toolVersion",
    )?;
    let evidence = &c["capabilityEvidence"];
    for name in [
        "adapterId",
        "adapterVersion",
        "evidenceDigest",
        "evidenceId",
        "effectiveSettingsDigest",
        "instructionMechanism",
        "toolVersion",
    ] {
        ensure(harness[name] == evidence[name], "harness/evidence binding")?;
    }
    executable_identity(&harness["executableIdentity"])?;
    ensure(
        canonical(&harness["executableIdentity"])? == canonical(&evidence["executableIdentity"])?,
        "shared executable identity bytes",
    )?;
    capability_evidence(evidence, &c["permissions"], role)?;
    model_record(&c["model"])?;
    let rendered = s(&c["renderedInstructions"])?;
    ensure(rendered.len() <= 262144, "rendered child byte ceiling")?;
    ensure(
        digest(&c["renderedInstructionsDigest"])? == sha256_hex(rendered.as_bytes()),
        "render digest",
    )?;
    sealed(
        c,
        "configurationDigest",
        "orkworks.assignment-configuration.v3",
    )?;
    ensure(
        s(&c["workspaceId"])? == context.workspace,
        "configuration workspace join",
    )?;
    Ok(json!({
        "task": {
            "workspaceId": context.workspace,
            "runId": context.run_id,
            "planId": c["planId"],
            "planRevision": c["planRevision"],
            "taskId": c["taskId"],
        },
        "configurationId": c["configurationId"],
        "configurationDigest": c["configurationDigest"],
    }))
}

/// Configuration-side model join record: exact closed fields; the detailed
/// policy artifact carries the mode and pins its own digest.
fn model_record(m: &Value) -> Result<()> {
    exact(
        m,
        "adapterGeneration mode modelId policyDigest policyId schemaVersion",
    )?;
    ensure(m["schemaVersion"] == 1, "model version")?;
    id(&m["adapterGeneration"])?;
    digest(&m["policyDigest"])?;
    Ok(())
}

/// `ChildReviewInputSnapshotV1`: the retained reviewer input, sealed by the
/// `child-review-input` domain; its identity key carries the reviewer task.
fn snapshot_identity(x: &Value) -> Result<Value> {
    exact(
        x,
        "criteria evidence inputDigest result rubric schemaVersion task target",
    )?;
    ensure(x["schemaVersion"] == 1, "snapshot version")?;
    task_key(&x["task"])?;
    exact(
        &x["target"],
        "assignmentIdentity resultDigest resultRevision",
    )?;
    child_identity(&x["target"]["assignmentIdentity"])?;
    integer(&x["target"]["resultRevision"], 1, 32)?;
    digest(&x["target"]["resultDigest"])?;
    criteria(&x["criteria"])?;
    rubric(&x["rubric"])?;
    for pair in list(&x["evidence"], 113, 0)? {
        exact(pair, "report source")?;
    }
    sealed(x, "inputDigest", "orkworks.child-review-input.v1")?;
    Ok(json!({
        "task": x["task"],
        "inputDigest": x["inputDigest"],
    }))
}

/// Child source evidence object: exact cited artifact bytes under strict
/// padded canonical Base64; the key carries the raw content digest and the
/// source digest covers the complete body.
fn evidence_identity(x: &Value) -> Result<Value> {
    exact(
        x,
        "contentBase64 contentDigest referenceId schemaVersion sizeBytes task",
    )?;
    ensure(x["schemaVersion"] == 1, "evidence version")?;
    let task = task_key(&x["task"])?;
    id(&x["referenceId"])?;
    let encoded = s(&x["contentBase64"])?;
    ensure(encoded.len() <= 87_384, "encoded evidence ceiling")?;
    let bytes = base64_decoded(encoded, 0, 65_536)?;
    let size = integer(&x["sizeBytes"], 0, 65_536)?;
    ensure(size as usize == bytes.len(), "evidence size binding")?;
    ensure(
        digest(&x["contentDigest"])? == sha256_hex(&bytes),
        "evidence content digest",
    )?;
    Ok(json!({
        "contentDigest": x["contentDigest"],
        "referenceId": x["referenceId"],
        "task": task,
    }))
}

/// TaskKey projection of a child assignment identity.
fn identity_task_key(identity: &Value) -> Value {
    json!({
        "planId": identity["planId"],
        "planRevision": identity["planRevision"],
        "runId": identity["runId"],
        "taskId": identity["taskId"],
        "workspaceId": identity["workspaceId"],
    })
}

/// `ChildAssignmentSourceV1`, joined against its own frozen identity.
fn assignment_source_identity(x: &Value) -> Result<Value> {
    exact(
        x,
        "allocation approval attachmentId configuration criteria input plan reservation rubric schemaVersion assignmentIdentity",
    )?;
    ensure(x["schemaVersion"] == 1, "source version")?;
    let identity = child_identity(&x["assignmentIdentity"])?;
    let task_key_value = identity_task_key(&identity);
    let plan = &x["plan"];
    exact(
        plan,
        "planDefinitionDigest planId planRevision runId workspaceId",
    )?;
    ensure(
        plan["workspaceId"] == identity["workspaceId"]
            && plan["runId"] == identity["runId"]
            && plan["planId"] == identity["planId"]
            && plan["planRevision"] == identity["planRevision"],
        "source plan projection",
    )?;
    plan_ref(plan)?;
    let configuration = &x["configuration"];
    exact(configuration, "configurationDigest configurationId task")?;
    ensure(
        canonical(&configuration["task"])? == canonical(&task_key_value)?,
        "source configuration binding",
    )?;
    configuration_ref(configuration)?;
    let approval = &x["approval"];
    exact(approval, "approvalDigest approvalId plan")?;
    ensure(
        canonical(&approval["plan"])? == canonical(plan)?,
        "source approval plan",
    )?;
    approval_ref(approval)?;
    let reservation = &x["reservation"];
    exact(reservation, "reservationDigest reservationId task")?;
    ensure(
        canonical(&reservation["task"])? == canonical(&task_key_value)?,
        "source reservation task",
    )?;
    reservation_ref(reservation)?;
    let allocation = &x["allocation"];
    exact(allocation, "allocationDigest allocationId reservation")?;
    ensure(
        canonical(&allocation["reservation"])? == canonical(reservation)?,
        "source allocation reservation",
    )?;
    allocation_ref(allocation)?;
    id(&x["attachmentId"])?;
    if x["input"].is_null() {
        // worker source: no retained input reference
    } else {
        ensure(
            canonical(&x["input"]["task"])? == canonical(&task_key_value)?,
            "source input task",
        )?;
        input_ref(&x["input"])?;
    }
    let entries = list(&x["criteria"], 32, 1)?;
    let names: Vec<&str> = entries
        .iter()
        .map(|entry| s(&entry["id"]))
        .collect::<Result<Vec<_>>>()?;
    let mut sorted_names = names.clone();
    sorted_names.sort_unstable();
    ensure(names == sorted_names, "stable-ID criteria order")?;
    for entry in entries {
        exact(entry, "description id requirement")?;
        id(&entry["id"])?;
        text(&entry["description"], 2048)?;
        tags(&entry["requirement"], &["required", "optional"])?;
    }
    ensure(
        entries
            .iter()
            .any(|entry| entry["requirement"] == "required"),
        "required criterion",
    )?;
    rubric(&x["rubric"])?;
    Ok(identity)
}

/// A validated child-source object body with its reconstructed canonical
/// `ObjectKey` and its owning contract's source digest. For the five
/// admitted kinds: plan/configuration/input/evidence/assignment-source.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdmittedSource {
    pub key: Value,
    /// Source digest exactly as the owning contract reconstructs it.
    pub digest: String,
    /// Canonical bytes; identical to the admitted input.
    pub bytes: Vec<u8>,
}

/// Metadata record kinds for the four `child_*` tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MetaKind {
    PlanApproval,
    Reservation,
    Allocation,
    Attachment,
}

/// A validated child metadata record with its reconstructed canonical key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AdmittedMeta {
    pub table: MetaKind,
    pub key: Value,
    /// The value envelope's base: `{schemaVersion:1, body, [digest, receipt],
    /// lastCommit}` where `lastCommit` is supplied by the journal.
    pub value_base: Value,
    /// Source-domain digest of the record body.
    pub digest: String,
}

/// `ChildPlanApprovalSourceV1`: exactly six closed fields; the approval
/// digest is the `child-source-plan-approval` domain over the whole body.
fn approval_source(x: &Value) -> Result<Value> {
    exact(
        x,
        "approvalId plan requestDigest reviewContextId schemaVersion sidecarGeneration",
    )?;
    ensure(x["schemaVersion"] == 1, "approval version")?;
    id(&x["approvalId"])?;
    digest(&x["reviewContextId"])?;
    digest(&x["sidecarGeneration"])?;
    plan_ref(&x["plan"])?;
    Ok(json!({
        "approvalId": x["approvalId"],
        "plan": x["plan"],
    }))
}

/// `ChildReservationSourceV1`: sealed by `child-source-reservation`; its key
/// is the frozen task key.
fn reservation_source(x: &Value) -> Result<Value> {
    exact(x, "approval assignmentTaskVersion childSessionId configurationDigest configurationId reservationDigest reservationId schemaVersion task")?;
    ensure(x["schemaVersion"] == 1, "reservation version")?;
    let task = task_key(&x["task"])?;
    id(&x["reservationId"])?;
    id(&x["childSessionId"])?;
    id(&x["configurationId"])?;
    digest(&x["configurationDigest"])?;
    approval_ref(&x["approval"])?;
    integer(&x["assignmentTaskVersion"], 1, 2_147_483_647)?;
    sealed(x, "reservationDigest", RESERVATION_DOMAIN)?;
    Ok(task)
}

fn allocation_source(x: &Value) -> Result<Value> {
    exact(
        x,
        "allocationDigest allocationId allocatedWorktree group schemaVersion reservation",
    )?;
    ensure(x["schemaVersion"] == 1, "allocation version")?;
    let reservation = reservation_ref(&x["reservation"])?;
    id(&x["allocationId"])?;
    plan_group(&x["group"])?;
    let allocated = &x["allocatedWorktree"];
    ensure(allocated["kind"] == "linked", "allocated worktree kind")?;
    ensure(
        canonical(&allocated["canonicalRoot"])? == canonical(&x["group"]["absolutePath"])?,
        "allocated planned path",
    )?;
    worktree_binding(allocated)?;
    sealed(x, "allocationDigest", "orkworks.child-source-allocation.v1")?;
    Ok(json!({
        "allocationId": x["allocationId"],
        "reservation": reservation,
    }))
}

fn attachment_source(x: &Value) -> Result<Value> {
    exact(
        x,
        "allocation assignmentIdentity attachmentDigest attachmentId reservation schemaVersion",
    )?;
    ensure(x["schemaVersion"] == 1, "attachment version")?;
    id(&x["attachmentId"])?;
    let reservation = reservation_ref(&x["reservation"])?;
    let allocation = &x["allocation"];
    ensure(
        canonical(&allocation["reservation"])? == canonical(&reservation)?,
        "attachment reservation join",
    )?;
    allocation_ref(allocation)?;
    let identity = child_identity(&x["assignmentIdentity"])?;
    sealed(x, "attachmentDigest", "orkworks.child-source-attachment.v1")?;
    Ok(json!(identity["childSessionId"]))
}

/// Source digest domains of the child-row records.
pub(crate) const APPROVAL_RECEIPT_CAP: usize = 4096;
pub(crate) const METADATA_WRAPPER_CAP: usize = 16 * 1024;
pub(crate) const APPROVAL_DOMAIN: &str = "orkworks.child-source-plan-approval.v1";
pub(crate) const RESERVATION_DOMAIN: &str = "orkworks.child-source-reservation.v1";
pub(crate) const ALLOCATION_DOMAIN: &str = "orkworks.child-source-allocation.v1";
pub(crate) const ATTACHMENT_DOMAIN: &str = "orkworks.child-source-attachment.v1";
pub(crate) const EVIDENCE_DOMAIN: &str = "orkworks.child-source-evidence.v1";
pub(crate) const ASSIGNMENT_SOURCE_DOMAIN: &str = "orkworks.child-assignment-source.v1";
pub(crate) const REVIEW_INPUT_DOMAIN: &str = "orkworks.child-review-input.v1";
pub(crate) const REQUEST_DOMAIN: &str = "orkworks.child-source-plan-request.v1";

/// Admission of one child metadata source record.
pub(crate) fn admit_meta_record(kind: MetaKind, bytes: &[u8]) -> Result<AdmittedMeta> {
    let value = decode(bytes, METADATA_WRAPPER_CAP)?;
    let (key, digest) = match kind {
        MetaKind::PlanApproval => {
            let key = approval_source(&value)?;
            (
                key,
                sha256_hex(
                    &[
                        APPROVAL_DOMAIN.as_bytes(),
                        b"\n",
                        canonical(&value)?.as_slice(),
                    ]
                    .concat(),
                ),
            )
        }
        MetaKind::Reservation => {
            let key = reservation_source(&value)?;
            (key, s_field(&value, "reservationDigest")?)
        }
        MetaKind::Allocation => {
            let key = allocation_source(&value)?;
            (key, s_field(&value, "allocationDigest")?)
        }
        MetaKind::Attachment => {
            let key = attachment_source(&value)?;
            (
                key,
                sha256_hex(
                    &[
                        ATTACHMENT_DOMAIN.as_bytes(),
                        b"\n",
                        canonical(&without_field(&value, "attachmentDigest"))?.as_slice(),
                    ]
                    .concat(),
                ),
            )
        }
    };
    Ok(AdmittedMeta {
        table: kind,
        key,
        value_base: json!({"schemaVersion": 1, "body": value}),
        digest,
    })
}

/// The digest-sealed field removed from an admitted record body.
pub(crate) fn without_field(value: &Value, field: &str) -> Value {
    let mut body = value.clone();
    if let Some(map) = body.as_object_mut() {
        map.remove(field);
    }
    body
}

/// Admission of one retained child source object from exact canonical bytes.
pub(crate) fn admit_source_object(
    kind: ObjectKind,
    context: &SourceContext,
    bytes: &[u8],
) -> Result<AdmittedSource> {
    let value = decode(bytes, kind.max_bytes())?;
    let (identity, digest) = match kind {
        ObjectKind::Plan => (
            plan_identity(&value, context.workspace)?,
            s_field(&value, "planDefinitionDigest")?,
        ),
        ObjectKind::Configuration => (
            configuration_identity(&value, context)?,
            s_field(&value, "configurationDigest")?,
        ),
        ObjectKind::ReviewInput => {
            let identity = snapshot_identity(&value)?;
            let digest = {
                let body = without_field(&value, "inputDigest");
                sha256_hex(
                    &[
                        REVIEW_INPUT_DOMAIN.as_bytes(),
                        b"\n",
                        canonical(&body)?.as_slice(),
                    ]
                    .concat(),
                )
            };
            (identity, digest)
        }
        ObjectKind::Evidence => {
            let identity = evidence_identity(&value)?;
            (
                identity,
                sha256_hex(
                    &[
                        EVIDENCE_DOMAIN.as_bytes(),
                        b"\n",
                        canonical(&value)?.as_slice(),
                    ]
                    .concat(),
                ),
            )
        }
        ObjectKind::AssignmentSource => {
            let identity = assignment_source_identity(&value)?;
            (
                identity,
                sha256_hex(
                    &[
                        ASSIGNMENT_SOURCE_DOMAIN.as_bytes(),
                        b"\n",
                        canonical(&value)?.as_slice(),
                    ]
                    .concat(),
                ),
            )
        }
    };
    Ok(AdmittedSource {
        key: json!({"identity": identity, "kind": kind.tag()}),
        digest,
        bytes: bytes.to_vec(),
    })
}

fn s_field(value: &Value, field: &str) -> Result<String> {
    value[field]
        .as_str()
        .map(|s| s.to_string())
        .ok_or(HierarchyStorageError("digest"))
}

impl MetaKind {
    /// Canonical UTF-8 table name (manifest and SQL catalog).
    pub(crate) fn table_name(self) -> &'static str {
        match self {
            MetaKind::PlanApproval => "child_plan_approvals",
            MetaKind::Reservation => "child_reservations",
            MetaKind::Allocation => "child_allocations",
            MetaKind::Attachment => "child_attachments",
        }
    }
}

/// Validated metadata row: the four `child_*` tables' complete key/value
/// pair with its excluded-lastCommit CommitRef.
pub(crate) struct MetadataRow {
    pub table: &'static str,
    pub key: Value,
    pub value: Value,
    pub last_commit: CommitRef,
}

/// Complete metadata pair validation: envelope shape per table, the key
/// reconstructed through the record body's explicit joins, digest fields,
/// the approval receipt (approvals table) and the 16 KiB wrapper cap.
pub(crate) fn check_metadata_row(
    table: MetaKind,
    key: &Value,
    value: &Value,
) -> Result<MetadataRow> {
    let last_commit = commit_ref(&value["lastCommit"])?;
    let row_value = {
        let row = json!({"key": key, "value": value});
        let bytes = canonical(&row)?;
        ensure(
            bytes.len() <= METADATA_WRAPPER_CAP,
            "metadata complete wrapper 16 KiB",
        )?;
        value.clone()
    };
    match table {
        MetaKind::PlanApproval => {
            exact(value, "body digest lastCommit receipt schemaVersion")?;
            let body = approval_source(&value["body"])?;
            ensure(canonical(key)? == canonical(&body)?, "approval key join")?;
            let approval_digest = sha256_hex(
                &[
                    APPROVAL_DOMAIN.as_bytes(),
                    b"\n",
                    canonical(&value["body"])?.as_slice(),
                ]
                .concat(),
            );
            ensure(
                value["digest"] == json!(approval_digest),
                "approval digest join",
            )?;
            let receipt = &value["receipt"];
            exact(
                receipt,
                "approvalDigest approvalId commit plan requestDigest schemaVersion",
            )?;
            ensure(receipt["schemaVersion"] == 1, "receipt version")?;
            ensure(
                canonical(receipt)?.len() <= APPROVAL_RECEIPT_CAP,
                "approval receipt 4 KiB",
            )?;
            ensure(
                receipt["plan"] == value["body"]["plan"]
                    && receipt["approvalId"] == value["body"]["approvalId"]
                    && receipt["approvalDigest"] == value["digest"],
                "receipt join",
            )?;
            commit_ref(&receipt["commit"])?;
        }
        MetaKind::Reservation => {
            exact(value, "body lastCommit schemaVersion")?;
            let body = reservation_source(&value["body"])?;
            ensure(canonical(key)? == canonical(&body)?, "reservation key join")?;
        }
        MetaKind::Allocation => {
            exact(value, "body lastCommit schemaVersion")?;
            let body = allocation_source(&value["body"])?;
            ensure(canonical(key)? == canonical(&body)?, "allocation key join")?;
        }
        MetaKind::Attachment => {
            exact(value, "body lastCommit schemaVersion")?;
            let body = attachment_source(&value["body"])?;
            ensure(canonical(key)? == canonical(&body)?, "attachment key join")?;
        }
    }
    ensure(
        canonical(&row_value)?.len() <= 2 * 1024 * 1024,
        "row ceiling",
    )?;
    Ok(MetadataRow {
        table: table.table_name(),
        key: key.clone(),
        value: value.clone(),
        last_commit,
    })
}
