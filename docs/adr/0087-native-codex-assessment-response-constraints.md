---
type: Architecture Decision
title: Native Codex assessment uses code-owned response constraints
description: Accepted static output schemas for native Codex assessment stages with unchanged inference authority and strict application validation.
tags: [orkworks, architecture, taskmaster, codex, assessment]
status: stable
---

# Native Codex assessment uses code-owned response constraints

- Status: accepted; owner approved the concrete design on 2026-10-18
- Deciders: repository owner
- Date: 2026-10-18
- Tracking: [#931](https://github.com/Rambolarsen/orkworks/issues/931), [#938](https://github.com/Rambolarsen/orkworks/pull/938)

## Context

Selected-model assessment evaluations repeatedly returned malformed, duplicate
or unknown JSON fields despite prompt guidance. Prose validation also has false
positives; broad grammar exceptions introduced bypasses and were removed.
Neither retries nor broader acceptance establishes the required quality gate.

## Decision

Extend [ADR 0086](0086-repository-led-taskmaster-assessment.md) with explicit,
code-owned response-shape requests for the existing discovery and final
assessment envelopes. Only native Codex assessment invocations opt into static
schemas through the CLI's output-schema facility. Preserve the existing
[ADR 0054](0054-taskmaster-honors-managed-cli-policy.md) inference profile,
selected model/effort, managed policy, timeout and authority. Other transports
and ordinary analysis remain unchanged.

The [approved design and delivery checks](../superpowers/specs/2026-10-18-assessment-response-constraints.md)
define schema ownership, temporary-file privacy/lifetime, typed invocation,
input identity, compatibility and recovery. The owner approved that concrete
proposal in this task before dependent transport implementation. This decision
extends the earlier ADRs and does not supersede their authority constraints.

Application decoding remains authoritative for bounds, citations, stage
combinations, commands and unsupported verification claims. Unsupported schema
execution fails without response repair, retry or an unconstrained fallback.
Selected-model compatibility must pass before production wiring; a fresh full
quality matrix with semantic adjudication still gates usefulness and PR readiness.

## Consequences

Provider constraints can reduce wire-shape failures without adding model calls
or changing recommendation authority. They cannot establish factual grounding,
prose safety or usefulness. The static schema introduces no repository-derived
retention and lives only with the existing owned inference temporary directory.
A code-owned contract marker makes the new input distinguishable without a record
migration; existing accepted/executing and pending-publication protections remain.
