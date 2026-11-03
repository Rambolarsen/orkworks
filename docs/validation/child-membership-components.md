---
type: Validation Evidence
title: Retained child membership component relations
description: Independent synthetic forward/reverse source/control, historical proof/debit and artifact custody relations plus conditional discovery-boundary traces; no complete physical index or store qualification.
tags: [orkworks, hierarchy, consumers, membership, custody, qualification]
status: stable
workflow_status: component-relations-only
---

# Retained child membership component relations

Tracking: [#1183](https://github.com/Rambolarsen/orkworks/issues/1183),
[#1176](https://github.com/Rambolarsen/orkworks/issues/1176).
Contract: [accepted scoped membership/discovery](../superpowers/specs/2026-11-03-child-consumer-membership-design.md#scoped-owner-acceptance-and-component-handoff--2026-11-03).
Base: `108a82880edbdc87a073c902c78e7e17c3c90114` (#1341).
Every identity, event, commit link and outcome is synthetic. Alternative control
states share keys and are not one coherent current store. The relation union below
exists to compare discovery directions; it cannot be installed as a current index.

## Delivered packet

The standard-library [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-child-membership-vectors.py)
produces [77 pinned relation cases and 58 reverse targets](child-membership-vectors.json).
A separate [JavaScript checker](child-membership-components.mjs) derives them from
the existing [source](child-source-components.md) and
[control](child-control-components.md) packets, rather than trusting a supplied
affected list. The [tests](child-membership-components.test.mjs) reproduce the
entire generated packet independently and compare it byte for byte.

- Exact original worker/reviewer plan/configuration/input/evidence/assignment-source
  and approval/reservation/allocation/attachment references retain original IDs
  and domains. Eight explicit synthetic source-metadata wrappers supply their own
  lastCommit links and format-4 row projections; reservation/allocation/attachment
  origins are not inferred from an approval receipt. These links have no original
  manifests and do not prove a coherent event history. Metadata stays metadata,
  with no invented ObjectRef purpose.
- All 45 existing control variants contribute typed direct references. Prepared
  replacement inputs keep prior-plan references; older source plans retain their
  original revision-one input binding. No replacement plan is manufactured.
- Frozen recovery snapshots retain their exact control-origin row projection and
  original CommitRefs. Repeated internal references remain hashed in the original
  body; graph reachability deduplication does not refund retained bytes or work.
  Direct proof targets stop at the explicitly required original/expected/new links,
  with no invented recursive predecessor chain or link-only row rewrite.
- Two narrowly constructed unassessed evaluation/artifact versions demonstrate
  reciprocal custody and stable stream mapping. The original artifact version,
  producer/target source and evaluation remain referenced after the correction.
  Their fields, raw evaluation sizes/content hashes and frozen declaration bounds
  are checked; this is no evaluation publication, quality or native-source proof.
- One complete synthetic debit body preserves its full permit key, original
  producerSource/artifact/evaluation/mapping and three owner-ledger target keys,
  plus expected/new direct proof links. It pins version one rather than a newer
  artifact head. No debit transaction, permit, ledger value or content read is
  performed or proven. Its exact six-row effect remains unchanged.

The checker returns only unverified-membership-components-only. Its reverse view
is rebuilt from the independently checked relation cases. A missing/extra/changed
forward or reverse member is denied, including historical, foreign and unsupported
metadata/ObjectRef substitutions even when outer hashes are recomputed.

## Conditional discovery traces

The trace helper returns only conditional-discovery-trace-only. Synthetic events
make budget accounting observable: page, returned index occurrence, owning row,
joined row and missed key each consume their defined observation; an additional
owned copy consumes canonical bytes again; an in-memory reuse consumes neither.
Repeated observations of identical bytes still count. It performs no query and
cannot measure actual native work, copying, index completeness or memory peaks.

Tests cover 4,095/4,096/4,097 observations; 16 MiB minus one byte, exactly 16 MiB
and one byte above; an extra end-of-range probe; changed captured headers; and
incomplete discovery at the limit. Completion/header values are supplied synthetic
premises, never evidence from a storage owner. The byte-boundary string traces
are arithmetic inputs, not valid producer records or a maximum-profile store.
Existing lower row/command/materializer/query/physical limits remain independent.

## Verification and remaining gates

Run from the repository root with Node 22:

```bash
rtk proxy python3 docs/validation/generate-child-membership-vectors.py --output /tmp/child-membership-vectors.json
rtk proxy cmp docs/validation/child-membership-vectors.json /tmp/child-membership-vectors.json
rtk proxy node --test docs/validation/child-membership-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

The initial checker stub failed 111 of 114 cases before implementation. All 130
focused checks now pass, including a later red/green regression requiring an
interrupted known-successful spawn to retain its original runtime-history pin.
A proposed spawn-pending generation still creates no successful-history target.
The suite also rejects altered raw artifact hashes/declarations/target refs,
correction predecessors, debit producer/permit/counter/B/owner bindings and quota
exhaustion. Original source/control catalogs and tests remain unchanged.

This packet exercises the selected new source/control membership slice and
artifact/debit relations. It does not instantiate every existing #744 head,
payload, summary/pending lineage, reserved/consumed receipt, candidate, compact
claim/lookup, ledger or manifest-family recipe from the accepted registry. A
quota/proof target here is a locator, not its owning row, value or proof. Root
preparation targets likewise do not establish fresh initialization or actual
ordinary-session attachment. No missing source or ledger is repaired.

Actual producer/current-authority participation, complete physical indexes and
absence, coherent metadata-only artifact precharge, original direct proofs,
transactional maintenance, native query/cancellation/commit behavior, complete
Q,S,D,P,K effects and source-valid old/new states remain unqualified in the
[handoff](../superpowers/plans/2026-10-31-child-review-source-qualification.md).
No database, migration, new table/resource, runtime caller, native/model probe,
artifact read or grant is added. #1183/#1176 remain open; #740 remains closed.


An author origin-link regression failed before replacing approval-derived source
metadata proof targets with explicit per-row synthetic wrappers. Each wrapper is
checked against the exact original source body/key and its own canonical row
projection; approval receipt/commit joins remain exact. No absent original source
manifest or real reservation/allocation/attachment event is thereby supplied.

## Bounded review and verification scope

Root is the sole writer. One read-only Luna reviewer checks reference/identity/
proof/debit compatibility, and a second checks independent expectations, trace
coverage and qualification claims. One round, at most three active agents
including root; no writer delegation, worktree metadata reporting or model/native
probe is authorized for reviewers. Existing accepted choices are reused.

The full repository verification command was invoked but stopped at prerequisites:
this docs-only worktree has no desktop dependencies installed. It did not run
runtime suites. Documentation build/CI and review dispositions are recorded with
the PR. Existing VitePress circular/large-chunk warnings concern unchanged site
bundling rather than these relations.

Both independent read-only Luna reviewers inspected code head
`e9008a8eb5c7816498eefca33da3e58a3cff8b8c` and reported no material findings.
Correctness covered source/control/debit/proof identity and discovery accounting;
coverage reviewed the independent expectations, negative cases and stated limits.
No second review round was needed. Node 22 documentation build passed all 1,202
checks, including the 130 new tests, with metadata/index/site validation complete.
Diff, documentation and worktree checks passed; a foreign merged worktree warning
was left to its owner. These results establish no actual index or runtime behavior.
