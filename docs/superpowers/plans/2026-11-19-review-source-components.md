---
type: Implementation Plan
title: Offline review source qualification components
description: Bounded codec and admission-state oracles for the proposed artifact epoch and ordinary Start source contract.
tags: [orkworks, review, qualification, plans]
status: draft
---

# Offline review source qualification components

**Outcome:** Validate the proposed review source record/transition rules with
finite executable oracles, without writing application metadata or launching a
session. Pass means malformed/conflicting/replayed inputs cannot obtain another
simulated creator invocation and every admitted/rejected fixture has a checked
encoding and disposition. It does not mean production source integration works.

**Tracking:** [#1899](https://github.com/Rambolarsen/orkworks/issues/1899),
[#68](https://github.com/Rambolarsen/orkworks/issues/68).
**Contract:** [artifact revision and Start admission sources](../specs/2026-11-19-review-source-qualification-design.md).
**Status:** proposed for the same scoped owner decision as that contract.
The owner authorized preparation; new record/cap/source choices and this
logical-only executable scope require acceptance before implementation.
Prior #1913 acceptance remains valid and is not requested again.

## Boundaries

Use Node 22 standard-library JavaScript modules under `docs/validation/`, in
the existing component-oracle style. No new package, Rust source, Electron API,
application endpoint, SQLite/JSON metadata authority, installed setting,
filesystem watch, provider call or native process. Test temporary output may be
written only to a caller-owned temporary directory and cleaned up; no read or
write of `~/.orkworks/`.

The epoch validator consumes a synthetic owner identity and generation plus
explicit pin/selection fields. It does not pretend a synthetic UUID is the real
Store reference or prove actual publication-lock coverage. Prepared-input digests
are fixture identities checked only for shape/equality; the producer's digest
recipe is explicitly unqualified. Simulated publication checkpoints and invocation
counters prove logical ordering only, not flush/replacement or OS behavior.

## Delivery units

### 1. Artifact binding and registry codec

Create `docs/validation/ordinary-review-source-components.mjs`,
`ordinary-review-source-components.test.mjs`, and
`ordinary-review-source-vectors.json`. Keep implementation in the module and
independently hand-derived expected records/dispositions in the vector file.

- [ ] Write failing cases before implementing the local validator/codec. Pin
  explicit schema/state/key sets, UUID/ID/counter/digest validation and complete
  byte/count limits from the linked contract, not values invented in the test.
- [ ] Cover matching binding, new owner, changed generation, pin change with an
  unchanged selection ID, conservative unrelated-publication invalidation,
  external current-content mismatch and generation exhaustion. The synthetic
  artifact generation permits zero like the existing source, while stored
  registry/attempt revisions are positive; the domains are not interchangeable. Do not assert
  detection of unobserved filesystem history.
- [ ] Use the source contract's bounded parse/validate/fixed-order encode/byte-
  equality check to retain duplicate-key rejection without a custom parser; reject duplicates,
  unknown keys, malformed/unsupported state, noncanonical/overflow counters,
  invalid identifiers/digest, wrong workspace identity, illegal state/reason
  combinations, oversized raw
  input and too many retained attempts.
  Encode a checked complete record/registry; count full UTF-8 bytes.
- [ ] Include valid maximum counter/identifier values and 256 retained records,
  then negative controls for overflow/257 records and corrupted keys. If a
  generous byte cap is unreachable by valid closed fields, document that bound;
  do not manufacture an invalid wider record and call it a valid maximum.

Pass: `node --test docs/validation/ordinary-review-source-components.test.mjs`
exits zero after observed RED/GREEN, with exact golden encoding/dispositions.

### 2. Admission and retry state machine

Extend the same focused module/tests/vectors with a pure immutable admission
model. Its only effect is an injected simulated creation counter; no process,
metadata or HTTP implementation is imported.

- [ ] Write failing literal transition traces for reserve, durable-dispatch
  checkpoint, final validation, creator entry, linked reply and settlement.
  Preserve original attempt/recommendation/preview tuple and allocated session ID.
- [ ] Cover two exact concurrent/repeated Starts, conflicting tuple, a second
  attempt/recommendation for the same occupied source slot, expired/lost preview
  after admission, known pre-creation rejection and source/configuration changes.
  Exact replay returns retained disposition; it never increments creation count.
- [ ] Enumerate fault checkpoints before/after each simulated publication and
  creator entry. Model uncertain publication/creation and owner loss as fenced;
  no automatic transition to reserved, no new creator call on replay, and no
  title/path/native-process adoption shortcut.
- [ ] Exercise both controlled-source change/admission orders and forward-only
  revisions, terminal states, unknown→linked/settled only with an explicit
  qualified-evidence input, slot release and saturation. That evidence input is
  a simulation predicate, not a production proof or an API accepting a boolean.
- [ ] Add a meaningful negative control for each duplicate/ordering gate by
  temporarily removing it and observing the corresponding existing trace fail;
  restore it before the final run. Do not assert mock behavior alone.

Pass: the same test command checks all traces against independent expected
records and invocation counts. Explicitly report the model's source/physical
limitations alongside passing logical results.

### 3. Repository checks and evidence handoff

Create `docs/validation/ordinary-review-source-components.md` with OKF metadata,
source contract, exact tested cases and qualification limits; add its directory
index entry. Add the new test file to the existing `docs/package.json`
`docs:check` list. Update this plan and the source contract with actual evidence.

- [ ] Run the focused tests under Node 22, docs metadata/index/component suite
  and site build. Verify no prior validation fixture/codec bytes changed.
- [ ] Run `bash scripts/verify-repo.sh` after code implementation, whitespace,
  documentation/worktree currency and a bounded fresh current-diff correctness
  review. Record failures and actual limits; do not turn simulated success into
  an accepted storage/producer/runtime profile.
- [ ] Keep #1899/#68 open for prepared-input encoding, actual Store capture/fence
  integration, checked durable publication, registry initialization/retention/
  migration compatibility and the private prepared-creation admission seam.

## Review and risk treatment

Reviewed once against the linked source contract and accepted #1913 boundary.
Disposition: **Ready for scoped owner review and offline execution after
acceptance; production admission Investigate.** No test implementation ran in
this planning task. Source audit findings are retained in the contract.

| Dimension | Rating | Treatment |
| --- | --- | --- |
| Dependencies | 2 | Closed candidate source rules; live producer/storage dependencies explicitly excluded |
| Blast radius | 1 | New offline module, vectors and docs-test registration |
| State changes | 1 | No application persistence or launch |
| Reversibility | 1 | Remove offline artifacts/test registration; no user data to restore |
| Uncertainty | 2 | Pure ordering/codec assertions; actual source/durability guarantees remain separate |

Offline slice: 7/25. No score grants production approval. Execute inline with
TDD and one final bounded review; additional workers are unnecessary for these
coupled source/codec/state interfaces.
