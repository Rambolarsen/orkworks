//! Native Windows transport fixtures, not live Claude/version qualification.
#![cfg(windows)]
use serde_json::{json, Value};
use std::{
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn claude_reporter_powershell_maps_only_valid_prompt_and_turn_payloads() {
    let dir = tempfile::tempdir().unwrap();
    let wrapper = dir.path().join("fixture.ps1");
    let cases_path = dir.path().join("cases.json");
    let capture = dir.path().join("requests.jsonl");
    let reporter =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/report-harness-event.ps1");
    let mut cases = Vec::new();
    let mut expected = Vec::new();
    for (event, kind, status) in [
        (
            "Notification",
            Some("permission_prompt"),
            "waiting_for_input",
        ),
        (
            "Notification",
            Some("elicitation_dialog"),
            "waiting_for_input",
        ),
        (
            "Notification",
            Some("elicitation_url_dialog"),
            "waiting_for_input",
        ),
        ("Notification", Some("idle_prompt"), "idle"),
        ("Notification", Some("elicitation_response"), "working"),
        ("Notification", Some("elicitation_complete"), "working"),
        ("UserPromptSubmit", None, "working"),
    ] {
        let mut payload = json!({"hook_event_name":event,"session_id":"native","prompt":"private-prompt","message":"private-question"});
        if let Some(kind) = kind {
            payload["notification_type"] = kind.into();
        }
        cases.push(json!({"event":event,"payload":payload,"fail":false}));
        expected.push(Some(status));
    }
    for (event, payload, fail) in [
        (
            "Notification",
            json!({"session_id":"native","notification_type":"permission_prompt"}),
            false,
        ),
        (
            "Notification",
            json!({"hook_event_name":"Stop","session_id":"native","notification_type":"permission_prompt"}),
            false,
        ),
        (
            "Notification",
            json!({"hook_event_name":"Notification","session_id":"native","notification_type":"agent_needs_input"}),
            false,
        ),
        (
            "Notification",
            json!({"hook_event_name":"Notification","session_id":"native","notification_type":42}),
            false,
        ),
        (
            "Notification",
            json!({"hook_event_name":"Notification","session_id":42,"notification_type":"permission_prompt"}),
            false,
        ),
        (
            "UserPromptSubmit",
            json!({"hook_event_name":"Stop","session_id":"native"}),
            false,
        ),
        (
            "UserPromptSubmit",
            json!({"hook_event_name":"UserPromptSubmit","session_id":"native"}),
            true,
        ),
    ] {
        cases.push(json!({"event":event,"payload":payload,"fail":fail}));
        expected.push(None);
    }
    // Aider completion has no prompt lifecycle; supplied status or question text
    // must not promote this unrelated marker to Claude/Copilot authority.
    for status in ["", "working", "waiting_for_input", "idle"] {
        for payload in [
            Value::Null,
            json!({"message":"Response complete"}),
            json!({"message":"private-question: Should I proceed?"}),
        ] {
            cases.push(json!({"marker":"orkworks:harness-integration:v2:aider",
                "event":"", "status":status, "payload":payload, "fail":false}));
            expected.push(None);
        }
    }
    std::fs::write(&cases_path, serde_json::to_vec(&cases).unwrap()).unwrap();
    std::fs::write(&wrapper, r#"
$ErrorActionPreference = 'Stop'
if (-not [string]::Equals($HOME, $env:ORKWORKS_FIXTURE_HOME, [StringComparison]::OrdinalIgnoreCase)) { throw 'fixture home not isolated' }
# The reporter runs in a child script scope. Keep stub state in this isolated
# PowerShell process so nested calls see the case index and rejection flag.
function Invoke-RestMethod {
    param($Method, $Uri, $Headers, $ContentType, $Body, $TimeoutSec)
    $record = @{ index=$global:orkworksFixtureCaseIndex; uri=$Uri; body=($Body | ConvertFrom-Json) }
    [IO.File]::AppendAllText($env:ORKWORKS_REQUEST_CAPTURE, (($record | ConvertTo-Json -Depth 10 -Compress) + "`n"))
    if ($global:orkworksFixtureFailRegistration -and $Uri.EndsWith('/harness-session')) { throw 'fixture registration denied' }
}
$cases = Get-Content -LiteralPath $env:ORKWORKS_CASES -Raw | ConvertFrom-Json
for ($i=0; $i -lt $cases.Count; $i++) {
    $global:orkworksFixtureCaseIndex=$i
    $global:orkworksFixtureFailRegistration=$cases[$i].fail
    $payload = if ($null -eq $cases[$i].payload) { "" } else { $cases[$i].payload | ConvertTo-Json -Depth 10 -Compress }
    [Console]::SetIn((New-Object IO.StringReader $payload))
    $marker = if ($cases[$i].marker) { $cases[$i].marker } else { 'orkworks:harness-integration:v2:claude-code' }
    & $env:ORKWORKS_REPORTER -Marker $marker -Event $cases[$i].event -Status $cases[$i].status
}
"#).unwrap();
    let mut child = Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy",
            "Bypass",
            "-File",
        ])
        .arg(&wrapper)
        .env_remove("PSModulePath")
        .env("USERPROFILE", dir.path())
        .env("ORKWORKS_FIXTURE_HOME", dir.path())
        .env("ORKWORKS_CASES", &cases_path)
        .env("ORKWORKS_REPORTER", &reporter)
        .env("ORKWORKS_REQUEST_CAPTURE", &capture)
        .env("ORKWORKS_SESSION_ID", "fixture-session")
        .env("ORKWORKS_PORT", "1")
        .env("ORKWORKS_REPORT_TOKEN", "fixture-token")
        .env("ORKWORKS_PROMPT_HOOK_GENERATION", "fixture-generation")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            other => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("native PowerShell fixture did not finish: {other:?}");
            }
        }
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let captured = std::fs::read_to_string(&capture).unwrap();
    assert!(!captured.contains("private-prompt") && !captured.contains("private-question"));
    let rows: Vec<Value> = captured
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    for (index, status) in expected.into_iter().enumerate() {
        let reports: Vec<_> = rows
            .iter()
            .filter(|row| row["index"] == json!(index))
            .collect();
        let attention: Vec<_> = reports
            .iter()
            .filter(|row| row["body"].get("status").is_some())
            .collect();
        if let Some(status) = status {
            assert_eq!(reports.len(), 2, "case {index}: {reports:?}");
            assert!(reports[0]["uri"]
                .as_str()
                .unwrap()
                .ends_with("/harness-session"));
            assert_eq!(attention.len(), 1, "case {index}");
            assert_eq!(attention[0]["body"]["status"], status, "case {index}");
            assert_eq!(attention[0]["body"]["harnessSessionId"], "native");
            assert_eq!(
                attention[0]["body"]["promptHookGeneration"],
                "fixture-generation"
            );
        } else {
            assert!(attention.is_empty(), "case {index}: {attention:?}");
            if cases[index]["marker"] == "orkworks:harness-integration:v2:aider" {
                assert!(reports.is_empty(), "Aider case {index}: {reports:?}");
            }
        }
    }
}
