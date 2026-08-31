# Task 1 report

## Status

Complete. The domain-model and strict-validation slice is implemented and
committed as `feat: add harness compatibility profiles`, with a follow-up
review-fix commit for persistence-boundary validation and diagnostics.

## Changes

- Added the closed `CompatibilityProfile::Copilot` enum and the single
  `derive_compatibility_metadata` mapping to compiled Copilot bindings.
- Added the v3 `compatibilityProfiles` map to `HarnessUserDocument`; v2
  documents migrate with an empty profile map.
- Added strict JSON parsing with the 256 KiB limit, duplicate-key rejection,
  and trailing-input rejection.
- Added restricted custom-definition parsing with unknown-field, compiled
  binding, malformed placeholder, and lowercase kebab-case validation.
- Routed persisted user documents through the restricted custom-definition
  parser so raw JSON cannot smuggle integration/session-signal bindings into a
  custom definition.
- Added JSON paths to schema diagnostics and retained null-versus-omitted
  patch semantics.
- Updated the version gate and diagnostic construction required by the v3
  document and schema diagnostics.
- Added a sidecar-only profile assignment method, orphan-profile validation,
  and atomic custom-definition/profile cleanup on deletion.
- Added field-specific paths for nested schema type and required-field errors
  and a trailing-input parser regression test.
- Routed create and update HTTP bodies through the strict parser and the
  restricted custom-definition parser, including strict envelope validation.
- Tightened ID segments, capability-variant field combinations, and Peon
  model-template placeholder validation.

## Verification

```text
rtk cargo fmt --manifest-path crates/orkworksd/Cargo.toml -- --check
pass

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml harness::
cargo test: 179 passed, 740 filtered out (2 suites)

rtk cargo test --manifest-path crates/orkworksd/Cargo.toml http::harness_handlers::tests
3 passed

rtk git diff --check
pass
```

The full Rust suite was not run for this focused task.

## Concerns

- The existing runtime `HarnessDefinition` remains intentionally broad for
  built-ins and derived metadata; persisted custom JSON uses the restricted
  parser and the registry still validates custom authority fields. The
  sidecar-owned profile map is loaded only through the restricted document
  parser and can be mutated through the sidecar method.
- Registry projection and provider behavior remain for Task 2.
