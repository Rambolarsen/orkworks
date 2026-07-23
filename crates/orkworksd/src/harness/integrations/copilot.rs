use std::path::Path;

use serde_json::{json, Map, Value};

use super::{render_reporter_command, FragmentState, JsonHookHandler, ToolHookContract};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

const MARKER: &str = "orkworks:harness-integration:v2:copilot";

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "copilot",
        tool_name: "GitHub Copilot CLI",
        relative_path: ".github/copilot/settings.local.json",
        ownership_marker: MARKER,
        coverage: IntegrationCoverage::Limited,
        activation: IntegrationActivation::Active,
    },
    probe,
    merge,
    remove,
);

fn hooks(document: &Map<String, Value>) -> Result<Vec<Value>, IntegrationError> {
    let Some(value) = document.get("hooks") else {
        return Ok(vec![]);
    };
    let hooks = value.as_object().ok_or_else(|| {
        IntegrationError::InvalidConfig("Copilot hooks must be an object.".into())
    })?;
    if hooks
        .get("version")
        .is_some_and(|version| version != &Value::from(1))
    {
        return Err(IntegrationError::InvalidConfig(
            "Copilot hooks require inline version 1.".into(),
        ));
    }
    hooks.get("notification").map_or(Ok(vec![]), |value| {
        value.as_array().cloned().ok_or_else(|| {
            IntegrationError::InvalidConfig("Copilot notification hooks must be an array.".into())
        })
    })
}

fn state(hook: &Value, reporter: Option<&Path>) -> FragmentState {
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
        hook.get("type").and_then(Value::as_str) == Some("command")
            && hook.get("command").and_then(Value::as_str)
                == Some(render_reporter_command(path, MARKER).as_str())
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
    let mut result = FragmentState::Absent;
    for hook in hooks(document)? {
        match state(&hook, Some(reporter)) {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
            FragmentState::Installed if result == FragmentState::Absent => {
                result = FragmentState::Installed
            }
            FragmentState::Installed | FragmentState::Drifted => result = FragmentState::Drifted,
        }
    }
    Ok(result)
}

fn merge(document: &mut Map<String, Value>, reporter: &Path) -> Result<(), IntegrationError> {
    let _ = remove(document)?;
    let hooks = document
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| {
            IntegrationError::InvalidConfig("Copilot hooks must be an object.".into())
        })?;
    match hooks.get("version") {
        None => {
            hooks.insert("version".into(), Value::from(1));
        }
        Some(version) if version == &Value::from(1) => {}
        Some(_) => {
            return Err(IntegrationError::InvalidConfig(
                "Copilot hooks require inline version 1.".into(),
            ))
        }
    }
    let notifications = hooks
        .entry("notification")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| {
            IntegrationError::InvalidConfig("Copilot notification hooks must be an array.".into())
        })?;
    notifications.push(json!({"type":"command","command":render_reporter_command(reporter, MARKER),"env":{"ORKWORKS_INTEGRATION_MARKER":MARKER}}));
    Ok(())
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    let existing = hooks(document)?;
    let mut count = 0;
    for hook in &existing {
        match state(hook, None) {
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
        .get_mut("notification")
        .and_then(Value::as_array_mut)
        .expect("validated notification array");
    notifications.retain(|hook| state(hook, None) == FragmentState::Absent);
    Ok(FragmentState::Drifted)
}
