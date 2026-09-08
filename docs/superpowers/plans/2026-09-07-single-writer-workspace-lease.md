# Single-writer workspace lease

## Goal

Prevent a second `orkworksd` instance from marking sessions owned by another
sidecar as dead during workspace initialization.

## Implementation

- [x] Add an OS advisory lease abstraction for a workspace metadata directory.
- [x] Acquire and retain the lease before migration or orphan reconciliation.
- [x] Return HTTP 409 when workspace ownership is already held elsewhere.
- [x] Add unit and handler coverage for exclusivity and no-reconciliation-on-conflict.
- [x] Run the complete Rust and desktop verification suites.

## Verification

The focused lease, workspace-open, and HTTP conflict tests must pass. Run the
full Rust suite, Rust formatting/build checks, and the desktop typecheck/tests
before handoff.
