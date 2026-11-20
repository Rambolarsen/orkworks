---
type: Implementation Plan
title: Offline ordinary-review configuration leaves
description: Finite source-byte commitments, resource-set coverage and prepared-command joins without source capture or production activation.
tags: [orkworks, review, configuration, qualification, plans]
status: stable
---

# Offline ordinary-review configuration leaves

Outcome: complete candidate catalog/integration leaf bytes have independent
golden commitments; omitted/mixed/changed resources, owner/epoch and command
bindings deny the conditional join. This proves byte and finite-set relations,
not actual resolver, adapter inventory, native eligibility or capture history.

Contract: [catalog and integration leaves](../specs/2026-11-20-review-configuration-leaves-design.md).
Track [#68](https://github.com/Rambolarsen/orkworks/issues/68),
[#66](https://github.com/Rambolarsen/orkworks/issues/66) and
[#1899](https://github.com/Rambolarsen/orkworks/issues/1899).
Status: owner accepted this plan and leaf/availability/policy/budget choices on
2026-11-20 with “keep going”. #1913/#1919/#1922 acceptance remains valid.
Executed evidence is in the [component report](../../validation/ordinary-review-configuration-leaf-components.md).

## Boundary and delivery

Only new Node22 standard-library artifacts under docs/validation and registration
in docs/package.json. No Rust/Electron, actual collector, source configuration
read, credential access, filesystem lease, install/reconcile/revoke, probe,
provider/native process, metadata, dependency or runtime activation. Existing
#1919/#1922 fixture/codec bytes stay unchanged. Execute inline with TDD.

### 1. Closed leaf byte commitments

Create `ordinary-review-configuration-leaf-components.mjs`, its `.test.mjs` and
`ordinary-review-configuration-leaf-vectors.json` under docs/validation.

- [x] Write failing literal catalog/integration values and complete canonical
  expected byte strings/digests, independently derived from the implementation.
  Include embedded/source byte changes, explicit absence versus empty file,
  resource states and domain separators; no raw fixture credential values.
- [x] Enforce closed key/enum/UUID/counter/ID/base64 domains, canonical UTF-8,
  ordered/dense resource/selection arrays, exact stock-only/absent-user state,
  runtime policy and exhausted epoch refusal. Unknown/extra/missing/duplicate or
  unreadable/symlink-like states cannot become absence.
- [x] Exercise raw64KiB builtin and256KiB resource limits plus complete128KiB
  catalog/512KiB integration/640KiB combined encoded charges. Construct valid
  boundaries only where the closed fields permit them; report unreachable or
  stricter constraints rather than invent invalid padding/maxima. Prove every
  byte is included, not a previous digest or truncated subset. The 640 KiB
  combined cap is the sum of the individual caps: do not invent an independent
  overflow witness with both leaves valid, or demand a redundant-guard mutation.

Pass: focused `node --test docs/validation/ordinary-review-configuration-leaf-components.test.mjs`
under Node22 RED then GREEN, exact independent bytes/digests and complete recounts.

### 2. Conditional coverage and prepared joins

- [x] Compare inventory/resources to a separate literal expected code-owned
  inventory fixture; reject omitted/extra/duplicate/reordered/mixed role/state
  records. Clearly label that expected set as unqualified for real adapters.
- [x] Join both leaves to one synthetic owner/ref/epoch and current workspace/
  active selection; join selected tool/model, final program/argv/cwd and both
  computed digests to a copied valid #1922 prepared value. Reuse its validator
  without editing its module/fixtures or treating it as a production consumer.
- [x] Test valid stock input, present/invalid user-file refusal, replaced owners,
  failed-publication/ABA epoch changes, changed assets/owned fragment/preferences,
  required-but-missing input, unknown qualification, late argv/native selector,
  fallback or altered instruction/command cases. All flags are explicit test
  predicates, never a public API accepting synthetic authority.
- [x] Temporary-copy negative controls remove canonical equality, complete leaf
  digest inclusion, expected-set membership, owner/epoch and final-command joins,
  full encoded-byte guards. Existing assertions must detect the removals; delete
  copies afterward. No actual history/concurrency/durability claim.

Pass: every independent fixture has expected byte/disposition; unsupported data
never acquires a synthetic qualified join. No source-native probe is used.

### 3. Evidence and regression handoff

- [x] Add `ordinary-review-configuration-leaf-components.md`, OKF/index entry
  and docs:check registration. Record independent byte facts, logical limits,
  unavailable actual inventories/producer/native/persistence/capture evidence.
- [x] Run focused tests, docs checks/build, `bash scripts/verify-repo.sh` after
  code, whitespace and documentation/worktree close-out checks. Inspect/restore
  generated pnpm executable metadata; no incidental dependency change.
- [x] One fresh whole-task correctness review; reproduce required issues with
  RED/GREEN. Keep parent issues open for actual stock resolver/catalog owner,
  adapter inventories/projectors, pure capture/epoch lanes, private prepared
  creation and physical admission lifecycle/retention qualification.

## Review and risk

Self-review once: scope is finite codecs/coverage/joins; stock absence/policy
restrictions are explicit new owner choices; actual read-set completeness and
physical capture remain unqualified. Evidence/motivating source findings are in
the design. Disposition: **Offline logical slice verified; production Investigate.**
Logical components are implemented; actual-source/native gates remain open.

| Dimension | Rating | Treatment |
| --- | --- | --- |
| Dependencies | 2 | Closed candidate wrapper rules; actual resolver/native inventories excluded |
| Blast radius | 1 | New offline artifacts and test registration |
| State changes | 1 | No application persistence, source capture or process |
| Reversibility | 1 | Remove artifacts/registration, no user data |
| Uncertainty | 2 | Finite byte/set checks only; actual producer gates retained |

Offline slice7/25; no score grants production authority. One owner approval of
this concrete design and plan suffices for unchanged bounded offline execution;
no separate planning approval round is required.

## Execution evidence

Owner accepted the concrete design/plan on 2026-11-20. All 180 focused cases pass,
including the observed RED/GREEN empty-inventory owner-map regression. All 36/36
temporary-copy guard mutations were detected. Fresh correctness review found no
required finding; optional canonical-buffer reuse was applied and tests passed
again. No deferred findings. Earlier #1919/#1922 codecs/tests/vectors are unchanged.

`bash scripts/verify-repo.sh` passed under Node22: Rust 3,615 passed/24 ignored
plus integrations; reporter32; native ABI; desktop 1,791 passed/5 skipped,
type-check/build; docs 2,490 and site build; whitespace/doc/worktree checks.
Existing compiler/chunk warnings concern unchanged source/configuration. README
drift requires no edit because these offline components change no shipped
architecture, installer, milestone or available product behavior. Generated pnpm
executable lock metadata was inspected/restored; no dependency change. Foreign
worktrees were preserved. Actual collector/resolver/inventory/native and physical
admission/Start qualification remains open.
