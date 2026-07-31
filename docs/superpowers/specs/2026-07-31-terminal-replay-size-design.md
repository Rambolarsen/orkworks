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

**Sidecar (`crates/orkworksd`):** `SessionMetadata` gains `pty_cols`/`pty_rows: Option<u16>`, following the existing pattern for optional fields on that struct (e.g. `harness_session_id_source`): `#[serde(rename = "ptyCols"/"ptyRows", skip_serializing_if = "Option::is_none")]`, no explicit `default` needed — serde already treats missing `Option` fields as `None` on deserialize, so existing on-disk metadata is unaffected.

`set_session_status` (`runtime/terminal_runtime.rs`) does *not* currently carry the in-memory `SessionHandle` state to the point where it writes `SessionMetadata` — it drops the `sessions.lock()` guard first, threading only a small captured tuple (`handle_decision`, `session_resume`, `entered_running`, `entered_terminal`) across to the later `workspace.lock()` block. This needs one more field in that tuple: capture `handle.runtime`'s last known `(cols, rows)` the same way `session_resume` is captured today, then in the terminal-status branch of the metadata-write block, stamp it onto `meta.pty_cols`/`meta.pty_rows` before `write_session`. In the "no in-memory handle" path further down that function (persisted-lifecycle-guard, used when the handle was already removed from `state.sessions`), there is no size to capture — `pty_cols`/`pty_rows` are simply left as whatever was already persisted (typically `None`), which is an acceptable degrade to the legacy fallback for that edge case.

`get_terminal_output` (`runtime/terminal_http.rs`) reads the session's `pty_cols`/`pty_rows` alongside the existing terminal-output lines and includes them (when present) in `TerminalOutputResponse` as optional `cols`/`rows`.

**Desktop (`apps/desktop/src`):** `getTerminalOutput` (`api.ts`) returns the new optional `cols`/`rows` fields. `HistoricalTerminal.tsx` branches on their presence:

- **Recorded size known:** construct the xterm `Terminal` with `cols`/`rows` fixed to the recorded values (no `FitAddon`/`ResizeObserver`-driven reflow — the buffer's wrap points are set once and never recomputed). The shared `.terminal-container .xterm { width: 100%; height: 100%; }` rule in `App.css` (used by the live terminal in `CenterPanel.tsx`) would force the root element to fill the container and defeat measurement, so this path renders into a container carrying an additional modifier class (e.g. `terminal-container--fixed-size`) whose CSS resets `.xterm` back to its intrinsic content size (`width: max-content; height: max-content;`) for that case only — the live-terminal path is untouched. Once xterm's public `onRender` event fires (first paint), measure the `.xterm` element's natural `getBoundingClientRect()` once and cache it, then apply `transform: scale(min(1, containerWidth / naturalWidth, containerHeight / naturalHeight))` with `transform-origin: top left` directly on that element. `.terminal-container` already fills the panel via the existing `flex: 1` / `overflow: hidden` rules (it is not sized to content), so no extra height bookkeeping is needed — the transform just visually shrinks the element inside the box that's already there, and `overflow: hidden` clips any remainder. A `ResizeObserver` on the container recomputes only the scale ratio against the cached natural size on resize — cheap, since it's a CSS transform, not a re-render.
- **Recorded size absent (legacy sessions):** keep today's exact behavior — `FitAddon.fit()` against the live container, `ResizeObserver`-driven refit. No behavior change for old data.

The scale-factor computation is a small pure function so it can be unit tested without mounting xterm.

## Error handling

- Missing or `null` `cols`/`rows` in the response → legacy fallback path, unchanged from current behavior.
- A `pty_cols`/`pty_rows` of `0` (shouldn't happen given `MIN_PTY_COLS`/`MIN_PTY_ROWS` floors in `terminalSize.ts`, but validated defensively) is treated the same as absent.

## Verification

- Rust: unit test that `set_session_status` stamps `pty_cols`/`pty_rows` from the runtime handle onto persisted metadata when entering `killed`/`ended`/`error`, and that they're included in the `get_terminal_output` JSON response. Existing delimiter-preservation tests (from #259) are unaffected.
- TypeScript: unit test for the scale-factor pure function (width-bound, height-bound, and no-scale-needed cases). Existing `terminalReplay.test.ts` coverage untouched.
- Manual: replay a dead session recorded at a wide terminal in a narrower detail panel and confirm lines no longer split mid-word.
