---
okf_version: "0.2"
---

# Agent Knowledge Bundle

Durable agent-facing project knowledge is organized into the concepts below.
Start with the context relevant to the task, then follow related links as
needed.

## Project and product context

- [Project context](project-context.md) — Product identity, repository state,
  CI routing, package management, and optional containerized development.
- [Product boundaries](product-boundaries.md) — Product scope, terminology,
  non-goals, and load-bearing user-experience constraints.
- [Architecture](architecture.md) — Electron, React, Rust sidecar, API, and
  runtime architecture reference.
- [Domain entities](domain-entities.md) — The session state model and its
  core domain concepts.

## Development and agent operations

- [Development workflow](development-workflow.md) — Issue, planning,
  documentation, decision, and maintenance workflow reference.
- [APM and agent plugins](apm.md) — APM-managed dependencies, generated
  assets, and agent-plugin operations.
- [Subagent model policy](subagent-model-policy.md) — Default model tier and
  escalation guidance for delegated work.

## Harnesses and integrations

- [Harness integration contracts](harness-integration-contracts.md) — Evidence
  and constraints for coding-tool integrations and signals.
- [Harness instruction coverage](harness-instruction-coverage.md) — How each
  supported harness receives repository and scoped instructions.

## Troubleshooting

- [Codex API troubleshooting](codex-api-troubleshooting.md) — Safe diagnostics
  for generic Codex API failures.
- [Peon timeout troubleshooting](peon-timeout-troubleshooting.md) — Recovery
  and diagnostics for Peon provider timeouts.
