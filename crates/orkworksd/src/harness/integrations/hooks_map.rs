//! Shared structural operations for JSON hook configurations that store
//! per-event arrays under a top-level `"hooks"` object (`claude`, `codex`,
//! `copilot`, `gemini`).
//!
//! Tool-specific marker extraction, schema validation (e.g. Copilot `version`),
//! exact-shape comparison, and reporter invocation stay in each handler module.

use serde_json::{json, Map, Value};

use super::FragmentState;
use crate::harness::integration::IntegrationError;

/// Returns the borrowed slice of entries under `document["hooks"][event]`.
///
/// Missing `"hooks"` or missing `event` yields an empty slice. A non-object
/// `"hooks"` value or non-array event value fails with `InvalidConfig`.
pub(super) fn event_entries<'a>(
    document: &'a Map<String, Value>,
    event: &str,
    tool_label: &str,
) -> Result<&'a [Value], IntegrationError> {
    let Some(hooks) = document.get("hooks") else {
        return Ok(&[]);
    };
    let hooks = hooks.as_object().ok_or_else(|| {
        IntegrationError::InvalidConfig(format!("{tool_label} hooks must be an object."))
    })?;
    hooks.get(event).map_or(Ok(&[]), |value| {
        value.as_array().map(Vec::as_slice).ok_or_else(|| {
            IntegrationError::InvalidConfig(format!("{tool_label} {event} hooks must be an array."))
        })
    })
}

/// Validates every event array in `events` and reports whether at least one
/// entry exists across them.
pub(super) fn has_any_event_entries(
    document: &Map<String, Value>,
    tool_label: &str,
    events: &[&str],
) -> Result<bool, IntegrationError> {
    let mut found = false;
    for event in events {
        if !event_entries(document, event, tool_label)?.is_empty() {
            found = true;
        }
    }
    Ok(found)
}

/// Classifies a single `{"hooks": [...]}` group object used by Claude, Codex,
/// and Gemini.
///
/// - No hook in `group["hooks"]` carrying a recognized OrkWorks marker -> `Absent`
/// - A different OrkWorks marker, or an owned marker sharing a group with other
///   hooks (`hooks.len() != 1`) -> `Ambiguous`
/// - The sole hook carries `expected_marker` -> `Installed` when `is_exact`
///   returns `true`, otherwise `Drifted`
pub(super) fn classify_hook_group<M, E>(
    group: &Value,
    expected_marker: &str,
    mut extract_marker: M,
    mut is_exact: E,
) -> FragmentState
where
    M: FnMut(&Value) -> Option<&str>,
    E: FnMut(&Value) -> bool,
{
    let Some(hooks) = group.get("hooks").and_then(Value::as_array) else {
        return FragmentState::Absent;
    };
    for hook in hooks {
        let Some(marker) = extract_marker(hook) else {
            continue;
        };
        if marker != expected_marker || hooks.len() != 1 {
            return FragmentState::Ambiguous;
        }
        return if is_exact(hook) {
            FragmentState::Installed
        } else {
            FragmentState::Drifted
        };
    }
    FragmentState::Absent
}

/// Reduces per-entry classifications within one event array.
///
/// At most one owned entry is valid per event: a second non-`Absent` entry or
/// any `Ambiguous` entry yields `Ambiguous`.
pub(super) fn reduce_event_entries<'a, I, F>(entries: I, mut classify: F) -> FragmentState
where
    I: IntoIterator<Item = &'a Value>,
    F: FnMut(&'a Value) -> FragmentState,
{
    let mut state = FragmentState::Absent;
    for entry in entries {
        let next = classify(entry);
        if next == FragmentState::Absent {
            continue;
        }
        if state != FragmentState::Absent || next == FragmentState::Ambiguous {
            return FragmentState::Ambiguous;
        }
        state = next;
    }
    state
}

/// Probes a fixed bundle of required events plus optional legacy events under
/// `document["hooks"]`.
///
/// All event arrays are validated for `InvalidConfig` before returning
/// `Ambiguous` so structural config errors are never masked by earlier
/// ambiguity.
pub(super) fn probe_event_bundle<E, N, F, L>(
    document: &Map<String, Value>,
    tool_label: &str,
    events: &[E],
    mut event_name: N,
    mut classify_event: F,
    legacy_events: &[E],
    mut classify_legacy: L,
) -> Result<FragmentState, IntegrationError>
where
    N: FnMut(&E) -> &str,
    F: FnMut(&E, &Value) -> FragmentState,
    L: FnMut(&E, &Value) -> FragmentState,
{
    let mut installed_count = 0;
    let mut absent_count = 0;
    let mut any_ambiguous = false;

    for event in events {
        let entries = event_entries(document, event_name(event), tool_label)?;
        match reduce_event_entries(entries, |entry| classify_event(event, entry)) {
            FragmentState::Ambiguous => any_ambiguous = true,
            FragmentState::Installed => installed_count += 1,
            FragmentState::Absent => absent_count += 1,
            FragmentState::Drifted => {}
        }
    }

    let mut legacy_present = false;
    for event in legacy_events {
        let entries = event_entries(document, event_name(event), tool_label)?;
        let state = reduce_event_entries(entries, |entry| classify_legacy(event, entry));
        any_ambiguous |= state == FragmentState::Ambiguous;
        legacy_present |= state != FragmentState::Absent;
    }

    if any_ambiguous {
        return Ok(FragmentState::Ambiguous);
    }
    if legacy_present {
        return Ok(FragmentState::Drifted);
    }
    if installed_count == events.len() {
        return Ok(FragmentState::Installed);
    }
    if absent_count == events.len() {
        return Ok(FragmentState::Absent);
    }
    Ok(FragmentState::Drifted)
}

/// Two-phase validate-then-retain removal for owned entries across `events`.
///
/// Phase 1 validates every event array and ensures no event is `Ambiguous`
/// before mutating `document`. If zero owned entries exist, `document` is left
/// untouched and `Absent` is returned. Otherwise owned entries are removed
/// while unrelated user entries and keys remain intact, returning `Drifted`.
pub(super) fn remove_event_bundle<E, N, F>(
    document: &mut Map<String, Value>,
    tool_label: &str,
    events: &[E],
    mut event_name: N,
    mut classify: F,
) -> Result<FragmentState, IntegrationError>
where
    N: FnMut(&E) -> &str,
    F: FnMut(&E, &Value) -> FragmentState,
{
    let mut any_ambiguous = false;
    let mut found_owned = false;

    for event in events {
        let entries = event_entries(document, event_name(event), tool_label)?;
        match reduce_event_entries(entries, |entry| classify(event, entry)) {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => any_ambiguous = true,
            FragmentState::Installed | FragmentState::Drifted => found_owned = true,
        }
    }

    if any_ambiguous {
        return Ok(FragmentState::Ambiguous);
    }
    if !found_owned {
        return Ok(FragmentState::Absent);
    }

    let Some(hooks) = document.get_mut("hooks").and_then(Value::as_object_mut) else {
        return Ok(FragmentState::Absent);
    };
    for event in events {
        if let Some(entries) = hooks
            .get_mut(event_name(event))
            .and_then(Value::as_array_mut)
        {
            entries.retain(|entry| classify(event, entry) == FragmentState::Absent);
        }
    }
    Ok(FragmentState::Drifted)
}

/// Appends `(event, entry)` pairs into `document["hooks"][event]`, validating
/// the existing `"hooks"` object and all target event arrays before mutating
/// `document` so an `InvalidConfig` error never leaves a partial write.
pub(super) fn append_event_entries<'a, I>(
    document: &mut Map<String, Value>,
    tool_label: &str,
    entries: I,
) -> Result<(), IntegrationError>
where
    I: IntoIterator<Item = (&'a str, Value)>,
{
    let items: Vec<(&'a str, Value)> = entries.into_iter().collect();
    for (event, _) in &items {
        let _ = event_entries(document, event, tool_label)?;
    }
    let hooks = document
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| {
            IntegrationError::InvalidConfig(format!("{tool_label} hooks must be an object."))
        })?;
    for (event, entry) in items {
        let array = hooks
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| {
                IntegrationError::InvalidConfig(format!(
                    "{tool_label} {event} hooks must be an array."
                ))
            })?;
        array.push(entry);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OWNED_MARKER: &str = "orkworks:harness-integration:v2:test";

    fn extract_test_marker(hook: &Value) -> Option<&str> {
        hook.get("marker")
            .and_then(Value::as_str)
            .filter(|value| value.starts_with("orkworks:harness-integration:"))
    }

    fn classify_test_group(group: &Value, expected_cmd: Option<&str>) -> FragmentState {
        classify_hook_group(group, OWNED_MARKER, extract_test_marker, |hook| {
            expected_cmd.is_some_and(|cmd| hook.get("command").and_then(Value::as_str) == Some(cmd))
        })
    }

    #[test]
    fn classify_hook_group_enforces_marker_and_single_hook_ownership() {
        let foreign = json!({"hooks": [{"command": "echo hi"}]});
        assert_eq!(
            classify_test_group(&foreign, Some("ok")),
            FragmentState::Absent
        );

        let wrong_marker = json!({"hooks": [{"marker": "orkworks:harness-integration:v2:other", "command": "ok"}]});
        assert_eq!(
            classify_test_group(&wrong_marker, Some("ok")),
            FragmentState::Ambiguous
        );

        let shared_group = json!({
            "hooks": [
                {"command": "echo user"},
                {"marker": OWNED_MARKER, "command": "ok"}
            ]
        });
        assert_eq!(
            classify_test_group(&shared_group, Some("ok")),
            FragmentState::Ambiguous
        );

        let owned_exact = json!({"hooks": [{"marker": OWNED_MARKER, "command": "ok"}]});
        assert_eq!(
            classify_test_group(&owned_exact, Some("ok")),
            FragmentState::Installed
        );
        assert_eq!(
            classify_test_group(&owned_exact, Some("other")),
            FragmentState::Drifted
        );
        assert_eq!(
            classify_test_group(&owned_exact, None),
            FragmentState::Drifted
        );
    }

    #[test]
    fn reduce_event_entries_marks_duplicate_owned_entries_ambiguous() {
        let user = json!({"hooks": [{"command": "user"}]});
        let installed = json!({"hooks": [{"marker": OWNED_MARKER, "command": "ok"}]});
        let drifted = json!({"hooks": [{"marker": OWNED_MARKER, "command": "stale"}]});

        assert_eq!(
            reduce_event_entries([&user], |entry| classify_test_group(entry, Some("ok"))),
            FragmentState::Absent
        );
        assert_eq!(
            reduce_event_entries([&user, &installed], |entry| {
                classify_test_group(entry, Some("ok"))
            }),
            FragmentState::Installed
        );
        assert_eq!(
            reduce_event_entries([&installed, &installed], |entry| {
                classify_test_group(entry, Some("ok"))
            }),
            FragmentState::Ambiguous
        );
        assert_eq!(
            reduce_event_entries([&installed, &drifted], |entry| {
                classify_test_group(entry, Some("ok"))
            }),
            FragmentState::Ambiguous
        );
        assert_eq!(
            reduce_event_entries([&drifted, &drifted], |entry| {
                classify_test_group(entry, Some("ok"))
            }),
            FragmentState::Ambiguous
        );
    }

    #[test]
    fn probe_event_bundle_aggregates_partial_legacy_and_invalid_config_precedence() {
        let events = ["A", "B"];
        let legacy = ["Old"];

        let complete = json!({
            "hooks": {
                "A": [{"hooks": [{"marker": OWNED_MARKER, "command": "cmd-A"}]}],
                "B": [{"hooks": [{"marker": OWNED_MARKER, "command": "cmd-B"}]}]
            }
        });
        assert_eq!(
            probe_event_bundle(
                complete.as_object().unwrap(),
                "Test",
                &events,
                |e| e,
                |e, entry| classify_test_group(entry, Some(&format!("cmd-{e}"))),
                &legacy,
                |_, entry| classify_test_group(entry, None),
            )
            .unwrap(),
            FragmentState::Installed
        );

        let partial = json!({
            "hooks": {
                "A": [{"hooks": [{"marker": OWNED_MARKER, "command": "cmd-A"}]}]
            }
        });
        assert_eq!(
            probe_event_bundle(
                partial.as_object().unwrap(),
                "Test",
                &events,
                |e| e,
                |e, entry| classify_test_group(entry, Some(&format!("cmd-{e}"))),
                &legacy,
                |_, entry| classify_test_group(entry, None),
            )
            .unwrap(),
            FragmentState::Drifted
        );

        let with_legacy = json!({
            "hooks": {
                "A": [{"hooks": [{"marker": OWNED_MARKER, "command": "cmd-A"}]}],
                "B": [{"hooks": [{"marker": OWNED_MARKER, "command": "cmd-B"}]}],
                "Old": [{"hooks": [{"marker": OWNED_MARKER, "command": "cmd-Old"}]}]
            }
        });
        assert_eq!(
            probe_event_bundle(
                with_legacy.as_object().unwrap(),
                "Test",
                &events,
                |e| e,
                |e, entry| classify_test_group(entry, Some(&format!("cmd-{e}"))),
                &legacy,
                |_, entry| classify_test_group(entry, None),
            )
            .unwrap(),
            FragmentState::Drifted
        );

        // InvalidConfig in event B must still surface even when event A is Ambiguous.
        let ambiguous_then_invalid = json!({
            "hooks": {
                "A": [{"hooks": [{"marker": "orkworks:harness-integration:v2:other"}]}],
                "B": "not-an-array"
            }
        });
        let err = probe_event_bundle(
            ambiguous_then_invalid.as_object().unwrap(),
            "Test",
            &events,
            |e| e,
            |e, entry| classify_test_group(entry, Some(&format!("cmd-{e}"))),
            &legacy,
            |_, entry| classify_test_group(entry, None),
        )
        .unwrap_err();
        assert_eq!(err.code(), "invalid_config");
    }

    #[test]
    fn remove_event_bundle_is_atomic_on_ambiguity_and_preserves_unrelated_entries() {
        let events = ["A", "B"];

        // Ambiguous in event B must prevent mutating event A.
        let mut ambiguous_doc = json!({
            "hooks": {
                "A": [{"hooks": [{"marker": OWNED_MARKER, "command": "cmd-A"}]}],
                "B": [
                    {"hooks": [{"marker": OWNED_MARKER, "command": "cmd-B"}]},
                    {"hooks": [{"marker": OWNED_MARKER, "command": "cmd-B"}]}
                ]
            }
        })
        .as_object()
        .unwrap()
        .clone();
        let before = ambiguous_doc.clone();
        assert_eq!(
            remove_event_bundle(
                &mut ambiguous_doc,
                "Test",
                &events,
                |e| e,
                |_, entry| { classify_test_group(entry, None) }
            )
            .unwrap(),
            FragmentState::Ambiguous
        );
        assert_eq!(ambiguous_doc, before);

        // Valid removal strips owned entries only and preserves user entries and unrelated events.
        let mut doc = json!({
            "otherKey": true,
            "hooks": {
                "A": [
                    {"hooks": [{"command": "user-a"}]},
                    {"hooks": [{"marker": OWNED_MARKER, "command": "cmd-A"}]}
                ],
                "B": [
                    {"hooks": [{"marker": OWNED_MARKER, "command": "cmd-B"}]}
                ],
                "Custom": [{"hooks": [{"command": "user-custom"}]}]
            }
        })
        .as_object()
        .unwrap()
        .clone();
        assert_eq!(
            remove_event_bundle(
                &mut doc,
                "Test",
                &events,
                |e| e,
                |_, entry| { classify_test_group(entry, None) }
            )
            .unwrap(),
            FragmentState::Drifted
        );
        assert_eq!(doc["otherKey"], json!(true));
        assert_eq!(
            doc["hooks"]["A"],
            json!([{"hooks": [{"command": "user-a"}]}])
        );
        assert_eq!(doc["hooks"]["B"], json!([]));
        assert_eq!(
            doc["hooks"]["Custom"],
            json!([{"hooks": [{"command": "user-custom"}]}])
        );

        // Second remove is an Absent no-op.
        assert_eq!(
            remove_event_bundle(
                &mut doc,
                "Test",
                &events,
                |e| e,
                |_, entry| { classify_test_group(entry, None) }
            )
            .unwrap(),
            FragmentState::Absent
        );
    }

    #[test]
    fn append_event_entries_fails_atomically_on_malformed_target_event() {
        let mut doc = json!({
            "hooks": {
                "A": [{"hooks": [{"command": "user"}]}],
                "B": "not-an-array"
            }
        })
        .as_object()
        .unwrap()
        .clone();
        let before = doc.clone();
        let err = append_event_entries(
            &mut doc,
            "Test",
            [("A", json!({"n": 1})), ("B", json!({"n": 2}))],
        )
        .unwrap_err();
        assert_eq!(err.code(), "invalid_config");
        assert_eq!(doc, before);
    }
}
