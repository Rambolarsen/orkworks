# Windows Installer Smoke-Test Design

- Status: proposed
- Issue: [#497](https://github.com/Rambolarsen/orkworks/issues/497)
- Deciders: OrkWorks maintainers
- Date: 2026-09-08

## Context

The alpha release pipeline already builds an unsigned Windows x64 NSIS
installer with electron-builder. The release verifier proves that the
installer file is non-empty and that the `win-unpacked` directory contains the
Electron application, Rust sidecar, and packaged harness reporter scripts.
It does not execute the installer or uninstaller, so a broken NSIS command,
installation-directory override, installed-file layout, or uninstall path can
still reach the draft release.

The existing release specification treats cross-platform testing as a
non-goal. This design narrows that statement: OrkWorks will not maintain a
Windows-version compatibility matrix or automate GUI interaction, but the
release artifact must pass one install/uninstall smoke test on the
`windows-latest` runner that built it.

## Decision

Add a testable Node.js Windows installer smoke-test module and run it as a
blocking step in the existing Windows release matrix job after artifact
verification and before artifact upload.

The smoke test will:

1. Require the `win32` and `x64` target.
2. Resolve the generated NSIS installer and a unique temporary installation
   directory.
3. Refuse to use a pre-existing installation directory rather than deleting
   an arbitrary path.
4. Invoke the installer silently with `/S` and a final `/D=<install-dir>`
   argument.
5. Verify the installed `OrkWorks.exe`, `resources/orkworksd.exe`, and every
   reporter script copied into `resources/scripts/`.
6. Invoke `Uninstall OrkWorks.exe` silently.
7. Wait for the installation directory to disappear, with a bounded timeout.

The smoke test will preserve the temporary installation directory when a
check fails, leaving CI evidence available until the ephemeral runner is
discarded. A successful run removes the directory through the uninstaller;
the smoke test will not perform a broad recursive cleanup of its own.

## Scope and non-goals

### In scope

- A reusable Node.js module under `apps/desktop/scripts/` with a CLI entry
  point.
- Unit tests for path derivation, command arguments, ordering, required
  installed files, bounded uninstall polling, and actionable failures.
- A `package.json` script for the smoke test.
- A blocking Windows-only step in `.github/workflows/release.yml`.
- Release documentation describing the install/uninstall contract.

### Out of scope

- Code signing, SmartScreen reputation, or notarization.
- Installer branding, custom icons, shortcuts, or install-scope redesign.
- Auto-update or update-server behavior.
- GUI automation or first-launch workspace/runtime testing.
- A Windows-version, architecture, or locale compatibility matrix.
- Changes to the Rust sidecar or Electron renderer runtime.

## Architecture

### Two release gates

The release job keeps two distinct checks:

```text
electron-builder output
        |
        +--> verify:release
        |      checks artifact and unpacked resources
        |
        +--> smoke:windows-installer (Windows x64 only)
               installs, checks installed resources, uninstalls
        |
        +--> upload top-level OrkWorks-* artifacts
```

`verify:release` remains responsible for the unpacked build layout. The new
smoke test is responsible for NSIS execution and the installed layout. The
two checks intentionally overlap on sidecar and reporter files because the
first catches packaging omissions before installation and the second catches
installer omissions or path mistakes.

### Smoke-test module

`apps/desktop/scripts/windowsInstallerSmokeTest.mjs` will expose a pure
expectation builder and a parameterized runner. The expectation will contain:

- `installerPath`: `release/OrkWorks-${version}-win-x64.exe`
- `installDir`: the caller-provided temporary installation directory
- `appPath`: `${installDir}/${productName}.exe`
- `uninstallerPath`: `${installDir}/Uninstall ${productName}.exe`
- `sidecarPath`: `${installDir}/resources/orkworksd.exe`
- `scriptsDir`: `${installDir}/resources/scripts`
- `scriptPaths`: the three reporter files already required by release
  verification

The runner will accept injected filesystem, process, clock, and output
dependencies where needed by unit tests. Its production defaults will use
Node's `fs`, `child_process.execFileSync`, and a monotonic deadline for the
post-uninstall wait.

The installer invocation will be equivalent to:

```text
<installerPath> /S /D=<installDir>
```

The `/D=` argument is intentionally last because that is the NSIS contract.
The uninstaller invocation will be equivalent to:

```text
<uninstallerPath> /S
```

The current product name comes from `apps/desktop/package.json`, so the
expected application and uninstaller names remain coupled to the package
metadata rather than duplicated as unrelated constants.

### Temporary-directory contract

CI will provide a unique path below the GitHub Actions runner temp directory.
Local invocations may use a unique path below the operating system temp
directory. The runner will fail before launching NSIS if the chosen path
already exists. The implementation will not resolve, delete, or recursively
clean a caller-selected existing directory.

## Failure behavior

- Non-Windows or non-x64 execution fails with an explicit unsupported-target
  error; it does not silently pass.
- A missing or empty installer fails before execution.
- A non-zero installer exit fails the step and includes the installer path.
- A missing installed executable, sidecar, scripts directory, or reporter file
  identifies the exact expected path.
- A non-zero uninstaller exit fails the step and includes the uninstaller
  path.
- If the installation directory remains after the bounded wait, the failure
  includes the directory and timeout so a CI reader can distinguish delayed
  cleanup from an uninstaller that did not run.
- When a post-install assertion fails, the temporary directory is retained for
  inspection on the runner. There is no best-effort deletion that could hide
  the evidence or target an unintended path.

## Testing strategy

`apps/desktop/tests/windowsInstallerSmokeTest.test.mjs` will test the module
without invoking an executable:

- Windows x64 expectation names the NSIS installer, installed executable,
  uninstaller, `.exe` sidecar, and all reporter scripts.
- Unsupported targets are rejected.
- The installer process receives `/S` followed by final `/D=<install-dir>`.
- The uninstaller receives `/S` only.
- Install verification checks every required path and reports the first exact
  missing path.
- The wait succeeds when the directory disappears and fails at the bounded
  deadline when it does not.
- A pre-existing install directory is rejected without invoking a process.
- A failing process propagates a contextual error.

The existing desktop test suite and Rust suite remain required baseline checks.
The actual NSIS install/uninstall execution is intentionally performed only by
the Windows release job because the generated executable cannot be run on the
development host's macOS environment.

## Repository changes

- Create `apps/desktop/scripts/windowsInstallerSmokeTest.mjs`.
- Create `apps/desktop/tests/windowsInstallerSmokeTest.test.mjs`.
- Modify `apps/desktop/package.json` to add the smoke-test command.
- Modify `.github/workflows/release.yml` to run the Windows smoke test before
  upload.
- Modify `specs/release-pipeline.md` to record the new release gate and its
  narrowed testing non-goal.
- Modify `README.md` to mention that Windows release artifacts are install /
  uninstall smoke-tested before upload.

No new runtime dependency, Electron IPC surface, sidecar route, or installer
configuration option is required.

## Alternatives considered

### Separate artifact-validation workflow

A second workflow could download the generated installer and validate it in a
fresh Windows job. This would isolate packaging from installation, but it
would add artifact handoff complexity and duplicate the existing release
matrix. Running the smoke test in the Windows build job validates the exact
artifact before it is uploaded and keeps the release graph small.

### Static-only verification

Extending the current path verifier without executing NSIS would be simpler,
but it would not exercise silent install arguments, install-directory
handling, uninstaller discovery, permissions, or removal behavior. It does not
close the reported gap.

### GUI launch automation

Launching the installed Electron application would test more runtime behavior,
but it introduces display-server and readiness concerns unrelated to installer
correctness. First-launch and GUI smoke coverage can be added as a separate
task if release requirements later demand it.
