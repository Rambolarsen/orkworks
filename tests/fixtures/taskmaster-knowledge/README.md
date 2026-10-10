# Synthetic Taskmaster knowledge proof corpus

Every page and signing key in this directory is synthetic test data. The
Ed25519 private key is deliberately committed only so the corpus can be
reproduced and shared with the Rust verifier tests. It is not a production
credential, cannot sign production-eligible content, and must never be copied
into application resources or release artifacts.

Run `rtk proxy node scripts/generate-knowledge-fixtures.mjs` from the repository
root to regenerate the corpus. The generator creates a test-only key pair once
and reuses the fixed private key thereafter; it refuses to replace an incomplete
key pair.
