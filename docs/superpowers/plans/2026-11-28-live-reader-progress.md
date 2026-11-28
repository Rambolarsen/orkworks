---
type: Implementation Plan
title: Live reader progress for the accepted Windows candidate
description: Implements the accepted isolated reader package before its sole gated candidate attempt.
tags: [orkworks, qualification, windows, diagnostics]
status: draft
workflow_status: ready
---

# Live reader progress implementation

Tracking [#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
The owner accepted the combined package through “keep going” after merged
[#2222](https://github.com/Rambolarsen/orkworks/pull/2222). The
[accepted contract](../specs/2026-11-27-live-reader-integration-design.md)
controls all limits, wire/output meanings and candidate selectors. Initial base
f937ab9f5586cfafbc3182de5a7a8bb0a1cea9c7. All four source pins and the exact
43,200-byte policy digest still match. Execute inline; root owns all changes.

## Delivery and interfaces

- [x] Task 1: Add the dedicated v2 codec and monotonic job ledger in
  scripts/qualification/pair-progress-windows-build-protocol.mjs, with pure
  tests in pair-progress-windows-build.test.mjs. Decoder consumes nonce/mode/
  grant and one shared ledger; it returns immutable arrival traces and closed
  settlement predicates. Ledger reserves one reader grant while enforcing the
  129 MiB admission subledger and 3 MiB emergency tail. Tests first: strict
  v1/v2 separation, all framing/order/count limits, late/overflow/partial/error
  paths, three sequential grants, maximal debit refusal, source/output-before-
  settlement and receipt prefix retention. Run Node tests; expected all pass.
- [x] Task 2: Fork the complete old PowerShell reader into the new
  pair-progress-windows-build.ps1 and byte-identical .inputs.json. Preserve all
  archive/helper/PE/reparse/source checks. Replace only request/root derivation,
  progress emission, expanded-read precharges, exclusive reusable payload scratch
  and output binding. Tests first: absent registry preflight, missing live root
  refusal, bounded canonical nonce request, immutable output, exclusive lease,
  empty/large-short digest goldens and every inherited refusal. New native
  harness exercises owned inputs only, within the accepted three-child envelope.
- [x] Task 3: Fork the candidate controller and SDK wrapper into
  pair-progress-windows-build.mjs and -sdk.py, using the new ledger/codec and
  streaming own runner. Keep the literal build/oracle/link/SDK recipe; retain
  complete parent validators bound to the new policy. Admit held sources/output
  before known settlement, conservatively debit all unknown cases, stop later
  stages, and publish receipt-only traces after failure. Tests first: spawn
  deadline crossing, streaming before close, cooperative error/bad close,
  drain/cleanup, source/output admission, publication failure and new schema.
  Pure guarded imported helpers remain sealed; imported main/runner never runs.
- [ ] Task 4: Add the new manual workflow, exact own LF rules and one five-minute
  fixture step in the existing SDK fixture job. CI can reach only owned fake
  inputs; no installed discovery or live main. Document test evidence and
  narrow fork differences. Run scoped fixtures, full scripts/verify-repo.sh,
  doc/worktree checks and current-diff review; merge only after applicable CI.
- [ ] Task 5: Freeze the reviewed merged tooling SHA, exact policy/commands and
  accepted one-job allowance on #1176. Verify fresh workflow history 0/1 and
  unchanged main, dispatch once, independently read bounded allowlisted evidence,
  record success or unavailable, close all unused slots and remove owned copies.
  No retries; any attempt spends 1/1. No resulting program executes.

## Review focus and recovery

Test malformed/late progress even after a valid prefix; source drift after a
clean terminal; rejected/duplicate-key/oversize output; maximal grant followed by
publication; and a timeout whose child remains undrained. These belong to tasks
1–3, respectively. No payload pass or archive check is omitted to fit a budget.
The new source list seals the entire transitive imported helper closure.
Failure publishes only bounded receipts and retained prefixes, never salvages
bulk evidence with emergency credit. Removal of new tooling/workflow/fixture step
reverses integration; old spent workflows and production gates stay unchanged.

Plan review: **Ready** for inline implementation of the accepted package.
Dependencies 2 (sealed helper imports); blast radius 1 (isolated tooling);
state changes 2 (additive records and one explicitly accepted finite job);
reversibility 3 (tooling reverts, attempted allowance cannot be recovered);
uncertainty 3 (native full closure capacity unverified). Total 11/25. These are
risk descriptions, not admission. The existing design investigation supplies
failure and capacity limits; no new design choice or second approval is needed.
One final read-only review round will cover correctness and missing coverage,
root applies findings; no implementer subagents or second review loop.
