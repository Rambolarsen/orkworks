use std::path::Path;

use serde_json::{json, Map, Value};

use super::{
    portable_reporter_invocation, reconcile_current, reporter_invocation, FragmentState,
    JsonHookHandler, ReporterInvocation, ReporterPlatform, ToolHookContract,
};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

/// `portable_reporter_invocation` is POSIX-only (ADR 0036) — Windows keeps
/// the standard, resolved-absolute-path invocation, matching `load()`'s own
/// platform check for which safety gate applies. Without this branch,
/// `probe`/`merge` would write a POSIX-shaped `$HOME`-relative command on
/// Windows even in the untracked-and-ignored case that worked before this
/// change, breaking Windows Codex installs entirely rather than just
/// falling back for the tracked case.
fn platform_invocation(reporter: &Path) -> Result<ReporterInvocation, IntegrationError> {
    if ReporterPlatform::current() == ReporterPlatform::Posix {
        portable_reporter_invocation(reporter, MARKER)
    } else {
        Ok(reporter_invocation(reporter, MARKER))
    }
}

const MARKER: &str = "orkworks:harness-integration:v2:codex";
// Codex's hook definitions have no dedicated "name"/marker field (unlike
// Claude's args array or Gemini's "name" key) — ownership is recognized by
// extracting the marker value from inside the single shell-interpreted
// "command" string produced by `reporter_invocation`, which always embeds
// the marker as the quoted value of a `--marker`/`-Marker` flag (POSIX vs
// PowerShell). Requiring that exact flag structure — not just the marker
// text appearing anywhere in the string — keeps an unrelated command that
// merely mentions the marker (e.g. in an echo or comment) from being
// misidentified as ours.
const MARKER_PREFIX: &str = "orkworks:harness-integration:";
const MARKER_FLAGS: [&str; 2] = ["--marker '", "-Marker '"];

fn extract_marker(command: &str) -> Option<&str> {
    for flag in MARKER_FLAGS {
        let Some(pos) = command.find(flag) else {
            continue;
        };
        let rest = &command[pos + flag.len()..];
        if !rest.starts_with(MARKER_PREFIX) {
            continue;
        }
        let end = rest.find('\'').unwrap_or(rest.len());
        return Some(&rest[..end]);
    }
    None
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
    let Some(hooks) = document.get("hooks") else {
        return Ok(vec![]);
    };
    let hooks = hooks
        .as_object()
        .ok_or_else(|| IntegrationError::InvalidConfig("Codex hooks must be an object.".into()))?;
    hooks.get("SessionStart").map_or(Ok(vec![]), |value| {
        value.as_array().cloned().ok_or_else(|| {
            IntegrationError::InvalidConfig("Codex SessionStart hooks must be an array.".into())
        })
    })
}

fn marker_state(group: &Value, expected: Option<&ReporterInvocation>) -> FragmentState {
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
        let exact = expected.is_some_and(|invocation| {
            // merge() never sets an outer "matcher" — it intentionally
            // matches every SessionStart source. A group edited to add one
            // (e.g. narrowing to "resume") stops firing on startup/clear/
            // compact even though the inner command is untouched, so that
            // must not read as Installed.
            group.get("matcher").is_none()
                && hook.get("type").and_then(Value::as_str) == Some("command")
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
    let invocation = platform_invocation(reporter)?;
    // A committed fragment can already byte-match this machine's expected
    // command — that's the whole point of the portable rewrite (ADR 0036),
    // since a teammate's tracked .codex/hooks.json can carry a fragment
    // this machine never wrote. But `install()` only reconciles the local
    // reporter-script copy on its Absent/Drifted branch (JsonHookHandler::
    // install, integrations/mod.rs) — it never runs for an already-
    // Installed probe. Before Codex could accept a tracked target, that
    // branch was unreachable on a fresh machine (the target could never be
    // pre-populated), so this gap was latent. Requiring the local reporter
    // script to actually exist before calling a text match "exact" keeps a
    // fresh teammate's probe at Drifted instead of a false Installed, so
    // install() reconciles the missing script instead of leaving a hook
    // that reports installed but can never run.
    let expected = if reporter.try_exists().unwrap_or(false) {
        Some(&invocation)
    } else {
        None
    };
    let mut state = FragmentState::Absent;
    for group in groups(document)? {
        let next = marker_state(&group, expected);
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
    let hooks = document
        .entry("hooks")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .ok_or_else(|| IntegrationError::InvalidConfig("Codex hooks must be an object.".into()))?;
    let session_start = hooks
        .entry("SessionStart")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| {
            IntegrationError::InvalidConfig("Codex SessionStart hooks must be an array.".into())
        })?;
    let invocation = platform_invocation(reporter)?;
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
    let hooks = document
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .expect("validated hooks object");
    let session_start = hooks
        .get_mut("SessionStart")
        .and_then(Value::as_array_mut)
        .expect("validated SessionStart array");
    session_start.retain(|group| marker_state(group, None) == FragmentState::Absent);
    Ok(FragmentState::Drifted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::FakeHome;

    fn reporter_path(home: &std::path::Path) -> std::path::PathBuf {
        home.join(".orkworks/hook-scripts/report-harness-event.sh")
    }

    #[test]
    fn marker_state_treats_a_foreign_harness_marker_as_ambiguous_not_drifted() {
        // A stray Claude Code marker sitting alone in .codex/hooks.json (e.g.
        // copy-pasted by mistake) must never be treated as codex's own
        // fragment with a stale invocation — that would make install/
        // uninstall silently overwrite or delete a different harness's hook.
        // The ambiguity check runs before the exact-match check, so a
        // placeholder invocation is fine here — its content is never read.
        let group = json!({
            "hooks": [
                {
                    "type": "command",
                    "command": "/path/to/report-harness-event.sh --marker 'orkworks:harness-integration:v2:claude-code'"
                }
            ]
        });
        let invocation = ReporterInvocation {
            program: String::new(),
            args: vec![],
            shell_command: String::new(),
        };

        assert_eq!(
            marker_state(&group, Some(&invocation)),
            FragmentState::Ambiguous
        );
    }

    #[test]
    fn merge_writes_session_start_nested_under_hooks_object_matching_the_real_codex_schema() {
        // The real .codex/hooks.json committed in this repo (installed by
        // APM's ponytail plugin) nests every event, including SessionStart,
        // under a top-level "hooks" object — the same shape claude.rs and
        // gemini.rs already use for their own events. A prior version of
        // this handler read/wrote a root-level "SessionStart" key instead,
        // which Codex silently ignores.
        let mut document = Map::new();
        document.insert(
            "hooks".into(),
            json!({
                "Stop": [{"hooks": [{"type": "command", "command": "some-other-hook.sh"}]}]
            }),
        );
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());

        merge(&mut document, &reporter_path(home.path())).unwrap();

        let hooks = document
            .get("hooks")
            .and_then(Value::as_object)
            .expect("hooks object");
        assert!(
            hooks.contains_key("Stop"),
            "must preserve the existing Stop hook group"
        );
        let session_start = hooks
            .get("SessionStart")
            .and_then(Value::as_array)
            .expect("SessionStart must be nested under hooks");
        assert_eq!(session_start.len(), 1);
        assert!(
            document.get("SessionStart").is_none(),
            "must not also write a stray root-level SessionStart key"
        );
    }

    #[test]
    fn extract_marker_ignores_the_marker_text_appearing_outside_the_marker_flag() {
        // A user's unrelated command that merely mentions the marker string
        // (e.g. in an echo or a comment) must not be claimed as ours.
        let command = "echo 'see orkworks:harness-integration:v2:codex in the docs'";
        assert_eq!(extract_marker(command), None);
    }

    #[test]
    fn marker_state_reports_drifted_when_a_matcher_narrows_which_sources_fire() {
        // merge() never sets "matcher" (it intentionally matches every
        // source). A group edited to add one, e.g. "matcher":"resume", no
        // longer fires on startup/clear/compact even though the inner
        // command is byte-for-byte what we generate — it must not be
        // reported Installed.
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let invocation =
            portable_reporter_invocation(&reporter_path(home.path()), MARKER).unwrap();
        let group = json!({
            "matcher": "resume",
            "hooks": [
                {"type": "command", "command": invocation.shell_command}
            ]
        });

        assert_eq!(
            marker_state(&group, Some(&invocation)),
            FragmentState::Drifted
        );
    }

    // These three tests exercise merge()/probe() through platform_invocation,
    // which is POSIX-only by design (ADR 0036) — on Windows it takes the
    // reporter_invocation branch instead, producing a powershell.exe/-File
    // command these assertions don't expect.
    #[cfg(unix)]
    #[test]
    fn merge_writes_a_home_relative_command_not_an_absolute_path() {
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let mut document = Map::new();

        merge(&mut document, &reporter_path(home.path())).unwrap();

        let command = document["hooks"]["SessionStart"][0]["hooks"][0]["command"]
            .as_str()
            .unwrap()
            .to_string();
        assert!(
            command.starts_with("\"$HOME/"),
            "expected a $HOME-relative command, got: {command}"
        );
        assert!(
            !command.contains(home.path().to_str().unwrap()),
            "command must not embed the real (machine-specific) home directory: {command}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn probe_reports_installed_after_merge_and_drifted_for_a_pre_portable_absolute_path_fragment()
    {
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let script = reporter_path(home.path());
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, "#!/bin/sh\n").unwrap();
        let mut document = Map::new();
        merge(&mut document, &script).unwrap();
        assert_eq!(probe(&document, &script).unwrap(), FragmentState::Installed);

        // Simulates a fragment written by a pre-fix OrkWorks version, which
        // embedded the resolved absolute path instead of a $HOME-relative
        // one — must read as Drifted (triggering reconciliation on the next
        // install), never silently as Installed.
        let mut stale = Map::new();
        stale.insert(
            "hooks".into(),
            json!({
                "SessionStart": [{
                    "hooks": [{
                        "type": "command",
                        "command": format!(
                            "{} --marker '{}'",
                            reporter_path(home.path()).display(),
                            MARKER
                        )
                    }]
                }]
            }),
        );
        assert_eq!(
            probe(&stale, &reporter_path(home.path())).unwrap(),
            FragmentState::Drifted
        );
    }

    #[test]
    fn probe_reports_drifted_not_installed_when_the_local_reporter_script_is_missing() {
        // A teammate's tracked .codex/hooks.json can already carry a
        // byte-identical, committed OrkWorks fragment (that's the whole
        // point of the portable rewrite) on a machine that has never
        // reconciled its own copy of the reporter script — reconcile only
        // runs from install()'s Absent/Drifted branch. If probe() called
        // this Installed anyway, the UI would show "installed" for a hook
        // that can never actually run, with no install-time trigger left
        // to fix it. Building the document via merge() (rather than a
        // second FakeHome-scoped tempdir) keeps the command text real
        // without ever writing the reporter script to disk.
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let mut document = Map::new();
        merge(&mut document, &reporter_path(home.path())).unwrap();

        assert_eq!(
            probe(&document, &reporter_path(home.path())).unwrap(),
            FragmentState::Drifted
        );
    }
}
