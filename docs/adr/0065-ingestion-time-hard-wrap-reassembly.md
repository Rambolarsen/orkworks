# Hard-wrap reassembly moves to PTY ingestion, before the shared output buffer

- Status: accepted
- Deciders: Rambolarsen, opencode
- Date: 2026-09-25

## Context

Harnesses such as Claude Code hard-wrap long output lines (paths, bulleted
commit lists) into multiple physical terminal rows separated by real `\n`
bytes. `drain_persist_records` splits the raw PTY byte stream on `\n`, so each
physical row became an independent record, and the `.trim()` before
`output_buffer.push` strips the leading indent that would otherwise hint that
a row is a wrap continuation. Downstream of `output_buffer` — status and
summary inference, password-prompt detection, capacity windowing, and
workflow-observation evidence grounding — a long logical line could appear as
several disconnected fragments.

Two narrower mitigations shipped first (PR #589's evidence-grounding guard and
PR #592's final-scan rejoin), both applying `peon::rejoin_hard_wrapped_lines`
to a local snapshot at read time. The shared `output_buffer` itself still
stored physical rows, so consumers that read the buffer directly
(`session_projection`'s capacity windowing, `looks_like_password_prompt`'s
`last_n(5)`) kept seeing wrap-mangled line boundaries. Issue #590 tracks the
ingestion-level fix.

## Decision

Reassemble hard-wrapped rows once, at PTY ingestion, before any line enters
the shared `output_buffer`:

1. The wrap signal stays the one already used at snapshot time: a captured row
   whose character count is at or beyond the PTY's current column width
   (`SessionRuntime::last_cols`) is treated as a wrapped row and concatenated
   directly onto the next captured row — no separator, since a hard wrap can
   split mid-word. Chaining is row-local: a row joins the accumulated line
   only while the most recent appended row itself filled the terminal width,
   so a short final continuation row ends the chain and the next logical line
   starts fresh. This deliberately avoids the snapshot helper's
   extended-length re-check, which would glue every row after the first
   full-width row into one line — acceptable on a throwaway read-time
   snapshot, but an over-join regression if it reshaped the shared buffer's
   line structure. As before, this is a best-effort heuristic, not a
   display-width-aware renderer (wide/CJK characters can throw the count off;
   a coincidentally full-width line followed by an unrelated row is joined) —
   the same documented limits as `rejoin_hard_wrapped_lines`.
2. Because output arrives in per-read chunks, a full-width row can be the last
   row of one chunk with its continuation in the next. Reassembly therefore
   keeps one per-session held row (`pending_wrap_prefix` on `SessionRuntime`)
   when a chunk ends on a full-width row, and prepends it to the next chunk's
   rows. The held row is flushed into `output_buffer` in
   `handle_runtime_exit`, the single choke point through which every
   ended/killed/errored runtime passes before finalization, so the final scan
   always sees the complete tail.
3. Raw physical rows are unchanged everywhere else: the append-only
   `events/<id>.terminal` history, replay persistence, and the raw-text
   `scan_buf` used by usage-limit scanning keep physical rows. Only the
   logical view consumed through `output_buffer` changes.
4. The read-time snapshot rejoins (Peon inference cycle, final scan) remain
   in place for content that reaches them without passing through ingestion
   reassembly. They cannot distinguish a reassembled logical line from a
   genuine full-width physical row: on a reassembled buffer they still glue
   the successor whenever a line is at or beyond the terminal width, exactly
   as they did before this change. The ingestion path is where reassembly is
   guaranteed; read-time rejoin is a fallback, not a second guarantee, and
   the buffers it mostly no longer helps are those built before this change.

## Consequences

Every `output_buffer` consumer now sees logical lines for harness wraps: line
counts in the buffer no longer match physical terminal rows. Read-time
snapshot rejoin cannot distinguish a reassembled logical line from a genuine
full-width physical row (see Decision point 4), so its gluing behavior over
reassembled content is unchanged from before this change — ingestion, not
read time, is where reassembly is guaranteed. Consumers that
pattern-match within a line (`looks_like_password_prompt`'s `contains`-based
check) are unaffected; consumers that count lines must treat a logical line as
one entry. The brief window where the newest full-width row is held in
`pending_wrap_prefix` rather than visible in `output_buffer` ends at the next
output chunk or runtime exit, whichever comes first. Evidence-grounding and
plan-path fallback operating on `raw_persist_lines` keep their raw-text
semantics.

Resize-with-held-row is an accepted one-join-per-resize trade: a row held
under the old column width has its continuation judged under the new width
(the held row itself is treated as full width regardless of the resize), so a
resize between the wrap and its continuation can glue one non-continuation
row. Raw history is unaffected; the heuristic-error bound is one wrong join
per resize, consistent with the best-effort signal this decision keeps.

Line-count bookkeeping deliberately stays physical: `output_lines_seen`
counts persisted rows, not the (fewer) logical lines pushed into
`output_buffer`, and the exit flush's held row is not counted there either.
Fresh-window arithmetic in `session_projection` that mixes the two counters
therefore over-estimates by the number of joins and skipped empty rows within
a window — a drift class that pre-existed via empty-row skips, backstopped by
the raw-text usage-limit scan path.
