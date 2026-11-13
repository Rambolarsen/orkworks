---
type: Validation Evidence
title: Offline research initializer values and charges
description: Independently derived fresh resource keys, initial research current-control charges and conditional logical row counts without a storage format or creation proof.
tags: [orkworks, research, initialization, accounting, qualification]
status: draft
workflow_status: initializer-components-only
---

# Offline research initializer values and charges

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689) and
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089).
Inspection base: `0b1d188dd51a7db91081953c6660777dfd728619`.

The owner [accepted both initializer choices](../superpowers/specs/2026-11-13-research-initialization-catalog-design.md#scoped-owner-acceptance-and-offline-initializer-components)
for offline components only. This packet compares the 14-workspace/9-run/5-subject
resource recipe and original-admission current-control charge. It is not a store,
initializer implementation, reducer, publisher, original creation proof, legal
history or whole-command packing witness. Gate 2 remains Investigate.

## Independent artifacts and input boundary

- [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-initialization.py) reads only the unchanged root-preparation-effects JSON and independently derives values, projection lengths and logical locations. It does not invoke JavaScript.
- [JSON oracle](research-initialization-components.json) retains separate workspace-zero, captured-peer, draft/cancel and initialized/never-initialized admission components. Four labelled origin refs and two header/proof labels are assumptions, not commits or rows in a created store.
- [Checker](research-initialization-components.mjs) uses unchanged source-only draft/approval/initial-input/state validators, independently derives the expected values and compares exact keys, counters, projections and locations. It generates no storage commitment or applies any row.
- [Tests](research-initialization-components.test.mjs) independently regenerate the oracle, preserve both inputs, cover semantic denials and pin Python-derived byte/count measurements. Copying the generator/input/output under spaces/percent signs preserves exact UTF-8/LF bytes.

The source context is the retained target root example, with its original
run/parent/root/bootstrap identities and empty-output assignment. It is distinct
from the earlier research child-prefix run. This packet does not graft that
child chain onto the target root or claim a path between them.

The baseline's old source/projection examples supply measurements only. Its
header/manifest rows are excluded from decoded component rows; rowDigest and
stateSha256 fields are not used for admission or new commitments. No old proof,
header, state fingerprint or source receipt is rewritten into another storage
format. The raw baseline SHA-256 identifies the trusted documentation input,
not a storage domain or producer authority. New ledger origin labels do not
adopt the old rows/history as a fresh store.

The checker accepts decoded packaging. It is not a new duplicate-aware wire
decoder. Selected canonical source values use the existing strict source decoder;
new source/row/protocol codecs are not introduced. Actual source custody/native
role evidence and authorization remain unqualified.

## Resource values and separate assumptions

The workspace-initialization group has fourteen full quota_usage key/value
examples, all used/reserved "0" with the labelled initialize origin. It includes
the five subject-resource peers, four summary resources, two preparation
resources and source/control/report resources. Zero rows are present values,
not missing-row defaults. There is no header or initialization manifest.

Each admission branch has thirteen fresh resource values: five root-assignment
rows, five run subject-resource rows and the run source/control/report rows.
The existing run preparation ledger is separately replaced, producing nine run
resource names after admission. There is no assignment-scoped report/source/
control ledger. All fresh subject/source/report values stay zero; the run control
ledger charges its actual new occurrence. No slot, artifact producer, bearer or
read-work credit follows from a resource row.

capturedWorkspacePeers is a **hypothetical prestate**, not a reconstructed
inventory. Older root examples supply its preparation/summary values; five
additional peer contributions are deliberately nonzero: artifact bytes 4,096,
artifact read-work 8,192, source bytes 16,384, control bytes 2,048 and report bytes
8,192. These are illustrative values, not independently reconciled totals.
Their source objects, debits, receipts and original proofs are not supplied.
Matching those counters cannot prove a coherent or fresh store. The component
checks only that the proposed admission does not reset or restamp unchanged
peers, and applies the actual changed deltas once.

New and changed ledger values use canonical u64 Counter strings. Source run/input/
control versions remain their separate bounded JSON integer types. Origin
sequence strings have bounded syntax but no predecessor proof or chronological
qualification. Their changeDigest fields contain raw hashes of component labels,
not hashes of storage manifests. No private file, live inventory, database or
native identity is read.

## Initial control and charge measurements

The new control is schemaVersion 1, accepted research profile, runVersion 1,
assessing, with exact original run/bootstrap/repository/source bindings and a
currentInput ref to original root input-1. The initial body remains null-decision/
empty-record root data, with no invented research input row/producer. Plan,
parent-runtime, collection, child and grant creation are absent. The old run_states
seed remains historical; this component does not create a second mutable counter.

Independent canonical UTF-8 measurements, also asserted by the checker/tests:

| Quantity | Bytes |
| --- | ---: |
| Initial control key plus committed-value projection | 1,202 |
| Complete initial control key/value wrapper | 1,419 |
| Fourteen workspace-zero key/value wrappers, summed | 4,744 |
| Target run preparation projection before admission | 19,259 |
| Target run preparation projection after admission | 21,741 |

The control charge excludes only its outer lastCommit and fits the existing
1 MiB complete wrapper ceiling. The run ledger starts at 1,202; the captured
workspace control peer advances 2,048→3,250. The 2,482-byte preparation delta is
applied to both the existing run and workspace preparation ledgers. Source and
report peers keep their hypothetical usage and origin labels unchanged.

The retained baseline has five capability-reference occurrences backed by four
distinct raw artifacts. All five retained preparation_artifacts rows count,
including equal bytes under separate reference IDs. Projection facts retain each
logical location's before/after byte count; no digest deduplication discount is
applied. Root source preparation projection omits only its existing outer
lastCommit and receipt.commit links. No storage row hash is computed.

The initialized summary branch contributes 1,307 summary-row bytes, one row and
866 checkpoint-material bytes; pending count does not change. The never-initialized
branch contributes zero to all four summary-resource deltas and retains the
inherited unavailable/null row/checkpoint semantics. These are projected source
measurements; the new packet does not include or publish a complete summary row.

## Logical locations, not transaction manifests

| Separate operation sample | Logical locations | Header/proof labels | Conditional count |
| --- | ---: | ---: | ---: |
| Workspace initialization | 14 resource keys | 2 | 16 |
| Draft prepare, five artifact occurrences | 9 | 2 | 11 |
| Exact draft cancellation | 9 | 2 | 11 |
| Root admission, initialized summary | 25 | 2 | 27 |
| Root admission, never-initialized summary | 22 | 2 | 24 |
| Exact retry comparison | 0 | 0 | 0 |

Admission locations contain six core source/control locations, thirteen fresh
resource keys, two preparation replacements, one workspace control replacement,
and three summary replacements only for initialized. Existing artifact locations
are read-only at admission. Only the new control/resource values and projection
facts are supplied; the complete root-source/summary values, read set, physical
lookups, header and proof outcome are not a new state/command here.

unqualifiedControls contains only the strings "header replacement" and "new
proof". There is no header value, schema/format number, manifest, before/after
commitment, storage-change hashing or eligible store. Counts include those two
obligations arithmetically, never as a receipt-only or partial publication proof.
The fixed fresh/retry comparison is not an authenticated operation or replay path.

## Checks and remaining gates

The generator/checker began as empty stubs. Initial positive derivation and
checker measurements failed before implementation; semantic denial tests failed
against the empty checker. Additional packaging measurements were asserted after
observing their absence fail. Meaningful denials cover omitted workspace/fresh
resource rows, draft-lifetime creation, foreign owner scopes, reset or restamped
peers, noncanonical/overflow counters, uncharged/doubled controls, wrong initial
versions/input/parent bindings, discounted artifact/preparation occurrences,
summary-branch changes, missing/extra logical keys, retry writes and fabricated
format/proof/qualification fields.

Pinned Node 22 verification passed all 1,662 documentation tests, including 55
new tests, OKF metadata/index and rendered-page/dead-link validation. All 20 OKF
regression tests, documentation drift, diff whitespace and worktree checks passed.
All 549 prior non-Markdown validation artifacts remain byte-identical; the prior
initializer proposal and first-report technical bodies are append-only. Five
temporary-copy guard removals (workspace resources, draft boundary, admission
values, occurrence charges and retry writes) were caught by their intended denial
tests. Owned files were not mutated for those checks. Existing VitePress circular/
large-chunk warnings concern unchanged bundler configuration and require no edit
for this packet. No passing check establishes actual source/store/authority proof.

Use pinned Node 22 from the repository root:

```bash
rtk proxy python3 docs/validation/generate-research-initialization.py --output /tmp/research-initialization-components.json
rtk proxy cmp docs/validation/research-initialization-components.json /tmp/research-initialization-components.json
rtk proxy node --test docs/validation/research-initialization-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

Every earlier artifact remains unchanged. Four origin labels and explicit
freshness/owner/proof assumptions cannot qualify actual original creation,
complete source/index coverage, mutable-before-state balances, current authority,
physical storage, ordinary lineage, child cutoff, recovery or atomic publication.
The candidate catalog and research original-preparation authorization remain
unclosed; no format-3/4/6 adoption or next-version selection follows. All existing
resource/row/command/discovery/store/index/physical limits still apply; these
partial measurements do not prove coupled maxima fit. No runtime/native/model
execution, Rust/Electron code, database/adapter/migration or executable product
plan is supplied. #1689/#1089 stay open; Gate 2 remains Investigate.
