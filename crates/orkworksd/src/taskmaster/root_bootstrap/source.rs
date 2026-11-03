use super::wire::*;
use super::{Result, RootBootstrapError};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
const ROLES: &[&str] = &[
    "orchestrator",
    "research",
    "implementation",
    "review",
    "verification",
    "remediation",
];
const ACTIONS: &[&str] = &[
    "orkworks.coordination.child-result.record.v1",
    "orkworks.coordination.clarification.request.v1",
    "orkworks.coordination.event.wait.v1",
    "orkworks.coordination.plan.propose.v1",
    "orkworks.coordination.run.read.v1",
    "orkworks.coordination.task.launch.v1",
];
const RESULT_TOOLS: &[&str] = &[
    "orkworks.result.receipt.read.v1",
    "orkworks.result.submit.v1",
];
pub(in crate::taskmaster) fn native_path(x: &Value, max: usize) -> Result<()> {
    exact(x, "encoding base64")?;
    tags(&x["encoding"], &["unix-bytes", "windows-utf16le"])?;
    let encoded = text(&x["base64"], max.div_ceil(3) * 4, 1)?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| RootBootstrapError("native base64"))?;
    ensure(
        STANDARD.encode(&bytes) == encoded && !bytes.is_empty() && bytes.len() <= max,
        "native path bytes",
    )?;
    if x["encoding"] == "unix-bytes" {
        ensure(
            bytes.first() == Some(&b'/') && !bytes.contains(&0),
            "native Unix path",
        )?;
    } else {
        ensure(bytes.len() % 2 == 0, "native Windows path")?;
        let u: Vec<_> = bytes
            .chunks_exact(2)
            .map(|b| u16::from_le_bytes([b[0], b[1]]))
            .collect();
        let drive = u.len() >= 3
            && ((65..=90).contains(&u[0]) || (97..=122).contains(&u[0]))
            && u[1] == 58
            && u[2] == 92;
        let unc = u.starts_with(&[92, 92])
            && u[2..]
                .iter()
                .position(|v| *v == 92)
                .is_some_and(|n| n > 0 && u.get(n + 3).is_some_and(|v| *v != 92));
        ensure(!u.contains(&0) && (drive || unc), "native Windows path")?;
    }
    Ok(())
}
fn directory(x: &Value) -> Result<()> {
    exact(x, "platform mechanismVersion volumeId fileId")?;
    for k in ["platform", "volumeId", "fileId"] {
        ensure(
            text(&x[k], 128, 1)?
                .bytes()
                .all(|b| (32..=126).contains(&b)),
            "native object",
        )?;
    }
    number(&x["mechanismVersion"], 1, 9007199254740991)?;
    Ok(())
}
pub(in crate::taskmaster) fn repository(x: &Value) -> Result<()> {
    exact(x,"schemaVersion registryEpoch repositoryId canonicalCommonDirectory commonDirectoryObject bindingDigest")?;
    ensure(x["schemaVersion"] == 1, "repository version")?;
    let epoch = s(&x["registryEpoch"])?;
    let repo = s(&x["repositoryId"])?;
    let hex = |v: &str| {
        v.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    };
    ensure(
        epoch.len() == 32
            && hex(epoch)
            && repo.len() == 37
            && repo.starts_with("repo-")
            && hex(&repo[5..]),
        "repository identity",
    )?;
    native_path(&x["canonicalCommonDirectory"], 4096)?;
    directory(&x["commonDirectoryObject"])?;
    ensure(canonical(x)?.len() <= 8192, "repository bytes")?;
    sealed(x, "bindingDigest", "orkworks.repository-binding.v1")
}
pub(in crate::taskmaster) fn worktree(x: &Value, r: &Value) -> Result<()> {
    exact(x,"repositoryBinding canonicalRoot rootObject canonicalPrivateGitDirectory privateGitDirectoryObject kind worktreeBindingDigest")?;
    ensure(x["repositoryBinding"] == *r, "worktree repository")?;
    tags(&x["kind"], &["primary", "linked"])?;
    for k in ["canonicalRoot", "canonicalPrivateGitDirectory"] {
        native_path(&x[k], 4096)?;
    }
    for k in ["rootObject", "privateGitDirectoryObject"] {
        directory(&x[k])?;
    }
    sealed(x, "worktreeBindingDigest", "orkworks.worktree-binding.v1")
}
fn provenance(x: &Value) -> Result<()> {
    tags(&x["kind"], &["local", "nonlocal"])?;
    exact(
        x,
        if x["kind"] == "local" {
            "schemaVersion kind sourceKind sourceIdentity sourceRevision sourcePath"
        } else {
            "schemaVersion kind sourceKind sourceIdentity sourceRevision"
        },
    )?;
    ensure(x["schemaVersion"] == 1, "provenance version")?;
    tags(
        &x["sourceKind"],
        &["repository", "bundled", "user-approved"],
    )?;
    text(&x["sourceIdentity"], 512, 1)?;
    text(&x["sourceRevision"], 512, 1)?;
    if x["kind"] == "local" {
        native_path(&x["sourcePath"], 2048)?;
    } else {
        ensure(x["sourceKind"] != "repository", "repository provenance")?;
    }
    Ok(())
}
pub(in crate::taskmaster) fn executable(x: &Value) -> Result<()> {
    ensure(canonical(x)?.len() <= 65536, "executable bytes")?;
    tags(&x["kind"], &["direct", "wrapped"])?;
    exact(
        x,
        if x["kind"] == "direct" {
            "schemaVersion kind executable"
        } else {
            "schemaVersion kind executable components"
        },
    )?;
    ensure(x["schemaVersion"] == 1, "executable version")?;
    exact(&x["executable"], "path contentDigest")?;
    native_path(&x["executable"]["path"], 2048)?;
    digest(&x["executable"]["contentDigest"])?;
    if x["kind"] == "wrapped" {
        let mut keys = BTreeSet::new();
        for c in list(&x["components"], 64, 1)? {
            exact(c, "kind path contentDigest")?;
            tags(
                &c["kind"],
                &["wrapper", "interpreter", "package-manifest", "package-file"],
            )?;
            native_path(&c["path"], 2048)?;
            digest(&c["contentDigest"])?;
            ensure(
                keys.insert(canonical(&json!({"kind":c["kind"],"path":c["path"]}))?),
                "duplicate executable component",
            )?;
        }
    }
    Ok(())
}
pub(super) fn output(x: &Value) -> Result<()> {
    exact(
        x,
        "schemaVersion declarations maxSnapshotBytes maxRehashBytes",
    )?;
    ensure(x["schemaVersion"] == 1, "output version")?;
    let ds = list(&x["declarations"], 32, 0)?;
    sorted(ds, Some("id"))?;
    let mut sum = 0u64;
    for d in ds {
        exact(d, "id kind maxBytes source")?;
        id(&d["id"])?;
        id(&d["kind"])?;
        exact(&d["source"], "kind")?;
        ensure(
            d["source"]["kind"] == "server-artifact" && d["kind"] != "assignment_evaluation",
            "root output",
        )?;
        sum = sum
            .checked_add(number(&d["maxBytes"], 1, 524288)?)
            .ok_or(RootBootstrapError("output overflow"))?;
    }
    let snapshot = number(&x["maxSnapshotBytes"], 0, 16777216)?;
    let rehash = number(&x["maxRehashBytes"], 0, 67108864)?;
    if ds.is_empty() {
        ensure(snapshot == 0 && rehash == 0, "empty output caps")
    } else {
        ensure(sum <= snapshot && rehash >= 2 * snapshot, "output capacity")
    }
}
pub(super) fn profile(x: &Value, b: &Value) -> Result<()> {
    exact(x,"tools readPaths writePaths commands externalSources connectors coordinationActions nativeDelegation accessChanges")?;
    for k in [
        "readPaths",
        "writePaths",
        "commands",
        "externalSources",
        "connectors",
    ] {
        ensure(x[k] == json!([]), "empty root access")?;
    }
    ensure(
        x["nativeDelegation"] == "denied" && x["accessChanges"] == "request-only",
        "root access policy",
    )?;
    let actions = list(&x["coordinationActions"], 16, 0)?;
    ensure(
        actions
            .iter()
            .map(|a| s(&a["actionId"]))
            .collect::<Result<Vec<_>>>()?
            == ACTIONS,
        "six root actions",
    )?;
    for a in actions {
        exact(a, "actionId workspaceId runId parentSessionId")?;
        for k in ["workspaceId", "runId", "parentSessionId"] {
            ensure(a[k] == b[k], "action owner")?;
        }
    }
    let tools = list(&x["tools"], 64, 0)?;
    for t in tools {
        id(t)?;
    }
    let count = RESULT_TOOLS
        .iter()
        .filter(|t| tools.contains(&json!(t)))
        .count();
    ensure(count == 0 || count == 2, "root tools")?;
    let mut expected: Vec<_> = ACTIONS
        .iter()
        .chain(if count == 2 {
            RESULT_TOOLS.iter()
        } else {
            [].iter()
        })
        .copied()
        .collect();
    expected.sort();
    ensure(
        tools.iter().map(s).collect::<Result<Vec<_>>>()? == expected,
        "root tools",
    )?;
    ensure(
        list(&b["outputContract"]["declarations"], 32, 0)?.is_empty() || count == 2,
        "root result tools",
    )
}
pub(super) fn assignment(x: &Value, run: &Value) -> Result<()> {
    exact(
        x,
        "schemaVersion label description inputs expectedOutput criteria dependencies worktreeScope",
    )?;
    ensure(x["schemaVersion"] == 1, "assignment version")?;
    text(&x["label"], 512, 1)?;
    text(&x["description"], 4096, 1)?;
    text(&x["expectedOutput"], 2048, 1)?;
    let mut ids = BTreeSet::new();
    for i in list(&x["inputs"], 32, 0)? {
        exact(i, "kind id content contentDigest")?;
        ensure(i["kind"] == "inline", "reference input unsupported")?;
        ensure(ids.insert(id(&i["id"])?), "input id")?;
        ensure(
            hash(text(&i["content"], 4096, 1)?) == digest(&i["contentDigest"])?,
            "inline input digest",
        )?;
    }
    let criteria = list(&x["criteria"], 32, 1)?;
    let mut ids = BTreeSet::new();
    for c in criteria {
        exact(c, "id requirement description")?;
        ensure(ids.insert(id(&c["id"])?), "criterion id")?;
        tags(&c["requirement"], &["required", "optional"])?;
        text(&c["description"], 2048, 1)?;
    }
    ensure(
        criteria.iter().any(|c| c["requirement"] == "required"),
        "required criterion",
    )?;
    ensure(x["dependencies"] == json!([]), "root dependencies")?;
    let scope = &x["worktreeScope"];
    exact(
        scope,
        "kind repositoryBindingDigest sourceWorktreeBindingDigest readPaths writePaths",
    )?;
    ensure(
        scope["kind"] == "source-worktree"
            && scope["repositoryBindingDigest"] == run["repositoryBindingDigest"]
            && scope["sourceWorktreeBindingDigest"] == run["sourceWorktreeBindingDigest"],
        "assignment scope",
    )?;
    ensure(
        scope["readPaths"] == json!([]) && scope["writePaths"] == json!([]),
        "empty root scope",
    )
}
fn content(x: &Value, key: &str, digest_key: &str) -> Result<()> {
    ensure(
        hash(text(&x[key], 262144, 1)?) == digest(&x[digest_key])?,
        "snapshot digest",
    )
}
pub(super) fn snapshots(b: &Value) -> Result<()> {
    snapshots_for_assignment(b, "orchestrator", &b["rootAssignmentId"])
}
pub(in crate::taskmaster) fn snapshots_for_assignment(
    b: &Value,
    role: &str,
    task: &Value,
) -> Result<()> {
    let r = &b["roleTemplate"];
    exact(
        r,
        "id version role instructions instructionsDigest contentDigest provenance",
    )?;
    id(&r["id"])?;
    number(&r["version"], 1, 2147483647)?;
    ensure(r["role"] == role, "root role")?;
    content(r, "instructions", "instructionsDigest")?;
    let mut body = r.clone();
    body.as_object_mut()
        .ok_or(RootBootstrapError("closed fields"))?
        .remove("contentDigest");
    ensure(
        raw(&body)? == digest(&r["contentDigest"])?,
        "role descriptor digest",
    )?;
    provenance(&r["provenance"])?;
    let rules = list(&b["rules"], 32, 0)?;
    let resources = list(&b["skillResources"], 32, 0)?;
    ensure(rules.len() + resources.len() <= 32, "rule resource cap")?;
    let skills = list(&b["skills"], 16, 0)?;
    sorted(skills, Some("id"))?;
    sorted(resources, Some("id"))?;
    let mut source_map = BTreeMap::from([(id(&r["id"])?, r)]);
    let mut locators = BTreeSet::new();
    for (i, rule) in rules.iter().enumerate() {
        exact(
            rule,
            "id sourcePath content contentDigest precedence authority provenance",
        )?;
        ensure(
            source_map.insert(id(&rule["id"])?, rule).is_none(),
            "source id",
        )?;
        let p = text(&rule["sourcePath"], 2048, 1)?;
        ensure(
            !p.contains(['\\', '\0'])
                && p.as_bytes().get(1) != Some(&b':')
                && p.split('/').all(|s| !s.is_empty() && s != "." && s != ".."),
            "rule path",
        )?;
        ensure(rule["precedence"] == json!(i), "rule precedence")?;
        ensure(rule["authority"] == "binding", "rule authority")?;
        ensure(
            locators.insert((s(&rule["provenance"]["sourceIdentity"])?, p)),
            "rule locator",
        )?;
    }
    let mut skill_map = BTreeMap::new();
    for skill in skills {
        exact(skill,"id version content contentDigest requirement selectionReason requirementSources resourceIds provenance")?;
        let k = id(&skill["id"])?;
        ensure(skill_map.insert(k, skill).is_none(), "skill id")?;
        text(&skill["version"], 128, 1)?;
        tags(&skill["requirement"], &["mandatory", "optional"])?;
        text(&skill["selectionReason"], 2048, 1)?;
        let ids = list(&skill["resourceIds"], 32, 0)?;
        sorted(ids, None)?;
        for rid in ids {
            id(rid)?;
        }
    }
    let mut resource_map = BTreeMap::new();
    for resource in resources {
        exact(
            resource,
            "id skillId sourceReference content contentDigest provenance",
        )?;
        let k = id(&resource["id"])?;
        ensure(resource_map.insert(k, resource).is_none(), "resource id")?;
        id(&resource["skillId"])?;
        text(&resource["sourceReference"], 2048, 1)?;
        ensure(
            skill_map
                .get(s(&resource["skillId"])?)
                .is_some_and(|skill| {
                    skill["resourceIds"]
                        .as_array()
                        .is_some_and(|rs| rs.contains(&resource["id"]))
                }),
            "resource closure",
        )?;
    }
    for x in rules.iter().chain(skills).chain(resources) {
        content(x, "content", "contentDigest")?;
        provenance(&x["provenance"])?;
    }
    for skill in skills {
        let expected: Vec<_> = resources
            .iter()
            .filter(|r| r["skillId"] == skill["id"])
            .map(|r| r["id"].clone())
            .collect();
        ensure(skill["resourceIds"] == json!(expected), "resource closure")?;
    }
    let manifests = list(&b["requirementManifests"], 33, 0)?;
    ensure(
        manifests
            .iter()
            .map(|m| s(&m["sourceId"]))
            .collect::<Result<Vec<_>>>()?
            == source_map.keys().copied().collect::<Vec<_>>(),
        "manifest coverage",
    )?;
    let mut required = BTreeSet::new();
    for m in manifests {
        exact(
            m,
            "sourceId sourceDigest taskId requiredSkillIds sourceLocations applicabilityReason",
        )?;
        let source = source_map
            .get(s(&m["sourceId"])?)
            .ok_or(RootBootstrapError("manifest binding"))?;
        ensure(
            m["sourceDigest"] == source["contentDigest"] && m["taskId"] == *task,
            "manifest binding",
        )?;
        text(&m["applicabilityReason"], 2048, 1)?;
        let ids = list(&m["requiredSkillIds"], 16, 0)?;
        sorted(ids, None)?;
        for rid in ids {
            required.insert(id(rid)?);
        }
        let locs = list(&m["sourceLocations"], 32, 1)?;
        sorted(locs, None)?;
        for loc in locs {
            text(loc, 512, 1)?;
        }
    }
    let mandatory: BTreeSet<_> = skills
        .iter()
        .filter(|s| s["requirement"] == "mandatory")
        .map(|skill| s(&skill["id"]))
        .collect::<Result<_>>()?;
    ensure(mandatory == required, "mandatory skill union")?;
    for skill in skills {
        list(&skill["requirementSources"], 32, 0)?;
        let expected: Vec<_> = if skill["requirement"] == "mandatory" {
            manifests.iter().filter(|m|m["requiredSkillIds"].as_array().is_some_and(|ids|ids.contains(&skill["id"]))).map(|m|json!({"sourceId":m["sourceId"],"sourceDigest":m["sourceDigest"],"sourceLocations":m["sourceLocations"]})).collect()
        } else {
            vec![]
        };
        ensure(
            skill["requirementSources"] == json!(expected),
            "requirement sources",
        )?;
    }
    let rubric = &b["rubric"];
    exact(rubric, "id version dimensions evaluatorRole")?;
    id(&rubric["id"])?;
    number(&rubric["version"], 1, 2147483647)?;
    tags(&rubric["evaluatorRole"], ROLES)?;
    let mut ids = BTreeSet::new();
    for d in list(&rubric["dimensions"], 16, 1)? {
        exact(d, "id description")?;
        ensure(ids.insert(id(&d["id"])?), "dimension id")?;
        text(&d["description"], 2048, 1)?;
    }
    Ok(())
}
pub(in crate::taskmaster) fn bindings(x: &Value) -> Result<()> {
    let xs = list(x, 32, 0)?;
    sorted(xs, Some("name"))?;
    for b in xs {
        exact(b, "name value")?;
        let name = text(&b["name"], 128, 1)?;
        ensure(
            name.bytes().enumerate().all(|(i, b)| {
                b.is_ascii_alphabetic()
                    || b == b'_'
                    || (i > 0 && (b.is_ascii_digit() || b".-".contains(&b)))
            }),
            "binding name",
        )?;
        text(&b["value"], 2048, 0)?;
    }
    Ok(())
}
pub(super) fn policy(x: &Value) -> Result<()> {
    exact(x,"schemaVersion kind harnessId adapterId adapterVersion policyId mode modelId settings unknownModelAllowed")?;
    ensure(
        x["schemaVersion"] == 1 && x["kind"] == "root-model-policy",
        "model policy version",
    )?;
    for k in ["harnessId", "adapterId", "policyId"] {
        id(&x[k])?;
    }
    text(&x["adapterVersion"], 512, 1)?;
    tags(&x["mode"], &["pinned", "tool-managed"])?;
    ensure(
        x["unknownModelAllowed"].is_boolean(),
        "model visibility policy",
    )?;
    if x["mode"] == "pinned" {
        text(&x["modelId"], 512, 1)?;
        ensure(x["unknownModelAllowed"] == false, "pinned policy")?;
    } else {
        ensure(x["modelId"].is_null(), "tool managed model")?;
    }
    bindings(&x["settings"])?;
    ensure(canonical(x)?.len() <= 65536, "policy bytes")
}
pub(super) fn projection(x: &Value) -> Result<()> {
    exact(
        x,
        "schemaVersion kind sourceIdentity sourceRevision bindings",
    )?;
    ensure(
        x["schemaVersion"] == 1 && x["kind"] == "root-effective-configuration",
        "projection version",
    )?;
    text(&x["sourceIdentity"], 512, 1)?;
    text(&x["sourceRevision"], 512, 1)?;
    bindings(&x["bindings"])?;
    ensure(canonical(x)?.len() <= 65536, "projection bytes")
}
pub(super) fn evidence(x: &Value, b: &Value) -> Result<()> {
    unverified_evidence_for_role(x, b, "orchestrator")
}
pub(in crate::taskmaster) fn unverified_evidence_for_role(
    x: &Value,
    b: &Value,
    role: &str,
) -> Result<()> {
    ensure(canonical(x)?.len() <= 65536, "evidence bytes")?;
    exact(x,"schemaVersion evidenceId harnessId definitionDigest adapterId adapterVersion executableIdentity toolVersion platform instructionMechanism effectiveSettingsDigest role requestedProfileDigest effectiveProfileDigest decision checks references assessedAt assessorIdentity evidenceDigest")?;
    ensure(
        x["schemaVersion"] == 1 && x["role"] == role,
        "evidence version role",
    )?;
    for k in ["evidenceId", "harnessId", "adapterId"] {
        id(&x[k])?;
    }
    for k in [
        "definitionDigest",
        "effectiveSettingsDigest",
        "requestedProfileDigest",
        "effectiveProfileDigest",
    ] {
        digest(&x[k])?;
    }
    for k in ["adapterVersion", "toolVersion", "assessorIdentity"] {
        text(&x[k], 512, 1)?;
    }
    tags(
        &x["instructionMechanism"],
        &["native-agent", "startup-context"],
    )?;
    let p = &x["platform"];
    exact(p, "os architecture osVersion")?;
    tags(&p["os"], &["macos", "linux", "windows"])?;
    tags(&p["architecture"], &["arm64", "x64"])?;
    text(&p["osVersion"], 512, 1)?;
    let stamp = s(&x["assessedAt"])?;
    ensure(
        stamp.len() == 20
            && stamp.is_ascii()
            && !stamp.starts_with("0000")
            && stamp.as_bytes()[17..19].iter().all(u8::is_ascii_digit)
            && &stamp[17..19] <= "59"
            && chrono::NaiveDateTime::parse_from_str(stamp, "%Y-%m-%dT%H:%M:%SZ")
                .is_ok_and(|d| d.format("%Y-%m-%dT%H:%M:%SZ").to_string() == stamp),
        "UTC timestamp",
    )?;
    executable(&x["executableIdentity"])?;
    ensure(x["decision"] == "unverified", "unverified slice")?;
    let refs = list(&x["references"], 16, 1)?;
    sorted(refs, Some("id"))?;
    for r in refs {
        exact(r, "id kind sourceReference contentDigest")?;
        id(&r["id"])?;
        tags(&r["kind"], &["primary-reference", "fixture"])?;
        text(&r["sourceReference"], 2048, 1)?;
        digest(&r["contentDigest"])?;
    }
    let checks = list(&x["checks"], 10, 10)?;
    let surfaces = [
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
    ensure(
        checks
            .iter()
            .map(|c| s(&c["surface"]))
            .collect::<Result<Vec<_>>>()?
            == surfaces,
        "ten evidence surfaces",
    )?;
    let mut unverified = false;
    for c in checks {
        exact(c, "surface result reason referenceIds")?;
        tags(
            &c["result"],
            &["passed", "failed", "unverified", "not-applicable"],
        )?;
        text(&c["reason"], 2048, 1)?;
        let ids = list(&c["referenceIds"], 16, 1)?;
        sorted(ids, None)?;
        let mut kinds = BTreeSet::new();
        for rid in ids {
            id(rid)?;
            let r = refs
                .iter()
                .find(|r| r["id"] == *rid)
                .ok_or(RootBootstrapError("evidence reference"))?;
            kinds.insert(s(&r["kind"])?);
        }
        if c["result"] == "passed" || c["result"] == "not-applicable" {
            ensure(
                kinds.contains("primary-reference") && kinds.contains("fixture"),
                "source and fixture evidence",
            )?;
        }
        unverified |= c["result"] == "unverified";
    }
    ensure(unverified, "unverified check required")?;
    ensure(
        x["requestedProfileDigest"] == raw(&b["permissions"]["requested"])?
            && x["effectiveProfileDigest"] == raw(&b["permissions"]["effective"])?,
        "profile digest",
    )?;
    sealed(x, "evidenceDigest", "orkworks.capability-evidence.v1")
}
