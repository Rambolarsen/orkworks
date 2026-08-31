# Resolved harness capability registry

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-07-22

## Decision

OrkWorks resolves embedded declarative built-ins plus sparse user overrides
into one immutable registry. Declarative closed capability variants implement
common behavior; closed compiled Rust bindings implement only verified tool
protocols. All consumers read the same published snapshot.

Workspace integration mutations require Electron-main confirmation and
sidecar mutation authority, canonical no-follow workspace containment,
ownership-aware edits, and durable write-before-publish transactions. The
renderer and reporter processes never receive mutation authority.

Complete custom definitions cannot directly select compiled signal handlers,
reporters, or authority-bearing paths. An explicitly supported duplicate
operation may instead attach a sidecar-owned compatibility profile, persisted
outside the editable definition and resolved only through a compiled
allowlist. The profile may derive an existing closed binding, but cannot carry
user-supplied code, paths, or handler selection.

## Consequences

Adding a simple coding tool is one definition plus tests. Protocol-specific
support requires a compiled binding and primary-source contract fixture. User
configuration cannot introduce executable integration code or authority-bearing
paths, or directly choose a compiled binding. A reviewed duplicate workflow
may preserve an existing allowlisted binding through sidecar-owned profile
metadata. Legacy v1 arrays and version-2 harness documents remain readable and
migrate on the next successful save.
