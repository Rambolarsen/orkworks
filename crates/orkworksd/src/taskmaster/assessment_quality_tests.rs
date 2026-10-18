//! Explicit live-provider evaluation; never runs as part of the default suite.
use super::super::{
    assessment_report::{AssessmentRecord, AssessmentState},
    assessment_store::AssessmentStore,
    runtime::{ContextLevel, TaskmasterRuntime, TaskmasterSettings},
};
use super::*;
use std::fs;
thread_local! { static EVIDENCE_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) }; }
pub(super) fn capture_stage_output(prompt: &str, output: &str) {
    EVIDENCE_DIR.with(|dir| {
        if let Some(dir) = dir.borrow().as_ref() {
            let prompt: serde_json::Value = serde_json::from_str(prompt).unwrap();
            let stage = if prompt["protocol"] == "taskmaster-discovery-v1" {
                "discovery"
            } else {
                "final"
            };
            fs::write(dir.join(format!("{stage}-output.json")), output).unwrap();
        }
    });
}

#[test]
#[ignore = "Explicit selected-provider evaluation requires output directory and settings file"]
fn selected_provider_quality_matrix() {
    let settings_path =
        std::env::var("ORKWORKS_ASSESSMENT_SELECTION_FILE").expect("explicit selection file");
    let selected: TaskmasterSettings =
        serde_json::from_slice(&fs::read(settings_path).unwrap()).unwrap();
    let output = PathBuf::from(
        std::env::var("ORKWORKS_ASSESSMENT_QUALITY_DIR").expect("explicit evidence destination"),
    );
    fs::create_dir_all(&output).unwrap();
    let selection = selected.selection.expect("configured selected provider");
    fs::write(
        output.join("selection.json"),
        serde_json::to_vec_pretty(&selection).unwrap(),
    )
    .unwrap();
    for case in ["positive", "positive_old_noise", "healthy", "refuted"] {
        if std::env::var("ORKWORKS_ASSESSMENT_QUALITY_CASE").is_ok_and(|filter| filter != case) {
            continue;
        }
        for run in 1..=3 {
            let directory = output.join(format!("{case}-{run}"));
            assert!(
                !directory.exists(),
                "retain prior evidence; use a fresh evaluation directory"
            );
            let workspace = directory.join("workspace");
            fs::create_dir_all(workspace.join(".github/workflows")).unwrap();
            fs::create_dir_all(workspace.join("src")).unwrap();
            fs::write(workspace.join("AGENTS.md"), "# Workflow\nEvery change requires meaningful unit tests and CI must execute the unit suite. A passing no-op is not test evidence. Keep changes bounded and reviewable.\n").unwrap();
            fs::write(workspace.join("README.md"), "A small TypeScript library. Unit tests validate arithmetic behavior. CI should run the package test suite.\n").unwrap();
            fs::write(
                workspace.join("src/add.ts"),
                "export function add(a: number, b: number) { return a + b; }\n",
            )
            .unwrap();
            fs::write(workspace.join("src/add.test.ts"), "import { test } from 'node:test';\nimport assert from 'node:assert/strict';\nimport { add } from './add.ts';\ntest('adds two numbers', () => assert.equal(add(2, 3), 5));\n").unwrap();
            let script = match case {
                "healthy" => "node --experimental-strip-types --test src/add.test.ts",
                "refuted" => "node scripts/verify.ts",
                _ => "echo unit-tests-skipped",
            };
            fs::write(workspace.join("package.json"), serde_json::json!({"name":"assessment-fixture","private":true,"type":"module","packageManager":"pnpm@11.24.0","scripts":{"test":script}}).to_string()).unwrap();
            fs::write(workspace.join(".github/workflows/ci.yml"), "name: CI\non: [push, pull_request]\njobs:\n  unit:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: pnpm/action-setup@v4\n      - uses: actions/setup-node@v4\n        with:\n          node-version: 22\n      - run: pnpm install --frozen-lockfile\n      - run: pnpm test\n").unwrap();
            fs::write(workspace.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\nsettings:\n  autoInstallPeers: true\n  excludeLinksFromLockfile: false\nimporters:\n  .: {}\n").unwrap();
            if case == "refuted" {
                fs::create_dir_all(workspace.join("scripts")).unwrap();
                fs::write(workspace.join("scripts/verify.ts"), "// This entry point runs the real unit suite and propagates failures.\nimport { spawnSync } from 'node:child_process';\nconst result = spawnSync('pnpm', ['exec', 'node', '--experimental-strip-types', '--test', 'src/add.test.ts'], { stdio: 'inherit' });\nprocess.exit(result.status ?? 1);\n").unwrap();
            }
            let state = crate::test_support::test_app_state_with_workspace(&workspace);
            if case == "positive_old_noise" {
                use crate::workflow_observations::{
                    Impact, ObservationCandidate, ObservationKind, ObservationOrigin,
                };
                let guard = state.workspace.lock().unwrap();
                let store = &guard.as_ref().unwrap().workflow_observations;
                store.test_set_clock(
                    chrono::DateTime::parse_from_rfc3339("2026-08-01T00:00:00Z")
                        .unwrap()
                        .with_timezone(&chrono::Utc),
                );
                for n in 0..200 {
                    store
                        .record_observation(
                            &format!("old-session-{n}"),
                            ObservationOrigin::Peon,
                            &format!("noise-{n}"),
                            ObservationCandidate {
                                kind: ObservationKind::VerificationGap,
                                description: format!("Historical terminal sample {n}"),
                                evidence: "▘▝▚▞".into(),
                                problem_area: None,
                                reported_impact: Impact::Low,
                                confidence: Some(0.7),
                            },
                        )
                        .unwrap();
                }
            }
            let runtime_root = directory.join("runtime");
            let runtime = TaskmasterRuntime::open(runtime_root.clone());
            runtime
                .activate_knowledge(include_bytes!(
                    "../../../../apps/desktop/resources/knowledge/starter.json"
                ))
                .unwrap();
            let settings = TaskmasterSettings {
                selection: Some(selection.clone()),
                context_level: ContextLevel::SourceCode,
                ..Default::default()
            };
            runtime.replace_settings(settings).unwrap();
            let root = state
                .workspace
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .metadata
                .root_path();
            let record = AssessmentRecord {
                format_version: 2,
                input_manifest: None,
                phase: None,
                id: format!("quality-{case}-{run}"),
                state: AssessmentState::Queued,
                started_at: chrono::Utc::now().to_rfc3339(),
                completed_at: None,
                report: None,
                pending_recommendation: None,
                error_summary: None,
                reused: false,
            };
            let store = AssessmentStore::new(&root);
            store.save(&record).unwrap();
            let lease = runtime.try_analysis_lease().unwrap().unwrap();
            EVIDENCE_DIR.with(|dir| *dir.borrow_mut() = Some(directory.clone()));
            run_assessment(state, runtime_root, workspace, root, record, lease);
            EVIDENCE_DIR.with(|dir| *dir.borrow_mut() = None);
            let record = store.load::<AssessmentRecord>().unwrap().unwrap();
            fs::write(
                directory.join("result.json"),
                serde_json::to_vec_pretty(&record).unwrap(),
            )
            .unwrap();
            let proposed = record
                .report
                .as_ref()
                .is_some_and(|r| r.outcome == "proposed");
            eprintln!(
                "quality {case} run {run}: {:?}, proposed={proposed}, error={:?}",
                record.state, record.error_summary
            );
            if proposed {
                fs::write(directory.join("review-template.json"), serde_json::to_vec_pretty(&serde_json::json!({
                    "grounded":null,"designedGap":null,"bounded":null,"validation":null,"rollback":null,"rationale":"Review the surfaced proposal against the exact fixture facts. Save adjudication as review.json."
                })).unwrap()).unwrap();
            }
        }
    }
    check_retained_quality_evidence(&output);
}

fn check_retained_quality_evidence(output: &Path) {
    let mut failures = Vec::new();
    for case in ["positive", "positive_old_noise", "healthy", "refuted"] {
        let mut designed_gaps = 0;
        for run in 1..=3 {
            let directory = output.join(format!("{case}-{run}"));
            let record = fs::read(directory.join("result.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<AssessmentRecord>(&bytes).ok());
            let Some(record) = record else {
                failures.push(format!("{case}-{run}: missing result"));
                continue;
            };
            if record.reused || !directory.join("discovery-output.json").is_file() {
                failures.push(format!(
                    "{case}-{run}: no fresh completed provider dispatch"
                ));
            }
            if record.state != AssessmentState::Succeeded {
                failures.push(format!("{case}-{run}: did not complete"));
                continue;
            }
            if !record
                .report
                .as_ref()
                .is_some_and(|r| r.outcome == "proposed")
            {
                continue;
            }
            let review = fs::read(directory.join("review.json"))
                .ok()
                .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok());
            let Some(review) = review else {
                failures.push(format!("{case}-{run}: semantic adjudication required"));
                continue;
            };
            if ["grounded", "bounded", "validation", "rollback"]
                .iter()
                .any(|key| review[*key] != true)
                || review["rationale"]
                    .as_str()
                    .is_none_or(|s| s.trim().is_empty())
            {
                failures.push(format!("{case}-{run}: proposal not grounded and scoped"));
            }
            if review["designedGap"] == true {
                designed_gaps += 1;
            }
            if !case.starts_with("positive") {
                failures.push(format!("{case}-{run}: proposal on healthy/refuted fixture"));
            }
        }
        if case.starts_with("positive") && designed_gaps < 2 {
            failures.push(format!(
                "{case}: only {designed_gaps}/3 adjudicated designed gaps"
            ));
        }
    }
    fs::write(
        output.join("verdict.json"),
        serde_json::to_vec_pretty(&serde_json::json!({"failures":failures})).unwrap(),
    )
    .unwrap();
    assert!(
        failures.is_empty(),
        "quality threshold failed: {failures:?}"
    );
}

#[test]
#[ignore = "Explicit adjudication of previously retained synthetic evaluation evidence"]
fn quality_verdict_from_recorded_reviews() {
    let output = PathBuf::from(
        std::env::var("ORKWORKS_ASSESSMENT_QUALITY_DIR").expect("explicit evidence destination"),
    );
    check_retained_quality_evidence(&output);
}
