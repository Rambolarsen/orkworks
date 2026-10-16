---
type: Plan
title: Codex API-only metadata producer
description: Prepare the source-owned producer and instruction path for the approved metadata protocol.
tags: [codex, metadata, runtime]
status: draft
---

# Codex API-only metadata producer

Implements the local producer work in [#789](https://github.com/Rambolarsen/orkworks/issues/789)
under the [approved metadata design](../specs/2026-10-08-codex-native-attention-layer-design.md).
The pass condition is an authenticated GET, validation-only PATCH, and follow-up
GET with the same revision before any agent PATCH, without direct-file fallback or blind conflict retry.
This does not enable native clearing or resolve #690.

## Delivery

- [x] Add the source-owned Python standard-library helper in
  `crates/orkworksd/scripts/session-metadata.py`. Commands provide bootstrap,
  read, revisioned patch, and Codex startup instructions. Require the exact
  protocol marker, live session environment and bounded loopback-only requests.
  Reject redirects, malformed responses, unsupported markers and failed
  bootstrap. Preserve caller revisions and report conflicts for reread.
- [x] Deliver the conditional API-only instructions through the existing Codex
  SessionStart reporter context. Reconcile the helper with reporter assets;
  preserve non-marked launches, hook trust, identity and attention mappings.
  Existing hook attention reporters already write through the serialized
  sidecar boundary; never convert their provenance to direct-agent patches.
- [x] Declare the closed source-owned definition capability without allowing
  custom definitions or overrides to opt into it. Keep the embedded capability
  absent and the launch marker disabled until complete producer/platform and
  actual agent-context evidence qualifies the migration. No native gate changes.
- [x] Exercise failures and exact HTTP traffic with behavioral helper tests,
  then run the helper against the real authenticated test router. Check
  validation leaves bytes/revision/timestamps unchanged; accepted identical
  attention writes revoke ownership; conflict/failure never changes files.
- [ ] Run sidecar/build/fmt, verify-repo, doc and worktree checks; review the
  current diff and deliver a PR. Keep #789 open until installed sandbox and
  generated instruction delivery evidence passes.

## Adapter boundaries

Codex adapter and harness ID remain `codex`. Direct launch remains `codex`;
exact resume remains `codex resume {harnessSessionId}` with the existing
exact-ID checks. Native identity comes from authenticated `codex_hook` reports
at confidence 0.98. Existing `/clear` and `/new`, model, voice and capacity
contracts are unaffected. Hook installation/trust remains user-approved.
No client, agent or second coding harness is launched for verification.

## Review

Ready for this local preparation slice. Dependencies 2 (existing sidecar API
and reporter asset resolver), blast radius 2 (marker-gated new path), state
changes 2 (existing API only), reversibility 1 (marker remains absent),
uncertainty 3 (installed agent-context evidence remains outstanding); total 10.
Full #789 and attention clearing remain gated on missing installed evidence.
The original hook-completion proposal also needs coordinated writers and a
separately approved non-native eligibility extension; this PR does neither.

## Retained validation and limitations

The helper's behavioral suite and real-router tests passed locally on macOS
arm64 with Python 3.9 and repository Node 22. The first full verification run
passed Rust build, 1,983 Rust tests (five ignored), reporter/producer integration,
OpenCode reporter tests, desktop type-check, native ABI repair, 1,320 desktop
tests (one skipped), and desktop build. Docs validation found a missing index
description; the entry was corrected and docs checks rerun separately.
The later installer-generated command regression also passed with spaces and an
apostrophe in the helper path. The separate code review found no required
changes and noted missing direct PowerShell instruction-path coverage; that
platform remains unqualified. This is test-router/source evidence, not an
installed agent sandbox handshake. #789 and #690 remain open.

## Review cycle 1 of 3

Codex's automated review identified two required asset-boundary fixes: include
the helper in integration currentness and in packaged/installed artifact gates.
Both were reproduced by failing regressions. The currentness check now consumes
additional assets declared by the existing hook contract, with Codex declaring
its helper and other handlers declaring none. The release script list is shared
with Windows installed expectations. Thirty-five artifact tests and the helper
drift regression pass; the scoped fresh review found no required changes.
Full verification is rerun for the corrected code before the next push.

## Review cycle 2 of 3

The current-head Codex review identified a missing follow-up GET in bootstrap
and ASCII escaping that could inflate valid UTF-8 patches over the byte limit.
Both were reproduced by failing behavioral regressions. Bootstrap now verifies
the revision again through GET before reporting success or mutating; request
serialization preserves UTF-8 before enforcing the network byte limit.
All fourteen producer behavioral cases pass. Full verification and the
current-session medium review are repeated on the corrected code before merge.
