use super::model::{CallBudget, InvestigationStep};
use crate::taskmaster::{
    assessment_context::improvement::ProgressiveEvidence, runtime::ContextLevel,
};
use std::time::{Duration, Instant};

#[test]
fn five_evidence_calls_leave_one_final_call_and_never_allow_a_seventh() {
    let start = Instant::now();
    let mut budget = CallBudget::new(start);
    for _ in 0..5 {
        budget.reserve(start, false).unwrap();
    }
    assert!(budget.reserve(start, false).is_err());
    budget.reserve(start, true).unwrap();
    assert!(budget.reserve(start, true).is_err());
}

#[test]
fn finalization_can_happen_early_but_deadline_cannot_be_extended() {
    let start = Instant::now();
    let mut budget = CallBudget::new(start);
    budget.reserve(start, true).unwrap();
    assert!(budget.reserve(start, false).is_err());
    let mut budget = CallBudget::new(start);
    assert!(budget
        .reserve(start + Duration::from_secs(480), true)
        .is_err());
}

#[test]
fn step_schema_rejects_unknown_duplicate_command_and_excess_requests() {
    let valid = r#"{"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}"#;
    assert!(InvestigationStep::decode(valid, 1, &[]).is_ok());
    for bad in [
        r#"{"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true,"command":"ls"}"#,
        r#"{"version":1,"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}"#,
        r#"{"version":1,"repositoryReads":[{"fileId":1,"offset":0,"length":10}],"knowledgePageIds":[],"finish":false}"#,
        r#"{"version":1,"repositoryReads":[],"knowledgePageIds":["invented.md"],"finish":false}"#,
        r#"{"version":1,"repositoryReads":[{"fileId":0,"offset":0,"length":8193}],"knowledgePageIds":[],"finish":false}"#,
        r#"{"version":1,"repositoryReads":[{"fileId":0,"offset":0,"length":1},{"fileId":0,"offset":0,"length":1}],"knowledgePageIds":[],"finish":false}"#,
    ] {
        assert!(InvestigationStep::decode(bad, 1, &[]).is_err(), "{bad}");
    }
}

fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("AGENTS.md"), "Repository instructions.").unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    std::fs::write(
        dir.path().join("src/feature.rs"),
        format!("{}important implementation", "a".repeat(9000)),
    )
    .unwrap();
    dir
}

#[test]
fn evidence_reads_real_implementation_after_the_original_prefix_without_candidate() {
    let dir = repository();
    let mut evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let file = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/feature.rs")
        .unwrap();
    let fact = evidence.read(file, 9000, 24).unwrap();
    assert_eq!(fact.text, "important implementation");
    assert_eq!(fact.offset, 9000);
    assert!(!fact.complete());
    assert!(evidence.facts().iter().any(|f| f.path == "AGENTS.md"));
    evidence.revalidate().unwrap();
}

#[test]
fn repeated_exact_reads_do_not_consume_budget_but_changed_evidence_invalidates() {
    let dir = repository();
    let mut evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let file = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/feature.rs")
        .unwrap();
    let first = evidence.read(file, 9000, 24).unwrap();
    let count = evidence.facts().len();
    assert_eq!(evidence.read(file, 9000, 24).unwrap(), first);
    assert_eq!(evidence.facts().len(), count);
    std::fs::write(
        dir.path().join("src/feature.rs"),
        format!("{}changed implementation!!", "a".repeat(9000)),
    )
    .unwrap();
    assert!(evidence.revalidate().is_err());
    assert!(evidence.read(file, 9000, 24).is_err());
}

#[test]
fn file_and_range_identity_and_utf8_boundaries_are_enforced() {
    let dir = repository();
    std::fs::write(dir.path().join("src/unicode.rs"), "åbc").unwrap();
    let mut evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let file = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/unicode.rs")
        .unwrap();
    assert!(evidence.read(file, 1, 1).is_err());
    assert!(evidence.read(file, 0, 1).is_err());
    let fact = evidence.read(file, 0, 4).unwrap();
    assert_eq!(fact.text, "åbc");
    assert!(fact.complete());
    assert!(evidence.read(file, u64::MAX, 2).is_err());
    assert!(evidence.read(usize::MAX, 0, 1).is_err());
}

#[test]
fn exclusions_and_sensitive_content_remain_denied_at_later_offsets() {
    let dir = repository();
    std::fs::write(dir.path().join("src/denied.rs"), "denied").unwrap();
    std::fs::write(
        dir.path().join("src/worker.rs"),
        format!("{}client_secret=secret", "x".repeat(9000)),
    )
    .unwrap();
    let mut evidence = ProgressiveEvidence::collect(
        dir.path(),
        ContextLevel::SourceCode,
        &["src/denied.rs".into()],
        "now",
    )
    .unwrap();
    assert!(!evidence.catalog().iter().any(|p| p == "src/denied.rs"));
    let id = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/worker.rs")
        .unwrap();
    assert!(evidence.read(id, 9000, 20).is_err());
}

fn final_fixture() -> (
    serde_json::Value,
    crate::taskmaster::readiness_rubric::ReadinessRubric,
) {
    use crate::taskmaster::readiness_rubric::CONCEPT_IDS;
    let rubric = serde_json::from_value(serde_json::json!({"id":"brain-agent-readiness","revision":1,"concepts":CONCEPT_IDS.iter().map(|id|serde_json::json!({"conceptId":id,"pageId":"playbooks/rubric.md","pageSha256":"b".repeat(64),"levels":["a","b","c","d","e"]})).collect::<Vec<_>>()})).unwrap();
    let result = serde_json::json!({"version":1,"summary":"Scoped guidance offers a practical next improvement.","concepts":CONCEPT_IDS.iter().map(|id|serde_json::json!({"conceptId":id,"maturity":2,"confidence":"medium","repositoryFactIds":[0],"rubricPageId":"playbooks/rubric.md","rationale":"Documented guidance","limits":["Behavior remains unverified"],"nextTransition":"Evidence of workflow integration","unassessedReason":null})).collect::<Vec<_>>(),
      "opportunities":[{"id":"scoped-guidance","targetSurface":"instructions","title":"Scoped guidance","summary":"The entry point would link the existing scoped instructions.","mechanism":"The inspected entry point contains only general guidance.","benefit":"The relevant scoped constraints would be discoverable.","evidenceStrength":"supported_hypothesis","effort":"low","risk":"low","reversibility":"high","repositoryFactHashes":["a".repeat(64)],"knowledgePageIds":["playbooks/rubric.md"],"validation":"Whether scoped constraints are discoverable remains unverified.","rollback":"The prior entry point would be restored."}],
      "preferredOpportunityId":"scoped-guidance","noProposalReason":null,"limitations":["Behavior remains unverified"]});
    (result, rubric)
}

#[test]
fn final_response_joins_rating_and_preferred_step_against_the_same_evidence() {
    let (value, rubric) = final_fixture();
    let result = super::model::FinalResult::decode(
        &value.to_string(),
        &["a".repeat(64)],
        &["playbooks/rubric.md".into()],
        &rubric,
    )
    .unwrap();
    assert_eq!(result.readiness().totals(), (14, Some(28), Some(50)));
    assert_eq!(result.preferred().unwrap().id, "scoped-guidance");
    for mutate in 0..4 {
        let mut bad = value.clone();
        match mutate {
            0 => {
                bad["opportunities"][0]["repositoryFactHashes"] =
                    serde_json::json!(["c".repeat(64)])
            }
            1 => bad["concepts"][0]["repositoryFactIds"] = serde_json::json!([1]),
            2 => bad["preferredOpportunityId"] = serde_json::json!("invented"),
            _ => bad["overallScore"] = serde_json::json!(99),
        }
        assert!(super::model::FinalResult::decode(
            &bad.to_string(),
            &["a".repeat(64)],
            &["playbooks/rubric.md".into()],
            &rubric
        )
        .is_err());
    }
}

#[test]
fn unknown_capabilities_and_no_supported_opportunity_do_not_manufacture_a_score_or_fix() {
    let (mut value, rubric) = final_fixture();
    for row in value["concepts"].as_array_mut().unwrap() {
        row["maturity"] = serde_json::Value::Null;
        row["confidence"] = serde_json::Value::Null;
        row["repositoryFactIds"] = serde_json::json!([]);
        row["unassessedReason"] = serde_json::json!("Evidence is unavailable.");
    }
    value["opportunities"] = serde_json::json!([]);
    value["preferredOpportunityId"] = serde_json::Value::Null;
    value["noProposalReason"] =
        serde_json::json!("Permitted evidence does not establish a useful next step.");
    let result = super::model::FinalResult::decode(
        &value.to_string(),
        &[],
        &["playbooks/rubric.md".into()],
        &rubric,
    )
    .unwrap();
    assert_eq!(result.readiness().totals(), (0, None, None));
    assert!(result.preferred().is_none());
    value["noProposalReason"] = serde_json::Value::Null;
    assert!(super::model::FinalResult::decode(
        &value.to_string(),
        &[],
        &["playbooks/rubric.md".into()],
        &rubric
    )
    .is_err());
}

#[test]
fn progressive_ranges_obey_the_exact_aggregate_budget_without_partial_mutation() {
    let dir = repository();
    std::fs::write(dir.path().join("src/large.rs"), "z".repeat(100_000)).unwrap();
    let mut evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let file = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/large.rs")
        .unwrap();
    let initial: usize = evidence.facts().iter().map(|f| f.text.len()).sum();
    let mut offset = 0;
    while offset + 8192 + initial <= 65536 {
        evidence.read(file, offset as u64, 8192).unwrap();
        offset += 8192;
    }
    let remaining = 65536 - initial - offset;
    if remaining != 0 {
        evidence.read(file, offset as u64, remaining).unwrap();
    }
    assert_eq!(
        evidence.facts().iter().map(|f| f.text.len()).sum::<usize>(),
        65536
    );
    let before = evidence.facts().to_vec();
    assert!(evidence.read(file, 90_000, 1).is_err());
    assert_eq!(evidence.facts(), before);
}

#[test]
fn changed_instructions_or_new_ignore_rules_invalidate_progressive_evidence() {
    let dir = repository();
    git2::Repository::init(dir.path()).unwrap();
    let mut evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let file = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/feature.rs")
        .unwrap();
    evidence.read(file, 9000, 24).unwrap();
    std::fs::write(dir.path().join(".gitignore"), "src/feature.rs\n").unwrap();
    assert!(evidence.revalidate().is_err());
    std::fs::remove_file(dir.path().join(".gitignore")).unwrap();
    std::fs::write(dir.path().join("AGENTS.md"), "Changed governing rules.").unwrap();
    assert!(evidence.revalidate().is_err());
}

#[cfg(unix)]
#[test]
fn symlinks_are_neither_catalog_authority_nor_a_later_replacement_bypass() {
    let dir = repository();
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("outside.rs"), "private text").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("outside.rs"),
        dir.path().join("src/link.rs"),
    )
    .unwrap();
    let mut evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    assert!(!evidence.catalog().iter().any(|p| p == "src/link.rs"));
    let file = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/feature.rs")
        .unwrap();
    std::fs::remove_file(dir.path().join("src/feature.rs")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("outside.rs"),
        dir.path().join("src/feature.rs"),
    )
    .unwrap();
    assert!(evidence.read(file, 0, 1).is_err());
}

#[test]
fn investigation_collects_a_later_range_and_finalizes_one_shared_result_without_observations() {
    let dir = repository();
    let evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let file = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/feature.rs")
        .unwrap();
    let (mut value, rubric) = final_fixture();
    let page: crate::taskmaster::runtime::KnowledgePage = serde_json::from_value(serde_json::json!({"id":"playbooks/rubric.md","title":"Guidance","type":"playbook","status":"active","content":"Inspect relevant mechanisms and choose one useful improvement.","sha256":"b".repeat(64),"relatedIds":[]})).unwrap();
    let mut calls = 0;
    let mut invoke = |prompt: &str, _remaining: Duration| {
        calls += 1;
        let p: serde_json::Value = serde_json::from_str(prompt).unwrap();
        if calls == 1 {
            assert_eq!(p["phase"], "evidence");
            Ok(serde_json::json!({"version":1,"repositoryReads":[{"fileId":file,"offset":9000,"length":24}],"knowledgePageIds":[],"finish":false}).to_string())
        } else if calls == 2 {
            assert!(prompt.contains("important implementation"));
            Ok(serde_json::json!({"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}).to_string())
        } else {
            assert_eq!(p["phase"], "final");
            let facts = p["repositoryFacts"].as_array().unwrap();
            value["opportunities"][0]["repositoryFactHashes"] =
                serde_json::json!([facts.last().unwrap()["sha256"]]);
            Ok(value.to_string())
        }
    };
    let run = super::engine::investigate(
        evidence,
        &rubric,
        &[page],
        "playbooks/rubric.md",
        &mut invoke,
        || false,
    )
    .unwrap();
    assert_eq!(calls, 3);
    assert_eq!(run.result.preferred().unwrap().id, "scoped-guidance");
    assert_eq!(run.result.readiness().totals().2, Some(50));
}

#[test]
fn saved_range_manifest_restores_exact_inputs_and_rejects_changed_uncited_ranges() {
    let dir = repository();
    let mut evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let file = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/feature.rs")
        .unwrap();
    evidence.read(file, 9000, 24).unwrap();
    let manifest = evidence.range_manifest();
    let restored = ProgressiveEvidence::restore(
        dir.path(),
        ContextLevel::SourceCode,
        &[],
        "now",
        evidence.catalog_digest(),
        &manifest,
    )
    .unwrap();
    assert_eq!(restored.facts(), evidence.facts());
    std::fs::write(
        dir.path().join("src/feature.rs"),
        format!("{}changed implementation!!", "a".repeat(9000)),
    )
    .unwrap();
    assert!(ProgressiveEvidence::restore(
        dir.path(),
        ContextLevel::SourceCode,
        &[],
        "now",
        evidence.catalog_digest(),
        &manifest
    )
    .is_err());
}

#[test]
fn improvement_store_is_separate_bounded_versioned_and_redacts_generated_results() {
    use super::store::{Record, ReportStore};
    let dir = tempfile::tempdir().unwrap();
    let store = ReportStore::new(dir.path());
    let record = Record::queued("test-run", "now");
    store.save(&record).unwrap();
    assert_eq!(store.load().unwrap(), Some(record.clone()));
    assert!(!dir.path().join("assessment.json").exists());
    let mut unsupported = record.clone();
    unsupported.format_version = 2;
    assert!(store.save(&unsupported).is_err());
    let mut excessive = record.clone();
    excessive.error_code = Some("x".repeat(128 * 1024));
    assert!(store.save(&excessive).is_err());
    let mut record = record;
    record.error_code = Some("private generated diagnostic".into());
    record.redact();
    assert!(!serde_json::to_string(&record)
        .unwrap()
        .contains("private generated"));
    store.save(&record).unwrap();
    assert_eq!(
        store.load().unwrap().unwrap().state,
        super::store::RunState::AccessRedacted
    );
}

fn guidance_fixture() -> crate::taskmaster::runtime::KnowledgePage {
    serde_json::from_value(serde_json::json!({"id":"playbooks/rubric.md","title":"Guidance","type":"playbook","status":"active","content":"Inspect relevant mechanisms and choose one useful improvement.","sha256":"b".repeat(64),"relatedIds":[]})).unwrap()
}

#[test]
fn absolute_deadline_includes_collection_and_denies_provider_dispatch_after_expiry() {
    let dir = repository();
    let evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let (_, rubric) = final_fixture();
    let mut calls = 0;
    let mut invoke = |_: super::engine::Invocation<'_>| {
        calls += 1;
        Ok(String::new())
    };
    let result = super::engine::investigate_from(
        Instant::now() - Duration::from_secs(480),
        evidence,
        &rubric,
        &[guidance_fixture()],
        "playbooks/rubric.md",
        &mut invoke,
        || false,
    );
    assert!(result.is_err());
    assert_eq!(calls, 0);
}

#[test]
fn cancellation_and_invalid_output_never_retry_or_publish_a_partial_result() {
    use std::cell::Cell;
    let dir = repository();
    let (_, rubric) = final_fixture();
    for invalid in [false, true] {
        let evidence =
            ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
        let cancelled = Cell::new(false);
        let mut calls = 0;
        let mut invoke = |_: &str, _: Duration| {
            calls += 1;
            if invalid {
                Ok("{invalid".into())
            } else {
                cancelled.set(true);
                Ok(
                    r#"{"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}"#
                        .into(),
                )
            }
        };
        assert!(super::engine::investigate(
            evidence,
            &rubric,
            &[guidance_fixture()],
            "playbooks/rubric.md",
            &mut invoke,
            || cancelled.get()
        )
        .is_err());
        assert_eq!(calls, 1);
    }
}

#[test]
fn investigation_sixth_call_is_final_and_not_an_extra_evidence_round() {
    let dir = repository();
    let evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let id = evidence
        .catalog()
        .iter()
        .position(|p| p == "AGENTS.md")
        .unwrap();
    let (mut value, rubric) = final_fixture();
    let mut calls = 0;
    let mut invoke = |prompt: &str, remaining: Duration| {
        calls += 1;
        assert!(remaining <= Duration::from_secs(480));
        let p: serde_json::Value = serde_json::from_str(prompt).unwrap();
        if calls < 6 {
            assert_eq!(p["phase"], "evidence");
            Ok(serde_json::json!({"version":1,"repositoryReads":[{"fileId":id,"offset":0,"length":1}],"knowledgePageIds":[],"finish":false}).to_string())
        } else {
            assert_eq!(calls, 6);
            assert_eq!(p["phase"], "final");
            value["opportunities"][0]["repositoryFactHashes"] =
                serde_json::json!([p["repositoryFacts"][0]["sha256"]]);
            Ok(value.to_string())
        }
    };
    assert!(super::engine::investigate(
        evidence,
        &rubric,
        &[guidance_fixture()],
        "playbooks/rubric.md",
        &mut invoke,
        || false
    )
    .is_ok());
    assert_eq!(calls, 6);
}

#[test]
fn guidance_and_range_count_limits_are_enforced_before_provider_or_partial_admission() {
    let dir = repository();
    let mut evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let file = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/feature.rs")
        .unwrap();
    for offset in 0..31 {
        evidence.read(file, offset, 1).unwrap();
    }
    assert_eq!(evidence.facts().len(), 32);
    assert!(evidence.read(file, 32, 1).is_err());
    assert_eq!(evidence.facts().len(), 32);
    let (_, rubric) = final_fixture();
    let mut page = guidance_fixture();
    page.content = "g".repeat(32769);
    let mut calls = 0;
    let mut invoke = |_: &str, _: Duration| {
        calls += 1;
        Ok(String::new())
    };
    assert!(super::engine::investigate(
        evidence,
        &rubric,
        &[page],
        "playbooks/rubric.md",
        &mut invoke,
        || false
    )
    .is_err());
    assert_eq!(calls, 0);
}

#[test]
fn dispatch_receives_trusted_evidence_pages_and_code_owned_phase() {
    let dir = repository();
    let evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let (mut value, rubric) = final_fixture();
    let mut calls = 0;
    let mut invoke = |input: super::engine::Invocation<'_>| {
        calls += 1;
        assert_eq!(
            input.selected_page_ids,
            &["playbooks/rubric.md".to_string()]
        );
        assert!(!input.evidence.facts().is_empty());
        assert!(input.remaining <= Duration::from_secs(480));
        if calls == 1 {
            assert_eq!(input.phase, super::engine::InvocationPhase::Evidence);
            Ok(r#"{"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}"#.into())
        } else {
            assert_eq!(input.phase, super::engine::InvocationPhase::Final);
            value["opportunities"][0]["repositoryFactHashes"] =
                serde_json::json!([input.evidence.facts()[0].sha256]);
            Ok(value.to_string())
        }
    };
    let result = super::engine::investigate_from(
        Instant::now(),
        evidence,
        &rubric,
        &[guidance_fixture()],
        "playbooks/rubric.md",
        &mut invoke,
        || false,
    )
    .unwrap();
    assert!(result.result.preferred().is_some());
    assert_eq!(calls, 2);
}

#[test]
fn mandatory_rubric_pages_are_supplied_before_the_first_exchange() {
    use super::engine::{investigate_from, Invocation, InvocationPhase};
    let dir = repository();
    let evidence =
        ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
    let (mut value, rubric) = final_fixture();
    value["opportunities"][0]["repositoryFactHashes"] =
        serde_json::json!([evidence.facts()[0].sha256]);
    let rubric_page = guidance_fixture();
    let mut entry = rubric_page.clone();
    entry.id = "playbooks/taskmaster-improvement.md".into();
    entry.content = "Use the complete linked rubric before assessing.".into();
    let mut catalog_only = rubric_page.clone();
    catalog_only.id = "concepts/catalog-only.md".into();
    let mut calls = 0;
    let result = investigate_from(
        Instant::now(),
        evidence,
        &rubric,
        &[entry.clone(), rubric_page.clone(), catalog_only.clone()],
        &entry.id,
        &mut |input: Invocation<'_>| {
            calls += 1;
            assert_eq!(
                input.selected_page_ids,
                &[entry.id.clone(), rubric_page.id.clone()]
            );
            assert!(input.prompt.contains(&rubric_page.content));
            if input.phase == InvocationPhase::Evidence {
                Ok(
                    r#"{"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true}"#
                        .into(),
                )
            } else {
                let prompt: serde_json::Value = serde_json::from_str(input.prompt).unwrap();
                let sources = &prompt["finalCitationSources"];
                assert_eq!(
                    sources["knowledgePageIds"],
                    serde_json::json!([entry.id, rubric_page.id])
                );
                assert!(prompt["knowledgeCatalog"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|page| page["id"] == catalog_only.id));
                let supplied_hashes = prompt["repositoryFacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|fact| fact["sha256"].clone())
                    .collect::<Vec<_>>();
                assert_eq!(
                    sources["repositoryFactHashes"],
                    serde_json::json!(supplied_hashes)
                );
                Ok(value.to_string())
            }
        },
        || false,
    )
    .unwrap();
    assert_eq!(calls, 2);
    assert_eq!(result.supplied_page_ids.len(), 2);
}

#[test]
fn missing_or_oversized_mandatory_rubric_prevents_any_dispatch() {
    let dir = repository();
    let (_, rubric) = final_fixture();
    let mut entry = guidance_fixture();
    entry.id = "playbooks/taskmaster-improvement.md".into();
    for oversized in [false, true] {
        let evidence =
            ProgressiveEvidence::collect(dir.path(), ContextLevel::SourceCode, &[], "now").unwrap();
        let mut pages = vec![entry.clone()];
        if oversized {
            let mut page = guidance_fixture();
            page.content = "a".repeat(32 * 1024);
            pages.push(page);
        }
        let mut calls = 0;
        assert!(super::engine::investigate(
            evidence,
            &rubric,
            &pages,
            &entry.id,
            &mut |_, _| {
                calls += 1;
                Ok(String::new())
            },
            || false
        )
        .is_err());
        assert_eq!(calls, 0);
    }
}

#[test]
fn cancellation_guard_stops_startup_and_later_range_reads() {
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    };
    let dir = repository();
    std::fs::write(
        dir.path().join("package.json"),
        r#"{"scripts":{"test":"node --test"}}"#,
    )
    .unwrap();
    let total = Arc::new(AtomicUsize::new(0));
    let calls = total.clone();
    ProgressiveEvidence::collect_controlled(
        dir.path(),
        ContextLevel::SourceCode,
        &[],
        "now",
        move || {
            calls.fetch_add(1, Ordering::SeqCst);
            Ok(())
        },
    )
    .unwrap();
    assert!(total.load(Ordering::SeqCst) > 3);
    // Cancel at each preparation/read boundary, including after preparation has begun.
    for cutoff in 1..=total.load(Ordering::SeqCst) {
        let calls = Arc::new(AtomicUsize::new(0));
        let observed = calls.clone();
        let result = ProgressiveEvidence::collect_controlled(
            dir.path(),
            ContextLevel::SourceCode,
            &[],
            "now",
            move || {
                if observed.fetch_add(1, Ordering::SeqCst) + 1 >= cutoff {
                    Err("cancelled".into())
                } else {
                    Ok(())
                }
            },
        );
        assert!(matches!(result, Err(ref error) if error == "cancelled"));
        assert_eq!(calls.load(Ordering::SeqCst), cutoff);
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    let mut evidence = ProgressiveEvidence::collect_controlled(
        dir.path(),
        ContextLevel::SourceCode,
        &[],
        "now",
        move || {
            if flag.load(Ordering::SeqCst) {
                Err("cancelled".into())
            } else {
                Ok(())
            }
        },
    )
    .unwrap();
    let id = evidence
        .catalog()
        .iter()
        .position(|p| p == "src/feature.rs")
        .unwrap();
    cancelled.store(true, Ordering::SeqCst);
    assert_eq!(evidence.read(id, 9000, 24).unwrap_err(), "cancelled");
    assert_eq!(evidence.revalidate().unwrap_err(), "cancelled");
}

#[test]
fn descriptive_readiness_inventory_and_benefit_preserve_a_grounded_opportunity() {
    let (mut value, rubric) = final_fixture();
    let row = value["concepts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["conceptId"] == "feedback-loops")
        .unwrap();
    row["limits"] =
        serde_json::json!(["No outcomes log, issue history, or correction records are supplied."]);
    value["opportunities"][0]["benefit"]=serde_json::json!("Preserved CI failure status would make automated test feedback more actionable and prevent failure suppression from appearing equivalent to a reliable signal.");
    let result = super::model::FinalResult::decode(
        &value.to_string(),
        &["a".repeat(64)],
        &["playbooks/rubric.md".into()],
        &rubric,
    )
    .unwrap();
    assert_eq!(result.preferred().unwrap().id, "scoped-guidance");
}
