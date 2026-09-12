//! Test-only compiler helper. The caller's TempDir owns every build artifact.
pub(crate) fn compile(directory: &std::path::Path) -> std::path::PathBuf {
    let executable = directory.join(format!("fixture-provider{}", std::env::consts::EXE_SUFFIX));
    let output = std::process::Command::new("rustc")
        .current_dir(directory)
        .args(["--edition=2021", "--crate-name=orkworks_inference_fixture"])
        .arg(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/inference.rs"
        ))
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("Rust test toolchain must provide rustc");
    assert!(
        output.status.success(),
        "fixture compilation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable.canonicalize().unwrap()
}
