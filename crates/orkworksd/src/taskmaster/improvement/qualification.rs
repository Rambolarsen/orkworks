//! Explicit, ignored paired qualification runner. This module is test-only;
//! it has no production admission or configuration path.
use super::{
    access::{self, Capture},
    authorized_dispatch, controlled_invoke, Active,
};
use crate::taskmaster::{
    assessment_context::improvement::ProgressiveEvidence,
    improvement::{
        engine,
        model::{CallBudget, InvestigationStep, MAX_CALLS, RUN_TIMEOUT},
    },
    readiness_rubric::ReadinessRubric,
    runtime::{ContextLevel, TaskmasterRuntime, TaskmasterSelection, TaskmasterSettings},
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::{atomic::AtomicBool, Arc, Mutex},
    time::Instant,
};

const CALL_PROMPT_LIMIT: usize = 128 * 1024;
const CALL_OUTPUT_LIMIT: usize = 64 * 1024;
const GLOBAL_CALL_LIMIT: u64 = 60;
const PUBLIC_KEY_SHA256: &str = "a35da5934c629903f8f31db5067a9c5086f8372c34e3d917c9efb7decfd465dd";
const PUBLIC_CORE_SHA256: &str = "06adeee0640ff57803a6d991d989531221cbce13b54b630b8cb180f51ff8434e";
const PUBLIC_PAYLOAD_SHA256: &str =
    "627c582c06b038d66b7fa196b241ef54b6cf61ee5f9f3cd87309c13c891552e3";
const CASES: [&str; 5] = [
    "positive-behavioral-assertions",
    "positive-entry-command",
    "positive-ci-failure",
    "negative-satisfied",
    "negative-restricted",
];

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Selection {
    provider: String,
    model: String,
    reasoning_effort: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct QualificationPlan {
    format_version: u8,
    corpus_root: PathBuf,
    pinned_inputs_path: PathBuf,
    trial_ledger_path: PathBuf,
    artifact_root: PathBuf,
    activation_path: PathBuf,
    public_payload_path: PathBuf,
    expected_public_payload_file_sha256: String,
    expected_public_core_sha256: String,
    expected_public_key_sha256: String,
    expected_bundle_sha256: String,
    expected_manifest_sha256: String,
    expected_version: String,
    expected_sequence: u64,
    expected_published_at: String,
    selection: Selection,
}

struct Prepared {
    plan: QualificationPlan,
    runtime: Arc<TaskmasterRuntime>,
    _runtime_dir: tempfile::TempDir,
    activation_sha256: String,
    knowledge: crate::taskmaster::runtime::KnowledgeBundle,
    rubric: ReadinessRubric,
    pinned_inputs: Value,
}

/// One invocation is reserved and its prompt is durable before transport entry.
/// An interrupted reservation is intentionally never reclaimed.
struct Ledger {
    path: PathBuf,
    artifact_root: PathBuf,
    value: Value,
    _lock: File,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Arm {
    App,
    Baseline,
}
impl Arm {
    fn label(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Baseline => "controlled-baseline",
        }
    }
    fn response_shape(
        self,
        final_call: bool,
    ) -> Option<crate::providers::native_inference::NativeResponseShape> {
        use crate::providers::native_inference::NativeResponseShape;
        match (self, final_call) {
            (Self::App, true) => Some(NativeResponseShape::ImprovementFinalV1),
            (_, false) => Some(NativeResponseShape::ImprovementEvidenceV1),
            (Self::Baseline, true) => None,
        }
    }
}

#[test]
#[ignore = "live qualification: parent executes only after the actual signed-publication gate"]
fn live_paired_brain_improvement_qualification() {
    let plan_path = std::env::var_os("ORKWORKS_IMPROVEMENT_QUALIFICATION_PLAN")
        .map(PathBuf::from)
        .expect("set ORKWORKS_IMPROVEMENT_QUALIFICATION_PLAN to the owner-prepared JSON");
    let prepared =
        prepare(&plan_path).unwrap_or_else(|error| panic!("qualification preflight: {error}"));
    let mut ledger = Ledger::begin(&prepared.plan)
        .unwrap_or_else(|error| panic!("qualification ledger: {error}"));
    let result = run_pairs(&prepared, &mut ledger);
    let fixture_check = verify_pinned_roots(&prepared.plan, &prepared.pinned_inputs);
    if let Err(error) = fixture_check {
        let _ = ledger.finish("incomplete: fixture identity changed");
        panic!("qualification fixture changed: {error}");
    }
    match result {
        Ok(()) => ledger
            .finish("completed; awaiting independent adjudication")
            .unwrap(),
        Err(error) => {
            ledger
                .finish("incomplete; attempted calls and failures retained")
                .unwrap();
            panic!("qualification stopped after preserving attempted calls: {error}");
        }
    }
}

#[test]
#[ignore = "read-only signed/public-core/corpus preflight for the parent; reserves no calls"]
fn verify_actual_signed_inputs() {
    let plan_path = std::env::var_os("ORKWORKS_IMPROVEMENT_QUALIFICATION_PLAN")
        .map(PathBuf::from)
        .expect("set ORKWORKS_IMPROVEMENT_QUALIFICATION_PLAN to the owner-prepared JSON");
    let prepared =
        prepare(&plan_path).unwrap_or_else(|error| panic!("qualification preflight: {error}"));
    let ledger = read_json(&prepared.plan.trial_ledger_path).unwrap();
    assert_eq!(ledger["callsReserved"], 0);
    assert_eq!(ledger["callsCompleted"], 0);
    assert!(ledger["outputs"].as_array().is_some_and(Vec::is_empty));
    assert!(!prepared.plan.artifact_root.exists());
}

fn prepare(plan_path: &Path) -> Result<Prepared, String> {
    let bytes = fs::read(plan_path).map_err(|_| "plan_unavailable")?;
    if bytes.len() > 32 * 1024 {
        return Err("plan_too_large".into());
    }
    let plan: QualificationPlan = serde_json::from_slice(&bytes).map_err(|_| "plan_invalid")?;
    validate_plan(&plan)?;
    let pinned_inputs: Value = read_json(&plan.pinned_inputs_path)?;
    validate_pinned_inputs(&plan, &pinned_inputs)?;
    verify_pinned_roots(&plan, &pinned_inputs)?;
    let trial_ledger = read_json(&plan.trial_ledger_path)?;
    validate_trial_ledger(&trial_ledger, &plan)?;
    if plan.artifact_root.exists() {
        return Err("previous_outputs_exist; no rerun is permitted".into());
    }

    // The production key is compiled into knowledge.rs. `open`, deliberately,
    // never selects the test key. Activation and all subsequent checks happen
    // in a temporary root so the user's settings, proof and ledger stay read-only.
    let temp = tempfile::Builder::new()
        .prefix("orkworks-brain-qualification-runtime-")
        .tempdir()
        .map_err(|_| "isolated_runtime_unavailable")?;
    let runtime = Arc::new(TaskmasterRuntime::open(temp.path().to_path_buf()));
    let activation = fs::read(&plan.activation_path).map_err(|_| "activation_unavailable")?;
    if activation.len() > crate::taskmaster::knowledge::MAX_ACTIVATION_BYTES {
        return Err("activation_too_large".into());
    }
    runtime
        .activate_knowledge(&activation)
        .map_err(|_| "signed_activation_rejected")?;
    let settings = TaskmasterSettings {
        enabled: true,
        selection: Some(TaskmasterSelection {
            provider: plan.selection.provider.clone(),
            model: plan.selection.model.clone(),
            reasoning_effort: Some(plan.selection.reasoning_effort.clone()),
            ollama_base_url: None,
        }),
        context_level: ContextLevel::SourceCode,
        ..TaskmasterSettings::default()
    };
    runtime.replace_settings(settings)?;
    let identity_workspace = plan.corpus_root.join(&CASES[0]);
    let (snapshot, rubric) = runtime.improvement_evaluation_snapshot(&identity_workspace)?;
    let identity = snapshot
        .knowledge_identity
        .ok_or("verified_identity_unavailable")?;
    if identity.bundle_sha256 != plan.expected_bundle_sha256
        || identity.version != plan.expected_version
        || identity.sequence != plan.expected_sequence
        || identity.public_key_sha256 != PUBLIC_KEY_SHA256
        || identity.public_key_sha256 != plan.expected_public_key_sha256
    {
        return Err("signed_identity_mismatch".into());
    }
    let knowledge = snapshot.knowledge.ok_or("verified_bundle_unavailable")?;
    verify_signed_payload(&plan, &activation, &knowledge)?;
    verify_payload_file(&plan)?;
    let activation_sha256 = sha256(&activation);
    let prepared = Prepared {
        plan,
        runtime,
        _runtime_dir: temp,
        activation_sha256,
        knowledge,
        rubric,
        pinned_inputs,
    };
    preflight_cases(&prepared)?;
    Ok(prepared)
}

fn preflight_cases(prepared: &Prepared) -> Result<(), String> {
    for case in CASES {
        let (_metadata_root, state) =
            test_state_for_fixture(&prepared.plan.corpus_root.join(case))?;
        let capture = access::capture(&state, &prepared.runtime, None)
            .map_err(|_| "provider_workspace_or_guidance_unavailable")?;
        verify_selection(&capture, &prepared.plan.selection)?;
        let entry = active_entry(&capture);
        // Exercise the same bounded, cancellable evidence collector before the
        // ledger can reserve a call. This is read-only and never invokes a model.
        let _ = collect_evidence(&capture, &entry, Instant::now())?;
    }
    Ok(())
}

fn test_state_for_fixture(
    fixture: &Path,
) -> Result<(tempfile::TempDir, Arc<crate::AppState>), String> {
    let metadata_root = tempfile::tempdir().map_err(|_| "workspace_metadata_unavailable")?;
    let state = crate::test_support::test_app_state_with_workspace(metadata_root.path());
    let mut workspace = state
        .workspace
        .lock()
        .map_err(|_| "workspace_unavailable")?;
    workspace.as_mut().ok_or("workspace_unavailable")?.path = fixture.to_path_buf();
    drop(workspace);
    Ok((metadata_root, state))
}

fn verify_selection(capture: &Capture, expected: &Selection) -> Result<(), String> {
    if capture
        .evaluation
        .settings
        .selection
        .as_ref()
        .map(|s| (&s.provider, &s.model, s.reasoning_effort.as_ref()))
        != Some((
            &expected.provider,
            &expected.model,
            Some(&expected.reasoning_effort),
        ))
    {
        return Err("captured_selection_mismatch".into());
    }
    Ok(())
}

fn validate_plan(plan: &QualificationPlan) -> Result<(), String> {
    if plan.format_version != 1
        || plan.expected_public_core_sha256 != PUBLIC_CORE_SHA256
        || plan.expected_public_payload_file_sha256 != PUBLIC_PAYLOAD_SHA256
        || plan.expected_public_key_sha256 != PUBLIC_KEY_SHA256
        || plan.expected_bundle_sha256.is_empty()
        || plan.expected_manifest_sha256.is_empty()
        || plan.expected_version.trim().is_empty()
        || plan.expected_sequence == 0
        || plan.expected_published_at.trim().is_empty()
        || plan.selection.provider != "codex"
        || plan.selection.model != "gpt-6-luna"
        || plan.selection.reasoning_effort != "medium"
    {
        return Err("plan_identity_or_selection_mismatch".into());
    }
    let root = canonical_existing(&plan.corpus_root)?;
    if canonical_existing(&plan.pinned_inputs_path)? != root.join("pinned-inputs.json")
        || canonical_existing(&plan.trial_ledger_path)? != root.join("trial-ledger.json")
        || canonicalize_parent(&plan.artifact_root)? != root.join("trial-run")
    {
        return Err("qualification_paths_mismatch".into());
    }
    if canonical_existing(&plan.activation_path).is_err()
        || canonical_existing(&plan.public_payload_path).is_err()
    {
        return Err("signed_artifacts_unavailable".into());
    }
    Ok(())
}

fn verify_payload_file(plan: &QualificationPlan) -> Result<(), String> {
    let bytes = fs::read(&plan.public_payload_path).map_err(|_| "public_payload_unavailable")?;
    if sha256(&bytes) != plan.expected_public_payload_file_sha256 {
        return Err("public_payload_file_hash_mismatch".into());
    }
    let value: Value = serde_json::from_slice(&bytes).map_err(|_| "public_payload_invalid")?;
    if value.get("formatVersion").and_then(Value::as_u64) != Some(2)
        || value
            .get("capabilities")
            .and_then(Value::as_array)
            .is_none()
        || value.get("pages").and_then(Value::as_array).is_none()
        || value.get("readinessRubric").is_none()
    {
        return Err("public_payload_identity_mismatch".into());
    }
    Ok(())
}

fn verify_signed_payload(
    plan: &QualificationPlan,
    activation: &[u8],
    verified: &crate::taskmaster::runtime::KnowledgeBundle,
) -> Result<(), String> {
    let outer: Value = serde_json::from_slice(activation).map_err(|_| "activation_invalid")?;
    let bundle_b64 = outer
        .get("bundleEnvelopeBase64")
        .and_then(Value::as_str)
        .ok_or("bundle_envelope_missing")?;
    let manifest_b64 = outer
        .get("manifestEnvelopeBase64")
        .and_then(Value::as_str)
        .ok_or("manifest_envelope_missing")?;
    let bundle_bytes = STANDARD
        .decode(bundle_b64)
        .map_err(|_| "bundle_envelope_invalid")?;
    let manifest_bytes = STANDARD
        .decode(manifest_b64)
        .map_err(|_| "manifest_envelope_invalid")?;
    if sha256(&bundle_bytes) != plan.expected_bundle_sha256
        || sha256(&manifest_bytes) != plan.expected_manifest_sha256
    {
        return Err("signed_envelope_hash_mismatch".into());
    }
    let envelope: Value =
        serde_json::from_slice(&bundle_bytes).map_err(|_| "bundle_envelope_invalid")?;
    let payload_text = envelope
        .get("payload")
        .and_then(Value::as_str)
        .ok_or("bundle_payload_missing")?;
    let signed_payload: Value =
        serde_json::from_str(payload_text).map_err(|_| "bundle_payload_invalid")?;
    let public_bytes =
        fs::read(&plan.public_payload_path).map_err(|_| "public_payload_unavailable")?;
    let public_payload: Value =
        serde_json::from_slice(&public_bytes).map_err(|_| "public_payload_invalid")?;
    let mut signed_core = signed_payload.clone();
    let mut public_core = public_payload;
    for key in ["version", "sequence", "publishedAt"] {
        signed_core
            .as_object_mut()
            .ok_or("signed_payload_shape")?
            .remove(key);
        public_core
            .as_object_mut()
            .ok_or("public_payload_shape")?
            .remove(key);
    }
    if signed_core != public_core
        || verified.version != plan.expected_version
        || verified.sequence != plan.expected_sequence
        || verified.published_at != plan.expected_published_at
        || verified.format_version != 2
    {
        return Err("signed_public_core_or_capability_mismatch".into());
    }
    Ok(())
}

fn validate_pinned_inputs(plan: &QualificationPlan, value: &Value) -> Result<(), String> {
    if value.get("formatVersion").and_then(Value::as_u64) != Some(1) {
        return Err("pinned_inputs_invalid".into());
    }
    let cases = value
        .get("cases")
        .and_then(Value::as_object)
        .ok_or("pinned_cases_missing")?;
    if cases.len() != CASES.len() || CASES.iter().any(|id| !cases.contains_key(*id)) {
        return Err("pinned_case_set_mismatch".into());
    }
    for id in CASES {
        let case = &cases[id];
        if case.get("revision").and_then(Value::as_str).is_none()
            || case
                .get("allArtifactFileSha256")
                .and_then(Value::as_object)
                .is_none()
        {
            return Err("pinned_case_identity_missing".into());
        }
    }
    if plan.corpus_root.join("adjudication.json").exists() {
        // It remains on disk for independent adjudication but is never opened or
        // included in either inference prompt.
    }
    Ok(())
}

fn validate_trial_ledger(value: &Value, plan: &QualificationPlan) -> Result<(), String> {
    let outputs_empty = value
        .get("outputs")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty);
    let identity = value.pointer("/guidance/signedBundleIdentity");
    if value.get("formatVersion").and_then(Value::as_u64) != Some(1)
        || value.get("callsReserved").and_then(Value::as_u64) != Some(0)
        || value.get("callsCompleted").and_then(Value::as_u64) != Some(0)
        || !outputs_empty
        || value
            .pointer("/authorization/callLimit")
            .and_then(Value::as_u64)
            != Some(GLOBAL_CALL_LIMIT)
        || value
            .pointer("/authorization/caseLimit")
            .and_then(Value::as_u64)
            != Some(CASES.len() as u64)
        || value
            .pointer("/authorization/appCallsPerCase")
            .and_then(Value::as_u64)
            != Some(MAX_CALLS as u64)
        || value
            .pointer("/authorization/baselineCallsPerCase")
            .and_then(Value::as_u64)
            != Some(MAX_CALLS as u64)
        || value
            .pointer("/authorization/silentRetries")
            .and_then(Value::as_bool)
            != Some(false)
        || value
            .pointer("/authorization/modelSwitches")
            .and_then(Value::as_bool)
            != Some(false)
        || value
            .pointer("/authorization/publicationApproved")
            .and_then(Value::as_bool)
            != Some(true)
        || value
            .pointer("/guidance/publicCoreSha256")
            .and_then(Value::as_str)
            != Some(PUBLIC_CORE_SHA256)
        || identity
            .and_then(|v| v.get("publicCoreSha256"))
            .and_then(Value::as_str)
            != Some(PUBLIC_CORE_SHA256)
        || identity
            .and_then(|v| v.get("publicPayloadFileSha256"))
            .and_then(Value::as_str)
            != Some(PUBLIC_PAYLOAD_SHA256)
        || identity
            .and_then(|v| v.get("publicKeySha256"))
            .and_then(Value::as_str)
            != Some(PUBLIC_KEY_SHA256)
        || identity
            .and_then(|v| v.get("version"))
            .and_then(Value::as_str)
            != Some(plan.expected_version.as_str())
        || identity
            .and_then(|v| v.get("sequence"))
            .and_then(Value::as_u64)
            != Some(plan.expected_sequence)
        || identity
            .and_then(|v| v.get("publishedAt"))
            .and_then(Value::as_str)
            != Some(plan.expected_published_at.as_str())
        || identity
            .and_then(|v| v.get("bundleSha256"))
            .and_then(Value::as_str)
            != Some(plan.expected_bundle_sha256.as_str())
        || identity
            .and_then(|v| v.get("manifestSha256"))
            .and_then(Value::as_str)
            != Some(plan.expected_manifest_sha256.as_str())
        || value.pointer("/selection/provider").and_then(Value::as_str)
            != Some(plan.selection.provider.as_str())
        || value.pointer("/selection/model").and_then(Value::as_str)
            != Some(plan.selection.model.as_str())
        || value
            .pointer("/selection/reasoningEffort")
            .and_then(Value::as_str)
            != Some(plan.selection.reasoning_effort.as_str())
    {
        return Err("trial_ledger_is_not_unused_or_authorized".into());
    }
    Ok(())
}

fn fixture_revisions(pinned: &Value) -> Result<Value, String> {
    let cases = pinned
        .get("cases")
        .and_then(Value::as_object)
        .ok_or("pinned_cases_missing")?;
    let mut revisions = Map::new();
    for id in CASES {
        revisions.insert(
            id.into(),
            cases
                .get(id)
                .and_then(|case| case.get("revision"))
                .cloned()
                .ok_or("pinned_revision_missing")?,
        );
    }
    Ok(Value::Object(revisions))
}

fn verify_pinned_roots(plan: &QualificationPlan, pinned: &Value) -> Result<(), String> {
    let cases = pinned
        .get("cases")
        .and_then(Value::as_object)
        .ok_or("pinned_cases_missing")?;
    for id in CASES {
        verify_pinned_case(plan, &cases[id], id)?;
    }
    Ok(())
}

fn verify_pinned_case(plan: &QualificationPlan, expected: &Value, id: &str) -> Result<(), String> {
    let root = plan.corpus_root.join(id);
    let revision = expected
        .get("revision")
        .and_then(Value::as_str)
        .ok_or("pinned_revision_missing")?;
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(&root)
            .args(args)
            .output()
            .map_err(|_| "fixture_git_unavailable")
    };
    let actual = git(&["rev-parse", "HEAD"])?;
    if !actual.status.success() || String::from_utf8_lossy(&actual.stdout).trim() != revision {
        return Err("fixture_revision_mismatch".into());
    }
    let status = git(&["status", "--porcelain", "--untracked-files=no"])?;
    if !status.status.success() || !status.stdout.is_empty() {
        return Err("fixture_tracked_changes".into());
    }
    let tracked = git(&["ls-files", "-z"])?;
    if !tracked.status.success() {
        return Err("fixture_tracked_files_unavailable".into());
    }
    let mut actual_tracked = tracked
        .stdout
        .split(|byte| *byte == 0)
        .filter(|bytes| !bytes.is_empty())
        .map(|bytes| String::from_utf8(bytes.to_vec()).map_err(|_| "fixture_path_invalid"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut expected_tracked = expected
        .get("trackedFiles")
        .and_then(Value::as_array)
        .ok_or("pinned_tracked_files_missing")?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or("pinned_tracked_path_invalid")
        })
        .collect::<Result<Vec<_>, _>>()?;
    actual_tracked.sort();
    expected_tracked.sort();
    if actual_tracked != expected_tracked {
        return Err("fixture_tracked_file_set_changed".into());
    }

    let expected_hashes = expected
        .get("allArtifactFileSha256")
        .and_then(Value::as_object)
        .ok_or("pinned_file_hashes_missing")?;
    let mut expected_files = expected_hashes.keys().cloned().collect::<Vec<_>>();
    expected_files.sort();
    let mut actual_files = Vec::new();
    collect_regular_files(&root, &root, &mut actual_files)?;
    actual_files.sort();
    if actual_files != expected_files {
        return Err("fixture_file_set_changed".into());
    }
    for relative in expected_files {
        let relative_path = Path::new(&relative);
        if relative_path.is_absolute()
            || relative_path
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            return Err("pinned_file_path_invalid".into());
        }
        let bytes = fs::read(root.join(&relative)).map_err(|_| "fixture_file_unavailable")?;
        if expected_hashes[&relative].as_str() != Some(sha256(&bytes).as_str()) {
            return Err("fixture_file_hash_mismatch".into());
        }
    }
    Ok(())
}

fn collect_regular_files(
    root: &Path,
    directory: &Path,
    files: &mut Vec<String>,
) -> Result<(), String> {
    for item in fs::read_dir(directory).map_err(|_| "fixture_directory_unavailable")? {
        let item = item.map_err(|_| "fixture_directory_unavailable")?;
        let path = item.path();
        let relative = path
            .strip_prefix(root)
            .map_err(|_| "fixture_path_invalid")?;
        if relative == Path::new(".git") {
            continue;
        }
        let kind = fs::symlink_metadata(&path)
            .map_err(|_| "fixture_file_unavailable")?
            .file_type();
        if kind.is_symlink() {
            return Err("fixture_symlink_present".into());
        }
        if kind.is_dir() {
            collect_regular_files(root, &path, files)?;
        } else if kind.is_file() {
            files.push(relative.to_str().ok_or("fixture_path_invalid")?.to_owned());
        } else {
            return Err("fixture_nonregular_file_present".into());
        }
    }
    Ok(())
}

impl Ledger {
    fn begin(plan: &QualificationPlan) -> Result<Self, String> {
        let lock_path = plan
            .trial_ledger_path
            .with_extension("json.qualification-lock");
        let lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(|_| "qualification_already_started_or_locked")?;
        let value = match read_json(&plan.trial_ledger_path) {
            Ok(value) => value,
            Err(error) => {
                let _ = fs::remove_file(lock_path);
                return Err(error);
            }
        };
        if validate_trial_ledger(&value, plan).is_err() {
            let _ = fs::remove_file(lock_path);
            return Err("trial_ledger_is_not_unused_or_authorized".into());
        }
        if let Some(parent) = plan.artifact_root.parent() {
            fs::create_dir_all(parent).map_err(|_| "artifact_parent_unavailable")?;
        }
        fs::create_dir(&plan.artifact_root)
            .map_err(|_| "previous_outputs_exist; no rerun is permitted")?;
        let mut ledger = Self {
            path: plan.trial_ledger_path.clone(),
            artifact_root: plan.artifact_root.clone(),
            value,
            _lock: lock,
        };
        ledger.value["status"] = json!("running paired qualification; no reset or retry");
        ledger.write()?;
        Ok(ledger)
    }

    fn reserve(&mut self, case: &str, arm: Arm, prompt: &str) -> Result<(PathBuf, usize), String> {
        if prompt.len() > CALL_PROMPT_LIMIT {
            return Err("prompt_exceeds_limit".into());
        }
        let outputs = self
            .value
            .get("outputs")
            .and_then(Value::as_array)
            .ok_or("trial_outputs_invalid")?;
        let arm_name = arm.label();
        let arm_count = outputs
            .iter()
            .filter(|item| {
                item.get("caseId").and_then(Value::as_str) == Some(case)
                    && item.get("arm").and_then(Value::as_str) == Some(arm_name)
            })
            .count();
        let reserved = self
            .value
            .get("callsReserved")
            .and_then(Value::as_u64)
            .ok_or("trial_counter_invalid")?;
        if reserved >= GLOBAL_CALL_LIMIT || arm_count >= MAX_CALLS {
            return Err("qualification_call_budget_exhausted".into());
        }
        let call = arm_count + 1;
        let directory = self.artifact_root.join(case).join(arm_name);
        fs::create_dir_all(&directory).map_err(|_| "artifact_directory_unavailable")?;
        let prompt_path = directory.join(format!("call-{call:02}.prompt.json"));
        let prompt_hash = write_new(&prompt_path, prompt.as_bytes())?;
        let prompt_value: Value = serde_json::from_str(prompt).map_err(|_| "prompt_not_json")?;
        let selected_facts = prompt_value
            .get("repositoryFacts")
            .and_then(Value::as_array)
            .map(|facts| {
                facts.iter().filter_map(|fact| Some(json!({
                "id": fact.get("id")?, "path": fact.get("path")?, "offset": fact.get("offset")?,
                "bytes": fact.get("text")?.as_str()?.len(), "sha256": fact.get("sha256")?
            }))).collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let selected_pages = prompt_value
            .get("knowledgePages")
            .and_then(Value::as_array)
            .map(|pages| {
                pages
                    .iter()
                    .filter_map(|page| {
                        Some(json!({"id":page.get("id")?,"sha256":page.get("sha256")?}))
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        self.value["callsReserved"] = json!(reserved + 1);
        let outputs = self.value["outputs"]
            .as_array_mut()
            .ok_or("trial_outputs_invalid")?;
        outputs.push(json!({"caseId":case,"arm":arm_name,"call":call,"state":"reserved","promptFile":relative_artifact(&self.artifact_root,&prompt_path),"promptSha256":prompt_hash,"promptBytes":prompt.len(),"selectedFacts":selected_facts,"selectedPages":selected_pages}));
        self.write()?;
        Ok((directory, call))
    }

    fn invoke(
        &mut self,
        case: &str,
        arm: Arm,
        prompt: &str,
        transport: impl FnOnce() -> Result<String, String>,
    ) -> Result<String, String> {
        let (directory, call) = self.reserve(case, arm, prompt)?;
        let result = transport();
        let output_path = directory.join(format!("call-{call:02}.output.txt"));
        let oversized = matches!(&result, Ok(text) if text.len() > CALL_OUTPUT_LIMIT);
        let (state, category, output_hash, output_bytes, truncated, original_bytes) = match &result
        {
            Ok(text) if text.len() <= CALL_OUTPUT_LIMIT => {
                let hash = write_new(&output_path, text.as_bytes())?;
                (
                    "completed",
                    "none",
                    Some(hash),
                    text.len(),
                    false,
                    text.len(),
                )
            }
            Ok(text) => {
                let prefix = utf8_prefix(text, CALL_OUTPUT_LIMIT);
                let hash = write_new(&output_path, prefix.as_bytes())?;
                (
                    "failed",
                    "response_limit",
                    Some(hash),
                    prefix.len(),
                    true,
                    text.len(),
                )
            }
            Err(error) if error == "fixture_identity_changed" => {
                ("failed", "fixture_identity_changed", None, 0, false, 0)
            }
            Err(_) => ("failed", "provider_or_policy_failure", None, 0, false, 0),
        };
        let outputs = self.value["outputs"]
            .as_array_mut()
            .ok_or("trial_outputs_invalid")?;
        let record = outputs.last_mut().ok_or("trial_output_missing")?;
        record["state"] = json!(state);
        record["failureCategory"] = json!(category);
        record["outputBytes"] = json!(output_bytes);
        record["truncated"] = json!(truncated);
        record["originalUtf8Bytes"] = json!(original_bytes);
        if let Some(hash) = output_hash {
            record["outputSha256"] = json!(hash);
            record["outputFile"] = json!(relative_artifact(&self.artifact_root, &output_path));
        }
        let completed = self
            .value
            .get("callsCompleted")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        self.value["callsCompleted"] = json!(completed + 1);
        self.write()?;
        if oversized {
            return Err("response_limit".into());
        }
        result.map_err(|error| {
            if error == "fixture_identity_changed" {
                "fixture_identity_changed".into()
            } else {
                "provider_or_policy_failure".into()
            }
        })
    }

    fn write(&self) -> Result<(), String> {
        let bytes =
            serde_json::to_vec_pretty(&self.value).map_err(|_| "trial_ledger_encode_failed")?;
        atomic_write(&self.path, &bytes)
    }

    fn finish(&mut self, status: &'static str) -> Result<(), String> {
        self.value["status"] = json!(status);
        self.write()
    }

    fn fail_last(&mut self, case: &str, arm: Arm, category: &'static str) -> Result<(), String> {
        let outputs = self.value["outputs"]
            .as_array_mut()
            .ok_or("trial_outputs_invalid")?;
        let record = outputs.iter_mut().rev().find(|item| {
            item.get("caseId").and_then(Value::as_str) == Some(case)
                && item.get("arm").and_then(Value::as_str) == Some(arm.label())
                && item.get("state").and_then(Value::as_str) == Some("completed")
        });
        if let Some(record) = record {
            record["state"] = json!("failed");
            record["failureCategory"] = json!(category);
            self.write()?;
        }
        Ok(())
    }
}

fn run_pairs(prepared: &Prepared, ledger: &mut Ledger) -> Result<(), String> {
    let mut failed_samples = 0;
    let mut setup = serde_json::Map::new();
    setup.insert("sourceImplementation".into(), json!(git_head()));
    setup.insert(
        "sourceHarnessSha256".into(),
        json!(sha256(include_bytes!("qualification.rs"))),
    );
    setup.insert(
        "sourceRuntimeModuleSha256".into(),
        json!(sha256(include_bytes!("runtime.rs"))),
    );
    setup.insert("activationSha256".into(), json!(prepared.activation_sha256));
    setup.insert(
        "bundleSha256".into(),
        json!(prepared.plan.expected_bundle_sha256),
    );
    setup.insert(
        "manifestSha256".into(),
        json!(prepared.plan.expected_manifest_sha256),
    );
    setup.insert("publicCoreSha256".into(), json!(PUBLIC_CORE_SHA256));
    setup.insert(
        "publicPayloadFileSha256".into(),
        json!(PUBLIC_PAYLOAD_SHA256),
    );
    setup.insert("selection".into(), json!(prepared.plan.selection));
    setup.insert("limits".into(), json!({"callsPerArm":MAX_CALLS,"callsTotal":GLOBAL_CALL_LIMIT,"deadlineSeconds":RUN_TIMEOUT.as_secs(),"ranges":32,"aggregateEvidenceBytes":65536,"rangeBytes":8192,"pages":16,"guidanceBytes":32768,"appFinalShape":"ImprovementFinalV1","baselineFinalShape":"natural advice; no forced result schema","baselineTools":"data-only; no shell, edits, or native tools"}));
    setup.insert(
        "fixtureGitOids".into(),
        fixture_revisions(&prepared.pinned_inputs)?,
    );
    setup.insert(
        "signedGuidanceIdentity".into(),
        json!({
            "bundleSha256":prepared.plan.expected_bundle_sha256,
            "manifestSha256":prepared.plan.expected_manifest_sha256,
            "publicCoreSha256":PUBLIC_CORE_SHA256,
            "publicPayloadFileSha256":PUBLIC_PAYLOAD_SHA256,
            "version":prepared.plan.expected_version,
            "sequence":prepared.plan.expected_sequence,
            "publishedAt":prepared.plan.expected_published_at,
            "publicKeySpkiSha256":PUBLIC_KEY_SHA256,
            "activationFileSha256":prepared.activation_sha256,
        }),
    );
    atomic_write(
        &prepared.plan.artifact_root.join("run.json"),
        &serde_json::to_vec_pretty(&Value::Object(setup))
            .map_err(|_| "run_metadata_encode_failed")?,
    )?;

    for case in CASES {
        let fixture = prepared.plan.corpus_root.join(case);
        let (_metadata_root, state) = test_state_for_fixture(&fixture)?;
        let capture = access::capture(&state, &prepared.runtime, None)
            .map_err(|_| "provider_workspace_or_guidance_unavailable")?;
        verify_selection(&capture, &prepared.plan.selection)?;
        let app_started = Instant::now();
        let app_entry = active_entry(&capture);
        let initial = collect_evidence(&capture, &app_entry, app_started)?;
        let app_result = run_app_arm(
            &state,
            &prepared.runtime,
            &capture,
            initial.clone(),
            app_entry,
            app_started,
            &prepared.plan,
            &prepared.pinned_inputs,
            ledger,
            case,
        );
        if let Err(error) = &app_result {
            failed_samples += 1;
            write_arm_result(
                &prepared.plan.artifact_root,
                case,
                Arm::App,
                json!({"state":"failed","failureCategory":safe_error(error)}),
            )?;
        }

        let baseline_started = Instant::now();
        let baseline_entry = active_entry(&capture);
        let baseline_evidence = collect_evidence(&capture, &baseline_entry, baseline_started)?;
        let baseline_result = run_baseline_arm(
            &state,
            &prepared.runtime,
            &capture,
            baseline_evidence,
            baseline_entry,
            baseline_started,
            &prepared.plan,
            &prepared.pinned_inputs,
            ledger,
            case,
        );
        if baseline_result.is_err() {
            failed_samples += 1;
        }
        write_arm_result(
            &prepared.plan.artifact_root,
            case,
            Arm::Baseline,
            baseline_result
                .as_ref()
                .map(|text| json!({"state":"completed","advice":text}))
                .unwrap_or_else(|e| json!({"state":"failed","failureCategory":safe_error(e)})),
        )?;
    }
    if failed_samples == 0 {
        Ok(())
    } else {
        Err("qualification_samples_failed".into())
    }
}

fn active_entry(capture: &Capture) -> Active {
    Active {
        id: format!("qualification-{}", capture.instance),
        instance: capture.instance,
        cancelled: Arc::new(AtomicBool::new(false)),
        user_cancelled: Arc::new(AtomicBool::new(false)),
        inputs_changed: Arc::new(AtomicBool::new(false)),
        settled: Arc::new(AtomicBool::new(false)),
        dispatch: Arc::new(Mutex::new(())),
    }
}

fn collect_evidence(
    capture: &Capture,
    entry: &Active,
    started: Instant,
) -> Result<ProgressiveEvidence, String> {
    let cancelled = entry.cancelled.clone();
    let deadline = started.checked_add(RUN_TIMEOUT).unwrap_or(started);
    ProgressiveEvidence::collect_controlled(
        &capture.path,
        ContextLevel::SourceCode,
        &capture.evaluation.settings.excluded_paths,
        &crate::taskmaster::readiness_store::now(),
        move || {
            if cancelled.load(std::sync::atomic::Ordering::SeqCst) {
                Err("Improvement investigation cancelled".into())
            } else if Instant::now() >= deadline {
                Err("Improvement investigation deadline reached".into())
            } else {
                Ok(())
            }
        },
    )
}

fn run_app_arm(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    capture: &Capture,
    evidence: ProgressiveEvidence,
    entry: Active,
    started: Instant,
    plan: &QualificationPlan,
    pinned: &Value,
    ledger: &mut Ledger,
    case: &str,
) -> Result<(), String> {
    let mut invoke = |input: engine::Invocation<'_>| {
        let manifest = access::supplied_manifest(capture, input.evidence, input.selected_page_ids)?;
        ledger.invoke(case, Arm::App, input.prompt, || {
            verify_pinned_case(plan, &pinned["cases"][case], case)
                .map_err(|_| "fixture_identity_changed".to_owned())?;
            controlled_invoke(
                state,
                runtime,
                capture,
                &manifest,
                &entry,
                started.checked_add(RUN_TIMEOUT).unwrap_or(started),
                input.prompt,
                input.phase == engine::InvocationPhase::Final,
            )
        })
    };
    let investigation = engine::investigate_from(
        started,
        evidence,
        &capture.rubric,
        &capture
            .evaluation
            .knowledge
            .as_ref()
            .ok_or("guidance_unavailable")?
            .pages,
        crate::taskmaster::runtime::assessment::IMPROVEMENT_ENTRY_PAGE_ID,
        &mut invoke,
        || entry.cancelled.load(std::sync::atomic::Ordering::SeqCst),
    );
    match investigation {
        Ok(value) => {
            let bytes =
                serde_json::to_vec_pretty(&json!({"state":"valid","parsedResult":value.result}))
                    .map_err(|_| "app_result_encode_failed")?;
            atomic_write(
                &ledger.artifact_root.join(case).join("app.result.json"),
                &bytes,
            )
        }
        Err(error) => {
            if error.contains("Invalid improvement") {
                ledger.fail_last(case, Arm::App, "invalid_model_output")?;
            }
            write_arm_result(
                &ledger.artifact_root,
                case,
                Arm::App,
                json!({"state":"failed","failureCategory":safe_error(&error)}),
            )?;
            Err(error)
        }
    }
}

fn run_baseline_arm(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    capture: &Capture,
    mut evidence: ProgressiveEvidence,
    entry: Active,
    started: Instant,
    plan: &QualificationPlan,
    pinned: &Value,
    ledger: &mut Ledger,
    case: &str,
) -> Result<String, String> {
    let pages = &capture
        .evaluation
        .knowledge
        .as_ref()
        .ok_or("guidance_unavailable")?
        .pages;
    let mut selected = Vec::new();
    add_baseline_page(
        &mut selected,
        pages,
        crate::taskmaster::runtime::assessment::IMPROVEMENT_ENTRY_PAGE_ID,
    )?;
    for concept in &capture.rubric.concepts {
        add_baseline_page(&mut selected, pages, &concept.page_id)?;
    }
    let allowed_ids = pages.iter().map(|page| page.id.clone()).collect::<Vec<_>>();
    let mut refusals = Vec::<String>::new();
    let mut budget = CallBudget::new(started);
    let mut finish = false;
    loop {
        if entry.cancelled.load(std::sync::atomic::Ordering::SeqCst) {
            return Err("cancelled".into());
        }
        evidence.revalidate()?;
        let final_call = finish || budget.final_due();
        let prompt = baseline_prompt(
            &evidence,
            &capture.rubric,
            pages,
            &selected,
            &refusals,
            final_call,
        )?;
        budget.reserve(Instant::now(), final_call)?;
        let manifest = access::supplied_manifest(capture, &evidence, &selected)?;
        let raw = ledger.invoke(case, Arm::Baseline, &prompt, || {
            verify_pinned_case(plan, &pinned["cases"][case], case)
                .map_err(|_| "fixture_identity_changed".to_owned())?;
            invoke_native_baseline(
                state,
                runtime,
                capture,
                &manifest,
                &entry,
                started.checked_add(RUN_TIMEOUT).unwrap_or(started),
                &prompt,
                final_call,
            )
        })?;
        evidence.revalidate()?;
        if final_call {
            require_baseline_advice(ledger, case, &raw)?;
            atomic_write(
                &ledger
                    .artifact_root
                    .join(case)
                    .join("controlled-baseline.advice.txt"),
                raw.as_bytes(),
            )?;
            return Ok(raw);
        }
        let step = match InvestigationStep::decode(&raw, evidence.catalog().len(), &allowed_ids) {
            Ok(step) => step,
            Err(error) => {
                ledger.fail_last(case, Arm::Baseline, "invalid_model_output")?;
                return Err(error);
            }
        };
        if step.finish {
            finish = true;
            continue;
        }
        for read in step.repository_reads {
            if evidence
                .read(read.file_id, read.offset, read.length)
                .is_err()
            {
                evidence.revalidate()?;
                refusals.push("A requested repository range was unavailable or over the shared evidence limit.".into());
            }
        }
        for id in step.knowledge_page_ids {
            if add_baseline_page(&mut selected, pages, &id).is_err() {
                refusals.push(
                    "A requested signed guidance page exceeded the shared page limit.".into(),
                );
            }
        }
    }
}

fn require_baseline_advice(ledger: &mut Ledger, case: &str, raw: &str) -> Result<(), String> {
    if raw.trim().is_empty() {
        ledger.fail_last(case, Arm::Baseline, "empty_model_output")?;
        return Err("empty_model_output".into());
    }
    Ok(())
}

fn invoke_native_baseline(
    state: &Arc<crate::AppState>,
    runtime: &TaskmasterRuntime,
    capture: &Capture,
    manifest: &super::store::Manifest,
    entry: &Active,
    deadline: Instant,
    prompt: &str,
    final_call: bool,
) -> Result<String, String> {
    use crate::providers::InferenceControl;
    let native = capture
        .evaluation
        .native_revision
        .as_ref()
        .ok_or("native_provider_unavailable")?;
    if capture.evaluation.custom_inference.is_some() {
        return Err("provider_policy_mismatch".into());
    }
    let selection = capture
        .evaluation
        .settings
        .selection
        .as_ref()
        .ok_or("provider_unavailable")?;
    let control = InferenceControl::new(deadline, entry.cancelled.clone());
    let mut gate = |start: &mut dyn FnMut() -> std::io::Result<()>| {
        authorized_dispatch(
            state,
            runtime,
            capture,
            manifest,
            entry,
            deadline,
            false,
            |_| Ok(start()),
        )
        .ok()
    };
    state
        .providers
        .invoke_native_taskmaster_prompt_controlled(
            native.profile,
            &selection.model,
            selection.reasoning_effort.as_deref(),
            selection.ollama_base_url.as_deref(),
            prompt.to_owned(),
            Arm::Baseline.response_shape(final_call),
            &mut gate,
            &control,
        )
        .map_err(|error| crate::taskmaster::assessment_failure::provider_code(&error).to_owned())
}

fn baseline_prompt(
    evidence: &ProgressiveEvidence,
    rubric: &ReadinessRubric,
    pages: &[crate::taskmaster::runtime::KnowledgePage],
    selected: &[String],
    refusals: &[String],
    final_call: bool,
) -> Result<String, String> {
    let selected_pages = pages
        .iter()
        .filter(|page| selected.contains(&page.id))
        .collect::<Vec<_>>();
    let prompt = json!({
        "protocol":"controlled-brain-improvement-baseline-v1",
        "phase":if final_call {"advice"} else {"evidence"},
        "request":"Use Brain to suggest the next improvement towards agentic development.",
        "instruction":"Give natural, concise advice about the most useful next improvement toward agentic development, grounded in the supplied repository evidence and signed Brain guidance. Explain the mechanism, benefit, affected surface, effort/risk/reversibility, citations, what remains uncertain, validation scope and rollback. A specific supported hypothesis is useful; do not invent a missing capability. Unknown behavior is not absence. If evidence is insufficient, say what bounded investigation would help. The supplied rubric is assessment context, not a requirement to produce ratings or JSON. You have a data-only broker: request only catalog file IDs, byte ranges and signed page IDs. No shell, edits, network fetches, native tools, commands or claims of checks. Do not mention case labels or expected outcomes.",
        "stepSchema":{"version":1,"repositoryReads":[{"fileId":0,"offset":0,"length":8192}],"knowledgePageIds":["signed page ID"],"finish":false},
        "stepRules":"Exactly these four fields and version 1. Set finish true with both repositoryReads and knowledgePageIds empty to request the final advice call. Set finish false only when requesting at least one item. Use at most eight total distinct requests across both lists; file IDs must be in the supplied repository catalog, and page IDs must be in the supplied signed guidance catalog. Each read length is 1..8192 bytes and offsets plus lengths must not overflow; selected text endpoints must be valid UTF-8. Unknown fields, paths, commands and unlisted IDs are invalid. Malformed or rejected evidence output is not retried.",
        "finishExample":{"version":1,"repositoryReads":[],"knowledgePageIds":[],"finish":true},
        "finalInstruction":"Return plain advice, not a structured assessment and not the app's final JSON schema.",
        "limits":{"calls":MAX_CALLS,"repositoryFiles":32,"repositoryRanges":32,"repositoryBytes":65536,"rangeBytes":8192,"guidancePages":16,"guidanceBytes":32768},
        "rubric":rubric,
        "repositoryFacts":evidence.facts().iter().enumerate().map(|(i,f)|json!({"id":i,"path":f.path,"offset":f.offset,"fileSize":f.file_size,"complete":f.complete(),"sha256":f.sha256,"text":f.text})).collect::<Vec<_>>(),
        "repositoryCatalog":evidence.catalog().iter().enumerate().map(|(i,path)|json!({"id":i,"path":path})).collect::<Vec<_>>(),
        "coverage":{"catalogComplete":evidence.catalog_complete(),"instructionCoverageComplete":evidence.instruction_coverage_complete()},
        "knowledgePages":selected_pages,
        "readRefusals":refusals,
    }).to_string();
    if prompt.len() > CALL_PROMPT_LIMIT {
        return Err("prompt_exceeds_limit".into());
    }
    Ok(prompt)
}

fn add_baseline_page(
    selected: &mut Vec<String>,
    pages: &[crate::taskmaster::runtime::KnowledgePage],
    id: &str,
) -> Result<(), String> {
    if selected.contains(&id.to_owned()) {
        return Ok(());
    }
    let page = pages
        .iter()
        .find(|page| page.id == id)
        .ok_or("signed_page_unavailable")?;
    let bytes = selected
        .iter()
        .filter_map(|id| pages.iter().find(|page| &page.id == id))
        .map(|page| page.content.len())
        .sum::<usize>();
    if selected.len() >= 16 || bytes.saturating_add(page.content.len()) > 32 * 1024 {
        return Err("guidance_limit".into());
    }
    selected.push(id.to_owned());
    Ok(())
}

fn write_arm_result(root: &Path, case: &str, arm: Arm, value: Value) -> Result<(), String> {
    let name = match arm {
        Arm::App => "app.result.json",
        Arm::Baseline => "controlled-baseline.result.json",
    };
    let bytes = serde_json::to_vec_pretty(&value).map_err(|_| "arm_result_encode_failed")?;
    atomic_write(&root.join(case).join(name), &bytes)
}

fn safe_error(error: &str) -> &'static str {
    if error.contains("Invalid improvement") {
        "invalid_model_output"
    } else if error.contains("deadline") || error.contains("time limit") {
        "deadline_or_budget"
    } else if error.contains("provider") || error.contains("policy") {
        "provider_or_policy_failure"
    } else {
        "invalid_or_unavailable"
    }
}

fn read_json(path: &Path) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|_| "artifact_unavailable")?;
    if bytes.len() > 6 * 1024 * 1024 {
        return Err("artifact_too_large".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "artifact_invalid".into())
}
fn sha256(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}
fn utf8_prefix(text: &str, max_bytes: usize) -> &str {
    let mut end = text.len().min(max_bytes);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}
fn git_head() -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_else(|| "unavailable".into())
}
fn canonical_existing(path: &Path) -> Result<PathBuf, String> {
    fs::canonicalize(path).map_err(|_| "path_unavailable".into())
}
fn canonicalize_parent(path: &Path) -> Result<PathBuf, String> {
    let parent = path.parent().ok_or("path_parent_missing")?;
    Ok(fs::canonicalize(parent)
        .map_err(|_| "path_parent_unavailable")?
        .join(path.file_name().ok_or("path_name_missing")?))
}
fn relative_artifact(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
fn write_new(path: &Path, bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > CALL_PROMPT_LIMIT.max(CALL_OUTPUT_LIMIT) {
        return Err("artifact_item_too_large".into());
    }
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|_| "artifact_exists_or_unavailable")?;
    file.write_all(bytes).map_err(|_| "artifact_write_failed")?;
    file.sync_all().map_err(|_| "artifact_sync_failed")?;
    Ok(sha256(bytes))
}
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = path.with_extension(format!("tmp-{}", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|_| "atomic_write_temp_exists")?;
    file.write_all(bytes).map_err(|_| "artifact_write_failed")?;
    file.sync_all().map_err(|_| "artifact_sync_failed")?;
    fs::rename(&temp, path).map_err(|_| "artifact_replace_failed")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn ledger_fixture(path: &Path, artifact_root: &Path) -> Ledger {
        let lock = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path.with_extension("json.qualification-lock"))
            .unwrap();
        fs::create_dir(artifact_root).unwrap();
        Ledger {
            path: path.to_owned(),
            artifact_root: artifact_root.to_owned(),
            value: json!({"formatVersion":1,"callsReserved":0,"callsCompleted":0,"authorization":{"callLimit":60,"appCallsPerCase":6,"baselineCallsPerCase":6},"outputs":[]}),
            _lock: lock,
        }
    }

    fn plan_for_ledger(root: &Path) -> QualificationPlan {
        QualificationPlan {
            format_version: 1,
            corpus_root: root.into(),
            pinned_inputs_path: root.join("pinned-inputs.json"),
            trial_ledger_path: root.join("trial-ledger.json"),
            artifact_root: root.join("trial-run"),
            activation_path: root.join("activation.json"),
            public_payload_path: root.join("payload.json"),
            expected_public_payload_file_sha256: PUBLIC_PAYLOAD_SHA256.into(),
            expected_public_core_sha256: PUBLIC_CORE_SHA256.into(),
            expected_public_key_sha256: PUBLIC_KEY_SHA256.into(),
            expected_bundle_sha256: "bundle".into(),
            expected_manifest_sha256: "manifest".into(),
            expected_version: "brain-4".into(),
            expected_sequence: 4,
            expected_published_at: "2026-11-12T18:03:27.297Z".into(),
            selection: Selection {
                provider: "codex".into(),
                model: "gpt-6-luna".into(),
                reasoning_effort: "medium".into(),
            },
        }
    }

    fn approved_trial_ledger(plan: &QualificationPlan) -> Value {
        json!({
            "formatVersion":1,"callsReserved":0,"callsCompleted":0,"outputs":[],
            "authorization":{"callLimit":60,"caseLimit":5,"appCallsPerCase":6,"baselineCallsPerCase":6,"silentRetries":false,"modelSwitches":false,"publicationApproved":true},
            "selection":{"provider":"codex","model":"gpt-6-luna","reasoningEffort":"medium"},
            "guidance":{"publicCoreSha256":PUBLIC_CORE_SHA256,"signedBundleIdentity":{
                "publicCoreSha256":PUBLIC_CORE_SHA256,"publicPayloadFileSha256":PUBLIC_PAYLOAD_SHA256,"publicKeySha256":PUBLIC_KEY_SHA256,
                "version":plan.expected_version,"sequence":plan.expected_sequence,"publishedAt":plan.expected_published_at,
                "bundleSha256":plan.expected_bundle_sha256,"manifestSha256":plan.expected_manifest_sha256
            }}
        })
    }

    #[test]
    fn invalid_raw_output_is_saved_and_consumes_the_reserved_attempt() {
        let dir = tempdir().unwrap();
        let mut ledger = ledger_fixture(&dir.path().join("ledger.json"), &dir.path().join("out"));
        let raw = ledger
            .invoke(
                "case-a",
                Arm::App,
                r#"{"repositoryFacts":[],"knowledgePages":[]}"#,
                || Ok("not JSON".into()),
            )
            .unwrap();
        assert_eq!(raw, "not JSON");
        let saved =
            fs::read_to_string(dir.path().join("out/case-a/app/call-01.output.txt")).unwrap();
        assert_eq!(saved, "not JSON");
        ledger
            .fail_last("case-a", Arm::App, "invalid_model_output")
            .unwrap();
        assert_eq!(ledger.value["callsReserved"], 1);
        assert_eq!(ledger.value["callsCompleted"], 1);
        assert_eq!(ledger.value["outputs"][0]["state"], "failed");
        assert_eq!(
            ledger.value["outputs"][0]["failureCategory"],
            "invalid_model_output"
        );
    }

    #[test]
    fn oversized_raw_output_is_bounded_and_never_returns_as_a_success() {
        let dir = tempdir().unwrap();
        let mut ledger = ledger_fixture(&dir.path().join("ledger.json"), &dir.path().join("out"));
        let raw = format!("{}ø", "x".repeat(CALL_OUTPUT_LIMIT - 1));
        let result = ledger.invoke(
            "case-a",
            Arm::Baseline,
            r#"{"repositoryFacts":[],"knowledgePages":[]}"#,
            || Ok(raw),
        );
        assert_eq!(result.unwrap_err(), "response_limit");
        let output = fs::read(
            dir.path()
                .join("out/case-a/controlled-baseline/call-01.output.txt"),
        )
        .unwrap();
        assert!(output.len() <= CALL_OUTPUT_LIMIT);
        assert!(std::str::from_utf8(&output).is_ok());
        assert_eq!(ledger.value["outputs"][0]["state"], "failed");
        assert_eq!(
            ledger.value["outputs"][0]["failureCategory"],
            "response_limit"
        );
        assert_eq!(ledger.value["outputs"][0]["truncated"], true);
        assert_eq!(
            ledger.value["outputs"][0]["originalUtf8Bytes"],
            CALL_OUTPUT_LIMIT + 1
        );
    }

    #[test]
    fn empty_baseline_final_is_retained_but_marked_failed() {
        let dir = tempdir().unwrap();
        let mut ledger = ledger_fixture(&dir.path().join("ledger.json"), &dir.path().join("out"));
        let raw = ledger
            .invoke(
                "case-a",
                Arm::Baseline,
                r#"{"repositoryFacts":[],"knowledgePages":[]}"#,
                || Ok(" \n\t".into()),
            )
            .unwrap();
        assert!(require_baseline_advice(&mut ledger, "case-a", &raw).is_err());
        assert_eq!(
            fs::read_to_string(
                dir.path()
                    .join("out/case-a/controlled-baseline/call-01.output.txt")
            )
            .unwrap(),
            " \n\t"
        );
        assert_eq!(ledger.value["outputs"][0]["state"], "failed");
        assert_eq!(
            ledger.value["outputs"][0]["failureCategory"],
            "empty_model_output"
        );
    }

    #[test]
    fn reservation_limits_each_arm_and_never_reuses_an_attempt_after_failure() {
        let dir = tempdir().unwrap();
        let mut ledger = ledger_fixture(&dir.path().join("ledger.json"), &dir.path().join("out"));
        for _ in 0..MAX_CALLS {
            let _ = ledger.invoke(
                "case-a",
                Arm::App,
                r#"{"repositoryFacts":[],"knowledgePages":[]}"#,
                || Err("secret stderr must not persist".into()),
            );
        }
        assert_eq!(ledger.value["callsReserved"], MAX_CALLS);
        assert!(ledger
            .invoke(
                "case-a",
                Arm::App,
                r#"{"repositoryFacts":[],"knowledgePages":[]}"#,
                || Ok("retry".into())
            )
            .is_err());
        assert_eq!(ledger.value["callsReserved"], MAX_CALLS);
        assert_eq!(ledger.value["outputs"].as_array().unwrap().len(), MAX_CALLS);
        assert!(!fs::read_to_string(dir.path().join("ledger.json"))
            .unwrap_or_default()
            .contains("secret stderr"));
    }

    #[test]
    fn incomplete_signed_identity_fails_before_an_inference_transport_can_be_entered() {
        let dir = tempdir().unwrap();
        let mut plan: QualificationPlan = serde_json::from_value(json!({
            "formatVersion":1,"corpusRoot":dir.path(),"pinnedInputsPath":dir.path().join("pinned-inputs.json"),
            "trialLedgerPath":dir.path().join("trial-ledger.json"),"artifactRoot":dir.path().join("trial-run"),
            "activationPath":dir.path().join("activation.json"),"publicPayloadPath":dir.path().join("payload.json"),
            "expectedPublicPayloadFileSha256":PUBLIC_PAYLOAD_SHA256,"expectedPublicCoreSha256":PUBLIC_CORE_SHA256,
            "expectedPublicKeySha256":PUBLIC_KEY_SHA256,"expectedBundleSha256":"","expectedManifestSha256":"",
            "expectedVersion":"","expectedSequence":0,"expectedPublishedAt":"",
            "selection":{"provider":"codex","model":"gpt-6-luna","reasoningEffort":"medium"}
        })).unwrap();
        let mut transports = 0;
        let result = validate_plan(&plan).and_then(|_| {
            transports += 1;
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(transports, 0);
        plan.expected_bundle_sha256 = "populated".into();
        assert!(validate_plan(&plan).is_err());
    }

    #[test]
    fn production_key_signature_failure_keeps_injected_transport_unentered() {
        let dir = tempdir().unwrap();
        let runtime = TaskmasterRuntime::open(dir.path().join("runtime"));
        let invalid_activation = b"{\"activationFormatVersion\":1,\"bundleEnvelopeBase64\":\"e30=\",\"manifestEnvelopeBase64\":\"e30=\"}";
        let mut transports = 0;
        let result = runtime
            .activate_knowledge(invalid_activation)
            .and_then(|_| {
                transports += 1;
                Ok(())
            });
        assert!(result.is_err());
        assert_eq!(transports, 0);
    }

    #[test]
    fn nonempty_trial_ledger_cannot_be_restarted_or_reset() {
        let dir = tempdir().unwrap();
        let ledger_path = dir.path().join("trial-ledger.json");
        fs::write(&ledger_path, json!({"formatVersion":1,"callsReserved":1,"callsCompleted":0,
            "authorization":{"callLimit":60,"appCallsPerCase":6,"baselineCallsPerCase":6},
            "outputs":[{"caseId":"positive-entry-command","arm":"app","call":1,"state":"reserved"}]}).to_string()).unwrap();
        let plan = QualificationPlan {
            format_version: 1,
            corpus_root: dir.path().into(),
            pinned_inputs_path: dir.path().join("pinned-inputs.json"),
            trial_ledger_path: ledger_path.clone(),
            artifact_root: dir.path().join("trial-run"),
            activation_path: dir.path().join("activation.json"),
            public_payload_path: dir.path().join("payload.json"),
            expected_public_payload_file_sha256: PUBLIC_PAYLOAD_SHA256.into(),
            expected_public_core_sha256: PUBLIC_CORE_SHA256.into(),
            expected_public_key_sha256: PUBLIC_KEY_SHA256.into(),
            expected_bundle_sha256: "bundle".into(),
            expected_manifest_sha256: "manifest".into(),
            expected_version: "brain-4".into(),
            expected_sequence: 4,
            expected_published_at: "2026-11-12T18:03:27.297Z".into(),
            selection: Selection {
                provider: "codex".into(),
                model: "gpt-6-luna".into(),
                reasoning_effort: "medium".into(),
            },
        };
        assert!(Ledger::begin(&plan).is_err());
        let ledger: Value = serde_json::from_slice(&fs::read(ledger_path).unwrap()).unwrap();
        assert_eq!(ledger["callsReserved"], 1);
        assert_eq!(ledger["outputs"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn trial_ledger_requires_exact_publication_approval_identity() {
        let dir = tempdir().unwrap();
        let plan = plan_for_ledger(dir.path());
        let approved = approved_trial_ledger(&plan);
        assert!(validate_trial_ledger(&approved, &plan).is_ok());

        let mut denied = approved.clone();
        denied["authorization"]["publicationApproved"] = json!(false);
        assert!(validate_trial_ledger(&denied, &plan).is_err());

        let mut wrong_core = approved.clone();
        wrong_core["guidance"]["publicCoreSha256"] = json!("wrong");
        assert!(validate_trial_ledger(&wrong_core, &plan).is_err());

        let mut wrong_key = approved;
        wrong_key["guidance"]["signedBundleIdentity"]["publicKeySha256"] = json!("wrong");
        assert!(validate_trial_ledger(&wrong_key, &plan).is_err());
    }

    #[test]
    fn pinned_fixture_verification_rejects_an_unlisted_file() {
        let dir = tempdir().unwrap();
        let corpus = dir.path();
        let root = corpus.join(CASES[0]);
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("README.md"), "pinned readme\n").unwrap();
        fs::write(root.join("src/main.js"), "console.log('pinned')\n").unwrap();
        let git = |args: &[&str]| {
            let output = Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(args)
                .output()
                .unwrap();
            assert!(output.status.success(), "git command failed");
            output
        };
        let init = Command::new("git")
            .arg("init")
            .arg("--quiet")
            .arg(&root)
            .output()
            .unwrap();
        assert!(init.status.success());
        git(&["config", "user.email", "qualification@example.invalid"]);
        git(&["config", "user.name", "Qualification Fixture"]);
        git(&["add", "README.md", "src/main.js"]);
        git(&["commit", "--quiet", "-m", "fixture"]);
        let revision = String::from_utf8(git(&["rev-parse", "HEAD"]).stdout)
            .unwrap()
            .trim()
            .to_owned();
        let mut hashes = Map::new();
        for relative in ["README.md", "src/main.js"] {
            hashes.insert(
                relative.into(),
                json!(sha256(&fs::read(root.join(relative)).unwrap())),
            );
        }
        let pinned = json!({"cases":{CASES[0]:{
            "revision":revision,"trackedFiles":["README.md","src/main.js"],"allArtifactFileSha256":hashes
        }}});
        let plan = plan_for_ledger(corpus);
        assert!(verify_pinned_case(&plan, &pinned["cases"][CASES[0]], CASES[0]).is_ok());
        fs::write(root.join("unexpected.txt"), "unlisted").unwrap();
        assert_eq!(
            verify_pinned_case(&plan, &pinned["cases"][CASES[0]], CASES[0]).unwrap_err(),
            "fixture_file_set_changed"
        );
    }

    #[test]
    fn rendered_baseline_step_contract_accepts_empty_finish_and_rejects_populated_finish() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("README.md"), "A small fixture.\n").unwrap();
        let evidence = ProgressiveEvidence::collect(
            dir.path(),
            ContextLevel::SourceCode,
            &[],
            "2026-11-12T00:00:00Z",
        )
        .unwrap();
        let rubric = ReadinessRubric {
            id: "brain-agent-readiness".into(),
            revision: 1,
            concepts: Vec::new(),
        };
        let page = crate::taskmaster::runtime::KnowledgePage {
            id: "playbooks/taskmaster-improvement.md".into(),
            title: "Improvement playbook".into(),
            page_type: "playbook".into(),
            status: "active".into(),
            content: "Public signed guidance.".into(),
            sha256: "a".repeat(64),
            related_ids: Vec::new(),
        };
        let selected = vec![page.id.clone()];
        let rendered =
            baseline_prompt(&evidence, &rubric, &[page.clone()], &selected, &[], false).unwrap();
        let prompt: Value = serde_json::from_str(&rendered).unwrap();
        assert!(prompt["stepRules"].as_str().is_some_and(|rules| {
            rules.contains("finish true")
                && rules.contains("repositoryReads")
                && rules.contains("knowledgePageIds")
                && rules.contains("finish false")
                && rules.contains("not retried")
        }));
        let allowed_pages = vec![page.id];
        let finish = prompt["finishExample"].clone();
        let decoded = InvestigationStep::decode(
            &finish.to_string(),
            evidence.catalog().len(),
            &allowed_pages,
        )
        .unwrap();
        assert!(decoded.finish);
        assert!(decoded.repository_reads.is_empty());
        assert!(decoded.knowledge_page_ids.is_empty());

        let mut invalid_reads = finish.clone();
        invalid_reads["repositoryReads"] = json!([{"fileId":0,"offset":0,"length":1}]);
        assert!(InvestigationStep::decode(
            &invalid_reads.to_string(),
            evidence.catalog().len(),
            &allowed_pages,
        )
        .is_err());
        let mut invalid_pages = finish;
        invalid_pages["knowledgePageIds"] = json!([allowed_pages[0]]);
        assert!(InvestigationStep::decode(
            &invalid_pages.to_string(),
            evidence.catalog().len(),
            &allowed_pages,
        )
        .is_err());
    }
}
