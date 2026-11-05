---
type: Design
title: Launch-scoped worktree instructions
description: Accepted run-local recipient selection for OpenCode delivery of the shared worktree helper instructions.
tags: [sessions, harnesses, git, worktrees, instructions]
status: stable
workflow_status: approved
---

# Launch-scoped worktree instructions

Owner-approved on 2026-11-05; source implementation prepared; native activation qualification pending. Tracking: [#1347](https://github.com/Rambolarsen/orkworks/issues/1347).
This narrows the [approved generic direction](2026-11-03-harness-workspace-signals-design.md)
using the [native evidence](../../validation/harness-workspace-signals.md).
The [implementation plan](../plans/2026-11-04-launch-scoped-worktree-instructions.md)
records the approved concrete protocol and remaining activation gate.

## Outcome and boundary

An eligible OpenCode owning root receives the same narrow worktree helper text
as other supported harnesses. Its child and peer roots receive none. The agent
still reports its chosen task directory from its actual tool context; nothing
infers a task worktree from native cwd, file edits or terminal text.

Reuse `session-metadata.py worktree-instructions`, the authenticated revisioned
worktree PATCH and the shared resolver under [ADR 0094](../../adr/0094-explicit-session-worktree-binding.md).
This design changes instruction recipient selection only. Native-ID reports,
attention authority, resume availability, launch cwd and durable metadata schemas
keep their existing contracts. The legacy reporter's child/peer capture gap stays
tracked separately and must never select the recipient of these instructions.
No #789/native clearing, completion or format-2 eligibility follows from this.

## Evidence that fixes the choice

The production reporter can capture a child-first event or rebind on a child's
`session.created` without looking up its parent. A private server alone does
not distinguish the intended root from every parentless peer inside it.
Exact-resume record existence likewise does not prove root parentage.

Separately owner-approved OpenCode 2.2.5 fixtures on macOS arm64 established:
exact-session parent/location reads; root/child independence; context-hook text
reaching the serialized root request but not its child; and normal interactive
CLI creation of a fresh caller-selected `--session` ID. The CLI probe used a new
isolated store, one fixture prompt and two derived records. Its actual context
ID equaled the candidate, read-back identity matched, parent was absent, location
was task B, marker serialized, and no request reached the local rejecting stub.
No real inference or successful model turn is claimed.

Therefore use independent launch intent plus the native read-back, not the
reporter's mutable captured ID or `ctx.location`. Native evidence is specific to
OpenCode 2.2.5/macOS arm64; initial enablement is restricted to that combination.

## Private per-run selector

Use one optional internal instruction intent carried by the live runtime:
`NewSession`, `ExactResume(id)` or `Unscoped`. The application sets it from the
requested launch/resume operation before startup. It is not inferred by parsing
arbitrary command strings, stored in SessionInfo or persisted as native identity.
Existing direct test/runtime callers default to Unscoped.

A closed adapter preparation step consumes that intent and the resolved harness,
executable identity, current installed helper/reporter assets and run generation.
It produces an optional `WorktreeInstructionScope { root_id, helper_path }` and
its matched argv adjustment. Core Git resolution does not switch on vendor names;
other harnesses retain their existing instruction channels without implementing
this optional selector.

For the initial qualified OpenCode builtin:

- Admit only the source-owned private-server command templates, exact stable
  CLI version 2.2.5 on macOS arm64, enabled integration, and current owned plugin
  plus stable helper bytes. Model selection may vary; custom/overridden command
  templates, wrappers and unqualified profiles are Unscoped.
- NewSession generates a fresh `ses_` plus 32 lowercase hex digits from OS
  randomness, appends literal `--session <id>` argv, and selects that same ID.
  Do not persist the candidate as an observed native ID. Randomness failure for
  an otherwise eligible launch aborts before spawn; never invent a weaker ID.
- ExactResume selects the exact ID already requested, preserving existing
  argv and read-only availability checks. Runtime API parent validation below
  determines instruction eligibility; this does not upgrade the existence-only
  resume check or claim that a historical captured ID is a root.
- Latest-folder/repository resume, unsupported versions/platforms and Unscoped
  intents retain current launch/resume behavior and emit no selector/context.
- Recheck runtime generation/cancellation and selected executable/profile after
  awaited preparation, before changing the admitted command or spawning it.
  A scope and its argv must never come from different runtime/profile snapshots.

Qualify assets by reading the installed regular files at the canonical owned
paths and comparing their full bytes with this binary's bundled reporter/helper
assets, using the existing ownership check as an additional prerequisite. Missing,
unreadable, foreign or unequal bytes yield Unscoped; a marker alone is insufficient.
After awaited preparation, repeat that byte comparison immediately before spawn,
with no await between this final check and spawn. Discard both scope and new-session
argv adjustment if this final qualification fails. The plugin also checks the
helper's bytes against its bundled expected helper digest immediately before each
helper execution, including after any awaited parent lookup; mismatch emits nothing.

These are cooperative compatibility checks, not an atomic filesystem or malicious
local replacement fence. The native loader may read a plugin after spawn; concurrent
replacement after the final check is outside the activation snapshot guarantee.
The current legacy plugin ignores the new protocol. Future incompatible reporter
or helper updates must change the protocol version and reject unknown versions;
normal reconciliation must not advertise mixed asset sets as qualified. Tests
exercise drift during awaited preparation and helper replacement after spawn.

Export only from the prepared runtime, after inherited environment filtering:
`ORKWORKS_WORKTREE_INSTRUCTION_PROTOCOL=1`,
`ORKWORKS_WORKTREE_INSTRUCTION_ROOT_ID=<id>`, and
`ORKWORKS_WORKTREE_INSTRUCTION_HELPER=<absolute installed helper path>`.
Strip all three inherited names case-insensitively even for ineligible launches.
Missing/unknown protocol, malformed ID, relative/control-character helper path,
or missing existing reporting capability makes delivery a no-op.

The selector is not a new credential or an authenticated native identity proof.
It is cooperative recipient guidance within the existing capability boundary;
processes with the session's environment retain the existing spoofing limitation.
The native ID remains opaque (at most 256 ASCII bytes with the native `ses_`
form); helper paths use the existing absolute-path/control-character bounds.

## Owned plugin delivery

The OpenCode plugin adds a `context` hook independent of its event-capture state.
On each primary context, require event.sessionID equal to the run's selector,
then call `ctx.session.get({sessionID})` and require an exactly equal read-back
ID with absent/null parent. Child, peer, unavailable/malformed data and mismatches
receive no worktree text. A candidate from an old run cannot be adopted by a
new run's environment. Nothing updates metadata or attention while checking.

Bound the read to two seconds and at most one unresolved lookup per plugin
instance; use native cancellation when supported, otherwise discard late results
and suppress further lookup starts until the outstanding call settles. Check
cleanup/cancellation before and after every await. An unresponsive read must not
stall model dispatch beyond the bound or create an unbounded queue.

For an eligible root, execute `python3 <helper> worktree-instructions` as fixed
argv, without a shell. Bound it to two seconds and 64 KiB; accept only its narrow
SessionStart additionalContext string, at most 16 KiB. Failures, unrelated/malformed
output or empty context emit nothing and never fall back to general metadata
instructions, copied instruction prose, direct files or host-side worktree writes.
The text can be cached only within that plugin instance/runtime environment.
Append it as one system text part; preserve the existing prompt, tools and model
settings. No new client HTTP or WebSocket interception belongs in production.

The helper generates text only in the host plugin. Actual reporting still runs
later in the agent's tool/sandbox context through the existing qualified transport.
A native TUI switch to a different root stays outside this run's instruction
selector. A new OrkWorks launch or exact resume establishes a new selector; do not
reinterpret a peer's creation or first model request as an authorized switch.

## Compatibility and recovery

Normal Settings reconciliation installs the changed owned reporter and stable
helper. Foreign files remain untouched; drift, missing assets or unsupported
versions prevent selector activation. Existing sessions have no new selector
in their environment and receive no retroactive instruction injection or restart.
The helper's current HTTP/proxy restrictions and all other harness channels stay
intact. New env names are not exposed through public session APIs or user-editable
harness definitions; do not add public supports flags for this first adapter.

Disable selector preparation to roll back. Current coding-tool operation and
explicit helper reports remain available. The chosen native ID is an ordinary
native conversation ID, so accepted IDs remain readable/resumable by prior
source. No new durable record or migration needs repair. Historical child/peer
captures are not rewritten or reopened. Mixed-version legacy identity writers
retain their existing limitations; this design promises no identity fence.

## Acceptance and rollout gates

Before runtime code, owner approval must be recorded and the authoritative
session Git contract, harness concept and ADR 0094 amendment must describe this
instruction-only extension. Existing source-priority/native gates remain unchanged.

Tests must distinguish root/child/peer, child-first and child-created event order,
misleading `ctx.location`, read-back mismatch/parent, delayed lookup/helper output,
cleanup, stale runtime generation, missing/unknown/spoofed ambient markers,
unsupported command/version/platform, asset drift and helper failure. Verify
new argv ID equals env selection, exact resume does not change argv, and latest
resume stays Unscoped. Assert no identity/attention posts arise from context
checks, and no existing metadata fields or Git binding are changed by delivery.

Full repository verification and current-code `/code-review low` are required.
Before initial activation, separately authorize one bounded zero-inference fixture
using the actual changed reporter/helper bytes and prepared launch selector.
Synthetic tests and this design do not substitute for that qualification. A failed
or unavailable native check leaves activation disabled; no automatic retry,
credential change or unsupported-platform rollout is authorized.
