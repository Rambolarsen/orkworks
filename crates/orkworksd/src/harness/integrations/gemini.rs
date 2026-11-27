use std::path::Path;

use serde_json::{json, Map, Value};

use super::{
    hooks_map, reconcile_current, reporter_invocation, FragmentState, JsonHookHandler,
    ToolHookContract,
};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

const MARKER: &str = "orkworks:harness-integration:v2:gemini";
const TOOL_LABEL: &str = "Gemini";
const EVENTS: [&str; 1] = ["Notification"];

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "gemini",
        additional_reporter_assets: &[],
        tool_name: "Gemini CLI",
        relative_path: ".gemini/settings.json",
        ownership_marker: MARKER,
        coverage: IntegrationCoverage::Limited,
        reports_attention: true,
        activation: IntegrationActivation::Unknown,
        reports_plan_path: false,
    },
    probe,
    merge,
    remove,
    reconcile_current,
);

fn extract_marker(hook: &Value) -> Option<&str> {
    hook.get("name")
        .and_then(Value::as_str)
        .filter(|name| name.starts_with("orkworks:harness-integration:"))
}

fn state(definition: &Value, reporter: Option<&Path>) -> FragmentState {
    hooks_map::classify_hook_group(definition, MARKER, extract_marker, |hook| {
        reporter.is_some_and(|path| {
            let invocation = reporter_invocation(path, MARKER);
            hook.get("type").and_then(Value::as_str) == Some("command")
                && hook.get("command").and_then(Value::as_str)
                    == Some(invocation.shell_command.as_str())
        })
    })
}

fn probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    hooks_map::probe_event_bundle(
        document,
        TOOL_LABEL,
        &EVENTS,
        |event| event,
        |_, definition| state(definition, Some(reporter)),
        &[],
        |_, _| FragmentState::Absent,
    )
}

fn merge(document: &mut Map<String, Value>, reporter: &Path) -> Result<(), IntegrationError> {
    if remove(document)? == FragmentState::Ambiguous {
        return Err(IntegrationError::OwnershipAmbiguous);
    }
    let invocation = reporter_invocation(reporter, MARKER);
    hooks_map::append_event_entries(
        document,
        TOOL_LABEL,
        [(
            "Notification",
            json!({"sequential":true,"hooks":[{"type":"command","name":MARKER,"command":invocation.shell_command}]}),
        )],
    )
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    hooks_map::remove_event_bundle(
        document,
        TOOL_LABEL,
        &EVENTS,
        |event| event,
        |_, definition| state(definition, None),
    )
}
