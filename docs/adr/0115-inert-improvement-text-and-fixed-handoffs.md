---
type: Architecture Decision
title: Inert improvement text and fixed handoffs
description: Accepted separate text validation for Brain improvement results and code-owned handoff authority.
tags: [orkworks, architecture, taskmaster, security]
status: stable
workflow_status: approved
---

# Inert improvement text and fixed handoffs

- Status: accepted; owner approved the full contract in PR #1753 on 2026-11-13.
- Decider: repository owner.
- Tracking: [#1694](https://github.com/Rambolarsen/orkworks/issues/1694).
- Contract: [Inert text and fixed handoffs](../superpowers/specs/2026-11-13-improvement-text-contract.md).

## Context

Three bounded comparisons exposed repeated rejection of ordinary explanatory
language. Expanding an English command classifier does not establish execution
authority. Existing implementation handoffs interpolate generated descriptions,
so text acceptance and handoff construction must change together.

## Decision

For the new Brain improvement producer, validate text as bounded single-line
reference data with explicit control/bidi exclusions and existing verification-
claim checks. Preserve structural/citation validation. Use fixed, server-owned,
ID-only instructions for both intents and refuse client overrides and completion
packets. Select policy 2 through consistent versioned producer/report/history
identities. Legacy producers retain their strict text and handoff behavior.

This supersedes only ADR 0114's delegated text-policy choice through the linked
approved amendment. Its bounded investigation, publication, privacy and activation
gates remain in force; no independent coding-tool confinement is claimed.

## Consequences

Ordinary explanations and literal command mentions remain inert data rather than
being classified as executable English. New producer records require coordinated
Rust/desktop version support and fail closed on unsupported clients/identities.
Offline replay and deterministic tests cannot establish usefulness. Activation
and another live comparison remain separately gated.
