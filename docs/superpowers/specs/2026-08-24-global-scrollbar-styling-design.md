# Global Scrollbar Styling Design

## Goal

Make native scrollbars in the desktop renderer blend into OrkWorks' dark UI instead of appearing as bright, platform-default controls.

## Design

Add one shared rule set to `apps/desktop/src/App.css`. Chromium/Electron scrollbar pseudo-elements provide the visual treatment used by the app, while `scrollbar-width` and `scrollbar-color` provide a standards-based fallback. The track remains transparent, the thumb is narrow, rounded, and muted, and hover raises contrast slightly.

This is appearance-only. It does not replace native scrolling, change overflow behavior, add dependencies, or alter terminal scrolling.

## Verification

Run the desktop TypeScript check and test suite, then run the repository doc-currency and worktree-currency checks.
