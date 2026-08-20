# Copilot Label Reset Commands — Final Fix Report

## Result

- Added Copilot's existing-schema `minVersion` floor of `1.0.33`.
- Extended the existing min-version regression test to preserve Codex's
  `0.114.0` assertion and assert Copilot's `1.0.33` floor.
- Updated the two design documents and ADR 0040 to distinguish the historical
  undeclared Copilot state from the current declaration, while retaining the
  authoritative Copilot CLI command-reference link and bare-command semantics.
- Updated the implementation plan's global constraint and focused-verification
  text for the Copilot version floor.

## TDD evidence

1. Added Copilot's `min_version` assertion before changing the JSON.
2. Ran:

   ```bash
   rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::min_version_round_trips_through_serde_and_patch_and_the_codex_and_copilot_builtins_have_it_set
   ```

   Result: expected RED failure — Copilot resolved `min_version` as `None`,
   while the assertion expected `Some(VersionRequirement { min: (1, 0, 33) })`.
3. Added `"minVersion": { "min": [1, 0, 33] }` to the Copilot builtin.
4. Re-ran the focused definition test: PASS, 1 passed and 644 filtered out.

## Verification

| Command | Result |
| --- | --- |
| `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::definition::tests::min_version_round_trips_through_serde_and_patch_and_the_codex_and_copilot_builtins_have_it_set` | PASS — 1 passed, 644 filtered out |
| `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml runtime::terminal_runtime::tests::each_declared_command_resets_its_own_harness` | PASS — 1 passed, 644 filtered out |
| `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml` | PASS — 645 passed |
| `rtk cargo build --manifest-path crates/orkworksd/Cargo.toml` | PASS — 0 errors; pre-existing dead-code warning for `merge_agent_attention_signal` and `append_terminal_output_lines` |
| `rtk git diff --check` | PASS |
| `rtk bash .claude/hooks/doc-check.sh` | Exit 0; advisory to consider `README.md` because an ADR changed; not updated because this fix does not change architecture or milestones |
| `rtk bash .claude/hooks/worktree-check.sh` | PASS |

## Concern

`rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check` remains
red on broad pre-existing formatting drift across unrelated Rust files, including
unchanged label-reset code and other sidecar modules. This fix wave leaves those
files untouched as requested; the new assertion itself was formatted to the
current formatter output.
