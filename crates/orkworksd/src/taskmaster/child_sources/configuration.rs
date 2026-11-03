use super::{wire::*, ByteError, ByteResult, ParsedBundle};
use crate::taskmaster::root_bootstrap::source;
use base64::{engine::general_purpose::STANDARD, Engine};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

pub(super) struct Context {
    pub task: Value,
    pub settings: Value,
    pub retained: BTreeMap<String, Vec<u8>>,
}
fn profile(x: &Value, review: bool) -> ByteResult<()> {
    exact(x,"tools readPaths writePaths commands externalSources connectors coordinationActions nativeDelegation accessChanges")?;
    let mut tools = vec![
        "orkworks.result.receipt.read.v1",
        "orkworks.result.submit.v1",
    ];
    if review {
        tools.extend([
            "orkworks.evaluation.artifact.read.v1",
            "orkworks.evaluation.receipt.read.v1",
            "orkworks.evaluation.submit.v1",
        ]);
        tools.sort();
    }
    same(&x["tools"], &json!(tools), "child tools")?;
    for k in [
        "readPaths",
        "writePaths",
        "commands",
        "externalSources",
        "connectors",
        "coordinationActions",
    ] {
        same(&x[k], &json!([]), "child access")?;
    }
    ensure(
        x["nativeDelegation"] == "denied" && x["accessChanges"] == "request-only",
        "child access",
    )
}
fn assignment(x: &Value, review: bool) -> ByteResult<()> {
    exact(x,"schemaVersion label description inputs expectedOutput criteria dependencies worktreeScope worktreeGroupId baseCommit")?;
    ensure(x["schemaVersion"] == 1, "assignment version")?;
    text(&x["label"], 512, 1)?;
    text(&x["description"], 4096, 1)?;
    text(&x["expectedOutput"], 2048, 1)?;
    same(&x["dependencies"], &json!([]), "child dependencies")?;
    same(
        &x["worktreeScope"],
        &json!({"readPaths":[],"writePaths":[]}),
        "child scope",
    )?;
    id(&x["worktreeGroupId"])?;
    let oid = s(&x["baseCommit"])?;
    ensure(
        [40, 64].contains(&oid.len())
            && oid
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
        "Git object syntax",
    )?;
    let inputs = list(&x["inputs"], 32, 0)?;
    let mut ids = BTreeSet::new();
    for v in inputs {
        exact(v, "kind id content contentDigest")?;
        ensure(
            v["kind"] == "inline" && ids.insert(id(&v["id"])?),
            "inline input",
        )?;
        let content = text(&v["content"], 4096, 1)?;
        ensure(
            hash(content) == digest(&v["contentDigest"])?,
            "inline digest",
        )?;
    }
    ensure(!review || inputs.is_empty(), "review inline inputs")?;
    criteria(&x["criteria"], false)
}
fn review_input(x: &Value, snapshot: Option<&Value>) -> ByteResult<()> {
    let Some(snapshot) = snapshot else {
        return ensure(x.is_null(), "worker review input");
    };
    exact(x,"schemaVersion kind target outputDeclarationIds evidenceSources evaluationOutputDeclarationId maxReadBytes")?;
    ensure(
        x["schemaVersion"] == 1 && x["kind"] == "accepted-child-result",
        "review input",
    )?;
    same(&x["target"], &snapshot["target"], "review target")?;
    same(
        &x["outputDeclarationIds"],
        &json!([]),
        "review declarations",
    )?;
    let evidence = list(&snapshot["evidence"], 113, 0)?;
    same(
        &x["evidenceSources"],
        &Value::Array(evidence.iter().map(|p| p["source"].clone()).collect()),
        "review evidence",
    )?;
    id(&x["evaluationOutputDeclarationId"])?;
    let max = number(&x["maxReadBytes"], 1, 262144)? as usize;
    let mut total = 0;
    for k in ["result", "criteria", "rubric"] {
        total = add(
            total,
            canonical(&snapshot[k])?.len(),
            max,
            "review read capacity",
        )?;
    }
    for pair in evidence {
        total = add(
            total,
            canonical(&pair["report"])?.len(),
            max,
            "review read capacity",
        )?;
    }
    Ok(())
}
fn output(x: &Value, review: bool, input: &Value) -> ByteResult<()> {
    exact(
        x,
        "schemaVersion declarations maxSnapshotBytes maxRehashBytes",
    )?;
    ensure(x["schemaVersion"] == 1, "output version")?;
    let max = number(&x["maxSnapshotBytes"], 0, 16777216)?;
    let rehash = number(&x["maxRehashBytes"], 0, 67108864)?;
    if !review {
        same(&x["declarations"], &json!([]), "worker declarations")?;
        return ensure(max == 0 && rehash == 0, "worker capacity");
    }
    let d = &list(&x["declarations"], 1, 1)?[0];
    exact(d, "id kind maxBytes source")?;
    id(&d["id"])?;
    ensure(
        d["id"] == input["evaluationOutputDeclarationId"] && d["kind"] == "assignment_evaluation",
        "review declaration",
    )?;
    same(
        &d["source"],
        &json!({"kind":"server-artifact"}),
        "review artifact",
    )?;
    let n = number(&d["maxBytes"], 1, 524288)?;
    ensure(
        n <= max && rehash >= max.checked_mul(2).ok_or(ByteError("output capacity"))?,
        "output capacity",
    )
}
fn artifact(
    map: &BTreeMap<String, Vec<u8>>,
    d: &Value,
    refs: &Value,
    fixture: bool,
) -> ByteResult<Value> {
    let key = digest(d)?;
    let refs = list(refs, 16, 1)?;
    ensure(
        refs.iter()
            .any(|r| r["contentDigest"] == *d && (!fixture || r["kind"] == "fixture")),
        "artifact citation",
    )?;
    decode(map.get(key).ok_or(ByteError("retained artifact"))?, 65536)
}
fn lookup<'a>(c: &'a Value, kind: &Value, identity: &Value) -> ByteResult<&'a Value> {
    let values: Vec<&Value> = match s(kind)? {
        "rule" => list(&c["rules"], 32, 0)?.iter().collect(),
        "role-template" => vec![&c["roleTemplate"]],
        "skill" => list(&c["skills"], 16, 0)?.iter().collect(),
        "skill-resource" => list(&c["skillResources"], 32, 0)?.iter().collect(),
        _ => return Err(ByteError("snapshot kind")),
    };
    values
        .into_iter()
        .find(|x| x["id"] == *identity)
        .ok_or(ByteError("automatic snapshot"))
}
fn settings(
    x: &Value,
    c: &Value,
    map: &BTreeMap<String, Vec<u8>>,
    snapshot: Option<&Value>,
) -> ByteResult<()> {
    exact(x,"schemaVersion kind harnessId adapterId adapterVersion argv nonsecretBindings configurationSources discoveredInstructions deliveryOrder instructionMechanism")?;
    ensure(
        x["schemaVersion"] == 1
            && x["kind"] == "child-startup-settings"
            && x["instructionMechanism"] == "startup-context",
        "startup settings",
    )?;
    for k in [
        "harnessId",
        "adapterId",
        "adapterVersion",
        "instructionMechanism",
    ] {
        same(&x[k], &c["harness"][k], "settings adapter")?;
    }
    for arg in list(&x["argv"], 64, 0)? {
        text(arg, 2048, 0)?;
    }
    source::bindings(&x["nonsecretBindings"])?;
    let sources = list(&x["configurationSources"], 32, 0)?;
    sorted(sources, Some("sourceIdentity"))?;
    for v in sources {
        exact(v, "sourceIdentity sourceRevision contentDigest")?;
        text(&v["sourceIdentity"], 512, 1)?;
        text(&v["sourceRevision"], 512, 1)?;
        let p = artifact(
            map,
            &v["contentDigest"],
            &c["capabilityEvidence"]["references"],
            false,
        )?;
        exact(
            &p,
            "schemaVersion kind sourceIdentity sourceRevision bindings",
        )?;
        ensure(
            p["schemaVersion"] == 1 && p["kind"] == "child-effective-configuration",
            "configuration projection",
        )?;
        same(
            &p["sourceIdentity"],
            &v["sourceIdentity"],
            "projection identity",
        )?;
        same(
            &p["sourceRevision"],
            &v["sourceRevision"],
            "projection revision",
        )?;
        source::bindings(&p["bindings"])?;
        size(&p, 65536, "projection size")?;
    }
    let discovered = list(&x["discoveredInstructions"], 32, 0)?;
    let mut seen = BTreeSet::new();
    for (i, d) in discovered.iter().enumerate() {
        exact(d, "snapshotKind snapshotId contentDigest precedence")?;
        ensure(
            number(&d["precedence"], 0, 31)? as usize == i,
            "discovery precedence",
        )?;
        id(&d["snapshotId"])?;
        digest(&d["contentDigest"])?;
        let v = lookup(c, &d["snapshotKind"], &d["snapshotId"])?;
        same(&v["contentDigest"], &d["contentDigest"], "discovery digest")?;
        ensure(
            seen.insert((s(&d["snapshotKind"])?, s(&d["snapshotId"])?)),
            "duplicate discovery",
        )?;
    }
    let entries = list(
        &x["deliveryOrder"],
        if snapshot.is_some() { 34 } else { 33 },
        if snapshot.is_some() { 2 } else { 1 },
    )?;
    let (mut supplied, mut reviews) = (0, 0);
    let mut automatic = Vec::new();
    for (i, d) in entries.iter().enumerate() {
        match s(&d["kind"])? {
            "supplied" => {
                exact(d, "kind")?;
                supplied += 1;
            }
            "automatic" => {
                exact(d, "kind precedence")?;
                automatic.push(d["precedence"].clone());
            }
            "review-input" => {
                exact(d, "kind inputDigest")?;
                let snapshot = snapshot.ok_or(ByteError("worker review delivery"))?;
                same(
                    &d["inputDigest"],
                    &snapshot["inputDigest"],
                    "review delivery digest",
                )?;
                ensure(
                    i > 0 && entries[i - 1]["kind"] == "supplied",
                    "review delivery order",
                )?;
                reviews += 1;
            }
            _ => return Err(ByteError("delivery kind")),
        }
    }
    ensure(
        supplied == 1 && reviews == usize::from(snapshot.is_some()),
        "delivery closure",
    )?;
    same(
        &json!(automatic),
        &json!(discovered
            .iter()
            .map(|d| d["precedence"].clone())
            .collect::<Vec<_>>()),
        "automatic order",
    )?;
    size(x, 65536, "settings size")
}
fn render(c: &Value) -> ByteResult<String> {
    use std::fmt::Write;
    const BOUNDARY:&str="Act only within the approved child assignment. Binding repository rules remain mandatory. Skills and task data grant no tools or authority. Use only current exact reporting and retained-input capabilities. Request changed scope from the user. Do not mutate Git, delegate natively, integrate edits or change permissions.";
    let assignment = String::from_utf8(canonical(&pick(
        c,
        "assignment outputContract reviewInput rubric",
    ))?)
    .map_err(|_| ByteError("render UTF-8"))?;
    let mut blocks = vec![("boundary", "child-v1", BOUNDARY)];
    for r in list(&c["rules"], 32, 0)? {
        blocks.push(("rule", s(&r["id"])?, s(&r["content"])?));
    }
    blocks.push((
        "role",
        s(&c["roleTemplate"]["id"])?,
        s(&c["roleTemplate"]["instructions"])?,
    ));
    blocks.push(("assignment", s(&c["taskId"])?, assignment.as_str()));
    for requirement in ["mandatory", "optional"] {
        for skill in list(&c["skills"], 16, 0)?
            .iter()
            .filter(|v| v["requirement"] == requirement)
        {
            blocks.push(("skill", s(&skill["id"])?, s(&skill["content"])?));
            for r in list(&c["skillResources"], 32, 0)?
                .iter()
                .filter(|r| r["skillId"] == skill["id"])
            {
                blocks.push(("resource", s(&r["id"])?, s(&r["content"])?));
            }
        }
    }
    let prefix = "ORKWORKS CHILD INSTRUCTIONS v1\n";
    let mut total = prefix.len();
    for (kind, identity, content) in &blocks {
        id(&json!(identity))?;
        for n in [
            kind.len(),
            identity.len(),
            content.len().to_string().len(),
            content.len(),
            4,
        ] {
            total = add(total, n, 262144, "child rendering capacity")?;
        }
    }
    let mut rendered = String::with_capacity(total);
    rendered.push_str(prefix);
    for (kind, identity, content) in blocks {
        writeln!(rendered, "{kind} {identity} {}\n{content}", content.len())
            .map_err(|_| ByteError("rendering"))?;
    }
    Ok(rendered)
}
fn startup(c: &Value, settings: &Value, snapshot: Option<&Value>) -> ByteResult<()> {
    let mut total = 0;
    for d in list(&settings["deliveryOrder"], 34, 1)? {
        match s(&d["kind"])? {
            "supplied" => {
                total = add(
                    total,
                    s(&c["renderedInstructions"])?.len(),
                    262144,
                    "startup capacity",
                )?;
            }
            "review-input" => {
                let bytes = canonical(snapshot.ok_or(ByteError("review snapshot"))?)?;
                for n in [
                    "\nORKWORKS RETAINED REVIEW INPUT v1\n".len(),
                    bytes.len().to_string().len(),
                    1,
                    bytes.len(),
                    1,
                ] {
                    total = add(total, n, 262144, "startup capacity")?;
                }
            }
            "automatic" => {
                let i = number(&d["precedence"], 0, 31)? as usize;
                let v = list(&settings["discoveredInstructions"], 32, 0)?
                    .get(i)
                    .ok_or(ByteError("discovery index"))?;
                let v = lookup(c, &v["snapshotKind"], &v["snapshotId"])?;
                let content = if v.get("content").is_some() {
                    &v["content"]
                } else {
                    &v["instructions"]
                };
                total = add(total, s(content)?.len(), 262144, "startup capacity")?;
            }
            _ => return Err(ByteError("delivery kind")),
        }
    }
    Ok(())
}
pub(super) fn validate(
    owner: &ParsedBundle,
    run: &Value,
    snapshot: Option<&Value>,
) -> ByteResult<Context> {
    let c = &owner.configuration;
    let review = snapshot.is_some();
    exact(c,"schemaVersion configurationId repositoryId repositoryBinding sourceWorktreeBinding workspaceId parentSessionId planId planRevision taskId assignmentKind roleTemplate taskCategory assignment rules requirementManifests skills skillResources rubric harness capabilityEvidence model permissions renderedInstructions renderedInstructionsDigest configurationDigest outputContract reviewInput")?;
    ensure(
        c["schemaVersion"] == 3 && c["assignmentKind"] == "child",
        "child configuration",
    )?;
    for k in [
        "configurationId",
        "repositoryId",
        "workspaceId",
        "parentSessionId",
        "planId",
        "taskId",
    ] {
        id(&c[k])?;
    }
    number(&c["planRevision"], 1, 64)?;
    text(&c["taskCategory"], 512, 1)?;
    for k in ["workspaceId", "parentSessionId"] {
        same(&c[k], &run[k], "run owner")?;
    }
    source::repository(&c["repositoryBinding"])?;
    source::worktree(&c["sourceWorktreeBinding"], &c["repositoryBinding"])?;
    same(
        &c["repositoryId"],
        &c["repositoryBinding"]["repositoryId"],
        "repository id",
    )?;
    same(
        &c["repositoryBinding"]["bindingDigest"],
        &run["repositoryBindingDigest"],
        "repository binding",
    )?;
    same(
        &c["sourceWorktreeBinding"]["worktreeBindingDigest"],
        &run["sourceWorktreeBindingDigest"],
        "source worktree",
    )?;
    let role = s(&c["roleTemplate"]["role"])?;
    ensure(
        if review {
            ["review", "verification"].contains(&role)
        } else {
            role == "verification"
        },
        "child role",
    )?;
    if let Some(snapshot) = snapshot {
        same(
            &c["roleTemplate"]["role"],
            &snapshot["rubric"]["evaluatorRole"],
            "target rubric role",
        )?;
    }
    assignment(&c["assignment"], review)?;
    review_input(&c["reviewInput"], snapshot)?;
    output(&c["outputContract"], review, &c["reviewInput"])?;
    exact(&c["permissions"], "requested effective")?;
    profile(&c["permissions"]["requested"], review)?;
    profile(&c["permissions"]["effective"], review)?;
    source::snapshots_for_assignment(c, role, &c["taskId"])?;
    source::unverified_evidence_for_role(&c["capabilityEvidence"], c, role)?;
    let h = &c["harness"];
    let e = &c["capabilityEvidence"];
    exact(h,"harnessId definitionDigest adapterId adapterVersion executableIdentity toolVersion platform instructionMechanism effectiveSettingsDigest evidenceId evidenceDigest")?;
    for k in h.as_object().ok_or(ByteError("harness"))?.keys() {
        same(&h[k], &e[k], "harness evidence binding")?;
    }
    source::executable(&h["executableIdentity"])?;
    let t = task(c, run);
    let refs = list(&e["references"], 16, 1)?;
    ensure(owner.evidence.len() == refs.len(), "evidence closure")?;
    let mut retained = BTreeMap::new();
    let mut total = 0;
    for (v, r) in owner.evidence.iter().zip(refs) {
        exact(
            v,
            "schemaVersion task referenceId contentBase64 sizeBytes contentDigest",
        )?;
        ensure(v["schemaVersion"] == 1, "evidence version")?;
        same(&v["task"], &t, "evidence task")?;
        id(&v["referenceId"])?;
        same(&v["referenceId"], &r["id"], "evidence reference")?;
        let encoded = text(&v["contentBase64"], 87384, 0)?;
        let bytes = STANDARD
            .decode(encoded)
            .map_err(|_| ByteError("evidence base64"))?;
        ensure(STANDARD.encode(&bytes) == encoded, "evidence base64")?;
        ensure(
            number(&v["sizeBytes"], 0, 65536)? as usize == bytes.len(),
            "evidence size",
        )?;
        ensure(
            hash(&bytes) == digest(&v["contentDigest"])?,
            "evidence digest",
        )?;
        same(&v["contentDigest"], &r["contentDigest"], "cited digest")?;
        std::str::from_utf8(&bytes).map_err(|_| ByteError("evidence UTF-8"))?;
        total = add(total, bytes.len(), 1048576, "evidence aggregate")?;
        retained.insert(s(&v["contentDigest"])?.into(), bytes);
    }
    let settings_value = artifact(
        &retained,
        &h["effectiveSettingsDigest"],
        &e["references"],
        true,
    )?;
    settings(&settings_value, c, &retained, snapshot)?;
    let model = &c["model"];
    exact(
        model,
        "schemaVersion mode modelId policyId policyDigest adapterGeneration",
    )?;
    ensure(model["schemaVersion"] == 1, "model version")?;
    id(&model["adapterGeneration"])?;
    let p = artifact(&retained, &model["policyDigest"], &e["references"], true)?;
    exact(&p,"schemaVersion kind harnessId adapterId adapterVersion policyId mode modelId settings unknownModelAllowed")?;
    ensure(
        p["schemaVersion"] == 1
            && p["kind"] == "child-model-policy"
            && p["unknownModelAllowed"].is_boolean(),
        "model policy",
    )?;
    for k in ["harnessId", "adapterId", "policyId"] {
        id(&p[k])?;
    }
    text(&p["adapterVersion"], 512, 1)?;
    if p["mode"] == "pinned" {
        text(&p["modelId"], 512, 1)?;
        ensure(p["unknownModelAllowed"] == false, "pinned model")?;
    } else {
        ensure(
            p["mode"] == "tool-managed" && p["modelId"].is_null(),
            "model mode",
        )?;
    }
    source::bindings(&p["settings"])?;
    size(&p, 65536, "policy size")?;
    for k in ["mode", "modelId", "policyId"] {
        same(&model[k], &p[k], "model policy binding")?;
    }
    for k in ["harnessId", "adapterId", "adapterVersion"] {
        same(&p[k], &h[k], "policy adapter")?;
    }
    ensure(
        s(&c["renderedInstructions"])? == render(c)?,
        "rendered child bytes",
    )?;
    ensure(
        hash(s(&c["renderedInstructions"])?) == digest(&c["renderedInstructionsDigest"])?,
        "rendered digest",
    )?;
    startup(c, &settings_value, snapshot)?;
    sealed(
        c,
        "configurationDigest",
        "orkworks.assignment-configuration.v3",
    )?;
    size(c, 1048576, "configuration size")?;
    Ok(Context {
        task: t,
        settings: settings_value,
        retained,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn configuration() -> Value {
        let p: Value = serde_json::from_str(include_str!(
            "../../../../../docs/validation/child-source-vectors.json"
        ))
        .unwrap();
        p["worker"]["configuration"].clone()
    }
    #[test]
    fn child_sources_render_and_composed_startup_count_complete_frames() {
        let mut c = configuration();
        let original = s(&c["rules"][0]["content"]).unwrap().len();
        let mut padding = 262144 - render(&c).unwrap().len();
        padding -= (original + padding).to_string().len() - original.to_string().len();
        let content = format!(
            "{}{}",
            s(&c["rules"][0]["content"]).unwrap(),
            "x".repeat(padding)
        );
        c["rules"][0]["content"] = json!(content);
        assert_eq!(render(&c).unwrap().len(), 262144);
        c["rules"][0]["content"] = json!(format!("{content}x"));
        assert!(render(&c).is_err());
        let mut c = configuration();
        c["renderedInstructions"] = json!("x".repeat(262144));
        let settings = json!({"deliveryOrder":[{"kind":"supplied"}]});
        assert!(startup(&c, &settings, None).is_ok());
        let settings = json!({"deliveryOrder":[{"kind":"supplied"},{"kind":"automatic","precedence":0}],"discoveredInstructions":[{"snapshotKind":"rule","snapshotId":c["rules"][0]["id"]}]});
        assert!(startup(&c, &settings, None).is_err());
        let snapshot = json!({"data":"é\u{0}\r\n"});
        let bytes = canonical(&snapshot).unwrap();
        let frame = "\nORKWORKS RETAINED REVIEW INPUT v1\n".len()
            + bytes.len().to_string().len()
            + 1
            + bytes.len()
            + 1;
        c["renderedInstructions"] = json!("x".repeat(262144 - frame));
        let settings = json!({"deliveryOrder":[{"kind":"supplied"},{"kind":"review-input"}]});
        assert!(startup(&c, &settings, Some(&snapshot)).is_ok());
        c["renderedInstructions"] = json!("x".repeat(262145 - frame));
        assert!(startup(&c, &settings, Some(&snapshot)).is_err());
    }
}
