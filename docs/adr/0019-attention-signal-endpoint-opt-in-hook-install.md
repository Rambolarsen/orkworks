---
type: decision
status: accepted
title: "Attention signal via unauthenticated localhost endpoint, opt-in hook install only"
---

# Attention signal via unauthenticated localhost endpoint, opt-in hook install only

- Status: accepted
- Deciders: Lars-Erik
- Date: 2026-06-25

## Context

Peon infers `waiting_for_input` and related attention states by feeding terminal output to an LLM. The inference is probabilistic and runs on a polling interval. Claude Code already emits a deterministic `Notification` hook event when it is idle waiting on input — a higher-confidence signal OrkWorks was not using.

The existing metadata priority model (`user > agent > peon > backend_inference > process > unknown`) already reserves an `"agent"` slot that outranks Peon, but nothing wrote with that source. There was also no HTTP endpoint for anything outside the Peon loop to push session metadata.

Two approaches were considered for how the sidecar should accept this signal:

1. **A new `POST /sessions/:id/attention` endpoint on the existing unauthenticated localhost listener**, called by a shell command the user explicitly installs as a Claude Code `Notification` hook.
2. **Silent, automatic hook installation at session spawn time**, writing a curl command into `.claude/settings.json` (or `settings.local.json`) the first time OrkWorks spawns a Claude Code session.

## Decision

Add `POST /sessions/:id/attention` to the existing unauthenticated `127.0.0.1` listener (same trust posture as all other sidecar endpoints — ADR 0003 and ADR 0009). Any harness whose notification mechanism can invoke a shell command can call this endpoint; no Rust-side abstraction is needed until a second implementation exists.

Inject `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT` into every spawned session's environment so an in-session hook can address the correct sidecar port and session without any config look-up.

Hook installation is **always explicit and user-confirmed**. A "Install Notification hook" button in the Claude Code harness settings area triggers a confirmation dialog that names the file to be modified (`.claude/settings.local.json` in the workspace root), then calls `POST /workspace/attention-hook/install`. The endpoint merges a single hook entry into `settings.local.json` (never `settings.json`), preserving all other keys, and is idempotent. No hook is written automatically at session spawn.

Automatic installation (option 2) was rejected because:
- It writes tool-specific configuration into the user's repo without their knowledge.
- The hook command embeds assumptions about a running OrkWorks sidecar; running Claude Code outside OrkWorks would silently fire a curl call against a non-existent port.
- `settings.json` is committed and shared; writing a personal sidecar address there is wrong. `settings.local.json` avoids that but auto-writing anything into the user's files without confirmation violates the product principle of "observe and recommend before controlling" (ADR 0007).

## Consequences

- Hook installs require one explicit user action per workspace. Users who never click "Install" get no attention signal upgrade over Peon inference — that is intentional.
- The sidecar's unauthenticated surface grows by one writable endpoint. Accepted: the threat model (localhost-only, single user, no secrets in the payload) is unchanged from ADR 0003/0009.
- `settings.local.json` must be in `.gitignore` for the protection to hold; OrkWorks does not enforce this, but it is Claude Code's own convention and is already gitignored in this repo.
- A future adapter for Codex, OpenCode, or Aider reuses the same endpoint and env vars with no Rust changes. A Rust-side `AttentionSignalSource` trait can be introduced later if an adapter needs in-process logic rather than an HTTP call.
- `POST /workspace/attention-hook/status` (read-only) lets the UI reflect current install state without any Peon-loop changes.
