//! Optional readiness-history projection of the same validated result.
use super::{
    access::Capture,
    store::{Record, Report},
};
use crate::taskmaster::{
    readiness_store::{
        self, Collector, GitIdentity, ImprovementHistoryExcerpt, ImprovementHistoryInput,
        InputIdentity, Manifest, SavedFact, Snapshot, SnapshotData, StoredRow,
    },
    KnowledgeEvidence, RepositoryEvidence,
};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

pub(crate) fn snapshot(
    capture: &Capture,
    record: &Record,
    report: &Report,
) -> Result<Snapshot, String> {
    super::publication::current_report(report, capture)?;
    let id = record
        .id
        .strip_prefix("improvement-")
        .ok_or("Invalid improvement run ID")?;
    uuid::Uuid::parse_str(id).map_err(|_| "Invalid improvement run ID")?;
    let completed_at = record
        .completed_at
        .clone()
        .ok_or("Improvement completion is unavailable")?;
    let facts = report
        .repository_evidence
        .iter()
        .map(|fact| RepositoryEvidence {
            path: fact.path.clone(),
            sha256: fact.sha256.clone(),
            excerpt: fact.text.chars().take(500).collect(),
            observed_at: completed_at.clone(),
        })
        .collect::<Vec<_>>();
    let saved = facts
        .iter()
        .map(|fact| {
            Ok(SavedFact {
                fact_id: readiness_store::fact_id(fact)?,
                fact: fact.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut rows = Vec::new();
    for row in &report.result.concepts {
        let hashes = row
            .repository_fact_ids
            .iter()
            .map(|index| {
                let range = report
                    .manifest
                    .ranges
                    .get(*index)
                    .ok_or("Improvement fact unavailable")?;
                saved
                    .iter()
                    .find(|fact| fact.fact.path == range.path && fact.fact.sha256 == range.sha256)
                    .map(|fact| fact.fact_id.clone())
                    .ok_or_else(|| "Improvement snapshot unavailable".to_owned())
            })
            .collect::<Result<Vec<_>, String>>()?;
        rows.push(StoredRow {
            concept_id: row.concept_id.clone(),
            maturity: row.maturity,
            confidence: row.confidence.clone(),
            repository_fact_hashes: hashes,
            rubric_page_id: row.rubric_page_id.clone(),
            rationale: row.rationale.clone(),
            limits: row.limits.clone(),
            next_transition: row.next_transition.clone(),
            unassessed_reason: row.unassessed_reason.clone(),
        });
    }
    let bundle = capture
        .evaluation
        .knowledge
        .as_ref()
        .ok_or("Guidance unavailable")?;
    let ids = rows
        .iter()
        .map(|row| row.rubric_page_id.as_str())
        .collect::<HashSet<_>>();
    let knowledge = bundle
        .pages
        .iter()
        .filter(|page| ids.contains(page.id.as_str()))
        .map(|page| KnowledgeEvidence {
            page_id: page.id.clone(),
            title: page.title.clone(),
            status: page.status.clone(),
            bundle_version: bundle.version.clone(),
            sha256: page.sha256.clone(),
            excerpt: String::new(),
        })
        .collect::<Vec<_>>();
    if knowledge.len() != ids.len() {
        return Err("Rubric guidance unavailable".into());
    }
    let excerpts = facts
        .iter()
        .map(|fact| ImprovementHistoryExcerpt {
            path: fact.path.clone(),
            sha256: fact.sha256.clone(),
            excerpt_sha256: hex::encode(Sha256::digest(fact.excerpt.as_bytes())),
        })
        .collect();
    let input = ImprovementHistoryInput {
        version: 2,
        manifest: report.manifest.clone(),
        excerpts,
    };
    let identity = InputIdentity {
        workspace_path: report.manifest.workspace_path.clone(),
        workspace_instance: report.manifest.workspace_instance,
        git: GitIdentity {
            head_oid: None,
            dirty: None,
        },
        input_fingerprint: report.manifest.fingerprint()?,
        comparison_fingerprint: readiness_store::hash(&(
            "brain-improvement-v2",
            &report.manifest.evaluation_key,
            &report.manifest.rubric_digest,
            report.manifest.context_level,
            &report.manifest.excluded_paths,
        ))?,
        input_manifest: Manifest {
            version: 1,
            purpose: "brain-improvement-v2".into(),
            collector: Collector::Improvement(input),
            observations_digest: readiness_store::hash(&Vec::<String>::new())?,
        },
        inference_identity: serde_json::json!({"evaluationKey":report.manifest.evaluation_key}),
        knowledge_identity: serde_json::to_value(&capture.evaluation.knowledge_identity)
            .map_err(|_| "Guidance identity unavailable")?,
        rubric_digest: report.manifest.rubric_digest.clone(),
        versions: serde_json::json!({"purpose":"brain-improvement-v2","collector":2,"schema":1,"text":2}),
        context_level: report.manifest.context_level,
        excluded_paths: report.manifest.excluded_paths.clone(),
    };
    let (coverage, raw_total, overall_score) = report.result.readiness().totals();
    Ok(Snapshot {
        id: id.into(),
        started_at: record.started_at.clone(),
        completed_at,
        redacted: false,
        data: Some(SnapshotData {
            identity,
            summary: report.result.summary.clone(),
            concepts: rows,
            repository_evidence: saved,
            knowledge_evidence: knowledge,
            coverage,
            raw_total,
            overall_score,
        }),
    })
}
