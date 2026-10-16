// Runs the real source-owned helper through controlled HTTP fixtures.
#[cfg(unix)]
#[test]
fn api_only_metadata_producer_behavior() {
    let output = std::process::Command::new("python3")
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/scripts/session-metadata.test.py"
        ))
        .output()
        .expect("run Python metadata producer tests");
    assert!(
        output.status.success(),
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
