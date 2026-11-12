---
type: Validation Evidence
title: Offline research input and current-control prefix
description: Independent prepared-input provenance and finite research control and collection views, with explicit producer, history, consumer and publication limits.
tags: [orkworks, research, preparation, controls, qualification, history]
status: draft
workflow_status: prefix-components-only
---

# Offline research input and current-control prefix

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689) and
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089).
Inspection base: `851a2d8a42225082132e907c33aecc08a872f589`.

The owner [accepted the four prefix choices](../superpowers/specs/2026-11-12-research-input-control-prefix-design.md#scoped-owner-acceptance-and-first-prefix-component-packet)
for offline component fixtures only. This packet supplies separate immutable
assessment/event origins and finite current-control/collection examples. Gate 2
remains Investigate. There is no product runtime, native execution, storage
publication, legal transaction history or executable implementation plan.

## Independent construction and retained artifacts

- [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-prefix.py) constructs a new input-2-bound source chain using unchanged Python byte/framing helpers and the earlier startup oracle. It never invokes the JavaScript checker.
- [JSON oracle](research-prefix-components.json) retains the new source chain, unchanged root initial-input shape, two research input rows, two runtime-source rows, a labelled report/head assumption and four independent views.
- [Checker](research-prefix-components.mjs) composes unchanged source-chain, initial-input, parent-attachment and report/evidence-source validators with explicit source joins and finite expected views. Its only results are measurements.
- [Tests](research-prefix-components.test.mjs) independently regenerate the oracle, compare byte preservation, reseal input mutations and distinguish assessment authority from the newer source-event producer.

The new plan/task/child/configuration/group IDs and derived source hashes differ
from the earlier [source-chain packet](research-chain-components.md). That packet's
unqualified preparation-1 binding remains unchanged; this packet does not adopt
or retoken it. The new chain binds input-2's complete digest and preserves the
original reservation pair expectedTaskVersion=1 / assignmentTaskVersion=2.
The generator reuses pure Python helper definitions, not prior event history.

The outer JSON packaging is neither a storage catalog nor a protocol envelope.
The checker receives decoded objects. It is not a duplicate-member-aware wire
decoder; canonical retained source bytes need the existing strict source decoder
before use. JSON.parse alone cannot qualify raw-byte canonicality or duplicates.
Raw startup evidence continues through its existing composed decoder.

## Producer and historical assessment joins

Input-1 remains exactly the root initial source: revisions 1/1, decision null,
records empty and no invented research producer or inputDigest field. Its ref
uses the unchanged preparation-input domain over its complete initial body.

Input-2 is a fresh parent-produced research assessment at revisions 2/2. Its
decision revision is 2, clarificationNeeded false and researchNeeded true.
Its records consume exactly the new configuration, scope and every scope capture.
Input-3 advances the paired revisions to 3/3, consumes the additional exact report
evidence source and copies decision-2 unchanged. Its producer is the reporting
child RuntimeRef. The parent assessment origin remains in input-2's own row.

For a stale copied decision, the checker resolves the unique exact run/k/k input
key, validates the original as a fresh parent assessment, then compares decision
bytes. A missing/ambiguous/foreign-key original, changed copied decision, missing
runtime history or child-authored fresh assessment denies. A matching decision
body does not reconstruct an original producer. The component validator does
not read a current view to elect a historical parent.

The standalone input check covers this packet's source set and its single older
assessment lookup. A revision-64 fresh assessment is a bounded source-shape
witness, not a launch, legal revision sequence or rebind of the finite plan.
Parent reassessment of an already selected report and broader histories remain
separate witnesses. Producer refs and symbolic CommitRefs establish only byte
joins; they do not authenticate a real parent/child or prove an event committed.

## Finite current views and prospective consumer ownership

The four views are independent component samples, with 1/5/7/7 row examples:

| View | Run version / input | Selected local facts |
| --- | --- | --- |
| initial-control | 1 / original input-1 | Research run owner, assessing, no invented parent or plan |
| proposed | 3 / input-2 | Proposed plan, pending task, active collection and collecting expectation before any report head exists |
| before-selection | 10 / input-2 | Active plan, running launch, nonterminal session, original admission tuple and live taskVersion 7 |
| selected-source | 11 / input-3 | Same live taskVersion 7 and frozen identity 2; collection/expectation versions 3 select the supplied report source |

The running samples retain the original approval, reservation, allocation,
attachment, native descriptor-shaped observations and child generations. Session
producerRevision remains the separate Counter string "2". No native observation
was made. Report selection leaves live taskVersion 7 unchanged and never replaces
the assignment's frozen version 2. These are byte/state comparisons, not a real
report request, CAS, grant, ordinary bridge event or accepted report receipt.

The proposed view positively owns one collecting expectation with no reservation
or selected report yet. The subsequent views bind the original reservation and
then the exact supplied source. An absent head does not mean zero consumers.
This is only one declared local membership; physical direct/reverse indexes,
global consumer completeness, actual report_heads currentness and all consumer
effects remain unqualified. The reportAssumption includes a valid synthetic
first child report and matching head fields, not a published report_head row.

Run versions 1/3/10/11, task version 7 and view ordering are explicit finite
samples. The checker does not derive intervening events, authenticate producers,
validate a legal event sequence, or fill missing state. It exports no dispatcher,
reducer, publisher, transaction command or authority result. Initial/proposed/
running/selected samples do not cover blocked, paused, resume, recovery, terminal,
correction, retry or cutoff behavior. No other current run owner is adopted.

## Measurements, bounds and checks

Independent Python canonical UTF-8 measurements agree with the checker:

| Quantity | Bytes |
| --- | ---: |
| Input-2 body | 2,160 |
| Input-3 body | 2,909 |
| Parent runtime-source body | 515 |
| Child runtime-source body | 692 |
| Initial view key/value wrappers, summed | 1,407 |
| Proposed view key/value wrappers, summed | 6,066 |
| Before-selection view key/value wrappers, summed | 11,828 |
| Selected-source view key/value wrappers, summed | 12,536 |

Wrappers measure canonical `{key,value}`; family labels/array order are packaging.
The checker enforces the accepted per-family complete wrapper ceilings: inputs
64 KiB, runtime history 4 KiB, run controls 1 MiB, launch controls 32 KiB and the
other sampled controls/collections/expectations 16 KiB. Complete input bodies
also obey 32 KiB, paired revisions 2..64, decision reasons 1..16 through 2 KiB
each and at most 256 exact sorted unique records. The tests include an input
body at exactly 32,768 bytes and one byte above, with otherwise bounded reasons.
They do not claim near-limit witnesses for every coupled wrapper maximum.

There are 24 row examples across the independent input/runtime/control groups.
They are not 24 rows in one store or a transaction count. View sums include
repeated bodies deliberately and are not shared-pool or aggregate quota charges.
No committed projections, command resources, source/assignment pool allocation,
8/64 MiB control-pool accounting, 32 MiB run accounting, quota initializer or
complete catalog/proof/index format is supplied.

The first positive measurements and semantic denials failed against the explicit
empty checker stub before implementation. The tests now cover exact regeneration,
paths containing spaces/percent signs, no checker mutation, unknown fields,
source ordering/consumption, producer/generation drift, stale-origin loss,
live/frozen confusion, prospective membership and direct-execution runtime-domain
substitution. Temporary-copy removal of each parent-producer, unique-origin,
copied-decision equality and finite-view guard fails its intended denial test.
No owned checker was altered for that mutation exercise.

Pinned Node 22 verification passed all 1,564 documentation tests, including 59
new prefix tests, metadata/index checks and the rendered-page/dead-link build.
All 20 OKF regression tests, documentation drift and diff whitespace checks
passed. Existing VitePress circular/large-chunk warnings concern unchanged
bundler configuration and require no edit for this packet. The scoped checks
execute only offline documentation tools; no Rust, Electron, native harness,
model or product runtime was executed.

Use pinned Node 22 from the repository root:

```bash
rtk proxy python3 docs/validation/generate-research-prefix.py --output /tmp/research-prefix-components.json
rtk proxy cmp docs/validation/research-prefix-components.json /tmp/research-prefix-components.json
rtk proxy node --test docs/validation/research-prefix-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

## Remaining producer/history/consumer contract

All 541 earlier non-Markdown validation artifacts remain byte-for-byte unchanged.
This packet supplies no successful attachment/approval event, current authority,
source custody or native identity. The supplied symbolic CommitRefs have bounded
syntax and exact local joins only; there are no proof manifests, fresh-store
origins, causal history or storage format selection. No format-3/4 adoption,
volatile bearer serialization, actual report cutoff or ordinary-session storage
bridge follows from source hashes.

The [first-report investigation](first-report-producer-contract.md) still requires
source-owner currentness/index closure, complete typed atomic effects and resource
accounting, producer history/proofs, qualified cutoff/ordinary bridge and recovery
before a complete positive transaction/history witness. Broader research states,
coordination/synthesis/execution/terminal behavior remain open too. This bounded
fixture acceptance authorizes none of the proposed Rust/Electron modules, native
probes, adapters, migrations, activation or executable runtime plans. #1689 and
#1089 stay open.
