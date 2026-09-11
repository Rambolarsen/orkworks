//! Local subprocess fixtures only; environment changes live in a separate test process.
use super::*;
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

const CHILD: &str = "providers::custom_inference::transport_tests::custom_inference_child_fixture";
const MODEL: &str = "provider model;$(touch must-not-exist){promptFile}";
const RESPONSE: &str =
    r#"{"version":1,"status":"success","result":"{\"enrichments\":[],\"proposals\":[]}"}"#;

fn exercise(mode: &str, file: bool) {
    let root = tempfile::tempdir().unwrap();
    let executable = crate::test_support::native_inference::compile(root.path());
    let login = root.path().join("fake-existing-login");
    let codex_config = root.path().join("fake-codex-config");
    let claude_config = root.path().join("fake-claude-config");
    for directory in [&login, &codex_config, &claude_config] {
        fs::create_dir(directory).unwrap();
    }
    // No real auth environment/configuration reaches this fixture process.
    let started = Instant::now();
    let mut child = Command::new(std::env::current_exe().unwrap());
    child.current_dir(root.path());
    // Windows requires its system directory, not a provider credential/config.
    child.env_clear();
    #[cfg(not(windows))]
    let test_path = root.path().as_os_str().to_owned();
    #[cfg(windows)]
    let test_path = {
        let system_root = std::env::var_os("SYSTEMROOT").expect("Windows system root");
        child.env("SYSTEMROOT", &system_root);
        // The process runner uses taskkill for timeout cleanup on Windows.
        std::env::join_paths([
            root.path().to_path_buf(),
            PathBuf::from(system_root).join("System32"),
        ])
        .unwrap()
    };
    let output = child
        .args(["--exact", CHILD, "--nocapture"])
        .env("PATH", &test_path)
        .env("TEMP", root.path())
        .env("TMP", root.path())
        .env("TMPDIR", root.path())
        .env("HOME", &login)
        .env("USERPROFILE", &login)
        .env("CODEX_HOME", &codex_config)
        .env("CLAUDE_CONFIG_DIR", &claude_config)
        .env("CUSTOM_PROVIDER_TOKEN", "fake-login-sentinel")
        .env("ORKWORKS_REPORT_TOKEN", "fake-report-sentinel")
        .env("ORKWORKS_ACTION_TOKEN", "fake-action-sentinel")
        .env("ORKWORKS_FUTURE_CAPABILITY", "fake-future-sentinel")
        .env("BASH_ENV", root.path().join("must-not-load"))
        .env("ENV", root.path().join("must-not-load"))
        .env("CUSTOM_TEST_ROOT", root.path())
        .env("CUSTOM_TEST_EXECUTABLE", &executable)
        .env("CUSTOM_TEST_MODE", mode)
        .env("CUSTOM_TEST_FILE", if file { "yes" } else { "no" })
        .env("CUSTOM_TEST_RESPONSE", RESPONSE)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "mode={mode}: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "fixture did not finish within cleanup allowance"
    );
    // Timeout may happen before the child can record anything. The helper
    // asserts timeout classification and ownership cleanup directly below.
    if mode == "timeout" {
        return;
    }
    let record = fs::read_to_string(root.path().join("record"))
        .unwrap_or_else(|error| panic!("mode={mode}: missing fixture record: {error}"));
    let fields: Vec<_> = record.lines().collect();
    assert_eq!(fields.len(), 12);
    assert!(fields[0].contains("orkworks-custom-inference-"));
    assert!(
        !Path::new(fields[0]).exists(),
        "private directory survived dispatch"
    );
    assert_eq!(
        &fields[1..4],
        &[
            login.to_str().unwrap(),
            codex_config.to_str().unwrap(),
            claude_config.to_str().unwrap()
        ]
    );
    assert_eq!(fields[4], "fake-login-sentinel");
    assert_eq!(&fields[5..10], &["unset"; 5]);
    assert_eq!(fields[10], test_path.to_str().unwrap());
    assert_eq!(fields[11], "unset");
    let args = fs::read(root.path().join("args")).unwrap();
    let args: Vec<_> = args
        .split(|byte| *byte == 0)
        .filter(|arg| !arg.is_empty())
        .collect();
    assert_eq!(args[0], MODEL.as_bytes());
    if file {
        let input_path = Path::new(std::str::from_utf8(args[1]).unwrap());
        assert_eq!(
            input_path.parent().unwrap().file_name(),
            Path::new(fields[0]).file_name()
        );
        assert!(!input_path.exists());
        assert!(fs::read(root.path().join("stdin")).unwrap().is_empty());
    }
    assert_eq!(args.last().unwrap(), &b"--effort=high".as_slice());
    assert_eq!(
        fs::read_to_string(root.path().join("prompt")).unwrap(),
        "private fixture context"
    );
    assert!(!root.path().join("must-not-exist").exists());
}

#[test]
fn custom_inference_transports_stdin_and_file_with_literal_argv_and_existing_login() {
    exercise("success", false);
    exercise("success", true);
}

#[test]
fn custom_inference_cleans_up_failures_and_bounds_output_and_runtime() {
    for mode in [
        "nonzero",
        "malformed",
        "timeout",
        "stdout-overflow",
        "stderr-overflow",
    ] {
        exercise(mode, true);
    }
}

#[test]
fn custom_inference_rejects_invalid_utf8_instead_of_repairing_response_bytes() {
    exercise("invalid-utf8", false);
}

#[test]
fn custom_inference_does_not_classify_provider_stderr_as_runner_timeout() {
    exercise("fake-timeout", false);
}

#[test]
fn custom_inference_child_fixture() {
    if std::env::var_os("CUSTOM_TEST_ROOT").is_none() {
        return;
    }
    let file = std::env::var("CUSTOM_TEST_FILE").unwrap() == "yes";
    let mode = std::env::var("CUSTOM_TEST_MODE").unwrap();
    let executable = PathBuf::from(std::env::var_os("CUSTOM_TEST_EXECUTABLE").unwrap());
    let mut args = vec!["transport", "{model}"];
    if file {
        args.push("{promptFile}");
    }
    let capability = serde_json::from_value(json!({"kind":"command","command":executable,
        "args":args,"input":if file {"file"} else {"stdin"},"output":"result-json-v1",
        "timeoutSecs":if mode == "timeout" {1} else {10},"reasoningEffortArgs":["--effort={effort}"]})).unwrap();
    let prepared = prepare(
        &capability,
        &executable,
        MODEL,
        Some("high"),
        "private fixture context".into(),
    )
    .unwrap();
    // Only this isolated helper process mutates its environment, never the parent test runner.
    std::env::set_var("ORKWORKS_LATE_CAPABILITY", "late-fake-token");
    let directory = prepared.command.get_current_dir().unwrap().to_path_buf();
    let prompt_file = prepared
        ._prompt_file
        .as_ref()
        .map(|file| file.path().to_path_buf());
    let result = prepared.run();
    assert!(!directory.exists(), "private directory survived dispatch");
    assert!(
        prompt_file.is_none_or(|file| !file.exists()),
        "private input survived dispatch"
    );
    if mode == "success" {
        assert_eq!(result.unwrap(), r#"{"enrichments":[],"proposals":[]}"#);
    } else {
        let error = result.expect_err("invalid or failed process must not produce accepted text");
        assert!(!error.message.contains("provider-secret"));
        let expected_code = match mode.as_str() {
            "timeout" => ProviderOperationErrorCode::Timeout,
            "malformed" => ProviderOperationErrorCode::Malformed,
            _ => ProviderOperationErrorCode::ProviderFailure,
        };
        assert_eq!(error.code, expected_code);
    }
}
