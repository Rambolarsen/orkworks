use std::path::Path;

use serde_json::{json, Map, Value};

use super::{FragmentState, JsonHookHandler, ToolHookContract};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

const MARKER: &str = "orkworks:harness-integration:v2:claude-code";

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "claude-code",
        tool_name: "Claude Code",
        relative_path: ".claude/settings.local.json",
        ownership_marker: MARKER,
        coverage: IntegrationCoverage::Limited,
        activation: IntegrationActivation::Active,
    },
    probe,
    merge,
    remove,
);

fn groups(document: &Map<String, Value>) -> Result<Vec<Value>, IntegrationError> {
    let Some(hooks) = document.get("hooks") else {
        return Ok(vec![]);
    };
    hooks
        .as_object()
        .and_then(|hooks| hooks.get("Notification"))
        .map_or(Ok(vec![]), |value| {
            value.as_array().cloned().ok_or_else(|| {
                IntegrationError::InvalidConfig(
                    "Claude Notification hooks must be an array.".into(),
                )
            })
        })
}

fn marker_state(group: &Value, reporter: Option<&Path>) -> FragmentState {
    let Some(hooks) = group.get("hooks").and_then(Value::as_array) else {
        return FragmentState::Absent;
    };
    let mut found = None;
    for hook in hooks {
        let marker = hook.get("args").and_then(Value::as_array).and_then(|args| {
            args.iter()
                .filter_map(Value::as_str)
                .find(|value| value.starts_with("orkworks:harness-integration:"))
        });
        let Some(marker) = marker else {
            continue;
        };
        if marker.starts_with("orkworks:harness-integration:") {
            if marker != MARKER || hooks.len() != 1 {
                return FragmentState::Ambiguous;
            }
            let exact = reporter.is_some_and(|path| {
                hook.get("type").and_then(Value::as_str) == Some("command")
                    && hook.get("command").and_then(Value::as_str)
                        == Some(path.to_string_lossy().as_ref())
                    && hook
                        .get("args")
                        .and_then(Value::as_array)
                        .is_some_and(|args| {
                            args == &vec![
                                Value::String("--marker".into()),
                                Value::String(MARKER.into()),
                            ]
                        })
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
    let mut state = FragmentState::Absent;
    for group in groups(document)? {
        match marker_state(&group, Some(reporter)) {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
            FragmentState::Installed if state == FragmentState::Absent => {
                state = FragmentState::Installed
            }
            FragmentState::Installed | FragmentState::Drifted => state = FragmentState::Drifted,
        }
    }
    Ok(state)
}

fn merge(document: &mut Map<String, Value>, reporter: &Path) -> Result<(), IntegrationError> {
    let _ = remove(document)?;
    let hooks = document.entry("hooks").or_insert_with(|| json!({}));
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| IntegrationError::InvalidConfig("Claude hooks must be an object.".into()))?;
    let notifications = hooks.entry("Notification").or_insert_with(|| json!([]));
    let notifications = notifications.as_array_mut().ok_or_else(|| {
        IntegrationError::InvalidConfig("Claude Notification hooks must be an array.".into())
    })?;
    notifications.push(json!({"matcher":"*","hooks":[{"type":"command","command":reporter,"args":["--marker",MARKER]}]}));
    Ok(())
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    let existing = groups(document)?;
    let mut state = FragmentState::Absent;
    for group in &existing {
        match marker_state(group, None) {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
            FragmentState::Installed | FragmentState::Drifted => {
                if state != FragmentState::Absent {
                    return Ok(FragmentState::Ambiguous);
                }
                state = FragmentState::Drifted;
            }
        }
    }
    if state == FragmentState::Absent {
        return Ok(state);
    }
    let hooks = document
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .expect("validated hooks object");
    let notifications = hooks
        .get_mut("Notification")
        .and_then(Value::as_array_mut)
        .expect("validated Notification array");
    notifications.retain(|group| marker_state(group, None) == FragmentState::Absent);
    Ok(FragmentState::Drifted)
}
