# Windows dialog drag review fix

Fix issue #535 and the late review finding on PR #532 within the approved Windows shell dragging requirement.

- [x] Reproduce the missing strip in an Electron layout test using real CSS.
- [x] Add a Windows-only drag strip to shared modal backdrops and the React error screen; keep underlying workspace actions blocked.
- [x] Bound tall session dialogs below the strip with scrolling.
- [ ] Verify the regression test, desktop build/types, and diff-scoped low review.
- [ ] Open a follow-up PR, wait for CI and automated reviews, merge if clean, then resolve the original thread.

Validation: the real Electron CSS regression failed before the fix (missing settings-backdrop drag strip) and passes afterward across all three overlay classes and Windows/macOS/Linux selectors. Header and recovery tests also pass (4 tests total). Renderer type checks and production build pass. Manual mouse-driven dragging is not automated by this layout test.
