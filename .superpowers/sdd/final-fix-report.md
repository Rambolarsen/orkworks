> **Superseded:** This intermediate report covered a discarded `/reset` and
> `minVersion` approach. The final scope is documented in
> `final-scope-fix-report.md`.

# Copilot Label Reset Commands — Intermediate Fix Report

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

## Final review regression coverage

### Change

- Added `codex_status_preserves_needs_trust_for_an_installed_compatible_tool`
  in `crates/orkworksd/src/http/integration_handlers.rs`.
- The test installs the Codex hook, provides a fake `codex` executable that
  reports the compatible minimum version `0.114.0`, then checks the serialized
  status response. It asserts `activation == "needs_trust"` and confirms no
  `unsupported_tool_version` diagnostic is present.
- No production, renderer, or voice code changed.

### TDD evidence

The preserved behavior already existed, so the new test initially passed.
To prove it protects the intended behavior, the shared handler's installed
activation branch was temporarily replaced with `IntegrationActivation::Unknown`.
The new test then failed at its activation assertion (`left: "unknown"`,
`right: "needs_trust"`). That temporary change was immediately restored before
the final verification commands below.

### Verification

| Command | Result |
| --- | --- |
| `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml codex_status_preserves_needs_trust_for_an_installed_compatible_tool` | PASS — 1 passed, 816 filtered out |
| Same command with the temporary broken installed-activation branch | Expected RED — 1 failed: `unknown` instead of `needs_trust` |
| `rtk cargo test --manifest-path crates/orkworksd/Cargo.toml integration_handlers::tests` | PASS — 23 passed, 794 filtered out |
| `rtk git diff --check` | PASS |

The first attempted exact test filter selected zero tests; it was corrected and
is not counted as verification evidence.
