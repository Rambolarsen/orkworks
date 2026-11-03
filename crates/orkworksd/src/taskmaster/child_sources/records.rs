use super::{configuration::Context, reports, wire::*, ByteError, ByteResult, ParsedBundle};
use crate::taskmaster::root_bootstrap::source;
use serde_json::{json, Value};

fn branch(x: &Value) -> ByteResult<()> {
    let x = text(x, 256, 1)?;
    ensure(
        !["main", "HEAD", "@"].contains(&x)
            && !x.starts_with('-')
            && !x
                .bytes()
                .any(|b| b <= 32 || b == 127 || b"~^:?*[\\".contains(&b))
            && !x.contains("..")
            && !x.contains("@{"),
        "Git branch syntax",
    )?;
    ensure(
        x.split('/').all(|p| {
            !p.is_empty() && !p.starts_with('.') && !p.ends_with('.') && !p.ends_with(".lock")
        }),
        "Git branch components",
    )
}
pub(super) fn bundle(
    o: &ParsedBundle,
    run: &Value,
    initial: &Value,
    snapshot: Option<&Value>,
    ctx: &Context,
) -> ByteResult<()> {
    let c = &o.configuration;
    let t = &ctx.task;
    let p = &o.plan;
    exact(p,"schemaVersion kind parentPlanBinding task batches maxParallelChildren scopeText planDefinitionDigest")?;
    ensure(
        p["schemaVersion"] == 1 && p["kind"] == "single-task-child-source",
        "child plan",
    )?;
    let b = &p["parentPlanBinding"];
    exact(b,"runId parentSessionId planId planRevision runDefinitionDigest bootstrapConfigurationDigest repositoryBindingDigest sourceWorktreeBindingDigest preparationRevision inputDigest")?;
    for k in [
        "runId",
        "parentSessionId",
        "runDefinitionDigest",
        "bootstrapConfigurationDigest",
        "repositoryBindingDigest",
        "sourceWorktreeBindingDigest",
    ] {
        same(&b[k], &run[k], "parent run binding")?;
    }
    for k in ["planId", "planRevision"] {
        same(&b[k], &c[k], "parent plan binding")?;
    }
    number(&b["preparationRevision"], 1, 64)?;
    same(
        &b["preparationRevision"],
        &initial["preparationRevision"],
        "preparation revision",
    )?;
    ensure(
        s(&b["inputDigest"])? == domain("orkworks.preparation-input.v1", initial)?,
        "parent input digest",
    )?;
    let task = &p["task"];
    exact(task, "taskId configuration input initialPrompt group")?;
    same(&task["taskId"], &c["taskId"], "plan task")?;
    let cfg = pick(c, "configurationId configurationDigest");
    same(&task["configuration"], &cfg, "plan configuration")?;
    let input = snapshot.map_or(
        Value::Null,
        |s| json!({"task":t,"inputDigest":s["inputDigest"]}),
    );
    same(&task["input"], &input, "plan review input")?;
    same(
        &task["initialPrompt"],
        &c["assignment"]["description"],
        "plan prompt",
    )?;
    let g = &task["group"];
    exact(g, "groupId branchName absolutePath baseCommit")?;
    same(
        &g["groupId"],
        &c["assignment"]["worktreeGroupId"],
        "group id",
    )?;
    same(
        &g["baseCommit"],
        &c["assignment"]["baseCommit"],
        "group base",
    )?;
    branch(&g["branchName"])?;
    source::native_path(&g["absolutePath"], 4096)?;
    same(&p["batches"], &json!([[c["taskId"]]]), "single-task batch")?;
    ensure(p["maxParallelChildren"] == 1, "single child")?;
    text(&p["scopeText"], 4096, 1)?;
    sealed(p, "planDefinitionDigest", "orkworks.child-source-plan.v1")?;
    size(p, 65536, "plan size")?;
    let mut total = add(
        canonical(p)?.len(),
        canonical(c)?.len(),
        2097152,
        "complete plan capacity",
    )?;
    if let Some(snapshot) = snapshot {
        total = add(
            total,
            canonical(snapshot)?.len(),
            2097152,
            "complete plan capacity",
        )?;
    }
    let mut refs = vec![
        &c["harness"]["effectiveSettingsDigest"],
        &c["capabilityEvidence"]["effectiveSettingsDigest"],
        &c["model"]["policyDigest"],
    ];
    refs.extend(
        list(&c["capabilityEvidence"]["references"], 16, 1)?
            .iter()
            .map(|r| &r["contentDigest"]),
    );
    refs.extend(
        list(&ctx.settings["configurationSources"], 32, 0)?
            .iter()
            .map(|r| &r["contentDigest"]),
    );
    for d in refs {
        total = add(
            total,
            ctx.retained
                .get(s(d)?)
                .ok_or(ByteError("retained artifact"))?
                .len(),
            2097152,
            "complete plan capacity",
        )?;
    }
    let plan = plan_ref(t, p);
    let request = &o.request;
    exact(
        request,
        "schemaVersion action plan approvalId reviewContextId",
    )?;
    ensure(
        request["schemaVersion"] == 1
            && request["action"] == "orkworks.child-source-plan.approve.v1",
        "approval request",
    )?;
    same(&request["plan"], &plan, "request plan")?;
    id(&request["approvalId"])?;
    digest(&request["reviewContextId"])?;
    let approval = &o.approval;
    exact(
        approval,
        "schemaVersion plan approvalId reviewContextId sidecarGeneration requestDigest",
    )?;
    ensure(approval["schemaVersion"] == 1, "approval version")?;
    same(&approval["plan"], &plan, "approval plan")?;
    for k in ["approvalId", "reviewContextId"] {
        same(&approval[k], &request[k], "approval request join")?;
    }
    digest(&approval["sidecarGeneration"])?;
    ensure(
        s(&approval["requestDigest"])? == domain("orkworks.child-source-plan-request.v1", request)?,
        "request digest",
    )?;
    let approval_digest = domain("orkworks.child-source-plan-approval.v1", approval)?;
    let approval_ref =
        json!({"plan":plan,"approvalId":approval["approvalId"],"approvalDigest":approval_digest});
    let receipt = &o.receipt;
    exact(
        receipt,
        "schemaVersion plan approvalId approvalDigest requestDigest commit",
    )?;
    same(
        &pick(
            receipt,
            "schemaVersion plan approvalId approvalDigest requestDigest",
        ),
        &json!({"schemaVersion":1,"plan":plan,"approvalId":approval["approvalId"],"approvalDigest":approval_digest,"requestDigest":approval["requestDigest"]}),
        "approval receipt",
    )?;
    commit(&receipt["commit"])?;
    size(receipt, 4096, "receipt size")?;
    let r = &o.reservation;
    exact(r,"schemaVersion task reservationId childSessionId configurationId configurationDigest approval assignmentTaskVersion reservationDigest")?;
    ensure(r["schemaVersion"] == 1, "reservation version")?;
    same(&r["task"], t, "reservation task")?;
    id(&r["reservationId"])?;
    id(&r["childSessionId"])?;
    same(
        &pick(r, "configurationId configurationDigest"),
        &cfg,
        "reservation configuration",
    )?;
    same(&r["approval"], &approval_ref, "reservation approval")?;
    number(&r["assignmentTaskVersion"], 1, 2147483647)?;
    sealed(
        r,
        "reservationDigest",
        "orkworks.child-source-reservation.v1",
    )?;
    let reservation = reservation_ref(t, r);
    let a = &o.allocation;
    exact(
        a,
        "schemaVersion reservation allocationId group allocatedWorktree allocationDigest",
    )?;
    ensure(a["schemaVersion"] == 1, "allocation version")?;
    same(&a["reservation"], &reservation, "allocation reservation")?;
    id(&a["allocationId"])?;
    same(&a["group"], g, "allocation group")?;
    source::worktree(&a["allocatedWorktree"], &c["repositoryBinding"])?;
    let worktree = &a["allocatedWorktree"];
    same(
        &worktree["canonicalRoot"],
        &g["absolutePath"],
        "allocated path",
    )?;
    ensure(worktree["kind"] == "linked", "allocated kind")?;
    for k in ["worktreeBindingDigest", "rootObject"] {
        ensure(
            worktree[k] != c["sourceWorktreeBinding"][k],
            "distinct allocation",
        )?;
    }
    sealed(a, "allocationDigest", "orkworks.child-source-allocation.v1")?;
    let allocation = json!({"reservation":reservation,"allocationId":a["allocationId"],"allocationDigest":a["allocationDigest"]});
    let mut assignment = t.clone();
    assignment["kind"] = json!("child");
    assignment["taskVersion"] = r["assignmentTaskVersion"].clone();
    for k in [
        "reservationId",
        "childSessionId",
        "configurationId",
        "configurationDigest",
    ] {
        assignment[k] = r[k].clone();
    }
    assignment["parentSessionId"] = c["parentSessionId"].clone();
    identity(&assignment)?;
    let attachment = &o.attachment;
    exact(
        attachment,
        "schemaVersion attachmentId reservation allocation assignmentIdentity attachmentDigest",
    )?;
    ensure(attachment["schemaVersion"] == 1, "attachment version")?;
    id(&attachment["attachmentId"])?;
    same(
        &attachment["reservation"],
        &reservation,
        "attachment reservation",
    )?;
    same(
        &attachment["allocation"],
        &allocation,
        "attachment allocation",
    )?;
    same(
        &attachment["assignmentIdentity"],
        &assignment,
        "frozen child identity",
    )?;
    sealed(
        attachment,
        "attachmentDigest",
        "orkworks.child-source-attachment.v1",
    )?;
    let mut projected = list(&c["assignment"]["criteria"], 32, 1)?.to_vec();
    projected.sort_by(|a, b| a["id"].as_str().cmp(&b["id"].as_str()));
    criteria(&o.source["criteria"], true)?;
    let mut configuration = cfg;
    configuration["task"] = t.clone();
    same(
        &o.source,
        &json!({"schemaVersion":1,"assignmentIdentity":assignment,"plan":plan,"configuration":configuration,"approval":approval_ref,"reservation":reservation,"allocation":allocation,"attachmentId":attachment["attachmentId"],"input":input,"criteria":projected,"rubric":c["rubric"]}),
        "complete source projection",
    )?;
    size(&o.source, 2097152, "source size")?;
    let rows = [
        json!({"key":{"plan":plan,"approvalId":approval["approvalId"]},"value":{"schemaVersion":1,"body":approval,"digest":approval_digest,"receipt":receipt,"lastCommit":receipt["commit"]}}),
        json!({"key":t,"value":{"schemaVersion":1,"body":r,"lastCommit":receipt["commit"]}}),
        json!({"key":reservation,"value":{"schemaVersion":1,"body":a,"lastCommit":receipt["commit"]}}),
        json!({"key":r["childSessionId"],"value":{"schemaVersion":1,"body":attachment,"lastCommit":receipt["commit"]}}),
    ];
    for row in rows {
        size(&row, 16384, "complete metadata wrapper")?;
    }
    Ok(())
}
fn result(x: &Value, worker: &ParsedBundle) -> ByteResult<String> {
    exact(x,"schemaVersion assignmentIdentity predecessor authorityGeneration outputs resultRevision candidateDigest timeCostEvidence")?;
    ensure(x["schemaVersion"] == 1, "result version")?;
    same(
        &x["assignmentIdentity"],
        &worker.source["assignmentIdentity"],
        "result identity",
    )?;
    identity(&x["assignmentIdentity"])?;
    let revision = number(&x["resultRevision"], 1, 32)?;
    digest(&x["candidateDigest"])?;
    same(&x["outputs"], &json!([]), "result outputs")?;
    let authority = &x["authorityGeneration"];
    exact(authority, "kind sidecarGeneration generation")?;
    ensure(authority["kind"] == "child-launch", "result authority kind")?;
    digest(&authority["sidecarGeneration"])?;
    digest(&authority["generation"])?;
    let prev = &x["predecessor"];
    if prev["kind"] == "no-head" {
        same(prev, &json!({"kind":"no-head"}), "no-head predecessor")?;
        ensure(revision == 1, "initial result")?;
    } else {
        exact(prev, "kind resultRevision resultDigest")?;
        ensure(
            prev["kind"] == "revision" && number(&prev["resultRevision"], 1, 31)? == revision - 1,
            "result predecessor",
        )?;
        digest(&prev["resultDigest"])?;
    }
    let mut previous = None;
    for e in list(&x["timeCostEvidence"], 32, 0)? {
        let key = format!(
            "{}\0{}",
            s(&e["kind"])?,
            e["usageRecordId"].as_str().unwrap_or("")
        );
        ensure(
            previous.as_ref().is_none_or(|p: &String| p < &key),
            "sorted time cost",
        )?;
        previous = Some(key);
        if e["kind"] == "elapsed" {
            exact(e, "kind elapsedMilliseconds authorityGeneration")?;
            number(&e["elapsedMilliseconds"], 0, 604800000)?;
            same(&e["authorityGeneration"], authority, "elapsed authority")?;
        } else {
            exact(
                e,
                "kind costMicroUsd providerId modelId pricingSnapshotId usageRecordId usageDigest",
            )?;
            ensure(e["kind"] == "cost", "cost kind")?;
            number(&e["costMicroUsd"], 0, 1000000000000)?;
            for k in [
                "providerId",
                "modelId",
                "pricingSnapshotId",
                "usageRecordId",
            ] {
                id(&e[k])?;
            }
            digest(&e["usageDigest"])?;
        }
        size(e, 512, "time cost size")?;
    }
    size(x, 524288, "result size")?;
    domain("orkworks.assignment-result.v1", x)
}
pub(super) fn snapshot(
    x: &Value,
    worker: &ParsedBundle,
    reviewer: &ParsedBundle,
    run: &Value,
    context: &Value,
) -> ByteResult<()> {
    exact(
        x,
        "schemaVersion task target result criteria rubric evidence inputDigest",
    )?;
    ensure(x["schemaVersion"] == 1, "snapshot version")?;
    same(
        &x["task"],
        &task(&reviewer.configuration, run),
        "reviewing task",
    )?;
    let d = result(&x["result"], worker)?;
    same(
        &x["target"],
        &json!({"assignmentIdentity":worker.source["assignmentIdentity"],"resultRevision":x["result"]["resultRevision"],"resultDigest":d}),
        "snapshot target",
    )?;
    for k in ["criteria", "rubric"] {
        same(&x[k], &worker.source[k], "target-owned review")?;
    }
    same(
        &context["repositoryBindingDigest"],
        &worker.configuration["repositoryBinding"]["bindingDigest"],
        "report repository",
    )?;
    let mut previous = None;
    for pair in list(&x["evidence"], 113, 0)? {
        exact(pair, "source report")?;
        same(
            &pair["source"]["reportIdentity"]["workspaceId"],
            &run["workspaceId"],
            "workspace-local report",
        )?;
        let key = domain("orkworks.assignment-evidence-source.v1", &pair["source"])?;
        ensure(
            previous.as_ref().is_none_or(|p: &String| p < &key),
            "sorted report evidence",
        )?;
        previous = Some(key);
        reports::validate(&pair["source"], &pair["report"], &context["profile"])?;
    }
    sealed(x, "inputDigest", "orkworks.child-review-input.v1")?;
    size(x, 262144, "snapshot size")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn child_sources_complete_plan_charges_repeated_retained_references() {
        // Exercise the accounting seam after configuration validation. Equal retained
        // bytes under two citations must still consume two reference occurrences.
        let p: Value = serde_json::from_str(include_str!(
            "../../../../../docs/validation/child-source-vectors.json"
        ))
        .unwrap();
        let w = &p["worker"];
        let owner = ParsedBundle {
            configuration: w["configuration"].clone(),
            evidence: vec![],
            plan: w["plan"].clone(),
            request: w["request"].clone(),
            approval: w["approval"].clone(),
            receipt: w["receipt"].clone(),
            reservation: w["reservation"].clone(),
            allocation: w["allocation"].clone(),
            attachment: w["attachment"].clone(),
            source: w["source"].clone(),
        };
        let settings_bytes = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            w["evidenceObjects"][0]["contentBase64"].as_str().unwrap(),
        )
        .unwrap();
        let settings: Value = serde_json::from_slice(&settings_bytes).unwrap();
        let c = &owner.configuration;
        let mut refs = vec![
            c["harness"]["effectiveSettingsDigest"].as_str().unwrap(),
            c["capabilityEvidence"]["effectiveSettingsDigest"]
                .as_str()
                .unwrap(),
            c["model"]["policyDigest"].as_str().unwrap(),
        ];
        refs.extend(
            c["capabilityEvidence"]["references"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["contentDigest"].as_str().unwrap()),
        );
        refs.extend(
            settings["configurationSources"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["contentDigest"].as_str().unwrap()),
        );
        let single = *refs
            .iter()
            .find(|key| refs.iter().filter(|x| *x == *key).count() == 1)
            .unwrap();
        let mut retained = std::collections::BTreeMap::new();
        for key in &refs {
            retained.insert((*key).into(), vec![0; 17]);
        }
        let fixed = canonical(&owner.plan).unwrap().len()
            + canonical(c).unwrap().len()
            + refs.iter().filter(|key| **key != single).count() * 17;
        retained.insert(single.into(), vec![0; 2097152 - fixed]);
        let mut ctx = Context {
            task: task(c, &p["run"]),
            settings: settings.clone(),
            retained,
        };
        assert!(bundle(&owner, &p["run"], &p["parentInput"], None, &ctx).is_ok());
        ctx.retained.get_mut(single).unwrap().push(0);
        assert_eq!(
            bundle(&owner, &p["run"], &p["parentInput"], None, &ctx),
            Err(ByteError("complete plan capacity"))
        );
        ctx.retained.get_mut(single).unwrap().pop();
        let repeated = *refs
            .iter()
            .find(|key| refs.iter().filter(|x| *x == *key).count() > 1)
            .unwrap();
        // Swap capacity between a single and multiply cited body: deduplication would
        // accept this, whereas the actual complete-plan count must reject it.
        ctx.retained.get_mut(single).unwrap().pop();
        ctx.retained.get_mut(repeated).unwrap().push(0);
        assert_eq!(
            bundle(&owner, &p["run"], &p["parentInput"], None, &ctx),
            Err(ByteError("complete plan capacity"))
        );
    }
}
