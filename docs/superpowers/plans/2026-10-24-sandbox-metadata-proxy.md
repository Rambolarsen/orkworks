---
type: Plan
title: Explicit metadata HTTP proxy implementation
description: Implement and verify the approved opt-in sandbox proxy transport for the metadata helper.
tags: [codex, metadata, sandbox, security]
status: draft
---

# Explicit metadata HTTP proxy implementation

Deliver the owner-approved [design](../specs/2026-10-24-sandbox-metadata-proxy-design.md)
under [#789](https://github.com/Rambolarsen/orkworks/issues/789). Pass condition:
an explicitly selected trusted local proxy carries the existing authenticated
metadata bootstrap and revisioned patches; ambient proxies, redirects, failures
and malformed arguments cannot redirect credentials or trigger fallback.

## Delivery

- [x] Extend the real subprocess/helper tests in
  `crates/orkworksd/scripts/session-metadata.test.py`: successful proxy bootstrap
  and patch with fixed absolute-form destination; invalid URLs before contact;
  redirect, conflict, corrupt/wrong-session responses and unavailable proxy with
  no direct/file fallback. Keep direct and ambient-proxy behavior covered.
  Observe a failing success-path regression before implementation.
- [x] Add optional `--http-proxy URL` parsing to `session-metadata.py` for
  bootstrap/get/patch. Use a strict whole-string loopback URL grammar; keep
  existing direct calls and instruction output unchanged. With an explicit
  proxy, connect only to that relay and use the fixed sidecar origin. No
  ambient discovery, fallback, redirect following, launch markers or callers.
- [x] Record approved contract and usage in the runtime/harness references and
  ADR 0076; keep capability/rollout gates intact. Verify source helper using the
  actual managed proxy in the ordinary sandbox with validation-only bootstrap.
- [x] Run focused tests, full verify-repo and close-out checks; obtain a fresh
  current-diff review.
- [ ] Open the PR and complete its bounded lifecycle.

## Review and risk treatment

Ready. Dependencies 2 (existing helper/API and caller-selected proxy); blast
radius 2 (explicit opt-in transport); state changes 1 (no schema or launch
changes); reversibility 1 (omit the option/revert); uncertainty 2 (actual proxy
bootstrap already demonstrated, durable implementation still needs tests).
Total 8. Security concern: the explicit relay observes the capability; user
approval explicitly accepts that trust relationship. Loopback is validation,
not identity proof. Invalid input never echoes the supplied URL or credentials.

Plan reviewed against the approved document: covers each preserved transport
invariant and rejects the automatic-caller migration the builder check flagged.
Ruling: use one coupled implementation task; do not split the CLI parser from
its transport/security tests or add a new network abstraction/dependency.

## Evidence ledger

The explicit-proxy bootstrap/get/patch regression failed on the missing CLI
option before implementation. All 20 helper tests now pass, including direct
compatibility, 18 invalid proxy spellings, redirect/no-fallback and revision
checks. The actual ordinary sandbox executed the new source helper through its
known Codex-managed proxy: GET/validation-only PATCH/follow-up GET passed.
The installed helper has not been reconciled yet; source execution does not
qualify an installed distribution. `bash scripts/verify-repo.sh` passed formatting, build, all 2,180 Rust unit
tests, six integration tests, OpenCode reporter tests, desktop type-check,
native tests, desktop tests (1,386 passed, one skipped) and desktop build. It
then caught an unescaped angle-bracket placeholder in this design’s Markdown.
After that docs-only correction, the separate docs metadata/build passed.
Diff check, doc-check and worktree-check passed. The README currency warning
needs no edit because no advertised capability or milestone changes. The
worktree report’s own-branch warning preceded the first commit; foreign
worktrees are left with their owners.
No automatic caller, marker, sandbox configuration or completion row changed.

The medium security-focused current-diff review approved the implementation
without actionable findings. Reviewed code digest:
`2160f3e657c14e681689ca16dff07d7eb7d81de82933d3ed982c85398c229062`
against base `fc19f0617a21da5651e32d925286a32cf50d7b08`. The reviewer inspected
tests but did not run them; execution evidence above is from the owning session.
