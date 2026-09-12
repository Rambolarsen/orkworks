# Taskmaster reuses CLI logins and honors managed policy

- Status: accepted
- Deciders: repository owner
- Date: 2026-09-10

## Context

The owner requires existing CLI authentication, not separate API credentials.
The initial ADR 0053 interpretation required isolation from every hook and
instruction source. Installed CLIs deliberately preserve administrator-managed
policy despite ordinary safe-mode or ignore-user-config switches. The owner
explicitly chose to honor those policies rather than bypass or reject them.

## Decision

Supersede ADR 0053's absolute CLI isolation requirement. Its signed knowledge
distribution, independent model selection, budgets, evidence validation, and
explicit action handoff remain the design, as recorded in the
[knowledge specification](../../specs/taskmaster-knowledge.md).

Use fixed recommendation-only CLI profiles with existing authentication and
private temporary working directories. Disable optional tools/integrations where
supported while leaving managed policy authoritative. Do not modify policy
sources, override a policy conflict, copy credentials, or fall back to another
provider. Preserve the explicit model argument and Peon's applied selection.
Version-check supported CLI profiles before sending context; failed or unknown
compatibility checks do not run inference. Custom command wrappers are not
automatically eligible for a builtin profile.

## Consequences

CLI inference can honor required hooks, added instructions, and managed routing.
OrkWorks bounds its own collected context and request; it cannot promise the CLI
has no additional policy-driven effects. Disclose this in Settings and user docs.
Unexpected tool-action output is rejected, but post-run parsing cannot undo CLI
side effects. Taskmaster still requests no coding actions and never automatically
executes generated recommendations. Unsupported transports remain unavailable.
