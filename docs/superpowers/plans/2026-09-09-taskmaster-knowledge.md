# Brain-informed Taskmaster implementation

Goal: implement [the approved specification](../../../specs/taskmaster-knowledge.md)
under issue #503. Architecture: Electron verifies reference bundles; Rust owns
analysis and recommendation state; renderer uses narrow settings/status APIs.

## Global constraints

- Independent Taskmaster model; no Peon selection changes or fallback.
- Eight daily evaluations, one at a time, one-hour workspace interval by default.
- Durable reservation; bounded context; no background repository commands.
- Signature-verified updates every six hours; compatible cached fallback.
- Evidence provenance, unchanged-evidence dismissal, explicit Fix with AI only.
- pnpm for Node work. Root and both scoped AGENTS.md apply.

## Tasks

1. Record accepted spec and ADR before code; create implementation issues.
2. Implement allowlisted signed publisher and verified Electron updater with
   tamper, privacy, compatibility, cache, and offline tests.
3. Implement independent provider inference, context restrictions, durable
   budgets, grounded recommendations and settings/status routes in Rust.
4. Integrate settings persistence, preload, restoration, background updater,
   renderer controls and provenance. Add boundary and regression tests.
5. Verify relevant suites, review changed code, open logical-unit PRs, and
   report any deployment prerequisites explicitly.

## Routing and ownership

Root owns final integration, specs, publisher/updater, and desktop changes.
One read-only explorer maps provider/settings interfaces while root writes
specs. A later Rust implementation worker owns only crates/orkworksd changes
after the interface is fixed. Root's updater does not consume worker results;
desktop runtime integration waits for the agreed Rust contract. No parallel
writers per file; maximum root plus three helpers. Final correctness and
coverage reviewers use separate contexts and distinct questions. One review
round; root applies evidence-backed corrections within the authorized task.

## Progress

- Initial checkout: 155b42f; clean main. Isolated brain-recommendations worktree.
- Issue #503 created. Accepted spec and ADR 0053 written before implementation.
- No application or brain publishing changes have been deployed.
