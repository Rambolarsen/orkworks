# Terminal replay size design

## Goal

Stop dead-session terminal replay from garbling wrapped lines. Replay currently re-wraps saved output to whatever width the panel happens to be today, splitting words mid-character wherever that differs from the width the session was actually recorded at.

## Scope

- Persist the PTY's last known column/row count once a session reaches a terminal status (`killed`/`ended`/`error`).
- Serve that size alongside the existing terminal-output replay payload.
- Render dead-session replay at the recorded size (no re-wrap), scaled down visually to fit the panel.
- Legacy sessions with no recorded size keep today's fit-to-container behavior unchanged.
- No mid-session size history, no per-line size tags, no new dependencies.

## Design

**Sidecar (`crates/orkworksd`):** `SessionMetadata` gains `pty_cols`/`pty_rows: Option<u16>` (serde default `None`, so existing on-disk metadata deserializes unchanged). `set_session_status` (`runtime/terminal_runtime.rs`) already has both the in-memory `SessionHandle` (whose `runtime` tracks `last_cols`/`last_rows`) and the persisted `SessionMetadata` in scope at the moment it transitions a session into `killed`/`ended`/`error`; it stamps the handle's last known size onto the metadata before writing it. This is the only write path — resize events during a live session stay in-memory only, since replay only ever needs the final size.

`get_terminal_output` (`runtime/terminal_http.rs`) reads the session's `pty_cols`/`pty_rows` alongside the existing terminal-output lines and includes them (when present) in `TerminalOutputResponse` as optional `cols`/`rows`.

**Desktop (`apps/desktop/src`):** `getTerminalOutput` (`api.ts`) returns the new optional `cols`/`rows` fields. `HistoricalTerminal.tsx` branches on their presence:

- **Recorded size known:** construct the xterm `Terminal` with `cols`/`rows` fixed to the recorded values (no `FitAddon`/`ResizeObserver`-driven reflow — the buffer's wrap points are set once and never recomputed). After the terminal renders, measure its natural pixel size against the container and apply `transform: scale(min(1, containerWidth / naturalWidth, containerHeight / naturalHeight))` with `transform-origin: top left`, and set the container's height to the scaled height so the panel doesn't leave a gap below it. Recompute the scale (not the terminal size) on `ResizeObserver` — cheap, since it's just a CSS transform, not a re-render.
- **Recorded size absent (legacy sessions):** keep today's exact behavior — `FitAddon.fit()` against the live container, `ResizeObserver`-driven refit. No behavior change for old data.

The scale-factor computation is a small pure function so it can be unit tested without mounting xterm.

## Error handling

- Missing or `null` `cols`/`rows` in the response → legacy fallback path, unchanged from current behavior.
- A `pty_cols`/`pty_rows` of `0` (shouldn't happen given `MIN_PTY_COLS`/`MIN_PTY_ROWS` floors in `terminalSize.ts`, but validated defensively) is treated the same as absent.

## Verification

- Rust: unit test that `set_session_status` stamps `pty_cols`/`pty_rows` from the runtime handle onto persisted metadata when entering `killed`/`ended`/`error`, and that they're included in the `get_terminal_output` JSON response. Existing delimiter-preservation tests (from #259) are unaffected.
- TypeScript: unit test for the scale-factor pure function (width-bound, height-bound, and no-scale-needed cases). Existing `terminalReplay.test.ts` coverage untouched.
- Manual: replay a dead session recorded at a wide terminal in a narrower detail panel and confirm lines no longer split mid-word.
