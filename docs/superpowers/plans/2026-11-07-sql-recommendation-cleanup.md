---
type: Implementation Plan
title: Captured SQL recommendation cleanup
description: Admit fresh recommendation graph cleanup through exact SQL deletion jobs and retain the original owner through progress persistence.
tags: [orkworks, sqlite, storage, taskmaster]
status: stable
workflow_status: implementation-complete
---

# Captured SQL recommendation cleanup

Continue [#1243](https://github.com/Rambolarsen/orkworks/issues/1243) after #1502,
within the accepted [external deletion contract](../specs/2026-11-01-sqlite-session-storage-design.md#external-payloads-events-backup-and-deletion).
Reuse the existing captured replay/workflow/image/settings admission path and the
bounded recommendation publication owner. No production selection or activation.

## Outcome and decisions

Add recommendation cleanup as source-only progress bit `0x10`. Capture the
recommendation store's shared-owner identity with the existing workspace/runtime
memory snapshot. Bind its original directories and lease outside application
guards during the existing capture I/O phase. Binding errors remain specific to
this stage; other cleanup stages retain their existing behavior.

Before effects, use the existing session SQL lane, re-read the exact tombstoned
job by ID/creation, validate original workspace generation/lease/reader/runtime,
check the captured recommendation shared owner, reserve the original session
operation and atomically admit the retained file worker. No SQL or filesystem
I/O occurs while projection/workspace/session/PID guards are held.

The worker performs bounded inspection, the existing complete graph closure and
v2 publication under the shared recommendation permit. Retain its journal result
receipt through SQL progress persistence and final detached-view checks, including
file failures, SQL Busy, admitted cancellation and caller loss. Persist bit `0x10`
only on successful file completion, preserving other progress bits. Keep the
hidden tombstone/job; no final row removal is implemented.

Pending v2 journals lack SQL job/creation/epoch binding. This stage rejects them
before effects instead of calling generic recovery under an unrelated job's
authority. A partial publication leaves visible pending evidence and no progress;
automatic identity-bound recovery routing remains a gate before activation.
Existing generic replacement-journal recovery remains unchanged. Existing explicit
Delete uses the full reference/rollup closure for all recommendation states;
background anti-spam citation protection is a different operation and is unchanged.
The immutable legacy session JSON remains untouched by this source-only stage.

Producer revalidation/revocation against SQL tombstones remains mandatory before
activation; a successful cleanup pass and held operation permit do not qualify
later legacy publishers or permanently prevent recreation after release. No
producer, retention, authority-selection, migration or final Delete behavior is
enabled here. Existing graph limits and bounded retained file-intent admission
remain; transient graph working memory has no measured RSS or latency claim.
macOS/Linux file effects remain qualified by fixtures, other platforms Unsupported.

## Delivery and checks

- [x] TDD exact full-graph removal, unrelated bytes and legacy metadata retention,
  bit preservation/reopen and no-op behavior.
- [x] Qualify Busy and malformed data, pending foreign journals, original-store
  and physical directory replacement, stale job/creation and capture ownership.
- [x] Qualify reservation/lease/permit lifetime through SQL failure and admitted
  caller loss, cancellation/expiry before admission, free application guards and
  acknowledged input while stalled, and one-blocking-worker completion.
- [x] Run store/application regressions, current-diff medium review and full
  verifier. PR terminal evidence records required CI and owned cleanup.

## Plan review

Ready under the accepted architecture, with activation gates preserved. Source
inspection confirmed that existing v2 journals carry no SQL identity and that
legacy explicit Delete removes the complete connected recommendation graph after
its session's close/finalization gates. Calling generic recovery here would adopt
unrelated deletion authority, so inspection deliberately fails closed on pending
journals. Existing captured stages already retain module ownership through SQL
progress and isolate binding errors. Same-owner comparison is memory only;
physical identity remains the anchored file module's responsibility.
Dependencies3(application/SQL/file owners), blast radius2(unwired stage plus common
capture), state changes4(qualified destructive effects), reversibility4(intentional
committed deletion, tombstone and recovery evidence retained), uncertainty2(existing
proven owners, new composition under test), total15. Fault/ownership tests address
composition risk; job-bound recovery and producer qualification remain explicit
future gates and are not inferred from a progress flag.


Integration discovery: legacy successful graph transactions leave an empty
`.rollup-transactions` parent. The full-graph integration regression initially
failed on that valid source layout. New removal preparation allows only a verified
empty parent; publication reuses its pinned directory and removes it during normal
cleanup. No-op publication may retain that empty scaffold. Existing strict
read-only inspection is unchanged, and any nonempty transaction parent is still
refused without recovery effects. This compatibility adjustment grants no SQL
job authority to existing journals.


Scoped verification: all 93 recommendation-store regressions and all 87 SQL
application tests pass with the new stage. The strengthened foreign-directory
fixture initially retained a stale expected byte literal; its assertion now
checks the exact valid JSON seeded into the foreign directory. This was a test
expectation correction, with no product relaxation. Existing shared admission
fixtures cover cancellation/expiry and stale runtime/lease/reader cases; new
stage fixtures cover its distinct recommendation ownership and receipt lifetime.


Both independent current-diff `/code-review medium` passes are clear on product
SHA256 `35387b2c5fbcda7532b7f5a2d2cc3c526f2e3941d0b91e147102663781984a05`.
One pass audited exact admission and retained resources; the other audited
anchored file/journal compatibility and pending-journal refusal. Medium effort is
justified by lifecycle/destructive composition and the diff size. Main's two new
documentation-only commits were included before final verification; every reviewed
source file and the full product diff stayed byte-identical across that rebase.


Final `bash scripts/verify-repo.sh` passed with `RUST_TEST_THREADS=1` and no
additional filters/skips: 3,144 Rust unit tests (18 existing ignored), seven
integration, 31 reporter, one Electron ABI proof, desktop type-check/build and
1,697 tests (three existing skipped), 1,202 docs tests/build and diff/doc/worktree
checks. Reviewed source files stayed unchanged. Public-doc currency reminder and
existing bundle-size warnings were reviewed against the diff: this private stage
changes no installer, onboarding or production flow; canonical architecture and
validation carry the actual source behavior and remaining rollout gates.
