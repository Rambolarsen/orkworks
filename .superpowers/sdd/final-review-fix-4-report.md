# Final review fix 4

## Change

Updated `crates/orkworksd/src/providers.rs` only. The Ollama test server now:

- sets a 2-second write timeout on the accepted `TcpStream` before writing its response;
- matches `Content-Length` case-insensitively after normalizing each header line;
- caps request size and uses checked arithmetic for header/body boundaries;
- preserves the bounded accept/read/EOF handling and the successful request-body/model assertions.

## Verification

- `rustfmt --edition 2021 --check crates/orkworksd/src/providers.rs` — passed.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests` — 32 passed; `providers::tests::ollama_request_uses_resolved_entry_model` failed because the sandbox blocked loopback accept with `Resource temporarily unavailable (os error 35)`.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests -- --skip ollama_request_uses_resolved_entry_model` — passed: 32 passed, 1 filtered out.
- `cargo test --manifest-path crates/orkworksd/Cargo.toml providers::tests::ollama_request_uses_resolved_entry_model -- --exact` — failed for the same sandbox loopback-accept error, even when run with escalation.
- `git diff --check` — passed.
- `bash .claude/hooks/doc-check.sh` — passed with no output.
- `bash .claude/hooks/worktree-check.sh` — passed with no output.
