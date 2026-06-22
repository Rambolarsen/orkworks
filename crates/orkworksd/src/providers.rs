use std::collections::HashMap;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::peon;

// --- Enums ---

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderCapacityState {
    Healthy,
    Degraded,
    Capped,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProviderEffectiveState {
    Healthy,
    Degraded,
    Capped,
    Unknown,
    Disabled,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PeonScope {
    Session,
    Repo,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AttemptOutcome {
    SkippedDisabled,
    SkippedCapped,
    Succeeded,
    Failed,
}

// --- Settings types ---

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderSettingsEntry {
    pub id: String,
    pub enabled: bool,
    #[serde(rename = "fallbackOrder")]
    pub fallback_order: usize,
    #[serde(rename = "peonModel")]
    pub peon_model: Option<String>,
    #[serde(rename = "defaultState")]
    pub default_state: ProviderCapacityState,
    #[serde(rename = "overrideState")]
    pub override_state: Option<ProviderCapacityState>,
}

impl ProviderSettingsEntry {
    pub fn effective_state(&self) -> ProviderEffectiveState {
        if !self.enabled {
            return ProviderEffectiveState::Disabled;
        }
        let state = self.override_state.as_ref().unwrap_or(&self.default_state);
        match state {
            ProviderCapacityState::Healthy => ProviderEffectiveState::Healthy,
            ProviderCapacityState::Degraded => ProviderEffectiveState::Degraded,
            ProviderCapacityState::Capped => ProviderEffectiveState::Capped,
            ProviderCapacityState::Unknown => ProviderEffectiveState::Unknown,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProviderSettingsPayload {
    pub version: u8,
    pub revision: u64,
    pub providers: Vec<ProviderSettingsEntry>,
}

impl Default for ProviderSettingsPayload {
    fn default() -> Self {
        Self { version: 1, revision: 0, providers: vec![] }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ProviderApplyStatus {
    #[serde(rename = "appliedRevision")]
    pub applied_revision: Option<u64>,
    #[serde(rename = "appliedAt")]
    pub applied_at: Option<String>,
    #[serde(rename = "lastApplyError")]
    pub last_apply_error: Option<String>,
}

// --- Registry types ---

#[derive(Clone, Debug)]
pub struct ProviderDefinition {
    pub id: &'static str,
    pub label: &'static str,
    pub command: &'static str,
    pub default_args: &'static [&'static str],
    pub model_arg_template: Option<&'static str>,
    pub supports_model: bool,
    pub timeout_secs: u64,
}

pub fn builtin_provider_registry() -> Vec<ProviderDefinition> {
    vec![
        ProviderDefinition {
            id: "opencode",
            label: "OpenCode",
            command: "opencode",
            default_args: &["run", "--pure"],
            model_arg_template: Some("--model={model}"),
            supports_model: true,
            timeout_secs: 30,
        },
        ProviderDefinition {
            id: "claude-code",
            label: "Claude Code",
            command: "claude",
            default_args: &["-p"],
            model_arg_template: Some("--model={model}"),
            supports_model: true,
            timeout_secs: 30,
        },
    ]
}

// --- Runtime types ---

#[derive(Clone, Debug, Default, Serialize)]
pub struct ProviderRuntimeEntry {
    #[serde(rename = "fallbackStep")]
    pub fallback_step: Option<usize>,
    #[serde(rename = "lastErrorSummary")]
    pub last_error_summary: Option<String>,
    #[serde(rename = "resetHint")]
    pub reset_hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderObservation {
    #[serde(rename = "providerId")]
    pub provider_id: String,
    #[serde(rename = "providerLabel")]
    pub provider_label: String,
    #[serde(rename = "providerModel")]
    pub provider_model: Option<String>,
    #[serde(rename = "providerState")]
    pub provider_state: String,
}

pub struct AttemptRecord {
    pub provider_id: String,
    pub outcome: AttemptOutcome,
    pub step: usize,
}

pub struct ProviderRunResult {
    pub inference: Option<peon::PeonInference>,
    pub winning_provider_id: Option<String>,
    pub observation: Option<ProviderObservation>,
    pub attempts: Vec<AttemptRecord>,
    pub runtime: HashMap<String, ProviderRuntimeEntry>,
}

// --- GET /providers response type ---

#[derive(Serialize)]
pub struct ProviderEntry {
    pub id: String,
    pub label: String,
    pub enabled: bool,
    #[serde(rename = "fallbackOrder")]
    pub fallback_order: usize,
    #[serde(rename = "effectiveState")]
    pub effective_state: String,
    #[serde(rename = "peonModel")]
    pub peon_model: Option<String>,
    pub runtime: ProviderRuntimeEntry,
}

#[derive(Serialize)]
pub struct ProvidersResponse {
    pub providers: Vec<ProviderEntry>,
    #[serde(rename = "appliedRevision")]
    pub applied_revision: Option<u64>,
}

// --- Invocation abstraction ---

struct InvocationResult {
    success: bool,
    stdout: String,
    stderr: String,
}

trait ProviderRunner: Send + Sync {
    fn run(
        &self,
        id: &str,
        command: &str,
        args: &[String],
        prompt: &str,
        timeout_secs: u64,
    ) -> InvocationResult;
}

struct ProcessRunner;

impl ProviderRunner for ProcessRunner {
    fn run(&self, id: &str, command: &str, args: &[String], prompt: &str, timeout_secs: u64) -> InvocationResult {
        let mut cmd = Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Peon({}): failed to spawn: {e}", id);
                return InvocationResult { success: false, stdout: String::new(), stderr: e.to_string() };
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(prompt.as_bytes()) {
                tracing::warn!("Peon({}): failed to write prompt: {e}", id);
                return InvocationResult { success: false, stdout: String::new(), stderr: e.to_string() };
            }
        }

        let pid = child.id();
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(child.wait_with_output());
        });

        let output = match rx.recv_timeout(Duration::from_secs(timeout_secs)) {
            Ok(Ok(out)) => out,
            _ => {
                let _ = Command::new("kill").arg(pid.to_string()).output();
                tracing::warn!("Peon({}): timed out", id);
                return InvocationResult { success: false, stdout: String::new(), stderr: "timed out".to_string() };
            }
        };

        InvocationResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

// --- ProviderManager ---

pub struct ProviderManager {
    registry: Vec<ProviderDefinition>,
    settings: Arc<RwLock<ProviderSettingsPayload>>,
    applied_revision: Arc<RwLock<Option<u64>>>,
    runtime: Arc<RwLock<HashMap<String, ProviderRuntimeEntry>>>,
    runner: Arc<dyn ProviderRunner>,
}

impl ProviderManager {
    pub fn new() -> Self {
        Self {
            registry: builtin_provider_registry(),
            settings: Arc::new(RwLock::new(ProviderSettingsPayload::default())),
            applied_revision: Arc::new(RwLock::new(None)),
            runtime: Arc::new(RwLock::new(HashMap::new())),
            runner: Arc::new(ProcessRunner),
        }
    }

    pub fn apply_settings(&self, settings: ProviderSettingsPayload) -> ProviderApplyStatus {
        let revision = settings.revision;
        {
            let mut guard = self.settings.write().unwrap();
            *guard = settings;
        }
        {
            let mut guard = self.applied_revision.write().unwrap();
            *guard = Some(revision);
        }
        ProviderApplyStatus {
            applied_revision: Some(revision),
            applied_at: Some(chrono_now()),
            last_apply_error: None,
        }
    }

    pub fn get_providers_response(&self) -> ProvidersResponse {
        let settings = self.settings.read().unwrap().clone();
        let applied_revision = *self.applied_revision.read().unwrap();
        let runtime = self.runtime.read().unwrap().clone();

        let providers = settings.providers.iter().map(|entry| {
            let effective = entry.effective_state();
            let effective_str = match effective {
                ProviderEffectiveState::Healthy => "healthy",
                ProviderEffectiveState::Degraded => "degraded",
                ProviderEffectiveState::Capped => "capped",
                ProviderEffectiveState::Unknown => "unknown",
                ProviderEffectiveState::Disabled => "disabled",
            };
            let label = self.registry.iter()
                .find(|d| d.id == entry.id.as_str())
                .map(|d| d.label.to_string())
                .unwrap_or_else(|| entry.id.clone());

            ProviderEntry {
                id: entry.id.clone(),
                label,
                enabled: entry.enabled,
                fallback_order: entry.fallback_order,
                effective_state: effective_str.to_string(),
                peon_model: entry.peon_model.clone(),
                runtime: runtime.get(&entry.id).cloned().unwrap_or_default(),
            }
        }).collect();

        ProvidersResponse { providers, applied_revision }
    }

    pub fn run_inference(&self, _scope: PeonScope, output: &[String]) -> ProviderRunResult {
        let settings = self.settings.read().unwrap().clone();
        let prompt = peon::build_prompt(output);

        let mut attempts = Vec::new();
        let mut runtime: HashMap<String, ProviderRuntimeEntry> = HashMap::new();

        let mut ordered_entries: Vec<&ProviderSettingsEntry> = settings.providers.iter().collect();
        ordered_entries.sort_by_key(|e| e.fallback_order);

        for (step_idx, entry) in ordered_entries.iter().enumerate() {
            let step = step_idx + 1;

            if !entry.enabled {
                attempts.push(AttemptRecord {
                    provider_id: entry.id.clone(),
                    outcome: AttemptOutcome::SkippedDisabled,
                    step,
                });
                continue;
            }

            if entry.effective_state() == ProviderEffectiveState::Capped {
                attempts.push(AttemptRecord {
                    provider_id: entry.id.clone(),
                    outcome: AttemptOutcome::SkippedCapped,
                    step,
                });
                continue;
            }

            let definition = match self.registry.iter().find(|d| d.id == entry.id.as_str()) {
                Some(d) => d,
                None => {
                    tracing::warn!("Peon: no registry entry for provider {}", entry.id);
                    attempts.push(AttemptRecord {
                        provider_id: entry.id.clone(),
                        outcome: AttemptOutcome::Failed,
                        step,
                    });
                    continue;
                }
            };

            let mut args: Vec<String> = definition.default_args.iter().map(|s| s.to_string()).collect();
            if definition.supports_model {
                if let Some(model) = &entry.peon_model {
                    if let Some(template) = definition.model_arg_template {
                        args.push(template.replace("{model}", model));
                    }
                }
            }

            let result = self.runner.run(&entry.id, definition.command, &args, &prompt, definition.timeout_secs);

            if result.success {
                if let Some(inference) = peon::parse_inference(&result.stdout) {
                    let rt_entry = ProviderRuntimeEntry { fallback_step: Some(step), ..Default::default() };
                    attempts.push(AttemptRecord {
                        provider_id: entry.id.clone(),
                        outcome: AttemptOutcome::Succeeded,
                        step,
                    });
                    runtime.insert(entry.id.clone(), rt_entry);
                    *self.runtime.write().unwrap() = runtime.clone();
                    let effective = entry.effective_state();
                    let state_str = match effective {
                        ProviderEffectiveState::Healthy => "healthy",
                        ProviderEffectiveState::Degraded => "degraded",
                        ProviderEffectiveState::Capped => "capped",
                        ProviderEffectiveState::Unknown => "unknown",
                        ProviderEffectiveState::Disabled => "disabled",
                    };
                    let observation = ProviderObservation {
                        provider_id: entry.id.clone(),
                        provider_label: definition.label.to_string(),
                        provider_model: entry.peon_model.clone(),
                        provider_state: state_str.to_string(),
                    };
                    return ProviderRunResult {
                        inference: Some(inference),
                        winning_provider_id: Some(entry.id.clone()),
                        observation: Some(observation),
                        attempts,
                        runtime,
                    };
                }
            }

            let stderr = result.stderr.trim().to_string();
            let rt_entry = if !stderr.is_empty() {
                let (summary, hint) = parse_error_hint(&stderr);
                ProviderRuntimeEntry { fallback_step: Some(step), last_error_summary: Some(summary), reset_hint: hint }
            } else {
                ProviderRuntimeEntry {
                    fallback_step: Some(step),
                    last_error_summary: Some(format!("provider {} failed", entry.id)),
                    ..Default::default()
                }
            };

            attempts.push(AttemptRecord {
                provider_id: entry.id.clone(),
                outcome: AttemptOutcome::Failed,
                step,
            });
            runtime.insert(entry.id.clone(), rt_entry);
        }

        *self.runtime.write().unwrap() = runtime.clone();
        ProviderRunResult { inference: None, winning_provider_id: None, observation: None, attempts, runtime }
    }
}

fn parse_error_hint(stderr: &str) -> (String, Option<String>) {
    if let Some(comma_pos) = stderr.find(',') {
        let summary = stderr[..comma_pos].trim().to_string();
        let after = stderr[comma_pos + 1..].trim();
        let hint = if after.is_empty() { None } else { Some(after.to_string()) };
        (summary, hint)
    } else {
        (stderr.to_string(), None)
    }
}

fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Approximate ISO-8601 format
    let (y, mo, d, h, mi, s) = secs_to_datetime(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn secs_to_datetime(secs: u64) -> (u64, u64, u64, u64, u64, u64) {
    let s = secs % 60;
    let total_min = secs / 60;
    let mi = total_min % 60;
    let total_hours = total_min / 60;
    let h = total_hours % 24;
    let total_days = total_hours / 24;

    // Days since Unix epoch (1970-01-01)
    let mut year = 1970u64;
    let mut days = total_days;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let month_days: &[u64] = if is_leap(year) {
        &[31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        &[31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u64;
    for &md in month_days {
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }

    (year, month, days + 1, h, mi, s)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

// --- Test helpers ---

#[cfg(test)]
pub struct FakeProvider {
    pub id: &'static str,
    stdout_val: String,
    stderr_val: String,
    exit_code: i32,
    sleep_ms: u64,
    call_count: Option<std::sync::Arc<std::sync::atomic::AtomicUsize>>,
}

#[cfg(test)]
impl FakeProvider {
    pub fn new(id: &'static str) -> Self {
        Self { id, stdout_val: String::new(), stderr_val: String::new(), exit_code: 0, sleep_ms: 0, call_count: None }
    }

    pub fn stdout(mut self, s: &str) -> Self {
        self.stdout_val = s.to_string();
        self
    }

    pub fn stderr(mut self, s: &str) -> Self {
        self.stderr_val = s.to_string();
        self
    }

    pub fn exit_code(mut self, code: i32) -> Self {
        self.exit_code = code;
        self
    }

    pub fn sleep_ms(mut self, ms: u64) -> Self {
        self.sleep_ms = ms;
        self
    }

    pub fn with_counter(mut self, counter: std::sync::Arc<std::sync::atomic::AtomicUsize>) -> Self {
        self.call_count = Some(counter);
        self
    }
}

#[cfg(test)]
struct FakeRunner {
    specs: HashMap<String, FakeProvider>,
}

#[cfg(test)]
impl ProviderRunner for FakeRunner {
    fn run(&self, id: &str, _command: &str, _args: &[String], _prompt: &str, _timeout_secs: u64) -> InvocationResult {
        match self.specs.get(id) {
            Some(spec) => {
                if let Some(ref counter) = spec.call_count {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                if spec.sleep_ms > 0 {
                    std::thread::sleep(std::time::Duration::from_millis(spec.sleep_ms));
                }
                InvocationResult {
                    success: spec.exit_code == 0,
                    stdout: spec.stdout_val.clone(),
                    stderr: spec.stderr_val.clone(),
                }
            }
            None => InvocationResult {
                success: false,
                stdout: String::new(),
                stderr: format!("no fake configured for {id}"),
            },
        }
    }
}

#[cfg(test)]
impl ProviderManager {
    pub fn for_tests(settings: ProviderSettingsPayload, fakes: Vec<FakeProvider>) -> Self {
        let specs: HashMap<String, FakeProvider> =
            fakes.into_iter().map(|f| (f.id.to_string(), f)).collect();
        Self {
            registry: builtin_provider_registry(),
            settings: Arc::new(RwLock::new(settings)),
            applied_revision: Arc::new(RwLock::new(None)),
            runtime: Arc::new(RwLock::new(HashMap::new())),
            runner: Arc::new(FakeRunner { specs }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestEntryBuilder {
        id: &'static str,
        enabled: bool,
        fallback_order: usize,
        peon_model: Option<String>,
        default_state: ProviderCapacityState,
        override_state: Option<ProviderCapacityState>,
    }

    impl TestEntryBuilder {
        fn new(id: &'static str) -> Self {
            let fallback_order = match id {
                "opencode" => 0,
                "claude-code" => 1,
                _ => 99,
            };
            Self {
                id,
                enabled: true,
                fallback_order,
                peon_model: None,
                default_state: ProviderCapacityState::Healthy,
                override_state: None,
            }
        }

        fn enabled(mut self, v: bool) -> Self { self.enabled = v; self }
        fn default_state(mut self, s: ProviderCapacityState) -> Self { self.default_state = s; self }
        fn override_state(mut self, s: Option<ProviderCapacityState>) -> Self { self.override_state = s; self }

        fn build(self) -> ProviderSettingsEntry {
            ProviderSettingsEntry {
                id: self.id.to_string(),
                enabled: self.enabled,
                fallback_order: self.fallback_order,
                peon_model: self.peon_model,
                default_state: self.default_state,
                override_state: self.override_state,
            }
        }
    }

    fn entry(id: &'static str) -> TestEntryBuilder {
        TestEntryBuilder::new(id)
    }

    fn sample_settings(builders: Vec<TestEntryBuilder>) -> ProviderSettingsPayload {
        ProviderSettingsPayload {
            version: 1,
            revision: 1,
            providers: builders.into_iter().map(|b| b.build()).collect(),
        }
    }

    fn fake_provider(id: &'static str) -> FakeProvider {
        FakeProvider::new(id)
    }

    fn registry_with(fakes: Vec<FakeProvider>) -> Vec<FakeProvider> {
        fakes
    }

    #[test]
    fn skips_disabled_and_capped_providers_before_spawn() {
        let manager = ProviderManager::for_tests(
            sample_settings(vec![
                entry("opencode").enabled(false).default_state(ProviderCapacityState::Healthy),
                entry("claude-code").override_state(Some(ProviderCapacityState::Capped)),
            ]),
            registry_with(vec![
                fake_provider("opencode"),
                fake_provider("claude-code"),
            ]),
        );

        let result = manager.run_inference(PeonScope::Session, &["terminal line".to_string()]);

        assert!(result.inference.is_none());
        assert_eq!(result.attempts.len(), 2);
        assert_eq!(result.attempts[0].outcome, AttemptOutcome::SkippedDisabled);
        assert_eq!(result.attempts[1].outcome, AttemptOutcome::SkippedCapped);
    }

    #[test]
    fn falls_back_to_second_provider_after_primary_quota_failure() {
        let manager = ProviderManager::for_tests(
            sample_settings(vec![
                entry("opencode"),
                entry("claude-code"),
            ]),
            registry_with(vec![
                fake_provider("opencode").stderr("usage limit reached, resets in 2h").exit_code(1),
                fake_provider("claude-code").stdout(r#"{"observedStatus":"working","confidence":0.9}"#),
            ]),
        );

        let result = manager.run_inference(PeonScope::Session, &["terminal line".to_string()]);

        assert!(result.inference.is_some());
        assert_eq!(result.runtime["opencode"].last_error_summary.as_deref(), Some("usage limit reached"));
        assert_eq!(result.runtime["opencode"].reset_hint.as_deref(), Some("resets in 2h"));
        assert_eq!(result.runtime["claude-code"].fallback_step, Some(2));
    }

    #[test]
    fn get_providers_response_exposes_last_runtime_state() {
        let manager = ProviderManager::for_tests(
            sample_settings(vec![
                entry("opencode"),
                entry("claude-code"),
            ]),
            registry_with(vec![
                fake_provider("opencode").stderr("usage limit reached, resets in 2h").exit_code(1),
                fake_provider("claude-code").stdout(r#"{"observedStatus":"working","confidence":0.9}"#),
            ]),
        );

        let _ = manager.run_inference(PeonScope::Session, &["terminal line".to_string()]);
        let response = manager.get_providers_response();

        let opencode = response.providers.iter().find(|provider| provider.id == "opencode").unwrap();
        assert_eq!(opencode.runtime.last_error_summary.as_deref(), Some("usage limit reached"));
        assert_eq!(opencode.runtime.reset_hint.as_deref(), Some("resets in 2h"));

        let claude = response.providers.iter().find(|provider| provider.id == "claude-code").unwrap();
        assert_eq!(claude.runtime.fallback_step, Some(2));
    }
}
