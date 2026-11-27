use std::path::Path;

use serde_json::{json, Map, Value};

use super::{
    event_reporter_invocation_for_platform, hooks_map, reconcile_all_platforms, FragmentState,
    JsonHookHandler, ReporterPlatform, ToolHookContract,
};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

const MARKER: &str = "orkworks:harness-integration:v2:copilot";
const TOOL_LABEL: &str = "Copilot";
const EVENTS: [(&str, &str); 3] = [
    ("notification", "notification"),
    ("sessionStart", "sessionStart"),
    ("userPromptSubmitted", "userPromptSubmitted"),
];

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "copilot",
        additional_reporter_assets: &[],
        tool_name: "GitHub Copilot CLI",
        relative_path: ".github/copilot/settings.local.json",
        ownership_marker: MARKER,
        coverage: IntegrationCoverage::Limited,
        reports_attention: true,
        activation: IntegrationActivation::Active,
        reports_plan_path: false,
    },
    probe,
    merge,
    remove,
    reconcile_all_platforms,
);

fn reporter_paths(reporter: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let parent = reporter.parent().unwrap_or_else(|| Path::new("."));
    (
        parent.join(ReporterPlatform::Posix.asset_name()),
        parent.join(ReporterPlatform::WindowsPowerShell.asset_name()),
    )
}

fn validate_version(document: &Map<String, Value>) -> Result<(), IntegrationError> {
    if document
        .get("version")
        .is_some_and(|version| version != &Value::from(1))
    {
        return Err(IntegrationError::InvalidConfig(
            "Copilot inline hooks require top-level version 1.".into(),
        ));
    }
    Ok(())
}

fn state(hook: &Value, reporter: Option<&Path>, event: &str) -> FragmentState {
    let Some(marker) = hook
        .get("env")
        .and_then(Value::as_object)
        .and_then(|env| env.get("ORKWORKS_INTEGRATION_MARKER"))
        .and_then(Value::as_str)
    else {
        return FragmentState::Absent;
    };
    if !marker.starts_with("orkworks:harness-integration:") {
        return FragmentState::Absent;
    }
    if marker != MARKER {
        return FragmentState::Ambiguous;
    }
    let exact = reporter.is_some_and(|path| {
        let (posix_path, powershell_path) = reporter_paths(path);
        let posix = event_reporter_invocation_for_platform(
            ReporterPlatform::Posix,
            &posix_path,
            MARKER,
            event,
        );
        let powershell = event_reporter_invocation_for_platform(
            ReporterPlatform::WindowsPowerShell,
            &powershell_path,
            MARKER,
            event,
        );
        hook.get("type").and_then(Value::as_str) == Some("command")
            && hook.get("bash").and_then(Value::as_str) == Some(posix.shell_command.as_str())
            && hook.get("powershell").and_then(Value::as_str)
                == Some(powershell.shell_command.as_str())
    });
    if exact {
        FragmentState::Installed
    } else {
        FragmentState::Drifted
    }
}

fn probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    validate_version(document)?;
    hooks_map::probe_event_bundle(
        document,
        TOOL_LABEL,
        &EVENTS,
        |(key, _)| key,
        |(_, event), hook| state(hook, Some(reporter), event),
        &[],
        |_, _| FragmentState::Absent,
    )
}

pub(super) fn prompt_attention_probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    validate_version(document)?;
    let mut result = FragmentState::Absent;
    for hook in hooks_map::event_entries(document, "notification", TOOL_LABEL)? {
        match state(hook, Some(reporter), "notification") {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
            FragmentState::Drifted => result = FragmentState::Drifted,
            FragmentState::Installed if result == FragmentState::Absent => {
                result = FragmentState::Installed
            }
            FragmentState::Installed => return Ok(FragmentState::Ambiguous),
        }
    }
    Ok(result)
}

fn merge(document: &mut Map<String, Value>, reporter: &Path) -> Result<(), IntegrationError> {
    if remove(document)? == FragmentState::Ambiguous {
        return Err(IntegrationError::OwnershipAmbiguous);
    }
    validate_version(document)?;
    if !document.contains_key("version") {
        document.insert("version".into(), Value::from(1));
    }
    let (posix_path, powershell_path) = reporter_paths(reporter);
    hooks_map::append_event_entries(
        document,
        TOOL_LABEL,
        EVENTS.into_iter().map(|(key, event)| {
            let posix = event_reporter_invocation_for_platform(
                ReporterPlatform::Posix,
                &posix_path,
                MARKER,
                event,
            );
            let powershell = event_reporter_invocation_for_platform(
                ReporterPlatform::WindowsPowerShell,
                &powershell_path,
                MARKER,
                event,
            );
            (
                key,
                json!({"type":"command","bash":posix.shell_command,"powershell":powershell.shell_command,"env":{"ORKWORKS_INTEGRATION_MARKER":MARKER}}),
            )
        }),
    )
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    validate_version(document)?;
    hooks_map::remove_event_bundle(
        document,
        TOOL_LABEL,
        &EVENTS,
        |(key, _)| key,
        |(_, event), hook| state(hook, None, event),
    )
}
