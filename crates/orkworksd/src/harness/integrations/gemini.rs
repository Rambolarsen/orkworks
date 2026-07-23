use std::path::Path;

use serde_json::{json, Map, Value};

use super::{
    reconcile_current, reporter_invocation, FragmentState, JsonHookHandler, ToolHookContract,
};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

const MARKER: &str = "orkworks:harness-integration:v2:gemini";

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "gemini",
        tool_name: "Gemini CLI",
        relative_path: ".gemini/settings.json",
        ownership_marker: MARKER,
        coverage: IntegrationCoverage::Limited,
        activation: IntegrationActivation::Unknown,
    },
    probe,
    merge,
    remove,
    reconcile_current,
);

fn definitions(document: &Map<String, Value>) -> Result<Vec<Value>, IntegrationError> {
    let Some(hooks) = document.get("hooks") else {
        return Ok(vec![]);
    };
    let hooks = hooks
        .as_object()
        .ok_or_else(|| IntegrationError::InvalidConfig("Gemini hooks must be an object.".into()))?;
    hooks.get("Notification").map_or(Ok(vec![]), |value| {
        value.as_array().cloned().ok_or_else(|| {
            IntegrationError::InvalidConfig("Gemini Notification hooks must be an array.".into())
        })
    })
}

fn state(definition: &Value, reporter: Option<&Path>) -> FragmentState {
    let Some(hooks) = definition.get("hooks").and_then(Value::as_array) else {
        return FragmentState::Absent;
    };
    let mut found = None;
    for hook in hooks {
        let Some(name) = hook.get("name").and_then(Value::as_str) else {
            continue;
        };
        if name.starts_with("orkworks:harness-integration:") {
            if name != MARKER || hooks.len() != 1 {
                return FragmentState::Ambiguous;
            }
            let exact = reporter.is_some_and(|path| {
                let invocation = reporter_invocation(path, MARKER);
                hook.get("type").and_then(Value::as_str) == Some("command")
                    && hook.get("command").and_then(Value::as_str)
                        == Some(invocation.shell_command.as_str())
            });
            if found.is_some() {
                return FragmentState::Drifted;
            }
            found = Some(if exact {
                FragmentState::Installed
            } else {
                FragmentState::Drifted
            });
        }
    }
    found.unwrap_or(FragmentState::Absent)
}

fn probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    let mut result = FragmentState::Absent;
    for definition in definitions(document)? {
        let next = state(&definition, Some(reporter));
        if result != FragmentState::Absent && next != FragmentState::Absent {
            return Ok(FragmentState::Ambiguous);
        }
        match next {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
            FragmentState::Installed => result = FragmentState::Installed,
            FragmentState::Drifted => result = FragmentState::Drifted,
        }
    }
    Ok(result)
}

fn merge(document: &mut Map<String, Value>, reporter: &Path) -> Result<(), IntegrationError> {
    if remove(document)? == FragmentState::Ambiguous {
        return Err(IntegrationError::OwnershipAmbiguous);
    }
    let hooks = document
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| IntegrationError::InvalidConfig("Gemini hooks must be an object.".into()))?;
    let notifications = hooks
        .entry("Notification")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| {
            IntegrationError::InvalidConfig("Gemini Notification hooks must be an array.".into())
        })?;
    let invocation = reporter_invocation(reporter, MARKER);
    notifications.push(json!({"sequential":true,"hooks":[{"type":"command","name":MARKER,"command":invocation.shell_command}]}));
    Ok(())
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    let existing = definitions(document)?;
    let mut count = 0;
    for definition in &existing {
        match state(definition, None) {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
            _ => count += 1,
        }
    }
    if count == 0 {
        return Ok(FragmentState::Absent);
    }
    if count > 1 {
        return Ok(FragmentState::Ambiguous);
    }
    let hooks = document
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .expect("validated hooks object");
    let notifications = hooks
        .get_mut("Notification")
        .and_then(Value::as_array_mut)
        .expect("validated Notification array");
    notifications.retain(|definition| state(definition, None) == FragmentState::Absent);
    Ok(FragmentState::Drifted)
}
