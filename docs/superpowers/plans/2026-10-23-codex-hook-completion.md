---
type: Plan
title: Codex completion mitigation implementation
description: Implement the approved completion correlation and owned attention transition while retaining independent rollout gates.
tags: [codex, attention, metadata, runtime]
status: draft
---

# Codex completion mitigation implementation

Implements the owner-approved [design](../specs/2026-10-23-codex-hook-completion-mitigation-design.md)
for [#690](https://github.com/Rambolarsen/orkworks/issues/690).
The owner authorized implementation with “go ahead” after the design PR merged.
This does not authorize a shipping qualification row, sandbox change or new
metadata transport. The pass condition for the local core is serial exact
completion clearing its own persisted wait, with all ambiguous/competing writes
preserved. Production qualification remains a separate pass condition.

## Delivery and boundaries

- [x] Record approval in the design, MVP and ADR 0051. Extend the written
  metadata eligibility contract to independently qualified non-native completion
  launches; do not advertise a capability or launch marker without qualification.
- [x] Add `runtime/codex_completion.rs`: bounded pure serial tracker with an
  accepted-prompt baseline, root/turn/generation fences, contiguous ordered
  delivery evidence, 64 retained invocation IDs, explicit wait acknowledgement
  and one-use completion proofs. Invalid/missing/reordered/overlapping input
  locks clearing until a new qualified prompt boundary. Add failing regressions
  before implementing each behavior.
- [x] Add `runtime/codex_completion_application.rs`: apply an immediate permission
  wait and conditionally clear it using live/store attention ownership tokens
  inside the existing storage lane. Check lifecycle, exact prompt tuple,
  harness-hook authority, generation/input fences and coordinated-writer
  eligibility. Never overwrite a concurrent user/question/identical agent write
  or a descriptive update. Exercise the real store and both transaction orders.
- [x] Run focused tests, full repository verification and scoped review.
- [ ] Complete the PR lifecycle.
  Record exact remaining production blockers instead of claiming the user-facing
  symptom fixed. Existing native code and its clear gate remain unchanged.

## Rollout prerequisites and investigation

The approved design requires a qualified non-native launch before relay wiring
may grant clear authority. The current restricted context cannot reach localhost;
#789 remains incomplete. The current relay sorts UUID filenames and carries no
contiguous stream identity, so it cannot prove complete ordered delivery. Its
native report type also lacks tool_name. These are not facts a reducer can infer.
Do not feed that stream to the new core as if qualified, retrofit synthetic
sequence numbers at consumption, or launch another coding harness for evidence.
A production row also requires a proved effective configuration excluding
internal subagents. None is qualified. Therefore this PR can land the independent
core but cannot advertise or activate it; production integration/installed
qualification remain blocked until those prerequisites are actually satisfied.

## Review and risk treatment

Ready for the independent local core under the approved design. Dependencies 4
(metadata migration and installed hook/configuration evidence); blast radius 2
(new dormant core); state changes 2 (existing canonical metadata transactions);
reversibility 1 (no launch capability); uncertainty 4 (installed complete stream
and single-root semantics unproved), total 13. The empty production qualification
state contains those uncertainties. No schema migration or external write is
needed to test the core.

Ruling: implement the independently testable correlation/storage boundary before
blocked launch integration, rather than inventing qualification evidence. Cost:
this core alone does not change running sessions or complete #690/#789. Acceptance
still requires real context/stream/configuration evidence before activation.

## Local evidence

The initial inert tracker failed the serial permission/completion assertions.
A cross-tracker proof-replay regression then failed and was fixed with opaque
tracker identity. The inert application failed to persist the permission wait;
the implementation now passes its real-store regression. A permanent-revocation
regression failed before eligibility was revoked as well as correlation. All
16 focused tests now pass, including 12 competing-source/runtime variants,
64-ID exhaustion, both revisioned-agent commit orders and a concurrent storage
lane test. These are core/fixture results, not installed rollout evidence.

The coverage review added a held-wait regression: an unrelated completion and
then a matching completion both preserve the persisted bytes and live Needs You
state after correlation is invalidated. The medium current-diff code review
approved the implementation without actionable findings. It reviewed staged
code digest `f53c1adf4ba76b14613f77fdc384de02fff168e49cc29843bcebc66e3f565e9f`
against base `6ab58464952d3edaf99d349d5e34d332ff2fe5e4`.

The first full repository run reached 2,177 passing Rust tests but failed the
unchanged `unavailable_manual_analysis_retries_deferred_run_recovery` assertion
that a deferred Taskmaster attempt had cleared. The same test passed in
isolation. This is recorded separately from the focused completion checks;
the full default-parallel rerun passed all 2,179 Rust unit tests and all six
integration tests. `bash scripts/verify-repo.sh` then passed all repository checks, including
OpenCode reporter tests, desktop type-check/native tests/test suite/build and
the docs build. Documentation metadata/index checks also passed. The public
guide and README currency warnings need no edits: the core is dormant and
changes no advertised capability, user behavior or milestone. The architecture
module map and runtime contracts document the new internal seam. The worktree
warning before the first commit reflects the branch still pointing at its
already-merged base; only this session’s worktree will be cleaned after merge.
