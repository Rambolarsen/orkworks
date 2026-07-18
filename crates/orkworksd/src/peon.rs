use std::collections::VecDeque;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct PeonConfig {
    pub harness: String,
    #[allow(dead_code)]
    pub harness_args: Vec<String>,
    #[allow(dead_code)]
    pub model: Option<String>,
    pub interval_secs: u64,
    pub max_lines: usize,
    #[allow(dead_code)]
    pub timeout_secs: u64,
    pub idle_timeout_secs: u64,
    pub final_scan_timeout_secs: u64,
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
            final_scan_timeout_secs: match std::env::var("PEON_FINAL_SCAN_TIMEOUT") {
                Ok(raw) => match raw.parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!("PEON_FINAL_SCAN_TIMEOUT is not a valid number, using default 2");
                        2
                    }
                },
                Err(_) => 2,
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
        if c != '\x1b' {
            out.push(c);
        } else {
            strip_ansi_escape(&mut chars, &mut out);
        }
    }
    out
}

/// Processes one escape sequence starting immediately after the ESC byte.
/// Extracted so OSC/DCS handlers can recurse when a bare ESC terminates the
/// string command and starts a new sequence (e.g. `ESC ] title ESC [ H`).
fn strip_ansi_escape<I: Iterator<Item = char>>(chars: &mut std::iter::Peekable<I>, out: &mut String) {
    match chars.peek().copied() {
        Some('[') => {
            // CSI: ESC [ params final (final = 0x40–0x7E)
            chars.next();
            let mut final_byte = '\0';
            for c2 in chars.by_ref() {
                if ('@'..='~').contains(&c2) { final_byte = c2; break; }
            }
            // Cursor-positioning finals: insert a space so adjacent screen
            // regions don't merge into a single token after stripping.
            if matches!(final_byte, 'A'|'B'|'C'|'D'|'E'|'F'|'G'|'H'|'d'|'f'|'s'|'u') {
                out.push(' ');
            }
        }
        Some(']') => {
            // OSC: ESC ] ... BEL  or  ESC \ (ST)
            chars.next();
            loop {
                match chars.next() {
                    Some('\x07') | None => break,
                    Some('\x1b') => {
                        if chars.peek() == Some(&'\\') {
                            chars.next(); // proper ST — consume backslash
                        } else {
                            // Bare ESC terminates OSC and starts a new sequence;
                            // recurse so the new sequence is handled correctly
                            // (e.g. a cursor-move CSI still emits its space).
                            strip_ansi_escape(chars, out);
                        }
                        break;
                    }
                    _ => {}
                }
            }
        }
        Some('P' | 'X' | '^' | '_') => {
            // DCS/SOS/PM/APC: string-mode sequences terminated by ST (ESC \)
            chars.next();
            loop {
                match chars.next() {
                    None => break,
                    Some('\x1b') => {
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        } else {
                            strip_ansi_escape(chars, out);
                        }
                        break;
                    }
                    _ => {}
                }
            }
        }
        Some('O') => {
            // SS3: ESC O x — function keys, consume the payload char
            chars.next(); chars.next();
        }
        Some('(' | ')') => {
            // Charset select: ESC ( x  or  ESC ) x
            chars.next(); chars.next();
        }
        Some('%') => {
            // ESC % G (select UTF-8) / ESC % @ (select default) — two-char sequences
            chars.next(); chars.next();
        }
        Some(_) => {
            // Single-char escape: ESC 7/8/M/c/= etc.
            chars.next();
        }
        None => {}
    }
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
        // Also check with whitespace collapsed: cursor-positioning moves insert
        // spaces, which can split "passphrase" → "pass phrase".
        let compact = lower.split_whitespace().collect::<String>();
        patterns.iter().any(|p| lower.contains(p) || compact.contains(p))
    })
}

/// Returns true if a completed user input line is descriptive enough to become
/// the session label. Command-prefixed input (harness slash commands, shell
/// escapes, vim ex commands, shell comments / Claude Code memory shortcuts),
/// input under 4 chars, and letter-less input (menu numbers, ports) say
/// nothing about the task — skip them and let the Peon summary win instead.
pub fn is_descriptive_input(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.chars().nth(3).is_some()
        && !trimmed.starts_with(['/', '!', ':', '#'])
        && trimmed.chars().any(char::is_alphabetic)
}

/// Detects usage limit in a raw text blob (for TUI apps that use cursor positioning, not newlines).
pub fn detect_usage_limit_raw<S: AsRef<str>>(patterns: &[S], text: &str) -> bool {
    if patterns.is_empty() { return false; }
    let lower = strip_ansi(text).to_lowercase();
    patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str()))
}

/// TUI status glyphs (spinners, separators, box drawing) that mark the end of
/// a reset hint when screen content follows it without a newline.
const HINT_STOP_GLYPHS: &[char] =
    &['✳', '✻', '✽', '✶', '●', '○', '◐', '·', '│', '╭', '╰', '─', '—'];
const HINT_MAX_CHARS: usize = 80;

/// Extracts the bounded "resets in X" fragment from ANSI-stripped text.
/// `lower` must be `plain.to_ascii_lowercase()`: ASCII lowercasing preserves
/// byte length, so indices found in `lower` are valid char boundaries in
/// `plain` (Unicode `to_lowercase` can shift byte offsets and panic here).
fn extract_reset_hint(plain: &str, lower: &str) -> Option<String> {
    let idx = lower
        .find("resets in")
        .or_else(|| lower.find("reset in"))
        .or_else(|| lower.find("resets "))
        .or_else(|| lower.find("try again at"))?;
    let fragment = &plain[idx..];
    let end = fragment.find(['.', '\n']).unwrap_or(fragment.len());
    // TUI redraws have no newline after the hint, so the rest of the redrawn
    // screen (spinner, status bar) follows directly. Stop at the first status
    // glyph and cap the length so screen content can't leak into the hint.
    let fragment = &fragment[..end];
    let end = fragment.find(HINT_STOP_GLYPHS).unwrap_or(fragment.len());
    let mut hint: String = fragment[..end].trim().chars().take(HINT_MAX_CHARS).collect();
    hint.truncate(hint.trim_end().len());
    Some(hint)
}

/// Extracts reset hint from a raw text blob (for TUI apps that use cursor positioning, not newlines).
pub fn detect_usage_limit_hint_raw<S: AsRef<str>>(patterns: &[S], text: &str) -> Option<String> {
    if patterns.is_empty() { return None; }
    let plain = strip_ansi(text);
    let lower = plain.to_ascii_lowercase();
    if !patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str())) {
        return None;
    }
    extract_reset_hint(&plain, &lower)
}

/// Returns the "reset in X" fragment from the usage-limit line, if present.
pub fn detect_usage_limit_hint<S: AsRef<str>>(patterns: &[S], lines: &[String]) -> Option<String> {
    if patterns.is_empty() { return None; }
    lines.iter().rev().find_map(|line| {
        let plain = strip_ansi(line);
        let lower = plain.to_ascii_lowercase();
        if !patterns.iter().any(|p| lower.contains(p.as_ref().to_lowercase().as_str())) {
            return None;
        }
        extract_reset_hint(&plain, &lower)
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
/// Returns true if the observed status is a finished/non-working state that
/// requires qualifying user input to leave: it should be cleared when the user
/// sends new terminal input (idle, stale, done, waiting_for_input, blocked,
/// failed), and must not be resumed to `working` by observer-only signals
/// (terminal output alone, timers, retries) per issue #170.
pub fn is_terminal_observed_status(observed: Option<&str>) -> bool {
    matches!(
        observed,
        Some("idle" | "stale" | "done" | "waiting_for_input" | "blocked" | "failed")
    )
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
        std::env::remove_var("PEON_FINAL_SCAN_TIMEOUT");

        let config = PeonConfig::from_env();
        assert!(config.enabled);
        assert_eq!(config.harness, "opencode");
        assert_eq!(config.harness_args, vec!["run", "--pure"]);
        assert!(config.model.is_none());
        assert_eq!(config.interval_secs, 5);
        assert_eq!(config.max_lines, 200);
        assert_eq!(config.timeout_secs, 30);
        assert_eq!(config.idle_timeout_secs, 15);
        assert_eq!(config.final_scan_timeout_secs, 2);
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
        std::env::set_var("PEON_FINAL_SCAN_TIMEOUT", "7");

        let config = PeonConfig::from_env();
        assert!(!config.enabled);
        assert_eq!(config.harness, "claude");
        assert_eq!(config.harness_args, vec!["-p", "--print"]);
        assert_eq!(config.model, Some("haiku".into()));
        assert_eq!(config.interval_secs, 10);
        assert_eq!(config.max_lines, 100);
        assert_eq!(config.timeout_secs, 60);
        assert_eq!(config.idle_timeout_secs, 10);
        assert_eq!(config.final_scan_timeout_secs, 7);

        std::env::remove_var("PEON_ENABLED");
        std::env::remove_var("PEON_HARNESS");
        std::env::remove_var("PEON_HARNESS_ARGS");
        std::env::remove_var("PEON_HARNESS_ARGS_JSON");
        std::env::remove_var("PEON_MODEL");
        std::env::remove_var("PEON_INTERVAL");
        std::env::remove_var("PEON_MAX_LINES");
        std::env::remove_var("PEON_TIMEOUT");
        std::env::remove_var("PEON_IDLE_TIMEOUT");
        std::env::remove_var("PEON_FINAL_SCAN_TIMEOUT");
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
        assert!(is_terminal_observed_status(Some("blocked")));
        assert!(is_terminal_observed_status(Some("failed")));
        assert!(!is_terminal_observed_status(Some("working")));
        assert!(!is_terminal_observed_status(None));
        assert!(!is_terminal_observed_status(Some("unknown")));
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

    #[test]
    fn detect_usage_limit_hint_handles_claude_reset_time() {
        let lines = vec!["You've hit your session limit · resets 5:10pm (Europe/Oslo)".into()];
        assert_eq!(
            detect_usage_limit_hint(&["you've hit your session limit"], &lines).as_deref(),
            Some("resets 5:10pm (Europe/Oslo)")
        );
    }

    #[test]
    fn detect_usage_limit_hint_raw_handles_claude_reset_time() {
        let text = "You've hit your session limit · resets 5:10pm (Europe/Oslo)";
        assert_eq!(
            detect_usage_limit_hint_raw(&["you've hit your session limit"], text).as_deref(),
            Some("resets 5:10pm (Europe/Oslo)")
        );
    }

    #[test]
    fn detect_usage_limit_hint_raw_stops_at_tui_status_glyphs() {
        // TUI redraws have no newline after the hint — the spinner and status
        // bar of the redrawn screen follow directly in the blob.
        let text = "You've hit your session limit · resets 1pm (Europe/Oslo) ✳Worked for 1s ● high · /effort ────";
        assert_eq!(
            detect_usage_limit_hint_raw(&["you've hit your session limit"], text).as_deref(),
            Some("resets 1pm (Europe/Oslo)")
        );
    }

    #[test]
    fn detect_usage_limit_hint_raw_caps_length_without_terminator() {
        let text = format!(
            "usage limit reached · resets in 2h {}",
            "trailing pane text without any glyph or period ".repeat(5)
        );
        let hint = detect_usage_limit_hint_raw(&["usage limit reached"], &text).unwrap();
        assert!(hint.starts_with("resets in 2h"));
        let len = hint.chars().count();
        assert!((70..=80).contains(&len), "cap not applied near 80: {len} ({hint})");
    }

    #[test]
    fn detect_usage_limit_hint_raw_stops_at_middle_dot_separator() {
        let text = "You've hit your session limit · resets 1pm (Europe/Oslo) · /effort ────";
        assert_eq!(
            detect_usage_limit_hint_raw(&["you've hit your session limit"], text).as_deref(),
            Some("resets 1pm (Europe/Oslo)")
        );
    }

    #[test]
    fn detect_usage_limit_hint_raw_survives_codepoints_that_shrink_when_lowercased() {
        // Kelvin sign (3 bytes) lowercases to 'k' (1 byte); with Unicode
        // to_lowercase the anchor index found in the lowered string is not a
        // char boundary in the original and slicing panics.
        let text = "\u{212A}\u{00E9} session limit reached, resets 5pm (UTC)";
        assert_eq!(
            detect_usage_limit_hint_raw(&["session limit"], text).as_deref(),
            Some("resets 5pm (UTC)")
        );
    }

    #[test]
    fn detect_usage_limit_hint_line_path_is_bounded_too() {
        let lines = vec!["You've hit your session limit · resets in 2h │ other column".into()];
        assert_eq!(
            detect_usage_limit_hint(&["you've hit your session limit"], &lines).as_deref(),
            Some("resets in 2h")
        );
    }

    #[test]
    fn descriptive_input_accepts_prose_task_text() {
        assert!(is_descriptive_input("fix the peon label capture bug"));
        assert!(is_descriptive_input("  review PR feedback  "));
        // Non-leading '!' and '#' are prose, not command prefixes.
        assert!(is_descriptive_input("fix the auth bug!"));
        assert!(is_descriptive_input("close issue #42"));
        // Exactly at the 4-char threshold.
        assert!(is_descriptive_input("docs"));
    }

    #[test]
    fn descriptive_input_rejects_command_prefixes() {
        assert!(!is_descriptive_input("/hooks"));
        assert!(!is_descriptive_input("/effort high"));
        assert!(!is_descriptive_input("  /compact"));
        assert!(!is_descriptive_input("!git status"));
        assert!(!is_descriptive_input("! ls -la"));
        assert!(!is_descriptive_input(":wq"));
        assert!(!is_descriptive_input(":help split"));
        assert!(!is_descriptive_input("#remember this pattern"));
    }

    #[test]
    fn descriptive_input_rejects_short_confirmations() {
        assert!(!is_descriptive_input("y"));
        assert!(!is_descriptive_input("no"));
        assert!(!is_descriptive_input("2"));
        assert!(!is_descriptive_input("ok"));
        // Just below the 4-char threshold.
        assert!(!is_descriptive_input("yes"));
        assert!(!is_descriptive_input(""));
        assert!(!is_descriptive_input("   "));
    }

    #[test]
    fn descriptive_input_rejects_letterless_input() {
        assert!(!is_descriptive_input("8080"));
        assert!(!is_descriptive_input("1234"));
        assert!(!is_descriptive_input("....!!"));
    }

    #[test]
    fn strip_ansi_removes_sgr_without_separator() {
        assert_eq!(strip_ansi("\x1b[1;31mhello\x1b[0m world"), "hello world");
    }

    #[test]
    fn strip_ansi_inserts_space_for_cursor_moves() {
        // ESC [ G = cursor horizontal absolute — fragments must not merge
        assert_eq!(strip_ansi("Worked\x1b[Gfor"), "Worked for");
        // ESC [ H = cursor position
        assert_eq!(strip_ansi("left\x1b[1;1Hright"), "left right");
        // ESC [ A/B/C/D = directional moves
        assert_eq!(strip_ansi("a\x1b[Ab"), "a b");
        assert_eq!(strip_ansi("a\x1b[Bb"), "a b");
        assert_eq!(strip_ansi("a\x1b[Cb"), "a b");
        assert_eq!(strip_ansi("a\x1b[Db"), "a b");
    }

    #[test]
    fn strip_ansi_consumes_osc_sequences() {
        // OSC terminated by BEL — must not leak trigger phrases into detection
        assert_eq!(strip_ansi("\x1b]0;resets in 1pm\x07content"), "content");
        // OSC terminated by ST (ESC \)
        assert_eq!(strip_ansi("\x1b]2;title\x1b\\rest"), "rest");
    }

    #[test]
    fn strip_ansi_consumes_single_char_and_ss3_escapes() {
        // ESC 7 = save cursor, ESC 8 = restore cursor
        assert_eq!(strip_ansi("\x1b7text\x1b8"), "text");
        // SS3: ESC O P = F1
        assert_eq!(strip_ansi("\x1bOP"), "");
        // Charset select: ESC ( B
        assert_eq!(strip_ansi("\x1b(Btext"), "text");
    }

    #[test]
    fn strip_ansi_osc_followed_by_csi_does_not_leak_csi_final() {
        // ESC ] title ESC [ H — the ESC [ is a new CSI, not ST; H must not leak
        assert_eq!(strip_ansi("\x1b]0;title\x1b[Hcontent"), " content");
        // Well-formed OSC + ST still works
        assert_eq!(strip_ansi("\x1b]0;title\x1b\\rest"), "rest");
    }

    #[test]
    fn strip_ansi_consumes_dcs_payload() {
        // DCS: ESC P payload ESC \ — payload must not appear in output
        assert_eq!(strip_ansi("\x1bP1$r0m\x1b\\ normal"), " normal");
        // APC (kitty): ESC _ payload ESC \
        assert_eq!(strip_ansi("\x1b_Ga=T;\x1b\\ text"), " text");
    }

    #[test]
    fn strip_ansi_consumes_esc_percent_sequences() {
        // ESC % G = select UTF-8, ESC % @ = select default — both two-char
        assert_eq!(strip_ansi("\x1b%Gtext"), "text");
        assert_eq!(strip_ansi("\x1b%@text"), "text");
    }

    #[test]
    fn password_prompt_detected_despite_cursor_split() {
        // A TUI rendering "passphrase:" with a cursor move inside the word
        let lines = vec!["pass\x1b[Gphrase:".to_string()];
        assert!(looks_like_password_prompt(&lines),
            "cursor-split passphrase must still be detected");
        let lines2 = vec!["pass\x1b[Gword:".to_string()];
        assert!(looks_like_password_prompt(&lines2),
            "cursor-split password must still be detected");
    }

    #[test]
    fn strip_ansi_osc_title_does_not_trigger_usage_limit_hint() {
        // OSC setting a window title containing a hint phrase must be invisible
        // to detect_usage_limit_hint_raw so it doesn't produce spurious hints.
        let raw = "\x1b]0;resets 1pm (Europe/Oslo)\x07\x1b[H\x1b[2J";
        let result = detect_usage_limit_hint_raw(&["resets"], raw);
        assert!(result.is_none(), "OSC title must not produce a usage-limit hint");
    }
}
