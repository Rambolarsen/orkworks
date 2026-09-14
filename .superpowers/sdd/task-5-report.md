# Task 5 report — signed release operations

## Files

The requested five documentation/spec files were changed:

- Created `docs/agents/release-signing.md` with the operator runbook,
  credential names, protected environment setup, rotation, base64 handling,
  publisher matching, and native verification commands.
- Updated `README.md` with the signed DMG/ZIP/NSIS artifact, metadata, and
  verification-gate status.
- Updated `docs/user/getting-started.md` to distinguish historical unsigned
  alpha installers from credential-backed releases.
- Updated `docs/agents/architecture.md` with CI signing/notarization,
  non-packaged secrets, and the Electron-main/#511 ownership boundary.
- Updated `specs/release-pipeline.md` with source-wiring status and the
  external credential/native-validation boundary.

Pre-existing changes to `.superpowers/sdd/task-3-report.md` and
`.superpowers/sdd/task-4-report.md` were preserved and not staged.

## Validation

- `git diff --check` — passed (exit 0).
- `git diff --cached --check` — passed (exit 0) for the five staged files.
- Staged-file audit — passed; exactly the five requested paths were staged.
- `bash scripts/verify-repo.sh` — blocked before repository checks. Bash
  returned `Access is denied` with `Bash/Service/CreateInstance/E_ACCESSDENIED`
  in this Windows environment.
- `bash scripts/doc-check.sh` — blocked before execution with the same Bash
  `E_ACCESSDENIED` startup error.
- `bash .claude/hooks/worktree-check.sh` — passed (exit 0, no output).
- `cargo --version` — available locally as Cargo 1.96.0, but the required
  verification wrapper did not reach its Rust steps because Bash could not
  start.
- Docs build — not run; `docs/node_modules` is absent, and the required Bash
  verification wrapper could not start.

No secret values were added to files, logs, or this report.

## Commit

- Commit: `7f562dc`
- Message: `docs: document signed release operations`

## Remaining external prerequisites

- Active Apple Developer membership, Developer ID Application `.p12`, App
  Store Connect `.p8`, and the required Apple IDs/team value.
- A trusted Authenticode `.pfx`/`.p12` or approved managed-signing service and
  the exact Windows certificate `SimpleName` for `WIN_EXPECTED_PUBLISHER`.
- Protected GitHub Environment `release`, required reviewers, and protected
  `v*` tag rules.
- A real credential-backed macOS and Windows native release run proving
  signing, notarization, stapling, certificate trust, metadata, and installer
  verification.
- Installed older-build-to-newer-build update testing remains owned by issue
  #511 and is not claimed here.

## Review-fix follow-up — 2026-09-14

### Fixes

- `.github/workflows/release.yml` now maps the protected `APPLE_API_KEY`
  secret to `APPLE_API_KEY_BASE64` only in a macOS preparation step, decodes
  it to a mode-600 `.p8` under `RUNNER_TEMP`, and exports only that path as
  `APPLE_API_KEY` through `GITHUB_ENV`. The macOS package step verifies the
  path exists and removes the file with an exit trap; an `always()` cleanup
  step provides a second removal path.
- `apps/desktop/tests/packageRelease.test.mjs` rejects direct secret-content
  mapping and pins the preparation order, temporary path, decode, file mode,
  `GITHUB_ENV` export, package-step path check, and cleanup wiring.
- Release documentation now requires an App Store Connect Team Key with App
  Manager access, states that the current Windows path is base64 `.pfx`/`.p12`
  secrets, and makes clear that managed signing requires separate workflow
  integration. User-facing wording now says verified artifacts are uploaded
  to a draft GitHub Release.

### TDD evidence

- RED: `node --experimental-strip-types --test tests/packageRelease.test.mjs`
  reported 11 passing and 2 failing tests. The failures showed the direct
  `APPLE_API_KEY` secret mapping and the missing API-key preparation step.
- GREEN: the same focused command reported 13 passing and 0 failing tests
  after the workflow implementation.

### Verification

- `node --experimental-strip-types --test tests/packageRelease.test.mjs` —
  passed, 13 tests.
- `pnpm.cmd docs:build` — passed. VitePress retained its existing circular
  chunk and large-chunk warnings.
- `bash scripts/doc-check.sh` — passed with no output.
- `bash scripts/verify-repo.sh` — blocked at its first step because `cargo`
  is not available on the Bash PATH in this Windows environment; the script
  did not reach its remaining checks.
- `git diff --check` — passed (exit 0); Git emitted only line-ending
  conversion warnings for existing working-tree files.

### External limitations

- The protected secret still has to contain the base64 content of a real App
  Store Connect Team Key `.p8` with App Manager access; source tests cannot
  validate the key, Apple account role, notarization, or stapling.
- Managed Windows signing is not supported by this workflow. Only the
  file-backed base64 `.pfx`/`.p12` path is wired; a managed-signing service
  needs separate workflow integration and credential-backed native testing.
- A real protected-environment macOS/Windows release run remains required to
  prove certificate trust and native signing/notarization behavior. Installed
  older-build update testing remains owned by issue #511.

### Commit

The review fixes and this report are included in the final fix commit; its
hash is reported at handoff because adding it here would change the hash.
