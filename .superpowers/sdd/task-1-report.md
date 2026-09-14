# Task 1 report

## Status

Complete. Release metadata validation and deterministic checksum generation
are implemented, and the Task 1 review findings are fixed.

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
  payload symlink escapes, blockmap symlink escapes, and version rejection.
  Temporary directories are cleaned up in `finally` blocks.

## Review fix summary

Metadata paths still undergo lexical containment checks, then existing
payloads and blockmaps are resolved with `realpathSync` and compared against
the real release root before `statSync` or `readFileSync`. Missing or
unresolvable paths fail closed, and symlink/junction escapes are rejected.

## Tests and results

- `pnpm.cmd install --frozen-lockfile --ignore-scripts --reporter append-only` —
  passed for the original implementation.
- TDD red stage after adding the review tests:
  `node --experimental-strip-types --test apps/desktop/tests/releaseMetadata.test.mjs`
  — 8 passed, 2 failed as expected because symlink escapes were accepted.
- TDD green stage after the minimal helper fix:
  `node --experimental-strip-types --test apps/desktop/tests/releaseMetadata.test.mjs`
  — 10 passed, 0 failed, 0 skipped.
- `git diff --check` — passed.

## Commits

- `6b84b9b` — `build: validate release metadata and checksums`
- `09f3fbb` — `fix: reject release metadata symlink escapes`

## Scope notes

No signing workflow, packaging configuration, runtime update behavior, #510,
or #511 work was included. Credential-backed native artifact verification
remains outside Task 1.
