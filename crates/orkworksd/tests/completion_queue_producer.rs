#[cfg(unix)]
#[test]
fn ordered_completion_publisher_behavior() {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/scripts/codex-completion-report.test.py"
        ))
        .output()
        .expect("run completion publisher subprocess tests");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
