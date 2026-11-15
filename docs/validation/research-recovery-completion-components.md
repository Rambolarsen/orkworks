---
type: Validation Evidence
title: Offline positive research recovery and completion components
description: Independent synthetic late-disposition source snapshots, paired recovery and completion records, exact retry and occurrence checks without native or physical-history qualification.
tags: [orkworks, research, recovery, completion, offline-validation]
status: draft
---

# Offline positive research recovery and completion components

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089); Gate 2 **Investigate**.
Base: `91414b67ee34f4fffd656837c2196a178e34bdac`.
The owner answered “keep going” after the three
[recovery/completion choices](../superpowers/specs/2026-11-15-research-recovery-completion-design.md#three-owner-choices-and-bounded-next-packet).
That acceptance covers these offline components only. Earlier accepted protocols,
families, domains and caps are unchanged. The old negative recovery example and
all previous non-Markdown validation artifacts remain byte-identical.

## Packet and source boundary

The independent [Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-recovery-completion.py)
emits the [UTF-8/LF JSON oracle](research-recovery-completion-components.json).
The finite [JavaScript checker](research-recovery-completion-components.mjs) and
[tests](research-recovery-completion-components.test.mjs) verify source joins,
literal historical/live versions, original decisions and projections. No product
code or persisted codec is implemented. The checker recognizes this one complete
decoded example; it is neither a general wire decoder nor a runtime reducer.

Source identities, definition/approval/reservation/allocation, original attachment,
frozen P, native-directory descriptors and parent/child runtime references come
from the unchanged prefix packet. The original prepared input is the prefix's
first input. Other continuation/completion bytes are fingerprinted for provenance
and preservation, not imported as a current store or replayed into this example.
New task/launch/run/session/selector snapshots are explicitly constructed from
those source fields. The old needs_parent_result observe-terminal example is
never relabelled into a completed task.

This packet supplies a hypothetical original UI turn. It makes no Codex Stop or
native capability claim. Original successful spawn, preparation/ordinary/source
proof, authenticated UI/parent/Main action, report/review eligibility, actual
native terminal ownership/quiescence, currentness, complete discovery and the
actual compound outcome are **supplied, unqualified facts**. Hashes and agreement
between two implementations cannot prove those facts.

CommitRef labels have sequence numbers 1..8 and SHA-256 fingerprints of explicit
unqualified labels. They express the stipulated ordering only. They are not
storage change hashes, direct proofs or a legal history. No selected format,
header, manifest, row/change hash, store, publisher, live event or actual C is
created. Candidate 57 and conditional admission counts 28/25 remain conditional.

## Source ordering and one compound outcome

| Supplied source boundary | Live task | Run | Retained fact |
| --- | ---: | ---: | --- |
| Original UI turn, label 3 | 7→8 | 10→11 | Exact original turn, original child runtime |
| Late interruption, label 4 | 8→9 | 11→12 | needs_parent_result preserved; original proven spawn retained, launch interrupted |
| Explicit completed result, label 5 | 9→10 | 12→13 | reported_complete; original request/receipt, plan still blocked by disposition |
| Current terminal bridge, label 6 | 10→11 | 13→14 | producerRevision 4, result retained, native settlement unresolved |
| Captured H label 7 / outcome C label 8 | 11→12 | 14→15 | Recovery and automatic completion share original H/C |

These are finite source views, not five executed transitions. The prefix's frozen
assignment taskVersion remains 2; turn acceptedTaskVersion remains 8 and result
acceptedTaskVersion remains 10. Neither historical receipt is retokened to 12.
Both decisions capture the same original run/H. Completion's recovery trigger
pins the entire original recovery digest; it does not read a repaired poststate
as its prestate or introduce C into its body.

The fixture-only LateDispositionBindingsV1 input supplies the full captured
launch-interrupted blocker and its original reservation/run/task/launch snapshots
at interruption label 4. It is outside both decision bodies and digests and is
not a catalog family or native proof. Exact blocker body/coverage, distinct
original origin, readiness, proven spawn identity and supplied-resolved status
are required. Missing, duplicate, altered, unknown or unresolved bindings deny
the selected positive case. The checker is bounded to the one-blocker example;
it does not claim general discovery or qualification of all 1..16 bindings.

The finite after-values repair launch interrupted→ended and clear only that
disposition blocker. Live task advances once with reported_complete and its exact
coordinationResult unchanged. Plan becomes complete/version 3; selector clears
currentPlan through the run owner and selects completedResearchPlan. Both
receipts and changed local controls bind the same supplied C. The already
published terminal bridge is byte-identical before/after recovery.

The independent recovery-approval-required blocker remains; consequently this
example's run stays blocked after true plan completion. Actual grant revocation,
summary invalidation, stage/pause/clarification precedence, collection activity,
ordinary reservation/session/worktree retention, capacity and all other native/
consumer/index/resource effects remain mandatory owning obligations. They are
not fabricated as executed rows here. This packet does not exercise a paused
parent or gate-free synthesis branch and cannot establish those transitions.

## Admission, retry and denial checks

Pure supplied-fact predicates require current Main authority, rate, exact shown
context, current owner, original complete source/membership proofs and known
committed disposition before original lookup. For a new selected outcome they
also require proven original spawn, native settlement, plan eligibility, grant
fencing and capacity facts. Cancellation-first/unknown facts deny; no timeout
rollback, replacement owner or late spawn is modeled.

An identical recovery body resolves both original receipts, even with supplied
later live versions, count exhaustion or terminal run state. Changed bytes
conflict. A missing or conflicting original completion companion denies; there
is no live predicate fallback, new completion, increment or write. These are
finite no-write oracles, not authentication, transport, lease or cutoff tests.

Negative mutations reseal decision digests and remeasure occurrences before
checking semantic joins, so stale hashes alone cannot hide missing validation.
They cover wrong readiness/result ordering, unknown first spawn, changed original
turn/result/frozen versions, incomplete bindings, source/terminal/runtime drift,
stale H/versions, wrong trigger, independent blocker loss, rewritten outcome,
double increments, new allocation and inconsistent C/selector poststate.

## Independent bytes and containing limits

| New retained occurrence | Key plus committed projection | Complete key/value wrapper |
| --- | ---: | ---: |
| Recovery record | 12,103 bytes | 12,499 bytes |
| Lifecycle completion record | 7,333 bytes | 7,729 bytes |

Both copied H/run snapshots and all embedded origins stay charged. Only each
outer lastCommit and its own receipt.commit are excluded. Their control charges
sum to 19,436 bytes, with no discount, against existing shared 8 MiB/run and
64 MiB/workspace control pools and the 32 MiB run aggregate. These are two new
occurrences only, not a ledger balance or complete outcome delta; changed owners,
all other retained copies, indexes/proofs/physical/scratch costs remain separate.

The checker applies 2 MiB decision wrappers, 4 KiB receipts, 1 MiB run snapshots,
16 KiB task/session/plan/selector and 32 KiB launch snapshots. A necessary sum of
the two wrappers is below the smaller 1 MiB paired-job ceiling; omitted owners
mean this proves no complete job fits. A width-only padding test keeps individual
wrappers below 2 MiB but exceeds the paired 1 MiB ceiling and must reject. It is
not a legal padded source body. Smaller command/discovery/store limits remain
unqualified and binding; no quota is increased or missing ledger created.

## Reproduction and remaining gates

```bash
rtk proxy python3 docs/validation/generate-research-recovery-completion.py --output /tmp/research-recovery-completion-components.json
rtk proxy cmp docs/validation/research-recovery-completion-components.json /tmp/research-recovery-completion-components.json
rtk proxy node --test docs/validation/research-recovery-completion-components.test.mjs
rtk proxy pnpm --dir docs docs:build
```

Initial packaging tests exposed the absent oracle; after generation the empty
checker failed the semantic/denial tests. Context-order and lower-width tests
also failed against missing guards before implementation. Literal contract
checks independently pin reported_complete, historical/live versions and byte
measurements; generator/checker agreement alone is not semantic evidence.

Actual complete source/native/ordinary participation, auth/lifecycle fencing,
nonblocking stop, outcome atomicity, direct proofs and complete physical catalog/
index/initializer/history remain open. Broader scope-resolution/review/
clarification/execution source variants need their own closed source/effect
contracts. The next written investigation should map those remaining owners and
supported variants before treating the conditional candidate inventory as closed.
No runtime/native/model call, product adapter, migration, executable product
plan or activation is supplied. Keep #1689/#1089 open and #740 closed research.

## Review and verification disposition

One bounded round used two fresh read-only Luna reviewers for contract correctness
and evidence/denial completeness. Both found no material required change. Their
reviews were static; neither executed project code or qualified the supplied
facts. The author corrected a generator link after VitePress rejected its local
Python path, following existing generator-link conventions.

All 59 new component tests and the full 1,881-test documentation suite pass,
alongside 20 OKF regressions. Exact generation, documentation metadata/index,
rendered links and close-out checks are recorded with the PR. All 565 prior
non-Markdown validation artifacts remain byte-identical, including the negative
recovery example, and the source design is append-only. No full product verifier
or product/native tests were run, consistent with the user's no-runtime scope;
no broad repository/native success is claimed. Existing bundler notices and
another owner's stale session-workflow-split worktree require no changed claim
or unrelated edit. Task-worktree reporting succeeded; primary residue was left
untouched. Hosted exact-head review and CI are separate PR evidence and confer
no source/native/history qualification.
