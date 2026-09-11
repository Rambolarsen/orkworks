//! Real subprocess coverage with fake CLI executables; never calls a model.
use super::*;
use crate::providers::{ProcessRunner, ProviderManager, ProviderSettingsPayload};
use std::{os::unix::fs::PermissionsExt, sync::Arc};

const CHILD: &str = "providers::inference::transport_tests::child_process_fixture";

#[test]
fn prepared_cli_preserves_login_environment_and_strips_report_capabilities() {
    for (id, executable, version, response) in [
        ("codex", "codex", "codex-cli 0.153.4", "{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"fixture response\"}}\n{\"type\":\"turn.completed\"}"),
        ("claude-code", "claude", "2.1.236 (Claude Code)", "{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"fixture response\"}"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let executable_path = directory.path().join(executable);
        fs::write(&executable_path, r#"#!/bin/sh
if [ "$1" = '--version' ]; then
  printf '%s' "$INFERENCE_FIXTURE_VERSION"
  exit 0
fi
# Characterize the documented managed-MCP startup conflict, not the vendor's
# entire policy engine: an adapter must not request replacing the managed set.
if [ "$INFERENCE_FIXTURE_PROVIDER" = 'claude-code' ]; then
  for arg in "$@"; do
    if [ "$arg" = '--strict-mcp-config' ] || [ "$arg" = '--mcp-config' ]; then
      printf '%s' 'cannot replace managed MCP configuration' >&2
      exit 31
    fi
  done
fi
printf '%s\n' "$PWD" "$HOME" "$CODEX_HOME" "$CLAUDE_CONFIG_DIR" "${ORKWORKS_REPORT_TOKEN-unset}" "${ORKWORKS_OTHER_CAPABILITY-unset}" > "$INFERENCE_FIXTURE_RECORD"
printf '%s\n' "$@" > "$INFERENCE_FIXTURE_ARGS"
/bin/cat > "$INFERENCE_FIXTURE_PROMPT"
printf '%s' "$INFERENCE_FIXTURE_RESPONSE"
"#).unwrap();
        fs::set_permissions(&executable_path, fs::Permissions::from_mode(0o755)).unwrap();
        let login = directory.path().join("existing-login");
        fs::create_dir(&login).unwrap();
        fs::write(login.join("config.toml"), "cli_auth_credentials_store='keyring'\n").unwrap();
        // A separate test process owns these environment mutations. The parent
        // test runner never changes HOME/PATH or any real authentication source.
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", CHILD, "--nocapture"])
            .env("INFERENCE_FIXTURE_PROVIDER", id)
            .env("INFERENCE_FIXTURE_VERSION", version)
            .env("INFERENCE_FIXTURE_RESPONSE", response)
            .env("INFERENCE_FIXTURE_RECORD", directory.path().join("record"))
            .env("INFERENCE_FIXTURE_ARGS", directory.path().join("args"))
            .env("INFERENCE_FIXTURE_PROMPT", directory.path().join("prompt"))
            .env("PATH", directory.path())
            .env("HOME", &login)
            .env("CODEX_HOME", &login)
            .env("CLAUDE_CONFIG_DIR", &login)
            .env("ORKWORKS_REPORT_TOKEN", "fixture-not-a-real-token")
            .env("ORKWORKS_OTHER_CAPABILITY", "fixture-not-a-real-token")
            .output().unwrap();
        assert!(output.status.success(), "{}\n{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let record = fs::read_to_string(directory.path().join("record")).unwrap();
        let fields: Vec<_> = record.lines().collect();
        assert_eq!(fields.len(), 6);
        assert!(fields[0].contains("orkworks-inference-"));
        assert!(!Path::new(fields[0]).exists(), "private cwd must be removed after dispatch");
        for path in &fields[1..4] { assert_eq!(*path, login.to_str().unwrap()); }
        assert_eq!(&fields[4..], &["unset", "unset"]);
        assert_eq!(fs::read_to_string(directory.path().join("prompt")).unwrap(), "bounded fixture context");
        let args = fs::read_to_string(directory.path().join("args")).unwrap();
        assert!(args.lines().any(|arg| arg == "--model=explicit-model"));
        if id == "codex" { assert!(args.lines().any(|arg| arg == "cli_auth_credentials_store=\"keyring\"")); }
    }
}

#[test]
fn child_process_fixture() {
    let Ok(id) = std::env::var("INFERENCE_FIXTURE_PROVIDER") else {
        return;
    };
    let mut manager = ProviderManager::for_tests(ProviderSettingsPayload::default(), vec![]);
    manager.runner = Arc::new(ProcessRunner);
    let profile = match id.as_str() {
        "codex" => crate::providers::native_inference::NativeProfile::Codex,
        "claude-code" => crate::providers::native_inference::NativeProfile::Claude,
        _ => panic!("unknown native fixture provider"),
    };
    assert_eq!(
        manager
            .invoke_native_taskmaster_prompt(
                profile,
                "explicit-model",
                None,
                None,
                "bounded fixture context".into()
            )
            .unwrap(),
        "fixture response"
    );
}
