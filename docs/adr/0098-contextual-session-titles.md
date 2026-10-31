---
type: Architecture Decision
title: Contextual session titles with native and user ownership
description: Preserve topic titles while accepting native renames and one contextual fallback refinement.
tags: [orkworks, sessions, peon, architecture]
---

# Contextual session titles with native and user ownership

- Status: accepted
- Deciders: Rambolarsen, Codex
- Date: 2026-10-31

## Context

The owner approved native-first naming, contextual fallback refinement and
explicit user rename precedence in #1203. Initial-input-only inference has too
little context; Codex scheduling stops once native provenance is accepted.
A live projection also copies activity summaries into titles.

## Decision

This supersedes the frozen-after-input naming clauses of ADR 0042 and ADR 0047,
and the stop-after-first-native-title scheduling in ADR 0063. Their summary,
bootstrap handoff, source ownership and exact-native-identity constraints stay.

- `label` remains the topic, independently of activity `summary`. Output scans
  mirror the latest persisted title rather than an activity summary.
- Subsequent authenticated Codex reports refresh automatic titles using the
  existing read-only exact-ID adapter, cooldown and runtime/reset fences.
  Identical title/source writes are no-ops. Idle renames remain opportunistic,
  observed on a later report without filesystem watching.
- Peon uses a dedicated title prompt with bounded instruction and optional
  current-runtime output. It can make one successful contextual refinement
  after concrete work evidence arrives, tracked by an additive default-false
  metadata marker. Later activity does not keep renaming the session. Bootstrap
  inference cannot overwrite the first descriptive terminal instruction.
- A desktop-authorized, expected-workspace-bound title operation reads, sets or
  restores automatic naming. Set uses existing `user` provenance. Manual titles
  survive Peon, native updates and conversation resets until explicitly reset
  to automatic. Set/reset invalidate pending work; durable success precedes
  publication to the live projection.
- User titles are non-empty, control-free and at most 100 Unicode scalar values.
  Titles are display data only: no attention, activity timestamps, workflow
  evidence, native identity or native title mutations.

## Consequences

Premature names can improve and native Codex names can refresh without turning
titles into an activity feed. Unknown legacy provenance stays protected from
native replacement. Codex remains the only qualified native title integration;
Claude/OpenCode adapters need separate version/payload evidence. No coding-tool
process is launched to rename or resume another conversation.

Revert needs no migration: older readers ignore the contextual marker and
existing user provenance already prevents automatic writes. Restoring automatic
naming uses descriptive stored input or the placeholder until normal inference
or a native report supplies a better topic.
