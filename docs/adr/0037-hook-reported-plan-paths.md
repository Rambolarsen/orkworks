# Hook-reported plan paths

- Status: accepted
- Deciders: OrkWorks maintainers
- Date: 2026-08-04

## Context

Terminal output cannot reliably establish which plan a session authored. The
existing fallback can retain an old false match and the shared attention route
would incorrectly turn a successful file write into `waiting_for_input`.

## Decision

Claude Code's owned integration installs a `PostToolUse` `Write|Edit` hook.
It forwards the raw hook file path to a dedicated path-only sidecar route.
That route canonicalizes the file, requires an existing regular Markdown file
under the supported plan/spec roots, stores its workspace-relative form, and
does not change attention. A successful hook report supersedes a prior terminal
fallback; fallback only fills an empty association.

Codex remains on the terminal fallback. Its documented `PostToolUse`
`apply_patch` payload provides patch text rather than a canonical file path, so
parsing it would recreate the unreliable inference this decision removes.

## Consequences

Claude plan associations become deterministic without widening the renderer or
terminal-input authority. POSIX and PowerShell reporters remain thin transport;
Rust owns path validation once. This extends the marker-specific hook semantics
noted in ADR 0035 and deliberately does not generalize `ToolHookContract`; that
broader concern remains issue #271. Renumbered from 0036 to 0037 after ADR 0036
(`codex-hooks-portable-reporter-path`) landed on `main` first.
