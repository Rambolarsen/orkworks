use std::path::Path;

use serde_json::{json, Map, Value};

use super::{
    reconcile_current, reporter_invocation_for_platform, FragmentState, JsonHookHandler,
    ReporterPlatform, ToolHookContract,
};
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
    reconcile_current,
);

fn groups(document: &Map<String, Value>, event: &str) -> Result<Vec<Value>, IntegrationError> {
    let Some(hooks) = document.get("hooks") else {
        return Ok(vec![]);
    };
    let hooks = hooks
        .as_object()
        .ok_or_else(|| IntegrationError::InvalidConfig("Claude hooks must be an object.".into()))?;
    hooks.get(event).map_or(Ok(vec![]), |value| {
        value.as_array().cloned().ok_or_else(|| {
            IntegrationError::InvalidConfig(format!("Claude {event} hooks must be an array."))
        })
    })
}

fn reporter_command_for_platform(
    platform: ReporterPlatform,
    reporter: &Path,
    status: Option<&str>,
) -> super::ReporterInvocation {
    let mut invocation = reporter_invocation_for_platform(platform, reporter, MARKER);
    if let Some(status) = status {
        let flag = match platform {
            ReporterPlatform::Posix => "--status",
            ReporterPlatform::WindowsPowerShell => "-Status",
        };
        invocation.args.extend([flag.into(), status.into()]);
    }
    invocation
}

fn reporter_command(reporter: &Path, status: Option<&str>) -> super::ReporterInvocation {
    reporter_command_for_platform(ReporterPlatform::current(), reporter, status)
}

fn marker_state(
    group: &Value,
    reporter: Option<&Path>,
    status: Option<&str>,
    requires_async: bool,
) -> FragmentState {
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
                let invocation = reporter_command(path, status);
                hook.get("type").and_then(Value::as_str) == Some("command")
                    && hook.get("command").and_then(Value::as_str)
                        == Some(invocation.program.as_str())
                    && hook
                        .get("args")
                        .and_then(Value::as_array)
                        .is_some_and(|args| {
                            args == &invocation
                                .args
                                .iter()
                                .cloned()
                                .map(Value::String)
                                .collect::<Vec<_>>()
                        })
                    && (!requires_async || hook.get("async").and_then(Value::as_bool) == Some(true))
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

fn event_state(
    document: &Map<String, Value>,
    event: &str,
    reporter: Option<&Path>,
    status: Option<&str>,
    requires_async: bool,
) -> Result<FragmentState, IntegrationError> {
    let mut state = FragmentState::Absent;
    for group in groups(document, event)? {
        let next = marker_state(&group, reporter, status, requires_async);
        if next == FragmentState::Absent {
            continue;
        }
        if state != FragmentState::Absent || next == FragmentState::Ambiguous {
            return Ok(FragmentState::Ambiguous);
        }
        state = next;
    }
    Ok(state)
}

fn probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    let notification = event_state(document, "Notification", Some(reporter), None, false)?;
    let pre_tool = event_state(
        document,
        "PreToolUse",
        Some(reporter),
        Some("working"),
        true,
    )?;
    if notification == FragmentState::Ambiguous || pre_tool == FragmentState::Ambiguous {
        return Ok(FragmentState::Ambiguous);
    }
    Ok(
        if notification == FragmentState::Installed && pre_tool == FragmentState::Installed {
            FragmentState::Installed
        } else if notification == FragmentState::Absent && pre_tool == FragmentState::Absent {
            FragmentState::Absent
        } else {
            FragmentState::Drifted
        },
    )
}

fn merge(document: &mut Map<String, Value>, reporter: &Path) -> Result<(), IntegrationError> {
    if remove(document)? == FragmentState::Ambiguous {
        return Err(IntegrationError::OwnershipAmbiguous);
    }
    let hooks = document.entry("hooks").or_insert_with(|| json!({}));
    let hooks = hooks
        .as_object_mut()
        .ok_or_else(|| IntegrationError::InvalidConfig("Claude hooks must be an object.".into()))?;
    let notifications = hooks.entry("Notification").or_insert_with(|| json!([]));
    let notifications = notifications.as_array_mut().ok_or_else(|| {
        IntegrationError::InvalidConfig("Claude Notification hooks must be an array.".into())
    })?;
    let invocation = reporter_command(reporter, None);
    notifications.push(json!({"matcher":"*","hooks":[{"type":"command","command":invocation.program,"args":invocation.args}]}));
    let pre_tool = hooks.entry("PreToolUse").or_insert_with(|| json!([]));
    let pre_tool = pre_tool.as_array_mut().ok_or_else(|| {
        IntegrationError::InvalidConfig("Claude PreToolUse hooks must be an array.".into())
    })?;
    let invocation = reporter_command(reporter, Some("working"));
    pre_tool.push(json!({"matcher":"*","hooks":[{"type":"command","command":invocation.program,"args":invocation.args,"async":true}]}));
    Ok(())
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    let mut found = false;
    for event in ["Notification", "PreToolUse"] {
        let state = event_state(document, event, None, None, false)?;
        if state == FragmentState::Ambiguous {
            return Ok(FragmentState::Ambiguous);
        }
        found |= state != FragmentState::Absent;
    }
    if !found {
        return Ok(FragmentState::Absent);
    }
    let hooks = document
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .expect("validated hooks object");
    for event in ["Notification", "PreToolUse"] {
        if let Some(groups) = hooks.get_mut(event).and_then(Value::as_array_mut) {
            groups.retain(|group| marker_state(group, None, None, false) == FragmentState::Absent);
        }
    }
    Ok(FragmentState::Drifted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_installs_a_working_pre_tool_hook() {
        let reporter = Path::new("/tmp/report-harness-event.sh");
        let mut document = Map::new();

        merge(&mut document, reporter).unwrap();

        let hooks = document["hooks"].as_object().unwrap();
        assert_eq!(hooks["Notification"].as_array().unwrap().len(), 1);
        let pre_tool = hooks["PreToolUse"].as_array().unwrap();
        assert_eq!(pre_tool.len(), 1);
        assert_eq!(pre_tool[0]["matcher"], "*");
        assert_eq!(pre_tool[0]["hooks"][0]["async"], true);
        assert_eq!(
            pre_tool[0]["hooks"][0]["args"],
            json!(["--marker", MARKER, "--status", "working"])
        );
    }

    #[test]
    fn working_hook_uses_powershell_parameter_syntax_on_windows() {
        let invocation = reporter_command_for_platform(
            ReporterPlatform::WindowsPowerShell,
            Path::new("C:\\report-harness-event.ps1"),
            Some("working"),
        );
        assert!(invocation.args.windows(2).any(|args| args == ["-Status", "working"]));
    }

    #[test]
    fn probe_marks_a_synchronous_working_hook_as_drifted() {
        let reporter = Path::new("/tmp/report-harness-event.sh");
        let mut document = Map::new();
        merge(&mut document, reporter).unwrap();
        document["hooks"]["PreToolUse"][0]["hooks"][0]["async"] = json!(false);

        assert_eq!(probe(&document, reporter).unwrap(), FragmentState::Drifted);
    }
}
