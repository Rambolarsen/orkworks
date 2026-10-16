#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
#[test]
fn sdk_adapter_rejects_partial_denied_wrong_state_and_malformed_socket_replies() {
    let temp = tempfile::tempdir().unwrap();
    let binary = temp.path().join("listener-sdk-test");
    let source =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/listener_identity_sdk.c");
    let output = std::process::Command::new("cc")
        .args(["-std=c11", "-Wall", "-Wextra", "-Werror"])
        .arg(source)
        .arg("-o")
        .arg(&binary)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(std::process::Command::new(binary)
        .status()
        .unwrap()
        .success());
}
