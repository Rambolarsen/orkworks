# Provider ops panel and app-wide Peon fallback

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-06-21

## Context

Peon currently executes a single harness command, defaulting to OpenCode. When OpenCode is capped or otherwise unavailable, Peon fails silently and the app loses the observer that should have explained the failure. The desktop app also lacks one place to inspect provider availability, set fallback order, or apply a manual cap override.

## Decision

Store provider preferences as app-wide Electron settings with one `defaultState` and one optional `overrideState` per provider. Keep executable provider definitions in the Rust sidecar, where a fixed registry owns labels, argv conventions, timeout policy, and runtime error classification. Expose live provider runtime state over `/providers`, and render the existing `capacity` panel slot as a Providers operations surface.

## Consequences

- Peon can fall back from one provider to another without storing arbitrary commands in user settings.
- The app gets a single provider-control model that future recommendation work can reuse.
- Electron and the sidecar must stay in sync on saved provider revisions, so startup and reconnect flows now include a settings push.
