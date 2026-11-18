---
type: Implementation Plan
title: Review handoff preview implementation
description: A bounded interactive preview plan with an explicit production read and approval contract handoff.
tags: [orkworks, taskmaster, review, desktop, plans]
status: draft
---

# Review handoff preview implementation

**Outcome:** Inspect and exercise the accepted review-handoff interaction in the
existing isolated development preview, including shared-branch identity, changed
or missing plans, coherent refresh and keyboard/compact layout. Pass means the
real proposed component works with synthetic inputs and cannot launch, select,
acknowledge or submit anything.

**Tracking:** [#1899](https://github.com/Rambolarsen/orkworks/issues/1899), within
[#610](https://github.com/Rambolarsen/orkworks/issues/610) and
[#746](https://github.com/Rambolarsen/orkworks/issues/746).
**Spec:** [accepted interaction design](../specs/2026-11-18-review-handoff-preview-design.md),
merged in [#1905](https://github.com/Rambolarsen/orkworks/pull/1905).
**Status:** owner-approved fixture-only implementation plan. The owner replied
“keep going” after draft PR #1906 presented this concrete boundary and its
production prerequisites on 2026-11-18. That approves tasks 1–3 only; production
connection remains **Investigate**.

## Evidence and delivery decision

Inspected at main `ce32c757a92e68fd9fba2f1eb3438983e63b1d53`:

| Owner / source seam | Verified current contract | Consequence |
| --- | --- | --- |
| `taskmaster/mod.rs::RecommendationType`, renderer `api.ts::WorkflowRecommendation` | Only `improve_workflow` and `cleanup` are current recommendation types. Suggested tool/model/prompt/directory fields exist; they do not create a review intent or coherent launch proposal. | Do not cast a completion packet or improvement card into a reviewer launch. |
| `branch_documents.rs::Document::selection_id` | Selection hashes branch identity, anchored worktree, relative path and content hash. `content_hash` normalizes CRLF. | Reuse the artifact owner's identity semantics later; branch/path alone is insufficient and a client-generated raw-byte hash is not an equivalent token. |
| `branch_documents_application.rs::read_document_content`, `http/branch_documents_handlers.rs::content`, Electron `planOpener.ts::getSessionPlanContent` | Optional kind/selection-bound reads validate context; response exposes content, not a combined reviewer/source/proposal snapshot. | A separately fetched session list plus content is not proof of coherent proposal identity. |
| `branch_documents_application.rs::request_document_review`, Electron `planOpener.ts::requestSessionPlanReview` | Existing authenticated, selected-live-session fixed prompt with document/runtime admission checks. | Leave this route and its authority unchanged; it is not a new-reviewer dispatch path. |
| Electron `main.ts::withReadyBackendGeneration` | Existing generation-bound authenticated requests and invalidation. | Future transport must reuse that ownership boundary, with independently declared IPC types. |
| `tests/fixtures/workflowPreview.tsx`, `scripts/preview-workflow.mjs` | Opt-in esbuild entry, synthetic scenarios, in-memory assets, loopback-only server, no production App/bridge import. | Reuse this development entry for an interactive review scenario; no new server, dependency, production destination or API is needed. |

File paths beginning `taskmaster/` or `branch_documents` above are below
`crates/orkworksd/src/`; other paths are below `apps/desktop/`.

The production composer is still scoped by
[#68](https://github.com/Rambolarsen/orkworks/issues/68), which requires explicit
Start and reconciled lifecycle/identity contracts. #610 additionally requires
exact approved plans, qualified configuration/adapters and child admission.
Therefore a production-connected preview cannot yet be described as a small
renderer-only change. The first slice is an owner-approved fixture-only development
exception, following the existing
[hierarchy preview pattern](2026-10-26-hierarchy-ui-preview.md). That earlier
approval does not automatically authorize this new slice.

## Boundaries and presentation contract

Use the existing React/TypeScript/CSS tokens. Add one focused component and one
small pure presentation-state module. Fixture data stays under `tests/fixtures/`.
The production entry, RecommendationsPanel, App, NewSessionDialog, API, preload,
Electron main and Rust source remain untouched in this slice. No new packages,
wire format, durable state, filesystem capability or model call.

The component resembles expanded recommendation detail rather than a new shell.
The fixture provides a recommendation-card invoker and Recommendations heading
for close/focus fallback. Its permanent banner says **Synthetic review handoff
preview**. It contains no real Sessions list or terminal. A synthetic selected
session/unread/attention indicator may demonstrate unchanged presentation state;
it does not qualify real session lifecycle behavior.

Define local readonly `ReviewHandoffSnapshot`, `ReviewHandoffState` and
`ReviewHandoffEvent` in `reviewHandoffPresentation.ts`. These are presentation
records, not proposed source-owned JSON or authenticated evidence. Snapshot fields
cover a synthetic workspace/backend scope key, proposal identity/revision,
source session/runtime/title/tool, proposed reviewer role/tool/model, separate
source/reviewer work locations, artifact display identity or a closed
missing/unavailable state, complete instruction and supplied unavailable reason.
All non-current field states are explicit; no fallback directory, inferred
reviewer, guessed plan or derived independence/capability badge.

The reducer owns only inspection/disclosure/refresh state. Opening binds one
snapshot. A material-change event marks it stale and retains its old labelled
fields. Refresh starts a new local request identity; only the matching complete
response in the unchanged scope replaces all displayed fields together. Replaced
scope, closed detail, removed recommendation or superseded request invalidates
late responses. Returning to the same text/path never clears a stale flag.

Expose local `onClose` and `onRefresh` presentation callbacks; never accept a
start/accept/review-prompt callback. A disabled continuation and adjacent reason
illustrate pending production gates. Even a synthetic current snapshot cannot
enable an execution action. Focus is component-owned only for heading, refresh
completion/status and close; background updates announce without stealing focus.

## Delivery tasks

### 1. Local snapshot and refresh transitions

Files: create `apps/desktop/src/components/review/reviewHandoffPresentation.ts`
and `apps/desktop/tests/reviewHandoffPresentation.test.ts`.

- [x] Write failing behavioral tests for shared branch but distinct source IDs,
  plan content change at the same path, changed anchor/pin identity, changed
  reviewer/instruction/work location and explicit missing/unavailable fields.
- [x] Implement the small immutable state model above. Preserve the inspected
  snapshot on change; require refresh to install one complete replacement.
- [x] Test close/removal and workspace/backend/runtime replacement during refresh,
  out-of-order refresh completion, and A→B→A context changes. Old completions
  cannot reopen detail, clear staleness or install a partial snapshot.
- [x] Run the focused test under Node 22; observe red before implementation and
  green after it. Do not add protocol decoding or a generic state framework.

Pass: `pnpm --dir apps/desktop exec node --experimental-strip-types --test tests/reviewHandoffPresentation.test.ts`.

### 2. Component and opt-in interactive scenario

Files: create `apps/desktop/src/components/review/ReviewHandoffPreview.tsx`,
`apps/desktop/src/components/review/ReviewHandoffPreview.css` and
`apps/desktop/tests/fixtures/reviewHandoffPreview.tsx`; modify only the existing
`apps/desktop/tests/fixtures/workflowPreview.tsx` to offer a review scenario.

- [x] Implement source/reviewer identity, separate work locations, anchored
  artifact display or missing state, complete instruction expanded by default,
  textual status, refresh, close and permanently disabled continuation.
- [x] Add selectable synthetic scenarios: current shared-branch source/companion;
  changed plan with old instruction retained; no selected plan; unavailable
  reviewer/location; replaced workspace/runtime; delayed/out-of-order refresh;
  removed recommendation; long labels/paths and markup-looking instruction.
- [x] Keep fixture callbacks memory-only. Opening, disclosure, refresh, scenario
  switches and close perform no backend, IPC, storage, selection, acknowledgement
  or launch operation. Closing restores the invoker or surviving heading.
- [x] Reuse the current `preview:workflow` command/strict port behavior. Preserve
  existing hierarchy scenarios and server restrictions; no server refactor.

Pass: `pnpm --dir apps/desktop preview:workflow` exposes the new scenario with
its persistent synthetic label and no enabled execution control.

### 3. Interaction, isolation and verification

Files: create `apps/desktop/tests/reviewHandoffPreview.test.mjs` and
`apps/desktop/tests/reviewHandoffBundleIsolation.test.mjs`, following the
existing isolated Electron/esbuild test pattern in `workflowOverview.test.mjs`;
update this plan with actual evidence after implementation.

- [x] Write DOM/keyboard tests before component implementation for heading focus,
  native disclosure, stable focus on asynchronous change, readable polite status,
  refresh completion, close fallback and ordinary Tab escape without a trap.
- [x] Assert exact source/reviewer labels even when titles/branches match; full
  long instruction remains selectable text, markup is inert, and stale/missing
  explanations remain visible with disabled continuation.
- [x] Exercise the real component at 1280, 700 and 360 CSS px, 200% text zoom and
  reduced motion. Assert no document-wide horizontal overflow and reachable
  close/refresh/status; capture and inspect wide/compact screenshots.
- [x] Fail on unexpected fetch, WebSocket, bridge or renderer storage access;
  assert no terminal mount and no action callback. Exercise refresh races via
  controlled promise ordering, not arbitrary sleeps.
- [x] Run renderer type-check, focused tests, existing workflow tests/server
  regression and production renderer build. Inspect the production Vite/Rollup chunk module graph
  for production entry exclusion of both fixture modules and this unused
  component; a string-only bundle grep is supporting evidence, not the sole check.
- [x] Run `bash scripts/verify-repo.sh`, docs build/checks, doc/worktree currency
  checks and the required current-code `/code-review low` (escalate only for
  documented size/risk). Update #1899 with observations and empirical limits.

Focused DOM command: `pnpm --dir apps/desktop exec node --experimental-strip-types --test tests/reviewHandoffPreview.test.mjs tests/workflowOverview.test.mjs tests/workflowPreviewServer.test.mjs`.
Type check: `pnpm --dir apps/desktop exec tsc --noEmit`.
Production build: `pnpm --dir apps/desktop exec vite build`.

These tests qualify presentation and development isolation only. They cannot
prove producer coherence, authenticated approval race rejection, exact-once
launch, real unread/attention preservation, native screen-reader behavior or
packaged-platform capability. Those need the production owner tests below.

## Production handoff: prerequisites before connection

No new endpoint/type name or numeric limit is prescribed here. Close these
contracts with their owners, obtain scoped approval, then revise only the
production part of this plan before dependent code:

| Owner | Required deliverable and pass condition |
| --- | --- |
| #66/#67/#68 review-transition recommendation and ordinary composer | Exact review intent, source/proposal revision/evidence identity, edits→new preview, explicit Start, lifecycle/linkage, duplicate/pending/recovery semantics. Existing improvement Fix with AI and cleanup remain unchanged. |
| Artifact/session read owners | Closed coherent read contract joining recommendation revision, workspace/backend generation, source/runtime identity, reviewer configuration, both work locations, artifact kind/anchored selection/content semantics, included content versus reference and complete instruction. Define races, bounds, unavailable states and visibility policy; do not concatenate unrelated reads. |
| Electron/preload/renderer boundary | Authenticated generation-bound transport with independent types, validated closed inputs, cancellation/late-response rejection and no arbitrary renderer-supplied path or terminal text capability. Reconcile location display with the existing content-only artifact access boundary. |
| Consuming ordinary Start or #610 exact-plan owner | Atomically revalidate displayed subject and all material bindings before effects; stale context denies without fallback/retry. Preserve existing idempotency and recovery ownership; prove both change/approval race orders and duplicate attempts at the actual side-effect boundary. |
| #610/#746 and role/configuration owners | Accepted relevant projection and exact-plan configuration, qualified initial capability slice and separately approved execution plan. A polished preview supplies none of this evidence. |

Production tests must cover same-path content/anchor/pin changes, same-name
runtime replacement, recommendation supersession/removal, workspace/backend
replacement, composer edits, shared-worktree attribution, double approval and
failure/recovery without double launch. Integrate in the existing Recommendations
detail and Actions paths only after those prerequisites; preserve the fixed
selected-session Review prompt unchanged. No second sidebar, review queue,
launcher or selection policy is part of this handoff.

## Investigated uncertainty, risk and review

Least-confidence question: could existing suggested fields and artifact selection
IDs already supply a safe production preview? The source trace above resolves
it: they supply useful independent inputs, but no current review recommendation
type or jointly bound proposal/approval contract. Spec examples are not a live
runtime implementation.

Project blind spot: an interactive fixture could be mistaken for approved source
records or a real reviewer launcher. Keep synthetic data outside production,
all continuation disabled and no execution callbacks. Reject the alternative of
adding a new review type/API just to populate this UI; it would also require the
#68 lifecycle and #610 authority work and materially enlarge this slice.

| Dimension | Rating | Evidence / treatment |
| --- | --- | --- |
| Dependencies | 2 | Existing isolated preview, React/CSS and Electron runner; production owners explicitly excluded |
| Blast radius | 1 | New component/state/tests plus opt-in fixture entry; production imports unchanged |
| State changes | 1 | Memory-only presentation; no persistence or migration |
| Reversibility | 1 | Remove fixture option/component; no user data to repair |
| Uncertainty | 2 | Known runner pattern; actual component layout/races await execution tests |

Fixture slice: 7/25. Production is not scored as ready: dependencies and
coherent admission remain unresolved. Ratings grant no authority.

Reviewed once against the accepted design and the exact source seams above.
Disposition: **Owner-approved and implemented for fixture-only presentation;
Investigate for production connection**. Final repository verification passed and the required current-code review
completed with no remaining findings; exact evidence is recorded below. Mandatory interaction requirements map
to tasks 1–3; real source/approval requirements map to the explicit prerequisite
table. No implementation tests or native capability probes ran in this planning
task. Existing design acceptance is retained and is not requested again.

## Execution evidence

The owner approved the proposed fixture-only scope with “keep going” after
PR #1906. Seven pure state regressions observed RED on the missing module,
then GREEN. The DOM fixture observed RED before the component/fixture existed;
the real shared preview entry now exercises the new scenario.

Under Node 22, the focused suite passed 11 tests: state transitions, real
Electron interaction/layout, production module-graph isolation, existing
hierarchy interaction and existing loopback preview-server restrictions.
The strengthened workspace-reset/immediate-reopen regression reproduced an
old animation-frame focus return stealing the new heading focus; cancelling
that return on a new explicit open made the unchanged assertion pass.

The new Electron runner initially timed out awaiting readiness during module
initialization; it now follows the existing whenReady callback pattern and
retains startup diagnostics. Its DOM serialization follows the established
runner. Original 5-second condition and 60-second child deadlines are unchanged.

Wide and 360px screenshots, including the lower instruction/action area, were
rendered and inspected. Labels wrap; full instructions remain scrollable and
selectable; disabled continuation has a readable reason. DOM checks include
200% text zoom, reduced motion, explicit Tab exit, close fallback, replaced
workspace/runtime, late completions and forbidden network/bridge/storage access.
These observations do not establish native screen-reader or packaged behavior.

The module-graph regression inspects the real production renderer Vite/Rollup
chunks in memory and finds no fixture or review-preview modules. No production
App, API, Electron, sidecar, launch or selection behavior changed.

### Final verification and review

`bash scripts/verify-repo.sh` passed under Node 22: Rust 3,600 passed,
0 failed, 21 ignored, plus integration tests; 31 reporter tests; renderer
type-check; native ABI repair; desktop 1,786 passed, 0 failed, 5 skipped;
desktop build; 2,056 documentation tests and site build; whitespace and
documentation/worktree currency checks. Existing Rust and Vite/VitePress
compile/chunk warnings concern unchanged sources/configuration. Foreign
worktree warnings were left to their owners; generated pnpm metadata is
excluded from this change.

Required `/code-review low` initially reviewed `3a3db965` and found two required
issues. Both were reproduced RED and corrected: an open preview's invoker
can no longer replace stale context without refresh, and the stale notice now
precedes visibly labelled previous fields. The 11-test focused suite passed
afterward. Correction review at code head
`22108798dd0eea30f599051a6228a6c4f6f96e4c` approved with no new findings.
Hosted Codex review of that same head also completed without findings.
Subsequent evidence-only documentation edits do not change the reviewed code.

Authenticated Taskmaster inventory contained 37 records; none proposed this
preview implementation, so no recommendation mutation was warranted. Native
Copilot review was requested once but remains unconfirmed and is not credited.
Required CI and terminal PR disposition are recorded in
[#1906](https://github.com/Rambolarsen/orkworks/pull/1906). #1899 remains open
for the production prerequisite handoff; no runtime authority is inferred.
