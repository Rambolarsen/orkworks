---
type: Design
title: Selective Rust CodeQL scanning
description: Proposed Rust scan routing with unchanged security queries and a staged default-to-advanced cutover.
tags: [ci, codeql, security, performance]
status: draft
workflow_status: approved
---

# Selective Rust CodeQL scanning

Reduce repeated Rust analysis for unrelated changes while preserving the existing
five-language security coverage. This is the owner-approved security design for
[issue #1312](https://github.com/Rambolarsen/orkworks/issues/1312), approved in conversation on 2026-11-02 ("yes, go ahead"). Implementation and
qualified activation are authorized within the boundaries below.

## Evidence and decision

The repository uses managed default setup with the default query suite and
`build-mode: none`. Completed Rust jobs in runs
[41456163054](https://github.com/Rambolarsen/orkworks/actions/runs/41456163054),
[41454180950](https://github.com/Rambolarsen/orkworks/actions/runs/41454180950), and
[41452680457](https://github.com/Rambolarsen/orkworks/actions/runs/41452680457)
took 388, 318, and 439 seconds. Analysis consumed 368, 301, and 424 seconds;
Autobuild consumed one second each. The first run spent approximately 131 seconds
extracting, 34 finalizing, and 196 evaluating queries. Its log reported no Rust
overlay mode or supported dependency-cache configuration. A Cargo build cache
would not address the measured bottleneck.

Use an owned advanced-setup workflow to route Rust selectively and cancel
superseded PR scans. The other four language jobs continue on every eligible
PR and main push. Do not introduce a general language-routing framework.
GitHub documents [advanced trigger control](https://docs.github.com/en/code-security/reference/code-scanning/workflow-configuration-options)
and the [default-to-advanced transition](https://docs.github.com/en/code-security/how-tos/find-and-fix-code-vulnerabilities/configure-code-scanning/configuring-advanced-setup-for-code-scanning).

## Scan contract

- Retain `actions`, `c-cpp`, `javascript-typescript`, `python`, and `rust`, the
  default query suite, existing threat-model settings, standard Linux runners,
  no-build extraction, and the `/language:<language>` analysis categories.
  Do not exclude sources, tests, queries, or findings to improve timing.
- On same-repository PRs targeting main, run the four non-Rust languages always.
  Skip Rust only when the complete cumulative PR diff contains exclusively
  explicitly recognized unrelated UI or prose paths. Rust inputs take precedence:
  all crates, Rust/Cargo/toolchain/config inputs, bundled knowledge, shared
  fixtures and external embedded Rust inputs must run Rust. Unrecognized paths,
  an empty/partial file inventory, ambiguous renames or detection failures run
  all five languages, or visibly fail; never silently skip Rust.
- The narrow skip allowlist may cover desktop renderer source and Markdown prose
  in docs/specs/skills, excluding fixtures and any externally embedded input.
  Regression checks against actual Rust includes must prevent an allowed path
  from silently becoming a Rust input. Do not reuse the product CI filter as a
  complete security dependency inventory; all Rust crates must be covered.
- Main pushes use immutable before/after event SHAs and the same conservative
  Rust decision. Do not cancel main-push scans: a later UI-only push must not
  interrupt an earlier Rust change. Initial pushes scan all five languages.
- Weekly schedules and manual dispatch always scan all five languages. Every
  selected Rust scan analyzes the whole Rust source scope, not only changed files.
  Retain default setup's existing same-repository PR scope; no fork expansion,
  `pull_request_target`, user secrets, or privileged analysis of PR code.
- Grant only the token permissions needed for checkout, change detection and
  security-result upload. Preserve supported CodeQL action-managed caching for
  other languages; do not promise unsupported Rust cache savings.

## Qualification and activation

Implement initially with result upload disabled while default setup remains
active. Confirm this can run without disrupting managed scans; if GitHub blocks
staging, stop and revise the rollout rather than disabling default setup to test.
Record exact source inputs, CodeQL/action/query-pack versions, effective config,
phase timings, extracted-file metrics, diagnostics and SARIF for comparable
full runs. Compare both the complete five-language run and a repeated Rust run;
then demonstrate an unrelated renderer/prose change skips Rust, and a Rust or
external-input change scans it. Do not claim lower per-scan time without evidence.

A green baseline is not proof of complete extraction: run 41456163054 reported
one Rust file extracted with errors. Its extraction log also reports a
`$crate::panic::panic_2021` macro-expansion warning at
`crates/orkworksd/src/taskmaster/runtime.rs:2150:9`. Confirm the counted file and
retain the complete diagnostic during qualification. Block new errors, missing source or unexplained finding loss;
retain and disclose baseline limitations rather than hiding them.

After the owner-approved design is implemented and qualification passes, record
the current default-setup configuration and latest per-language analyses. Disable
managed default setup, enable upload in the already-validated workflow, and
immediately dispatch a full main scan. Verify all five existing category names
receive current results and inspect coverage/finding differences. Staging flags
must default to no upload; fork code cannot activate publication. This approved
cutover permits those specific repository-setting mutations, not other security
or reviewer changes. Activation remains incomplete until the full upload passes.

If staging fails, leave default setup active. If activation fails, restore the
recorded default setup configuration immediately, disable advanced publication,
and confirm a new managed full scan succeeds. Do not delete prior analyses or
alerts to make the migration appear clean. Record the failed attempt and exact
restoration results; do not keep both publishers active.

## Acceptance and boundaries

Success is fewer Rust jobs for proven unrelated changes, unchanged source/query
scope when Rust runs, a functioning weekly full scan, current analysis categories,
and a tested recovery procedure. Report observed skipped-job counts and runtime;
forecast neither billing nor a percentage saving from the supplied usage window.
Product CI, manual review policy, AI security-findings quotas, paid runners and
existing Rust extraction defects are outside this change. The
[implementation plan](../plans/2026-11-02-codeql-scan-routing.md) owns delivery checks.
