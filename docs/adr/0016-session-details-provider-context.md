# Session details provider context

- Status: superseded by 0017 (Settings surface)
- Supersedes: 0015 (UI surface only)
- Date: 2026-06-22
- Deciders: OrkWorks maintainers

## Context

Provider fallback remains necessary for Peon reliability, but a dedicated Providers panel overstates how important provider management is to the main user workflow. OrkWorks is session-centric and should expose only the provider context relevant to the selected session.

## Decision

Show session-specific provider context in read-only `Details` fields:

- `Provider`
- `Model`
- `State`

Keep provider editing app-wide in `Settings`. Remove Providers as a primary Dockview surface. The backend fallback system from ADR 0015 remains in place.

## Consequences

- Session details become the only always-relevant provider surface in the main window.
- Provider editing remains available without breaking the read-only interaction model in `Details`.
- ADR 0015's backend fallback decision stands, but its primary-panel UI decision is superseded.
