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
                        tracing::warn!(
                            "PEON_FINAL_SCAN_TIMEOUT is not a valid number, using default 2"
                        );
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
    lines: VecDeque<(u64, String)>,
    capacity: usize,
    next_revision: u64,
}

impl RingBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            lines: VecDeque::new(),
            capacity,
            next_revision: 1,
        }
    }

    pub fn push(&mut self, line: String) -> u64 {
        let revision = self.next_revision;
        self.next_revision += 1;
        self.lines.push_back((revision, line));
        while self.lines.len() > self.capacity {
            self.lines.pop_front();
        }
        revision
    }

    pub fn snapshot(&self) -> Vec<String> {
        self.lines.iter().map(|(_, line)| line.clone()).collect()
    }

    pub fn snapshot_after(&self, revision: u64) -> Vec<String> {
        self.lines
            .iter()
            .filter(|(line_revision, _)| *line_revision > revision)
            .map(|(_, line)| line.clone())
            .collect()
    }

    pub fn snapshot_after_with_revisions(&self, revision: u64) -> Option<RingSnapshot> {
        let lines: Vec<(u64, String)> = self
            .lines
            .iter()
            .filter(|(line_revision, _)| *line_revision > revision)
            .cloned()
            .collect();
        let first_revision = lines.first().map(|(revision, _)| *revision)?;
        let last_revision = lines.last().map(|(revision, _)| *revision)?;
        Some(RingSnapshot {
            first_revision,
            last_revision,
            lines: lines.into_iter().map(|(_, line)| line).collect(),
        })
    }

    pub fn has_after(&self, revision: u64) -> bool {
        self.lines
            .back()
            .is_some_and(|(line_revision, _)| *line_revision > revision)
    }

    pub fn last_n(&self, n: usize) -> Vec<String> {
        self.lines
            .iter()
            .rev()
            .take(n)
            .map(|(_, line)| line.clone())
            .collect()
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RingSnapshot {
    pub first_revision: u64,
    pub last_revision: u64,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PeonOutputCapture {
    pub(crate) lines: Vec<String>,
    pub(crate) input_generation: u64,
    pub(crate) min_revision: u64,
    pub(crate) first_revision: u64,
    pub(crate) last_revision: u64,
    pub(crate) runtime_instance_id: String,
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
fn strip_ansi_escape<I: Iterator<Item = char>>(
    chars: &mut std::iter::Peekable<I>,
    out: &mut String,
) {
    match chars.peek().copied() {
        Some('[') => {
            // CSI: ESC [ params final (final = 0x40–0x7E)
            chars.next();
            let mut final_byte = '\0';
            for c2 in chars.by_ref() {
                if ('@'..='~').contains(&c2) {
                    final_byte = c2;
                    break;
                }
            }
            // Cursor-positioning finals: insert a space so adjacent screen
            // regions don't merge into a single token after stripping.
            if matches!(
                final_byte,
                'A' | 'B' | 'C' | 'D' | 'E' | 'F' | 'G' | 'H' | 'd' | 'f' | 's' | 'u'
            ) {
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
            chars.next();
            chars.next();
        }
        Some('(' | ')') => {
            // Charset select: ESC ( x  or  ESC ) x
            chars.next();
            chars.next();
        }
        Some('%') => {
            // ESC % G (select UTF-8) / ESC % @ (select default) — two-char sequences
            chars.next();
            chars.next();
        }
        Some(_) => {
            // Single-char escape: ESC 7/8/M/c/= etc.
            chars.next();
        }
        None => {}
    }
}

pub fn detect_usage_limit<S: AsRef<str>>(patterns: &[S], lines: &[String]) -> bool {
    if patterns.is_empty() {
        return false;
    }
    lines.iter().any(|line| {
        let plain = strip_ansi(line);
        patterns
            .iter()
            .any(|p| bounded_limit_match(&plain, &p.as_ref().to_ascii_lowercase()))
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
        patterns
            .iter()
            .any(|p| lower.contains(p) || compact.contains(p))
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

fn explicit_pr_numbers(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let bytes = lower.as_bytes();
    let mut numbers = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if index > 0 && bytes[index - 1].is_ascii_alphanumeric() {
            index += 1;
            continue;
        }
        let prefix_len = if bytes[index..].starts_with(b"pr #") {
            4
        } else if bytes[index..].starts_with(b"pull request #") {
            14
        } else {
            index += 1;
            continue;
        };

        let digits_start = index + prefix_len;
        let mut digits_end = digits_start;
        while digits_end < bytes.len() && bytes[digits_end].is_ascii_digit() {
            digits_end += 1;
        }
        if digits_end > digits_start {
            numbers.push(lower[digits_start..digits_end].to_string());
        }
        index = digits_end.max(index + prefix_len);
    }

    numbers
}

fn referenced_pr_numbers(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut numbers = Vec::new();
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != b'#' {
            index += 1;
            continue;
        }

        let digits_start = index + 1;
        let mut digits_end = digits_start;
        while digits_end < bytes.len() && bytes[digits_end].is_ascii_digit() {
            digits_end += 1;
        }
        if digits_end > digits_start {
            numbers.push(text[digits_start..digits_end].to_string());
        }
        index = digits_end.max(digits_start);
    }

    numbers
}

fn normalize_generic_instruction(label: &str) -> String {
    label
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase()
}

/// Returns whether an input-triggered label names the task and retains all PR
/// numbers explicitly mentioned in the submitted input.
pub fn is_usable_input_label(label: &str, input_hint: &str) -> bool {
    const GENERIC_PREFIXES: &[&str] = &[
        "instructing system",
        "instructing the system",
        "instructing agent",
        "instructing the agent",
    ];

    let normalized = normalize_generic_instruction(label);
    let candidate_pr_numbers = referenced_pr_numbers(label);
    !normalized.is_empty()
        && !GENERIC_PREFIXES
            .iter()
            .any(|prefix| normalized.starts_with(prefix))
        && !normalized.contains("current task execution")
        && explicit_pr_numbers(input_hint)
            .iter()
            .all(|number| candidate_pr_numbers.contains(number))
}

/// Returns the only Peon summary eligible for the live work headline and
/// durable task history. Terminal state alone is not work evidence.
pub fn work_history_summary(output: &[String], inference_summary: Option<&str>) -> Option<String> {
    if let Some(summary) = output
        .iter()
        .rev()
        .find_map(|line| line.strip_prefix("[User input]:").map(str::trim))
        .filter(|input| is_descriptive_input(input))
        .and_then(|input| inference_summary.filter(|summary| is_usable_input_label(summary, input)))
        .map(normalize_summary)
    {
        return Some(summary);
    }

    command_outcome_summary(output)
}

fn command_outcome_summary(output: &[String]) -> Option<String> {
    let mut command = None;
    let mut outcome = None;
    for line in output {
        let line = strip_ansi(line).to_ascii_lowercase();
        let command_line = line.trim_start_matches(['$', '>', ' ']).trim_start();
        if ["cargo test", "pnpm test", "pnpm run test", "npm test"]
            .iter()
            .any(|command| command_line.starts_with(command))
        {
            command = Some("test");
        } else if ["cargo build", "pnpm build", "npm run build"]
            .iter()
            .any(|command| command_line.starts_with(command))
        {
            command = Some("build");
        }

        outcome = match command {
            Some("test")
                if line.contains("test result: failed")
                    || line.contains("tests failed")
                    || line.contains("could not compile") =>
            {
                command = None;
                Some("Tests failed".into())
            }
            Some("test") if line.contains("test result: ok") || line.contains("tests passed") => {
                command = None;
                Some("Tests passed".into())
            }
            Some("build")
                if line.contains("could not compile") || line.contains("build failed") =>
            {
                command = None;
                Some("Build failed".into())
            }
            Some("build") if line.contains("finished ") || line.contains("build completed") => {
                command = None;
                Some("Build passed".into())
            }
            _ => outcome,
        };
    }
    outcome
}

/// A genuine usage-limit banner is emitted by the harness as its own short
/// status line, with the pattern near the start and at most a reset hint plus
/// TUI status decoration after it. The same pattern text buried inside a long
/// line of displayed content (echoed source code, AI conversation text,
/// scrollback of either) is display, not a cap signal, and must not latch the
/// session capped. Both sides of the match are therefore bounded; the
/// co-located reset-hint fragment does not count toward the suffix budget.
const LIMIT_CONTEXT_MAX_CHARS: usize = 48;
/// A real banner is emitted at the start of a status line: at most a spinner
/// glyph or box-drawing decoration precedes the pattern. Displayed code and
/// prose routinely carry tens of characters of context before a buried
/// pattern, so the prefix budget must stay tight.
const LIMIT_PREFIX_MAX_CHARS: usize = 24;
/// A real banner's reset hint ("resets in 2h") starts within a couple of
/// separator characters of the pattern; a later prose mention gets no relief.
const LIMIT_HINT_ANCHOR_MAX_PREFIX: usize = 4;

fn bounded_limit_match(plain: &str, pattern_lower: &str) -> bool {
    let lower = plain.to_ascii_lowercase();
    for (start, matched) in lower.match_indices(pattern_lower) {
        let end = start + matched.len();
        let prefix_chars = lower[..start].chars().count();
        let mut suffix_chars = lower[end..].chars().count();
        let suffix_plain = &plain[end..];
        let anchor = suffix_lower_anchor(&lower[end..]);
        if let Some(anchor_rel) = anchor {
            if anchor_rel <= LIMIT_HINT_ANCHOR_MAX_PREFIX {
                if let Some(hint) = extract_reset_hint(suffix_plain, &lower[end..]) {
                    suffix_chars -= hint.chars().count().min(suffix_chars);
                }
            }
        }
        if prefix_chars <= LIMIT_PREFIX_MAX_CHARS && suffix_chars <= LIMIT_CONTEXT_MAX_CHARS {
            return true;
        }
    }
    false
}

fn suffix_lower_anchor(suffix_lower: &str) -> Option<usize> {
    suffix_lower
        .find("resets in")
        .or_else(|| suffix_lower.find("reset in"))
        .or_else(|| suffix_lower.find("resets "))
        .or_else(|| suffix_lower.find("try again at"))
}

/// Detects usage limit in a raw text blob (for TUI apps that use cursor positioning, not newlines).
pub fn detect_usage_limit_raw<S: AsRef<str>>(patterns: &[S], text: &str) -> bool {
    if patterns.is_empty() {
        return false;
    }
    let plain = strip_ansi(text);
    plain.split(['\r', '\n']).any(|segment| {
        patterns
            .iter()
            .any(|p| bounded_limit_match(segment, &p.as_ref().to_ascii_lowercase()))
    })
}

/// TUI status glyphs (spinners, separators, box drawing) that mark the end of
/// a reset hint when screen content follows it without a newline.
const HINT_STOP_GLYPHS: &[char] = &[
    '✳', '✻', '✽', '✶', '●', '○', '◐', '·', '│', '╭', '╰', '─', '—',
];
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
    let mut hint: String = fragment[..end]
        .trim()
        .chars()
        .take(HINT_MAX_CHARS)
        .collect();
    hint.truncate(hint.trim_end().len());
    Some(hint)
}

/// Extracts reset hint from a raw text blob (for TUI apps that use cursor positioning, not newlines).
pub fn detect_usage_limit_hint_raw<S: AsRef<str>>(patterns: &[S], text: &str) -> Option<String> {
    if patterns.is_empty() {
        return None;
    }
    let plain = strip_ansi(text);
    let lower = plain.to_ascii_lowercase();
    if !patterns
        .iter()
        .any(|p| lower.contains(p.as_ref().to_lowercase().as_str()))
    {
        return None;
    }
    extract_reset_hint(&plain, &lower)
}

/// Returns the "reset in X" fragment from the usage-limit line, if present.
pub fn detect_usage_limit_hint<S: AsRef<str>>(patterns: &[S], lines: &[String]) -> Option<String> {
    if patterns.is_empty() {
        return None;
    }
    lines.iter().rev().find_map(|line| {
        let plain = strip_ansi(line);
        let lower = plain.to_ascii_lowercase();
        if !patterns
            .iter()
            .any(|p| lower.contains(p.as_ref().to_lowercase().as_str()))
        {
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
- summary: one-line summary of concrete work only. Omit it unless the output contains a descriptive '[User input]:' instruction or a matching test/build command and result. Never summarize terminal redraws, ANSI escape codes, spinners, loading, or connection state.
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
- workflowObservations: array of at most five concrete workflow-friction candidates. Each candidate must have kind (one of repetition, obstacle, missing_context, assumption, correction, workaround, verification_gap), description, evidence, reportedImpact (low, medium, or high), and confidence from 0.0 to 1.0. Only report friction that made the work harder than necessary; never report ordinary progress, terminal redraws, or speculative advice.

If a line starting with '[User input]:' is present, it is what the user just typed to the AI coding tool. Use it to derive a short, direct, present-tense summary of what the user is doing — like a commit-message subject line. NEVER start the summary with \"User\", \"User is\", \"User wants\", \"User asked\", \"User requested\", or \"User typed\". Examples: \"Fixing peon model detection\" not \"User is fixing peon model detection\". \"Reviewing PR feedback\" not \"User wants to review PR feedback\". Keep it under 8 words. The summary must name the concrete task topic, never a generic instruction or control narration such as \"instructing the agent\" or \"continuing current task execution\". Preserve every explicit PR number from the user input (for example, \"PR #249\" or \"pull request #249\").";

const MAX_WORKFLOW_CANDIDATES: usize = 5;

const VALID_STATUSES: &[&str] = &[
    "waiting_for_input",
    "blocked",
    "failed",
    "done",
    "stale",
    "working",
    "idle",
];

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PeonWorkflowObservation {
    pub kind: crate::workflow_observations::ObservationKind,
    pub description: String,
    pub evidence: String,
    #[serde(rename = "reportedImpact")]
    pub reported_impact: crate::workflow_observations::Impact,
    pub confidence: f64,
}

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
    #[serde(
        rename = "workflowObservations",
        default,
        deserialize_with = "deserialize_workflow_candidates"
    )]
    pub workflow_observations: Vec<PeonWorkflowObservation>,
}

/// Best-effort per-candidate extraction: malformed candidates are dropped and
/// the list is capped, so a quirk in the optional workflow-observation output
/// never discards the core session-situation inference (issue #342).
fn deserialize_workflow_candidates<'de, D>(
    deserializer: D,
) -> Result<Vec<PeonWorkflowObservation>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let raw = serde_json::Value::deserialize(deserializer)?;
    let mut candidates = Vec::new();
    if let serde_json::Value::Array(items) = raw {
        for item in items {
            if candidates.len() == MAX_WORKFLOW_CANDIDATES {
                break;
            }
            if let Ok(candidate) = serde_json::from_value::<PeonWorkflowObservation>(item) {
                if (0.0..=1.0).contains(&candidate.confidence) {
                    candidates.push(candidate);
                }
            }
        }
    }
    Ok(candidates)
}

pub fn extract_json(raw: &str) -> Option<String> {
    // Models frequently wrap the JSON object in code fences, leading prose,
    // or trailing garbage (llama3.2 has been observed emitting a stray
    // trailing brace). Extract the first balanced `{...}` object instead of
    // requiring the whole output to be the JSON payload.
    let start = raw.find('{')?;
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;
    for (offset, byte) in raw[start..].bytes().enumerate() {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }
        match byte {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth = depth.checked_sub(1)?;
                if depth == 0 {
                    return Some(raw[start..=start + offset].to_string());
                }
            }
            _ => {}
        }
    }
    None
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

    if inf.workflow_observations.len() > MAX_WORKFLOW_CANDIDATES {
        return Err(format!(
            "workflow observation candidates exceed the maximum of {MAX_WORKFLOW_CANDIDATES}"
        ));
    }
    for candidate in &inf.workflow_observations {
        if !(0.0..=1.0).contains(&candidate.confidence) {
            return Err(format!(
                "workflow observation confidence {} out of range [0.0, 1.0]",
                candidate.confidence
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

pub fn build_prompt(output: &[String]) -> String {
    let output_text: String = output
        .iter()
        .map(|l| l.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let required_pr_references = explicit_pr_numbers(&output_text)
        .into_iter()
        .map(|number| format!("#{number}"))
        .collect::<Vec<_>>();

    let truncated: String = if output_text.len() > 4096 {
        output_text.chars().take(4096).collect()
    } else {
        output_text
    };

    let required_pr_references = (!required_pr_references.is_empty())
        .then(|| {
            format!(
                "\nRequired PR references: {}",
                required_pr_references.join(", ")
            )
        })
        .unwrap_or_default();

    format!("{SYSTEM_PROMPT}\n\nTerminal output:\n```\n{truncated}\n```{required_pr_references}")
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
    fn test_ring_buffer_snapshot_after_excludes_prior_lines() {
        let mut buf = RingBuffer::new(5);
        let first = buf.push("old prompt".into());
        let boundary = buf.push("old question".into());
        buf.push("new output".into());

        assert!(boundary > first);
        assert_eq!(buf.snapshot_after(boundary), vec!["new output"]);
        assert!(buf.has_after(boundary));
        assert!(!buf.has_after(boundary + 1));
    }

    #[test]
    fn ring_buffer_snapshot_after_reports_the_captured_revision_range() {
        let mut buf = RingBuffer::new(5);
        let boundary = buf.push("old".into());
        let first = buf.push("new one".into());
        let last = buf.push("new two".into());

        let snapshot = buf.snapshot_after_with_revisions(boundary).unwrap();
        assert_eq!(snapshot.first_revision, first);
        assert_eq!(snapshot.last_revision, last);
        assert_eq!(snapshot.lines, vec!["new one", "new two"]);
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
        let parsed: PeonInference = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed.observed_status, Some("working".into()));
        assert!((parsed.confidence - 0.9).abs() < 0.001);
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
    fn test_extract_json_tolerates_trailing_garbage() {
        // llama3.2 has been observed emitting a stray trailing brace after an
        // otherwise valid JSON object.
        let raw = r#"{"observedStatus": "working", "confidence": 0.9}]}"#;
        let result = extract_json(raw);
        let parsed: PeonInference = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed.observed_status, Some("working".into()));
    }

    #[test]
    fn test_extract_json_tolerates_leading_prose() {
        let raw = "Here is the analysis:\n{\"observedStatus\": \"idle\", \"confidence\": 0.5}";
        let result = extract_json(raw);
        let parsed: PeonInference = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed.observed_status, Some("idle".into()));
    }

    #[test]
    fn test_extract_json_ignores_braces_inside_strings() {
        let raw = r#"{"summary": "wrote {a, b} handling", "confidence": 0.7} trailing text"#;
        let result = extract_json(raw);
        let parsed: PeonInference = serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed.summary.as_deref(), Some("wrote {a, b} handling"));
    }

    #[test]
    fn test_extract_json_returns_none_for_unbalanced_json() {
        assert!(extract_json(r#"{"confidence": 0.9"#).is_none());
        assert!(extract_json("} } } no object here").is_none());
    }

    #[test]
    fn test_extract_json_skips_garbage_before_the_object() {
        let result = extract_json(r#"}{"confidence": 0.9}"#);
        let parsed: PeonInference = serde_json::from_str(&result.unwrap()).unwrap();
        assert!((parsed.confidence - 0.9).abs() < 0.001);
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
            workflow_observations: Vec::new(),
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
            workflow_observations: Vec::new(),
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
            workflow_observations: Vec::new(),
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
            workflow_observations: Vec::new(),
        };
        assert!(validate_inference(&inf2).is_err());
    }

    #[test]
    fn peon_parses_workflow_observation_candidates_with_camel_case_impact() {
        let raw = r#"{
            "observedStatus": "working",
            "confidence": 0.8,
            "workflowObservations": [{
                "kind": "verification_gap",
                "description": "The same integration test had to be rerun manually",
                "evidence": "cargo test failed once because the fixture was stale",
                "reportedImpact": "medium",
                "confidence": 0.75
            }]
        }"#;

        let inference = parse_inference(raw).expect("valid Peon response");
        assert_eq!(inference.workflow_observations.len(), 1);
        assert_eq!(
            inference.workflow_observations[0].kind,
            crate::workflow_observations::ObservationKind::VerificationGap
        );
        assert_eq!(
            inference.workflow_observations[0].reported_impact,
            crate::workflow_observations::Impact::Medium
        );
    }

    #[test]
    fn peon_drops_out_of_range_observation_candidates_but_keeps_inference() {
        let raw = r#"{
            "observedStatus": "blocked",
            "summary": "Fixing the failing test",
            "confidence": 0.8,
            "workflowObservations": [{
                "kind": "obstacle",
                "description": "A command was blocked",
                "evidence": "The command returned permission denied",
                "reportedImpact": "high",
                "confidence": 1.1
            }, {
                "kind": "repetition",
                "description": "The same fixture had to be rebuilt manually",
                "evidence": "The rebuild step ran three times",
                "reportedImpact": "low",
                "confidence": 0.6
            }]
        }"#;

        let inference =
            parse_inference(raw).expect("core inference must survive an invalid candidate");
        assert_eq!(inference.observed_status, Some("blocked".into()));
        assert_eq!(inference.summary, Some("Fixing the failing test".into()));
        assert_eq!(inference.workflow_observations.len(), 1);
        assert_eq!(
            inference.workflow_observations[0].kind,
            crate::workflow_observations::ObservationKind::Repetition
        );
    }

    #[test]
    fn peon_drops_malformed_observation_candidates_and_keeps_inference() {
        let raw = r#"{
            "observedStatus": "working",
            "summary": "Refactoring the parser",
            "confidence": 0.8,
            "workflowObservations": [
                {"kind": "not_a_real_kind", "description": "x", "evidence": "e", "reportedImpact": "low", "confidence": 0.5},
                "just a string, not a candidate",
                {"kind": "workaround", "description": "A manual step stood in for automation", "evidence": "e", "reportedImpact": "medium", "confidence": 0.7}
            ]
        }"#;

        let inference =
            parse_inference(raw).expect("core inference must survive malformed candidates");
        assert_eq!(inference.observed_status, Some("working".into()));
        assert_eq!(inference.summary, Some("Refactoring the parser".into()));
        assert_eq!(inference.workflow_observations.len(), 1);
        assert_eq!(
            inference.workflow_observations[0].kind,
            crate::workflow_observations::ObservationKind::Workaround
        );
    }

    #[test]
    fn peon_drops_non_array_workflow_observations_but_keeps_inference() {
        let raw = r#"{
            "observedStatus": "working",
            "summary": "Refactoring the parser",
            "confidence": 0.8,
            "workflowObservations": "oops"
        }"#;

        let inference = parse_inference(raw)
            .expect("core inference must survive an unparseable candidate list");
        assert_eq!(inference.observed_status, Some("working".into()));
        assert_eq!(inference.summary, Some("Refactoring the parser".into()));
        assert!(inference.workflow_observations.is_empty());
    }

    #[test]
    fn peon_limits_workflow_observation_candidates_to_five() {
        let raw = r#"{
            "confidence": 0.8,
            "workflowObservations": [
                {"kind":"obstacle","description":"one","evidence":"e","reportedImpact":"low","confidence":0.7},
                {"kind":"obstacle","description":"two","evidence":"e","reportedImpact":"low","confidence":0.7},
                {"kind":"obstacle","description":"three","evidence":"e","reportedImpact":"low","confidence":0.7},
                {"kind":"obstacle","description":"four","evidence":"e","reportedImpact":"low","confidence":0.7},
                {"kind":"obstacle","description":"five","evidence":"e","reportedImpact":"low","confidence":0.7},
                {"kind":"obstacle","description":"six","evidence":"e","reportedImpact":"low","confidence":0.7}
            ]
        }"#;

        let inference =
            parse_inference(raw).expect("core inference must survive candidate overflow");
        assert_eq!(inference.workflow_observations.len(), 5);
        assert_eq!(inference.workflow_observations[0].description, "one");
    }

    #[test]
    fn parse_inference_returns_none_for_malformed_top_level_json() {
        assert!(parse_inference("{not json").is_none());
        assert!(parse_inference(r#"{"confidence": 0.5, "observedStatus": 42}"#).is_none());
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
    fn work_history_summary_accepts_a_descriptive_user_task() {
        let output = vec!["[User input]: fix task history noise".into()];
        assert_eq!(
            work_history_summary(&output, Some("Fixing task history noise")),
            Some("Fixing task history noise".into())
        );
    }

    #[test]
    fn work_history_summary_uses_canonical_test_outcomes() {
        let output = vec!["$ cargo test".into(), "test result: ok. 42 passed".into()];
        assert_eq!(
            work_history_summary(&output, Some("Terminal is healthy")),
            Some("Tests passed".into())
        );
    }

    #[test]
    fn work_history_summary_prefers_the_latest_user_task() {
        let output = vec![
            "$ cargo test".into(),
            "test result: ok. 42 passed".into(),
            "[User input]: fix task history noise".into(),
        ];
        assert_eq!(
            work_history_summary(&output, Some("Fixing task history noise")),
            Some("Fixing task history noise".into())
        );
    }

    #[test]
    fn work_history_summary_uses_the_latest_command_outcome() {
        let output = vec![
            "$ cargo test".into(),
            "test result: FAILED. 1 failed".into(),
            "$ cargo test".into(),
            "test result: ok. 42 passed".into(),
        ];
        assert_eq!(
            work_history_summary(&output, None),
            Some("Tests passed".into())
        );
    }

    #[test]
    fn work_history_summary_rejects_terminal_state_guesses() {
        let output = vec!["\u{1b}[2K⠋ loading".into()];
        assert_eq!(
            work_history_summary(&output, Some("Session is loading")),
            None
        );
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
        let lines = vec![
            "some output".into(),
            "usage limit reached, resets in 2h".into(),
        ];
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
    fn detect_usage_limit_ignores_pattern_buried_in_a_long_line() {
        // Echoed source code / AI conversation text can contain the harness's
        // limit pattern mid-line. A real limit banner is short with the
        // pattern near the start of the line; a buried match is display
        // content, not a cap signal.
        let lines = vec![
            "                ┃        if re.search(r'usage limit reached', line, re.I):".into(),
        ];
        assert!(!detect_usage_limit(&["usage limit reached"], &lines));
    }

    #[test]
    fn detect_usage_limit_ignores_pattern_buried_in_prose_line() {
        let lines = vec![
            "the way to check is events/<session-id>.terminal for an actual usage limit reached \
             line from OpenCode itself (I checked your current sessions and found none)"
                .into(),
        ];
        assert!(!detect_usage_limit(&["usage limit reached"], &lines));
    }

    #[test]
    fn detect_usage_limit_ignores_pattern_at_prose_wrap_boundary() {
        // Terminal wrapping can start a display row with the pattern; the
        // prose continuing after it is what betrays the false positive.
        let lines = vec![
            "usage limit reached line from OpenCode itself (I checked your current sessions \
             and found none) plus trailing prose to push the row well past the suffix budget"
                .into(),
        ];
        assert!(!detect_usage_limit(&["usage limit reached"], &lines));
    }

    #[test]
    fn detect_usage_limit_raw_ignores_pattern_buried_in_long_segment() {
        let text = format!(
            "{} usage limit reached {}",
            "the way to check is events/<session-id>.terminal for an actual ".repeat(2),
            " line from OpenCode itself (I checked your current sessions and found none)".repeat(2)
        );
        assert!(!detect_usage_limit_raw(&["usage limit reached"], &text));
    }

    #[test]
    fn detect_usage_limit_raw_matches_short_banner_with_reset_hint() {
        assert!(detect_usage_limit_raw(
            &["usage limit reached"],
            "usage limit reached, resets in 2h"
        ));
    }

    #[test]
    fn detect_usage_limit_raw_matches_tui_banner_with_trailing_status_decoration() {
        // TUI redraws place the banner on a shared segment with spinner and
        // status-bar content after the reset hint — still a real cap.
        let text = "✻ You've hit your session limit · resets 1pm (Europe/Oslo) \
                    ✳Worked for 1s ● high · /effort ────";
        assert!(detect_usage_limit_raw(
            &["you've hit your session limit"],
            text
        ));
    }

    #[test]
    fn detect_usage_limit_raw_matches_pattern_on_short_segment_mid_buffer() {
        let text = format!(
            "{}\r\nusage limit reached\r\n{}",
            "cursor-addressed screen content ".repeat(40),
            "more screen content ".repeat(40)
        );
        assert!(detect_usage_limit_raw(&["usage limit reached"], &text));
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
        assert!(
            (70..=80).contains(&len),
            "cap not applied near 80: {len} ({hint})"
        );
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
    fn input_label_validator_rejects_generic_instruction_forms() {
        let cases = [
            ("Monitoring PR #249", "keep watching PR #249", true),
            ("Monitoring #249", "keep watching PR #249", true),
            ("Monitoring pull request", "keep watching PR #249", false),
            (
                "Instructing system to review PR #249",
                "review PR #249",
                false,
            ),
            (
                "Instructing the system to review PR #249",
                "review PR #249",
                false,
            ),
            (
                "Instructing agent to review PR #249",
                "review PR #249",
                false,
            ),
            (
                "Instructing the agent to review PR #249",
                "review PR #249",
                false,
            ),
            (
                "Reviewing PR #249 during current task execution",
                "review PR #249",
                false,
            ),
        ];

        for (label, input_hint, expected) in cases {
            assert_eq!(
                is_usable_input_label(label, input_hint),
                expected,
                "{label}"
            );
        }
    }

    #[test]
    fn input_label_contract_preserves_explicit_prs_and_rejects_generic_controls() {
        // The prompt instructs the provider to retain every explicit PR
        // reference and avoid generic control-language labels. Validation
        // enforces the same contract even across punctuation and casing.
        assert!(SYSTEM_PROMPT.contains("Preserve every explicit PR number"));
        assert!(SYSTEM_PROMPT.contains("generic instruction or control narration"));

        assert!(!is_usable_input_label(
            "INSTRUCTING—the AGENT: review PR #249",
            "review PR #249",
        ));
        assert_eq!(
            explicit_pr_numbers("APR #249: update docs"),
            Vec::<String>::new()
        );
        assert!(is_usable_input_label(
            "Reviewing PR #249",
            "APR #249: update docs",
        ));
        assert!(!is_usable_input_label(
            "Reviewing PR #249",
            "review PR #249 and pull request #250",
        ));
        assert!(is_usable_input_label(
            "Reviewing PR #249 and PR #250",
            "review PR #249 and pull request #250",
        ));
        assert!(is_usable_input_label(
            "修复登录重定向",
            "fix the login redirect"
        ));
    }

    #[test]
    fn build_prompt_retains_pr_references_after_terminal_output_truncation() {
        let long_input = format!("[User input]: {} PR #249", "x".repeat(4097));

        let prompt = build_prompt(&[long_input]);

        assert!(prompt.contains("Required PR references: #249"));
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
        assert!(
            looks_like_password_prompt(&lines),
            "cursor-split passphrase must still be detected"
        );
        let lines2 = vec!["pass\x1b[Gword:".to_string()];
        assert!(
            looks_like_password_prompt(&lines2),
            "cursor-split password must still be detected"
        );
    }

    #[test]
    fn strip_ansi_osc_title_does_not_trigger_usage_limit_hint() {
        // OSC setting a window title containing a hint phrase must be invisible
        // to detect_usage_limit_hint_raw so it doesn't produce spurious hints.
        let raw = "\x1b]0;resets 1pm (Europe/Oslo)\x07\x1b[H\x1b[2J";
        let result = detect_usage_limit_hint_raw(&["resets"], raw);
        assert!(
            result.is_none(),
            "OSC title must not produce a usage-limit hint"
        );
    }
}
