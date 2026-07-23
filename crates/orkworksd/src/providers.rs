use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::io::Write as IoWrite;
use std::process::{Command, Stdio};
use std::sync::{Arc, RwLock};

#[cfg(unix)]
use std::os::unix::process::CommandExt;
use std::time::Duration;

use reqwest::Client as HttpClient;
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::harness::definition::{BuiltinDocument, HarnessUserDocument, EMBEDDED_BUILTINS};
#[cfg(test)]
use crate::harness::registry::resolve_document;
use crate::harness::registry::HarnessCatalog;
use crate::peon;

// --- Ollama API types ---

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModelEntry>,
}

#[derive(Deserialize)]
struct OllamaModelEntry {
    name: String,
}

#[derive(Deserialize)]
struct OllamaGenerateResponse {
    response: String,
    #[allow(dead_code)]
    done: bool,
}

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
    #[serde(rename = "peonModel", default)]
    pub peon_model: Option<String>,
    #[serde(rename = "ollamaBaseUrl", default = "default_ollama_base_url")]
    pub ollama_base_url: String,
    pub providers: Vec<ProviderSettingsEntry>,
}

pub(crate) fn default_ollama_base_url() -> String {
    "http://127.0.0.1:11434".to_string()
}

impl Default for ProviderSettingsPayload {
    fn default() -> Self {
        Self {
            version: 1,
            revision: 0,
            peon_model: None,
            ollama_base_url: default_ollama_base_url(),
            providers: vec![],
        }
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

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OllamaVerificationStatus {
    Connected,
    ConnectedEmpty,
    Failed,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OllamaVerificationReasonCode {
    Connected,
    NoModelsReturned,
    AllModelsFiltered,
    InvalidUrl,
    Unreachable,
    Timeout,
    HttpError,
    ParseError,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct OllamaVerificationResponse {
    pub ok: bool,
    #[serde(rename = "normalizedBaseUrl")]
    pub normalized_base_url: String,
    pub status: OllamaVerificationStatus,
    #[serde(rename = "reasonCode")]
    pub reason_code: OllamaVerificationReasonCode,
    #[serde(rename = "httpStatus")]
    pub http_status: Option<u16>,
    pub models: Vec<String>,
    #[serde(rename = "excludedModels")]
    pub excluded_models: Vec<String>,
    pub diagnostic: Option<String>,
}

pub(crate) fn normalize_ollama_base_url(raw: &str) -> Result<String, String> {
    let trimmed = raw.trim().trim_end_matches('/');
    let parsed = reqwest::Url::parse(trimmed).map_err(|_| "invalid Ollama URL".to_string())?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err("Ollama URL must start with http:// or https://".to_string());
    }
    if parsed.path() != "/" || parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("Ollama URL must be origin-only with no path, query, or fragment".to_string());
    }
    Ok(parsed.origin().unicode_serialization())
}

pub(crate) fn filter_peon_candidate_models(mut models: Vec<String>) -> (Vec<String>, Vec<String>) {
    models.sort();
    let (excluded, included): (Vec<_>, Vec<_>) = models.into_iter().partition(|name| {
        let lower = name.to_ascii_lowercase();
        lower.contains("embed") || lower.contains("embedding")
    });
    (included, excluded)
}

#[derive(Clone, Debug, Deserialize)]
pub struct OllamaVerifyRequest {
    #[serde(rename = "baseUrl")]
    pub base_url: String,
}

fn build_ollama_verification_response(
    normalized_base_url: String,
    raw_models: Vec<String>,
) -> OllamaVerificationResponse {
    let (models, excluded_models) = filter_peon_candidate_models(raw_models);
    let reason_code = if models.is_empty() {
        if excluded_models.is_empty() {
            OllamaVerificationReasonCode::NoModelsReturned
        } else {
            OllamaVerificationReasonCode::AllModelsFiltered
        }
    } else {
        OllamaVerificationReasonCode::Connected
    };
    let status = if models.is_empty() {
        OllamaVerificationStatus::ConnectedEmpty
    } else {
        OllamaVerificationStatus::Connected
    };

    OllamaVerificationResponse {
        ok: true,
        normalized_base_url,
        status,
        reason_code,
        http_status: Some(200),
        models,
        excluded_models,
        diagnostic: None,
    }
}

fn failed_ollama_verification(
    normalized_base_url: String,
    error: reqwest::Error,
) -> OllamaVerificationResponse {
    let (reason_code, diagnostic) = if error.is_connect() {
        (
            OllamaVerificationReasonCode::Unreachable,
            format!("Ollama endpoint unreachable at {normalized_base_url}"),
        )
    } else if error.is_timeout() {
        (
            OllamaVerificationReasonCode::Timeout,
            "Ollama request timed out".to_string(),
        )
    } else {
        (
            OllamaVerificationReasonCode::HttpError,
            format!("Ollama request failed: {error}"),
        )
    };

    OllamaVerificationResponse {
        ok: false,
        normalized_base_url,
        status: OllamaVerificationStatus::Failed,
        reason_code,
        http_status: None,
        models: vec![],
        excluded_models: vec![],
        diagnostic: Some(diagnostic),
    }
}

// --- Registry types ---

#[derive(Clone, Debug)]
pub struct ProviderDefinition {
    pub id: String,
    pub label: String,
    pub command: String,
    pub default_args: Vec<String>,
    pub model_arg_template: Option<String>,
    pub supports_model: bool,
    pub timeout_secs: u64,
    pub list_models_command: Option<String>,
    pub list_models_args: Vec<String>,
    pub static_models: Vec<String>,
    pub http_list_models: bool,
}

pub fn builtin_provider_registry() -> Vec<ProviderDefinition> {
    vec![ProviderDefinition {
        id: "ollama".into(),
        label: "Ollama".into(),
        command: String::new(),
        default_args: vec![],
        model_arg_template: None,
        supports_model: false,
        timeout_secs: 30,
        list_models_command: None,
        list_models_args: vec![],
        static_models: vec![],
        http_list_models: true,
    }]
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
    #[allow(dead_code)]
    pub provider_id: String,
    #[allow(dead_code)]
    pub step: usize,
    #[allow(dead_code)]
    pub outcome: AttemptOutcome,
}

pub struct ProviderRunResult {
    pub inference: Option<peon::PeonInference>,
    pub observation: Option<ProviderObservation>,
    #[allow(dead_code)]
    pub attempts: Vec<AttemptRecord>,
    #[allow(dead_code)]
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

fn block_on_http<F: std::future::Future>(f: F) -> F::Output {
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => handle.block_on(f),
        Err(_) => {
            let rt =
                tokio::runtime::Runtime::new().expect("failed to create tokio runtime for HTTP");
            rt.block_on(f)
        }
    }
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

struct CompositeRunner {
    process: ProcessRunner,
    http: HttpRunner,
}

impl ProviderRunner for CompositeRunner {
    fn run(
        &self,
        id: &str,
        command: &str,
        args: &[String],
        prompt: &str,
        timeout_secs: u64,
    ) -> InvocationResult {
        match id {
            "ollama" => self.http.run(id, command, args, prompt, timeout_secs),
            _ => self.process.run(id, command, args, prompt, timeout_secs),
        }
    }
}

struct ProcessRunner;

impl ProviderRunner for ProcessRunner {
    fn run(
        &self,
        id: &str,
        command: &str,
        args: &[String],
        prompt: &str,
        timeout_secs: u64,
    ) -> InvocationResult {
        let mut cmd = Command::new(command);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        #[cfg(unix)]
        unsafe {
            cmd.pre_exec(|| {
                // Detach from the controlling terminal so the harness
                // subprocess cannot write to the user's PTY via /dev/tty.
                let _ = libc::setsid();
                // Close inherited file descriptors >= 3 to prevent leaks
                // into parent PTY master fds. Capped at 1024 to stay fast.
                let max_fd = libc::sysconf(libc::_SC_OPEN_MAX).max(3).min(1024);
                for fd in (3..=max_fd).rev() {
                    libc::close(fd as i32);
                }
                Ok(())
            });
        }

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!(provider = %id, error = %e, "peon: failed to spawn");
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: e.to_string(),
                };
            }
        };

        if let Some(mut stdin) = child.stdin.take() {
            if let Err(e) = stdin.write_all(prompt.as_bytes()) {
                tracing::warn!(provider = %id, error = %e, "peon: failed to write prompt");
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: e.to_string(),
                };
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
                tracing::warn!(provider = %id, "peon: provider timed out");
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: "timed out".to_string(),
                };
            }
        };

        InvocationResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }
}

struct HttpRunner {
    settings: Arc<RwLock<ProviderSettingsPayload>>,
}

impl ProviderRunner for HttpRunner {
    fn run(
        &self,
        id: &str,
        _command: &str,
        _args: &[String],
        prompt: &str,
        timeout_secs: u64,
    ) -> InvocationResult {
        let settings = self.settings.read().unwrap().clone();
        let base_url = match id {
            "ollama" => settings.ollama_base_url.clone(),
            _ => {
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("HttpRunner does not support provider {id}"),
                }
            }
        };

        let model = match &settings.peon_model {
            Some(m) if !m.is_empty() => m.clone(),
            _ => {
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: "no Ollama model selected in Peon settings".to_string(),
                }
            }
        };

        let url = format!("{base_url}/api/generate");
        let body = serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
        });

        let client = HttpClient::new();

        let request_fut = client.post(&url).json(&body).send();
        let resp = match block_on_http(async {
            tokio::time::timeout(Duration::from_secs(timeout_secs), request_fut).await
        }) {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                let msg = if e.is_connect() {
                    format!("Ollama endpoint unreachable at {base_url}")
                } else if e.is_timeout() {
                    "Ollama generate request timed out".to_string()
                } else {
                    format!("Ollama generate request failed: {e}")
                };
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: msg,
                };
            }
            Err(_) => {
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: "Ollama generate request timed out".to_string(),
                };
            }
        };

        if !resp.status().is_success() {
            let status = resp.status();
            let err_body = block_on_http(resp.text()).unwrap_or_default();
            return InvocationResult {
                success: false,
                stdout: String::new(),
                stderr: format!(
                    "Ollama returned HTTP {}: {}",
                    status.as_u16(),
                    err_body.trim()
                ),
            };
        }

        let text = match block_on_http(resp.text()) {
            Ok(t) => t,
            Err(e) => {
                return InvocationResult {
                    success: false,
                    stdout: String::new(),
                    stderr: format!("failed to read Ollama response: {e}"),
                }
            }
        };

        match serde_json::from_str::<OllamaGenerateResponse>(&text) {
            Ok(gen) => InvocationResult {
                success: true,
                stdout: gen.response,
                stderr: String::new(),
            },
            Err(e) => InvocationResult {
                success: false,
                stdout: String::new(),
                stderr: format!("failed to parse Ollama generate response: {e}"),
            },
        }
    }
}

// --- ProviderManager ---

#[derive(Clone)]
pub struct ProviderManager {
    registry: Vec<ProviderDefinition>,
    harness_catalog: Option<HarnessCatalog>,
    settings: Arc<RwLock<ProviderSettingsPayload>>,
    applied_revision: Arc<RwLock<Option<u64>>>,
    runtime: Arc<RwLock<HashMap<String, ProviderRuntimeEntry>>>,
    runner: Arc<dyn ProviderRunner>,
    session_capped: Arc<RwLock<HashMap<String, bool>>>,
    session_reset_hint: Arc<RwLock<HashMap<String, String>>>,
    session_checking: Arc<RwLock<HashSet<String>>>,
}

impl ProviderManager {
    #[cfg(test)]
    pub fn new() -> Self {
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let registry =
            Arc::new(resolve_document(&builtins, &HarnessUserDocument::default()).unwrap());
        Self::new_with_catalog(Arc::new(RwLock::new(registry)))
    }

    pub fn new_with_catalog(catalog: HarnessCatalog) -> Self {
        let settings = Arc::new(RwLock::new(ProviderSettingsPayload::default()));
        let runtime = Arc::new(RwLock::new(HashMap::new()));
        Self {
            registry: builtin_provider_registry(),
            harness_catalog: Some(catalog),
            settings: settings.clone(),
            applied_revision: Arc::new(RwLock::new(None)),
            runtime,
            runner: Arc::new(CompositeRunner {
                process: ProcessRunner,
                http: HttpRunner { settings },
            }),
            session_capped: Arc::new(RwLock::new(HashMap::new())),
            session_reset_hint: Arc::new(RwLock::new(HashMap::new())),
            session_checking: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    fn definitions(&self) -> Vec<ProviderDefinition> {
        let mut definitions = self
            .harness_catalog
            .as_ref()
            .map(|catalog| {
                catalog
                    .read()
                    .expect("harness catalog lock poisoned")
                    .providers()
                    .to_vec()
            })
            .unwrap_or_default();
        definitions.extend(self.registry.clone());
        definitions
    }

    /// Called by list_sessions after each peon scan cycle to keep provider
    /// capacity state in sync with what sessions actually observe.
    pub fn update_session_capping(
        &self,
        capped: HashMap<String, bool>,
        reset_hints: HashMap<String, String>,
        checking: HashSet<String>,
    ) {
        *self.session_capped.write().unwrap() = capped;
        *self.session_reset_hint.write().unwrap() = reset_hints;
        *self.session_checking.write().unwrap() = checking;
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
        let session_capped = self.session_capped.read().unwrap().clone();
        let session_reset_hint = self.session_reset_hint.read().unwrap().clone();
        let session_checking = self.session_checking.read().unwrap().clone();
        let definitions = self.definitions();

        let providers = settings
            .providers
            .iter()
            .map(|entry| {
                let effective = entry.effective_state();
                let session_is_capped = session_capped.get(&entry.id).copied().unwrap_or(false);
                let effective_str = if effective == ProviderEffectiveState::Disabled {
                    "disabled"
                } else if session_checking.contains(&entry.id) {
                    "checking_capacity"
                } else if session_is_capped {
                    "capped"
                } else {
                    match effective {
                        ProviderEffectiveState::Healthy => "healthy",
                        ProviderEffectiveState::Degraded => "degraded",
                        ProviderEffectiveState::Capped => "capped",
                        ProviderEffectiveState::Unknown => "unknown",
                        ProviderEffectiveState::Disabled => unreachable!(),
                    }
                };
                let label = definitions
                    .iter()
                    .find(|d| d.id == entry.id)
                    .map(|d| d.label.clone())
                    .unwrap_or_else(|| entry.id.clone());
                let mut rt = runtime.get(&entry.id).cloned().unwrap_or_default();
                if rt.reset_hint.is_none() {
                    rt.reset_hint = session_reset_hint.get(&entry.id).cloned();
                }

                ProviderEntry {
                    id: entry.id.clone(),
                    label,
                    enabled: entry.enabled,
                    fallback_order: entry.fallback_order,
                    effective_state: effective_str.to_string(),
                    runtime: rt,
                }
            })
            .collect();

        ProvidersResponse {
            providers,
            applied_revision,
        }
    }

    pub fn list_models(&self, provider_id: &str) -> Result<Vec<String>, String> {
        let definitions = self.definitions();
        let definition = definitions
            .iter()
            .find(|d| d.id == provider_id)
            .ok_or_else(|| format!("unknown provider: {provider_id}"))?;

        if definition.http_list_models {
            return self.list_models_http(provider_id);
        }

        if definition.list_models_command.is_none() || definition.list_models_args.is_empty() {
            return Ok(definition.static_models.clone());
        }

        let command = definition.list_models_command.as_deref().unwrap();
        let args = &definition.list_models_args;

        let mut child = std::process::Command::new(command)
            .args(args)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to run {command}: {e}"))?;

        let mut child_stdout = child.stdout.take().unwrap();
        let mut child_stderr = child.stderr.take().unwrap();
        let timeout = std::time::Duration::from_secs(definition.timeout_secs);

        let (tx, rx) = std::sync::mpsc::channel::<std::io::Result<(String, String)>>();
        std::thread::spawn(move || {
            let mut out = String::new();
            let mut err = String::new();
            let r1 = child_stdout.read_to_string(&mut out);
            let r2 = child_stderr.read_to_string(&mut err);
            let _ = tx.send(r1.and(r2).map(|_| (out, err)));
        });

        let receive_result = rx.recv_timeout(timeout);
        let exit_status = match child.try_wait() {
            Ok(Some(status)) => Some(status),
            _ => None,
        };

        if let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = receive_result {
            let _ = child.kill();
            let _ = child.wait();
            return Err(format!(
                "{command} timed out after {}s",
                definition.timeout_secs
            ));
        }

        let (stdout, stderr) = match receive_result {
            Ok(Ok((out, err))) => (out, err),
            Ok(Err(e)) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("failed to read {command} output: {e}"));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("failed to read {command} output"));
            }
        };

        let status = match exit_status {
            Some(s) => s,
            None => child
                .wait()
                .map_err(|e| format!("failed to wait on {command}: {e}"))?,
        };

        if !status.success() {
            let stderr = stderr.trim().to_string();
            return Err(if stderr.is_empty() {
                format!("{command} exited with status {}", status)
            } else {
                stderr
            });
        }

        let trimmed = stdout.trim();
        let models: Vec<String> = if trimmed.starts_with('[') {
            serde_json::from_str::<Vec<String>>(trimmed)
                .map_err(|e| format!("failed to parse JSON model list: {e}"))?
        } else {
            trimmed
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        };

        Ok(models)
    }

    pub fn verify_ollama(&self, base_url: &str) -> OllamaVerificationResponse {
        let normalized = match normalize_ollama_base_url(base_url) {
            Ok(value) => value,
            Err(error) => {
                return OllamaVerificationResponse {
                    ok: false,
                    normalized_base_url: base_url.trim().trim_end_matches('/').to_string(),
                    status: OllamaVerificationStatus::Failed,
                    reason_code: OllamaVerificationReasonCode::InvalidUrl,
                    http_status: None,
                    models: vec![],
                    excluded_models: vec![],
                    diagnostic: Some(error),
                };
            }
        };
        let client = HttpClient::new();
        let url = format!("{normalized}/api/tags");

        let (status, body) = match block_on_http(async {
            tokio::time::timeout(Duration::from_secs(10), async {
                let response = client.get(&url).send().await?;
                let status = response.status();
                let body = response.text().await?;
                Ok::<_, reqwest::Error>((status, body))
            })
            .await
        }) {
            Ok(Ok((status, body))) => (status, body),
            Ok(Err(error)) => {
                if error.is_body() {
                    return OllamaVerificationResponse {
                        ok: false,
                        normalized_base_url: normalized,
                        status: OllamaVerificationStatus::Failed,
                        reason_code: OllamaVerificationReasonCode::ParseError,
                        http_status: None,
                        models: vec![],
                        excluded_models: vec![],
                        diagnostic: Some(format!("failed to read Ollama response: {error}")),
                    };
                }
                return failed_ollama_verification(normalized, error);
            }
            Err(_) => {
                return OllamaVerificationResponse {
                    ok: false,
                    normalized_base_url: normalized,
                    status: OllamaVerificationStatus::Failed,
                    reason_code: OllamaVerificationReasonCode::Timeout,
                    http_status: None,
                    models: vec![],
                    excluded_models: vec![],
                    diagnostic: Some("Ollama request timed out".to_string()),
                };
            }
        };

        if !status.is_success() {
            return OllamaVerificationResponse {
                ok: false,
                normalized_base_url: normalized,
                status: OllamaVerificationStatus::Failed,
                reason_code: OllamaVerificationReasonCode::HttpError,
                http_status: Some(status.as_u16()),
                models: vec![],
                excluded_models: vec![],
                diagnostic: Some(format!("Ollama returned HTTP {}", status.as_u16())),
            };
        }

        let tags: OllamaTagsResponse = match serde_json::from_str(&body) {
            Ok(parsed) => parsed,
            Err(error) => {
                return OllamaVerificationResponse {
                    ok: false,
                    normalized_base_url: normalized,
                    status: OllamaVerificationStatus::Failed,
                    reason_code: OllamaVerificationReasonCode::ParseError,
                    http_status: Some(status.as_u16()),
                    models: vec![],
                    excluded_models: vec![],
                    diagnostic: Some(format!(
                        "failed to parse Ollama /api/tags response: {error}"
                    )),
                };
            }
        };

        build_ollama_verification_response(
            normalized,
            tags.models.into_iter().map(|model| model.name).collect(),
        )
    }

    pub fn run_inference(&self, _scope: PeonScope, output: &[String]) -> ProviderRunResult {
        self.run_inference_with_timeout(_scope, output, None)
    }

    pub fn run_inference_with_timeout(
        &self,
        _scope: PeonScope,
        output: &[String],
        timeout_secs_override: Option<u64>,
    ) -> ProviderRunResult {
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
                    step,
                    outcome: AttemptOutcome::SkippedDisabled,
                });
                continue;
            }

            if entry.effective_state() == ProviderEffectiveState::Capped {
                attempts.push(AttemptRecord {
                    provider_id: entry.id.clone(),
                    step,
                    outcome: AttemptOutcome::SkippedCapped,
                });
                continue;
            }

            let definitions = self.definitions();
            let definition = match definitions.iter().find(|d| d.id == entry.id.as_str()) {
                Some(d) => d,
                None => {
                    tracing::warn!(provider = %entry.id, "peon: no registry entry for provider");
                    attempts.push(AttemptRecord {
                        provider_id: entry.id.clone(),
                        step,
                        outcome: AttemptOutcome::Failed,
                    });
                    continue;
                }
            };

            let mut args: Vec<String> = definition.default_args.clone();
            if definition.supports_model {
                if let Some(model) = &settings.peon_model {
                    if let Some(template) = definition.model_arg_template.as_deref() {
                        args.push(template.replace("{model}", model));
                    }
                }
            }

            let timeout_secs = timeout_secs_override.unwrap_or(definition.timeout_secs);
            let result =
                self.runner
                    .run(&entry.id, &definition.command, &args, &prompt, timeout_secs);

            if result.success {
                if let Some(inference) = peon::parse_inference(&result.stdout) {
                    let rt_entry = ProviderRuntimeEntry {
                        fallback_step: Some(step),
                        ..Default::default()
                    };
                    attempts.push(AttemptRecord {
                        provider_id: entry.id.clone(),
                        step,
                        outcome: AttemptOutcome::Succeeded,
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
                        provider_label: definition.label.clone(),
                        provider_model: settings.peon_model.clone(),
                        provider_state: state_str.to_string(),
                    };
                    return ProviderRunResult {
                        inference: Some(inference),
                        observation: Some(observation),
                        attempts,
                        runtime,
                    };
                }
            }

            let stderr = result.stderr.trim().to_string();
            let rt_entry = if !stderr.is_empty() {
                let (summary, hint) = parse_error_hint(&stderr);
                ProviderRuntimeEntry {
                    fallback_step: Some(step),
                    last_error_summary: Some(summary),
                    reset_hint: hint,
                }
            } else {
                ProviderRuntimeEntry {
                    fallback_step: Some(step),
                    last_error_summary: Some(format!("provider {} failed", entry.id)),
                    ..Default::default()
                }
            };

            attempts.push(AttemptRecord {
                provider_id: entry.id.clone(),
                step,
                outcome: AttemptOutcome::Failed,
            });
            runtime.insert(entry.id.clone(), rt_entry);
        }

        *self.runtime.write().unwrap() = runtime.clone();
        ProviderRunResult {
            inference: None,
            observation: None,
            attempts,
            runtime,
        }
    }

    fn list_models_http(&self, provider_id: &str) -> Result<Vec<String>, String> {
        let settings = self.settings.read().unwrap().clone();
        let base_url = match provider_id {
            "ollama" => &settings.ollama_base_url,
            _ => return Err(format!("no HTTP base URL configured for {provider_id}")),
        };

        let url = format!("{base_url}/api/tags");
        let client = HttpClient::new();

        let request_fut = client.get(&url).send();
        let resp = match block_on_http(async {
            tokio::time::timeout(Duration::from_secs(10), request_fut).await
        }) {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                let msg = if e.is_connect() {
                    format!("Ollama endpoint unreachable at {base_url}")
                } else if e.is_timeout() {
                    format!("Ollama request timed out for {url}")
                } else {
                    format!("Ollama request failed: {e}")
                };
                return Err(msg);
            }
            Err(_) => return Err(format!("Ollama request timed out for {url}")),
        };

        if !resp.status().is_success() {
            return Err(format!("Ollama returned HTTP {}", resp.status().as_u16()));
        }

        let body = block_on_http(resp.text())
            .map_err(|e| format!("failed to read Ollama response: {e}"))?;

        let tags: OllamaTagsResponse = serde_json::from_str(&body)
            .map_err(|e| format!("failed to parse Ollama /api/tags response: {e}"))?;

        if tags.models.is_empty() {
            return Err("Ollama returned an empty model list".to_string());
        }

        let models: Vec<String> = tags.models.into_iter().map(|m| m.name).collect();
        Ok(models)
    }
}

fn parse_error_hint(stderr: &str) -> (String, Option<String>) {
    if let Some(comma_pos) = stderr.find(',') {
        let summary = stderr[..comma_pos].trim().to_string();
        let after = stderr[comma_pos + 1..].trim();
        let hint = if after.is_empty() {
            None
        } else {
            Some(after.to_string())
        };
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
        Self {
            id,
            stdout_val: String::new(),
            stderr_val: String::new(),
            exit_code: 0,
            sleep_ms: 0,
            call_count: None,
        }
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
    fn run(
        &self,
        id: &str,
        _command: &str,
        _args: &[String],
        _prompt: &str,
        timeout_secs: u64,
    ) -> InvocationResult {
        match self.specs.get(id) {
            Some(spec) => {
                if let Some(ref counter) = spec.call_count {
                    counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                }
                if spec.sleep_ms > 0 {
                    if spec.sleep_ms > timeout_secs.saturating_mul(1000) {
                        return InvocationResult {
                            success: false,
                            stdout: String::new(),
                            stderr: "timed out".to_string(),
                        };
                    }
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
        let builtins = BuiltinDocument::parse(EMBEDDED_BUILTINS).unwrap();
        let resolved =
            Arc::new(resolve_document(&builtins, &HarnessUserDocument::default()).unwrap());
        Self {
            registry: builtin_provider_registry(),
            harness_catalog: Some(Arc::new(RwLock::new(resolved))),
            settings: Arc::new(RwLock::new(settings)),
            applied_revision: Arc::new(RwLock::new(None)),
            runtime: Arc::new(RwLock::new(HashMap::new())),
            runner: Arc::new(FakeRunner { specs }),
            session_capped: Arc::new(RwLock::new(HashMap::new())),
            session_reset_hint: Arc::new(RwLock::new(HashMap::new())),
            session_checking: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub fn for_tests_with_registry(
        registry: Vec<ProviderDefinition>,
        settings: ProviderSettingsPayload,
        fakes: Vec<FakeProvider>,
    ) -> Self {
        let specs: HashMap<String, FakeProvider> =
            fakes.into_iter().map(|f| (f.id.to_string(), f)).collect();
        Self {
            registry,
            harness_catalog: None,
            settings: Arc::new(RwLock::new(settings)),
            applied_revision: Arc::new(RwLock::new(None)),
            runtime: Arc::new(RwLock::new(HashMap::new())),
            runner: Arc::new(FakeRunner { specs }),
            session_capped: Arc::new(RwLock::new(HashMap::new())),
            session_reset_hint: Arc::new(RwLock::new(HashMap::new())),
            session_checking: Arc::new(RwLock::new(HashSet::new())),
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
                default_state: ProviderCapacityState::Healthy,
                override_state: None,
            }
        }

        fn enabled(mut self, v: bool) -> Self {
            self.enabled = v;
            self
        }
        fn default_state(mut self, s: ProviderCapacityState) -> Self {
            self.default_state = s;
            self
        }
        fn override_state(mut self, s: Option<ProviderCapacityState>) -> Self {
            self.override_state = s;
            self
        }

        fn build(self) -> ProviderSettingsEntry {
            ProviderSettingsEntry {
                id: self.id.to_string(),
                enabled: self.enabled,
                fallback_order: self.fallback_order,
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
            peon_model: None,
            ollama_base_url: default_ollama_base_url(),
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
                entry("opencode")
                    .enabled(false)
                    .default_state(ProviderCapacityState::Healthy),
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
            sample_settings(vec![entry("opencode"), entry("claude-code")]),
            registry_with(vec![
                fake_provider("opencode")
                    .stderr("usage limit reached, resets in 2h")
                    .exit_code(1),
                fake_provider("claude-code")
                    .stdout(r#"{"observedStatus":"working","confidence":0.9}"#),
            ]),
        );

        let result = manager.run_inference(PeonScope::Session, &["terminal line".to_string()]);

        assert!(result.inference.is_some());
        assert_eq!(
            result.runtime["opencode"].last_error_summary.as_deref(),
            Some("usage limit reached")
        );
        assert_eq!(
            result.runtime["opencode"].reset_hint.as_deref(),
            Some("resets in 2h")
        );
        assert_eq!(result.runtime["claude-code"].fallback_step, Some(2));
    }

    #[test]
    fn get_providers_response_exposes_last_runtime_state() {
        let manager = ProviderManager::for_tests(
            sample_settings(vec![entry("opencode"), entry("claude-code")]),
            registry_with(vec![
                fake_provider("opencode")
                    .stderr("usage limit reached, resets in 2h")
                    .exit_code(1),
                fake_provider("claude-code")
                    .stdout(r#"{"observedStatus":"working","confidence":0.9}"#),
            ]),
        );

        let _ = manager.run_inference(PeonScope::Session, &["terminal line".to_string()]);
        let response = manager.get_providers_response();

        let opencode = response
            .providers
            .iter()
            .find(|provider| provider.id == "opencode")
            .unwrap();
        assert_eq!(
            opencode.runtime.last_error_summary.as_deref(),
            Some("usage limit reached")
        );
        assert_eq!(opencode.runtime.reset_hint.as_deref(), Some("resets in 2h"));

        let claude = response
            .providers
            .iter()
            .find(|provider| provider.id == "claude-code")
            .unwrap();
        assert_eq!(claude.runtime.fallback_step, Some(2));
    }

    #[test]
    fn pending_capacity_overrides_runtime_state_for_enabled_provider() {
        let manager = ProviderManager::for_tests(
            sample_settings(vec![entry("opencode")]),
            registry_with(vec![fake_provider("opencode")]),
        );

        manager.update_session_capping(
            HashMap::from([("opencode".into(), false)]),
            HashMap::new(),
            HashSet::from(["opencode".into()]),
        );

        let response = manager.get_providers_response();
        let opencode = response
            .providers
            .iter()
            .find(|provider| provider.id == "opencode")
            .unwrap();
        assert_eq!(opencode.effective_state, "checking_capacity");
    }

    #[test]
    fn disabled_provider_stays_disabled_when_pending() {
        let manager = ProviderManager::for_tests(
            sample_settings(vec![entry("opencode").enabled(false)]),
            registry_with(vec![fake_provider("opencode")]),
        );

        manager.update_session_capping(
            HashMap::from([("opencode".into(), false)]),
            HashMap::new(),
            HashSet::from(["opencode".into()]),
        );

        let response = manager.get_providers_response();
        let opencode = response
            .providers
            .iter()
            .find(|provider| provider.id == "opencode")
            .unwrap();
        assert_eq!(opencode.effective_state, "disabled");
    }

    #[test]
    fn list_models_returns_empty_when_no_list_command_configured() {
        let manager = ProviderManager::for_tests_with_registry(
            vec![ProviderDefinition {
                id: "test-provider".into(),
                label: "Test".into(),
                command: "test".into(),
                default_args: vec![],
                model_arg_template: None,
                supports_model: false,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec![],
                http_list_models: false,
            }],
            sample_settings(vec![]),
            vec![],
        );

        let result = manager.list_models("test-provider").unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn list_models_returns_static_models_when_no_command() {
        let manager = ProviderManager::for_tests_with_registry(
            vec![ProviderDefinition {
                id: "claude-code".into(),
                label: "Claude Code".into(),
                command: "claude".into(),
                default_args: vec![],
                model_arg_template: None,
                supports_model: false,
                timeout_secs: 30,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec!["sonnet".into(), "opus".into(), "haiku".into()],
                http_list_models: false,
            }],
            sample_settings(vec![]),
            vec![],
        );

        let result = manager.list_models("claude-code").unwrap();
        assert_eq!(result, vec!["sonnet", "opus", "haiku"]);
    }

    #[test]
    fn provider_manager_uses_supplied_harness_peon_configs() {
        let manager = ProviderManager::for_tests_with_registry(
            vec![ProviderDefinition {
                id: "custom-ai".into(),
                label: "Custom AI".into(),
                command: "custom-ai-peon".into(),
                default_args: vec!["infer".into()],
                model_arg_template: Some("--model={model}".into()),
                supports_model: true,
                timeout_secs: 7,
                list_models_command: None,
                list_models_args: vec![],
                static_models: vec!["custom-small".into(), "custom-large".into()],
                http_list_models: false,
            }],
            sample_settings(vec![]),
            vec![],
        );

        let result = manager.list_models("custom-ai").unwrap();
        assert_eq!(result, vec!["custom-small", "custom-large"]);
    }

    #[test]
    fn list_models_returns_error_for_unknown_provider() {
        let manager = ProviderManager::for_tests(sample_settings(vec![]), vec![]);

        let err = manager.list_models("nonexistent").unwrap_err();
        assert!(err.contains("unknown provider"));
    }

    #[test]
    fn ollama_provider_definition_in_registry() {
        let registry = builtin_provider_registry();
        let ollama = registry.iter().find(|d| d.id == "ollama");
        assert!(ollama.is_some());
        let ollama = ollama.unwrap();
        assert_eq!(ollama.label, "Ollama");
        assert!(ollama.http_list_models);
    }

    #[test]
    fn ollama_run_inference_fails_when_no_runner_configured() {
        let manager = ProviderManager::for_tests(
            ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: "http://127.0.0.1:11434".to_string(),
                providers: vec![ProviderSettingsEntry {
                    id: "ollama".to_string(),
                    enabled: true,
                    fallback_order: 0,
                    default_state: ProviderCapacityState::Healthy,
                    override_state: None,
                }],
            },
            vec![],
        );
        let result = manager.run_inference(PeonScope::Session, &["test".to_string()]);
        assert!(result.inference.is_none());
        assert_eq!(result.attempts.len(), 1);
        assert_eq!(result.attempts[0].outcome, AttemptOutcome::Failed);
    }

    #[test]
    fn ollama_disabled_is_skipped() {
        let manager = ProviderManager::for_tests(
            ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: "http://127.0.0.1:11434".to_string(),
                providers: vec![ProviderSettingsEntry {
                    id: "ollama".to_string(),
                    enabled: false,
                    fallback_order: 0,
                    default_state: ProviderCapacityState::Healthy,
                    override_state: None,
                }],
            },
            vec![],
        );
        let result = manager.run_inference(PeonScope::Session, &["test".to_string()]);
        assert!(result.inference.is_none());
        assert_eq!(result.attempts[0].outcome, AttemptOutcome::SkippedDisabled);
    }

    #[test]
    fn ollama_list_models_http_reaches_endpoint_or_fails_gracefully() {
        let manager = ProviderManager::for_tests(
            ProviderSettingsPayload {
                version: 1,
                revision: 1,
                peon_model: None,
                ollama_base_url: "http://127.0.0.1:49999".to_string(),
                providers: vec![],
            },
            vec![],
        );
        // This bypasses FakeRunner — list_models() dispatches on http_list_models
        // Uses a non-default port to avoid conflicting with a running Ollama
        let result = manager.list_models("ollama");
        assert!(
            result.is_err(),
            "expected error connecting to unused port 49999"
        );
        let e = result.unwrap_err();
        assert!(
            e.contains("unreachable") || e.contains("connection refused"),
            "expected connection refused error, got: {e}"
        );
    }

    #[test]
    fn normalize_ollama_base_url_trims_and_strips_trailing_slash() {
        let normalized = normalize_ollama_base_url(" http://127.0.0.1:11434/ ").unwrap();
        assert_eq!(normalized, "http://127.0.0.1:11434");
    }

    #[test]
    fn normalize_ollama_base_url_rejects_non_origin_urls() {
        let err = normalize_ollama_base_url("http://127.0.0.1:11434/api/tags").unwrap_err();
        assert!(err.contains("origin-only"));
    }

    #[test]
    fn filter_peon_candidate_models_excludes_embedding_names_case_insensitively() {
        let (models, excluded) = filter_peon_candidate_models(vec![
            "llama3.1:latest".into(),
            "nomic-embed-text".into(),
            "BGE-EMBED-M3:latest".into(),
        ]);

        assert_eq!(models, vec!["llama3.1:latest"]);
        assert_eq!(excluded, vec!["BGE-EMBED-M3:latest", "nomic-embed-text"]);
    }

    #[test]
    fn verify_ollama_all_models_filtered_is_connected_empty() {
        let response = build_ollama_verification_response(
            "http://127.0.0.1:11434".into(),
            vec!["nomic-embed-text".into()],
        );

        assert!(response.ok);
        assert_eq!(response.status, OllamaVerificationStatus::ConnectedEmpty);
        assert!(response.models.is_empty());
        assert_eq!(
            response.reason_code,
            OllamaVerificationReasonCode::AllModelsFiltered
        );
    }

    #[test]
    fn verify_ollama_unreachable_maps_to_failed_response() {
        let manager = ProviderManager::for_tests(ProviderSettingsPayload::default(), vec![]);

        let response = manager.verify_ollama("http://127.0.0.1:49999");
        assert!(!response.ok);
        assert_eq!(response.status, OllamaVerificationStatus::Failed);
        assert_eq!(
            response.reason_code,
            OllamaVerificationReasonCode::Unreachable
        );
    }

    #[test]
    fn verify_ollama_invalid_url_maps_to_failed_response() {
        let manager = ProviderManager::for_tests(ProviderSettingsPayload::default(), vec![]);

        let response = manager.verify_ollama("http://127.0.0.1:11434/api");
        assert!(!response.ok);
        assert_eq!(response.status, OllamaVerificationStatus::Failed);
        assert_eq!(
            response.reason_code,
            OllamaVerificationReasonCode::InvalidUrl
        );
    }
}
