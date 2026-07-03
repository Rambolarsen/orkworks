use std::collections::VecDeque;
use std::io::Write;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::warn;

#[derive(Clone, Debug)]
pub struct PeonConfig {
    pub harness: String,
    pub harness_args: Vec<String>,
    #[allow(dead_code)]
    pub model: Option<String>,
    pub interval_secs: u64,
    pub max_lines: usize,
    pub timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub enabled: bool,
}

impl PeonConfig {
    pub fn from_env() -> Self {
        let harness_args = std::env::var("PEON_HARNESS_ARGS_JSON")
            .ok()
            .and_then(|raw| match serde_json::from_str::<Vec<String>>(&raw) {
                Ok(args) => Some(args),
                Err(e) => {
                    tracing::warn!(error = %e, "PEON_HARNESS_ARGS_JSON is not a valid JSON string array");
                    None
                }
            })
            .or_else(|| {
                std::env::var("PEON_HARNESS_ARGS")
                    .ok()
                    .map(|raw| raw.split_whitespace().map(|arg| arg.to_string()).collect())
            })
            .unwrap_or_else(|| vec!["run".into(), "--pure".into()]);

        Self {
            harness: std::env::var("PEON_HARNESS").unwrap_or_else(|_| "opencode".into()),
            harness_args,
            model: std::env::var("PEON_MODEL").ok(),
            interval_secs: match std::env::var("PEON_INTERVAL") {
                Ok(raw) => match raw.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!("PEON_INTERVAL is not a valid number, using default 5");
                        5
                    }
                },
                Err(_) => 5,
            },
            max_lines: match std::env::var("PEON_MAX_LINES") {
                Ok(raw) => match raw.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!("PEON_MAX_LINES is not a valid number, using default 200");
                        200
                    }
                },
                Err(_) => 200,
            },
            timeout_secs: match std::env::var("PEON_TIMEOUT") {
                Ok(raw) => match raw.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!("PEON_TIMEOUT is not a valid number, using default 30");
                        30
                    }
                },
                Err(_) => 30,
            },
            idle_timeout_secs: match std::env::var("PEON_IDLE_TIMEOUT") {
                Ok(raw) => match raw.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!("PEON_IDLE_TIMEOUT is not a valid number, using default 15");
                        15
                    }
                },
                Err(_) => 15,
            },
            enabled: std::env::var("PEON_ENABLED")
                .ok()
                .map(|v| v == "true" || v == "1")
                .unwrap_or(true),
        }
    }
}

#[derive(Clone, Debug)]
pub struct RingBuffer {
    lines: VecDeque<String>,
    capacity: usize,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self { lines: VecDeque::new(), capacity }
    }

    pub fn push(&mut self, line: String) {
        self.lines.push_back(line);
        while self.lines.len() > self.capacity {
            self.lines.pop_front();
        }
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines.iter().cloned().collect()
    }

    pub fn last_n(&self, n: usize) -> Vec<String> {
        self.lines.iter().rev().take(n).cloned().collect()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.lines.len()
    }
}

/// Strips ANSI CSI escape sequences (e.g. \x1b[31m) so pattern matching works on raw PTY output.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') { chars.next(); }
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() { break; }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn detect_usage_limit<S: AsRef<str>>(patterns: &[S], lines: &[String]) -> bool {
    if patterns.is_empty() { return false; }
    lines.iter().any(|line| {
        let lower = strip_ansi(line).to_lowercase();
        patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str()))
    })
}

/// Returns true if recent terminal output looks like it prompted for a password or passphrase.
/// Used to suppress raw input from being stored as the session label.
pub fn looks_like_password_prompt(recent_lines: &[String]) -> bool {
    let patterns = ["password", "passphrase", "pin:"];
    recent_lines.iter().rev().take(3).any(|line| {
        let lower = strip_ansi(line).to_lowercase();
        patterns.iter().any(|p| lower.contains(p))
    })
}

/// Detects usage limit in a raw text blob (for TUI apps that use cursor positioning, not newlines).
pub fn detect_usage_limit_raw<S: AsRef<str>>(patterns: &[S], text: &str) -> bool {
    if patterns.is_empty() { return false; }
    let lower = strip_ansi(text).to_lowercase();
    patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str()))
}

/// Extracts reset hint from a raw text blob (for TUI apps that use cursor positioning, not newlines).
pub fn detect_usage_limit_hint_raw<S: AsRef<str>>(patterns: &[S], text: &str) -> Option<String> {
    if patterns.is_empty() { return None; }
    let plain = strip_ansi(text);
    let lower = plain.to_lowercase();
    if !patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str())) {
        return None;
    }
    let idx = lower.find("resets in").or_else(|| lower.find("reset in")).or_else(|| lower.find("try again at"))?;
    let fragment = &plain[idx..];
    let end = fragment.find(['.', '\n']).unwrap_or(fragment.len());
    Some(fragment[..end].trim().to_string())
}

/// Returns the "reset in X" fragment from the usage-limit line, if present.
pub fn detect_usage_limit_hint<S: AsRef<str>>(patterns: &[S], lines: &[String]) -> Option<String> {
    if patterns.is_empty() { return None; }
    lines.iter().rev().find_map(|line| {
        let plain = strip_ansi(line);
        let lower = plain.to_lowercase();
        if !patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str())) {
            return None;
        }
        let idx = lower.find("resets in").or_else(|| lower.find("reset in")).or_else(|| lower.find("try again at"))?;
        let fragment = &plain[idx..];
        let end = fragment.find(['.', '\n']).unwrap_or(fragment.len());
        Some(fragment[..end].trim().to_string())
    })
}

const SYSTEM_PROMPT: &str = "\
You are a terminal output analyzer. Analyze the following terminal session output and return a JSON object describing the session state. Only include fields you are confident about. Return ONLY valid JSON, no other text.

Available fields:
- observedStatus: one of \"waiting_for_input\", \"blocked\", \"failed\", \"done\", \"stale\", \"working\", \"idle\"
- phase: short description of current work phase
- summary: one-line summary of what's happening
- nextAction: suggested next step
- needsUserInput: boolean, true if the terminal is prompting for user input
- detectedQuestion: the question the user needs to answer
- suggestedOptions: array of possible answers
- blockerDescription: what's blocking progress
- failedCommand: the command that failed
- failedTest: the test that failed
- capacityHints: array of cap/rate-limit related strings found in output
- confidence: number 0.0 to 1.0 indicating your confidence in this analysis
- detectedHarness: name of the AI coding harness visible in the terminal (e.g. \"claude-code\", \"opencode\", \"codex\", \"aider\", \"gemini-cli\"), or omit if not detectable
- detectedModel: model identifier visible in the terminal output (e.g. \"claude-sonnet-4-5\", \"gpt-4o\"), or omit if not detectable
- harnessSessionId: the harness's internal session identifier visible in terminal output (e.g. a UUID, session hex string, or ID shown in a \"resume\" or \"continue\" prompt), or omit if not detectable

If a line starting with '[User input]:' is present, it is what the user just typed to the AI coding tool. Use it to derive a short, direct, present-tense summary of what the user is doing — like a commit-message subject line. NEVER start the summary with \"User\", \"User is\", \"User wants\", \"User asked\", \"User requested\", or \"User typed\". Examples: \"Fixing peon model detection\" not \"User is fixing peon model detection\". \"Reviewing PR feedback\" not \"User wants to review PR feedback\". Keep it under 8 words.";

const VALID_STATUSES: &[&str] = &[
    "waiting_for_input", "blocked", "failed", "done",
    "stale", "working", "idle",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PeonInference {
    #[serde(rename = "observedStatus", alias = "status")]
    pub observed_status: Option<String>,
    pub phase: Option<String>,
    pub summary: Option<String>,
    #[serde(rename = "nextAction")]
    pub next_action: Option<String>,
    #[serde(rename = "needsUserInput")]
    pub needs_user_input: Option<bool>,
    #[serde(rename = "detectedQuestion")]
    pub detected_question: Option<String>,
    #[serde(rename = "suggestedOptions")]
    pub suggested_options: Option<Vec<String>>,
    #[serde(rename = "blockerDescription")]
    pub blocker_description: Option<String>,
    #[serde(rename = "failedCommand")]
    pub failed_command: Option<String>,
    #[serde(rename = "failedTest")]
    pub failed_test: Option<String>,
    #[serde(rename = "capacityHints")]
    pub capacity_hints: Option<Vec<String>>,
    pub confidence: f64,
    #[serde(rename = "detectedHarness", default)]
    pub detected_harness: Option<String>,
    #[serde(rename = "detectedModel", default)]
    pub detected_model: Option<String>,
    #[serde(rename = "harnessSessionId", default)]
    pub harness_session_id: Option<String>,
}

pub fn extract_json(raw: &str) -> Option<String> {
    let trimmed = raw.trim();

    if trimmed.starts_with('{') {
        return Some(trimmed.to_string());
    }

    let without_fences = trimmed
        .strip_prefix("```json\n")
        .or_else(|| trimmed.strip_prefix("```json"))
        .or_else(|| trimmed.strip_prefix("```\n"))
        .or_else(|| trimmed.strip_prefix("```"))
        .unwrap_or(trimmed);

    let without_suffix = without_fences
        .strip_suffix("\n```")
        .or_else(|| without_fences.strip_suffix("```"))
        .unwrap_or(without_fences);

    if without_suffix.trim().starts_with('{') {
        Some(without_suffix.trim().to_string())
    } else {
        None
    }
}

pub fn is_valid_observed_status(status: &str) -> bool {
    VALID_STATUSES.contains(&status)
}

pub fn validate_inference(inf: &PeonInference) -> Result<(), String> {
    if inf.confidence < 0.0 || inf.confidence > 1.0 {
        return Err(format!(
            "confidence {} out of range [0.0, 1.0]",
            inf.confidence
        ));
    }

    if let Some(ref status) = inf.observed_status {
        if !VALID_STATUSES.contains(&status.as_str()) {
            return Err(format!(
                "invalid status '{}', must be one of {:?}",
                status, VALID_STATUSES
            ));
        }
    }

    Ok(())
}

fn normalize_summary(s: &str) -> String {
    let trimmed = s.trim();
    let lower = trimmed.to_lowercase();
    let prefixes = [
        "user is ",
        "user wants ",
        "user wants to ",
        "user asked ",
        "user requested ",
        "user typed ",
        "user ",
    ];
    for prefix in &prefixes {
        if lower.starts_with(prefix) {
            let rest = &trimmed[prefix.len()..];
            if rest.is_empty() {
                return trimmed.to_string();
            }
            let mut chars = rest.chars();
            let normalized = match chars.next() {
                Some(c) => c.to_uppercase().collect::<String>() + chars.as_str(),
                None => return trimmed.to_string(),
            };
            return normalized;
        }
    }
    trimmed.to_string()
}

pub fn parse_inference(stdout: &str) -> Option<PeonInference> {
    let json_str = extract_json(stdout)?;
    let mut inference: PeonInference = serde_json::from_str(&json_str).ok()?;
    validate_inference(&inference).ok()?;
    if let Some(ref summary) = inference.summary {
        inference.summary = Some(normalize_summary(summary));
    }
    Some(inference)
}

/// Returns true if Peon is allowed to overwrite the given metadata source.
/// `last_modified_secs_ago`: seconds since the metadata file was last modified.
/// None means the file doesn't exist or has no timestamp.
/// Returns true if the observed status is a terminal state that should be
/// cleared when new terminal output arrives (idle, stale, done, waiting_for_input).
pub fn is_terminal_observed_status(observed: Option<&str>) -> bool {
    matches!(observed, Some("idle" | "stale" | "done" | "waiting_for_input"))
}

pub fn should_overwrite(source: &str, last_modified_secs_ago: Option<u64>) -> bool {
    match source {
        "user" => false,
        "agent" => {
            // Overwrite agent metadata only if stale (> 5 minutes since last modify)
            last_modified_secs_ago.map(|s| s > 300).unwrap_or(false)
        }
        "peon" | "backend_inference" | "process" | "unknown" | "" => true,
        _ => true, // unknown source type, allow overwrite
    }
}

/// Seconds Peon must wait before it may overwrite a fresh `agent`-sourced status.
/// Short relative to the 5-minute window `should_overwrite` uses elsewhere: long
/// enough to avoid Peon's inference racing/flickering against a signal that just
/// landed, short enough that a deterministic hook reporting `waiting_for_input`
/// doesn't leave the UI stuck showing that for minutes after fresh terminal
/// output shows the user answered and work resumed.
const PEON_AGENT_OVERWRITE_SECS: u64 = 15;

/// Same priority gate as `should_overwrite`, for Peon's own write path
/// specifically. Every other source keeps the same rule; only the `agent`
/// staleness window is shortened, since Peon reacting to genuinely fresh
/// terminal output is exactly the correction a stuck attention signal needs.
pub fn peon_should_overwrite(source: &str, last_modified_secs_ago: Option<u64>) -> bool {
    match source {
        "agent" => last_modified_secs_ago.map(|s| s > PEON_AGENT_OVERWRITE_SECS).unwrap_or(false),
        _ => should_overwrite(source, last_modified_secs_ago),
    }
}

pub fn build_prompt(output: &[String]) -> String {
    let output_text: String = output.iter()
        .map(|l| l.as_str())
        .collect::<Vec<_>>()
        .join("\n");

    let truncated: String = if output_text.len() > 4096 {
        output_text.chars().take(4096).collect()
    } else {
        output_text
    };

    format!("{SYSTEM_PROMPT}\n\nTerminal output:\n```\n{truncated}\n```")
}

pub fn run_inference(config: &PeonConfig, output: &[String]) -> Option<PeonInference> {
    let prompt = build_prompt(output);

    let mut cmd = Command::new(&config.harness);

    for arg in &config.harness_args {
        cmd.arg(arg);
    }

    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => {
            warn!(harness = %config.harness, error = %e, "peon: failed to spawn harness");
            return None;
        }
    };

    match child.stdin.take() {
        Some(mut stdin) => {
            if let Err(e) = stdin.write_all(prompt.as_bytes()) {
                warn!(error = %e, "peon: failed to write prompt to harness stdin");
                return None;
            }
        }
        None => {
            warn!("Peon: harness stdin was unavailable");
            return None;
        }
    }

    let timeout = Duration::from_secs(config.timeout_secs);
    let pid = child.id();
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(child.wait_with_output());
    });

    let output = match rx.recv_timeout(timeout) {
        Ok(Ok(output)) => output,
        _ => {
            let _ = Command::new("kill").arg(pid.to_string()).output();
            warn!(harness = %config.harness, "peon: harness timed out or failed");
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        warn!(harness = %config.harness, status = %output.status, stderr = %stderr, "peon: harness exited with error");
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json_str = extract_json(&stdout)?;

    let inference: PeonInference = match serde_json::from_str(&json_str) {
        Ok(inf) => inf,
        Err(e) => {
            warn!(error = %e, raw = %stdout, "peon: failed to parse JSON from harness output");
            return None;
        }
    };

    if let Err(e) = validate_inference(&inference) {
        warn!(error = %e, "peon: inference validation failed");
        return None;
    }

    Some(inference)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn test_ring_buffer_push_and_snapshot() {
        let mut buf = RingBuffer::new(3);
        buf.push("line1".into());
        buf.push("line2".into());
        let snapshot = buf.snapshot();
        assert_eq!(snapshot, vec!["line1", "line2"]);
    }

    #[test]
    fn test_ring_buffer_capacity_enforcement() {
        let mut buf = RingBuffer::new(2);
        buf.push("a".into());
        buf.push("b".into());
        buf.push("c".into());
        let snapshot = buf.snapshot();
        assert_eq!(snapshot, vec!["b", "c"]);
        assert_eq!(buf.len(), 2);
    }

    #[test]
    fn test_ring_buffer_empty() {
        let buf = RingBuffer::new(5);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        let snapshot = buf.snapshot();
        assert!(snapshot.is_empty());
    }

    #[test]
    fn test_peon_config_defaults() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("PEON_ENABLED");
        std::env::remove_var("PEON_HARNESS");
        std::env::remove_var("PEON_HARNESS_ARGS");
        std::env::remove_var("PEON_MODEL");
        std::env::remove_var("PEON_INTERVAL");
        std::env::remove_var("PEON_MAX_LINES");
        std::env::remove_var("PEON_TIMEOUT");
        std::env::remove_var("PEON_IDLE_TIMEOUT");

        let config = PeonConfig::from_env();
        assert!(config.enabled);
        assert_eq!(config.harness, "opencode");
        assert_eq!(config.harness_args, vec!["run", "--pure"]);
        assert!(config.model.is_none());
        assert_eq!(config.interval_secs, 5);
        assert_eq!(config.max_lines, 200);
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.idle_timeout_secs, 15);
    }

    #[test]
    fn test_peon_config_from_env() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::set_var("PEON_ENABLED", "false");
        std::env::set_var("PEON_HARNESS", "claude");
        std::env::set_var("PEON_HARNESS_ARGS_JSON", r#"["-p","--print"]"#);
        std::env::set_var("PEON_MODEL", "haiku");
        std::env::set_var("PEON_INTERVAL", "10");
        std::env::set_var("PEON_MAX_LINES", "100");
        std::env::set_var("PEON_TIMEOUT", "60");
        std::env::set_var("PEON_IDLE_TIMEOUT", "10");

        let config = PeonConfig::from_env();
        assert!(!config.enabled);
        assert_eq!(config.harness, "claude");
        assert_eq!(config.harness_args, vec!["-p", "--print"]);
        assert_eq!(config.model, Some("haiku".into()));
        assert_eq!(config.interval_secs, 10);
        assert_eq!(config.max_lines, 100);
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.idle_timeout_secs, 10);

        std::env::remove_var("PEON_ENABLED");
        std::env::remove_var("PEON_HARNESS");
        std::env::remove_var("PEON_HARNESS_ARGS");
        std::env::remove_var("PEON_HARNESS_ARGS_JSON");
        std::env::remove_var("PEON_MODEL");
        std::env::remove_var("PEON_INTERVAL");
        std::env::remove_var("PEON_MAX_LINES");
        std::env::remove_var("PEON_TIMEOUT");
        std::env::remove_var("PEON_IDLE_TIMEOUT");
    }

    #[test]
    fn test_extract_json_plain() {
        let raw = r#"{"observedStatus": "working", "confidence": 0.9}"#;
        let result = extract_json(raw);
        assert!(result.is_some());
    }

    #[test]
    fn test_extract_json_with_markdown_fences() {
        let raw = "```json\n{\"observedStatus\": \"working\", \"confidence\": 0.8}\n```";
        let result = extract_json(raw);
        let parsed: PeonInference = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed.observed_status, Some("working".into()));
        assert!((parsed.confidence - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_extract_json_non_json_returns_none() {
        let raw = "just some terminal output, no json here";
        assert!(extract_json(raw).is_none());
    }

    #[test]
    fn test_validate_inference_valid() {
        let inf = PeonInference {
            observed_status: Some("working".into()),
            phase: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 0.85,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
        };
        assert!(validate_inference(&inf).is_ok());
    }

    #[test]
    fn test_validate_inference_invalid_status() {
        let inf = PeonInference {
            observed_status: Some("invalid_status".into()),
            phase: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 0.5,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
        };
        assert!(validate_inference(&inf).is_err());
    }

    #[test]
    fn test_validate_inference_confidence_out_of_range() {
        let inf = PeonInference {
            observed_status: None,
            phase: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: 1.5,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
        };
        assert!(validate_inference(&inf).is_err());

        let inf2 = PeonInference {
            observed_status: None,
            phase: None,
            summary: None,
            next_action: None,
            needs_user_input: None,
            detected_question: None,
            suggested_options: None,
            blocker_description: None,
            failed_command: None,
            failed_test: None,
            capacity_hints: None,
            confidence: -0.1,
            detected_harness: None,
            detected_model: None,
            harness_session_id: None,
        };
        assert!(validate_inference(&inf2).is_err());
    }

    #[test]
    fn test_peon_inference_deserialization() {
        let json = r#"{"status": "blocked", "summary": "test is failing", "needsUserInput": true, "confidence": 0.7, "harnessSessionId": "sess-abc123", "detectedHarness": "claude-code"}"#;
        let inf: PeonInference = serde_json::from_str(json).unwrap();
        assert_eq!(inf.observed_status, Some("blocked".into()));
        assert_eq!(inf.summary, Some("test is failing".into()));
        assert_eq!(inf.needs_user_input, Some(true));
        assert!((inf.confidence - 0.7).abs() < 0.001);
        assert!(inf.phase.is_none());
        assert_eq!(inf.harness_session_id.as_deref(), Some("sess-abc123"));
        assert_eq!(inf.detected_harness.as_deref(), Some("claude-code"));
    }

    #[test]
    fn test_should_overwrite_user_never() {
        assert!(!should_overwrite("user", None));       // no last_modified
        assert!(!should_overwrite("user", Some(0)));    // stale
    }

    #[test]
    fn test_should_overwrite_agent_stale_vs_fresh() {
        // agent metadata modified 10 minutes ago (stale) -> overwrite
        assert!(should_overwrite("agent", Some(600)));
        // agent metadata modified 1 minute ago (fresh) -> skip
        assert!(!should_overwrite("agent", Some(60)));
        // missing file age should be treated conservatively for agent metadata
        assert!(!should_overwrite("agent", None));
    }

    #[test]
    fn test_should_overwrite_lower_priority() {
        assert!(should_overwrite("peon", None));
        assert!(should_overwrite("peon", Some(9999)));  // always overwrite peon
        assert!(should_overwrite("backend_inference", None));
        assert!(should_overwrite("process", None));
        assert!(should_overwrite("unknown", None));
        assert!(should_overwrite("", None));             // absent source
    }

    #[test]
    fn test_peon_should_overwrite_agent_uses_short_window() {
        // agent metadata modified 1 minute ago is stale enough for Peon,
        // even though the full 5-minute should_overwrite window would say no.
        assert!(peon_should_overwrite("agent", Some(60)));
        assert!(!should_overwrite("agent", Some(60)));

        // still yields to a signal that just landed, avoiding immediate flicker.
        assert!(!peon_should_overwrite("agent", Some(5)));
        assert!(!peon_should_overwrite("agent", None));
    }

    #[test]
    fn test_peon_should_overwrite_matches_should_overwrite_for_other_sources() {
        for source in ["user", "peon", "backend_inference", "process", "unknown", ""] {
            for age in [None, Some(0), Some(60), Some(600)] {
                assert_eq!(peon_should_overwrite(source, age), should_overwrite(source, age));
            }
        }
    }

    #[test]
    fn test_is_terminal_observed_status() {
        assert!(is_terminal_observed_status(Some("idle")));
        assert!(is_terminal_observed_status(Some("stale")));
        assert!(is_terminal_observed_status(Some("done")));
        assert!(is_terminal_observed_status(Some("waiting_for_input")));
        assert!(!is_terminal_observed_status(Some("working")));
        assert!(!is_terminal_observed_status(Some("blocked")));
        assert!(!is_terminal_observed_status(Some("failed")));
        assert!(!is_terminal_observed_status(None));
        assert!(!is_terminal_observed_status(Some("unknown")));
    }

    #[test]
    fn test_run_inference_success() {
        let harness = std::env::current_dir()
            .unwrap()
            .join("tests/mock-peon-harness.sh");
        let config = PeonConfig {
            harness: harness.display().to_string(),
            harness_args: vec!["--print".into(), "-p".into()],
            model: None,
            interval_secs: 5,
            max_lines: 200,
            timeout_secs: 30,
            idle_timeout_secs: 15,
            enabled: true,
        };
        let output = vec![
            "running cargo test...".to_string(),
            "test result: ok. 5 passed; 0 failed;".to_string(),
        ];
        let result = run_inference(&config, &output);
        assert!(result.is_some());
        let inf = result.unwrap();
        assert_eq!(inf.observed_status, Some("working".into()));
        assert!((inf.confidence - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_run_inference_harness_not_found() {
        let config = PeonConfig {
            harness: "/nonexistent/harness".into(),
            harness_args: vec!["--print".into(), "-p".into()],
            model: None,
            interval_secs: 5,
            max_lines: 200,
            timeout_secs: 30,
            idle_timeout_secs: 15,
            enabled: true,
        };
        let output = vec!["some output".to_string()];
        let result = run_inference(&config, &output);
        assert!(result.is_none());
    }

    #[test]
    fn test_peon_config_uses_json_argv_env() {
        let _guard = ENV_LOCK.lock().unwrap();

        std::env::remove_var("PEON_HARNESS_ARGS");
        std::env::set_var("PEON_HARNESS_ARGS_JSON", r#"["--print","--model","haiku"]"#);

        let config = PeonConfig::from_env();
        assert_eq!(config.harness_args, vec!["--print", "--model", "haiku"]);

        std::env::remove_var("PEON_HARNESS_ARGS_JSON");
    }

    #[test]
    fn test_run_inference_sends_prompt_on_stdin_not_argv() {
        let dir = tempfile::tempdir().unwrap();
        let harness = dir.path().join("stdin-harness.sh");
        std::fs::write(
            &harness,
            r#"#!/bin/bash
if [ "$#" -ne 1 ] || [ "$1" != "--print" ]; then
  echo "unexpected args: $*" >&2
  exit 2
fi
input="$(cat)"
if ! printf '%s' "$input" | grep -q "Terminal output:"; then
  echo "missing stdin prompt" >&2
  exit 3
fi
echo '{"status":"working","confidence":0.9}'
"#,
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&harness, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        let config = PeonConfig {
            harness: harness.display().to_string(),
            harness_args: vec!["--print".into()],
            model: None,
            interval_secs: 5,
            max_lines: 200,
            timeout_secs: 30,
            idle_timeout_secs: 15,
            enabled: true,
        };
        let result = run_inference(&config, &["hello from terminal".to_string()]);
        assert!(result.is_some());
    }

    #[test]
    fn detect_usage_limit_returns_false_when_no_patterns() {
        let lines: Vec<String> = vec!["usage limit reached".into()];
        assert!(!detect_usage_limit::<&str>(&[], &lines));
    }

    #[test]
    fn detect_usage_limit_returns_true_on_match() {
        let lines = vec!["some output".into(), "usage limit reached, resets in 2h".into()];
        assert!(detect_usage_limit(&["usage limit reached"], &lines));
    }

    #[test]
    fn detect_usage_limit_is_case_insensitive() {
        let lines = vec!["Usage Limit Reached".into()];
        assert!(detect_usage_limit(&["usage limit reached"], &lines));
    }

    #[test]
    fn detect_usage_limit_returns_false_when_no_match() {
        let lines = vec!["working on task".into(), "tool call made".into()];
        assert!(!detect_usage_limit(&["usage limit reached"], &lines));
    }

    #[test]
    fn detect_usage_limit_scans_full_buffer() {
        let mut lines: Vec<String> = (0..60).map(|_| "no match".into()).collect();
        lines[0] = "usage limit reached".into(); // buried at start — still found
        assert!(detect_usage_limit(&["usage limit reached"], &lines));
    }

    #[test]
    fn detect_usage_limit_matches_anywhere_in_buffer() {
        let mut lines: Vec<String> = (0..60).map(|_| "no match".into()).collect();
        lines[15] = "usage limit reached".into();
        assert!(detect_usage_limit(&["usage limit reached"], &lines));
    }
}
