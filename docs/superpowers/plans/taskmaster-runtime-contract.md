# Taskmaster runtime implementation contract

Applies with specs/taskmaster-knowledge.md (issue #503). Root owns desktop and
publisher; Rust worker owns crates/orkworksd only, no commits or subagents.

## Interfaces fixed before implementation

JSON Bundle: `{formatVersion:1,version:string,sequence:integer,publishedAt:ISO,
pages:[{id:string,title:string,type:string,status:string,content:string,
sha256:string,relatedIds:string[]}]}`. Page IDs are relative Markdown paths,
all lengths bounded, SHA-256 of UTF-8 content. Electron verifies signatures;
it sends the validated bundle to authenticated POST /settings/taskmaster/knowledge.

Settings: `{enabled:boolean,selection:null|{provider:string,model:string,
reasoningEffort?:string,ollamaBaseUrl?:string},contextLevel:
"session_observations"|"workflow_context"|"source_code",excludedPaths:string[],
dailyEvaluationLimit:integer,minIntervalMinutes:integer,
automaticKnowledgeUpdates:boolean,workspaceOverrides:Record<string,{
enabled?:boolean,selection?:Selection|null,contextLevel?:ContextLevel,
excludedPaths?:string[],minIntervalMinutes?:integer}>}`.
Overrides keyed by canonical workspace path. Global settings persisted in
~/.orkworks/taskmaster/settings.json; root's desktop does not duplicate them in
appSettings. App-wide daily limit cannot be enlarged by a workspace override.

GET /settings/taskmaster returns `{settings,effectiveSettings,
remainingEvaluations:number,analysisStatus:string,knowledgeVersion:string|null,
lastEvaluatedAt:string|null,lastError:string|null}`. POST /settings/taskmaster
accepts complete Settings, requires the existing Electron sidecar authority,
validates/persists before applying, returns the same status. Reads also require
authority if they expose workspace paths. Paths never come from knowledge.

Recommendation additions are optional/defaulted `repositoryEvidence` and
`knowledgeEvidence` arrays so old evidence remains typed session observations.
Repository evidence: `{path,sha256,excerpt,observedAt}`. Knowledge evidence:
`{pageId,title,status,bundleVersion,sha256,excerpt}`. Keep `evidence` unchanged
for old consumers. Proactive recurrenceCount = 0 and sourceSessionIds = [].
Existing explicit lifecycle and handoff remain the only action path.

## Worker deliverable

Implement settings/status/knowledge routes, independent prompt inference,
bounded context collection, persisted daily reservations, single-flight scheduler,
grounded proactive and enriched recommendations, stale-result rejection,
durable cache keys and existing dismissal preservation. Poll with a bounded
interval but provider calls obey hour/day gates and unchanged inputs cache.
No raw terminal text, arbitrary repository commands, or symlink escapes.
Provider invocation must be tool-free; do not assume arbitrary custom CLI
profiles enforce this. Investigate existing compiled bindings and report a
blocker before silently widening authority or narrowing provider support.

Use TDD and focused real tests; read Rust skill/scoped rules. Show red/green
evidence, then cargo test and fmt checks. One file owner per module. Root may
edit specs and desktop concurrently. Do not revert others' changes.
