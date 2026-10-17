//! Freshness and access fences shared by every assessment consumer.
use super::runtime::{ContextLevel, TaskmasterSettings};
use super::{assessment_report::AssessmentRecord, Recommendation};

const REDACTED_SUMMARY: &str = "Assessment evidence was redacted after context access changed.";

pub(crate) fn access_narrowed(
    previous: ContextLevel,
    excluded: &[String],
    current: &TaskmasterSettings,
) -> bool {
    fn rank(level: ContextLevel) -> u8 {
        match level {
            ContextLevel::SessionObservations => 0,
            ContextLevel::WorkflowContext => 1,
            ContextLevel::SourceCode => 2,
        }
    }
    rank(current.context_level) < rank(previous)
        || current.excluded_paths.iter().any(|new| {
            !excluded
                .iter()
                .any(|old| super::context::path_is_excluded(new, old))
        })
}

pub(crate) fn redact_recommendation(
    recommendation: &mut Recommendation,
    settings: &TaskmasterSettings,
) {
    let Some(provenance) = &recommendation.assessment else {
        return;
    };
    if !access_narrowed(
        provenance.context_level,
        &provenance.excluded_paths,
        settings,
    ) {
        return;
    }
    recommendation.repository_evidence.retain(|fact| {
        super::context::fact_is_permitted(
            &fact.path,
            settings.context_level,
            &settings.excluded_paths,
        )
    });
    recommendation.title = "Assessment evidence access changed".into();
    recommendation.summary = REDACTED_SUMMARY.into();
    recommendation.reason.clear();
    recommendation.evidence.clear();
    recommendation.suggested_prompt = None;
    recommendation
        .workflow_improvement
        .proposed_improvement
        .clear();
    recommendation.workflow_improvement.expected_benefit.clear();
    recommendation.proposed_change = None;
    recommendation
        .assessment
        .as_mut()
        .expect("assessment provenance")
        .access_redacted = true;
    if let Some(packet) = recommendation.completion_packet.as_mut() {
        packet.action.prompt = REDACTED_SUMMARY.into();
        packet.action.scope = REDACTED_SUMMARY.into();
        packet.subject.scope = REDACTED_SUMMARY.into();
        packet.subject.changed_paths.retain(|path| {
            super::context::fact_is_permitted(
                path,
                settings.context_level,
                &settings.excluded_paths,
            )
        });
        if let Some(verification) = packet.verification.as_mut() {
            verification.command = REDACTED_SUMMARY.into();
        }
        for issue in packet
            .missing_evidence
            .iter_mut()
            .chain(packet.conflicting_evidence.iter_mut())
        {
            issue.detail = REDACTED_SUMMARY.into();
        }
        // Approval, fingerprints and lineage are immutable audit identities, not
        // authority for the sanitized action. Every mutation rejects the marker.
    }
}

pub(crate) fn redact_record(record: &mut AssessmentRecord, settings: &TaskmasterSettings) {
    if let Some(report) = record.report.as_mut() {
        if access_narrowed(report.context_level, &report.excluded_paths, settings) {
            report.repository_evidence.retain(|fact| {
                super::context::fact_is_permitted(
                    &fact.path,
                    settings.context_level,
                    &settings.excluded_paths,
                )
            });
            report.summary = REDACTED_SUMMARY.into();
            report.proposal = None;
            report.no_proposal_reason = None;
            report.unverified_checks.clear();
        }
    }
    if let Some(recommendation) = record.pending_recommendation.as_mut() {
        redact_recommendation(recommendation, settings);
    }
}

pub(crate) fn reconcile_metadata(
    root: &std::path::Path,
    settings: &TaskmasterSettings,
) -> Result<(), String> {
    let report_store = super::assessment_store::AssessmentStore::new(root);
    if let Some(mut record) = report_store.load::<AssessmentRecord>()? {
        let original = record.clone();
        redact_record(&mut record, settings);
        if record != original {
            report_store.save(&record)?;
        }
    }
    let store = super::store::RecommendationStore::open(root.into())
        .map_err(|_| "Assessment recommendations are unavailable")?;
    for mut recommendation in store
        .list()
        .map_err(|_| "Assessment recommendations are unavailable")?
    {
        let original = recommendation.clone();
        redact_recommendation(&mut recommendation, settings);
        if recommendation != original {
            store
                .put(&recommendation)
                .map_err(|_| "Assessment evidence redaction could not be saved")?;
        }
    }
    Ok(())
}

pub(crate) fn fence_recommendation(
    recommendation: &mut Recommendation,
    record: Option<&AssessmentRecord>,
    current_fingerprint: Option<&str>,
    mutating: bool,
) -> bool {
    use super::{assessment_report::AssessmentState, RecommendationStatus};
    let Some(provenance) = recommendation.assessment.as_ref() else {
        return true;
    };
    let pending = record.is_some_and(|record| {
        record.state == AssessmentState::PendingRecommendation
            && (record.id == provenance.id
                || record
                    .pending_recommendation
                    .as_ref()
                    .is_some_and(|pending| pending.id == recommendation.id))
    });
    if pending {
        return false;
    }
    let active = matches!(
        recommendation.status,
        RecommendationStatus::Proposed
            | RecommendationStatus::Accepted
            | RecommendationStatus::Executing
    );
    if !active {
        return true;
    }
    let fresh = !provenance.access_redacted
        && record.is_some_and(|record| {
            record.state == AssessmentState::Succeeded
                && record.pending_recommendation.is_none()
                && record.id == provenance.id
                && record.report.as_ref().is_some_and(|report| {
                    report.id == provenance.id
                        && report.recommendation_id.as_deref() == Some(recommendation.id.as_str())
                        && report.input_fingerprint == provenance.input_fingerprint
                        && current_fingerprint == Some(report.input_fingerprint.as_str())
                })
        });
    if !fresh && mutating && recommendation.status == RecommendationStatus::Proposed {
        recommendation.status = RecommendationStatus::Superseded;
        recommendation.updated_at = chrono::Utc::now().to_rfc3339();
    }
    fresh
}

/// Validates a redacted packet as history without rebinding its old approval.
pub(crate) fn validate_redacted_packet(
    packet: &super::completion::CompletionPacket,
    workspace: &str,
) -> Result<(), String> {
    let fingerprint =
        |value: &str| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit());
    if packet.action.prompt != REDACTED_SUMMARY
        || packet.action.scope != REDACTED_SUMMARY
        || packet.subject.scope != REDACTED_SUMMARY
        || packet
            .verification
            .as_ref()
            .is_some_and(|verification| verification.command != REDACTED_SUMMARY)
        || packet
            .missing_evidence
            .iter()
            .chain(packet.conflicting_evidence.iter())
            .any(|issue| issue.detail != REDACTED_SUMMARY)
        || !fingerprint(&packet.evidence_fingerprint)
        || packet.approval.as_ref().is_some_and(|approval| {
            approval.evidence_fingerprint != packet.evidence_fingerprint
                || !fingerprint(&approval.action_fingerprint)
        })
    {
        return Err("Redacted assessment packet history is invalid".into());
    }
    // The clone verifies remaining structural invariants only. Never persist its
    // synthetic hashes or return it as an approvable/actionable packet.
    let mut structural = packet.clone();
    structural.evidence_fingerprint = structural.computed_evidence_fingerprint();
    let action_fingerprint = structural.action_fingerprint();
    if let Some(approval) = structural.approval.as_mut() {
        approval.evidence_fingerprint = structural.evidence_fingerprint.clone();
        approval.action_fingerprint = action_fingerprint;
    }
    structural.validate(workspace)
}

pub(crate) fn ensure_mutable(
    recommendation: &Recommendation,
) -> Result<(), super::store::StoreError> {
    if recommendation
        .assessment
        .as_ref()
        .is_some_and(|provenance| provenance.access_redacted)
    {
        return Err(super::store::StoreError::InvalidTransition);
    }
    Ok(())
}

fn reconcile_known_workspaces(
    metadata_root: &std::path::Path,
    settings: &TaskmasterSettings,
    active_metadata_root: Option<&std::path::Path>,
) -> Result<(), String> {
    let entries = match std::fs::read_dir(metadata_root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err("Known workspace metadata is unavailable".into()),
    };
    for entry in entries {
        let entry = entry.map_err(|_| "Known workspace metadata is unavailable")?;
        if !entry
            .file_type()
            .map_err(|_| "Known workspace metadata is unavailable")?
            .is_dir()
        {
            continue;
        }
        let root = entry.path();
        if active_metadata_root == Some(root.as_path()) {
            // The active workspace is reconciled separately under its lock.
            continue;
        }
        let Some(origin) = crate::workspace_runtime::read_origin_file(&root) else {
            if root.join("assessment.json").exists() {
                return Err("Assessment workspace identity is unavailable".into());
            }
            continue;
        };
        let key = origin
            .canonical_path
            .to_str()
            .ok_or("Assessment workspace identity is unavailable")?;
        // Origin is already canonical. Keep deleted/unopened workspaces in the
        // privacy reconciliation rather than requiring their repository to exist.
        let effective = super::runtime::effective_settings(settings, key);
        reconcile_metadata(&root, &effective)?;
    }
    Ok(())
}

/// Called under the runtime persistence guard before new settings are durable.
pub(crate) fn reconcile_settings(
    state: &std::sync::Arc<crate::AppState>,
    settings: &TaskmasterSettings,
) -> Result<(), String> {
    // Keep selection stable through the global scan and active pass, and
    // serialize both against delivery completion and recommendation writes.
    let workspace = state
        .workspace
        .lock()
        .map_err(|_| "Workspace state is unavailable")?;
    #[cfg(not(test))]
    if let Some(root) = crate::workspace_runtime::orkworks_workspaces_root() {
        let active_root = workspace
            .as_ref()
            .map(|workspace| workspace.metadata.root_path());
        reconcile_known_workspaces(&root, settings, active_root.as_deref())?;
    }
    if let Some(workspace) = workspace.as_ref() {
        let key = workspace
            .path
            .to_str()
            .ok_or("Workspace identity is unavailable")?;
        reconcile_metadata(
            &workspace.metadata.root_path(),
            &super::runtime::effective_settings(settings, key),
        )?;
    }
    Ok(())
}

pub(crate) fn runtime_for(state: &crate::AppState) -> super::runtime::TaskmasterRuntime {
    #[cfg(test)]
    let root = state
        .workspace
        .lock()
        .expect("workspace lock poisoned")
        .as_ref()
        .map(|workspace| workspace.metadata.root_path().join("taskmaster"))
        .unwrap_or_else(|| std::env::temp_dir().join("orkworks-taskmaster-test"));
    #[cfg(not(test))]
    let root = {
        let _ = state;
        super::runtime::taskmaster_global_dir()
            .unwrap_or_else(|| std::env::temp_dir().join("orkworks-taskmaster"))
    };
    super::runtime::TaskmasterRuntime::open(root)
}

fn project_workspace(
    workspace: &crate::WorkspaceState,
    settings: Option<&TaskmasterSettings>,
    fingerprint: Option<&str>,
    mutating: bool,
) -> Result<Vec<Recommendation>, super::store::StoreError> {
    let record = super::assessment_store::AssessmentStore::new(&workspace.metadata.root_path())
        .load::<AssessmentRecord>();
    let mut visible = Vec::new();
    for mut recommendation in workspace.recommendation_store.list()? {
        let original = recommendation.clone();
        if let Some(settings) = settings {
            redact_recommendation(&mut recommendation, settings);
        }
        let pending_id = record
            .as_ref()
            .ok()
            .and_then(|record| record.as_ref())
            .and_then(|record| record.pending_recommendation.as_ref())
            .map(|pending| pending.id.as_str());
        let allowed = if pending_id == Some(recommendation.id.as_str()) {
            false
        } else if recommendation.assessment.is_some() && (record.is_err() || settings.is_none()) {
            false
        } else {
            let proven_invalid = fingerprint.is_some()
                || recommendation
                    .assessment
                    .as_ref()
                    .is_some_and(|provenance| provenance.access_redacted)
                || record
                    .as_ref()
                    .ok()
                    .and_then(|record| record.as_ref())
                    .and_then(|record| record.report.as_ref())
                    .is_some_and(|report| {
                        report.workspace_path != workspace.path.to_string_lossy()
                            || settings.is_some_and(|settings| {
                                settings.context_level != report.context_level
                                    || settings.excluded_paths != report.excluded_paths
                                    || settings.selection.as_ref().is_none_or(|selection| {
                                        selection.provider != report.provider
                                            || selection.model != report.model
                                    })
                            })
                    });
            fence_recommendation(
                &mut recommendation,
                record.as_ref().ok().and_then(|record| record.as_ref()),
                fingerprint,
                mutating && proven_invalid,
            )
        };
        if mutating && recommendation != original {
            workspace.recommendation_store.put(&recommendation)?;
        }
        if allowed {
            visible.push(recommendation);
        }
    }
    Ok(visible)
}

/// Keeps current inference/configuration authority through workspace reads and
/// mutations. Inference unavailability hides active assessments while ordinary
/// observation recommendations remain usable.
pub(crate) fn with_recommendations<T, E>(
    state: &std::sync::Arc<crate::AppState>,
    mutating: bool,
    store_error: fn(super::store::StoreError) -> E,
    action: impl FnOnce(&crate::WorkspaceState, Vec<Recommendation>) -> Result<T, E>,
) -> Result<Option<T>, E> {
    use super::store::StoreError;
    let (path, instance, derived_id) = {
        let workspace = state.workspace.lock().expect("workspace lock poisoned");
        let Some(workspace) = workspace.as_ref() else {
            return Ok(None);
        };
        let record = super::assessment_store::AssessmentStore::new(&workspace.metadata.root_path())
            .load::<AssessmentRecord>();
        let recommendations = workspace.recommendation_store.list().map_err(store_error)?;
        if recommendations.iter().all(|item| item.assessment.is_none())
            && record.as_ref().is_ok_and(|record| {
                record
                    .as_ref()
                    .is_none_or(|record| record.pending_recommendation.is_none())
            })
        {
            return action(workspace, recommendations).map(Some);
        }
        (
            workspace.path.clone(),
            workspace.workflow_observations.instance_id(),
            record
                .ok()
                .flatten()
                .and_then(|record| record.report)
                .and_then(|report| report.recommendation_id),
        )
    };
    let runtime = runtime_for(state);
    let snapshot = super::assessment::capture_current_snapshot(
        state,
        runtime.root_path(),
        derived_id.as_deref(),
    );
    let mut action = Some(action);
    let mut result = None;
    if let Ok(snapshot) = snapshot {
        let _ = runtime.with_current_evaluation(
            &state.harness_store,
            &path,
            &snapshot.evaluation,
            || {
                let workspace = state.workspace.lock().expect("workspace lock poisoned");
                let Some(workspace) = workspace.as_ref().filter(|workspace| {
                    workspace.path == path
                        && workspace.workflow_observations.instance_id() == instance
                }) else {
                    return;
                };
                let fingerprint = super::assessment::capture_workspace_input(
                    workspace,
                    &snapshot.evaluation,
                    derived_id.as_deref(),
                )
                .ok()
                .map(|input| input.fingerprint);
                let projected = project_workspace(
                    workspace,
                    Some(&snapshot.evaluation.settings),
                    fingerprint.as_deref(),
                    mutating,
                );
                result = Some(projected.map_err(store_error).and_then(|recommendations| {
                    action.take().expect("single consumer action")(workspace, recommendations)
                }));
            },
        );
    }
    if let Some(result) = result {
        return result.map(Some);
    }
    let guarded = runtime.with_workspace_settings(&path, |settings| {
        let workspace = state.workspace.lock().expect("workspace lock poisoned");
        let Some(workspace) = workspace.as_ref().filter(|workspace| {
            workspace.path == path && workspace.workflow_observations.instance_id() == instance
        }) else {
            return None;
        };
        Some(
            project_workspace(workspace, Some(settings), None, mutating)
                .map_err(store_error)
                .and_then(|recommendations| {
                    action.take().expect("single consumer action")(workspace, recommendations)
                }),
        )
    });
    match guarded {
        Ok(Some(result)) => result.map(Some),
        Ok(None) => Ok(None),
        Err(_) => {
            let workspace = state.workspace.lock().map_err(|_| {
                store_error(StoreError::Recovery(
                    "Workspace state is unavailable".into(),
                ))
            })?;
            let Some(workspace) = workspace.as_ref().filter(|workspace| {
                workspace.path == path && workspace.workflow_observations.instance_id() == instance
            }) else {
                return Ok(None);
            };
            let ordinary = workspace
                .recommendation_store
                .list()
                .map_err(store_error)?
                .into_iter()
                .filter(|item| item.assessment.is_none())
                .collect();
            action.take().expect("single consumer action")(workspace, ordinary).map(Some)
        }
    }
}

pub(crate) fn status_for(
    state: &std::sync::Arc<crate::AppState>,
    runtime: &super::runtime::TaskmasterRuntime,
) -> serde_json::Value {
    use super::assessment_report::AssessmentState;
    let unavailable = || serde_json::json!({"state":"unavailable","report":null,"errorSummary":"Assessment report is unavailable or its inputs have changed.","reused":false});
    let record = {
        let workspace = state.workspace.lock().expect("workspace lock poisoned");
        let Some(workspace) = workspace.as_ref() else {
            return unavailable();
        };
        super::assessment_store::AssessmentStore::new(&workspace.metadata.root_path())
            .load::<AssessmentRecord>()
    };
    let record = match record {
        Ok(None) => {
            return serde_json::json!({"state":"idle","report":null,"errorSummary":null,"reused":false})
        }
        Ok(Some(record)) => record,
        Err(_) => return unavailable(),
    };
    if record.state == AssessmentState::PendingRecommendation
        || record.pending_recommendation.is_some()
    {
        return unavailable();
    }
    if record.state != AssessmentState::Succeeded {
        let state = match record.state {
            AssessmentState::Queued => "queued",
            AssessmentState::Running => "running",
            AssessmentState::Failed => "failed",
            AssessmentState::Interrupted => "interrupted",
            _ => return unavailable(),
        };
        let error = match record.state {
            AssessmentState::Failed => {
                Some("Assessment failed. Check Taskmaster settings and try again.")
            }
            AssessmentState::Interrupted => Some("Assessment was interrupted before completion."),
            _ => None,
        };
        return serde_json::json!({"state":state,"report":null,"errorSummary":error,"reused":false});
    }
    let Some(report) = record
        .report
        .as_ref()
        .filter(|report| report.id == record.id && display_report_valid(report))
    else {
        return unavailable();
    };
    let Ok(snapshot) = super::assessment::capture_current_snapshot(
        state,
        runtime.root_path(),
        report.recommendation_id.as_deref(),
    ) else {
        return unavailable();
    };
    let mut projected = None;
    let _ = runtime.with_current_evaluation(&state.harness_store, &snapshot.workspace_path, &snapshot.evaluation, || {
        let workspace = state.workspace.lock().expect("workspace lock poisoned");
        let Some(workspace) = workspace.as_ref().filter(|workspace| workspace.path == snapshot.workspace_path && workspace.workflow_observations.instance_id() == snapshot.workspace_instance) else { return; };
        let current_record = super::assessment_store::AssessmentStore::new(&workspace.metadata.root_path()).load::<AssessmentRecord>().ok().flatten();
        if current_record.as_ref() != Some(&record) { return; }
        let Ok(current) = super::assessment::capture_workspace_input(workspace, &snapshot.evaluation, report.recommendation_id.as_deref()) else { return; };
        if report.workspace_path != workspace.path.to_string_lossy() || current.fingerprint != report.input_fingerprint { return; }
        projected = Some(serde_json::json!({
            "state":"succeeded","errorSummary":null,"reused":record.reused,
            "report":{
                "id":report.id,"workspacePath":report.workspace_path,"observedAt":report.observed_at,
                "provider":report.provider,"model":report.model,"bundleVersion":report.bundle_version,
                "outcome":report.outcome,"summary":report.summary,"noProposalReason":report.no_proposal_reason,
                "unverifiedChecks":report.unverified_checks,"repositoryEvidence":report.repository_evidence,
                "knowledgeEvidence":report.knowledge_evidence,"recommendationId":report.recommendation_id,
            }
        }));
    });
    projected.unwrap_or_else(unavailable)
}

fn display_report_valid(report: &super::assessment_report::AssessmentReport) -> bool {
    let text = |value: &str, max| {
        !value.trim().is_empty()
            && value.chars().count() <= max
            && !value.chars().any(char::is_control)
    };
    text(&report.summary, 1000)
        && report.repository_evidence.len() <= 32
        && report.knowledge_evidence.len() <= 8
        && report.unverified_checks.len() <= 4
        && report
            .unverified_checks
            .iter()
            .all(|check| text(check, 200))
        && match report.outcome.as_str() {
            "proposed" => {
                report.no_proposal_reason.is_none()
                    && !report.repository_evidence.is_empty()
                    && !report.knowledge_evidence.is_empty()
                    && report.proposal.as_ref().is_some_and(|proposal| {
                        text(&proposal.title, 240) && text(&proposal.summary, 1000)
                    })
            }
            "no_proposal" => {
                report.proposal.is_none()
                    && report
                        .no_proposal_reason
                        .as_ref()
                        .is_some_and(|reason| text(reason, 1000))
            }
            _ => false,
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::taskmaster::{assessment_report::tests::proposal, RecommendationStatus};

    #[test]
    fn narrowing_detects_levels_and_new_prefixes_without_treating_expansion_as_narrowing() {
        let mut settings = TaskmasterSettings::default();
        assert!(access_narrowed(ContextLevel::SourceCode, &[], &settings));
        assert!(!access_narrowed(
            ContextLevel::SessionObservations,
            &[],
            &settings
        ));
        settings.excluded_paths = vec!["private/docs".into()];
        assert!(access_narrowed(
            ContextLevel::WorkflowContext,
            &[],
            &settings
        ));
        assert!(!access_narrowed(
            ContextLevel::WorkflowContext,
            &["private".into()],
            &settings
        ));
        assert!(access_narrowed(
            ContextLevel::WorkflowContext,
            &["private/docs".into()],
            &TaskmasterSettings {
                excluded_paths: vec!["private".into()],
                ..settings
            }
        ));
    }

    #[test]
    fn access_reduction_redacts_model_prose_in_every_retaining_lifecycle_state() {
        let (_, original) = proposal();
        let settings = TaskmasterSettings {
            context_level: ContextLevel::SessionObservations,
            ..TaskmasterSettings::default()
        };
        for status in [
            RecommendationStatus::Proposed,
            RecommendationStatus::Accepted,
            RecommendationStatus::Dismissed,
            RecommendationStatus::Completed,
            RecommendationStatus::Superseded,
        ] {
            let mut recommendation = original.clone();
            recommendation.status = status.clone();
            recommendation.suggested_prompt = Some("source-derived private instructions".into());
            redact_recommendation(&mut recommendation, &settings);
            assert!(recommendation.repository_evidence.is_empty());
            assert_eq!(recommendation.title, "Assessment evidence access changed");
            assert_eq!(
                recommendation.summary,
                "Assessment evidence was redacted after context access changed."
            );
            assert!(recommendation.reason.is_empty());
            assert!(recommendation.suggested_prompt.is_none());
            assert!(recommendation
                .workflow_improvement
                .proposed_improvement
                .is_empty());
            assert!(recommendation
                .workflow_improvement
                .expected_benefit
                .is_empty());
            assert_eq!(recommendation.id, original.id);
            assert_eq!(recommendation.status, status);
            assert_eq!(recommendation.created_at, original.created_at);
        }
    }

    #[test]
    fn pending_report_redaction_includes_the_embedded_recommendation_and_checks() {
        let (mut record, _) = proposal();
        record
            .report
            .as_mut()
            .unwrap()
            .unverified_checks
            .push("Check private architecture".into());
        redact_record(
            &mut record,
            &TaskmasterSettings {
                context_level: ContextLevel::SessionObservations,
                ..TaskmasterSettings::default()
            },
        );
        let report = record.report.unwrap();
        assert!(report.repository_evidence.is_empty());
        assert!(report.unverified_checks.is_empty());
        assert!(report.proposal.is_none());
        assert_eq!(
            report.summary,
            "Assessment evidence was redacted after context access changed."
        );
        assert!(record
            .pending_recommendation
            .unwrap()
            .repository_evidence
            .is_empty());
        assert_eq!(report.id, "assessment-1");
        assert_eq!(
            report.recommendation_id.as_deref(),
            Some("recommendation-1")
        );
    }
    #[test]
    fn persisted_access_reduction_reconciles_unopened_workspace_reports_and_terminal_recommendations(
    ) {
        let directory = tempfile::tempdir().unwrap();
        let (mut record, mut recommendation) = proposal();
        recommendation.status = RecommendationStatus::Dismissed;
        record.pending_recommendation = None;
        let store =
            crate::taskmaster::store::RecommendationStore::open(directory.path().into()).unwrap();
        store.put(&recommendation).unwrap();
        let report_store =
            crate::taskmaster::assessment_store::AssessmentStore::new(directory.path());
        report_store.save(&record).unwrap();
        let settings = TaskmasterSettings {
            excluded_paths: vec!["README.md".into()],
            ..TaskmasterSettings::default()
        };
        reconcile_metadata(directory.path(), &settings).unwrap();
        let saved = store.get(&recommendation.id).unwrap().unwrap();
        assert!(saved.repository_evidence.is_empty());
        assert_eq!(saved.status, RecommendationStatus::Dismissed);
        assert_eq!(saved.summary, REDACTED_SUMMARY);
        assert!(report_store
            .load::<AssessmentRecord>()
            .unwrap()
            .unwrap()
            .report
            .unwrap()
            .repository_evidence
            .is_empty());
        reconcile_metadata(directory.path(), &settings).unwrap();
        assert_eq!(store.get(&recommendation.id).unwrap().unwrap(), saved);
    }

    #[test]
    fn pending_and_stale_assessment_consumers_fail_closed_without_status_polling() {
        use crate::taskmaster::assessment_report::AssessmentState;
        let (mut record, recommendation) = proposal();
        record.state = AssessmentState::PendingRecommendation;
        let mut pending = recommendation.clone();
        assert!(!fence_recommendation(
            &mut pending,
            Some(&record),
            Some("fingerprint"),
            false
        ));
        assert_eq!(pending.status, RecommendationStatus::Proposed);
        record.state = AssessmentState::Succeeded;
        record.pending_recommendation = None;
        assert!(fence_recommendation(
            &mut pending,
            Some(&record),
            Some("fingerprint"),
            false
        ));
        assert!(!fence_recommendation(
            &mut pending,
            Some(&record),
            Some("changed"),
            false
        ));
        assert_eq!(pending.status, RecommendationStatus::Proposed);
        assert!(!fence_recommendation(
            &mut pending,
            Some(&record),
            Some("changed"),
            true
        ));
        assert_eq!(pending.status, RecommendationStatus::Superseded);
        assert_eq!(
            pending.repository_evidence,
            recommendation.repository_evidence
        );
        let mut accepted = recommendation.clone();
        accepted.status = RecommendationStatus::Accepted;
        assert!(!fence_recommendation(
            &mut accepted,
            Some(&record),
            Some("changed"),
            true
        ));
        assert_eq!(accepted.status, RecommendationStatus::Accepted);
        let mut ordinary = recommendation.clone();
        ordinary.assessment = None;
        assert!(fence_recommendation(&mut ordinary, None, None, true));
    }

    #[test]
    fn redacted_completion_packets_preserve_audit_history_but_cannot_be_mutated_or_handed_off() {
        use crate::taskmaster::completion::{CompletionApproval, CompletionMutationRequest};
        for status in [
            RecommendationStatus::Proposed,
            RecommendationStatus::Accepted,
            RecommendationStatus::Completed,
        ] {
            let (_, mut recommendation) = proposal();
            let mut packet = crate::taskmaster::completion_tests::test_packet();
            recommendation.workspace_id = packet.subject.workspace_id.clone();
            recommendation.source_session_ids = vec![packet.source_session_id.clone()];
            recommendation.status = status.clone();
            recommendation.target_session_id = Some("target-session".into());
            if status != RecommendationStatus::Proposed {
                packet.approval = Some(CompletionApproval {
                    approved_at: "now".into(),
                    approver: "user".into(),
                    revision: packet.revision,
                    evidence_fingerprint: packet.evidence_fingerprint.clone(),
                    action_fingerprint: packet.action_fingerprint(),
                    idempotency_key: "approval-1".into(),
                });
            }
            if status == RecommendationStatus::Completed {
                packet.completion_idempotency_key = Some("approval-1".into());
            }
            let original = packet.clone();
            recommendation.completion_packet = Some(packet);
            let directory = tempfile::tempdir().unwrap();
            let store =
                crate::taskmaster::store::RecommendationStore::open(directory.path().into())
                    .unwrap();
            store.put(&recommendation).unwrap();
            let settings = TaskmasterSettings {
                context_level: ContextLevel::SessionObservations,
                ..TaskmasterSettings::default()
            };
            redact_recommendation(&mut recommendation, &settings);
            let packet = recommendation.completion_packet.as_ref().unwrap();
            assert_eq!(packet.action.prompt, REDACTED_SUMMARY);
            assert_eq!(packet.action.scope, REDACTED_SUMMARY);
            assert_eq!(packet.subject.scope, REDACTED_SUMMARY);
            assert!(packet.subject.changed_paths.is_empty());
            assert_eq!(packet.packet_id, original.packet_id);
            assert_eq!(packet.revision, original.revision);
            assert_eq!(packet.approval, original.approval);
            assert_eq!(packet.evidence_fingerprint, original.evidence_fingerprint);
            assert_eq!(
                packet.completion_idempotency_key,
                original.completion_idempotency_key
            );
            let redacted_packet = packet.clone();
            let mutation = CompletionMutationRequest {
                packet_revision: original.revision,
                evidence_fingerprint: original.evidence_fingerprint.clone(),
                idempotency_key: "approval-1".into(),
            };
            store.put(&recommendation).unwrap();
            assert_eq!(
                store.get(&recommendation.id).unwrap().unwrap(),
                recommendation
            );
            assert!(store
                .begin_execution(&recommendation.id, "target".into(), "now".into())
                .is_err());
            assert!(store
                .begin_execution_checked(
                    &recommendation.id,
                    "target".into(),
                    "now".into(),
                    &mutation,
                    "user".into()
                )
                .is_err());
            let completed =
                store.complete_accepted_checked(&recommendation.id, "now".into(), &mutation);
            if status == RecommendationStatus::Accepted {
                let completed = completed.unwrap().unwrap();
                assert_eq!(completed.status, RecommendationStatus::Completed);
                let packet = completed.completion_packet.as_ref().unwrap();
                assert_eq!(packet.approval, original.approval);
                assert_eq!(
                    packet.completion_idempotency_key.as_deref(),
                    Some("approval-1")
                );
                assert_eq!(packet.action.prompt, REDACTED_SUMMARY);
            } else if status == RecommendationStatus::Completed {
                let completed = completed.unwrap().unwrap();
                assert_eq!(completed.status, RecommendationStatus::Completed);
                assert_eq!(completed.completion_packet, Some(redacted_packet.clone()));
            } else {
                assert!(completed.is_err());
            }
            assert!(store
                .replace_completion_packet(
                    &recommendation.id,
                    original.revision,
                    original,
                    "now".into()
                )
                .is_err());
        }
    }

    #[test]
    fn redacted_accepted_assessment_can_record_delivery_completion() {
        let (_, mut recommendation) = proposal();
        recommendation.status = RecommendationStatus::Accepted;
        recommendation.target_session_id = Some("target-session".into());
        let directory = tempfile::tempdir().unwrap();
        let store =
            crate::taskmaster::store::RecommendationStore::open(directory.path().into()).unwrap();
        store.put(&recommendation).unwrap();

        redact_recommendation(
            &mut recommendation,
            &TaskmasterSettings {
                context_level: ContextLevel::SessionObservations,
                ..TaskmasterSettings::default()
            },
        );
        store.put(&recommendation).unwrap();

        let completed = store
            .complete_accepted(&recommendation.id, "completed-at".into())
            .unwrap()
            .unwrap();

        assert_eq!(completed.status, RecommendationStatus::Completed);
        assert!(completed.assessment.unwrap().access_redacted);
        assert_eq!(completed.title, "Assessment evidence access changed");
    }

    #[test]
    fn global_reduction_reconciles_every_known_workspace_while_override_is_scoped() {
        use crate::taskmaster::runtime::WorkspaceOverride;
        let root = tempfile::tempdir().unwrap();
        let workspace_a = tempfile::tempdir().unwrap();
        let workspace_b = tempfile::tempdir().unwrap();
        let mut stores = Vec::new();
        for (name, workspace) in [("a", workspace_a.path()), ("b", workspace_b.path())] {
            let metadata = root.path().join(name);
            std::fs::create_dir(&metadata).unwrap();
            crate::workspace_runtime::write_origin_file_if_absent(
                &metadata,
                &workspace.canonicalize().unwrap(),
            );
            let (_, recommendation) = proposal();
            let store = crate::taskmaster::store::RecommendationStore::open(metadata).unwrap();
            store.put(&recommendation).unwrap();
            stores.push(store);
        }
        let mut scoped = TaskmasterSettings::default();
        scoped.workspace_overrides.insert(
            workspace_a
                .path()
                .canonicalize()
                .unwrap()
                .display()
                .to_string(),
            WorkspaceOverride {
                context_level: Some(ContextLevel::SessionObservations),
                ..WorkspaceOverride::default()
            },
        );
        reconcile_known_workspaces(root.path(), &scoped, None).unwrap();
        assert!(stores[0].list().unwrap()[0].repository_evidence.is_empty());
        assert!(!stores[1].list().unwrap()[0].repository_evidence.is_empty());
        scoped.context_level = ContextLevel::SessionObservations;
        let active_root = root.path().join("b");
        reconcile_known_workspaces(root.path(), &scoped, Some(&active_root)).unwrap();
        assert!(
            !stores[1].list().unwrap()[0].repository_evidence.is_empty(),
            "the global pass must not read/replace the active recommendation"
        );
        let mut delivered = stores[1].list().unwrap().remove(0);
        delivered.status = RecommendationStatus::Accepted;
        stores[1].put(&delivered).unwrap();
        // The active pass observes the delivered lifecycle under the workspace
        // lock rather than applying a pre-delivery clone from the global scan.
        reconcile_metadata(&active_root, &scoped).unwrap();
        let final_record = stores[1].list().unwrap().remove(0);
        assert_eq!(final_record.status, RecommendationStatus::Accepted);
        assert!(final_record.repository_evidence.is_empty());
    }

    #[test]
    fn application_list_and_detail_hide_pending_assessments_without_writing_the_transaction() {
        let directory = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(directory.path());
        let (mut record, recommendation) = proposal();
        record.state = crate::taskmaster::assessment_report::AssessmentState::PendingRecommendation;
        let root = {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            workspace.recommendation_store.put(&recommendation).unwrap();
            workspace.metadata.root_path()
        };
        crate::taskmaster::assessment_store::AssessmentStore::new(&root)
            .save(&record)
            .unwrap();
        let before = std::fs::read(root.join("assessment.json")).unwrap();
        let application = crate::session_application::SessionApplication::new(state);
        assert!(application.list_recommendations().unwrap().0.is_empty());
        assert!(application
            .get_recommendation(&recommendation.id)
            .unwrap()
            .is_none());
        assert_eq!(std::fs::read(root.join("assessment.json")).unwrap(), before);
    }

    #[tokio::test]
    async fn application_stale_acceptance_fails_before_target_lookup_and_supersedes_proposed_only()
    {
        let directory = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(directory.path());
        let (mut record, recommendation) = proposal();
        record.pending_recommendation = None;
        {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            workspace.recommendation_store.put(&recommendation).unwrap();
            crate::taskmaster::assessment_store::AssessmentStore::new(
                &workspace.metadata.root_path(),
            )
            .save(&record)
            .unwrap();
        }
        let application = crate::session_application::SessionApplication::new(state.clone());
        assert!(application
            .accept_recommendation_with_packet(&recommendation.id, "missing-target", None, None)
            .await
            .unwrap()
            .is_none());
        let workspace = state.workspace.lock().unwrap();
        let saved = workspace
            .as_ref()
            .unwrap()
            .recommendation_store
            .get(&recommendation.id)
            .unwrap()
            .unwrap();
        assert_eq!(saved.status, RecommendationStatus::Superseded);
        assert_eq!(
            saved.repository_evidence,
            recommendation.repository_evidence
        );
    }

    #[test]
    fn assessment_status_hides_pending_and_unverifiable_reports_without_rewriting_storage() {
        let directory = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(directory.path());
        let (mut record, _) = proposal();
        let root = state
            .workspace
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .metadata
            .root_path();
        let store = crate::taskmaster::assessment_store::AssessmentStore::new(&root);
        for pending in [true, false] {
            record.state = if pending {
                crate::taskmaster::assessment_report::AssessmentState::PendingRecommendation
            } else {
                crate::taskmaster::assessment_report::AssessmentState::Succeeded
            };
            if !pending {
                record.pending_recommendation = None;
            }
            store.save(&record).unwrap();
            let before = std::fs::read(root.join("assessment.json")).unwrap();
            let status = status_for(&state, &runtime_for(&state));
            assert_eq!(status["state"], "unavailable");
            assert!(status["report"].is_null());
            assert!(!status.to_string().contains("Grounded hypothesis"));
            assert_eq!(std::fs::read(root.join("assessment.json")).unwrap(), before);
        }
    }

    #[test]
    fn workspace_open_recovers_interrupted_assessment_before_returning() {
        let home = tempfile::tempdir().unwrap();
        crate::test_support::with_fake_home(home.path(), || {
            let directory = tempfile::tempdir().unwrap();
            let state = crate::test_support::test_app_state_with_workspace(directory.path());
            let application = crate::session_application::SessionApplication::new(state.clone());
            application.open_workspace(directory.path().into()).unwrap();
            let root = state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .root_path();
            let (mut record, _) = proposal();
            record.state = crate::taskmaster::assessment_report::AssessmentState::Queued;
            record.pending_recommendation = None;
            record.report = None;
            let store = crate::taskmaster::assessment_store::AssessmentStore::new(&root);
            store.save(&record).unwrap();
            state.workspace.lock().unwrap().take();
            application.open_workspace(directory.path().into()).unwrap();
            assert_eq!(
                store.load::<AssessmentRecord>().unwrap().unwrap().state,
                crate::taskmaster::assessment_report::AssessmentState::Interrupted
            );
        });
    }
    #[test]
    fn active_analysis_admission_reconciles_a_proven_stale_assessment_before_provider_gates() {
        let directory = tempfile::tempdir().unwrap();
        let state = crate::test_support::test_app_state_with_workspace(directory.path());
        let (mut record, recommendation) = proposal();
        record.pending_recommendation = None;
        {
            let workspace = state.workspace.lock().unwrap();
            let workspace = workspace.as_ref().unwrap();
            workspace.recommendation_store.put(&recommendation).unwrap();
            crate::taskmaster::assessment_store::AssessmentStore::new(
                &workspace.metadata.root_path(),
            )
            .save(&record)
            .unwrap();
        }
        let application = crate::session_application::SessionApplication::new(state.clone());
        assert!(application
            .list_recommendations_for_action()
            .unwrap()
            .0
            .is_empty());
        let workspace = state.workspace.lock().unwrap();
        assert_eq!(
            workspace
                .as_ref()
                .unwrap()
                .recommendation_store
                .get(&recommendation.id)
                .unwrap()
                .unwrap()
                .status,
            RecommendationStatus::Superseded
        );
    }
}
