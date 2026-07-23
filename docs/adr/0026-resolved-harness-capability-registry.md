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

## Consequences

Adding a simple coding tool is one definition plus tests. Protocol-specific
support requires a compiled binding and primary-source contract fixture. User
configuration cannot introduce executable integration code or authority-bearing
paths. Legacy v1 arrays remain readable and migrate on the next successful
save.
