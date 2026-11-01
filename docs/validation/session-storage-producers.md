---
type: Validation Record
title: Ordinary session metadata producer compatibility
description: Source preparation and explicitly scoped qualification evidence for ordinary API reporting before SQLite cutover.
tags: [orkworks, sessions, metadata, producers, qualification]
status: stable
---

# Ordinary session metadata producer compatibility

Tracking: [#1242](https://github.com/Rambolarsen/orkworks/issues/1242).
Contract: [session storage](../../specs/session-storage.md),
[accepted design](../superpowers/specs/2026-11-01-sqlite-session-storage-design.md).
Initial source base: `ae8e246d`. Evidence date: 2026-11-01.

## Scope and production status

The source helper supports the independent ordinary reporting marker
`ORKWORKS_SESSION_STORAGE_PROTOCOL=sidecar-v1`. It uses the existing authenticated
revisioned metadata API and GET → validation-only PATCH → GET bootstrap. The
marker is a required write-protocol declaration, not a credential, root-process
proof, database-format proof or native attention eligibility.

**No production launch emits this marker yet.** Format-2 storage/activation is
not implemented. #1244 must derive ordinary reporting policy from trusted store
state and qualified installed producers, after the backup/quiescence gates.
This source preparation never reuses #789's gated native capability to enable
ordinary reporting. Existing launches and JSON authority stay unchanged.

## Marker and failure contract

- With only ordinary `sidecar-v1`, the helper enables API-only reporting without
  requiring `ORKWORKS_SESSION_METADATA_API_VERSION=1`.
- With only the existing native marker `1`, its current API behavior remains.
- For general API commands/instruction selection, a present unknown/empty
  ordinary value fails before contacting the sidecar,
  even if the native marker is valid. A valid ordinary value with an unknown
  native marker also fails. Both valid values use the ordinary envelope budget.
- No marker retains the existing narrow worktree command/instructions; it does
  not permit general agent metadata writes. Legacy unknown native instruction
  selection keeps its existing behavior, while general API commands reject it.
- Parent environment forwarding strips the ordinary marker case-insensitively,
  as it already strips native metadata eligibility. CLI subagents may inherit a
  parent's report capability; this filter applies to newly managed OrkWorks
  runtimes and does not attest native CLI child identity or qualify clearing.
- Ordinary metadata and the narrow worktree command allow a 2 MiB response so
  the accepted 1 MiB canonical record fits with its revision/envelope. Legacy
  native Client commands retain their 1 MiB response bound. Patch requests remain
  capped at 8 KiB; complete scopes, revisions and server-side arbitration remain.
- General metadata commands still ignore ambient proxies. A caller may explicitly
  select an already owner-authorized managed relay using `--http-proxy`; origin
  pinning, redirect rejection and no direct/file fallback remain unchanged.
  Automatic Codex managed-relay selection remains worktree-command-only.

Stale revisions are returned as conflicts without replay. Bootstrap failure,
unavailable APIs, malformed replies or unsupported protocols do not write JSON
or bypass source/identity gates. Bootstrap success reports no native eligibility.

## Source-owned producer inventory

| Producer / owner | Existing behavior | Ordinary-storage status |
| --- | --- | --- |
| `session-metadata.py` invoked by the owning agent | Versioned, bearer-authenticated scoped API writes | Independent ordinary mode prepared and actual-context bootstrap measured below |
| Codex SessionStart POSIX reporter | Generates helper context from the installed command | Selects ordinary API instructions when the ordinary marker is present; generated install-command fixture exercises both marker modes |
| Codex SessionStart PowerShell reporter | Same installed reporter/helper ownership | Matching ordinary selection prepared; Windows agent/sandbox qualification is not established by macOS tests |
| POSIX/PowerShell harness attention/native-ID reporters | Existing HTTP routes and private native report spools | HTTP producers retained; no session JSON replacement added; native/root-source gates unchanged |
| OpenCode reporter plugin | HTTP attention/native-ID reporting | Retained unchanged; does not establish general agent metadata instruction/handshake qualification |
| Other/custom/generic tools | Existing hookless observation or tool-specific reporting | No automatic ordinary marker and no blanket generic-agent qualification |
| Repo skills/generated agent instructions | MVP permits legacy file writes without a required mediated protocol | Canonical contract now names the ordinary marker; do not hand-edit generated APM assets; new required sessions receive API-only helper context where qualified |

The current helper contains no direct metadata-file read/write fallback. Event
NDJSON remains a separately owned legacy append surface; this work does not move
events into SQL, alter checkpoint semantics or qualify concurrent backup writers.

## Qualification matrix

| Profile / artifact | Evidence | Verdict |
| --- | --- | --- |
| Source Python helper, owning Codex tool shell, macOS arm64, explicit established managed relay | Read-only live bootstrap with ordinary marker and native marker absent; all three requests accepted | Qualified for this exact helper invocation/route; not database activation or a whole installed harness claim |
| Source Python transport against controlled local/relay endpoints | Bootstrap/patch, stale revision, unsupported versions, unavailable API, bounded replies and no-fallback regressions | Automated behavioral coverage; not another sandbox/platform qualification |
| Source Python helper against the real Rust metadata router | Both marker modes bootstrap without mutation; existing ownership/source semantics still enforced on scoped patch | Automated integration evidence |
| Installer-generated POSIX Codex SessionStart command with current source assets | Temporary integration install invokes the produced command, delivers both native and ordinary API context; helper drift/repair fixtures retained | Installed-command fixture coverage; not reconciliation of the user's stable live assets |
| User's current stable helper | Bytes differ from new source; ordinary bootstrap rejects with exit 2 | Unqualified for ordinary mode until normal integration reconciliation installs matching assets |
| Windows PowerShell + actual coding-agent sandbox | Source selector prepared; no local Windows/actual-agent evidence | Unqualified; require corresponding installed/profile evidence before support |
| Claude/OpenCode/other agent instruction delivery for general metadata | Existing HTTP hook reporting alone does not exercise new agent bootstrap/context | Unqualified for general ordinary agent writes until delivery and actual-context evidence exists |

Version/platform/tool rows must be extended with measured evidence rather than
assuming parity. Missing/old helpers, unknown tool contexts or failed bootstrap
keep the affected profile ineligible for format-2 launch/cutover. Reconciliation
uses the normal integration owner/confirmation flow; this work does not silently
replace the user's installed helpers or relaunch coding tools.

## Verification and remaining gate

The final source-helper read-only probe completed at 2026-11-01T06:32:50Z in
Codex CLI 0.166.0's owning tool shell on macOS arm64. Helper SHA-256:
`5a8a588e60831a6ddd64bc1f60612aa67cb47659b9a83f088731b20c14eb1755`.
The ordinary marker was selected, the native marker removed, and the established
managed relay explicitly passed to the helper. Exit status was zero and the
bootstrap response reported `validated: true`. No metadata mutation was requested.
The local diagnostic contains only nonsecret hashes/outcomes; credentials,
proxy values and complete session records are not part of this evidence.

The then-installed stable helper SHA-256 was
`0a599712c26280d67e47b6c05e67d5959df8673b267bf941396bc210ff59e194`;
its ordinary-marker bootstrap exited 2. This is a negative compatibility result,
not an installation/reconciliation claim. Recheck installed bytes before launch
eligibility; a later installed change is not automatically covered by this probe.


The new ordinary bootstrap, instruction delivery, unknown-version downgrade and
parent-marker regressions were observed failing before their fixes. Separate
ordinary/worktree maximum-record regressions exposed the envelope boundary and
passed after correcting the response budget while preserving the legacy cap.

Tests cover the existing source arbitration, user/native wait ownership, exact
identity, worktree scope, stale patches and reporter hooks. A bootstrap is a
reachability/protocol check, not proof that arbitrary future patches will win
source arbitration. Positive/negative endpoint fixtures never establish a new
production capability or sandbox identity.

The issue remains open until its required installed and supported agent-context
profiles are qualified. Source preparation can merge independently and #1243 can
develop its isolated store, but #1244 cannot activate format 2 based on this
partial matrix. Native startup/clearing and hierarchy gates remain unchanged.
