# ADR 0062: Guided completion packets stay recommendation projections

- Status: accepted
- Date: 2026-09-24

## Context

Taskmaster needs a reproducible handoff between implementation, verification,
independent review, and user inspection. A separate packet lifecycle or an
autonomous coordinator would weaken the existing recommendation contract and
could imply that `ready_for_user_review` means accepted by the user.

## Decision

Represent a guided completion packet as an optional projection inside the
existing recommendation record. The sidecar validates a packet's schema and
evidence versions, source session, workspace snapshot, derived readiness,
revision, evidence fingerprint, action fingerprint, approval binding, and
idempotency key. Packet replacement preserves a bounded lineage entry, clears
approval, rejects stale mutations, and returns the recommendation to the
existing `proposed` state. An authenticated source-session report may attach or
supersede a packet, but it cannot start or focus a session, modify files, mutate
Git, or delegate work.

Packet readiness is evidence-derived: verification is needed, review is ready,
findings need a fix, or the result is ready for user review. The final state is
never user acceptance. The existing single-active-context and explicit user
approval rules remain unchanged. Missing target sessions can be recovered to a
retryable proposal while retaining packet evidence.

Recursive delegation, child-session orchestration, and bounded root-plan
approval are deliberately deferred. They require a separate specification and
ADR covering non-forgeable coordinator capabilities, approval semantics, role
enforcement, budgets, graph invariants, cancellation, crash recovery, and
authenticated version-checked child reports before any implementation begins.

## Consequences

- Packet evidence is auditable and revision-bound without introducing a second
  lifecycle system.
- Stale, malformed, unauthorized, cancelled, cross-workspace, and late
  actions fail closed.
- The UI can present packet provenance and readiness while retaining the
  existing user-driven action model.
- Future coordinator work must pass a separate design gate and cannot be
  inferred from this packet implementation.
