---
type: Implementation Plan
title: New-session image context implementation
description: Deliver the approved paste, drop, and picker attachment contract through ordinary session creation.
tags: [orkworks, sessions, images]
status: draft
workflow_status: active
---

# New-session image context implementation

Owner: this session, branch session-image-context. Issue [#979](https://github.com/Rambolarsen/orkworks/issues/979), PR [#982](https://github.com/Rambolarsen/orkworks/pull/982).
Spec: [approved image context](../../../specs/session-image-context.md).

## Delivery steps

- [ ] Sidecar: add an image-context module for bounded real decoding, private bundles,
  publication/rollback/cleanup, and compiled delivery support. Extend authenticated
  session creation additively and advertise explicit support; capture workspace
  ownership across asynchronous preparation. Tests first for validation, authorization,
  storage, and text-only compatibility. Use image crate with only PNG/JPEG/WebP
  decoders: the existing Rust dependencies have no real image decoder.
- [ ] Desktop: a renderer ingestion module owns bounded metadata inspection and
  sequential preview generation; NewSessionDialog shares the attachment list across
  paste/drop/picker. Main/preload expose only bounded creation data, use the existing
  backend-generation request guard, and positively negotiate sidecar capability.
  Tests first for batch limits, invalid content, lifecycle races, and transport errors.
- [ ] Qualification: verify the real model sees distinctive visual content through
  each enabled tool. Keep unverified routes disabled. Run repo verification, UI
  checks, and `/code-review low` on the current code diff; update the same PR to
  reflect implementation, inventory all reviews/checks, then merge only after gates.

## Review and ledger

Ready for execution: reviewed inline against the approved spec. Coupled steps share
one creation contract; no independent writers. Mandatory input methods, ownership,
compatibility, cleanup, and real delivery checks are covered. Apply the spec's risk
assessment; no materially changed architecture. Native CLI help alone cannot enable
support. Failures preserve drafts; accepted sessions retain complete image bundles.

- 2026-10-23: user approved contract; runtime work authorized. TDD begins with
  sidecar validation/storage and browser ingestion behavior. No harness enabled yet.

- Ruling: Codex native image startup owns the entire first turn, with text passed
  as a literal argv positional and no queued PTY prompt. Live qualification showed
  `--image` alone auto-submits before readiness-gated text, causing two turns.
  Labels/work signals still derive from the same text. This implements the
  approved exactly-once outcome through the native delivery policy.

- Qualification: Codex 0.163.0 on macOS arm64, configured GPT-6.1-Sol, identified
  the four fixture colors in order with one native image/text turn, and described
  the image-only fixture. Paths included spaces and non-ASCII characters.
- Claude Code 2.1.292 did not qualify: an external CLAUDE.md-import approval chooser
  prevented prompt delivery. No chooser was automated or global setting changed.
  Claude, OpenCode, Copilot, and other platforms remain disabled for this increment.

- Verification ledger: Rust full suite passed (2161 unit tests, 10 ignored;
  integration suites 1/4/1 passed). After removing the unqualified Claude row,
  all eight focused image tests passed. All 47 existing Electron shell scenarios
  passed. The first full desktop run exposed a stale cancellation source assertion;
  the shared cancellation generation now preserves that contract. Added a keyboard
  regression that caught Enter intercepting attachment buttons; 58 focused UI/shell
  checks passed after the fix. Native ABI was restored before the final full
  desktop run, since build:native and Node tests must run sequentially.
- Documentation warnings: sessions guide and README now describe source-only image
  support and its qualified tool/platform limits. docs/home.md has no conflicting
  attachment claim. Root AGENTS needs no update for a private decoder dependency;
  this change introduces no universal policy or routing entry.
