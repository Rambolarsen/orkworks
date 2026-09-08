# Agent Knowledge Bundle Design

## Goal

Reduce the size and cognitive load of the root `AGENTS.md` while preserving
its role as the authoritative router for repository-wide rules and making
durable project knowledge easier for humans and agents to discover.

## Problem

`AGENTS.md` currently combines load-bearing workflow instructions with
project context, architecture reference material, operational runbooks, and
product terminology. That makes every task pay the context cost of reading
information that may not apply. The repository already has a growing
`docs/agents/` collection, but it has no progressive-disclosure index or
consistent machine-readable metadata.

## Decision

Use an Open Knowledge Format (OKF) v0.2-style bundle rooted at
`docs/agents/`:

- `docs/agents/index.md` is the bundle index and the entry point for durable
  agent knowledge.
- Each knowledge document is a Markdown concept with YAML frontmatter
  containing at least `type`, `title`, `description`, and `tags`.
- Concepts link to related concepts with ordinary Markdown links.
- Existing specialized agent documents remain separate concepts and gain
  metadata rather than being merged into one larger file.

Keep `AGENTS.md` concise and authoritative for instructions that must be
visible at repository scope:

- product identity and non-goals;
- authoritative specs and issue-board rules;
- universal development, branch, PR, verification, and documentation rules;
- scoped instruction routing and hard boundaries;
- load-bearing terminology, architecture, metadata, and UX constraints;
- a new `## Documentation` section linking to the OKF bundle index.

Move explanatory or operational reference material into focused concepts:

- project context, CI, and the optional container workflow;
- development workflow and decision tracking;
- product boundaries, terminology, and UX principles;
- architecture and metadata protocol details;
- APM, MCP, and agent-plugin operations;
- troubleshooting and maintenance runbooks.

The existing uncommitted `Assumption discipline` addition is preserved and
placed with the development workflow guidance. No product behavior, runtime
code, dependency, CI behavior, or authoritative product specification changes
are included.

## OKF compatibility boundary

This repository adopts OKF's structural conventions, not a new runtime or
validation dependency. The bundle uses `index.md` for progressive disclosure,
concept frontmatter for routing metadata, and Markdown links for relationships.
Optional provenance and lifecycle fields are added only when the repository
has an authoritative value; invented authorship or freshness claims are not
introduced merely to fill the schema.

## File layout

```text
docs/agents/
  index.md
  project-context.md
  development-workflow.md
  product-boundaries.md
  architecture.md
  apm.md
  codex-api-troubleshooting.md
  domain-entities.md
  harness-instruction-coverage.md
  harness-integration-contracts.md
  peon-timeout-troubleshooting.md
  subagent-model-policy.md
```

The three new concept files provide homes for material extracted from the
root guide. Existing files are edited only to add frontmatter and update
cross-links where necessary.

## Consequences

Agents still receive the rules that constrain repository-wide work from the
root guide. Agents that need deeper context can follow one stable index and
read only the relevant concept. Humans get smaller, topic-focused documents;
the tradeoff is that maintaining the bundle requires updating both the root
router and the relevant concept when a universal rule changes.

## Validation

- Every `docs/agents/*.md` concept has valid frontmatter and an accurate
  description in `docs/agents/index.md`.
- Root-to-bundle and bundle-internal links resolve.
- `AGENTS.md` retains all load-bearing rules and links to the moved reference
  material.
- `bash scripts/doc-check.sh` passes without unresolved documentation drift.
- The docs site build passes with the new Markdown files.

## Rollback

Restore the moved reference sections to `AGENTS.md`, remove the new concept
files and index, and retain the existing specialized documents. No source or
runtime rollback is required.
