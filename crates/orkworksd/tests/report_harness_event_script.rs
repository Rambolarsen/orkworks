// Exercises crates/orkworksd/scripts/report-harness-event.sh end to end: a
// fake `curl` on PATH captures the POST body it would have sent, so these
// tests pin the actual JSON payload the script produces without hitting a
// real sidecar.
use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

#[cfg(unix)]
fn make_executable(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(path, perms).unwrap();
}

#[cfg(unix)]
fn run_reporter(hook_fingerprint: &str, harness_session_id: &str) -> serde_json::Value {
    let script = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/scripts/report-harness-event.sh"
    );

    let dir = tempfile::tempdir().unwrap();
    let capture = dir.path().join("curl-capture.txt");
    let fake_curl = dir.path().join("curl");
    fs::write(
        &fake_curl,
        format!(
            "#!/bin/sh\nfor a in \"$@\"; do\n  if [ \"$prev\" = \"-d\" ]; then\n    printf '%s' \"$a\" >> {capture:?}\n  fi\n  prev=\"$a\"\ndone\n",
            capture = capture.display()
        ),
    )
    .unwrap();
    make_executable(&fake_curl);

    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap()
    );

    let mut child = Command::new("bash")
        .arg(script)
        .arg("--marker")
        .arg("orkworks:harness-integration:v2:codex")
        .arg("--event")
        .arg("SessionStart")
        .arg("--hook-fingerprint")
        .arg(hook_fingerprint)
        .env("PATH", path)
        .env("ORKWORKS_SESSION_ID", "test-session")
        .env("ORKWORKS_PORT", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(
            format!(r#"{{"session_id":"{harness_session_id}","source":"startup"}}"#).as_bytes(),
        )
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "reporter script exited non-zero");

    let captured = fs::read_to_string(&capture).unwrap_or_else(|_| {
        panic!("expected {capture:?} to exist \u{2014} curl was never invoked with -d")
    });
    serde_json::from_str(&captured).unwrap_or_else(|e| {
        panic!("harness-session POST body was not valid JSON: {e}\nbody: {captured}")
    })
}

#[cfg(unix)]
#[test]
fn codex_hook_fingerprint_reaches_the_harness_session_payload_intact() {
    let payload = run_reporter("abc123fingerprint", "session-42");

    assert_eq!(payload["harnessSessionId"], "session-42");
    assert_eq!(payload["source"], "codex_hook");
    assert_eq!(payload["hookFingerprint"], "abc123fingerprint");
}

#[cfg(unix)]
#[test]
fn a_fingerprint_containing_quotes_is_escaped_into_valid_json() {
    let payload = run_reporter(r#"weird"fingerprint\value"#, "session-43");

    assert_eq!(payload["harnessSessionId"], "session-43");
    assert_eq!(payload["hookFingerprint"], r#"weird"fingerprint\value"#);
}
