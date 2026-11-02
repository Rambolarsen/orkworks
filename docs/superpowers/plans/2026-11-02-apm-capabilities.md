---
type: Implementation Plan
title: APM capability compatibility
description: Replace exact APM version admission with capability discovery and automated behavioral evidence.
tags: [apm, skills, desktop, compatibility]
status: stable
---

# APM capability compatibility

Owner approved the design on 2026-11-02. Tracks [#1300](https://github.com/Rambolarsen/orkworks/issues/1300).
Contract: [workspace skills](../../../specs/workspace-skills.md).

## Decision and boundaries

Electron main shares one probe between status and preview: read version for
information and install help for required flags (--target, --only, --dry-run,
--no-trust-bin, --global). Missing flags or failed probes disable reviewed adds
with a specific reason. No version allowlist or runtime test installs. Existing
source, destination, policy, confirmation, fingerprint and platform gates remain.
CLI declarations cannot establish behavioral safety; disposable real-APM checks
run on scheduled/manual CI against the latest stable release and on relevant PRs.
They cover workspace and personal scope, existing manifests and private preview
behavior. Failures report compatibility regressions; they do not remotely disable
or reconfigure installed OrkWorks copies.

## Deliverables and checks

1. Record the accepted compatibility decision and update the spec. Add failing
   tests: new versions with capabilities pass, old versions missing capabilities
   fail with named flags, errors block, preview uses the same contract.
2. Implement the shared probe in electron/skillInstall.ts and remove version-bound
   UI claims. Keep existing IPC types. Run desktop settings/install/UI fixtures.
3. Extend real source-CLI qualification to workspace and personal operations with
   process-local Path.home/expanduser redirection; retain real user environment. Automate
   latest-release resolution, detached upstream checkout and uv execution in CI.
   Verify the unmodified executable through the desktop service only on disposable
   CI runners (APM writes user-level lock/receipts even for workspace operations). Record tested release and limitations in validation evidence.
4. Run repository verification, documentation/worktree checks and current-diff
   /code-review low before PR/merge; follow existing branch cleanup procedure.

## Review

Ready. Shared probe prevents status/preview divergence. All behavioral tests use
disposable sources and destinations; no runtime automatic installation or trust.
Dependencies 3 (upstream CLI), blast radius 2 (reviewed adds), state changes 2
(existing bounded installs), reversibility 1 (revert; no migration), uncertainty 3
(help cannot prove semantics). Total 11. Rollback restores the version gate;
existing user installations require ordinary inspection, as before.

## Execution evidence

Implemented the approved shared capability probe and daily/manual/relevant-PR
workflow. APM 0.35.0 source fixtures passed workspace/personal fresh/existing
manifest and private preview checks. Desktop suite: 1,667 passed, 3 skipped;
UI layout fixture, type checks, native ABI check, desktop/docs builds and reporter
tests passed. `/code-review low` and independent fixture-completeness review
completed; both isolation findings were fixed and verified.

Full verify-repo stopped at Rust test
`runtime::codex_approval_application::sql_tests::native_sql_queued_effect_rechecks_every_runtime_boundary`
with RecvError (2,550 passed, 1 failed, 17 ignored). The exact test passed on an
isolated rerun. No Rust files changed; remaining verifier stages were run
separately. Documentation warnings prompted the coding-tools and README updates;
homepage onboarding makes no version-compatibility claim. The pre-commit worktree
warning calls the branch merged because it still points at its base; this is
not evidence of a merged PR, and the owned worktree remains until PR close-out.
