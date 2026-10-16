fn main() {
    println!("cargo:rerun-if-changed=src/runtime/codex_native/listener_identity.c");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos")
        && std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("aarch64")
    {
        cc::Build::new()
            .file("src/runtime/codex_native/listener_identity.c")
            .flag_if_supported("-std=c11")
            .warnings_into_errors(true)
            .compile("orkworks_listener_identity");
    }
}
