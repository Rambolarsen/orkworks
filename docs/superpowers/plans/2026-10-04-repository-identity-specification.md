# Repository identity specification task

- Status: documentation task in progress; runtime planning remains gated
- Date: 2026-10-04
- Tracking: [#745](https://github.com/Rambolarsen/orkworks/issues/745)
- Scope: the early repository-identity portion accepted for this session;
  learning/cohort/history work remains downstream of #743/#744.

## Owned deliverable

Draft [the repository-identity contract](../specs/2026-10-04-configuration-learning-design.md),
link it from #741 and both specification/admission plans, validate documentation,
and open one PR for written review. Use branch `repository-identity-spec` in its
owned sibling worktree. Do not change runtime source, launch agents as capability
probes, claim a verified filesystem adapter or close #745.

## Evidence and uncertainty checkpoint

`GitContext.repo_root` is a checkout root; `plan_handoff.rs` compares repository
families only for advisory plan access. Disposable Git 2.54.0 fixtures confirmed
primary/linked/alias common-directory equality and separate clone/submodule/
external-Git-dir layouts. Primary Git documentation supports discovery semantics.

The remaining outcome-changing risks are replacement at a reused path,
filesystem-object identity availability/reuse, native path aliases, durable
registration across sidecars and exact approval/resume binding. The draft
proposes local registration with explicit limitations, no path-only fallback,
finite storage and platform fixtures. These defaults need written review;
source implementation is unauthorized until contract/ADR/plan acceptance.

## Tasks and validation

- [x] Read #745/#741/#610, accepted ADR 0077, product design, relevant plans and
  current Git/context/plan-handoff seams.
- [x] Investigate both uncertainty questions and compare identity approaches.
- [x] Run disposable discovery/layout assertions without model probes.
- [x] Draft concrete identity/digest, registry, consumer binding, failure and
  recovery cases; synchronize scope/readiness links.
- [x] Self-review, then obtain a bounded independent diff/spec review and
  disposition any actionable findings.
- [x] Run `rtk git diff --check`, `rtk proxy bash scripts/doc-check.sh`,
  `rtk proxy pnpm --dir docs docs:build` and the worktree currency hook in the
  owned checkout; inspect outputs before claiming verification.
- [x] Commit and open a documentation PR for the owner's written contract review;
  update #745 with partial progress and remaining gates, preserving its scope.
- [ ] Obtain written contract acceptance; only then prepare the separate
  ADR/runtime tracker/executable plan and its explicit approval.

This task supplies a review artifact. It does not complete the full #745
acceptance criterion (identity plus comparison cohorts) or its handoff.

## Bounded review routing

One writer (this session) owns all five documentation files. Dispatch at most
two read-only Luna reviewers at high reasoning effort, one round, with isolated
contexts. Correctness reviewer checks identity/digest/layout/authority claims
against the draft and existing Git/configuration/admission sources. Completeness
reviewer independently checks the narrow #745/#741/#610 prerequisite, bounds,
failure/recovery/race cases and remaining scope gates. Both consume the same
owned draft; neither consumes the other's findings or edits files. The owner
reconciles evidence and applies actionable corrections, then returns the
concrete artifact to the human for written review before runtime planning.

## Independent review disposition

One correctness reviewer reported no actionable findings. The independent
completeness reviewer identified two gaps, both corrected in the owned draft:
approved source-worktree root/private-directory identity is now retained and
revalidated, and supported registry restoration uses a new epoch with explicit
direct-copy/rollback limitations. These are proposed specification corrections;
no platform or runtime success is claimed. No second reviewer round was run.

## PR review pass

The owner authorized babysitting, review fixes and merge of
[PR #753](https://github.com/Rambolarsen/orkworks/pull/753) on 2026-10-05.
Codex reviewed initial head `3e82304` and identified the same-object relocation
retirement gap. The contract now requires explicit retirement of the old active
locator before new enrollment, rejects duplicate active object bindings under
the registry lock, and covers moving away/back plus interrupted enrollment.
Substantial external review cycle 1/3 will request review of the corrected head.
Merge authorization applies to this documentation deliverable; platform
eligibility, accepted identity ADR and executable runtime planning remain gated.
