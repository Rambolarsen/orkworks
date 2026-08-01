# Session Plan Review Design

The Details panel owns a session's review request. A single Review tab, adjacent to Terminal, is only the reader; it follows the selected session's validated Markdown artifact. The card is available for every session with an artifact, with urgency-specific copy only when the session needs the user.

The review action is a deliberately narrow exception to the terminal-input non-goal: an explicit user click sends one sidecar-owned review prompt to one live session and submits it. Electron main authenticates the call; the renderer sends only the session ID. The sidecar validates the persisted, relative Markdown path immediately before writing to the session's existing PTY input channel and appends an audit event. There is no generic text API, prompt editor, auto-send, repo inbox, watcher, digest, or new-session flow.

When a harness reports `planPath`, it remains the association source. To cover existing agents that only announce their artifact in terminal output, a conservative fallback accepts an existing path printed beneath `docs/superpowers/plans/` or `specs/`; it is still validated by the same workspace-boundary helper before display or injection.
