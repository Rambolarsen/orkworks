# Final Scope Fix Report — Copilot Label Reset Commands

## Delivered scope

- Copilot's builtin `labelResetCommands` is exactly `["/clear", "/new"]`.
- The Copilot `minVersion` declaration and its assertion were removed. The
  existing min-version regression once again covers Codex only.
- The Copilot `/reset` runtime case was removed; `/clear` and `/new` runtime
  coverage remains.
- The current design, historical design, ADR 0040, and implementation plan say
  that Copilot currently declares `/clear` and `/new`, retain exact bare-command
  semantics and the authoritative command-reference link, and explicitly defer
  `/reset`: `minVersion` gates integration-status probing only and cannot
  protect terminal label-reset matching.

## TDD evidence

1. RED — after changing the definition assertion to the two-command scope:

   ```bash
   rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::embedded_builtins_are_complete_and_valid
   ```

   Result: failed as expected. The actual list was `["/clear", "/new",
   "/reset"]`; the assertion required `["/clear", "/new"]`.

2. GREEN — after removing Copilot's `minVersion` and `/reset` declaration:

   ```bash
   rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::embedded_builtins_are_complete_and_valid
   rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::min_version_round_trips_through_serde_and_patch_and_the_codex_builtin_has_it_set
   rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests::each_declared_command_resets_its_own_harness
   ```

   Result: each command passed — `1 passed, 644 filtered out`.

## Full verification

```bash
CARGO_TARGET_DIR=/private/tmp/orkworks-copilot-label-reset-commands-target rtk cargo test --manifest-path crates/orkworksd/Cargo.toml
```

Result: `645 passed (1 suite, 22.71s)` when rerun with elevated test
permission. The first sandboxed run compiled and ran 645 tests but had three
`Operation not permitted` failures at `src/main.rs:584`; the elevated rerun
eliminated those environment-only failures.

```bash
CARGO_TARGET_DIR=/private/tmp/orkworks-copilot-label-reset-commands-target rtk cargo build --manifest-path crates/orkworksd/Cargo.toml
```

Result: `0 errors, 1 warnings`; the warning is pre-existing dead code for
`merge_agent_attention_signal` and `append_terminal_output_lines` in
`src/metadata.rs`.

```bash
rtk git diff --check
```

Result: passed with no whitespace errors.

```bash
rtk bash .claude/hooks/doc-check.sh
rtk bash .claude/hooks/worktree-check.sh
```

Result: the worktree check completed with no output. The doc check suggested
considering `README.md` because ADR 0040 changed; no README update is needed
for this correction because it changes no architecture, milestone, or ADR
index entry, and the requested scope excludes unrelated edits.

## Verification caveat

```bash
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
```

Result: failed on pre-existing formatting drift across many unrelated sidecar
files. No formatting changes were applied. The requested correction does not
reformat unrelated code.

The default worktree Cargo target also rejected its pre-existing
`.cargo-build-lock` with `Operation not permitted`; the isolated temporary
target above was used for the successful full test and build verification.
