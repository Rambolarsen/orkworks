---
type: Design Proposal
title: Repository-led Brain workflow assessment
description: A bounded two-stage assessment that selects related repository evidence before judging one workflow improvement.
tags: [orkworks, taskmaster, brain, assessment]
status: draft
workflow_status: proposed
---

# Repository-led Brain workflow assessment

Tracking: [#929](https://github.com/Rambolarsen/orkworks/issues/929).
Owner approved the direction on 2026-10-18; this concrete design is proposed.
Runtime implementation requires owner approval of this document and
[ADR 0086](../../adr/0086-repository-led-taskmaster-assessment.md).
The accepted [assessment contract](../../../specs/taskmaster-knowledge.md#manual-brain-guided-next-step-assessment-769)
and ADR 0085 remain authoritative until then.

## Outcome and evidence

**Assess workflow** should identify one concrete, reviewable workflow improvement
from related current repository facts, even without recurring Peon friction.
It may return no proposal when the repository is healthy or the evidence is
insufficient. It does not become a general code review or feature planner.

On 2026-10-17, the selected workspace's latest 128 textual observations contained
only two October reports; 73 evidence strings were shorter than 30 characters.
A read-only probe of the real collector with source-code access selected
scattered files using those observations. Both ordinary analysis and a source-code
assessment completed with no proposal. The latter explicitly cited incomplete
excerpts and fragmentary observations as insufficient evidence. These findings
establish a context limitation, not that the workspace must contain a useful gap.

## Decision and alternatives

Use two bounded inference stages inside the existing manual assessment.
Repository-first seed collection establishes instructions, manifests and current
workflow guidance. A discovery response selects one candidate and related files;
the sidecar reads those permitted files and asks for a final assessment.

A prompt-only change is smaller but cannot resolve missing supporting context.
An unrestricted agentic audit can investigate deeply but adds tool, execution and
usage authority. The selected approach adds one constrained evidence selection,
with no model tools or open-ended loop.

Ordinary **Analyze now**, background evaluation, provider/model selection and
automatic knowledge updates keep their existing behavior. Use the existing
signed assessment capability and relevant general guidance; no publisher change
or private Brain access is required. Application-defined discovery instructions
define the selection protocol; Brain pages remain reference material.

## Repository discovery and evidence limits

Use one collector owner for seed discovery, the permitted file catalog,
follow-up reads and revalidation. It enforces current context level, exclusions,
Git ignores, credential filtering and component-by-component no-symlink opens.
The model chooses from data the sidecar has already permitted; its choice grants
no access. Never accept arbitrary model paths, commands, URLs or glob patterns.

- Preserve the traversal limit of 2,000 entries and depth 5. Build the catalog
  from that same traversal, with deterministic ordering and an explicit partial flag.
- Catalog at most 512 permitted regular-file paths and 16 KiB of serialized
  path data. Omit paths longer than 512 UTF-8 bytes. Catalog IDs are stable
  indices in this run's ordered catalog. A partial catalog proves no absence.
- Seed at most 16 files and 24 KiB of bounded file bytes. Reserve priority for
  root/scoped instructions, manifests, current workflow guidance and CI.
  Existing observations may rank within these groups, never replace them with
  incidental matching source files. Historic plans/reports rank last.
- One follow-up may select at most eight distinct catalog IDs. Across both
  stages, supply at most 32 distinct files, 8 KiB per file and 64 KiB total.
  Integrity revalidation rereads those same bounded prefixes; it cannot add
  context beyond this shared evidence allowance.
  Reuse already-read bytes; remaining capacity is not a fresh allowance.
- Keep each supplied excerpt a verbatim span of at most 2,000 characters within
  the permitted 8 KiB prefix. Select seed spans from workflow purpose; follow-up
  spans may use the bounded candidate hypothesis as relevance data.
  Content beyond the prefix remains unknown.
- Keep each provider prompt within 128 KiB and each response within 64 KiB.
  Repository-derived prompts, catalogs and discovery text remain memory-only.

At session-observations access, repository assessment reports unavailable because
it cannot obtain the required repository fact; it does not widen access.
Workflow-context access permits workflow files only. Source-code access also
permits related code and tests. Catalog filtering follows the same distinction.

A denied or changed requested file interrupts the assessment; it does not become
evidence of absence. If the shared read budget cannot cover the requested set,
return an explained no-proposal result with the evidence limit and unverified
check. Do not partially inspect that set and claim it was complete.

## Discovery and final assessment protocol

Use a new internal discovery envelope, version 1, with exactly these keys:

| Field | Contract |
| --- | --- |
| outcome | `inspect` or `no_candidate` |
| candidate | Null, or targetSurface/title/hypothesis using existing target surfaces; title ≤240 and hypothesis ≤1,000 Unicode scalar values |
| repositoryFactHashes | At most 32 distinct supplied hashes |
| knowledgePageIds | At most eight distinct supplied page IDs |
| requestedFileIds | At most eight distinct IDs from this run's permitted catalog |
| reason | Null for inspect; nonempty ≤1,000 characters for no_candidate |
| version | Integer 1 |

For inspect, require one candidate, at least one supplied repository hash and
one supplied Brain page ID. Empty requestedFileIds is valid when the seed
already contains the supporting facts. For no_candidate, require a null
candidate, empty requestedFileIds and an explained reason; citations may be empty.
Reject unknown fields, IDs, control characters, command/tool-action output and
oversized responses. No unvalidated discovery output reaches the next prompt.

Discovery must consider repository facts independently of session friction.
Existing instructions are applicability constraints, not proof of effectiveness.
A candidate needs positive evidence of a concrete problem or opportunity;
omitted text, a missing catalog entry, or stale Peon text cannot establish a gap.

For inspect, send the candidate as untrusted hypothesis data with seed and
follow-up facts and relevant signed guidance. Ask the final stage to challenge
the hypothesis: does supporting context show that it is already addressed,
inapplicable, speculative or contradicted? It returns the existing strict
assessment-v1 final envelope and at most one proposal. The proposal summary
names a bounded target, change, expected benefit, validation and rollback, with
at least one supplied repository hash and Brain page ID. Checks not performed
remain unverified. A no_candidate response maps to the existing no-proposal
report; it creates no recommendation and skips the second call.

There are at most two provider calls per manual assessment, with no retries or
fallback model. Both use the same captured selected transport and effort.
Existing native invocation timeout is 90 seconds; custom adapters retain their
validated 1–120 second timeout. Thus at most 240 seconds of configured provider
execution is admitted, excluding existing process cleanup and bounded filesystem
work. Manual assessments remain exempt from the background daily allowance.
The shared lease covers both calls; recheck admission before each dispatch.

## Identity, compatibility and recovery

Introduce assessment record format 2, explicitly decoded alongside legacy
format 1. Retain the existing final response schema and recommendation family.

Persist a bounded input manifest with the final report: collector/prompt versions,
ordered catalog digest and completeness, seed/follow-up path roster and bounded
byte hashes, stage/excerpt identities, discovery-response digest, selected page
identities and the remaining existing full-input identities. Store no uncited
file contents, catalog listing, discovery prose or full prompts. Manifest entries
are limited by the same 32-file cap; the complete report/pending record still
must fit 64 KiB or fail before publication.

A saved selection is data for safe revalidation, never authority to reopen a
path. Re-enumerate the permitted catalog, rerun seed selection and safely reread
the saved permitted follow-up roster under current settings. Any catalog,
seed/uncited fact, excerpt, observation, recommendation, provider/harness,
knowledge, workspace or version change invalidates reuse. Bind the immutable
discovery digest and exact stage selection to that identity without rerunning
the model on status reads. A new file can invalidate the catalog digest.
Cache hits return the existing report, explicitly marked reused.

Revalidate before each call, publication and every existing status,
recommendation, acceptance and Fix with AI consumer. Preserve exclusion of only
the exact derived recommendation from the pre-run recommendation snapshot.
Access narrowing must redact the manifest's disallowed paths/hashes and all
source-derived fields as well as existing cited snapshots; invalidate reuse
and retain audit/lifecycle identity. Reconcile unopened workspaces through
the existing settings-narrowing path before later exposure.

Legacy completed reports are not reusable as format-2 assessments.
On the next state-changing assessment, invalidate legacy reports and supersede
only still-proposed legacy assessment recommendations through existing lifecycle
rules. Legacy accepted/executing recommendations retain their states and block
new assessment until resolved through authorized existing paths. Never restart,
resume or modify their sessions. Preserve ordinary recommendations and completion.

Queued/running format-2 attempts interrupted by restart are marked interrupted;
do not replay provider calls or resume discovery. The existing hidden pending
recommendation transaction remains the only publication path, including
idempotent upsert and immediate failed-final-commit reconciliation. Validate
format-2 input manifests before recovery or exposure. Unknown/corrupt formats
fail unavailable and never overwrite the last durable record silently.

Before writing format 2, binary rollback is unchanged. Afterward, old binaries
cannot parse its deny-unknown-fields record and cannot enforce its input
semantics: supported recovery is a forward fix with new assessment admission
disabled while consumer fences remain active. Do not downgrade or hand-edit
metadata. Document this boundary before shipping.

## Presentation and authority

Keep the existing Assess workflow action and progress surface. Show fixed phases
`discovering`, `reading_evidence`, and `assessing`; do not stream discovery prose
or uncited evidence. The final result still shows one proposal or an explained
no-proposal result, provenance and unverified checks. Do not imply that code was
tested, that all repository files were inspected, or that every run should yield
a recommendation.

Only the sidecar reads selected files. No shell commands, tests, writes,
new/resumed sessions, GitHub queries or publication run as part of assessment.
Existing inference-adapter managed-policy limitations still apply.

## Implementation plan

All deliverables depend on owner approval of this concrete proposal.

1. **Repository evidence owner and discovery contract.** Extend
   `taskmaster/context.rs` and add a focused discovery decoder beside
   `taskmaster/assessment_model.rs`. Test repository-first selection despite
   hundreds of noisy observations, related instruction/CI/test files, catalog
   determinism/partiality, aggregate limits, credential exclusion, symlink
   replacement, forbidden IDs and context narrowing. Every requested file
   must remain confined at open time.
2. **Two-stage assessment and durable identity.** Integrate through
   `assessment.rs`, `assessment_report.rs`, `assessment_store.rs` and
   `assessment_access.rs`. Add explicit format-1/2 decoding, complete manifest
   revalidation, legacy admission handling, redaction and interrupted/pending
   recovery. Stub the selected inference transport to verify 0/1/2 calls,
   no retries, unchanged ordinary analysis, and configuration/workspace/file
   changes between stages preventing dispatch or publication.
3. **Visible result and quality evidence.** Update independent Electron/renderer
   types, preload/status projection and existing assessment summary tests for
   fixed phases and truthful bounds. In isolated test workspaces, exercise
   context levels and restart/narrowing/final-commit failure. Use the configured
   provider only for explicitly triggered evaluations; do not change provider
   selection or call private Brain. Evaluate the matrix below, record exact
   evidence and update the authoritative spec/ADR status only after approval.

Run scoped Rust and desktop tests, required Rust build/test/fmt, then
`bash scripts/verify-repo.sh`, documentation checks and the worktree check.
Complete the current-diff code-review gate before runtime PR merge.

## Quality and acceptance

| Case | Expected assessment behavior |
| --- | --- |
| Current guidance requires unit tests, but the supplied CI check is an explicit no-op; no observations | Read the related test setup, then propose a bounded correction with exact evidence |
| Candidate gap already addressed in a related file | Follow-up refutes it; no duplicate recommendation |
| Old/glyph-heavy observations plus a current repository gap | Repository evidence remains selected; no recurrence requirement |
| Healthy linked instructions, tooling and checks | No invented gap; an explained no-proposal result is valid |
| Excluded, ignored, secret-bearing or substituted symlink target | No forbidden content enters either prompt or saved result |
| Candidate requires unseen text or running a command | Explain the missing evidence; never claim verification |
| Provider failure, malformed output or changed settings between calls | Explicit failure/interruption; no second call under stale authority and no proposal publication |
| Changed uncited file/catalog or access reduction after publication | Reuse/consumers fence stale data; narrowing redacts all retaining states |

Use deterministic transport fixtures for protocol/lifecycle guarantees.
For usefulness, run each positive and healthy case three times with the selected
provider/model and fixed fixtures. Require every surfaced proposal to be grounded
and scoped, at least two of three positive-case runs to find the designed gap,
and zero invented proposals on healthy/refuted cases. Record model, effort,
bundle and outputs in local sanitized evaluation evidence. This evaluates the
implementation, not a guarantee for arbitrary repositories or future models.
A failed quality threshold blocks claiming that this solves proposal usefulness.

## Plan review and investigated risks

Disposition: **Ready for owner design review; runtime execution remains gated.**

The main agent uncertainty was how to revalidate model-selected files without
saving the full prompt. Existing assessment consumers reconstruct the complete
input, and report types reject unknown fields. The manifest plus explicit format
2 resolves that dependency; treating optional ad hoc fields as backward-compatible
would be incorrect. Existing custom inference validates timeout at 1–120 seconds,
so this proposal bounds call count without inventing a new process lifetime rule.

The main product blind spot is that broader access does not establish a real
workflow gap. The healthy/refuted fixtures and retained no-proposal outcome
prevent manufactured recommendations from becoming the success metric.
Bounded prefixes still cannot investigate large-file regions beyond 8 KiB;
this first extension makes no deeper-code-review claim.

| Dimension | Rating | Evidence and treatment |
| --- | --- | --- |
| Dependencies | 2 | Reuses selected inference and signed capability; no publisher release dependency |
| Blast radius | 3 | Manual assessment plus its shared consumer fences; ordinary analysis unchanged |
| State changes | 4 | New local record format; explicit legacy handling and transaction tests |
| Reversibility | 4 | Forward-fix boundary after format-2 publication; admission can be disabled |
| Uncertainty | 3 | Collection/protocol can be fixture-tested; usefulness requires the recorded model evaluations |

Total: 16/25. Ratings describe risk, not implementation approval.
