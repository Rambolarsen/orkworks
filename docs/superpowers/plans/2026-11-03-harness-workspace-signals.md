---
type: Implementation Plan
title: Qualify shared harness workspace signals
description: Deliver a bounded location-only probe and evidence before enabling new native workspace authority.
tags: [harnesses, sessions, git, qualification]
status: draft
---

# Qualify shared harness workspace signals

Owner approved the [design](../specs/2026-11-03-harness-workspace-signals-design.md)
on 2026-11-03. Tracking: [#1347](https://github.com/Rambolarsen/orkworks/issues/1347).
Execute inline in the owning `harness-workspace-signals` worktree.

## Deliverables and boundaries

- [x] Add `scripts/qualify-workspace-signals.py` and behavioral tests. The probe
  consumes one bounded hook/plugin JSON object from stdin and emits only fixed
  field names and classifications against supplied A/B/C roots. It never writes
  session metadata, invokes a harness, installs hooks, or logs raw strings.
  Session ID comparison is an optional equality check against the separately
  supplied `ORKWORKS_QUALIFY_NATIVE_SESSION_ID`; equality does not prove root
  identity. Unknown schemas remain unknown. Output is explicitly diagnostic.
- [x] Record a per-harness qualification table and owner-run fixture procedure in
  the approved design. Pin primary schema evidence separately from live payload
  and installed-version evidence. Preserve current capture-only attention gates.
- [x] Enable no new native location source without a demonstrated root-session,
  directory and ordering contract. Implement only independently qualified sources;
  otherwise deliver the probe and specific remaining evidence requirement, leaving
  existing explicit reports and labeled fallbacks intact.

## Checks

Write and run failing behavioral tests before the probe implementation. Exercise
actual subprocess stdin/stdout, A/B/C and symlink/subdirectory classification,
fixed output with secrets in ignored payload fields, child/session ambiguity,
unknown and unsupported harness schemas, malformed/duplicate/oversized input,
invalid roots and bounded output. Every successful output remains diagnostic.
Run Python unittest discovery for repository script tests and the existing helper
suite where executable. Add the probe suite to an existing scripts CI route so
it continues to run. Run repository verification after implementation and record
surface/environment failures rather than claiming full success. Run docs checks,
docs build, doc currency and worktree currency before handoff.

## Risks and recovery

No live harness launch is authorized from this session by repository policy.
A user-run fixture or separately authorized session is required for unobserved
native behavior. Do not install a live capture hook or change trust automatically.
The disposable probe reads at most 64 KiB and returns only known field names,
fixed directory labels and booleans. No network or metadata writes occur.
Removing the probe has no runtime effect. Dependencies 1, blast radius 1, state
changes 1, reversibility 1, uncertainty 3 (external signal semantics); total 7/25
for this evidence tooling only. Native activation retains the design's higher
uncertainty gate.

## Review

Ready for evidence tooling. Scope, privacy, unknown-schema behavior, test coverage
and recovery reviewed inline on 2026-11-03. No new runtime authority is planned
until the evidence checks pass; owner architecture approval is already recorded.

## Execution evidence

The 11 subprocess probe tests failed before implementation (missing script) and
passed after implementation. The fixture adds no dependency and uses the same
fixed output policy for all harness labels. Native activation was deferred
because no new live directory/root/ordering source was qualified; this satisfies
the explicit activation boundary, not the original runtime bug's resolution.

Verification on 2026-11-03: probe suite 11/11; existing Python script discovery
30/30 (the dotted probe filename is run separately); existing metadata helper
42/42 after permitting its synthetic loopback server; documentation checks
1,072/1,072 and VitePress build passed. The full repository verifier stopped at
its Node prerequisite: this tool context has Node 26.13.0 rather than pinned
Node 22. No product Rust or desktop code changed. Build emitted the existing
large/circular chunk warnings; no changed bundling claim depends on them.


## Approved delivery follow-up

After the owner authorized continuing with disposable qualification sessions,
Claude 2.1.309 verified native directory semantics and actual startup-context
delivery. A bounded source change extends the existing worktree-only context to
Claude/Copilot SessionStart through their documented envelopes on POSIX and
PowerShell. No new metadata protocol, location authority or persisted schema is
added. Codex general metadata gates remain unchanged. See the
[qualification record](../../validation/harness-workspace-signals.md).

Affected files are `report-harness-event.sh`, `report-harness-event.ps1`,
`session-metadata.test.py` and the cross-platform CI route. The new POSIX
behavior test failed specifically because Claude/Copilot emitted no context,
then passed after delivery was extended. Native PowerShell behavior is routed
to Windows CI and skipped locally. Scope/risk review: this is the design's
existing instruction-delivery step, with unchanged binding semantics; native
automation remains deferred. Review disposition: Ready for code review and
platform verification, without another architecture approval.


Follow-up scoped checks: the complete metadata helper suite passes 45 tests,
with the native Windows test skipped on macOS; the POSIX reporter suite and
11 probe regressions pass. Pinned Node 22 was found under the local Homebrew
installation, locked desktop dependencies were installed in the owned worktree,
and the full repository verifier is now running with the pinned toolchain.


Documentation currency warning reviewed: README's existing worktree paragraph
already describes generic cooperative reports, source precedence and unchanged
launch/resume cwd correctly. The delivery detail is recorded in the canonical
integration contract and validation record, so no README claim needs changing.
