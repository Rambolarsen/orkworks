---
type: Validation Evidence
title: Offline research continuation components
description: Independent alternative turn, coordination and original-attempt recovery source bytes with explicit hypothetical event and publication limits.
tags: [orkworks, research, continuation, recovery, qualification]
status: draft
workflow_status: continuation-components-only
---

# Offline research continuation components

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: `cd9816b31dbd9493350695ca4b26fb037c05a283`.

The owner [accepted the three continuation choices](../superpowers/specs/2026-11-15-research-continuation-design.md#scoped-owner-acceptance--2026-11-15)
for offline fixtures only. This packet checks finite source representations for
turn readiness, explicit parent outcomes and original-attempt recovery. It does
not perform those events or prove their original source/authentication/outcomes.
Gate 2 remains **Investigate**.

## Artifacts and hypothetical input boundary

- [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-continuation.py) reads only the unchanged prefix JSON and independently builds typed source/snapshot bodies, domain digests and canonical byte measurements. It never calls JavaScript.
- [JSON oracle](research-continuation-components.json) retains three turn variants, three coordination outcomes and four recovery kinds as **separate alternative examples**, not rows in one store or a legal event sequence.
- [JavaScript checker](research-continuation-components.mjs) independently reconstructs exact finite bodies from original source records and compares keys, values, digests, original joins, policy labels and byte measurements. It reuses the unchanged prefix source validator, not the Python derivation.
- [Tests](research-continuation-components.test.mjs) independently regenerate bytes, reseal semantic mutations, exercise supplied-fact retry/count/width oracles and pin literal measurements/digests. The oracle has explicit LF attributes; paths with spaces/percent signs preserve UTF-8 bytes.

The reused source profile names **fixture-tool**, not an eligible Codex adapter.
The codex_stop_receipt example is therefore a hypothetical shape under an
explicit supplied-handler assumption; it establishes neither Codex capability,
actual Stop delivery, installed integration nor native event authenticity. The
turn row is the proposed durable sidecar one-shot receipt; no extra unspecified
upstream receipt is fabricated. Runtime refs and native directory descriptors
are synthetic source data. The terminal example supplies a terminal bridge
snapshot as an observation, not a process event or coherent ordinary transaction.

Original source bodies supply immutable IDs/configuration/plan/reservation/
allocation/attachment/frozen P bindings. Their occurrence in the old JSON does
not prove those rows exist in any alternative here, nor defeat or establish a
no-child absence claim. All source/proof/native membership facts remain supplied
and unqualified. Snapshots are explicitly constructed from those source fields,
never copied from a later running/report control view and relabelled as prestate.

The raw input SHA-256 fingerprints documentation bytes. Symbolic CommitRef
changeDigest fields hash unqualified labels, never manifests or storage domains.
No storage header, manifest, row/change hash, selected format, recognized profile,
complete store, publisher, wire decoder or runtime reducer is generated. Old
format-3/4/6 components remain unchanged and are not retokened or imported.

## Alternative turn and coordination examples

The three turn source bodies cover codex_stop_receipt, ui_action and
child_terminal_without_receipt. Stop/UI pin the original child RuntimeRef; UI
additionally retains its shown context and sidecar generation. Terminal retains
the full typed terminal bridge snapshot and its original committed-origin label.
The source-owned event/authentication and direct proof are assumptions, not
inferred from a digest or kind string.

Each gate has its exact TaskKey/reservation/child and sample live version 7→8,
while frozen P remains at 2. Separate task views change running→needs_parent_result;
run version labels change 10→11. These are illustrative no-intervening-event
values, never admission constants or complete mutation proofs. No task result,
report/head, capacity release or launch grant follows from the turn source.

The three parent request alternatives are completed, blocked and failed, each
bound to the original UI gate. They retain the complete original request/domain/
receipt, task version 8→9 and run labels 11→12. coordinationResult pins the original
requestDigest; reported status is the exact selected outcome. Completion's report/
review eligibility is explicitly supplied and not established by the request.
Blocker/grant-fencing/summary/index/publication effects for failed or blocked
outcomes remain mandatory actual effects, not produced by these finite views.

The supplied-fact coordination retry helper checks current-authority/rate labels
before accessing the retained row. Identical original bytes return the original
receipt even when a supplied live task version has advanced; changed bytes
conflict. The recovery retry helper similarly checks supplied current UI authority
before original-body comparison and bypasses new-decision count/live CAS. They
are pure no-write lookup oracles, not credential validators, rate limiters,
HTTP handlers, currentness proofs or complete admission implementations.

## Four original-attempt recovery examples

| Alternative | Selected snapshot/decision boundary |
| --- | --- |
| settle-no-child | Interrupted original allocation, null allocation/attachment/session and no owned observations; complete absence/native quiescence remains an external assumption |
| confirm-original-allocation | Original supplied allocation/ref and observations; captured interrupted launch has no established allocation/attachment/session, so the example does not claim a proven outcome |
| confirm-original-attachment | Original allocation/attachment with prepared session/runtime null, captured interrupted spawn and original ownership/proof supplied |
| observe-terminal | Original allocation/attachment/spawn history and terminal session snapshot, needs_parent_result; the illustrative recovery receipt leaves the unchanged live task version at 8 |

Each decision retains exact run/task/launch/session keys/bodies/origins and H,
UI context/generation, original reservation and native descriptors. The captured
run version is 20 and the receipt's sample version is 21. Other recovery samples
use live task version 5→6; these differing snapshots are alternatives, never
one task's reconstructed history. Frozen assignment identity stays at 2.

Two original task-related blockers are retained exactly, including the independent
recovery-approval-required blocker. The packet checks missing/changed blockers,
expected-version/key/reservation/frozen identity joins, wrong kind/nullability,
wrong child families/domains and copied origins. It publishes no blocker removal,
capacity reconciliation, native recovery, retry task or grant restoration.

The count helper checks a **supplied** run inventory's unique keys and room for
one genuinely new decision below 32. An existing key requires original retry
binding; it is not free capacity. The tests construct 31/32-key examples without
pretending to provide their bodies/history/proofs or discover complete membership.
Actual authenticated replay and full historical/source inventory remain unqualified.

## Independent occurrence measurements

These values are canonical UTF-8 key plus committed projection / complete wrapper
bytes, independently generated in Python and remeasured in JavaScript:

| Alternative | Charge bytes | Wrapper bytes |
| --- | ---: | ---: |
| Stop turn | 1,341 | 1,541 |
| UI turn | 1,504 | 1,704 |
| Terminal turn | 2,878 | 3,078 |
| Completed coordination | 1,540 | 1,936 |
| Blocked coordination | 1,538 | 1,934 |
| Failed coordination | 1,537 | 1,933 |
| Settle no child | 8,213 | 8,609 |
| Confirm allocation | 9,078 | 9,474 |
| Confirm attachment | 11,445 | 11,841 |
| Observe terminal | 11,949 | 12,345 |

Projection excludes only outer lastCommit and receipt.commit on the two
receipt-bearing families. Embedded terminal/recovery snapshot origins and H
remain committed and fully charged. These are independent occurrences in
alternative examples; do not sum them as one command or one task's history.
They belong to the existing shared 8 MiB/run and 64 MiB/workspace control pools
and 32 MiB run aggregate. No full ledger balance/delta, summary/index/proof/store
or complete job/transaction packing is supplied. Existing source/report and other
control copies remain separately charged; there is no sharing discount or refund.

The checker/tests enforce 64 KiB turn, 16 KiB coordination, 2 MiB recovery wrappers,
4 KiB receipts, and 1 MiB run / 16 KiB task/session / 32 KiB launch snapshots.
The independent 1 MiB lower job ceiling is tested separately: a width-only run
snapshot fits exactly its own ceiling while the containing recovery component
already exceeds the job ceiling. Padding is a measurement vector, not a legal
source body. All smaller worker/command/discovery/physical limits still win;
these small examples prove no coupled maximum or full command fits.

## Reproduction and qualification boundary

Initial generator/checker stubs failed both packaging tests. Once the oracle was
generated, 43 of 45 tests failed against the empty checker. The recovery retry
case separately failed against its null stub before implementation. These checks
exercise finite decoded examples; they are not a duplicate-aware research wire
codec, real event dispatcher or physical cutoff test.

From the repository root with pinned Node 22:

```bash
rtk proxy python3 docs/validation/generate-research-continuation.py --output /tmp/research-continuation-components.json
rtk proxy cmp docs/validation/research-continuation-components.json /tmp/research-continuation-components.json
rtk proxy node --test docs/validation/research-continuation-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

No authority/native event/coverage/currentness/command C/proof/outcome field
qualifies itself. The eight policy labels preserve late Stop/UI idempotence,
exact retry/conflict/revocation order, unknown recovery retention/no retry, count
exhaustion, live-child capacity and stage-gate retention. Comparing those labels
runs no lifecycle schedule or actual native recovery. Real source participation,
nonblocking ordinary stop, complete catalog/index/proof/initializer/history,
finish/stage-cancel/run-cancel/synthesis/execution continuation and separately
approved activation remain open. The candidate inventory is 54 logical names,
not a final catalog or maximum. #1689/#1089 stay open and #740 stays closed
research. Gate 2 remains Investigate; no runtime/native/model call, Rust/Electron
code, adapter, migration or executable product plan is supplied.

## Review and verification disposition

One bounded round used two fresh read-only Luna reviewers for contract correctness
and evidence/denial completeness. Neither found a material correction. A minor
label was clarified: duplicate alternative case labels and duplicate stored
recovery keys are separate tests. Both reviews were static, without executing
native/runtime/model/database code or qualifying any open producer gate.

All 48 new component tests pass; all 1,752 documentation tests, pinned Node 22
VitePress build, metadata/index/dead links and 20 OKF regressions pass. Independent
regeneration preserves exact UTF-8/LF bytes. Six temporary-copy guard removals
were caught by their intended tests: turn body, turn domain, coordination domain,
recovery snapshot body, 32-decision count and the independent lower job ceiling.
Owned files were not mutated for that audit. All 557 prior non-Markdown validation
artifacts remain byte-identical and the source specification is append-only.

The full repository verifier stopped at prerequisites because desktop dependencies
are absent from this docs-only worktree; no full-repository success is claimed.
Existing bundler circular/large-chunk notices concern unchanged configuration.
Diff/documentation/worktree close-out and rendered evidence checks are recorded
with the PR. The separate stale session-workflow-split worktree belongs to another
owner and was left untouched. Task-worktree metadata reporting was unavailable
(invalid session environment); work stayed in the isolated owned checkout and
primary lockfile residue was untouched. Hosted exact-head review/CI supply no
additional native/source/proof qualification and are recorded separately in the PR.
