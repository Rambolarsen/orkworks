//! Captured inference authority and currentness for the new report purpose.
use super::store::{Manifest, PageIdentity};
use crate::taskmaster::{
    assessment_context::improvement::ProgressiveEvidence,
    readiness_rubric::ReadinessRubric,
    runtime::{ContextLevel, EvaluationSnapshot, TaskmasterRuntime},
};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

const IDENTITY_VERSION: &str = "brain-improvement-v2:collector-1:prompt-2:response-1:text-2";

#[derive(Clone)]
pub(crate) struct Capture {
    pub path: PathBuf,
    pub root: PathBuf,
    pub instance: u64,
    pub evaluation: EvaluationSnapshot,
    pub rubric: ReadinessRubric,
    pub evaluation_key: String,
    pub recommendation_digest: String,
}

fn recommendation_digest(
    workspace: &crate::WorkspaceState,
    exclude: Option<&str>,
) -> Result<String, String> {
    let mut records = workspace
        .recommendation_store
        .list()
        .map_err(|_| "Recommendations unavailable")?;
    records.retain(|item| Some(item.id.as_str()) != exclude);
    records.sort_by(|a, b| a.id.cmp(&b.id));
    crate::taskmaster::readiness_store::hash(&records)
}

pub(crate) fn capture(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    exclude: Option<&str>,
) -> Result<Capture, String> {
    let (path, root, instance) = {
        let lock = state
            .workspace
            .lock()
            .map_err(|_| "Workspace unavailable")?;
        let workspace = lock.as_ref().ok_or("Workspace unavailable")?;
        (
            workspace.path.clone(),
            workspace.metadata.root_path(),
            workspace.workflow_observations.instance_id(),
        )
    };
    let (mut evaluation, rubric) = runtime.improvement_evaluation_snapshot(&path)?;
    if evaluation.settings.context_level == ContextLevel::SessionObservations {
        return Err("Repository access unavailable".into());
    }
    let trust = crate::taskmaster::inference_trust::InferenceTrustStore::new(
        runtime.root_path().to_path_buf(),
    );
    let providers =
        crate::taskmaster::provider_catalog::inspect(&state.harness_store, Some(&trust))
            .map_err(|_| "Provider unavailable")?;
    let selection = evaluation
        .settings
        .selection
        .as_ref()
        .ok_or("Provider unavailable")?;
    if crate::taskmaster::provider_catalog::evaluation_availability(&providers, selection)
        != crate::taskmaster::provider_catalog::Availability::Ready
    {
        return Err("Provider unavailable".into());
    }
    let transport = providers
        .iter()
        .find(|provider| provider.id == selection.provider)
        .map(|provider| provider.transport.clone())
        .ok_or("Provider unavailable")?;
    if !crate::taskmaster::evaluator::bind_evaluation_transport(
        runtime,
        &state.harness_store,
        &path,
        &mut evaluation,
        transport,
    ) {
        return Err("Provider changed".into());
    }
    let evaluation_key = evaluation.cache_key(IDENTITY_VERSION)?;
    let lock = state
        .workspace
        .lock()
        .map_err(|_| "Workspace unavailable")?;
    let workspace = lock
        .as_ref()
        .filter(|w| {
            w.path == path
                && w.metadata.root_path() == root
                && w.workflow_observations.instance_id() == instance
        })
        .ok_or("Workspace changed")?;
    let recommendation_digest = recommendation_digest(workspace, exclude)?;
    Ok(Capture {
        path,
        root,
        instance,
        evaluation,
        rubric,
        evaluation_key,
        recommendation_digest,
    })
}

pub(crate) fn manifest(
    capture: &Capture,
    evidence: &ProgressiveEvidence,
    selected: &[String],
) -> Result<Manifest, String> {
    let bundle = capture
        .evaluation
        .knowledge
        .as_ref()
        .ok_or("Guidance unavailable")?;
    let pages = selected
        .iter()
        .map(|id| {
            let page = bundle
                .pages
                .iter()
                .find(|page| &page.id == id)
                .ok_or("Guidance changed")?;
            Ok(PageIdentity {
                id: page.id.clone(),
                sha256: page.sha256.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Manifest {
        purpose: "brain-improvement-v2".into(),
        workspace_path: capture.path.to_str().ok_or("Workspace unavailable")?.into(),
        workspace_instance: capture.instance,
        context_level: capture.evaluation.settings.context_level,
        excluded_paths: capture.evaluation.settings.excluded_paths.clone(),
        evaluation_key: capture.evaluation_key.clone(),
        rubric_digest: capture.rubric.digest()?,
        catalog_digest: evidence.catalog_digest().into(),
        ranges: evidence.range_manifest(),
        pages,
        recommendation_digest: capture.recommendation_digest.clone(),
    })
}

fn verify_inputs(
    manifest: &Manifest,
    path: &Path,
    level: ContextLevel,
    excluded: &[String],
    evaluation_key: &str,
    rubric_digest: &str,
    pages: &[PageIdentity],
    recommendation_digest: &str,
) -> Result<ProgressiveEvidence, String> {
    if manifest.purpose != "brain-improvement-v2"
        || manifest.workspace_path != path.to_str().ok_or("Workspace unavailable")?
        || manifest.context_level != level
        || manifest.excluded_paths != excluded
        || manifest.evaluation_key != evaluation_key
        || manifest.rubric_digest != rubric_digest
        || manifest.recommendation_digest != recommendation_digest
        || manifest.pages.iter().any(|saved| !pages.contains(saved))
    {
        return Err("Improvement inputs changed".into());
    }
    ProgressiveEvidence::restore(
        path,
        level,
        excluded,
        &crate::taskmaster::readiness_store::now(),
        &manifest.catalog_digest,
        &manifest.ranges,
    )
}

/// Called under the inference guard for custom dispatch, or by `guarded`.
pub(crate) fn guarded_workspace<T>(
    state: &Arc<crate::AppState>,
    capture: &Capture,
    manifest: Option<&Manifest>,
    exclude: Option<&str>,
    apply: impl FnOnce(&crate::WorkspaceState) -> Result<T, String>,
) -> Result<T, String> {
    let lock = state
        .workspace
        .lock()
        .map_err(|_| "Workspace unavailable")?;
    let workspace = lock
        .as_ref()
        .filter(|w| {
            w.path == capture.path
                && w.metadata.root_path() == capture.root
                && w.workflow_observations.instance_id() == capture.instance
        })
        .ok_or("Workspace changed")?;
    let digest = recommendation_digest(workspace, exclude)?;
    if digest != capture.recommendation_digest {
        return Err("Recommendations changed".into());
    }
    if let Some(manifest) = manifest {
        let bundle = capture
            .evaluation
            .knowledge
            .as_ref()
            .ok_or("Guidance unavailable")?;
        let pages = bundle
            .pages
            .iter()
            .map(|page| PageIdentity {
                id: page.id.clone(),
                sha256: page.sha256.clone(),
            })
            .collect::<Vec<_>>();
        verify_inputs(
            manifest,
            &capture.path,
            capture.evaluation.settings.context_level,
            &capture.evaluation.settings.excluded_paths,
            &capture.evaluation_key,
            &capture.rubric.digest()?,
            &pages,
            &digest,
        )?;
    }
    apply(workspace)
}

/// The callback runs while current provider/trust, workspace and all inputs are guarded.
pub(crate) fn guarded<T>(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    capture: &Capture,
    manifest: Option<&Manifest>,
    exclude: Option<&str>,
    apply: impl FnOnce(&crate::WorkspaceState) -> Result<T, String>,
) -> Result<T, String> {
    let mut action = Some(apply);
    let mut outcome = None;
    let current = runtime.with_current_evaluation(
        &state.harness_store,
        &capture.path,
        &capture.evaluation,
        || {
            outcome = Some(guarded_workspace(
                state,
                capture,
                manifest,
                exclude,
                action.take().expect("one guarded action"),
            ));
        },
    )?;
    if !current {
        return Err("Improvement authority changed".into());
    }
    outcome.ok_or("Improvement authority unavailable")?
}

pub(crate) fn supplied_manifest(
    capture: &Capture,
    evidence: &ProgressiveEvidence,
    selected: &[String],
) -> Result<Manifest, String> {
    manifest(capture, evidence, selected)
}

#[cfg(test)]
#[path = "access_tests.rs"]
mod tests;
