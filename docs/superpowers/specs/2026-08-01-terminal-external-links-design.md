# Terminal External Links Design

## Goal

Open HTTP(S) links clicked in an xterm terminal in the operating system's default browser.

## Design

The terminal renderer calls a narrow `openExternalLink(url)` preload bridge instead of `window.open()`. Electron main reuses the existing HTTP(S) validator and delegates accepted URLs to `shell.openExternal`. The global popup/navigation deny handler remains in place for all other renderer navigation.

## Error handling

Malformed and non-web URLs are rejected. Failed OS handoffs are logged without creating an unhandled rejection.

## Validation

Add a focused test for the preload/main handoff and run the desktop type-check and test suite.

## Non-goals

No in-app browser, link preference, new dependency, or change to local file/plan opening.
