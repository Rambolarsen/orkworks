# Synthetic Taskmaster knowledge proof corpus

Every page and signing key in this directory is synthetic test data. The
Ed25519 key pairs are deliberately test-only material so the corpus can be
reproduced and shared with the Rust verifier tests. They are not production
credentials, cannot sign production-eligible content, and must never be copied
into application resources or release artifacts. `fixture-index.json` names
the API, expected outcome, and invariant for each shared fixture. Oversized
activation/envelope wrappers are generated in memory by each language's tests;
the two exact 2 MiB signed-envelope fixtures remain shared.

Run `rtk proxy node scripts/generate-knowledge-fixtures.mjs` from the repository
root to regenerate the corpus. The generator creates a test-only key pair once
and reuses the fixed private key thereafter; it refuses to replace an incomplete
key pair.
