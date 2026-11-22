---
type: Validation Evidence
title: Offline research terminal-completion components
description: Accepted execution final-completion source examples with typed trigger reuse, absorbing terminal transition and label-assertion grants; supplied authority and launch-chain facts stay unqualified.
tags: [orkworks, research, execution, completion, terminal, qualification]
status: draft
workflow_status: terminal-components-only
---

# Offline research terminal-completion components

Tracking: [#1689](https://github.com/Rambolarsen/orkworks/issues/1689),
[#1089](https://github.com/Rambolarsen/orkworks/issues/1089), initiative #738.
Inspection base: the merged terminal-completion contract plus the acceptance
record (`b9f2524d8acf`).

The owner [accepted the five terminal-completion choices](../superpowers/specs/2026-11-19-research-execution-completion-design.md#scoped-owner-acceptance-—-2026-11-19)
for offline components only on 2026-11-19. These are separate decoded examples
and supplied-fact oracles, not an event sequence, complete store, publisher or
authority implementation. Gate 2 remains **Investigate**.

## Artifacts and source boundary

- [Independent Python generator](https://github.com/Rambolarsen/orkworks/blob/main/docs/validation/generate-research-terminal.py)
  reads the unchanged prefix, continuation, completion and execution-proposal
  packets; it never calls JavaScript.
- [JSON oracle](research-terminal-components.json) retains the synthetic
  execution-proposal prestate labeled against that contract's accepted
  choices, the execution task's turn gate (`child_terminal_without_receipt`
  variant with synthetic runtime under supplied-origin labeling), the
  completed coordination outcome with its flat family row, the typed
  coordination trigger driving the lifecycle plan-completion record, the
  absorbing terminal transition (run control `complete` with nulled pause
  fields, preserved raw gates, cleared pointers, unchanged selector), the
  collection/expectation retirement, the live-child capacity row with rule
  labels, the failed/blocked non-completion variant, the accounting and
  catalog labels and effect labels.
- [JavaScript checker](research-terminal-components.mjs) independently
  rebuilds finite source bodies, validates exact keys/values/domains/joins and
  digest exclusions, the trigger union, receipt limits, and supplies the
  completion/retry helper oracles.
- [Tests](research-terminal-components.test.mjs) regenerate exact UTF-8/LF
  bytes and exercise twelve semantic-mutation denials.

## What the packet establishes

- The packet-prestate authority boundary: the execution-proposal prestate is
  synthetic supplied context labeled against that contract's accepted
  choices, with the post-launch state `awaiting_execution_approval` under an
  explicit supplied label (the launch chain's own state writer remains
  separately gated).
- The accepted typed trigger union drives plan completion in the same-C
  lifecycle machinery; one original record per exact execution `PlanRef`;
  the receipt reports the target plan version; no second completion record.
- The absorbing terminal transition in the same compound outcome: `complete`
  in the sole run owner with pause-only fields nulled and raw gates preserved;
  complete authority revocation with grants as label assertions over the
  retained approval facts; `run_closed` waits as effect labels; collection/
  expectation retirement per the accepted finish pattern with selected
  reports retained; pointer clears with the selector's
  `completedResearchPlan` unchanged (the execution plan identified through
  its own control/record).
- Terminal state lives only in `research_run_controls` — no
  TerminalReportRunV1 cast, no `run_states` rewrite, no second counter.
- A live child still consumes capacity after terminal state (no capacity
  release for nonterminal children); failed/blocked execution keeps the run
  nonterminal with the execution recovery revision unclosed; parent-level
  post-completion coordination retries are denied by the preserved
  authentication rules while source-level idempotence returns the original
  outcome.
- No new families or kinds: the conditional candidate catalog stays at 63;
  the terminal record charges to the child-control pools and the plan source
  to the child-source pools (charged at creation, unchanged here).

## Supplied and unqualified facts

Execution child authority, turn authentication, the launch chain, current
Main shown context; grants as volatile label assertions; capability/profile
qualification; complete source/consumer/index membership; original H/C/proofs;
resource ledgers and actual effects. Matching digests and repeated source
bytes establish none of those facts. Source fingerprint hashes cover
documentation bytes; CommitRef changeDigest hashes explicitly unqualified
labels. No wire decoder, header, manifest, row/change hash, runtime reducer,
native/model probe, database, migration or executable product plan is
supplied.

## Verification and measurements

Node 22 `node --test` runs all prior documentation packets unchanged (the
packet's own 17 regressions pass, including independent regeneration
equality, twelve semantic-mutation denials, helper oracles separating
completion admission from terminal retries, and literal domain/bound/catalog
pins). The generator-derived measurements record the exact key plus committed
occurrence projection for the retained rows; they are not storage charges or
a legal transaction witness.
