//! Safe startup diagnostics. Internal error context is deliberately not serializable.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StartupFailureCode {
    CodexHelpFailed,
    CodexHelpTimeout,
    CodexHelpOversized,
    PtyOpenFailed,
    ProcessSpawnFailed,
    TerminalIoFailed,
    InstructionSetupFailed,
    NativeStartupFailed,
    StartupInternalFailed,
}
impl StartupFailureCode {
    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::CodexHelpFailed => "Could not check Codex session isolation support. Check the configured executable and retry.",
            Self::CodexHelpTimeout => "The Codex session isolation check timed out. Check that the configured executable responds to --help, then retry.",
            Self::CodexHelpOversized => "The Codex session isolation check returned too much output. Check the configured executable and retry.",
            Self::PtyOpenFailed => "Could not open a terminal for this session. Check available system resources and retry.",
            Self::ProcessSpawnFailed => "Could not start the coding tool. Check its configured executable and working directory, then retry.",
            Self::TerminalIoFailed => "Could not connect the session terminal. Check available system resources and retry.",
            Self::InstructionSetupFailed => "Could not prepare session instructions. Check workspace access and retry.",
            Self::NativeStartupFailed => "Could not prepare the coding-tool integration. Check its integration status in Settings and retry.",
            Self::StartupInternalFailed => "Session startup failed. Check sidecar diagnostics before retrying.",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StartupOperation {
    Launch,
    Resume,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StartupFailure {
    pub(crate) code: StartupFailureCode,
    pub(crate) operation: StartupOperation,
    pub(crate) occurred_at: String,
}
impl StartupFailure {
    pub(crate) fn new(code: StartupFailureCode, operation: StartupOperation) -> Self {
        Self {
            code,
            operation,
            occurred_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
        }
    }
}
/// A malformed diagnostic must not make an otherwise readable session disappear.
pub(crate) fn deserialize_optional<'de, D: serde::Deserializer<'de>>(
    de: D,
) -> Result<Option<StartupFailure>, D::Error> {
    let raw = serde_json::Value::deserialize(de)?;
    if serde_json::to_vec(&raw).map_or(true, |bytes| bytes.len() > 256) {
        return Ok(None);
    }
    let Ok(value) = serde_json::from_value::<StartupFailure>(raw) else {
        return Ok(None);
    };
    let valid = chrono::DateTime::parse_from_rfc3339(&value.occurred_at).is_ok_and(|timestamp| {
        timestamp.to_rfc3339_opts(chrono::SecondsFormat::Millis, true) == value.occurred_at
    });
    Ok(valid.then_some(value))
}

#[derive(Debug)]
pub(crate) struct StartupError {
    pub(crate) code: Option<StartupFailureCode>,
    context: String,
}
impl StartupError {
    pub(crate) fn classified(code: StartupFailureCode, context: impl ToString) -> Self {
        Self {
            code: Some(code),
            context: context.to_string(),
        }
    }
    pub(crate) fn cancelled(context: impl ToString) -> Self {
        Self {
            code: None,
            context: context.to_string(),
        }
    }
}
impl std::fmt::Display for StartupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.context.fmt(formatter)
    }
}
impl From<String> for StartupError {
    fn from(context: String) -> Self {
        Self::classified(StartupFailureCode::StartupInternalFailed, context)
    }
}
impl From<&str> for StartupError {
    fn from(context: &str) -> Self {
        Self::from(context.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_reads_keep_valid_diagnostics_and_ignore_untrusted_shapes() {
        let meta = crate::test_support::test_session_metadata(
            "safe", "Safe", "/tmp", "error", "before", "before",
        );
        let mut raw = serde_json::to_value(meta).unwrap();
        let valid = serde_json::json!({"code":"process_spawn_failed", "operation":"launch", "occurredAt":"2026-11-07T12:00:00.000Z"});
        raw["startupFailure"] = valid.clone();
        let decoded: crate::metadata::SessionMetadata =
            serde_json::from_value(raw.clone()).unwrap();
        assert_eq!(
            serde_json::to_value(decoded).unwrap()["startupFailure"],
            valid
        );
        for invalid in [
            serde_json::json!(null),
            serde_json::json!("secret-token"),
            serde_json::json!({"code":"future", "operation":"launch", "occurredAt":"2026-11-07T12:00:00.000Z"}),
            serde_json::json!({"code":"process_spawn_failed", "operation":"execute", "occurredAt":"2026-11-07T12:00:00.000Z"}),
            serde_json::json!({"code":"process_spawn_failed", "operation":"launch", "occurredAt":"bad"}),
            serde_json::json!({"code":"process_spawn_failed", "operation":"launch", "occurredAt":"2026-11-07T12:00:00.000Z", "message":"secret-token"}),
            serde_json::json!({"code":"process_spawn_failed", "operation":"launch", "occurredAt":"x".repeat(257)}),
        ] {
            raw["startupFailure"] = invalid;
            let decoded: crate::metadata::SessionMetadata =
                serde_json::from_value(raw.clone()).unwrap();
            assert!(decoded.startup_failure.is_none());
            assert_eq!(decoded.id, "safe");
        }
    }
    #[test]
    fn diagnostic_catalog_is_bounded_and_shared_with_desktop() {
        let desktop = include_str!("../../../apps/desktop/src/startupDiagnostics.ts");
        for code in [
            StartupFailureCode::CodexHelpFailed,
            StartupFailureCode::CodexHelpTimeout,
            StartupFailureCode::CodexHelpOversized,
            StartupFailureCode::PtyOpenFailed,
            StartupFailureCode::ProcessSpawnFailed,
            StartupFailureCode::TerminalIoFailed,
            StartupFailureCode::InstructionSetupFailed,
            StartupFailureCode::NativeStartupFailed,
            StartupFailureCode::StartupInternalFailed,
        ] {
            assert!(
                desktop.contains(code.message()),
                "desktop catalog missing {code:?}"
            );
            let failure = StartupFailure::new(code, StartupOperation::Resume);
            assert!(serde_json::to_vec(&failure).unwrap().len() <= 256);
            let body = serde_json::json!({"error":"session_startup_failed", "message":code.message(), "startupFailure":failure});
            assert!(serde_json::to_vec(&body).unwrap().len() <= 1024);
        }
    }
}
