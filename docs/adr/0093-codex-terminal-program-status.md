---
type: Architecture Decision
title: Codex terminal program status
description: Prefer observed Codex UI status over approval-start hooks after a runtime emits OSC 7501.
tags: [orkworks, architecture, codex]
---

# Codex terminal program status

- Status: accepted
- Deciders: Lars-Erik, Codex
- Date: 2026-10-25
- Supersedes: ADR 0051 for attention in runtimes emitting OSC 7501; identity,
  installation, legacy-runtime behavior and native/completion gates remain.

## Context

PermissionRequest precedes automatic review and therefore does not establish
that a human prompt exists. Completion correlation cannot reliably avoid this
false wait. Codex already emits its UI program status through OSC 7501.

## Decision

Adopt the owner-approved [terminal status design](../superpowers/specs/2026-10-25-codex-automatic-approval-signal-design.md).
Observe live PTY bytes with a bounded parser. The first valid Codex status
selects this source for that runtime; ignore subsequent attention hooks.
Preserve explicit user/agent attention, suppress competing inference, and
record terminal evidence at backend-inference priority, not agent authority.

## Consequences

Manual prompts appear when the UI blocks; automatic review stays Working.
A missed/spoofed terminal frame remains possible and is accepted as a display
limitation. This does not prove native request identity or approval decisions.
No launch protocol, configuration, integration install or API-only migration is
needed. A revert and fresh runtime restore legacy behavior. Dormant observer
and completion infrastructure remains disabled and outside this change.
