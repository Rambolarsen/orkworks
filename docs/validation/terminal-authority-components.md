---
type: Validation Evidence
title: Offline terminal UI authority and receipt components
description: Conditional scope ordering, local pending predicates and exact receipt-v2 membership and byte checks without native or complete-publication qualification.
tags: [orkworks, hierarchy, reports, authority, fixtures]
status: draft
---

# Offline terminal UI authority and receipt components

Tracking: [#1669](https://github.com/Rambolarsen/orkworks/issues/1669),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), preparation
[#742](https://github.com/Rambolarsen/orkworks/issues/742), initiative
[#738](https://github.com/Rambolarsen/orkworks/issues/738).
Source base: `30b88342ae193492a02050189aeedf229da7a8a7`.
Contract: [owner-accepted protocol and codec](../superpowers/specs/2026-11-10-terminal-ui-authority-protocol-design.md#scoped-protocol-acceptance-and-component-handoff--2026-11-10),
reviewed in [#1664](https://github.com/Rambolarsen/orkworks/pull/1664).

**Evidence: offline conditional components only.** No listener, HTTP route,
Electron IPC, production module, real credential, SQL store, lease, native/model
probe or runtime plan is used. Passing this packet does not establish any profile
eligible for production, native commit ordering or a complete admitted publication.
Existing [terminal publication](report-publication-components.md),
[R2 source](r2-report-components.md) and broader Gate 2 limitations remain intact.

## Artifact ownership and independent oracles

- `terminal-authority-components.mjs` is documentation-only model/checker code.
  It contains pure scope transitions, main pending/delivery predicates and the
  receipt-v2 normalization/commit-membership/byte predicates. No production caller
  imports it. It reuses the existing report source validator and canonical JSON.
- `terminal-authority-components.test.mjs` runs 64 behavioral tests in `docs:check`.
  Cases include literal expected results, malformed/self-consistent adversaries,
  isolated size boundaries and six independently specified traces.
- `generate-terminal-authority-vectors.py` uses only Python stdlib, never imports
  or invokes the JS model. Expected scope state/status tuples are literal; it
  independently computes canonical receipt bytes, row/manifest hashes and sizes.
- `terminal-authority-vectors.json` pins raw hashes of the original publication
  and R2/report-profile inputs and retains the exact new oracles. Existing input
  catalogs are unchanged. LF checkout attributes preserve its retained bytes.

Run from repository root using Node 22:

```bash
node --test docs/validation/terminal-authority-components.test.mjs
pnpm --dir docs docs:check
pnpm --dir docs docs:build
python3 docs/validation/generate-terminal-authority-vectors.py --output /tmp/terminal-authority-vectors.json
cmp docs/validation/terminal-authority-vectors.json /tmp/terminal-authority-vectors.json
```

The test suite also independently regenerates into a fresh temporary directory,
compares all bytes, and removes that directory. It does not rewrite the retained
oracle. No network, environment credential or installed coding-tool configuration
is read by the generator/model.

## Qualified component behavior and boundaries

| Component | Observed logical behavior / boundary |
| --- | --- |
| Closure high-water mark | Close-before-open denies the delayed open; old duplicate closes preserve a newer open scope; gaps cannot raise counters; full u64 counters never wrap |
| Exact admission and retirement | Admission-first retains one exact request across closure, blocks another decision/new scope and requires captured disposition/owner predicates before retirement; close-first cannot allocate a decision |
| Health/rate/control predicates | Missing captured owner health/source/lease/rate denies new admission; reserved close can proceed without a read-rate token. This tests conditional branches, not actual enforcement or scheduling |
| Local pending state | A second pending/unknown request cannot replace the original. No-active with still-open scope does not release it; matching closed-through and no unsettled decision releases only with durable outcome still unknown |
| Local response context | Original frame, request and exact scope plus current open state are required. These are synthetic context values, not actual Electron sender proof |
| Source normalization | Existing closed ResearchReport validator checks captured approved stream, questions, runtime lineage, references/excerpts, source shape and digest before receipt/request joins; self-consistent extra/missing/changed-question reports reject |
| Historical receipt | Separate current auth/rate/source/producer predicates precede original request/response/row/manifest joins. No live decision or current-lifetime equality is required; no original authorization is rewritten |
| Version/metadata joins | v1 input is not silently adopted by the v2 validator; unknown fields, changed authority metadata, response/request/commit/key substitution and missing/duplicate manifest membership reject |
| Receipt bytes | Complete key plus full value fits 2,048 exactly and rejects at 2,049. Retained-byte charge excludes only the outer lastCommit, preserving the existing report resource projection |

The scope model starts from explicitly supplied valid synthetic owner state.
Counter-edge seeds are not an admitted producer history. Keeping that same state
through a modeled handoff demonstrates that its counters are retained; it cannot
prove a real namespace writer preserves them or that a restarted process owns a
new lifetime. Captured predicate booleans stand for **unqualified** original owner,
authentication, live rates, lease, source access, producer and durable disposition.
They do not constitute permission or an authenticated producer constructor.

The receipt checker accepts parsed fixture rows and canonical request strings.
It is not a production storage parser or universal schema registry. Control
canonical-string checking rejects duplicates/unknown members and oversize input;
HTTP framing, duplicate header handling and main-only header minting remain
transport qualification tasks. The report profile is captured source context,
not authority copied from the submitted report.

## Isolated receipt membership is not a legal publication transaction

The new fixture retains one receipt-only manifest membership component using the
existing format-3 domains and predecessor reference. It deliberately omits report,
head, run, quota, summary and other effects required by a complete report-publication
command. Its command label/hash consistency does not make it a valid terminal
producer command or recovery proof. `originalProducerQualified` is an explicitly
supplied assumption, not a conclusion of this component. Never pass this manifest
to a production publication/recovery adapter or use it to qualify a whole store.
The v1-shaped input to the v2 constructor is a fixed source-field template for a
separate synthetic new-command context, not authority to migrate an accepted v1
row. The model never mutates a stored history or existing catalog; the fixture
binds its new value to its separate receipt-only proof. No automatic v1 upgrade
or original-producer adoption is established.

The checker verifies original manifest identity/hash, exact receipt key membership,
its row digest/link, request/response binding and the unchanged 2 KiB cap. It does
not verify complete effect discovery, all other row bodies, ledger initialization,
source custody, physical index membership, durability, lease possession or a live
admission event. A fresh current UI context may inspect a historical receipt only
when those separate real source/authority gates are qualified. No volatile decision
is restored from its stored policy annotation.

## Measured receipt accounting

For the retained C1 component, complete receipt key/value metadata is **1,244 bytes**;
its report resource projection is **1,044 bytes**, excluding only outer lastCommit.
The new authorization member adds **186 bytes** for scope/admission sequence 1.
The exact original report, normalized request and wire response bytes remain pinned
and unchanged; UI admission lifetime differs from copied report runtime lineage.
Each run/workspace report ledger must charge that same projection under its existing
ownership rules; this packet does not construct or qualify those ledgers.

At maximum u64 scope/admission width the added member is **224 bytes**. The existing
1,925-byte width witness becomes **2,149 bytes**, **101 over** the unchanged 2,048-byte
cap and must reject. This width construction is not a source-valid/admitted run.
The cap-only predicate is tested independently of source validation, so a source
mismatch cannot conceal a missing size check. No quota expansion, provenance
omission, #744 receipt credit, new table or partial publication is used to fit it.

These are exact compact logical byte measurements, not RSS, physical SQL/index/
journal usage, whole-command/changed-set packing, visited-row or native latency
qualification. Smaller individual records fitting does not establish a maximum
eligible profile or complete legal publication transaction.

## Failing-first and review evidence

Initial scaffold run: 39 failed / 3 passed out of 42. After implementing those
components all 42 passed. Added owner-health, pending and membership cases exposed
8 failures before their checks; 51 then passed. Two isolated size-predicate cases
failed before implementation. Independent vector/regeneration cases then passed.
A receipt review identified the missing closed-report validation: three newly
written self-consistent extra/missing/changed-question cases failed first, then
all 64 passed after reuse of the existing captured-profile report validator.

Temporary behavioral mutations removing the close-watermark advance, requiring
historical metadata to match a current lifetime, and raising the receipt cap to
4 KiB each failed the applicable tests. All mutations were restored. Local logs
are under `/private/tmp/orkworks-terminal-fixtures-*.log`; they are local execution
evidence, not distributed source or native receipts.

One fresh scope correctness review found no issue in the pure transition/pending
logic. A separate receipt/byte/oracle review found the normalization gap described
above; its correction uses the owning validator rather than widening source rules.
Review covers these offline components only. Runtime/source-valid terminal proof,
complete consumer/writer coverage, actual control admission/priority, sender and
counter ownership, lease/lifecycle/storage/native/recovery and separately approved
executable plans remain open. Keep #1089/#742/#744/#746 open.

The receipt reviewer separately checked the targeted normalization correction and
confirmed it resolved the finding without a source-schema or authority widening;
the 64-test suite passed in that follow-up. Full docs validation passed 1,268
tests, including all 64 new cases. The repository verifier stopped at its
prerequisite check because this owned worktree has no desktop dependencies; no
broad Rust/desktop pass is claimed. Applicable hosted results are recorded on
the PR before its terminal state.
