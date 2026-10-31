//! Opt-in selected-provider trials; fixture data and outputs stay in a private destination.
use super::*;
use crate::taskmaster::runtime::{ContextLevel, TaskmasterSettings};
use std::fs;

thread_local! {
    static EVIDENCE: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}
pub(super) fn capture_stage<T: std::fmt::Debug>(prompt: &str, result: &Result<String, T>) {
    EVIDENCE.with(|directory| {
        if let Some(directory) = directory.borrow().as_ref() {
            let value: Value = serde_json::from_str(prompt).unwrap();
            let stage = if value["protocol"] == "taskmaster-readiness-discovery-v2" {
                "discovery"
            } else {
                "final"
            };
            fs::write(directory.join(format!("{stage}-input.json")), prompt).unwrap();
            match result {
                Ok(output) => {
                    fs::write(directory.join(format!("{stage}-output.json")), output).unwrap()
                }
                Err(error) => fs::write(
                    directory.join(format!("{stage}-error.txt")),
                    format!("{error:?}"),
                )
                .unwrap(),
            }
        }
    });
}
fn fixture(path: &Path, improved: bool) {
    fs::create_dir_all(path.join("docs")).unwrap();
    fs::create_dir_all(path.join("src")).unwrap();
    fs::create_dir_all(path.join(".github/workflows")).unwrap();
    fs::write(path.join("AGENTS.md"), "# Catalog agent guide\nRead README.md for setup, docs/architecture.md for module boundaries and docs/domain.md for domain terms before changing catalog behavior. Use pnpm for packages. Changes need a small plan, focused tests and review before merge. Ask the maintainer before API changes, data deletion or publishing. Document lessons in docs/lessons.md. No autonomous deployment.\n").unwrap();
    fs::write(path.join("README.md"), "# Catalog library\nA TypeScript library that stores product inventory in memory. Node 22 and pnpm 11.24.0. Setup: pnpm install --frozen-lockfile. Verification: pnpm test. No services, network access or secrets required. Agent entry point: AGENTS.md. Architecture: docs/architecture.md. Domain glossary: docs/domain.md. No recorded verification results are available.\n").unwrap();
    fs::write(path.join("package.json"), json!({"name":"catalog-fixture","private":true,"type":"module","engines":{"node":"22.x"},"packageManager":"pnpm@11.24.0","scripts":{"test":"node --experimental-strip-types --test src/catalog.test.ts"}}).to_string()).unwrap();
    fs::write(path.join("pnpm-lock.yaml"), "lockfileVersion: '9.0'\nsettings:\n  autoInstallPeers: true\n  excludeLinksFromLockfile: false\nimporters:\n  .: {}\n").unwrap();
    fs::write(path.join(".node-version"), "22.20.0\n").unwrap();
    fs::write(path.join("docs/architecture.md"), "# Architecture\nThe public createCatalog factory in src/catalog.ts owns the in-memory product map. add validates the product identifier and stock count before storage. The catalog has no I/O boundary, database or deployment component. src/catalog.test.ts describes invalid input and duplicate behavior.\n").unwrap();
    fs::write(path.join("docs/domain.md"), if improved {
        "# Domain glossary\nA Product is a catalog entry with a unique nonempty SKU string and a nonnegative integer stock count. SKU means stock keeping unit; it is case-sensitive and is not a display label. A Catalog owns products keyed by SKU. Adding an existing SKU is an error, never an update. Stock is the count of individual available units, not currency or a reservation. Negative and fractional stock are invalid. src/catalog.ts add enforces these invariants and src/catalog.test.ts encodes the duplicate, negative and fractional cases. There is no persistence, order or reservation model.\n"
    } else {
        "# Domain glossary\nProducts have SKU and stock fields. Detailed term definitions and business invariants are not documented here yet.\n"
    }).unwrap();
    fs::write(path.join("docs/lessons.md"), "# Lessons\nNo completed task outcomes have been recorded. This file is the designated place for durable lessons; there is no automated feedback or curation process.\n").unwrap();
    fs::write(path.join("src/catalog.ts"), "export function createCatalog() {\n  const products = new Map<string, number>();\n  return { add(sku: string, stock: number) {\n    if (!sku || !Number.isInteger(stock) || stock < 0 || products.has(sku)) throw new Error('Invalid product');\n    products.set(sku, stock);\n  }, stock(sku: string) { return products.get(sku); } };\n}\n").unwrap();
    fs::write(path.join("src/catalog.test.ts"), "import { test } from 'node:test';\nimport assert from 'node:assert/strict';\nimport { createCatalog } from './catalog.ts';\ntest('catalog invariants', () => { const catalog = createCatalog(); catalog.add('A', 2); assert.equal(catalog.stock('A'), 2); assert.throws(() => catalog.add('A', 3)); assert.throws(() => catalog.add('B', -1)); assert.throws(() => catalog.add('B', 0.5)); });\n").unwrap();
    fs::write(path.join(".github/workflows/ci.yml"), "name: CI\non: [push, pull_request]\njobs:\n  unit:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: pnpm/action-setup@v4\n      - uses: actions/setup-node@v4\n        with:\n          node-version: 22\n      - run: pnpm install --frozen-lockfile\n      - run: pnpm test\n").unwrap();
}

#[test]
#[ignore = "Explicit live Taskmaster inference; requires selection and fresh evidence destination"]
fn selected_provider_readiness_quality_matrix() {
    let selected: TaskmasterSettings = serde_json::from_slice(
        &fs::read(
            std::env::var("ORKWORKS_READINESS_SELECTION_FILE").expect("explicit selection file"),
        )
        .unwrap(),
    )
    .unwrap();
    let output = PathBuf::from(
        std::env::var("ORKWORKS_READINESS_QUALITY_DIR").expect("explicit private output directory"),
    );
    fs::create_dir_all(&output).unwrap();
    let selection = selected.selection.expect("configured Taskmaster selection");
    fs::write(
        output.join("selection.json"),
        serde_json::to_vec_pretty(&selection).unwrap(),
    )
    .unwrap();
    let mut failures = Vec::new();
    for case in ["baseline", "improved", "healthy"] {
        if std::env::var("ORKWORKS_READINESS_QUALITY_CASE").is_ok_and(|filter| filter != case) {
            continue;
        }
        for trial in 1..=3 {
            let directory = output.join(format!("{case}-{trial}"));
            assert!(
                !directory.exists(),
                "retain earlier evidence; select a fresh destination"
            );
            let workspace = directory.join("workspace");
            fixture(&workspace, case != "baseline");
            let state = crate::test_support::test_app_state_with_workspace(&workspace);
            let runtime = TaskmasterRuntime::open(directory.join("runtime"));
            runtime
                .activate_knowledge(include_bytes!(
                    "../../../../apps/desktop/resources/knowledge/starter.json"
                ))
                .unwrap();
            runtime
                .replace_settings(TaskmasterSettings {
                    selection: Some(selection.clone()),
                    context_level: ContextLevel::SourceCode,
                    ..Default::default()
                })
                .unwrap();
            let lease = runtime.try_analysis_lease().unwrap().unwrap();
            let captured = capture(&state, &runtime)
                .expect("selected transport and signed rubric must be eligible");
            fs::write(
                directory.join("dispatch-identity.json"),
                serde_json::to_vec_pretty(&captured.identity).unwrap(),
            )
            .unwrap();
            let store = ReadinessStore::new(&captured.root);
            let mut run = Run::queued();
            run.input_identity = Some(captured.identity.clone());
            store.save_run(&run).unwrap();
            EVIDENCE.with(|target| *target.borrow_mut() = Some(directory.clone()));
            let result = execute(&state, &runtime, captured, &mut run);
            EVIDENCE.with(|target| *target.borrow_mut() = None);
            if let Err(error) = &result {
                failures.push(format!("{case}-{trial}: {error}"));
            }
            fs::write(directory.join("outcome.json"), serde_json::to_vec_pretty(&json!({"ok":result.is_ok(),"error":result.err(),"run":run,"history":store.history().unwrap()})).unwrap()).unwrap();
            drop(lease);
            eprintln!("readiness quality {case}-{trial}: {:?}", run.state);
        }
    }
    assert!(failures.is_empty(), "Provider trials failed: {failures:?}");
    eprintln!("Transport trials complete; independent semantic review remains required.");
}

/// Five fixed samples, no retries or replacements. At most four dispatches each.
#[test]
#[ignore = "Owner-authorized synthetic score-directed trials; maximum twenty calls"]
fn selected_provider_score_directed_matrix() {
    let settings: Value = serde_json::from_slice(
        &fs::read(std::env::var("ORKWORKS_READINESS_SELECTION_FILE").expect("explicit selection"))
            .unwrap(),
    )
    .unwrap();
    let selection: super::super::runtime::TaskmasterSelection =
        serde_json::from_value(settings["selection"].clone()).expect("configured selection");
    assert_eq!(selection.provider, "codex");
    assert_eq!(selection.model, "gpt-6-luna");
    assert_eq!(selection.reasoning_effort.as_deref(), Some("medium"));
    let output = PathBuf::from(
        std::env::var("ORKWORKS_SCORE_DIRECTED_QUALITY_DIR").expect("fresh private output"),
    );
    assert!(!output.exists(), "No repeated or replacement trials");
    fs::create_dir_all(&output).unwrap();
    fs::write(
        output.join("selection.json"),
        serde_json::to_vec_pretty(&selection).unwrap(),
    )
    .unwrap();
    for case in [
        "skipped-tests",
        "ci-omits-tests",
        "assertions-omitted",
        "already-satisfied",
        "evidence-unknown",
    ] {
        let directory = output.join(case);
        let workspace = directory.join("workspace");
        fixture(&workspace, true);
        fs::write(workspace.join("AGENTS.md"),"# Agent guide\nREADME.md describes setup and verification. docs/architecture.md describes module ownership and docs/domain.md describes product invariants. Changes require focused behavior checks for valid input, duplicates, negative stock and fractional stock, plus review before merge. Keep those docs consistent with changes. Maintainer approval is required for publishing, API changes or deletion. No autonomous deployment. No recorded execution results are supplied.\n").unwrap();
        fs::write(workspace.join("docs/lessons.md"),"# Lessons\nNo task outcomes are supplied in this synthetic assessment. This absence does not establish whether a feedback process exists elsewhere.\n").unwrap();
        match case {
            "skipped-tests" => {
                let mut package: Value =
                    serde_json::from_slice(&fs::read(workspace.join("package.json")).unwrap())
                        .unwrap();
                package["scripts"]["test"] =
                    json!("node -e \"console.log('Unit suite intentionally skipped')\"");
                fs::write(workspace.join("package.json"), package.to_string()).unwrap();
            }
            "ci-omits-tests" => {
                fs::write(workspace.join(".github/workflows/ci.yml"),"name: CI\non: [push, pull_request]\njobs:\n  unit:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: actions/setup-node@v4\n        with:\n          node-version: 22\n      - run: node --check src/catalog.ts\n").unwrap();
            }
            "assertions-omitted" => {
                fs::write(workspace.join("src/catalog.test.ts"),"import { test } from 'node:test';\nimport { createCatalog } from './catalog.ts';\ntest('catalog invariants', () => { const catalog=createCatalog(); catalog.add('A',2); catalog.stock('A'); });\n").unwrap();
            }
            "evidence-unknown" => {
                fs::remove_dir_all(workspace.join("src")).unwrap();
                fs::remove_dir_all(workspace.join("docs")).unwrap();
                fs::remove_dir_all(workspace.join(".github")).unwrap();
                for name in ["package.json", "pnpm-lock.yaml", ".node-version"] {
                    fs::remove_file(workspace.join(name)).unwrap();
                }
                fs::write(workspace.join("AGENTS.md"),"# Agent guide\nThe supplied repository is an evidence placeholder. Application source, operational history and workflow configuration are outside this fixture. Missing context does not establish a defect.\n").unwrap();
                fs::write(workspace.join("README.md"),"# Evidence placeholder\nNo application behavior or workflow implementation is supplied. No change request or defect is documented.\n").unwrap();
            }
            _ => {}
        }
        let state = crate::test_support::test_app_state_with_workspace(&workspace);
        let runtime = TaskmasterRuntime::open(directory.join("runtime"));
        runtime
            .activate_knowledge(include_bytes!(
                "../../../../apps/desktop/resources/knowledge/starter.json"
            ))
            .unwrap();
        runtime
            .replace_settings(TaskmasterSettings {
                selection: Some(selection.clone()),
                context_level: ContextLevel::SourceCode,
                ..Default::default()
            })
            .unwrap();
        let lease = runtime.try_analysis_lease().unwrap().unwrap();
        let captured = capture(&state, &runtime).expect("configured provider available");
        let store = ReadinessStore::new(&captured.root);
        let mut run = Run::queued();
        run.input_identity = Some(captured.identity.clone());
        store.save_run(&run).unwrap();
        let mut parent = super::super::readiness_suggestions::SuggestionRun::queued(&run.id);
        super::super::readiness_suggestions::save(&captured.root, &parent).unwrap();
        let root = captured.root.clone();
        EVIDENCE.with(|target| *target.borrow_mut() = Some(directory.join("score")));
        fs::create_dir_all(directory.join("score")).unwrap();
        let result = execute(&state, &runtime, captured, &mut run);
        EVIDENCE.with(|target| *target.borrow_mut() = None);
        if let Ok(captured) = &result {
            fs::create_dir_all(directory.join("suggestion")).unwrap();
            super::super::assessment::quality_tests::evidence_directory(Some(
                directory.join("suggestion"),
            ));
            run_suggestion(&state, &runtime, captured, &mut parent, &lease);
            super::super::assessment::quality_tests::evidence_directory(None);
        } else {
            parent.finish(RunState::Failed, "scoring_failed");
        }
        let report = super::super::assessment_store::AssessmentStore::new(&root)
            .load::<super::super::assessment_report::AssessmentRecord>()
            .unwrap();
        fs::write(directory.join("outcome.json"),serde_json::to_vec_pretty(&json!({"run":run,"parent":parent,"report":report,"history":store.history().unwrap(),"scoringError":result.err()})).unwrap()).unwrap();
        eprintln!(
            "score-directed sample {case}: {:?} {:?}",
            parent.state, parent.reason_code
        );
    }
    eprintln!("Five fixed samples complete; independent quality adjudication required.");
}
