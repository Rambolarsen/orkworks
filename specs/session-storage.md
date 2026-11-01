# Ordinary session storage and history

Status: proposed; owner review required before implementation.
Tracking: [#1240](https://github.com/Rambolarsen/orkworks/issues/1240),
[#1241](https://github.com/Rambolarsen/orkworks/issues/1241).
Concrete contract: [SQLite session storage design](../docs/superpowers/specs/2026-11-01-sqlite-session-storage-design.md).

## Purpose

Keep long-lived ordinary session history available without adding historical
record processing to terminal input or routine live-session updates. Session
count is not active runtime count. Preserve the existing observe/recommend
product boundary and exact session/runtime ownership.

## Proposed scope

Use one workspace-owned SQLite store as the authority for ordinary session
metadata, revisions, searchable/indexed history fields and session organization.
Retain replay, event history, terminal-size records and images as external files.
Keep their existing ownership and content bounds. Do not maintain a permanent
authoritative JSON mirror or let external producers mutate the database directly.

This proposal changes the MVP's legacy direct-session-file protocol only after
the qualified producer transition and recoverable cutover defined by the linked
design. Existing source behavior and published installers remain unchanged until
their corresponding implementation and rollout are verified.

## User-visible requirements

- Show active awareness independently of bounded, server-filtered history pages.
  Preserve existing state filters, date ordering and one selected terminal.
- Support lightweight metadata search over title, summary, recorded repository/
  branch and native reference. Full terminal-content search is outside scope.
- Load historical metadata, replay/events and current resource validation by
  exact selected session identity, even if its row is outside loaded pages.
- Paging, filtering, refresh and reconnect never select/acknowledge/resume a
  session or treat an unloaded row as deleted.
- Retain recorded historical facts when repository/worktree state changes.
  Distinguish those facts from current plan/worktree/native resume availability.
- Missing native conversations or external optional payloads do not erase
  readable history. Native resume retains exact-ID checks and existing gates.
- Surface incomplete/unavailable storage and failed persistence accurately;
  never report empty history or durable success as a fallback for storage errors.

## Preservation and authority

Preserve creation/native identities, source priority, freshness/scoped provenance,
checked revisions, titles, summary snapshots, lifecycle outcomes, archive choices,
plan/worktree references and existing content. Active remains derived from actual
runtime/finalization; SQL persistence is not process ownership or acceptance.
Keep close-first Archive/Delete, stale-action checks, archived/unknown-state
retention protection and applied/default retention settings.

Agent metadata writes in migrated workspaces use an independently qualified,
authenticated sidecar protocol with no file fallback. Do not piggyback on gated
Codex native eligibility. Actual agent-context reachability, installed helpers
and generated instructions must be qualified before activation.

Taskmaster, conflict checks, cleanup and reconciliation retain their complete
domain view through appropriate indexed/aggregate/batched queries; the UI page
is never a complete workspace inventory.

## Migration, recovery and exclusions

Migrate only under the selected workspace lease during an explicit quiescent
maintenance window. A valid activation record switches authority once. Preserve
legacy source/backup data, report invalid records without silent omission, and
recover deterministically after an interrupted import/cutover.

Any cutover evidence without a valid activation record blocks normal workspace
opening; a missing marker beside a database never enables legacy JSON writers.
After activation, SQL loss/corruption or an invalid format record is a visible
recovery condition. Never automatically recreate an empty store or fall back to
stale JSON. Unmodified old binaries are unsupported on the migrated workspace;
a lossless downgrade requires a separately verified current-state export under
maintenance. Same-version complete-backup restore is the initial recovery path.

Backup/restore covers the database and required workspace-owned related files
as a consistent set, with explicit exclusions and integrity checks. Database
transactions do not make external-file copies/deletions atomic. External cleanup
uses recoverable identity-bound progress and existing evidence owners.

No automatic replay pruning, retention-default change, native tool storage change,
repository backup, process adoption or live session restart is part of migration.
[#1249](https://github.com/Rambolarsen/orkworks/issues/1249) separately proposes
bulk replay retention. Recommendations, workflow observations, capacity,
readiness, installation settings and separately gated hierarchy storage retain
their existing authorities; backups may capture their files without migrating
their logical ownership.

## Acceptance

The concrete design supplies proposed finite page/queue/payload limits and
numeric performance budgets. Approve those before implementation; verify them
with the same five active sessions at 724, 10,000 and 100,000 historical records.
Measure input latency, history-query time/bytes, CPU and memory with reproducible
fixtures and failure cases. Database use alone is not performance evidence.

Complete producer, migration/crash, backup/recovery, archive/retention, exact
native-resume, off-page selection and stale ownership tests. Run scoped checks,
full repository verification, applicable current-diff code review and required
CI before owner-controlled rollout. No live recovery or supported platform is
claimed until corresponding evidence exists.
