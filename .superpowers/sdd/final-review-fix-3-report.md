# Final review fix 3

## Change

Updated `crates/orkworksd/src/providers.rs` only. The Ollama test server now:

- returns explicit bounded errors for EOF and read errors while reading request headers;
- propagates missing or malformed header errors with actionable messages;
- propagates socket-timeout setup, invalid body UTF-8, and response-write errors;
- preserves the existing 2-second socket read timeout, request timeout behavior, response body, and model/body assertions.

## Verification

- `cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check` — failed because pre-existing formatting drift exists in unrelated Rust files, including `src/harness/definition.rs`, `src/harness/integrations/claude.rs`, `src/taskmaster/store.rs`, and others. No unrelated files were changed.
- `rustfmt --edition 2021 --check crates/orkworksd/src/providers.rs` — passed.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml` — compiled and ran 827 tests; 826 passed and `providers::tests::ollama_request_uses_resolved_entry_model` failed because the sandbox blocked loopback accept with `Resource temporarily unavailable (os error 35)`.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml -- --skip ollama_request_uses_resolved_entry_model` — passed: 826 passed, 1 filtered out.
- `cargo check --manifest-path crates/orkworksd/Cargo.toml` — passed with 0 errors and 4 pre-existing warnings.
- `git diff --check` — passed.
- `bash .claude/hooks/doc-check.sh` — passed with no output.
- `bash .claude/hooks/worktree-check.sh` — passed with no output.
