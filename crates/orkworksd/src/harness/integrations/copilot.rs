use std::path::Path;

use serde_json::{json, Map, Value};

use super::{
    event_reporter_invocation_for_platform, FragmentState, JsonHookHandler, ReporterPlatform,
    ToolHookContract,
};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

const MARKER: &str = "orkworks:harness-integration:v2:copilot";
const EVENTS: [(&str, &str); 2] = [
    ("notification", "notification"),
    ("sessionStart", "sessionStart"),
];

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "copilot",
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
    reconcile_reporters,
);

fn reconcile_reporters(
    resolver: &crate::harness::integration::ReporterAssetResolver,
) -> Result<std::path::PathBuf, IntegrationError> {
    resolver.reconcile(ReporterPlatform::Posix.asset_name())?;
    resolver.reconcile(ReporterPlatform::WindowsPowerShell.asset_name())?;
    resolver.stable_path(ReporterPlatform::current().asset_name())
}

fn reporter_paths(reporter: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let parent = reporter.parent().unwrap_or_else(|| Path::new("."));
    (
        parent.join(ReporterPlatform::Posix.asset_name()),
        parent.join(ReporterPlatform::WindowsPowerShell.asset_name()),
    )
}

fn hooks(document: &Map<String, Value>, event: &str) -> Result<Vec<Value>, IntegrationError> {
    if document
        .get("version")
        .is_some_and(|version| version != &Value::from(1))
    {
        return Err(IntegrationError::InvalidConfig(
            "Copilot inline hooks require top-level version 1.".into(),
        ));
    }
    let Some(value) = document.get("hooks") else {
        return Ok(vec![]);
    };
    let hooks = value.as_object().ok_or_else(|| {
        IntegrationError::InvalidConfig("Copilot hooks must be an object.".into())
    })?;
    hooks.get(event).map_or(Ok(vec![]), |value| {
        value.as_array().cloned().ok_or_else(|| {
            IntegrationError::InvalidConfig(format!("Copilot {event} hooks must be an array."))
        })
    })
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
    let mut installed_count = 0;
    let mut absent_count = 0;
    let mut ambiguous = false;
    for (key, event) in EVENTS {
        let mut result = FragmentState::Absent;
        for hook in hooks(document, key)? {
            let next = state(&hook, Some(reporter), event);
            if result != FragmentState::Absent && next != FragmentState::Absent {
                result = FragmentState::Ambiguous;
                break;
            }
            match next {
                FragmentState::Absent => {}
                FragmentState::Ambiguous => {
                    result = FragmentState::Ambiguous;
                    break;
                }
                FragmentState::Installed | FragmentState::Drifted => result = next,
            }
        }
        match result {
            FragmentState::Absent => absent_count += 1,
            FragmentState::Installed => installed_count += 1,
            FragmentState::Drifted => {}
            FragmentState::Ambiguous => ambiguous = true,
        }
    }
    if ambiguous {
        Ok(FragmentState::Ambiguous)
    } else if installed_count == EVENTS.len() {
        Ok(FragmentState::Installed)
    } else if absent_count == EVENTS.len() {
        Ok(FragmentState::Absent)
    } else {
        Ok(FragmentState::Drifted)
    }
}

pub(super) fn prompt_attention_probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    let mut result = FragmentState::Absent;
    for hook in hooks(document, "notification")? {
        match state(&hook, Some(reporter), "notification") {
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
    match document.get("version") {
        None => {
            document.insert("version".into(), Value::from(1));
        }
        Some(version) if version == &Value::from(1) => {}
        Some(_) => {
            return Err(IntegrationError::InvalidConfig(
                "Copilot inline hooks require top-level version 1.".into(),
            ))
        }
    }
    let hooks = document
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| {
            IntegrationError::InvalidConfig("Copilot hooks must be an object.".into())
        })?;
    let (posix_path, powershell_path) = reporter_paths(reporter);
    for (key, event) in EVENTS {
        let entries = hooks
            .entry(key)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| {
                IntegrationError::InvalidConfig(format!("Copilot {key} hooks must be an array."))
            })?;
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
        entries.push(json!({"type":"command","bash":posix.shell_command,"powershell":powershell.shell_command,"env":{"ORKWORKS_INTEGRATION_MARKER":MARKER}}));
    }
    Ok(())
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    let mut found_owned = false;
    for (key, event) in EVENTS {
        let mut event_count = 0;
        for hook in hooks(document, key)? {
            match state(&hook, None, event) {
                FragmentState::Absent => {}
                FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
                _ => event_count += 1,
            }
        }
        if event_count > 1 {
            return Ok(FragmentState::Ambiguous);
        }
        found_owned |= event_count == 1;
    }
    if !found_owned {
        return Ok(FragmentState::Absent);
    }
    let hooks = document
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .expect("validated hooks object");
    for (key, event) in EVENTS {
        if let Some(entries) = hooks.get_mut(key).and_then(Value::as_array_mut) {
            entries.retain(|hook| state(hook, None, event) == FragmentState::Absent);
        }
    }
    Ok(FragmentState::Drifted)
}
