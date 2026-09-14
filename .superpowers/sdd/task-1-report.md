# Task 1 report

## Status

Complete. Release metadata validation and deterministic checksum generation are
implemented, and the remaining Task 1 review findings are fixed.

## Complete implementation

- `apps/desktop/package.json` adds direct `js-yaml` dev dependency `^4.2.0`.
- `apps/desktop/pnpm-lock.yaml` records the matching locked dependency.
- `apps/desktop/scripts/releaseMetadata.mjs` exports:
  - `writeChecksumManifest({ releaseDir, outputPath })`, which hashes only
    top-level `OrkWorks-*`, `latest*.yml`, and `*.blockmap` files, sorts names,
    excludes `SHA256SUMS.txt`, and writes complete SHA-256 manifest lines.
  - `verifyUpdateMetadata({ metadataPath, releaseDir, expectedVersion })`,
    which parses YAML and validates version, file entries, integer sizes,
    SHA-512/base64 payload digests, payload files, and matching blockmaps.
- `apps/desktop/tests/releaseMetadata.test.mjs` uses real temporary files and
  covers complete checksum lines, valid metadata, missing payloads, incorrect
  real-file SHA-512 values, size mismatches, missing blockmaps, traversal,
  payload and blockmap symlink escapes, checksum input/output symlink escapes,
  broken checksum output symlinks, metadata symlink escapes, malformed YAML,
  invalid `files[]` entry shapes, and version rejection.

## Review fixes

- Checksum inputs are resolved with `realpathSync` and compared with the real
  release root before `statSync` or `readFileSync`; distributable symlinks that
  resolve outside the release are rejected.
- The checksum output is lexically constrained to the release directory,
  checked with `lstatSync`/`realpathSync` when present, and validated through
  its real parent before a new file is written. Existing, broken, or escaping
  output symlinks fail closed.
- Metadata is resolved and checked against the real release root before it is
  read or parsed. Outside metadata symlinks and traversal paths are rejected.
- YAML parsing and `files[]` shape validation remain fail-closed with focused
  real-filesystem regression coverage.

## Tests and results

- `pnpm.cmd install --frozen-lockfile --ignore-scripts --reporter append-only` —
  passed for the original implementation.
- TDD red stage after adding checksum input/output and metadata symlink tests:
  `node --experimental-strip-types --test tests/releaseMetadata.test.mjs`
  — 12 passed, 3 failed as expected because those escapes were accepted.
- TDD red stage after adding the broken-output-symlink test:
  `node --experimental-strip-types --test tests/releaseMetadata.test.mjs`
  — 15 passed, 1 failed as expected because the broken symlink could be
  followed during output.
- TDD green stage:
  `node --experimental-strip-types --test tests/releaseMetadata.test.mjs`
  — 16 passed, 0 failed, 0 skipped.
- `git diff --check` — passed before commit.
- `bash scripts/verify-repo.sh` — previously unable to complete in this
  environment because `cargo` is unavailable; the focused Task 1 suite above
  is the verified scope for this fix.

## Commits

- `6b84b9b` — `build: validate release metadata and checksums`
- `09f3fbb` — `fix: reject release metadata symlink escapes`
- `04e4c3e` — `fix: contain release metadata and checksum paths`

## Concerns

- Full repository verification remains environment-limited by the missing
  `cargo` executable; no Rust or unrelated files were changed here.
- Credential-backed native artifact verification remains outside Task 1.
- No signing workflow, packaging configuration, runtime update behavior, #510,
  or #511 work was included.
