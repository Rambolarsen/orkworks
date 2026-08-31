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
        .write_all(format!(r#"{{"session_id":"{harness_session_id}"}}"#).as_bytes())
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

// Regresses a defect where the codex hookFingerprint field was merged into
// the already-built harness-session payload via a second python3 call with
// no fallback: if that specific call failed (missing/broken python3), the
// command substitution captured empty stdout and unconditionally overwrote
// session_payload, losing the harnessSessionId/source fields too, not just
// the fingerprint. A fake `python3` that only fails for that one merge
// invocation (leaving codex's own session_id extraction, which is a
// separate, pre-existing python3 call, working normally) proves the
// harness-session payload survives regardless.
#[cfg(unix)]
#[test]
fn the_harness_session_payload_survives_a_failing_fingerprint_merge_helper() {
    let real_python3 = String::from_utf8(
        Command::new("sh")
            .arg("-c")
            .arg("command -v python3")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap()
    .trim()
    .to_string();
    assert!(!real_python3.is_empty(), "test requires python3 on PATH");

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

    // Fails only the fingerprint-merge invocation (identified by the
    // distinctive "hookFingerprint" substring in its -c script); every other
    // python3 call — including codex's own session_id extraction a few
    // lines earlier in the same script — falls through to the real binary.
    let fake_python3 = dir.path().join("python3");
    fs::write(
        &fake_python3,
        format!(
            "#!/bin/sh\ncase \"$2\" in\n  *hookFingerprint*) exit 1 ;;\nesac\nexec {real_python3:?} \"$@\"\n"
        ),
    )
    .unwrap();
    make_executable(&fake_python3);

    let path = format!(
        "{}:{}",
        dir.path().display(),
        std::env::var("PATH").unwrap()
    );

    let mut child = Command::new("bash")
        .arg(script)
        .arg("--marker")
        .arg("orkworks:harness-integration:v2:codex")
        .arg("--hook-fingerprint")
        .arg("abc123fingerprint")
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
        .write_all(br#"{"session_id":"session-44"}"#)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success(), "reporter script exited non-zero");

    let captured = fs::read_to_string(&capture).unwrap_or_else(|_| {
        panic!("expected {capture:?} to exist \u{2014} curl was never invoked with -d")
    });
    let payload: serde_json::Value = serde_json::from_str(&captured).unwrap_or_else(|e| {
        panic!("harness-session POST body was not valid JSON: {e}\nbody: {captured}")
    });

    assert_eq!(
        payload["harnessSessionId"], "session-44",
        "a failing fingerprint merge must not blank out fields that were already correctly populated"
    );
    assert_eq!(payload["source"], "codex_hook");
}
