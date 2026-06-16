# M6: Peon — Observer Inference Design

## Overview

Peon is a low-cost AI observer embedded in `orkworksd` that reads terminal output and infers session metadata. It normalizes messy terminal output into structured OrkWorks state. Observer-only — never types into terminals, never approves commands.

## Integration

Peon runs as a tokio task inside `orkworksd`. Shares process memory with the session registry, PTY output paths, and the existing `MetadataStore`. Single binary, no separate process.

## Module: `src/peon.rs`

New module added to `crates/orkworksd/src/peon.rs`, declared in `main.rs`.

### Data Structures

```rust
/// Configuration parsed from environment variables.
struct PeonConfig {
    harness: String,            // PEON_HARNESS, default "opencode"
    harness_args: String,       // PEON_HARNESS_ARGS, default "--print -p"
    model: Option<String>,      // PEON_MODEL, optional model override
    interval_secs: u64,         // PEON_INTERVAL, default 5
    max_lines: usize,           // PEON_MAX_LINES, default 200
    timeout_secs: u64,          // PEON_TIMEOUT, default 30
    enabled: bool,              // PEON_ENABLED, default true
}

/// Fixed-size ring buffer of terminal output lines per session.
struct RingBuffer {
    lines: VecDeque<String>,
    capacity: usize,
}

/// Structured inference result from Peon.
/// All fields optional — Peon infers what it can.
#[derive(Deserialize, Serialize)]
struct PeonInference {
    status: Option<String>,            // waiting_for_input, blocked, failed, done, stale, working, idle
    phase: Option<String>,             // current work phase
    summary: Option<String>,           // one-line summary
    next_action: Option<String>,       // suggested next action
    needs_user_input: Option<bool>,    // session needs user response
    detected_question: Option<String>, // the question needing an answer
    suggested_options: Option<Vec<String>>,
    blocker_description: Option<String>,
    failed_command: Option<String>,
    failed_test: Option<String>,
    capacity_hints: Option<Vec<String>>, // cap/rate-limit hints from output
    confidence: f64,                    // 0.0–1.0, always present
}
```

### Per-session additions to `SessionHandle`

```rust
struct SessionHandle {
    info: SessionInfo,
    kill_tx: watch::Sender<bool>,
    output_buffer: RingBuffer,  // NEW
}
```

Peon maintains a separate `HashMap<String, Instant>` (`last_output` per session) behind an `RwLock` to track when each session last produced output, avoiding contention with the main session mutex.

## Output Collection

### Ring buffer

- Fixed capacity (`PEON_MAX_LINES`, default 200).
- Stores decoded UTF-8 lines from PTY output.
- When full, oldest lines are dropped.
- Implemented as `VecDeque<String>` with a `push(line)` method that truncates at capacity.

### Integration into PTY output path

In `handle_session_terminal`, after the blocking reader task reads PTY output and before sending to the mpsc channel:

1. Decode the `[u8]` chunk as UTF-8.
2. Split into lines.
3. For each line, push to the session's ring buffer.
4. Update the session's `last_output` timestamp in a shared map (behind its own `RwLock`, separate from the sessions mutex to avoid contention).

### Debounce trigger

The Peon tokio task runs a loop: every second, it scans the `last_output` timestamps for all sessions. For any session whose `last_output` is older than `PEON_INTERVAL` seconds AND whose `last_inference` is older than `last_output` (i.e., new output arrived since the last inference), Peon:

1. Locks the session registry briefly to read the session ID and cwd.
2. Collects the ring buffer contents for that session.
3. Spawns an inference task (non-blocking — does not hold the lock during LLM call).
4. Returns the lock.

This ensures output silence triggers inference (agent finished generating), not continuous streaming (agent mid-generation). The 1-second poll loop is simpler than per-session timers and avoids `Sleep` values inside the session mutex.

## Inference Pipeline

### 1. Build prompt

System prompt (compact, inlined in code):

```
You are a terminal output analyzer. Analyze the following terminal session output and return a JSON object describing the session state. Only include fields you are confident about. Return ONLY valid JSON, no other text.

Available fields:
- status: one of "waiting_for_input", "blocked", "failed", "done", "stale", "working", "idle"
- phase: short description of current work phase
- summary: one-line summary of what's happening
- next_action: suggested next step
- needs_user_input: boolean, true if the terminal is prompting for user input
- detected_question: the question the user needs to answer
- suggested_options: array of possible answers
- blocker_description: what's blocking progress
- failed_command: the command that failed
- failed_test: the test that failed
- capacity_hints: array of cap/rate-limit related strings found in output
- confidence: number 0.0 to 1.0 indicating your confidence in this analysis
```

User message: the ring buffer contents, truncated to fit within reasonable token limits (~4K chars).

### 2. Invoke harness

Shell out to the configured harness binary with a one-shot prompt:

```
<harness> --print -p "<prompt>"
```

For `opencode` specifically: `opencode --print -p "<prompt>"`. Other harnesses may need different flags; the `PEON_HARNESS` env var selects the binary and `PEON_HARNESS_ARGS` provides custom flags for non-standard harnesses (e.g. `--print` for one-shot, `--model` override).

The subprocess:
- Receives the prompt on stdin or as a CLI arg.
- Outputs the model response to stdout.
- Peon captures stdout and extracts JSON.

Timeout: 30 seconds per inference call. If the harness doesn't respond in time, kill the subprocess and log a warning.

### 3. Extract and validate JSON

1. Parse stdout. Strip any markdown code fences (```json ... ```).
2. Deserialize into `PeonInference` using `serde_json`.
3. Validate:
   - `confidence` must be present and in range 0.0–1.0.
   - If `status` is present, it must be one of the valid status strings.
   - Other fields are free-form.
4. If validation fails, log a warning with the raw response, discard the result.

### 4. Priority preservation

Before writing inference results to `.orkworks/sessions/<id>.json`:

1. Read the current `SessionMetadata` from the `MetadataStore`.
2. Check `metadata_source`:
   - `"user"` → never overwrite.
   - `"agent"` → skip if the file was modified within the last 5 minutes.
   - `"peon"`, `"backend_inference"`, `"process"`, `"unknown"`, or absent → write.
3. If skipping, log at debug level and return.

This respects the metadata source priority hierarchy: user > agent > peon > backend_inference > process > unknown.

### 5. Write metadata

1. Merge inferred fields into `SessionMetadata`:
   - Only set fields that Peon inferred (non-None).
   - Always set `metadata_source` to `"peon"`.
   - Always set `metadata_confidence` to the inference confidence.
   - Preserve all other existing fields.
2. Write via `MetadataStore::write_session()`.
3. Append a Peon inference event to `.orkworks/events/<id>.ndjson`:
   ```json
   {"type": "peon.inference", "timestamp": "<iso>", "status": "<inferred status>", "confidence": 0.85}
   ```

### 6. Update in-memory state

After writing to disk, update the in-memory `SessionInfo` in the session registry so `GET /sessions` returns fresh metadata without a disk read on every poll.

## Configuration

All configuration via environment variables, read at `orkworksd` startup:

| Variable | Default | Purpose |
|----------|---------|---------|
| `PEON_ENABLED` | `true` | Enable/disable Peon entirely |
| `PEON_HARNESS` | `opencode` | Harness binary to invoke for inference |
| `PEON_HARNESS_ARGS` | `--print -p` | Arguments for one-shot non-interactive invocation |
| `PEON_MODEL` | — | Optional model override (passed as `--model` to harness) |
| `PEON_INTERVAL` | `5` | Debounce seconds after last output before inference |
| `PEON_MAX_LINES` | `200` | Ring buffer capacity per session |
| `PEON_TIMEOUT` | `30` | Subprocess timeout in seconds |

If `PEON_ENABLED` is `false`, the Peon tokio task is never spawned. The ring buffer collection still runs (low overhead), but inference is skipped.

No separate API key — the harness handles authentication with its own configuration.

## Frontend Changes

### Existing (already works)

- `metadataSource: "peon"` renders as a blue badge with confidence percentage in `LeftSidebar` and `RightSidebar`.
- `needsAttention()` in `RightSidebarHelpers.ts` flags `blocked`, `failed`, `waiting_for_input` with warning icons.
- The 2-second session poll picks up Peon-written metadata via `GET /sessions`.

### New additions

1. **Peon activity indicator** in `RightSidebar.tsx`: show when Peon last ran for the active session (e.g., "Peon observed 12s ago"). Backend provides `peon_last_inference` field in `SessionInfo`.

2. **`SessionInfo` type update** in `api.ts`: add optional `peonLastInference?: string` field.

No new components or panels needed.

## Error Handling

| Failure | Behavior |
|---------|----------|
| Harness binary not found | Log warning, disable Peon for that session, retry on next interval |
| Harness non-zero exit | Log warning with stderr, skip inference this cycle |
| Harness timeout (30s) | Kill subprocess, log warning, skip |
| Malformed JSON response | Log warning with raw output, discard result |
| Schema validation failure | Log warning, discard result |
| Metadata write failure | Log error, skip (non-fatal to session) |
| Priority skip (agent/user metadata present) | Debug log, skip silently |

All errors are non-fatal. Peon never crashes the server or kills a session.

## Testing Strategy

### Unit tests (`peon.rs`)

- **RingBuffer**: push, capacity enforcement, iteration order, empty state.
- **Priority logic**: test all source combinations (user, agent-fresh, agent-stale, peon, backend_inference, process, unknown, absent).
- **JSON extraction**: strip markdown fences, handle empty response, handle non-JSON.
- **Schema validation**: valid full response, valid partial response (only status), invalid status string, missing confidence, out-of-range confidence.

### Integration tests (`main.rs` tests module)

- **End-to-end**: set `PEON_HARNESS` to a test script that echoes known JSON, feed terminal output to a session's ring buffer, trigger inference, verify `.orkworks/sessions/<id>.json` is written with correct `metadataSource: "peon"` and correct fields.
- **Priority non-overwrite**: write agent metadata first, trigger Peon inference, verify agent metadata is preserved.
- **Disabled**: set `PEON_ENABLED=false`, verify inference is never triggered.

### Test harness

Use `tempfile` (already a dev dependency) for `.orkworks/` directories in integration tests. Mock the harness invocation by setting `PEON_HARNESS` to a test script that echoes known JSON.

## File Changes Summary

| File | Change |
|------|--------|
| `crates/orkworksd/src/peon.rs` | **New** — ring buffer, inference pipeline, harness invocation, validation |
| `crates/orkworksd/src/main.rs` | Add `mod peon`, add ring buffer to `SessionHandle`, wire PTY output → ring buffer, wire debounce → peon inference, add `peon_last_inference` to `SessionInfo` |
| `crates/orkworksd/src/metadata.rs` | Add `merge_peon_inference()` method to `MetadataStore` |
| `apps/desktop/src/api.ts` | Add `peonLastInference` to `SessionInfo` interface |
| `apps/desktop/src/components/RightSidebar.tsx` | Add Peon activity indicator |

## Non-goals (MVP)

- Peon does NOT type into terminals.
- Peon does NOT approve commands.
- Peon does NOT have a dedicated settings panel (env vars only).
- Peon does NOT support multiple harnesses simultaneously.
- Peon does NOT cache or batch inferences across sessions.
- Peon does NOT parse token usage or cost.
