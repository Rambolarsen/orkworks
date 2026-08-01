# Terminal External Links Design

## Goal

Open HTTP(S) links clicked in an xterm terminal in the operating system's default browser.

## Design

Both live and historical terminals configure xterm's OSC-8 `linkHandler` to call a narrow `openExternalLink(url)` preload bridge instead of `window.open()`. Electron main accepts an untrusted IPC value, reuses the existing HTTP(S) validator, and delegates accepted URLs to `shell.openExternal`. The global popup/navigation deny handler remains in place for all other renderer navigation.

## Error handling

Malformed and non-web URLs are rejected. Failed OS handoffs are logged without creating an unhandled rejection.

## Validation

Add focused tests that terminal link activation forwards the exact URL, invalid IPC values and non-web URLs do not reach the OS, and rejected handoffs are caught. Run the desktop type-check and test suite.

## Non-goals

No in-app browser, link preference, new dependency, or change to local file/plan opening.
