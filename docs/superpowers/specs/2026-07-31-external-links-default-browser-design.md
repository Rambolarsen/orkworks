# External Links in Default Browser Design

## Goal

Open web links from OrkWorks in the operating system's default browser instead of an Electron-owned window.

## Design

Configure the existing Electron main window's popup handler. It will reject each popup request so Electron creates no child window, and send only `http:` and `https:` URLs to Electron's native `shell.openExternal` API. The renderer, preload bridge, sidecar, and local plan-opening flow remain unchanged.

## Error handling

Malformed or non-web URLs are rejected. Failures from the OS handoff are reported to Electron's console; no retry or user setting is added.

## Validation

Add a focused Electron-main unit test that proves web URLs are delegated and all requests are denied, then run the desktop type-check and test suite.

## Non-goals

- No in-app browser, link preference, protocol handler, or new dependency.
- No changes to local file/plan opening.
