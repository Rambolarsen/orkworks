use std::path::Path;

use serde_json::{json, Map, Value};

use super::{
    reconcile_current, reporter_invocation, FragmentState, JsonHookHandler, ToolHookContract,
};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

const MARKER: &str = "orkworks:harness-integration:v2:codex";
// Codex's hook definitions have no dedicated "name"/marker field (unlike
// Claude's args array or Gemini's "name" key) — ownership is recognized by
// extracting the marker value from inside the single shell-interpreted
// "command" string produced by `reporter_invocation`, which always embeds
// `--marker '<value>'`. Markers we generate are fixed literals with no
// embedded quotes, so reading up to the closing `'` recovers the exact value.
const MARKER_PREFIX: &str = "orkworks:harness-integration:";

fn extract_marker(command: &str) -> Option<&str> {
    let start = command.find(MARKER_PREFIX)?;
    let rest = &command[start..];
    let end = rest.find('\'').unwrap_or(rest.len());
    Some(&rest[..end])
}

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "codex",
        tool_name: "Codex",
        relative_path: ".codex/hooks.json",
        ownership_marker: MARKER,
        coverage: IntegrationCoverage::Limited,
        // Codex requires a one-time `/hooks` approval inside the tool before
        // an installed hook definition actually runs (hash-pinned trust).
        // Installing the file is not the same as it being active yet.
        activation: IntegrationActivation::NeedsTrust,
    },
    probe,
    merge,
    remove,
    reconcile_current,
);

fn groups(document: &Map<String, Value>) -> Result<Vec<Value>, IntegrationError> {
    let Some(hooks) = document.get("SessionStart") else {
        return Ok(vec![]);
    };
    hooks.as_array().cloned().ok_or_else(|| {
        IntegrationError::InvalidConfig("Codex SessionStart hooks must be an array.".into())
    })
}

fn marker_state(group: &Value, reporter: Option<&Path>) -> FragmentState {
    let Some(hooks) = group.get("hooks").and_then(Value::as_array) else {
        return FragmentState::Absent;
    };
    let mut found = None;
    for hook in hooks {
        let Some(command) = hook.get("command").and_then(Value::as_str) else {
            continue;
        };
        let Some(marker) = extract_marker(command) else {
            continue;
        };
        if marker != MARKER || hooks.len() != 1 {
            return FragmentState::Ambiguous;
        }
        let exact = reporter.is_some_and(|path| {
            let invocation = reporter_invocation(path, MARKER);
            hook.get("type").and_then(Value::as_str) == Some("command")
                && command == invocation.shell_command.as_str()
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
    found.unwrap_or(FragmentState::Absent)
}

fn probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    let mut state = FragmentState::Absent;
    for group in groups(document)? {
        let next = marker_state(&group, Some(reporter));
        if state != FragmentState::Absent && next != FragmentState::Absent {
            return Ok(FragmentState::Ambiguous);
        }
        match next {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
            FragmentState::Installed => state = FragmentState::Installed,
            FragmentState::Drifted => state = FragmentState::Drifted,
        }
    }
    Ok(state)
}

fn merge(document: &mut Map<String, Value>, reporter: &Path) -> Result<(), IntegrationError> {
    if remove(document)? == FragmentState::Ambiguous {
        return Err(IntegrationError::OwnershipAmbiguous);
    }
    let session_start = document
        .entry("SessionStart")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| {
            IntegrationError::InvalidConfig("Codex SessionStart hooks must be an array.".into())
        })?;
    let invocation = reporter_invocation(reporter, MARKER);
    session_start.push(json!({"hooks":[{"type":"command","command":invocation.shell_command}]}));
    Ok(())
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    let existing = groups(document)?;
    let mut count = 0;
    for group in &existing {
        match marker_state(group, None) {
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
    let session_start = document
        .get_mut("SessionStart")
        .and_then(Value::as_array_mut)
        .expect("validated SessionStart array");
    session_start.retain(|group| marker_state(group, None) == FragmentState::Absent);
    Ok(FragmentState::Drifted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_state_treats_a_foreign_harness_marker_as_ambiguous_not_drifted() {
        // A stray Claude Code marker sitting alone in .codex/hooks.json (e.g.
        // copy-pasted by mistake) must never be treated as codex's own
        // fragment with a stale invocation — that would make install/
        // uninstall silently overwrite or delete a different harness's hook.
        let group = json!({
            "hooks": [
                {
                    "type": "command",
                    "command": "/path/to/report-harness-event.sh --marker 'orkworks:harness-integration:v2:claude-code'"
                }
            ]
        });
        let reporter = Path::new("/path/to/report-harness-event.sh");

        assert_eq!(marker_state(&group, Some(reporter)), FragmentState::Ambiguous);
    }
}
