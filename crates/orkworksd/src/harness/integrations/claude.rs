use std::path::Path;

use serde_json::{json, Map, Value};

use super::{
    event_reporter_invocation_for_platform, reconcile_current, FragmentState, JsonHookHandler,
    ReporterInvocation, ReporterPlatform, ToolHookContract,
};
use crate::harness::integration::{IntegrationActivation, IntegrationCoverage, IntegrationError};

const MARKER: &str = "orkworks:harness-integration:v2:claude-code";

pub(crate) static HANDLER: JsonHookHandler = JsonHookHandler::new(
    ToolHookContract {
        harness_id: "claude-code",
        additional_reporter_assets: &[],
        tool_name: "Claude Code",
        relative_path: ".claude/settings.local.json",
        ownership_marker: MARKER,
        coverage: IntegrationCoverage::Limited,
        reports_attention: true,
        activation: IntegrationActivation::Active,
        // ADR 0038: Claude's PostToolUse Write|Edit hook forwards the
        // written file path to /sessions/:id/plan-path, giving the sidecar
        // a deterministic association that the terminal-text fallback can
        // mislead. Only Claude's PostToolUse payload exposes a canonical
        // `tool_input.file_path`; other JsonHookHandlers stay false.
        reports_plan_path: true,
    },
    probe,
    merge,
    remove,
    reconcile_current,
);

/// Current turn/path hooks must be synchronous; notification/lifecycle and
/// legacy-entry ownership scans accept either async setting.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AsyncSpec {
    Any,
    SyncMandatory,
}

/// The owned events Claude's installer attaches. The matcher, reporter
/// flag set, and async constraint vary per-event; `EventProfile` keeps the
/// per-event variation in one place so `marker_state`/`merge`/`probe`/
/// `remove` never disagree about which event has which contract.
#[derive(Clone, Copy, PartialEq, Eq)]
enum EventProfile {
    SessionStart,
    Notification,
    UserPromptSubmit,
    // Retained only to identify and remove owned legacy registration.
    PreToolUse,
    PostToolUse,
}

impl EventProfile {
    fn event_name(self) -> &'static str {
        match self {
            Self::SessionStart => "SessionStart",
            Self::Notification => "Notification",
            Self::UserPromptSubmit => "UserPromptSubmit",
            Self::PreToolUse => "PreToolUse",
            Self::PostToolUse => "PostToolUse",
        }
    }

    fn matcher(self) -> &'static str {
        match self {
            // Lifecycle and notification events match every occurrence.
            Self::SessionStart | Self::Notification | Self::UserPromptSubmit | Self::PreToolUse => {
                "*"
            }
            // PostToolUse goes through every Write/Edit; restricting the
            // matcher to `Write|Edit` keeps Bash/TodoWrite/NotebookEdit from
            // triggering the path-only route on churn that cannot carry a
            // plan/spec file path anyway.
            Self::PostToolUse => "Write|Edit",
        }
    }

    fn invocation(self, platform: ReporterPlatform, reporter: &Path) -> ReporterInvocation {
        match self {
            Self::PostToolUse => plan_path_invocation_for_platform(platform, reporter),
            _ => event_reporter_invocation_for_platform(
                platform,
                reporter,
                MARKER,
                self.event_name(),
            ),
        }
    }

    fn async_spec(self) -> AsyncSpec {
        match self {
            Self::SessionStart | Self::Notification => AsyncSpec::Any,
            Self::PreToolUse => AsyncSpec::Any,
            Self::PostToolUse | Self::UserPromptSubmit => AsyncSpec::SyncMandatory,
        }
    }
}

const LEGACY_EVENTS: [EventProfile; 1] = [EventProfile::PreToolUse];

const EVENTS: [EventProfile; 4] = [
    EventProfile::SessionStart,
    EventProfile::Notification,
    EventProfile::UserPromptSubmit,
    EventProfile::PostToolUse,
];

/// Builds the reporter invocation shape for the path-only PostToolUse hook:
/// `--marker <MARKER> --report-plan-path` (POSIX) / `-Marker <MARKER>
/// -ReportPlanPath` (PowerShell). The shared reporter script then forwards
/// the hook payload's `tool_input.file_path` to `/sessions/:id/plan-path`
/// and skips both the generic attention POST and the harness-session POST.
fn plan_path_invocation_for_platform(
    platform: ReporterPlatform,
    reporter: &Path,
) -> ReporterInvocation {
    let mut invocation = event_reporter_invocation_for_platform(
        platform,
        reporter,
        MARKER,
        EventProfile::PostToolUse.event_name(),
    );
    let flag = match platform {
        ReporterPlatform::Posix => "--report-plan-path",
        ReporterPlatform::WindowsPowerShell => "-ReportPlanPath",
    };
    invocation.args.push(flag.into());
    invocation
}

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

fn marker_state(
    group: &Value,
    expected: Option<&ReporterInvocation>,
    expected_matcher: Option<&str>,
    async_spec: AsyncSpec,
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
            let exact = expected.is_some_and(|invocation| {
                let async_ok = match async_spec {
                    AsyncSpec::Any => true,
                    // PostToolUse is intentionally synchronous: `merge`
                    // never writes the `async` field for it, and a hook
                    // edited to set `async: true` would just delay the
                    // path report — Drifted so install can converge it back.
                    AsyncSpec::SyncMandatory => {
                        !hook.get("async").and_then(Value::as_bool).unwrap_or(false)
                    }
                };
                // A hand-edited matcher must not silently claim Installed.
                // This matters most for PostToolUse: widening its matcher to
                // `*` or `Read|Write|Edit` would make Read events feed
                // `tool_input.file_path` to /plan-path, falsely associating
                // the session with documents the agent merely read (ADR 0038).
                let matcher_ok = expected_matcher.map_or(true, |matcher| {
                    group.get("matcher").and_then(Value::as_str) == Some(matcher)
                });
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
                    && async_ok
                    && matcher_ok
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
    profile: EventProfile,
    expected: Option<&ReporterInvocation>,
    async_spec: AsyncSpec,
) -> Result<FragmentState, IntegrationError> {
    // Verify the matcher too when probing (expected is Some). On remove the
    // caller passes None and uses AsyncSpec::Any to flag any drift marker
    // for cleanup without requiring byte-exact match.
    let expected_matcher = expected.is_some().then_some(profile.matcher());
    let mut state = FragmentState::Absent;
    for group in groups(document, profile.event_name())? {
        let next = marker_state(&group, expected, expected_matcher, async_spec);
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
    let platform = ReporterPlatform::current();
    let mut installed_count = 0;
    let mut absent_count = 0;
    let mut any_ambiguous = false;
    for profile in EVENTS {
        let invocation = profile.invocation(platform, reporter);
        let next = event_state(document, profile, Some(&invocation), profile.async_spec())?;
        match next {
            FragmentState::Ambiguous => any_ambiguous = true,
            FragmentState::Installed => installed_count += 1,
            FragmentState::Absent => absent_count += 1,
            FragmentState::Drifted => {}
        }
    }
    let mut legacy_present = false;
    for profile in LEGACY_EVENTS {
        let state = event_state(document, profile, None, AsyncSpec::Any)?;
        any_ambiguous |= state == FragmentState::Ambiguous;
        legacy_present |= state != FragmentState::Absent;
    }
    if any_ambiguous {
        return Ok(FragmentState::Ambiguous);
    }
    if legacy_present {
        return Ok(FragmentState::Drifted);
    }
    if installed_count == EVENTS.len() {
        return Ok(FragmentState::Installed);
    }
    if absent_count == EVENTS.len() {
        return Ok(FragmentState::Absent);
    }
    // At least one event is Installed or Drifted, and at least one is missing
    // or differently shaped — install must converge, so report Drifted.
    Ok(FragmentState::Drifted)
}

pub(super) fn prompt_attention_probe(
    document: &Map<String, Value>,
    reporter: &Path,
) -> Result<FragmentState, IntegrationError> {
    let invocation = EventProfile::Notification.invocation(ReporterPlatform::current(), reporter);
    event_state(
        document,
        EventProfile::Notification,
        Some(&invocation),
        EventProfile::Notification.async_spec(),
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
    let platform = ReporterPlatform::current();
    for profile in EVENTS {
        let invocation = profile.invocation(platform, reporter);
        let entry = hooks
            .entry(profile.event_name())
            .or_insert_with(|| json!([]));
        let entries = entry.as_array_mut().ok_or_else(|| {
            IntegrationError::InvalidConfig(format!(
                "Claude {} hooks must be an array.",
                profile.event_name()
            ))
        })?;
        let hook = json!({
            "type": "command",
            "command": invocation.program,
            "args": invocation.args,
        });
        let matcher = profile.matcher();
        entries.push(json!({"matcher": matcher, "hooks": [hook]}));
    }
    Ok(())
}

fn remove(document: &mut Map<String, Value>) -> Result<FragmentState, IntegrationError> {
    let mut found = false;
    for profile in EVENTS.into_iter().chain(LEGACY_EVENTS) {
        let state = event_state(document, profile, None, AsyncSpec::Any)?;
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
    for profile in EVENTS.into_iter().chain(LEGACY_EVENTS) {
        if let Some(groups) = hooks
            .get_mut(profile.event_name())
            .and_then(Value::as_array_mut)
        {
            groups.retain(|group| {
                marker_state(group, None, None, AsyncSpec::Any) == FragmentState::Absent
            });
        }
    }
    Ok(FragmentState::Drifted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness::definition::IntegrationBinding;
    use crate::harness::integration::{
        IntegrationContext, IntegrationRegistration, ReporterAssetResolver,
    };
    use crate::harness::integrations::handler;
    use crate::test_support::FakeHome;

    #[test]
    fn merge_installs_a_working_pre_tool_hook() {
        let reporter = Path::new(if cfg!(windows) {
            r"C:\report-harness-event.ps1"
        } else {
            "/tmp/report-harness-event.sh"
        });
        let mut document = Map::new();

        merge(&mut document, reporter).unwrap();

        let hooks = document["hooks"].as_object().unwrap();
        let session_start = hooks["SessionStart"].as_array().unwrap();
        assert_eq!(session_start.len(), 1);
        let expected = EventProfile::SessionStart.invocation(ReporterPlatform::current(), reporter);
        assert_eq!(session_start[0]["hooks"][0]["args"], json!(expected.args));
        assert_eq!(hooks["Notification"].as_array().unwrap().len(), 1);
        let expected = EventProfile::Notification.invocation(ReporterPlatform::current(), reporter);
        assert_eq!(
            hooks["Notification"][0]["hooks"][0]["args"],
            json!(expected.args)
        );
        let submitted = hooks["UserPromptSubmit"].as_array().unwrap();
        assert_eq!(submitted.len(), 1);
        assert!(submitted[0]["hooks"][0].get("async").is_none());
        let expected =
            EventProfile::UserPromptSubmit.invocation(ReporterPlatform::current(), reporter);
        assert_eq!(submitted[0]["hooks"][0]["args"], json!(expected.args));
    }

    #[test]
    fn merge_installs_submission_without_pre_tool_attention() {
        let reporter = Path::new("/fixture/reporter");
        let mut document = Map::new();
        merge(&mut document, reporter).unwrap();
        assert_eq!(
            document["hooks"]["UserPromptSubmit"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(document["hooks"].get("PreToolUse").is_none());
        assert!(document["hooks"].get("Stop").is_none());
        assert_ne!(
            document["hooks"]["UserPromptSubmit"][0]["hooks"][0]["async"],
            json!(true)
        );
    }

    #[test]
    fn merge_removes_only_owned_legacy_pre_tool_use() {
        let reporter = Path::new("/fixture/reporter");
        let foreign =
            json!({"matcher":"Bash", "hooks":[{"type":"command", "command":"foreign-command"}]});
        let mut document = json!({"hooks":{"PreToolUse":[
            {"matcher":"*", "hooks":[{"type":"command", "command":"old-reporter", "args":["--marker",MARKER,"--event","PreToolUse","--status","working"],"async":true}]},
            foreign.clone()
        ]}}).as_object().unwrap().clone();
        assert_eq!(probe(&document, reporter).unwrap(), FragmentState::Drifted);
        merge(&mut document, reporter).unwrap();
        assert_eq!(document["hooks"]["PreToolUse"], json!([foreign]));
        assert_eq!(
            probe(&document, reporter).unwrap(),
            FragmentState::Installed
        );
        remove(&mut document).unwrap();
        assert_eq!(document["hooks"]["PreToolUse"].as_array().unwrap().len(), 1);
        assert_eq!(probe(&document, reporter).unwrap(), FragmentState::Absent);
    }

    /// Merge must install a synchronous PostToolUse `Write|Edit` entry whose
    /// reporter passes `--report-plan-path` (ADR 0038). The matcher matters —
    /// PostToolUse is the only Claude event whose payload carries a written
    /// file path, and `Write|Edit` excludes noisy tool classes (TodoWrite,
    /// Bash, NotebookEdit, ...) from triggering the hook at all.
    #[test]
    fn merge_installs_a_post_tool_use_plan_path_hook() {
        let reporter = Path::new(if cfg!(windows) {
            r"C:\report-harness-event.ps1"
        } else {
            "/tmp/report-harness-event.sh"
        });
        let mut document = Map::new();

        merge(&mut document, reporter).unwrap();

        let post_tool = document["hooks"]["PostToolUse"].as_array().unwrap();
        assert_eq!(post_tool.len(), 1, "exactly one OrkWorks PostToolUse group");
        assert_eq!(
            post_tool[0]["matcher"], "Write|Edit",
            "matcher must restrict to Write|Edit"
        );
        let hook = &post_tool[0]["hooks"][0];
        let args = hook["args"].as_array().unwrap();
        assert!(
            hook.get("async").and_then(Value::as_bool).is_none(),
            "PostToolUse plan-path hook must be synchronous (no async: true); got: {hook}"
        );
        let expected = EventProfile::PostToolUse.invocation(ReporterPlatform::current(), reporter);
        assert_eq!(hook["command"], json!(expected.program));
        assert_eq!(
            args.iter()
                .map(Value::as_str)
                .collect::<Vec<Option<&str>>>(),
            expected
                .args
                .iter()
                .map(String::as_str)
                .map(Some)
                .collect::<Vec<_>>(),
            "PostToolUse hook must invoke the shared reporter in plan-path mode"
        );
    }

    /// probe() reports Installed only when all four owned events (SessionStart,
    /// Notification, UserPromptSubmit, PostToolUse) match the exact installed shape.
    /// A missing event is Drifted — install must converge it.
    #[test]
    fn probe_marks_a_missing_post_tool_use_hook_as_drifted() {
        let reporter = Path::new("/tmp/report-harness-event.sh");
        let mut document = Map::new();
        merge(&mut document, reporter).unwrap();
        let removed = document["hooks"]["PostToolUse"].as_array_mut().unwrap();
        removed.clear();

        assert_eq!(probe(&document, reporter).unwrap(), FragmentState::Drifted);
    }

    #[test]
    fn prompt_readiness_ignores_drift_in_unrelated_claude_events() {
        let reporter = Path::new("/tmp/report-harness-event.sh");
        let mut document = Map::new();
        merge(&mut document, reporter).unwrap();
        document["hooks"]["PostToolUse"] = json!([]);

        assert_eq!(probe(&document, reporter).unwrap(), FragmentState::Drifted);
        assert_eq!(
            prompt_attention_probe(&document, reporter).unwrap(),
            FragmentState::Installed
        );

        document["hooks"]["Notification"] = json!([]);
        assert_eq!(
            prompt_attention_probe(&document, reporter).unwrap(),
            FragmentState::Absent
        );
    }

    /// The conformance matrix checks probe/remove symmetry for every JSON
    /// handler. This specializes the same cycle for Claude's owned events:
    /// install, probe Installed, uninstall (remove + commit), probe Absent,
    /// re-install Installed — exercising the third event through the same
    /// round-trip that the cross-handler conformance test would otherwise
    /// not be sensitive to.
    #[test]
    fn post_tool_use_round_trips_through_probe_and_remove() {
        let workspace = tempfile::tempdir().unwrap();
        git2::Repository::init(workspace.path()).unwrap();
        std::fs::write(
            workspace.path().join(".gitignore"),
            ".claude/settings.local.json\n",
        )
        .unwrap();
        let target = workspace.path().join(".claude/settings.local.json");
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, "{}").unwrap();

        let assets = tempfile::tempdir().unwrap();
        std::fs::write(
            assets.path().join(ReporterPlatform::Posix.asset_name()),
            "#!/bin/sh\n",
        )
        .unwrap();
        std::fs::write(
            assets
                .path()
                .join(super::ReporterPlatform::WindowsPowerShell.asset_name()),
            "# noop\n",
        )
        .unwrap();
        let stable = tempfile::tempdir().unwrap();
        let _fake_home = FakeHome::set(stable.path());
        let reporter = ReporterAssetResolver {
            source_dir: assets.path().to_path_buf(),
            stable_dir: stable.path().join(".orkworks").join("hook-scripts"),
        };
        let ctx = IntegrationContext {
            workspace: workspace.path(),
            workspace_metadata: None,
            orkworks_root: stable.path(),
            enabled: true,
            detected_tool: None,
            reporter_assets: &reporter,
        };

        let installed = handler(&IntegrationBinding::Claude).install(&ctx).unwrap();
        assert_eq!(
            installed.registration,
            IntegrationRegistration::Installed,
            "first install"
        );
        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert_eq!(
            persisted["hooks"]["PostToolUse"][0]["matcher"], "Write|Edit",
            "PostToolUse entry must land on disk"
        );

        let removed = handler(&IntegrationBinding::Claude)
            .uninstall(&ctx)
            .unwrap();
        assert_eq!(
            removed.registration,
            IntegrationRegistration::Absent,
            "uninstall"
        );
        let persisted: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
        assert!(
            persisted["hooks"]
                .get("PostToolUse")
                .map_or(true, |v| v.as_array().map_or(true, Vec::is_empty)),
            "PostToolUse must be gone (or absent) after uninstall; got: {persisted}"
        );

        let reinstalled = handler(&IntegrationBinding::Claude).install(&ctx).unwrap();
        assert_eq!(
            reinstalled.registration,
            IntegrationRegistration::Installed,
            "re-install"
        );
    }

    #[test]
    fn submission_hook_uses_powershell_event_parameter() {
        let invocation = EventProfile::UserPromptSubmit.invocation(
            ReporterPlatform::WindowsPowerShell,
            Path::new("C:\\report-harness-event.ps1"),
        );
        assert!(invocation
            .args
            .windows(2)
            .any(|args| args == ["-Event", "UserPromptSubmit"]));
        assert!(!invocation.args.iter().any(|arg| arg == "-Status"));
    }

    #[test]
    fn plan_path_hook_uses_powershell_switch_syntax_on_windows() {
        let invocation = EventProfile::PostToolUse.invocation(
            ReporterPlatform::WindowsPowerShell,
            Path::new("C:\\report-harness-event.ps1"),
        );
        assert!(invocation.args.iter().any(|arg| arg == "-ReportPlanPath"));
    }

    #[test]
    fn probe_marks_an_async_submission_hook_as_drifted() {
        let reporter = Path::new("/tmp/report-harness-event.sh");
        let mut document = Map::new();
        merge(&mut document, reporter).unwrap();
        document["hooks"]["UserPromptSubmit"][0]["hooks"][0]["async"] = json!(true);

        assert_eq!(probe(&document, reporter).unwrap(), FragmentState::Drifted);
    }

    #[test]
    fn probe_marks_an_async_post_tool_use_hook_as_drifted() {
        // PostToolUse is intentionally synchronous — a hook edited to set
        // `async: true` would delay the path report; install must converge
        // it back, so probe reads it Drifted rather than Installed.
        let reporter = Path::new("/tmp/report-harness-event.sh");
        let mut document = Map::new();
        merge(&mut document, reporter).unwrap();
        document["hooks"]["PostToolUse"][0]["hooks"][0]["async"] = json!(true);

        assert_eq!(probe(&document, reporter).unwrap(), FragmentState::Drifted);
    }

    #[test]
    fn probe_marks_a_narrowed_or_widened_post_tool_use_matcher_as_drifted() {
        // A matcher drift is silent corruption of the install: a wider
        // matcher (e.g. `Read|Write|Edit`) would feed Read payloads'
        // `tool_input.file_path` to /plan-path, falsely associating the
        // session with documents the agent merely read (ADR 0038). Probe
        // must not report Installed for a narrowed matcher either; install
        // should converge either direction back to `Write|Edit`.
        let reporter = Path::new("/tmp/report-harness-event.sh");
        let mut widened = Map::new();
        merge(&mut widened, reporter).unwrap();
        widened["hooks"]["PostToolUse"][0]["matcher"] = json!("Read|Write|Edit");
        assert_eq!(
            probe(&widened, reporter).unwrap(),
            FragmentState::Drifted,
            "a widened matcher must read Drifted so install converges it"
        );

        let mut narrowed = Map::new();
        merge(&mut narrowed, reporter).unwrap();
        narrowed["hooks"]["PostToolUse"][0]["matcher"] = json!("Write");
        assert_eq!(
            probe(&narrowed, reporter).unwrap(),
            FragmentState::Drifted,
            "a narrowed matcher must read Drifted so install converges it"
        );
    }
}
