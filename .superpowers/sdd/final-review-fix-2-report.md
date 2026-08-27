# Final Review Fix 2

## Change

Updated `crates/orkworksd/src/providers.rs` so the Ollama request test returns an
error immediately when the server receives EOF (`Ok(0)`) before the declared
`Content-Length` bytes. Read errors are also returned. The existing two-second
accept deadline and successful request-body assertion are unchanged.

## Verification

- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check`: **FAIL** (exit 1). The workspace-wide check reports pre-existing formatting differences in unrelated files; `rustfmt --edition 2021 --check crates/orkworksd/src/providers.rs` passed.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests -- --skip ollama_request_uses_resolved_entry_model`: **PASS** — 32 passed, 795 filtered out.
- Targeted Ollama test, `cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests::ollama_request_uses_resolved_entry_model -- --nocapture`: **BLOCKED by sandbox** before test execution; Cargo could not open `crates/orkworksd/target/debug/.cargo-build-lock` (`Operation not permitted`).
- `git diff --check`: **PASS**.
- `bash .claude/hooks/doc-check.sh`: **PASS** (exit 0, no output).
- `bash .claude/hooks/worktree-check.sh`: **PASS** (exit 0, no output).
