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
            let stage = if value["protocol"] == "taskmaster-readiness-discovery-v1" {
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
