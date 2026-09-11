# Taskmaster Windows fixture preparation

> Use executing-plans; root is the sole writer.

**Goal:** Make custom inference transport and activation fixtures native and run
them in a focused Windows CI job, without publishing or dispatching CI here.

**Spec:** [custom adapters design](../specs/2026-09-10-custom-inference-adapters-design.md),
issue #503. Windows verification remains required before release.

**Architecture:** Compile one std-only fixture with the installed `rustc` into
each fixture's owned temporary directory. Reuse the source from transport and
activation tests on Unix and Windows; do not ship it or add a production binary.
Keep native CLI profiles and production inference behavior unchanged.

- [x] Replace the activation shell fixture with a native compiler helper;
  observe the missing helper, implement native activation behavior, and run the
  six existing activation tests without a Unix-only gate.
- [x] Port transport fixtures to the same native executable, preserving literal
  argv, stdin/file, fake-login environment, bounded-output/error/timeout and
  cleanup assertions. Run all existing custom transport tests locally.
- [x] Add a Windows PR job for `custom_inference`, `activation_tests` and
  `inference_trust`, routing changes to Rust or the workflow itself. Preserve
  existing required job names and do not dispatch the job.
- [x] Two read-only reviewers, one round: fixture fidelity versus CI/Windows
  gaps. Root patches verified findings; no other writers or external reviews.
- [x] Run focused tests and full local verification; document Windows execution
  as pending, not passed. No commits, publication, runner installs or real CLIs.

## Execution notes

The activation tests first failed on the missing native compiler helper, then
passed with native activation behavior. Transport tests failed while the native
executable lacked its transport protocol, then all ten custom-inference tests
passed after conversion. A missing PathBuf test import was corrected before
the transport behavior failure was observed.

Each fixture compiles into its own TempDir, rather than a static process cache:
this keeps compiler artifacts under RAII cleanup instead of leaking a static
TempDir at process exit. The fixture is std-only and is not a Cargo binary target
or packaged resource. Compilation remains outside provider timeout measurements.

The Windows test environment retains SYSTEMROOT and System32 in PATH for the
existing taskkill-based runner cleanup; login/configuration values remain fake
sentinels. Unix tests use only their fixture directory in PATH, proving no shell
utilities are needed by the native transport. Guarded-spawn lock stress tests
remain Unix-only and are not claimed as portable coverage.

Workflow YAML parsed successfully with Ruby YAML. actionlint is unavailable
locally. No Windows job was dispatched, and no Windows runtime result is claimed.

## Verification and review checkpoint

Full verification initially exposed inherited-CWD failures from fixture compilation.
Investigation found an existing plan_handoff test changes process-wide CWD and
then drops that directory. Compiler and isolated transport child now set their
own fixture-owned current directories; the unrelated test was not changed.
The initial run also hit the previously observed one-second closed-stdin process
fixture timeout. No production timeout was changed.

After the CWD fix, full `RUST_TEST_THREADS=4 bash scripts/verify-repo.sh` passed,
including Rust tests/build/fmt, all 695 desktop tests, typecheck, both builds,
and diff/documentation/worktree checks. Existing warnings remain.

The std-only fixture passed `rustc --target x86_64-pc-windows-gnu --emit=metadata`.
This proves target type-checking only, not Windows linking or execution. Full
sidecar cross-compilation is blocked locally by the missing MinGW C compiler.

Fidelity review found no concrete defect. Windows/CI review found zero-match
filter risk and missing toolchain routing. The job now requires eleven named
transport/activation/trust tests to be discoverable before running the suites,
and `rust-toolchain.toml` routes to Rust and Windows checks. All eleven names were
validated against actual local Cargo discovery; PowerShell execution still awaits
Windows. Built-in Codex/Claude CLI fixtures and guarded-spawn stress fixtures
remain Unix-only, explicitly documented as outside this custom fixture slice.

No additional review round, dependency installation, commit, push, publication,
workflow dispatch or real provider invocation occurred. Native Windows validation
remains the release gate; these changes only prepare that run.
