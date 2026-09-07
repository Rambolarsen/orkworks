use std::path::Path;

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

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
const CODEX_EVENTS: [&str; 4] = [
    "SessionStart",
    "UserPromptSubmit",
    "PermissionRequest",
    "Stop",
];

fn base_platform_invocation(
    reporter: &Path,
    event: &str,
) -> Result<ReporterInvocation, IntegrationError> {
    let mut invocation = if ReporterPlatform::current() == ReporterPlatform::Posix {
        portable_reporter_invocation(reporter, MARKER)?
    } else {
        reporter_invocation(reporter, MARKER)
    };
    let event_flag = if ReporterPlatform::current() == ReporterPlatform::Posix {
        "--event"
    } else {
        "-Event"
    };
    invocation.args.extend([event_flag.into(), event.into()]);
    invocation.shell_command.push(' ');
    invocation.shell_command.push_str(event_flag);
    invocation.shell_command.push(' ');
    invocation
        .shell_command
        .push_str(&super::shell_quote(event));
    Ok(invocation)
}

fn bundle_fingerprint(reporter: &Path) -> Result<String, IntegrationError> {
    let mut canonical = String::new();
    for event in CODEX_EVENTS {
        canonical.push_str(&base_platform_invocation(reporter, event)?.shell_command);
        canonical.push('\n');
    }
    Ok(format!("{:x}", Sha256::digest(canonical.as_bytes())))
}

pub(crate) fn current_hook_fingerprint(reporter: &Path) -> Result<String, IntegrationError> {
    bundle_fingerprint(reporter)
}

pub(crate) fn hook_fingerprint(reporter: &Path) -> Result<String, IntegrationError> {
    current_hook_fingerprint(reporter)
}

fn platform_invocation(
    reporter: &Path,
    event: &str,
) -> Result<ReporterInvocation, IntegrationError> {
    let mut invocation = base_platform_invocation(reporter, event)?;
    let fingerprint = current_hook_fingerprint(reporter)?;
    let fingerprint_flag = if ReporterPlatform::current() == ReporterPlatform::Posix {
        "--hook-fingerprint"
    } else {
        "-HookFingerprint"
    };
    invocation
        .args
        .extend([fingerprint_flag.into(), fingerprint.clone()]);
    invocation.shell_command.push(' ');
    invocation.shell_command.push_str(fingerprint_flag);
    invocation.shell_command.push(' ');
    invocation
        .shell_command
        .push_str(&super::shell_quote(&fingerprint));
    Ok(invocation)
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
        reports_attention: true,
        // Codex stays on the terminal-fallback `(printed_plan_path)`
        // because its `apply_patch` hook payload carries patch text rather
        // than a canonical file path — see ADR 0037 / ADR 0038.
        reports_plan_path: false,
    },
    probe,
    merge,
    remove,
    reconcile_current,
);

fn groups(document: &Map<String, Value>) -> Result<Vec<(String, Value)>, IntegrationError> {
    let Some(hooks) = document.get("hooks") else {
        return Ok(vec![]);
    };
    let hooks = hooks
        .as_object()
        .ok_or_else(|| IntegrationError::InvalidConfig("Codex hooks must be an object.".into()))?;
    let mut groups = Vec::new();
    for event in CODEX_EVENTS {
        let Some(value) = hooks.get(event) else {
            continue;
        };
        let event_groups = value.as_array().cloned().ok_or_else(|| {
            IntegrationError::InvalidConfig(format!("Codex {event} hooks must be an array."))
        })?;
        groups.extend(
            event_groups
                .into_iter()
                .map(|group| (event.to_owned(), group)),
        );
    }
    Ok(groups)
}

fn marker_state(
    _event: &str,
    group: &Value,
    expected: Option<&ReporterInvocation>,
) -> FragmentState {
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
            // matches every source for this event. A group edited to add one
            // no longer fires for the complete event contract, so it must
            // not read as Installed.
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
    let expected_available = reporter.try_exists().unwrap_or(false);
    let mut installed_events = 0;
    let mut saw_owned = false;
    let mut saw_drifted = false;
    let mut owned_events = std::collections::HashSet::new();
    for (event, group) in groups(document)? {
        let expected = expected_available
            .then(|| platform_invocation(reporter, &event))
            .transpose()?;
        let next = marker_state(&event, &group, expected.as_ref());
        match next {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
            FragmentState::Installed => {
                if !owned_events.insert(event) {
                    return Ok(FragmentState::Ambiguous);
                }
                saw_owned = true;
                installed_events += 1;
            }
            FragmentState::Drifted => {
                if !owned_events.insert(event) {
                    return Ok(FragmentState::Ambiguous);
                }
                saw_owned = true;
                saw_drifted = true;
            }
        }
    }
    if !saw_owned {
        Ok(FragmentState::Absent)
    } else if !saw_drifted && installed_events == CODEX_EVENTS.len() {
        Ok(FragmentState::Installed)
    } else {
        Ok(FragmentState::Drifted)
    }
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
    for event in CODEX_EVENTS {
        let event_hooks = hooks
            .entry(event)
            .or_insert_with(|| json!([]))
            .as_array_mut()
            .ok_or_else(|| {
                IntegrationError::InvalidConfig(format!("Codex {event} hooks must be an array."))
            })?;
        let invocation = platform_invocation(reporter, event)?;
        event_hooks.push(json!({
            "hooks":[{"type":"command","command":invocation.shell_command}]
        }));
    }
    Ok(())
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    let existing = groups(document)?;
    let mut owned_events = std::collections::HashSet::new();
    for (event, group) in &existing {
        match marker_state(event, group, None) {
            FragmentState::Absent => {}
            FragmentState::Ambiguous => return Ok(FragmentState::Ambiguous),
            FragmentState::Installed | FragmentState::Drifted => {
                // One OrkWorks group per event is the owned shape. Multiple
                // owned groups for the same event are ambiguous, while one
                // group on each of the four Codex events is the complete
                // bundle and must be removable as one unit.
                if !owned_events.insert(event) {
                    return Ok(FragmentState::Ambiguous);
                }
            }
        }
    }
    if owned_events.is_empty() {
        return Ok(FragmentState::Absent);
    }
    let hooks = document
        .get_mut("hooks")
        .and_then(Value::as_object_mut)
        .expect("validated hooks object");
    for event in CODEX_EVENTS {
        let Some(event_hooks) = hooks.get_mut(event).and_then(Value::as_array_mut) else {
            continue;
        };
        event_hooks.retain(|group| marker_state(event, group, None) == FragmentState::Absent);
    }
    Ok(FragmentState::Drifted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::integration::{
        IntegrationContext, IntegrationHandler, ReporterAssetResolver,
    };
    use crate::test_support::FakeHome;

    fn reporter_path(home: &std::path::Path) -> std::path::PathBuf {
        home.join(".orkworks/hook-scripts/report-harness-event.sh")
    }

    fn gitignored_workspace() -> tempfile::TempDir {
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        std::fs::write(workspace.path().join(".gitignore"), ".codex/hooks.json\n").unwrap();
        workspace
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
            marker_state("SessionStart", &group, Some(&invocation)),
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
    fn merge_writes_the_codex_four_event_bundle_without_dropping_foreign_hooks() {
        let mut document = Map::new();
        document.insert(
            "hooks".into(),
            json!({
                "Stop": [{
                    "hooks": [{"type": "command", "command": "user-stop"}]
                }]
            }),
        );
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());

        merge(&mut document, &reporter_path(home.path())).unwrap();

        let hooks = document["hooks"].as_object().unwrap();
        for event in [
            "SessionStart",
            "UserPromptSubmit",
            "PermissionRequest",
            "Stop",
        ] {
            let groups = hooks[event].as_array().unwrap();
            let command = groups
                .last()
                .and_then(|group| group["hooks"].as_array())
                .and_then(|entries| entries.first())
                .and_then(|entry| entry["command"].as_str())
                .unwrap();
            assert!(
                command.contains("--event"),
                "missing event in {event}: {command}"
            );
            assert!(
                command.contains("--hook-fingerprint"),
                "missing fingerprint in {event}: {command}"
            );
        }
        assert_eq!(hooks["Stop"].as_array().unwrap().len(), 2);
        assert_eq!(hooks["Stop"][0]["hooks"][0]["command"], "user-stop");
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
            base_platform_invocation(&reporter_path(home.path()), "SessionStart").unwrap();
        let group = json!({
            "matcher": "resume",
            "hooks": [
                {"type": "command", "command": invocation.shell_command}
            ]
        });

        assert_eq!(
            marker_state("SessionStart", &group, Some(&invocation)),
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
        assert!(
            command.contains("--hook-fingerprint '"),
            "Codex hook command must identify the exact definition: {command}"
        );
    }

    #[test]
    fn hook_fingerprint_changes_when_the_reporter_command_changes() {
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let first = hook_fingerprint(&reporter_path(home.path())).unwrap();
        let second = hook_fingerprint(&home.path().join(".orkworks/other.sh")).unwrap();
        assert_ne!(first, second);
        assert_eq!(first.len(), 64);
    }

    #[cfg(unix)]
    #[test]
    fn probe_reports_installed_after_merge_and_drifted_for_a_pre_portable_absolute_path_fragment() {
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
    fn probe_reports_ambiguous_when_one_event_has_two_owned_groups() {
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let script = reporter_path(home.path());
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, "#!/bin/sh\n").unwrap();
        let mut document = Map::new();
        merge(&mut document, &script).unwrap();

        let duplicate = document["hooks"]["SessionStart"][0].clone();
        document["hooks"]["SessionStart"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);

        assert_eq!(probe(&document, &script).unwrap(), FragmentState::Ambiguous);
    }

    #[test]
    fn probe_reports_ambiguous_when_duplicate_event_replaces_a_missing_event() {
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let script = reporter_path(home.path());
        std::fs::create_dir_all(script.parent().unwrap()).unwrap();
        std::fs::write(&script, "#!/bin/sh\n").unwrap();
        let mut document = Map::new();
        merge(&mut document, &script).unwrap();

        let hooks = document["hooks"].as_object_mut().unwrap();
        hooks.remove("Stop");
        let duplicate = hooks["SessionStart"][0].clone();
        hooks["SessionStart"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);

        assert_eq!(probe(&document, &script).unwrap(), FragmentState::Ambiguous);
    }

    #[test]
    fn install_reconciles_a_stale_reporter_even_when_the_hook_bundle_is_installed() {
        let workspace = gitignored_workspace();
        let source = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(home.path());
        let asset_name = ReporterPlatform::current().asset_name();
        let source_asset = source.path().join(asset_name);
        std::fs::write(&source_asset, "current reporter\n").unwrap();
        let resolver = ReporterAssetResolver {
            source_dir: source.path().to_path_buf(),
            stable_dir: home.path().join(".orkworks/hook-scripts"),
        };
        let ctx = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: home.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &resolver,
        };

        HANDLER.install(&ctx).unwrap();
        let stable_asset = resolver.stable_path(asset_name).unwrap();
        std::fs::write(&stable_asset, "stale reporter\n").unwrap();

        HANDLER.install(&ctx).unwrap();

        assert_eq!(
            std::fs::read_to_string(stable_asset).unwrap(),
            "current reporter\n"
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
