---
type: Implementation Plan
title: Hierarchy UI preview
description: A fixture-backed hierarchy preview for reviewing interaction before production orchestration.
tags: [orkworks, hierarchy, desktop, plans]
status: draft
---

# Hierarchy UI preview

**Outcome:** Open a local interactive preview of the hierarchy, inspect tasks and
skills, switch between Timeline and Tree, and find blockers without a running
coding tool or sidecar. This makes the hierarchy tangible while #740 remains
deferred.

**Status:** Owner-approved for this bounded preview scope on 2026-10-26. This document
records the accepted fixture-only development exception to the broader runtime handoff;
it is not approval of #746's production projection or of orchestration launches.
The owner authorized preparing this first UI slice on 2026-10-26. The owner replied “go ahead” after reviewing the concrete plan and scope
exception. Execution is tracked by [#1084](https://github.com/Rambolarsen/orkworks/issues/1084).

**Tracking:** [#746](https://github.com/Rambolarsen/orkworks/issues/746), within
[#738](https://github.com/Rambolarsen/orkworks/issues/738). Implementation: [#1084](https://github.com/Rambolarsen/orkworks/issues/1084);
the owner approval is recorded there and in the hierarchy interaction specification.

**Sources:** [hierarchy interaction proposal](../specs/2026-10-04-agent-hierarchy-ui-design.md),
[accepted shell design](../specs/2026-10-05-application-shell-navigation-design.md),
[ADR 0078](../../adr/0078-fixed-desktop-shell-and-central-navigation.md), and
[product boundaries](../../agents/product-boundaries.md).

## Scope and design decisions

Build a React component with local presentation inputs and an isolated browser
preview using the desktop's existing React, TypeScript, CSS tokens and Vite.
No graph library or new dependency is needed. Timeline is the default; Tree is
a second presentation of the same logical outline, inspected task and disclosure
state. This is a retained stage sequence, not a duration-scaled chart.

Use the accepted hierarchy layout: one orchestrator root, non-agent plan-stage
headings and direct task children. Lead with role, task and textual status;
place Git references and full skill evidence in inspected details. Reuse a
neutral decorative role glyph initially rather than commissioning portraits.
Keep delegation connectors decorative and label dependencies explicitly.

The preview has an obvious persistent “Synthetic workflow preview” label and
scenario selector. It does not render the real Sessions list, mount a terminal,
or import the production App, API, Electron bridge or session controllers.
Inspection stays local. A session-bound synthetic task may show a disabled
terminal/history command with “Unavailable in preview”; planned, starting and
missing-session tasks have no such command. No fake terminal or session
selection callback is provided.

This component consumes deliberately local presentation types, not proposed
wire JSON. Synthetic IDs distinguish every run, stage/revision and task, even
when labels repeat. Values such as overall result and evidence state are
supplied by fixtures; the UI never computes evaluation scores, validates delivery,
or derives success from process exit. These types are not evidence of approved
upstream identity mappings or a usable production projection.

Manual stage collapse follows the proposal's inspected/focused-descendant
protections and shows the applicable reason. Collapsed headings retain blocker
counts. Automatic collapse, durable viewed-summary tracking and live usage
animations are deferred: they depend on exact revision/receipt semantics and
are unnecessary to review the first visual slice. Static reported/observed-use
markers remain available with reduced motion.

No production Workflow destination, navigation persistence, aggregate Actions
counts, global shortcut, approval control, launch route, registry or backend
storage is introduced. The production shell and ordinary sessions keep their
existing behavior. Connecting real session selection and proving hidden-terminal
continuity belongs to the separately approved integration slice.

## Files and ownership

All changes belong to this session's `hierarchy-preview` worktree/branch.

| File | Responsibility |
| --- | --- |
| `apps/desktop/src/components/workflow/WorkflowOverview.tsx` | Presentational outline, Timeline/Tree switch, stage disclosure, focused-node toolbar and task details |
| `apps/desktop/src/components/workflow/workflowPresentation.ts` | Local presentation records and pure focus/inspection/disclosure transitions |
| `apps/desktop/src/components/workflow/WorkflowOverview.css` | Token-based responsive layout and distinct focus/inspection states |
| `apps/desktop/scripts/preview-workflow.mjs` | Explicit local preview command using existing esbuild and a loopback HTTP server; no Electron or sidecar startup |
| `apps/desktop/tests/fixtures/workflowPreview.tsx` | Synthetic scenarios and preview-only React entry |
| `apps/desktop/tests/workflowPresentation.test.ts` | Pure interaction-state regressions |
| `apps/desktop/tests/workflowOverview.test.mjs` | Isolated Electron DOM/layout/keyboard checks using the existing esbuild fixture pattern |
| `apps/desktop/package.json` | Add `preview:workflow` command only |
| `docs/superpowers/specs/2026-10-04-agent-hierarchy-ui-design.md` | Record approved preview boundary and reviewed observations without upgrading runtime eligibility |

Keep fixture data outside `src/`. The ordinary Vite entry must never import the
preview entry or synthetic scenarios. Use the preview script's in-memory output instead of modifying production Vite
inputs; release it when the server shuts down.
Bind to loopback with a configurable strict port (default 5276); report port
conflicts rather than attaching to another checkout's server. The script bundles
only the preview entry and serves only its generated HTML/JS/CSS artifacts.
It does not serve workspace source files, accept mutations or proxy requests.

## Delivery and acceptance checks

### 1. Presentation model and interaction state

- [x] Define small readonly records for root, stages, task keys, phases, supplied
  status/result, dependencies and skill display states. Separate focused and
  inspected keys; retain stable order across activity/status updates.
- [x] Add failing pure tests, then implement keyboard-visible ordering, focus
  movement, inspection, manual disclosure protection and disappearing-node
  fallback to a surviving stage/root.
- [x] Cover repeated labels in different revisions, stage append without focus
  theft, all blocker causes on a refused collapse, manual reopening, and
  scenario replacement resetting local state rather than leaking inspection.

Pass: `node --experimental-strip-types --test tests/workflowPresentation.test.ts`
from `apps/desktop/` exits zero, after observed red/green development.

### 2. Interactive overview and isolated preview

- [x] Implement Timeline and Tree from the same logical outline, using a single
  roving treeitem tab stop and a sibling focused-node toolbar. Implement the
  hierarchy spec's Up/Down/Home/End, Left/Right, type-ahead, Space inspection,
  Enter-to-details and Escape-to-invoker behavior; Tab leaves normally.
- [x] Show at most three skill badges plus overflow. Keep selected/unconfirmed
  delivery separate from loaded and reported/observed usage. Details expose
  full labels and all supplied skills; unassessed values never appear as zero.
- [x] Supply scenarios for mixed research/execution stages, planned/starting/live/
  remembered/missing-session tasks, a missing parent, blocked work, repeated
  labels across revisions, stale evidence and many tasks/long labels.
- [x] Add the explicit preview command and synthetic banner. All inputs render
  as text, including markup-like labels. Preview operation performs no API,
  WebSocket, Electron IPC, credential, filesystem-write or coding-tool operation
  from the renderer. The preview script keeps generated build output in memory.

Pass: `pnpm --dir apps/desktop preview:workflow` serves a usable preview on
loopback. Switching scenarios and presentations preserves the declared behavior.
Production `pnpm --dir apps/desktop exec vite build` excludes the preview entry
and fixture data, verified through emitted bundle inspection.

### 3. Browser behavior, layout and handoff

- [x] Add DOM tests before implementation for focus versus inspection, refused
  collapse, visible collapsed blockers, plain-text rendering, badge delivery/use
  separation, scenario reset and disabled/absent terminal commands.
- [x] Exercise the real component at wide, medium and compact widths, 360px and
  200% text zoom; assert no document-wide horizontal overflow or clipped actions.
  Check reduced motion and keyboard access to complete labels/details. Reuse
  the existing Electron fixture runner approach, not the large shell fixture.
- [x] Fail tests on unexpected fetch, WebSocket or bridge access. Assert one
  logical outline and no terminal mount; preserve focus when changing views.
- [x] Capture and inspect wide/compact screenshots and walk through finding a
  blocker, inspecting skills, changing view and returning focus. Record actual
  observations and limits; DOM checks do not establish screen-reader support.
- [x] Run desktop type-check, focused tests, production renderer build and
  `bash scripts/verify-repo.sh`; run docs and worktree currency checks. Complete
  current-diff `/code-review medium` (size escalation); required CI and the
  ordinary branch/PR merge gate remain tracked in PR #1078.

Pass: focused tests and required verification pass, screenshots show readable
labels/actions, and a user can review the preview without any tool-capability
claim. Broader #746 acceptance and native assistive-technology validation remain
explicit follow-ups, alongside real projection/session/navigation integration.

## Investigated uncertainties and risk treatment

The principal uncertainty was whether a fixture-backed UI would still activate
the gated Workflow destination. Current `shellNavigation.ts` supports Terminal
and Review only, and the hierarchy spec requires projection agreement before
production integration. A separate fixture entry avoids changing those contracts;
the owner accepted this preview exception before code.

The likely project blind spot is mistaking polished synthetic cards for verified
orchestration. Keep the banner permanent, backend-derived values explicitly
synthetic, and production imports absent. Defer automatic collapse rather than
inventing summary/viewed-state semantics. The next integration plan must consume
accepted source identities; it cannot promote these presentation types into a
protocol by convenience.

| Dimension | Rating | Evidence / treatment |
| --- | --- | --- |
| Dependencies | 2 | Existing React/CSS/esbuild and shell direction; production contracts remain outside this slice |
| Blast radius | 1 | New component and opt-in preview entry; no App or shell navigation edits |
| State changes | 1 | Memory-only presentation state and generated preview output |
| Reversibility | 1 | Remove preview command/component; no migration or saved user state |
| Uncertainty | 2 | Existing DOM runner pattern; actual layout/keyboard behavior verified during implementation |

Total: 7/25. Ratings size review effort; they do not grant approval.

## Plan review and approval gate

Reviewed once against the linked UI proposal, current shell navigation, desktop
build inputs and existing Electron/esbuild test pattern. Initial disposition:
**Ready for owner scope/plan review**; subsequent approval is recorded below.
This remains outside the executable production handoff. The review
removed shell integration and automatic collapse from this slice, made fixture
identity distinct from wire contracts, and required production-bundle isolation.

The owner accepted this fixture-only development preview exception and concrete
implementation plan on 2026-10-26. That approval permits this slice only;
#740 eligibility, production projection/storage, live session navigation and
orchestration launch remain subject to their existing gates. Approval was recorded on #746 and implementation issue #1084 was created before
code. The owned branch and PR #1078 carry implementation. Do not infer broader
contract acceptance.

## Execution evidence

- Pure state regressions: five passing cases covering identity, inspection,
  collapse protection, append/removal and scenario reset; missing module observed
  before implementation.
- Electron DOM/layout regression passes with the real component at 1280, 1000,
  700 and 360 CSS px, 200% root text size and emulated reduced motion. It checks
  keyboard navigation, collapse refusal, static evidence distinctions, scenario
  reset and forbidden bridge/network access. Offscreen focus is exercised with
  explicit DOM focus events; this is not native screen-reader qualification.
- Real loopback server regression passes: generated assets only, CSP with
  connect-src none, read-only methods, invalid port rejection and occupied-port
  failure.
- Wide and compact Electron screenshots were rendered and inspected: task
  labels, blockers and controls wrap within the viewport. Screenshots are local
  validation artifacts rather than released product captures.
- Renderer type-check and production Vite build pass; emitted assets contain no
  preview/scenario markers. Full repository verification and current-head review
  completed successfully; final counts and review are recorded below.

Implementation refinement: the preview script holds the generated HTML/JS/CSS
in memory rather than writing temporary build files. It serves only those exact
artifacts and cleans up by closing the server; this removes filesystem cleanup
without changing the approved preview behavior. No production navigation, wire
contract, storage, source-valid fixture or coding-tool eligibility was added.

### Final verification and review

`bash scripts/verify-repo.sh` passed under Node 22.23.4: 2,271 Rust unit tests
plus integration tests, 13 reporter tests, native ABI repair, 1,544 desktop tests
(2 skipped), desktop type-check/build, documentation build, whitespace and
documentation/worktree currency. Existing Rust dead-code/drop-reference and
Vite/VitePress chunk warnings do not arise from changed Rust or build configuration;
no unrelated cleanup was included. Another owner's merged worktree was left
untouched when it appeared in an earlier currency check.

A read-only Luna-tier `/code-review medium` covered code head
`0cfe119ca7f6817b3d91202602b94248855f5b3d` against planning head
`49d85eb0b93f38ae51b21da7997f81668e27d3a3`: no actionable findings. Medium
effort reflects the code/test diff size. Native assistive-technology validation
was explicitly declined and remains unclaimed; the reviewer did not independently
rerun verification. The authenticated sidecar audit found no matching active
hierarchy/preview recommendation requiring tie-off. Required CI and merge
disposition are recorded in PR #1078.
