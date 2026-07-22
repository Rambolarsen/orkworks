# Authenticated session plan handoff

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-07-22

## Context

An agent can report a Markdown plan that explains why a session needs user attention. Opening that plan is a privileged desktop action: the renderer must not receive arbitrary filesystem paths, and the localhost sidecar must not expose an unauthenticated endpoint that returns them.

## Decision

Electron generates a cryptographically random token for each sidecar process and passes it only in the sidecar's startup environment. The plan-open endpoint requires that token, and Electron's preload API accepts only a session ID.

The sidecar validates that the stored plan is a regular Markdown file within the current workspace before returning its canonical path to Electron main. Immediately before invoking the operating system's file opener, Electron canonicalizes and revalidates both the workspace and returned file, including containment, file type, and extension. The token is excluded from terminal child-process environments. The renderer sees only `hasOpenablePlan` and sanitized failures.

## Consequences

Session-plan handoff can offer an explicit `Open plan` action without widening the renderer's filesystem authority or making plan paths readable through the regular localhost API. Sidecar startup paths must consistently provide the secret, and both validators must stay aligned as the supported plan-file policy evolves.
