# JSON-defined Taskmaster inference adapters

- Status: accepted
- Deciders: repository owner
- Date: 2026-09-10

## Decision

Extend the harness registry with an independent inference capability. User JSON
may define a direct executable, argv templates, stdin/file input, bounded
timeout, and a fixed result JSON protocol. Preserve custom model identifiers
and existing coding-tool provider configuration/authentication. Do not copy
credentials or demand separate API keys.

Code-owned built-in profiles and explicitly trusted custom commands use the
same capability boundary but retain distinct provenance. Importing JSON is not
approval to execute it. Grants live separately, bind to the normalized
definition and resolved executable path, and participate in scheduling,
cache identity, and atomic result acceptance. Existing executable updates and
configuration remain within the user's disclosed trust in that installed tool.

This extends [ADR 0054](0054-taskmaster-honors-managed-cli-policy.md), not its
managed-policy rule. A custom command is trusted executable code, not certified
recommendation-only by its JSON declaration. Required policy remains in force;
custom automation side effects and timeout-limited revocation are disclosed.
Taskmaster never executes generated recommendations automatically.

## Consequences

Users can provide adapters without a compiled provider allowlist. A wrapper may
be necessary to translate native output into the fixed result envelope. The
first implementation slice only validates and persists definitions; no custom
command becomes runnable until trust, transport, and lifecycle integration are
implemented and verified.

The [reviewed design](../superpowers/specs/2026-09-10-custom-inference-adapters-design.md)
defines bounds, trust semantics, migration, and acceptance requirements.
