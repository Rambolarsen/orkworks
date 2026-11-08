use super::*;
use crate::taskmaster::assessment_report::{AssessmentRecord, AssessmentState};
use crate::taskmaster::assessment_sources::{tests::original, SqlAssessmentSources};
use serde_json::json;
const EPOCH: &str = "12345678-1234-4234-8234-123456789abc";

pub(crate) fn pending(sources: SqlAssessmentSources) -> AssessmentRecord {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("README.md"), "workflow guidance").unwrap();
    let context = crate::taskmaster::assessment_context::RepositoryContext::collect(
        root.path(),
        crate::taskmaster::runtime::ContextLevel::WorkflowContext,
        &[],
        "now",
    )
    .unwrap();
    let (mut record, _) = crate::taskmaster::assessment_report::tests::proposal();
    record.format_version = 4;
    record.state = AssessmentState::PendingRecommendation;
    let mut manifest = context.manifest();
    manifest.discovery_digest = Some("a".repeat(64));
    record.input_manifest = Some(manifest);
    record.report.as_mut().unwrap().input_fingerprint = sources
        .input_fingerprint(&json!({"fixture":"input"}))
        .unwrap();
    let report = record.report.as_ref().unwrap();
    let provenance = record
        .pending_recommendation
        .as_mut()
        .unwrap()
        .assessment
        .as_mut()
        .unwrap();
    provenance.input_fingerprint = report.input_fingerprint.clone();
    provenance.sql_sources = Some(sources.clone());
    record.sql_sources = Some(sources);
    record
}
#[test]
fn format_four_round_trips_every_state_and_keeps_published_sources_after_report_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let store = AssessmentStore::new(dir.path());
    let sources = original(EPOCH, "/workspace", &[("s", "created")]).into_sources();
    let pending = pending(sources.clone());
    let published = pending.pending_recommendation.clone().unwrap();
    assert!(published.source_session_ids.is_empty());
    for state in [
        AssessmentState::Queued,
        AssessmentState::Running,
        AssessmentState::PendingRecommendation,
        AssessmentState::Succeeded,
        AssessmentState::Failed,
        AssessmentState::Interrupted,
    ] {
        let mut record = pending.clone();
        record.state = state.clone();
        if state != AssessmentState::PendingRecommendation {
            record.pending_recommendation = None;
        }
        if matches!(
            state,
            AssessmentState::Queued
                | AssessmentState::Running
                | AssessmentState::Failed
                | AssessmentState::Interrupted
        ) {
            record.report = None;
            record.input_manifest = None;
        }
        store.save(&record).unwrap();
        assert_eq!(store.load_sql().unwrap().unwrap(), record);
        let bytes = std::fs::read(dir.path().join("assessment.json")).unwrap();
        assert!(store.load::<AssessmentRecord>().is_err());
        assert_eq!(
            std::fs::read(dir.path().join("assessment.json")).unwrap(),
            bytes
        );
    }
    let mut no_proposal = pending.clone();
    no_proposal.state = AssessmentState::Succeeded;
    no_proposal.pending_recommendation = None;
    let report = no_proposal.report.as_mut().unwrap();
    report.outcome = "no_proposal".into();
    report.proposal = None;
    report.recommendation_id = None;
    report.no_proposal_reason = Some("No grounded improvement".into());
    store.save(&no_proposal).unwrap();
    assert_eq!(store.load_sql().unwrap().unwrap(), no_proposal);
    let decoded: crate::taskmaster::Recommendation =
        serde_json::from_slice(&serde_json::to_vec(&published).unwrap()).unwrap();
    assert_eq!(decoded.assessment.unwrap().sql_sources, Some(sources));
}
#[test]
fn format_four_rejects_cross_copy_mismatch_missing_and_redacted_sql_use() {
    let dir = tempfile::tempdir().unwrap();
    let store = AssessmentStore::new(dir.path());
    let record = pending(original(EPOCH, "/workspace", &[]).into_sources());
    store.save(&record).unwrap();
    let before = std::fs::read(dir.path().join("assessment.json")).unwrap();
    let value = serde_json::to_value(&record).unwrap();
    for pointer in [
        "/sqlSources/assessmentId",
        "/pendingRecommendation/assessment/sqlSources/assessmentId",
        "/pendingRecommendation/assessment/inputFingerprint",
        "/report/id",
    ] {
        let mut bad = value.clone();
        *bad.pointer_mut(pointer).unwrap() = json!("other");
        assert!(store.save(&bad).is_err(), "{pointer}");
        assert_eq!(
            std::fs::read(dir.path().join("assessment.json")).unwrap(),
            before
        );
    }
    for field in ["sqlSources", "inputManifest"] {
        let mut bad = value.clone();
        bad.as_object_mut().unwrap().remove(field);
        assert!(store.save(&bad).is_err());
    }
    let mut redacted = record.clone();
    redacted.input_manifest.as_mut().unwrap().redact();
    redacted
        .pending_recommendation
        .as_mut()
        .unwrap()
        .assessment
        .as_mut()
        .unwrap()
        .access_redacted = true;
    store.save(&redacted).unwrap();
    assert!(store.load_sql().is_err());
    assert_eq!(redacted.sql_sources, record.sql_sources);
}
#[test]
fn legacy_formats_refuse_raw_top_and_nested_sql_fields_and_sql_use_without_rewrite() {
    let dir = tempfile::tempdir().unwrap();
    let store = AssessmentStore::new(dir.path());
    let mut record = pending(original(EPOCH, "/workspace", &[]).into_sources());
    record.sql_sources = None;
    record
        .pending_recommendation
        .as_mut()
        .unwrap()
        .assessment
        .as_mut()
        .unwrap()
        .sql_sources = None;
    for version in [1, 2, 3] {
        record.format_version = version;
        if version == 1 {
            record.input_manifest = None;
        } else if record.input_manifest.is_none() {
            record.input_manifest =
                pending(original(EPOCH, "/workspace", &[]).into_sources()).input_manifest;
        }
        record.readiness_reference = if version == 3 {
            Some(crate::taskmaster::readiness_focus::ReadinessReference {
                snapshot_id: EPOCH.into(),
                rubric_digest: "a".repeat(64),
            })
        } else {
            None
        };
        store.save(&record).unwrap();
        let before = std::fs::read(dir.path().join("assessment.json")).unwrap();
        assert!(store.load_sql().is_err());
        assert_eq!(
            std::fs::read(dir.path().join("assessment.json")).unwrap(),
            before
        );
        for nested in [false, true] {
            for field in [
                json!(null),
                serde_json::to_value(original(EPOCH, "/workspace", &[]).into_sources()).unwrap(),
            ] {
                let mut bad = serde_json::to_value(&record).unwrap();
                if nested {
                    bad["pendingRecommendation"]["assessment"]["sqlSources"] = field;
                } else {
                    bad["sqlSources"] = field;
                }
                assert!(store.save(&bad).is_err());
                assert_eq!(
                    std::fs::read(dir.path().join("assessment.json")).unwrap(),
                    before
                );
            }
        }
    }
}
#[test]
fn format_four_legacy_publication_and_recovery_never_acknowledge_or_write() {
    let mut record = pending(original(EPOCH, "/workspace", &[]).into_sources());
    let before = record.clone();
    let writes = std::cell::Cell::new(0);
    assert!(crate::taskmaster::assessment_report::publish_with(
        &mut record,
        |_| {
            writes.set(writes.get() + 1);
            Ok(())
        },
        |_| {
            writes.set(writes.get() + 1);
            Ok(())
        }
    )
    .is_err());
    assert!(crate::taskmaster::assessment_report::recover_with(
        &mut record,
        |_| {
            writes.set(writes.get() + 1);
            Ok(())
        },
        |_| {
            writes.set(writes.get() + 1);
            Ok(())
        }
    )
    .is_err());
    assert_eq!(record, before);
    assert_eq!(writes.get(), 0);
}
#[test]
fn format_four_total_limit_and_failed_save_preserve_pending_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let store = AssessmentStore::new(dir.path());
    let mut record = pending(original(EPOCH, "/workspace", &[]).into_sources());
    store.save(&record).unwrap();
    let before = std::fs::read(dir.path().join("assessment.json")).unwrap();
    set_save_failures(dir.path(), 0, 1);
    record.error_summary = Some("failure".into());
    assert!(store.save(&record).is_err());
    assert_eq!(
        std::fs::read(dir.path().join("assessment.json")).unwrap(),
        before
    );
    record.error_summary = Some(String::new());
    let remaining = MAX_ASSESSMENT_BYTES - serde_json::to_vec(&record).unwrap().len();
    record.error_summary = Some("x".repeat(remaining));
    store.save(&record).unwrap();
    let boundary = std::fs::read(dir.path().join("assessment.json")).unwrap();
    assert_eq!(boundary.len(), MAX_ASSESSMENT_BYTES);
    assert_eq!(store.load_sql().unwrap().unwrap(), record);
    record.error_summary.as_mut().unwrap().push('x');
    assert!(store.save(&record).is_err());
    assert_eq!(
        std::fs::read(dir.path().join("assessment.json")).unwrap(),
        boundary
    );
}
#[test]
fn duplicate_parent_fields_cannot_hide_legacy_sql_source_fields() {
    let dir = tempfile::tempdir().unwrap();
    let store = AssessmentStore::new(dir.path());
    let (record, _) = crate::taskmaster::assessment_report::tests::proposal();
    let bytes=serde_json::to_string(&record).unwrap().replacen("\"pendingRecommendation\":","\"pendingRecommendation\":{\"assessment\":{\"sqlSources\":null}},\"pendingRecommendation\":",1);
    std::fs::write(dir.path().join("assessment.json"), &bytes).unwrap();
    assert!(store.load::<serde_json::Value>().is_err());
    assert_eq!(
        std::fs::read(dir.path().join("assessment.json")).unwrap(),
        bytes.as_bytes()
    );
}

#[test]
fn format_four_keeps_independent_pending_readiness_validation() {
    use crate::taskmaster::readiness_focus::{
        ReadinessReference, RecommendationReadinessReference,
    };
    let dir = tempfile::tempdir().unwrap();
    let store = AssessmentStore::new(dir.path());
    let mut record = pending(original(EPOCH, "/workspace", &[]).into_sources());
    record.readiness_reference = Some(ReadinessReference {
        snapshot_id: EPOCH.into(),
        rubric_digest: "a".repeat(64),
    });
    assert!(
        store.save(&record).is_err(),
        "pending provenance must carry the same readiness source"
    );
    let reference = RecommendationReadinessReference {
        source: record.readiness_reference.clone().unwrap(),
        concept_ids: vec![crate::taskmaster::readiness_rubric::CONCEPT_IDS[0].into()],
    };
    record
        .pending_recommendation
        .as_mut()
        .unwrap()
        .assessment
        .as_mut()
        .unwrap()
        .readiness_reference = Some(reference);
    store.save(&record).unwrap();
    assert_eq!(store.load_sql().unwrap().unwrap(), record);
    record
        .pending_recommendation
        .as_mut()
        .unwrap()
        .assessment
        .as_mut()
        .unwrap()
        .readiness_reference
        .as_mut()
        .unwrap()
        .concept_ids
        .clear();
    assert!(store.save(&record).is_err());
}
