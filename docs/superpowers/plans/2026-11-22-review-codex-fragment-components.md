---
type: Implementation Plan
title: Codex owned-fragment source components
description: Approved bounded shared command renderer and strict pure owned-fragment projector with canonical bytes and manual compatibility checks.
tags: [orkworks, review, codex, implementation]
status: stable
---

# Codex owned-fragment source components

**Workflow: owner approved the concrete design/plan on 2026-11-22 with “go ahead”
after draft #1959. Source implementation and focused compatibility checks are complete; full repository verification passed; final review is pending.**

Goal: derive only the exact standard POSIX OrkWorks hook bundle from bounded
shared JSON, using actual source command generation. Pass when canonical bytes
and ambiguity denials are proved while manual integration behavior is preserved.

Spec: [concrete design](../specs/2026-11-22-review-codex-fragment-design.md).
Track [#1956](https://github.com/Rambolarsen/orkworks/issues/1956), under
#68/#66/#1899 and #738/#741. Execute inline with TDD and one fresh final reviewer;
there is no need for concurrent writers.

## Owned files and interfaces

- Modify `crates/orkworksd/src/harness/integrations/mod.rs`: factor the existing
  pure portable-command formatting from its home-resolution wrapper, without
  changing other adapters’ behavior.
- Modify `.../integrations/codex.rs`: share pure event/bundle/fingerprint rendering
  between existing manual generation and the standard POSIX template. Keep
  probe/merge/remove classification and file/existence checks compatible.
- Create `.../integrations/codex_fragment.rs`: private source-only projector,
  payload-free error enum, opaque owned bytes and canonical serialization.
  Register it privately alongside the Codex module; no public route or active
  capture caller. A narrowly documented dead-code allowance for this dormant
  boundary is a builder detail, not permission to wire an unqualified consumer.
- Tests live beside their implementation or in a dedicated adjacent test module;
  keep independently specified expected byte data out of implementation helpers.
  Update the design/plan evidence and indexes at close-out.

Material projector interface is exactly the design’s bytes-to-Result/Option
signature; `OwnedCodexFragment::as_bytes(&self) -> &[u8]` cannot construct a value.
Keep template types internal. No interface accepts caller-supplied expected
commands/markers/platform or eligibility booleans. Reuse
`harness::definition::parse_strict_json` with the 256 KiB input bound and map
all its detailed diagnostics to fixed payload-free categories.

## One coupled implementation task

- [x] Write failing independent template/fingerprint byte fixtures for actual
  manual POSIX and Windows generation, then extract the pure formatter/bundle
  seams and run the existing Codex/Antigravity integration conformance tests.
  Preserve six-event fingerprint order, per-command LF, quoting and current
  Windows flags. No native tool, user file or installed asset is needed.
- [x] Write failing projector cases for exact complete output, None for no
  namespace, malformed/duplicate/trailing input, partial/duplicate/foreign owned
  groups, unknown event/root/key occurrences, extra hook/group keys and sibling
  hooks. Independent sentinels in unrelated data must be absent from successful
  output and every error; avoid deriving expectations from the renderer under test.
- [x] Implement the pure projector per the design: strict bounded parse, namespace
  scan, exact internally generated whole-group equality, six unique matches,
  reconstructed canonical output and fixed error categories. Validate every
  decoded namespace occurrence rather than accepting a projected subset. Keep
  physical absence/currentness and native validity outside this result.
- [x] Exercise inclusive 256 KiB carrier input and one extra byte, UTF-8/escaped
  duplicate keys, wrong fingerprint, namespace text in ignored-shaped data and
  unrelated groups around valid owned groups. Assert output equals the independent
  full literal and fits the 16 KiB cap; no artificial caller-selectable maximum
  template. Run focused tests and existing manual install/status/uninstall,
  fingerprint, portable command, Windows fixture and Antigravity regressions.
- [x] Run `bash scripts/verify-repo.sh` with pinned Node/pnpm setup; run doc/worktree/
  whitespace close-out checks. Inspect/restore generated lock metadata only.
  Record actual counts and classify warnings against this diff. No source-only
  result may be described as a complete capture owner or native proof.
- [x] Obtain fresh whole-diff review plus current `/code-review medium` before merge
  and handle findings through focused tests. Track the normal PR lifecycle in #1959.
  Keep source/native/physical/Start parent gates open and tie off only an actually
  matching active recommendation.

## Review focus

Review a duplicate hidden by ordinary JSON parsing; an owned command in an unknown
location omitted by a six-event loop; extra native executable fields that survive
partial comparison; a parser diagnostic leaking unrelated user data; and a shared
renderer refactor changing Windows or Antigravity manual commands. Each has a
negative fixture or existing/manual byte regression in the steps above.

## Investigated risks and self-review

The current parser/status probe is permissive; a separate strict projector avoids
a compatibility change. The strict parser already exists, but its diagnostics can
include key names: map categories instead of returning those errors. The portable
formatter reads home through its wrapper; factoring its pure body avoids hidden
reads in the projector. Current Antigravity imports the marker helper, so preserve
it unchanged and cover the shared formatting dependency. Exact first POSIX commands
are source-owned and fixed; whole native/user configuration and actual home remain
outside the returned bytes.

| Dimension | Rating | Reason |
| --- | ---: | --- |
| Dependencies | 3 | Existing shared formatter, Codex builder and strict parser; actual collection remains deferred. |
| Blast radius | 3 | Manual renderers share extracted code, protected by complete byte regressions. |
| State changes | 1 | Pure source helper/projector, no new mutation or active capture caller. |
| Reversibility | 1 | Local revert; no metadata migration or installed config rewrite. |
| Uncertainty | 3 | Source behavior inspected; exact projector and adversarial evidence not yet executed. |

Total11/25 for this bounded source slice. Self-review completed once against
the design for interface consistency, complete namespace traversal, retention/
error behavior and compatibility; no material gap remained. Disposition: **Approved for bounded source implementation;
source/focused/full checks complete; required routing fix verified; current medium review passed.** The five-file candidate set is not registered or
qualified as the actual integration inventory by this plan.

Review effort: use `/code-review medium` for the final current diff. The coupled
renderer/projector plus adversarial tests is about 500 Rust changed lines and
contains a new untrusted-JSON retention boundary. This meets the repository’s
explicit size/security escalation conditions; keeping the coupled byte-contract
and manual-compatibility evidence together avoids an incomplete handoff. No new
implementation scope or authority is added by this review-effort choice.
