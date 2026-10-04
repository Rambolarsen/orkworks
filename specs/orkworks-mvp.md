# OrkWorks — Updated MVP Direction


Mission Control for AI Agents

## Naming and Terminology

Product name: **OrkWorks**.

Protocol directory: **`.orkworks/`** (under `~/.orkworks/`, see [ADR 0018](../docs/adr/0018-global-metadata-store.md)).

Low-cost metadata worker: **Peon**.

Use normal engineering terminology everywhere else: sessions, events, capacity, recommendations, workspaces, worktrees, harnesses, and models. Avoid expanding the fantasy theme into additional product terms. Peon is the single memorable product-specific worker name.


## Product Boundary

OrkWorks is a local-first observability and recommendation layer for AI coding sessions.

The app should help the user understand:

- what AI terminal sessions are currently running
- what each session is doing
- which sessions need user attention
- which sessions are blocked, failed, done, stale, or working
- which harnesses/models are capped, degraded, healthy, cheap, or expensive
- which harness/model is recommended for the next task
- what repo, branch, directory, or worktree each session is running in

OrkWorks should not own the user’s development workflow by default.

It should observe and recommend before it controls. It does not replace Claude Code, Codex, OpenCode, Gemini CLI, or Aider.

## Revised Product Principle

A bad version says:

> Here are twelve terminals.

A good version says:

> Two sessions need your input.  
> One session is done and ready for review.  
> Three are working.  
> One agent is running in a dirty shared workspace.  
> Claude is capped.  
> For your next implementation task, use OpenCode + DeepSeek.

The app should reduce cognitive load without taking over the repo workflow.

## Control Boundary

OrkWorks should own:

- terminal visibility
- session overview
- session metadata
- session status detection
- terminal activity monitoring
- capacity/cap tracking
- harness/model recommendations
- optional repo-local metadata files
- optional Peon metadata normalization

OrkWorks should not own by default:

- git workflow
- branch strategy
- worktree creation
- merging
- rebasing
- resetting
- stashing
- cleanup of branches/worktrees
- task decomposition
- automatic terminal input, except the explicit user-approved session-plan review prompt defined in `specs/session-plan-review.md`
- automatic command approval

Workflow actions may be added later as explicit opt-in conveniences, but they are not part of the initial MVP.

## Git and Worktree Context

Git worktree support should be context detection first, not workflow control.

OrkWorks should detect and display Git context for each terminal session:

- current working directory
- Git repository root
- current branch
- whether the directory is a Git worktree
- whether the working tree is dirty
- changed file count
- live added/removed line counts for uncommitted changes
- changed files where practical
- relationship between multiple sessions in the same repo
- whether multiple sessions are sharing the same working directory

OrkWorks may use this context for recommendations.

The session Details Git row shows current uncommitted changes as, for example,
`3 files · +124 −37`. Compare the working tree against `HEAD`, accounting for
staged and unstaged changes together without double counting, and include
non-ignored untracked files. Before the first commit, use an empty-tree
baseline. Detect renames before counting lines so pure renames contribute no
line changes. Binary files count as changed files but contribute no line totals.
Clean repositories show zero line totals; unavailable statistics are omitted,
never substituted with zero. Refresh through the existing session Git-context
projection and reuse the diff for sessions sharing a worktree. These totals
describe the worktree, not edits attributable to one session; sessions sharing
a worktree show the same totals. No coding-tool status-line parsing is needed.

Example recommendations:

```text
This session is running in a separate worktree. Good isolation for parallel agent work.
```

```text
Three active sessions are running in the same dirty workspace. Consider using separate worktrees for new coding tasks.
```

```text
This looks like a review/debugging task. Running in the main workspace is probably fine.
```

```text
This looks like a risky implementation task. A separate worktree may be safer.
```

The user, repo skill, harness, or existing workflow decides whether to create a worktree.

OrkWorks should not create, delete, merge, rebase, reset, or clean up worktrees in the MVP.
The proposed [ordinary-child orchestration extension](taskmaster.md#proposed-ordinary-child-orchestration-extension)
is a narrow opt-in exception, tracked by
[#610](https://github.com/Rambolarsen/orkworks/issues/610). Its scope alignment
was accepted in PR #747; detailed contract review, capability evidence and
scoped implementation approval remain required before runtime work. One UI-created
parent run coordinates separate exactly approved research/execution plans;
completion of research permits synthesis, not execution launches. An exact
immutable plan approval is required before provisioning branches/worktrees or
launching declared children through the existing sidecar. Independent chains
use separate worktrees; sequential reuse within one plan requires an ended
predecessor and the user's explicit quiescence acknowledgement.

Each child task has one attempt. OrkWorks does not transfer code, commit, merge,
rebase, push, change existing branches or delete branches. Child edits remain
for manual integration. The initial slice has no automated worktree cleanup;
any later separately reviewed removal requires a clean, quiescent, plan-owned
worktree and preserves branches/commits. Coding-tool profile/delivery evidence,
detailed contract review and scoped implementation approval remain prerequisites;
native confinement is not claimed or required by this proposed slice.

## Repo Skills Boundary

Repo skills and OrkWorks may overlap, but they should have different responsibilities.

OrkWorks is the runtime observability layer.

Repo skills describe how agents should behave inside a repository.

The split should be:

```text
OrkWorks = observe sessions, detect state, show overview, recommend next action
Repo skills = tell agents how to work in this repo and how to report status
Peon = fallback metadata normalizer when agents do not report well
```

Repo skills may instruct agents to:

- update `.orkworks/sessions/<session-id>.json`
- append events to `.orkworks/events/<session-id>.ndjson`
- summarize current work
- report blockers
- report test status
- report files touched
- avoid merging unless explicitly asked
- follow the repo’s preferred branch/worktree workflow

Repo skills should not redefine the OrkWorks protocol.

OrkWorks should treat repo skills as instructions for agents, not as the source of application lifecycle logic.

## Metadata Source Priority

When multiple systems provide session metadata, OrkWorks should use explicit priority.

Priority order:

1. User/manual override
2. Explicit agent-written session JSON
3. Peon inference
4. Backend deterministic inference
5. Process state only

Session metadata should include source and confidence where useful:

```json
{
  "status": "waiting_for_input",
  "summary": "Needs decision about API compatibility.",
  "metadataSource": "agent",
  "metadataConfidence": "high"
}
```

Valid metadata sources:

- `user`
- `agent`
- `peon`
- `backend_inference`
- `process`
- `unknown`
- `debug` (debug-only temporary state injection; lower priority than normal runtime sources)

Peon must not overwrite higher-priority metadata unless the higher-priority metadata is stale or explicitly cleared.

### Resolved harness capabilities and integrations

OrkWorks resolves versioned, embedded built-in harness definitions together
with sparse version-2 user overrides into one immutable registry. Launch,
resume, models, Peon, capacity, native signals, voice projection, and workspace
integration status all read that one snapshot. A definition expresses support
by the presence of a closed capability binding; it does not use independent
"supports" booleans. User overrides preserve omitted built-in fields, while
complete custom definitions cannot directly select compiled signal handlers,
reporters, or authority-bearing paths. An explicitly supported duplicate
operation may attach a sidecar-owned, allowlisted compatibility profile stored
outside the editable custom definition; the profile derives an existing closed
binding and cannot contain user-supplied code, paths, or handler selection.

The version-3 user document contains `version`, `overrides`, `custom`, and a
sidecar-owned `compatibilityProfiles` map. An override is keyed by the
immutable built-in ID. Arrays replace, nested objects merge, tagged-kind
changes replace the complete capability, and `null` removes only an optional
capability.
Legacy v1 arrays and version-2 documents remain readable and are migrated in
memory; only a later successful save writes v3. A failed write never publishes
an unpersisted registry. Compatibility profiles are sidecar-owned, keyed by
immutable custom harness ID, and validated against a compiled allowlist.

Each harness has read-only, independent integration axes: `enabled`, tool
detection, registration (`unsupported`, `absent`, `installed`, `drifted`, or
`error`), ownership, activation/trust, coverage, and diagnostics. Workspace
install, repair, and uninstall are explicit, idempotent actions only. They
require Electron-main confirmation and sidecar mutation authority; the
renderer and child processes never receive that authority. Mutations are
workspace-only, canonical/no-follow contained, ownership-aware, and durable
write-before-publish. OrkWorks never edits tracked/shareable configuration,
`.gitignore`, or arbitrary user hook commands. See
[ADR 0026](../docs/adr/0026-resolved-harness-capability-registry.md).

### Deterministic harness-supplied signals

Alongside Peon's LLM-based inference, some harnesses expose deterministic, higher-confidence signals that OrkWorks can consume directly instead of inferring them from terminal output:

- **Attention state** (`waiting_for_input` and related statuses): a harness's own notification mechanism — e.g. Claude Code's `Notification` hook — can call `POST /sessions/:id/attention` on the sidecar. Writes use `metadataSource: "agent"` with `metadataConfidence: 1.0` and respect the same priority/staleness rule Peon already respects: they cannot overwrite fresh `user` or fresh `agent` metadata, but always outrank `peon`/`backend_inference`/`process`/`unknown`.
- **Session plan/spec association**: a harness may report an optional workspace-relative Markdown `planPath` independently of attention state; JSON `null` clears it and omission preserves it. When no harness path exists, OrkWorks may conservatively associate a valid printed path below `docs/superpowers/plans/` or `specs/`. The renderer receives availability and validated document content, never a filesystem path, and displays it in the reusable Review tab. Electron main may request the one user-approved fixed review prompt through its per-sidecar secret; the sidecar revalidates the artifact before PTY input. See [ADR 0025](../docs/adr/0025-authenticated-session-plan-handoff.md) and [ADR 0034](../docs/adr/0034-user-approved-session-review-prompt.md).
- **Harness-native session ID and Codex label enrichment**: a harness-specific mechanism (env var, hook JSON, structured JSONL event) reports the session's native ID via `POST /sessions/:id/harness-session`, tagged with a source string and confidence. This is the same generic capture endpoint used for OpenCode's `OPENCODE_SESSION_ID`, Claude Code's hook `session_id`, and Codex's hook `session_id`; when a Codex report also authenticates with `ORKWORKS_REPORT_TOKEN`, the sidecar may read the exact native thread from the supported local `state_5.sqlite` store and use `threads.name`, then `threads.title`, as the automatic session label. Unsupported or unavailable data preserves the existing label; prompt and rollout JSONL parsing is out of scope. See `skills/adding-harness/`.
- **Codex hook report transport under network sandboxing**: Codex hooks may be unable to POST to OrkWorks' loopback sidecar when command networking is sandboxed. For Codex only, the sidecar may give the session a private, temporary report mailbox and consume native harness-session reports from that mailbox. The mailbox relays identity reports only; it does not grant loopback or outbound network access to ordinary Codex commands. The sidecar applies the same in-memory reporting-token authentication, hook fingerprint/provenance checks, and Codex identity-reset rules as the HTTP route. The reporting token is never written to a mailbox file. Attention and other reports retain their existing transport. The mailbox path is a session reporting capability, not proof of which child process wrote a report; see [ADR 0069](../docs/adr/0069-codex-session-id-hook-report-mailbox.md).
- **Codex identity and resume integrity**: Codex CLI subagents are internal to their owning CLI session; they do not create OrkWorks sessions or receive independent OrkWorks session IDs. Retain the first accepted native Codex session ID; a different ID may replace it only on an authenticated root `SessionStart` with `source=clear` after OrkWorks recorded the explicit reset. Resume only with that exact ID and only when the corresponding thread row and rollout file exist in the supported local Codex store. Missing or unsaved IDs never fall back to another Codex conversation. See [ADR 0068](../docs/adr/0068-codex-subagents-share-owning-session-identity.md).
- **Claude/Copilot native identity across explicit resets**: Keep the OrkWorks
launch generation immutable. Rebind a native ID only for an exact reset
declared in the persisted harness definition. Reserve before PTY dispatch;
queue an early lifecycle report until successful delivery acknowledgement,
then advance the epoch, retire the old ID, clear the prior prompt tuple, and
bind the replacement. Failed or canceled delivery discards candidates and
preserves prior state. Copilot timestamps must be later than reservation
creation. Claude has no timestamp, so a delayed report from an earlier clear
may be accepted during a later reservation. The event/source claim is not
root-process proof. Copilot currently declares only bare `/clear` and `/new`;
prompt-bearing forms and `/reset` remain outside the accepted scope in ADR
0040 and issue #326.

These are opt-in per harness and never installed automatically. Generic
workspace integration routes report status and, only after explicit
Electron-main confirmation, install/reconcile or uninstall a supported
integration. A contract without an exact primary-source payload fixture remains
limited or unsupported rather than inferred.

An installed integration or declared harness capability does not by itself
make a signal authoritative or suppress Peon and terminal fallback. Hook
authority is scoped to one live session and begins only after the sidecar
accepts a session-correlated, event-validated report. Before that report,
Claude Code and GitHub Copilot sessions retain Peon and terminal fallback for
prompt inference until the sidecar accepts a recognized report from the
session's prompt-notification hook (`Notification` for Claude Code;
`notification` for GitHub Copilot). Before activation, all turn events are
attention no-ops: they do not write turn status, clear an existing wait, set
prompt-authority state, or block Peon from updating prompt fields. Only a
recognized prompt notification activates that channel's authority. Before
activation, recognized nonprompt notifications also do not write attention or
a readiness-only state. After activation, only the allowlisted turn and
prompt-clear events mapped for that harness may update or clear attention;
they do not activate authority. Unknown or malformed notifications do not
change state. Each Claude/Copilot report must include
the native ID carried by that event (`session_id` for Claude, `sessionId` for
Copilot), the OrkWorks session's valid report token, and the immutable
generation inherited from that session's launch environment. Before
the reporter sends an attention report, it must register that event's native
ID through `POST /sessions/:id/harness-session` with the same token and
generation, using `Authorization: Bearer <ORKWORKS_REPORT_TOKEN>` on both
requests, and send attention only after that registration succeeds. This
ordering applies to the first prompt notification as well as later reports;
registration failure or rejection stops the reporter from sending attention.
The owned hook set also reports Claude `SessionStart` and Copilot
`sessionStart` solely to bind a replacement native ID after an explicit
recorded reset; those lifecycle events never write attention or activate
prompt authority by themselves. Their native-ID registration carries the
actual event name and `source` through `sessionStartEvent` and
`sessionStartSource`, and the sidecar accepts only the harness-specific event
and source pair that matches the recorded reset.
Identity registration alone does not activate prompt authority. The sidecar
accepts the identity binding only for the live session and its unrevoked
launch generation. The first successful registration binds the current
conversation epoch to one native ID. A different ID can rebind only for an
exact reset command declared in the persisted harness definition: Claude
`SessionStart(source=clear)` or Copilot `sessionStart(source=new)`. Before
dispatching that command to the PTY, OrkWorks opens a single-use reservation
for the current epoch. A matching lifecycle registration for a replacement ID
that arrives before the PTY write acknowledgement is held as a candidate,
along with any following reports for that ID; it does not change the binding,
epoch, prompt tuple, or attention yet. The lifecycle registration is not
reported as accepted until delivery resolves. On successful delivery
acknowledgement, the sidecar commits the reset, advances the epoch, retires
the old ID, and clears the prior prompt tuple. If a candidate is queued, it
binds the replacement ID and queued reports are checked afterward; otherwise
the committed reservation stays available for one matching lifecycle
registration, which then binds the replacement ID. Until that binding is
accepted, reports and exact-resume lookups using the old ID are rejected; the
sidecar does not fall back to the prior conversation. Failed or canceled delivery
closes the reservation, discards its candidate reports, and leaves the
existing binding, epoch, and tuple unchanged. Only one reset delivery may be
pending per session; a later exact reset supersedes an unmatched committed
reservation. Copilot's event timestamp must be later than reservation
creation, and successful delivery acknowledgement is still required. Claude's
payload has no documented timestamp, so receipt order cannot distinguish a
delayed report from an older clear received during a later reservation.
The launch generation remains unchanged. Copilot currently declares only bare `/clear` and `/new`;
optional prompt-bearing forms and `/reset` remain outside the accepted
label-reset scope in [ADR
0040](../docs/adr/0040-harness-declared-session-label-resets.md) and [issue
#326](https://github.com/Rambolarsen/orkworks/issues/326), even though [the Copilot CLI command reference](https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-command-reference) lists them as
starting new conversations. They do not authorize rebind under this
proposal; expanding support requires a separate reviewed spec and
implementation scope.
The sidecar accepts attention only when the same token and
generation accompany an allowlisted harness/event/status and the event's native
ID matches the sidecar's current conversation-epoch binding. The sidecar issues
an immutable generation at launch only
when the integration is enabled and its owned prompt-notification hook is
available; otherwise the environment variable is absent and reports are
rejected. It passes the value as `ORKWORKS_PROMPT_HOOK_GENERATION`. Hook
reporters capture and forward that exact
value for both requests; they do not fetch a replacement generation while
submitting. Disable, uninstall, or detected drift revokes it. Re-enabling does
not update an already-running harness: that session remains on fallback until
it is relaunched under a new OrkWorks live session with a fresh generation.
Missing, invalid, stale,
unregistered, or mismatched values are rejected without changing authority or
attention. Codex and OpenCode
retain their accepted nonprompt-only fallback: before activation, Peon may
supply nonprompt status and descriptive metadata; conversational text cannot
establish prompt attention before or after hook activation. Aider and hookless
tools retain their existing Peon and terminal fallback.

For Codex and OpenCode, an accepted direct signal owns that session's attention
fields (`observed_status`/`attention`, `needsUserInput`, `detectedQuestion`,
and `suggestedOptions`). For Claude Code and GitHub Copilot, an accepted
prompt-notification report activates authority over those fields. Metadata
source is record-wide, so the sidecar cannot identify provenance for each
prompt field separately; treat the four fields as one tuple. On activation or
demotion, clear the whole tuple when the record-wide source is not `user`, and
preserve the whole tuple when it is `user`. This intentionally clears any
non-user tuple, including a newer accepted direct report, because the record
does not retain per-field provenance. Source priority governs later writes; it
cannot preserve individual non-user fields during this group clear. Do not
selectively retain fields based on assumed per-field ownership.
Peon continues
summaries, phase, diagnostics, and workflow evidence but cannot create or
replace attention or prompt fields from conversational text after the
applicable authority activates. A prompt can be missed when a hook is absent,
disabled, delayed, or unable to report. After activation, a lost prompt-clear
event can leave stale attention until a later recognized turn event, accepted
terminal input, or session lifecycle transition. Explicit disable/uninstall, or
integration reconciliation that detects the owned prompt-notification hook is
missing or drifted, ends Claude/Copilot authority, clears the prompt tuple as a
unit under the record-wide source rule above, and returns future inference to
Peon/terminal fallback. Silence alone cannot prove hook loss. The UI must
preserve that uncertainty rather than treat an installed configuration as
proof that hooks are running. Before activation, Claude/Copilot turn-event
reports are attention no-ops: writing session-wide `agent` metadata would
temporarily prevent Peon from writing prompt fields under the current
record-wide source-priority rule. The sidecar's existing committed-terminal-
input transition continues to clear waits independently. Every Claude/Copilot
attention report carries the immutable, per-live-session generation issued by
the sidecar at launch when the integration is enabled and its owned
prompt-notification hook is available. Otherwise the environment variable is
absent and reports are rejected. The sidecar passes it as
`ORKWORKS_PROMPT_HOOK_GENERATION`; reporters inherit it and forward it
unchanged, capturing it at invocation start rather than fetching the current
generation at POST time. Disable, uninstall, or detected drift revokes that
generation before clearing the prompt tuple under the record-wide source rule
above, and reports from a revoked generation cannot restore authority.
Re-enabling does not change the
environment of an already-running harness; that session remains on
Peon/terminal fallback until it is relaunched under a new OrkWorks live session
with a fresh generation. Reports without a generation are rejected. This fences
lifecycle races without claiming to authenticate the sending process.

The proposed signal contract is harness-specific:

| Harness | Direct prompt signal | Turn and prompt-clear signals | Authority and tradeoff |
| --- | --- | --- | --- |
| Claude Code | `Notification` with `notification_type` `permission_prompt`, `elicitation_dialog`, or `elicitation_url_dialog` means an explicit prompt is awaiting the user. Do not treat `PermissionRequest` alone as proof that a user-facing prompt appeared; it runs before the permission flow and may end without a visible prompt. | After authority activates, `UserPromptSubmit` clears an earlier wait and reports `working`. Do not clear permission waits from generic successful `PostToolUse`: its `tool_use_id` cannot be matched to `PermissionRequest`, which has no such ID. `Stop` is not a completion signal: another configured Stop hook can block stopping, so it must not report `idle` or clear a wait ([Claude Stop contract](https://code.claude.com/docs/en/hooks#stop-decision-control)). `idle_prompt` may report `idle` after its documented delay but does not clear an elicitation. `elicitation_response` and `elicitation_complete` clear an unambiguous outstanding elicitation after activation; without a request ID they must not clear a permission wait or an ambiguous parallel elicitation. `PreToolUse` must not write attention or clear a prompt. Before authority activates, turn events do not write attention. | Only an accepted prompt `Notification` report for the captured native session activates authority; turn events before activation are attention no-ops. A different native ID can bind only for an exact declared reset. The sidecar reserves before PTY dispatch, holds an early lifecycle report, and binds only after successful delivery acknowledgement. Claude has no event timestamp, so a delayed report from an earlier clear may arrive during a later reservation. The launch generation is unchanged. The report does not prove root-process origin. Reconcile detected removal or drift of the owned notification hook by revoking its report generation, demoting, and clearing its fields. Permission notifications may be delayed about six seconds and have no timestamp or turn ID; a late event may reopen stale Needs You. `idle_prompt` arrives about 60 seconds after a response only if the user has not typed. |
| GitHub Copilot CLI | `notification` with `notification_type` `permission_prompt` or `elicitation_dialog` means an explicit user prompt. `agent_idle`, `agent_completed`, and shell-completion notifications describe background work, not a prompt for the owning OrkWorks session. | After authority activates, `userPromptSubmitted` clears prior waits and reports `working`. Do not clear permission waits from generic successful `postToolUse`, whose payload has no invocation ID to correlate with the prompt. `agentStop` can be blocked by another configured hook and force continuation, so it does not report `idle` or clear a wait. Elicitation clears on accepted input or session end. The documented contract has no matching notification for a rejected permission decision. Before authority activates, turn events do not write attention. | Only a recognized prompt `notification` report for the matching native session activates authority; turn events before activation are attention no-ops. A different native ID can bind only for an exact declared reset. The sidecar reserves before PTY dispatch, holds an early lifecycle report, and binds only after successful delivery acknowledgement; Copilot's timestamp must be later than reservation creation. Only bare `/clear` and `/new` are declared; prompt-bearing forms and `/reset` remain outside the accepted scope in ADR 0040 and issue #326. The launch generation is unchanged. The report does not prove root-process origin. `agent_idle` and `agent_completed` are background-subagent notifications; with `agentStop` non-final, the documented hooks provide no safe root-turn idle signal. After `userPromptSubmitted`, `working` may persist until another authorized status event or session end. Treat reliable root-idle reporting as a separate signal-design gap. Reconcile detected removal or drift of the owned notification hook by revoking its report generation, demoting, and clearing its fields. Order timestamped reports against accepted reports and committed terminal input; reject older or invalid/missing timestamps. |
| Aider | No supported event identifies an explicit user question or permission request. Its notification command runs when a response finishes and Aider is ready for another input; that is ordinary turn completion, not proof of Needs You. | The callback is correlated to the owning OrkWorks session through launch environment, but carries no structured Aider event payload or paired prompt lifecycle. Do not write `waiting_for_input` or `idle`: either agent-priority status can suppress Peon's ability to recognize a concrete question. | Aider notifications never activate prompt authority or write attention state. Remove its launch-time static hook flag so Peon and terminal fallback remain available even when the callback is installed. This can miss a question Peon does not recognize, but avoids speculative waits and suppressing Peon with an idle report. |

**Proposed Codex approval extension (#690; not implemented).** The serial
ADR 0051 capture gate passed, but the shipped PermissionRequest mapping still
has the automatic-review false-positive gap. The
[written native approval design](../docs/superpowers/specs/2026-10-03-codex-native-approval-status-design.md)
and [proposed ADR 0076](../docs/adr/0076-codex-owned-native-approval-observer.md)
add a two-second grace and runtime-owned passive native confirmation/resolution,
with isolated execution, native terminal approval control, and conservative
fallback. Written-spec approval and the documented signal, configuration, and
platform gates are required before implementation/rollout. This proposal does
not weaken hook authority or authorize OrkWorks to answer approvals.

The hook-capable integrations report `waiting_for_input` only for the explicit
prompt events above. Unknown events, malformed payloads, reports for another
native session, and reports that fail session or harness validation do not
change authority or attention. A future deterministic signal may extend a
harness's mapping only after its source contract, clear behavior, missing-hook
fallback, and false-positive/false-negative tradeoffs are reviewed. Summary and
diagnostic inference stays independent of attention authority.

For Claude/Copilot prompt reports, the session-bound report token and
unrevoked launch generation are required on both native-ID registration and
attention submission. They bind accepted reports to the live OrkWorks session
and fence disabled or stale integrations, but do not authenticate the sending
process: spawned children inherit the token, and a process with access to the
same session environment can submit matching values. This protocol does not
claim to prevent same-session process spoofing or prove that the harness itself
emitted an event. Validate the allowlisted harness/event/status and native-ID
binding as report consistency checks, not as cryptographic proof of event
origin. Aider lacks native identity, so its callback is correlated only to the
OrkWorks session that launched it and must not be treated as proof of an
explicit prompt.
Copilot events carry timestamps and must be rejected when older than a newer
accepted event or committed terminal input. Claude notifications have no
documented event timestamp or turn ID; process them in receipt order and retain
the documented stale-reopen risk rather than claiming reliable stale-event
rejection.

“Accepted terminal input” means input committed by the sidecar as work through
the existing Enter-terminated or deterministic single-key `CommittedWorking`
transition. Raw character typing and queued, unsent input do not clear a wait.
For Copilot timestamp ordering, only a committed-work transition advances the
input boundary that can reject an older report. Raw input frames must not
advance that boundary; keep a separate committed-input timestamp if the
existing input timestamp records every nonempty frame. Once Claude/Copilot
authority has recorded a permission prompt, its deterministic single-key
`CommittedWorking` response clears that permission wait. Preserve enough
runtime prompt-kind state to avoid clearing a free-form elicitation wait on raw
typing or a single key; elicitation clears on accepted submitted input, its
mapped clear event, or session end.

The detailed evidence, current implementation gaps, and validation bar are
recorded in the [other-harness attention review](../docs/superpowers/specs/2026-09-27-other-harness-prompt-attention-design.md).
Codex and OpenCode's existing session-scoped authority boundary is described in
the [OpenCode prompt attention design](../docs/superpowers/specs/2026-09-26-opencode-prompt-attention-design.md).

## Peon

The MVP should include Peon: a low-cost AI observer responsible for maintaining session metadata and improving observability.

Peon should help normalize messy terminal output into useful OrkWorks state.

Peon may infer:

- status
- phase
- summary
- next action
- whether user input is needed
- detected question
- suggested options
- blocker description
- failed command or failed test summary
- capacity/cap hints
- confidence
- workflow-observation candidates (see "Workflow observations" below), independently of the current-situation fields above

Peon may update:

- `.orkworks/sessions/<session-id>.json`
- `.orkworks/events/<session-id>.ndjson`
- `.orkworks/capacity/<capacity-id>.json`
- `.orkworks/workflow-observations/<session-id>.ndjson`, through the shared recording module only (never by direct file write from the inference path)

Peon must not:

- type into terminals automatically
- approve commands
- decide merges
- delete files
- modify source code
- override user decisions
- treat inference as more authoritative than explicit agent/user metadata
- turn ordinary progress, terminal redraws, or speculative advice into a workflow observation
- write recommendations

The first MVP autonomy level for Peon is observer-only.

Later versions may add suggested terminal input, but it must be gated by explicit user approval.

### Current-summary snapshot

`summary` is a first-class snapshot, not text whose provenance is borrowed from unrelated record-wide metadata fields. The session contract carries `summary`, `summarySource` (`agent` | `peon`), `summaryConfidence`, and `summaryObservedAt` together:

- An accepted non-empty Peon summary or agent attention message (with a message) replaces all four fields together.
- An attention report without a message leaves all four fields unchanged.
- A newly submitted descriptive user instruction, and an accepted session-label reset command, clear all four fields synchronously, so the previous turn's activity cannot appear current while new work starts. Non-descriptive confirmations and hotkeys do not clear the snapshot.
- The snapshot does not expire by wall-clock age. Taskmaster uses only summaries carrying the dedicated source/confidence/timestamp fields for handoff evidence; legacy sessions with only a flat `summary` remain displayable in the selected-session headline but are not Taskmaster handoff evidence until a new accepted summary populates the dedicated fields. The last summary of an ended session remains useful handoff context. It is never treated as workflow-friction evidence.
- Historical event records containing superseded summary-checkpoint fields (see [ADR 0042](../docs/adr/0042-workflow-observations-replace-summary-checkpoints.md)) remain readable; no checkpoint is appended for new updates and no checkpoint-log route is offered.

## Workflow observations

Peon and coding agents also produce a second, independent output: `WorkflowObservation` records answering "what made this work harder than necessary?" — deliberately separate from the current-summary snapshot above, which answers "what is happening now?" Taskmaster never parses activity-summary prose to manufacture workflow evidence, and Peon never writes recommendations. See [ADR 0042](../docs/adr/0042-workflow-observations-replace-summary-checkpoints.md) and `specs/taskmaster.md` for how Taskmaster correlates these records into improvement recommendations.

### Record shape

```text
WorkflowObservation
  id                  stable occurrence identity
  sequence            durable monotonic workspace append order
  sessionId           originating OrkWorks session
  observedAt          accepted timestamp
  kind                repetition | obstacle | missing_context | assumption |
                      correction | workaround | verification_gap
  description         concise statement of the friction (max 500 Unicode scalar values)
  evidence            concrete action, missing fact, correction, or outcome (max 2,000 Unicode scalar values)
  reportedImpact      low | medium | high
  source              agent | peon
  confidence          confidence that the observation is accurate
  fingerprint         versioned, server-derived correlation key
  idempotencyKeyHash  server-derived durable retry identity; not API-exposed
```

`source: agent` carries a fixed confidence of `0.9`, assigned by the authenticated reporting adapter; the caller cannot set it. `source: peon` carries Peon's own required per-candidate confidence. An observation is immutable while retained; bounded storage and explicit session deletion may remove it. Higher confidence makes evidence more useful; it does not make the claim unquestionably true, and every resulting recommendation remains dismissible.

### Recording module

One workflow-evidence module owns validation, normalization, fingerprinting, deduplication, persistence, retention, and retrieval, reachable only through:

```text
record_observation(session_id, origin, idempotency_key, candidate)
  -> accepted observation, duplicate identity, or rejection
workspace_observations(workspace_id) -> observations in append order
delete_session_observations(session_id) -> deletion outcome
```

Two adapters cross this seam: the explicit agent-report HTTP adapter and the Peon inference adapter. Neither adapter implements storage, fingerprinting, or deduplication rules directly, and Taskmaster reads through the module rather than opening metadata files directly.

### Explicit agent reporting

```text
POST /sessions/:id/workflow-observations
```

Every live session receives an independent 256-bit random reporting capability in `ORKWORKS_REPORT_TOKEN` (not persisted, replaced on resume), alongside the existing `ORKWORKS_SESSION_ID` and `ORKWORKS_PORT`. The route requires `Authorization: Bearer <ORKWORKS_REPORT_TOKEN>` and an `Idempotency-Key` header (1–128 visible ASCII characters), rejects missing/malformed/wrong capabilities without recording an observation, accepts only `kind`, `description`, `evidence`, and `reportedImpact` in the request body (workspace identity, source, confidence, fingerprint, and recommendation fields are all server-derived), limits the complete body to 8 KiB, and enforces at most 30 reports per session in a rolling 60-second window (`429` beyond that). The route reports evidence only; it cannot create or mutate a Taskmaster recommendation directly.

The same per-session capability authenticates the agent completion handoff:

```text
POST /taskmaster/recommendations/:id/complete
```

The sidecar derives the caller's live session from `ORKWORKS_REPORT_TOKEN`,
requires the recommendation's `targetSessionId` to match that session, accepts
only an optional bounded completion summary, and transitions an accepted
`improve_workflow` recommendation to `completed`. It rejects caller-supplied
session IDs, target sessions, recommendation content, and lifecycle states.
Missing or failed completion reports leave the recommendation accepted rather
than inferring completion from terminal output.

### Storage and limits

```text
~/.orkworks/workspaces/<hash>/workflow-observations/<session-id>.ndjson
~/.orkworks/workspaces/<hash>/workflow-observations/sequence
```

Segments are session-scoped for exact deletion but aggregated workspace-wide for correlation. Each session segment is bounded to the newest 1,000 observations and 2 MiB (including reserved compact idempotency tombstones); workspace reconstruction for Taskmaster reads at most the newest 10,000 observations across all segments, ordered by `sequence`. `DELETE /sessions/:id/forget` and automatic retention delete a session's observation segment and every recommendation derived from it, in the same cleanup path as session metadata and events.

## Updated MVP Scope

The first useful MVP should include:

### Must Have

#### Electron Desktop Shell

- Electron app shell
- On Windows, one integrated 38px header replaces the separate native title bar: OrkWorks icon at the far left, workspace name and switch action, connection status, and native minimize/maximize/close controls at the right. Preserve dragging, resizing, and keyboard menu access (Alt reveals the auto-hidden application menu). macOS and Linux retain their existing window chrome.
- React + TypeScript UI
- VS Code-like three-column layout
- left sidebar with workspaces/sessions
- center embedded terminal
- right sidebar with action overview, capacity, and recommendation panels
- Electron launches Rust backend sidecar
- frontend communicates with backend over localhost HTTP/WebSocket
- secure preload bridge
- `nodeIntegration: false`
- `contextIsolation: true`

#### App Settings and Hotkeys

- persist app-level settings in Electron user data
- use a versioned app settings object that can grow over time
- implement `hotkeys` as the first settings section
- support the currently implemented shortcuts only:
  - new session
  - sessions panel shortcut
  - detail panel shortcut
  - terminal panel shortcut
  - capacity panel shortcut
  - recommendations panel shortcut
  - reset layout shortcut
- default hotkeys must match the shipped accelerators
- build Electron menu accelerators from saved settings rather than hard-coded constants
- expose an in-app settings entry point in the desktop UI
- provide a settings modal with a Hotkeys section
- support edit, per-hotkey reset, restore defaults, cancel, and save
- validate invalid, duplicate, and required hotkey values before persisting changes
- prevent existing accelerators from firing while a replacement hotkey chord is being captured
- preserve the current Sessions panel shortcut behavior when that shortcut is customized
- saved hotkeys must survive app restart

#### Rust Backend

- Rust sidecar process
- Axum HTTP/WebSocket API
- dynamic localhost port
- health endpoint
- session registry
- PTY process manager using `portable-pty`
- terminal output streaming
- terminal input forwarding
- terminal resize support
- kill/archive session support

#### Terminal Sessions

- start terminal sessions through backend
- support multiple running sessions
- switch between sessions without killing processes
- preserve recent scrollback per session
- show active session metadata:
  - task
  - harness
  - model
  - workspace
  - working directory
  - status
  - phase
  - last activity

#### Session Lifetime

- PTY/process lifetime is owned by the sidecar session runtime, not by the renderer's terminal WebSocket (ADR 0022)
- a renderer reload, crash, or a switched-away/disposed terminal detaches the WebSocket but does not kill the child process
- detached session runtimes keep draining PTY output, persisting terminal history, and feeding Peon/metadata inference while `orkworksd` stays alive
- one interactive terminal attachment per session; reattaching replays bounded recent history (ADR 0024) from the current cursor
- explicit kill (or the process exiting on its own) is what ends a session — not losing the WebSocket
- app-restart PTY persistence is out of scope for the initial implementation: after a sidecar restart, sessions reconcile through existing metadata/lifecycle rules rather than being treated as live detached PTYs

#### Session Metadata Protocol

- create `.orkworks/` structure under `~/.orkworks/workspaces/<path-hash>/` when enabled:
  - `sessions/`
  - `events/`
  - `capacity/`
  - `skills/`
  - `workflow-observations/`
- read/write `sessions/<session-id>.json`
- watch session JSON files for changes
- trust explicit agent-written session JSON
- infer state when JSON is missing or stale
- append basic event logs to `events/<session-id>.ndjson`
- record bounded, sequenced workflow observations to `workflow-observations/<session-id>.ndjson` through the shared recording module (see "Workflow observations" above)

#### Git Context Detection

- detect whether session directory is inside a Git repo
- detect repo root
- detect branch name
- detect dirty/clean state
- detect changed file count
- detect whether directory is a worktree where practical
- show Git context in the UI
- include Git context in session metadata
- warn when multiple active sessions share the same dirty working directory
- recommend worktree isolation for suitable parallel coding tasks

#### Peon

- collect recent terminal output per session
- call cheap model with compact context
- require strict JSON response
- validate model response against schema
- update session JSON with inferred metadata
- append Peon notes to event log
- may emit workflow-observation candidates (kind, description, evidence, reportedImpact, own confidence) alongside or independently of session-situation inference
- show confidence/source in UI
- never send terminal input automatically

#### Right Sidebar

The right sidebar should answer:

> What do I need to look at right now?

Groups:

- Needs You
- Blocked
- Failed
- Done
- Stale
- Working
- Idle
- Capacity
- Start Next Task

Sessions should be prioritized:

1. `waiting_for_input`
2. `blocked`
3. `failed`
4. `done`
5. `stale`
6. `working`
7. `idle`

#### Harness Configuration

- declarative built-ins plus sparse user overrides and complete custom definitions
- generic terminal adapter
- ability to start configured commands such as:
  - OpenCode
  - Codex
  - Claude Code
  - Antigravity CLI
  - Aider
- harness/model labels in UI
- initial prompt/instruction injection or display
- small compiled bindings only for verified tool-specific protocols

#### Capacity Tracking

- manual capacity state
- capacity JSON files
- status values:
  - `healthy`
  - `degraded`
  - `capped`
  - `unknown`
  - `disabled`
- cost tiers:
  - `local`
  - `low`
  - `medium`
  - `high`
  - `premium`
- output pattern detection for:
  - usage limit reached
  - rate limit
  - quota exceeded
- Peon-assisted capacity classification
- confidence/source fields for capacity status

#### Recommendation Engine

- optional brain-informed enrichment and background discovery, with independent
  Taskmaster model selection, bounded read-only context, explicit context
  settings, and independently updated reference knowledge as defined in
  [Taskmaster knowledge](taskmaster-knowledge.md)

- rule-based recommendation engine
- uses:
  - task description
  - selected workspace
  - harness configuration
  - capacity status
  - cost tier
  - current active sessions
  - session summaries
  - Git context
- recommends a harness/model for the next task
- explains recommendation in plain language
- may recommend workflow context, such as:
  - use cheap model
  - save premium model for review
  - wait until capped model resets
  - consider a worktree for parallel implementation
  - avoid starting another agent in a dirty shared workspace

### Should Have

- workspaces
- Git worktree detection
- list known worktrees for a repo
- configurable output patterns
- configurable waiting/input patterns
- basic event timeline
- session archive
- Peon provider configuration
- local model option where available
- manual override for Peon-derived status
- OpenCode-specific adapter
- Codex-specific adapter
- Claude Code-specific adapter

### Could Have Later

- create worktree from OrkWorks
- archive worktree from OrkWorks
- cleanup worktree from OrkWorks
- branch management
- auto-suggest terminal input gated by user approval
- notifications
- cost history
- token usage parsing
- provider API integration
- PR/diff integration
- multi-machine daemon
- VS Code extension client
- mobile dashboard
- shared team dashboard
- cloud sync

## Suggested Monorepo Structure

```text
orkworks/
├─ apps/
│  └─ desktop/
├─ crates/
│  └─ orkworksd/
├─ docs/
└─ examples/
```

The backend sidecar should be named `orkworksd`.

## Updated Milestones

### Milestone 1 — Shell and Backend

Goal: Electron can launch the Rust backend.

Deliverables:

- Electron app window
- Rust backend process
- dynamic localhost port
- health endpoint
- frontend can call backend
- logs visible in dev mode

### Milestone 2 — Embedded Terminal

Goal: Run one terminal session inside the app.

Deliverables:

- create PTY
- stream output to xterm.js
- send keyboard input to PTY
- resize terminal
- kill process

### Milestone 3 — Multiple Sessions

Goal: Run and switch between multiple terminals.

Deliverables:

- session registry
- session list
- switch active terminal
- preserve scrollback
- kill/archive sessions

### Milestone 4 — Session Metadata Protocol

Goal: Show meaningful session metadata.

Deliverables:

- create `sessions/<id>.json` under the global metadata root
- watch file changes
- reflect status in UI
- append basic event log
- show Needs You / Working / Done groups

### Milestone 5 — Git Context Detection

Goal: Show where each session is running and whether it is isolated.

Deliverables:

- detect Git repo root
- detect branch
- detect dirty/clean state
- detect changed file count
- detect worktree context where practical
- show Git context in session UI
- warn about multiple active sessions in the same dirty workspace
- use Git context in recommendations

### Milestone 6 — Peon

Goal: Use Peon to keep session metadata useful with low-cost observer-only inference.

Deliverables:

- collect recent output per session
- call cheap model with compact context
- require strict JSON response
- validate response against schema
- update session JSON with inferred metadata
- append Peon notes to event log
- show confidence/source in UI
- no automatic terminal input

### Milestone 7 — Harness Configuration

Goal: Start AI harnesses as configured terminal commands.

Deliverables:

- configurable harness commands
- start session dialog
- harness/model labels
- generic terminal adapter
- initial prompt/instruction support

### Milestone 8 — Capacity Panel

Goal: Track basic harness/model availability.

Deliverables:

- capacity files
- manual capacity override
- capped/healthy/unknown badges
- output pattern detection for caps
- Peon-assisted capacity classification
- confidence/source display

### Milestone 9 — Recommendation Engine

Goal: Recommend which harness/model to use for the next task.

Deliverables:

- simple task classifier
- rule-based scoring
- capacity-aware recommendation
- cost-aware recommendation
- Git-context-aware recommendation
- recommendation UI
- start recommended session

## Explicit MVP Non-Goals

The MVP is not:

- a new AI coding harness
- an IDE/editor replacement
- a full multi-agent planner
- a repo workflow manager
- a Git worktree manager
- an automatic merge system
- a cloud sync service
- a universal billing tracker
- a replacement for OpenCode, Claude Code, Codex CLI, Antigravity CLI, or Aider

The separately gated ordinary-child orchestration extension is limited to
UI-created runs with explicit exact-plan approvals, ordered batches, declared
dependencies and bounded live-child capacity. It does not make the MVP a
general-purpose multi-agent planner or Git worktree manager. Its proposed
scope, detailed contracts, capability evidence and scoped implementation plans
must pass their review gates before runtime work.

Gemini CLI is retired for new sessions because its individual Code Assist tier
is no longer supported. Its legacy `gemini` harness definition remains readable
only to preserve existing session history and settings; OrkWorks does not launch
or select it for new sessions. Antigravity CLI is the supported replacement and
is launched as `agy`; it currently has no compiled integration, signal, voice,
capacity, or model-selection capability beyond its documented launch and resume
commands.
