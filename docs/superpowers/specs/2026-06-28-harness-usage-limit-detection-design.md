# Harness Usage Limit Detection

**Date:** 2026-06-28
**Status:** Approved

## Problem

When a harness (Claude Code, OpenCode) hits its API usage/rate limit, the session keeps running but new prompts fail. OrkWorks has no way to surface this — the UI can't warn the user or adapt its suggestions.

The peon already captures `capacityHints` via LLM inference, but that path requires an inference call and isn't always running. Usage limit messages are deterministic, fixed strings — no LLM needed.

## Scope

- Detect usage/rate limit from terminal output for live sessions
- Expose `atUsageLimit: bool` on the session list API
- No gate on `POST /sessions` — user may create a new session regardless; the field is informational

Out of scope: context window limits, cooldown timers, per-harness capacity endpoints.

## Design

### 1. Limit patterns on HarnessAdapter

Add `limit_patterns: &'static [&'static str]` to `HarnessAdapter` in `harness.rs`:

```rust
pub struct HarnessAdapter {
    pub id: String,
    pub name: String,
    pub capabilities: HarnessCapabilities,
    pub limit_patterns: &'static [&'static str],
    // ...existing fields
}
```

Builtin adapter patterns in `builtin_adapters()` (`main.rs`):

| Harness | Patterns | Status |
|---|---|---|
| `opencode` | `["usage limit reached"]` | Confirmed (from test fixture) |
| `claude-code` | `["Claude Code is currently unavailable", "usage limit"]` | **Needs verification against live CLI output** |
| `generic-shell` | `[]` | N/A |

The Claude Code patterns are placeholders. They must be verified by observing actual CLI output when the account hits a rate limit before this ships to prod.

### 2. Scan function in peon.rs

```rust
pub fn detect_usage_limit(patterns: &[&str], lines: &[String]) -> bool {
    if patterns.is_empty() { return false; }
    lines.iter().rev().take(50).any(|line| {
        let lower = line.to_lowercase();
        patterns.iter().any(|p| lower.contains(p))
    })
}
```

Pure function, no I/O. Scans the last 50 lines (sufficient — limit messages appear near session termination).

### 3. SessionInfo field

Add to the `SessionInfo` struct and its JSON serialization:

```rust
#[serde(rename = "atUsageLimit", skip_serializing_if = "Option::is_none")]
at_usage_limit: Option<bool>,
```

`None` for dead/remembered sessions (no buffer). `Some(true/false)` for live sessions only.

### 4. list_sessions plumbing

Change the live session collection to also capture the buffer snapshot:

```rust
let live_sessions: Vec<(SessionInfo, Vec<String>)> = {
    let sessions = state.sessions.lock().unwrap();
    sessions.values()
        .map(|h| (h.info.clone(), h.output_buffer.snapshot()))
        .collect()
};
```

After `merge_live_session_info`, resolve the harness adapter and call `detect_usage_limit`:

```rust
let adapter = state.adapters.get(harness_id);
let at_usage_limit = adapter.map(|a| {
    peon::detect_usage_limit(a.limit_patterns, &snapshot)
});
merged_info.at_usage_limit = at_usage_limit;
```

Dead/remembered sessions leave `at_usage_limit` at `None` — they have no buffer to scan.

### 5. TypeScript

Add to `SessionInfo` interface (both `src/` and `electron/` copies if shared):

```typescript
atUsageLimit?: boolean;
```

The new session dialog and session list can use this to surface a warning. No changes required to the create-session API call.

## What's not included

- No 409 gate on `POST /sessions` — creating a session when capped is valid (may be intentional, detection may be wrong, different model/account)
- No capacity endpoint — `list_sessions` already carries the signal
- No TTL cache after a session dies — if all sessions of a harness are gone, we have no buffer to scan; the limit will reappear quickly if it's still active

## Verification needed before shipping

The Claude Code patterns (`"Claude Code is currently unavailable"`, `"usage limit"`) are guesses. Before merging to main, trigger a real rate-limit scenario with the Claude CLI and capture the exact terminal output. Update the patterns in `builtin_adapters()` accordingly.
