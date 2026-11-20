use std::path::Path;

use serde_json::{json, Map, Value};

use super::{
    portable_reporter_invocation, reporter_invocation, FragmentState, JsonHookHandler,
    ReporterInvocation, ReporterPlatform, ToolHookContract,
};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

const MARKER: &str = "orkworks:harness-integration:v2:antigravity";
/// Antigravity's `.agents/hooks.json` maps a hook *name* to a definition that
/// holds event arrays, so there is no shared top-level `hooks` object to
/// merge into: OrkWorks owns exactly one named definition.
const OWNED_DEFINITION: &str = "orkworks-harness-integration";
/// `SessionStart` and the first `PreInvocation` capture the conversation ID,
/// `PreInvocation` reports working, and a fully idle `Stop` reports idle.
/// `PreToolUse` is deliberately absent: its hook output carries a permission
/// `decision`, and an observer must never take part in permissions.
const EVENTS: [&str; 3] = ["SessionStart", "PreInvocation", "Stop"];

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "antigravity",
        additional_reporter_assets: &[],
        tool_name: "Antigravity CLI",
        relative_path: ".agents/hooks.json",
        ownership_marker: MARKER,
        coverage: IntegrationCoverage::Limited,
        reports_attention: true,
        // Workspace hooks ran without a trust prompt in the 2026-11-20 live
        // fixture, unlike Codex's hash-pinned `/hooks` approval.
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

/// Same `$HOME`-relative POSIX command as Codex so a committed file is
/// byte-identical for every teammate (ADR 0036); Windows keeps the resolved
/// absolute path.
fn invocation(reporter: &Path, event: &str) -> Result<ReporterInvocation, IntegrationError> {
    let posix = ReporterPlatform::current() == ReporterPlatform::Posix;
    let mut invocation = if posix {
        portable_reporter_invocation(reporter, MARKER)?
    } else {
        reporter_invocation(reporter, MARKER)
    };
    let flag = if posix { "--event" } else { "-Event" };
    invocation
        .shell_command
        .push_str(&format!(" {flag} {}", super::shell_quote(event)));
    Ok(invocation)
}

fn definition(
    document: &Map<String, Value>,
) -> Result<Option<&Map<String, Value>>, IntegrationError> {
    document.get(OWNED_DEFINITION).map_or(Ok(None), |value| {
        value.as_object().map(Some).ok_or_else(|| {
            IntegrationError::InvalidConfig("Antigravity hook definitions must be objects.".into())
        })
    })
}

fn handlers<'a>(
    definition: &'a Map<String, Value>,
    event: &str,
) -> Result<&'a [Value], IntegrationError> {
    definition.get(event).map_or(Ok(&[][..]), |value| {
        value.as_array().map(Vec::as_slice).ok_or_else(|| {
            IntegrationError::InvalidConfig(format!("Antigravity {event} hooks must be an array."))
        })
    })
}

/// The OrkWorks marker carried by one handler's command, if any.
fn owned_marker(handler: &Value) -> Option<&str> {
    handler
        .get("command")
        .and_then(Value::as_str)
        .and_then(super::codex::extract_marker)
}

fn probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    let Some(definition) = definition(document)? else {
        return Ok(FragmentState::Absent);
    };
    let mut installed_events = 0;
    let mut saw_owned = false;
    for (key, value) in definition {
        // Only the three installed events plus the documented `enabled` flag
        // belong to an OrkWorks definition; anything else means a user shaped
        // this definition and replacing it would destroy their configuration.
        if key != "enabled" && !EVENTS.contains(&key.as_str()) {
            return Ok(FragmentState::Ambiguous);
        }
        if key == "enabled" && !value.is_boolean() {
            return Ok(FragmentState::Ambiguous);
        }
    }
    let expected_available = reporter.try_exists().unwrap_or(false);
    for event in EVENTS {
        let event_handlers = handlers(definition, event)?;
        let mut owned_in_event = 0;
        for handler in event_handlers {
            match owned_marker(handler) {
                Some(MARKER) => {
                    owned_in_event += 1;
                    saw_owned = true;
                    let exact = expected_available
                        && handler.get("type").and_then(Value::as_str) == Some("command")
                        && handler
                            .as_object()
                            .is_some_and(|handler| handler.len() == 2)
                        && handler.get("command").and_then(Value::as_str)
                            == Some(invocation(reporter, event)?.shell_command.as_str());
                    installed_events += usize::from(exact);
                }
                _ => return Ok(FragmentState::Ambiguous),
            }
        }
        if owned_in_event > 1 {
            return Ok(FragmentState::Ambiguous);
        }
    }
    if !saw_owned {
        return Ok(FragmentState::Ambiguous);
    }
    if installed_events == EVENTS.len() && definition.get("enabled") != Some(&Value::Bool(false)) {
        Ok(FragmentState::Installed)
    } else {
        Ok(FragmentState::Drifted)
    }
}

fn merge(document: &mut Map<String, Value>, reporter: &Path) -> Result<(), IntegrationError> {
    if remove(document)? == FragmentState::Ambiguous {
        return Err(IntegrationError::OwnershipAmbiguous);
    }
    let mut owned = Map::new();
    for event in EVENTS {
        owned.insert(
            event.into(),
            json!([{"type": "command", "command": invocation(reporter, event)?.shell_command}]),
        );
    }
    document.insert(OWNED_DEFINITION.into(), Value::Object(owned));
    Ok(())
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    // An all-foreign or hand-edited definition under our name is not ours to
    // delete; the empty reporter path is fine because only ownership matters.
    match probe(document, Path::new(""))? {
        FragmentState::Absent => Ok(FragmentState::Absent),
        FragmentState::Ambiguous => Ok(FragmentState::Ambiguous),
        FragmentState::Installed | FragmentState::Drifted => {
            document.remove(OWNED_DEFINITION);
            Ok(FragmentState::Drifted)
        }
    }
}
