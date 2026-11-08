---
type: Qualification Preparation Plan
title: Prepare one stdio native continuation packet
description: Model-free preparation of a two-phase synthetic Copilot canary with exact tools, observation, limits and an unapproved live budget.
tags: [orchestration, copilot, continuation, qualification]
status: draft
workflow_status: prepared-not-authorized
---

# Prepare one stdio native continuation packet

Outcome: one executable, independently reviewable canary packet for #1573 under
[#742](https://github.com/Rambolarsen/orkworks/issues/742). Preparation may run only
its own fake clients/processes. **No Copilot process, discovery command or model
request runs before a new exact-packet owner approval.** #740 stays closed.

Reuse the accepted [three checks](2026-11-08-parent-continuation-capability-handoff.md)
and [offline oracles](../../validation/parent-continuation-fixture.md). No production
module, authority, profile eligibility, storage or installed integration changes.

## One route and two phases

Use stdio MCP only, with exactly wait_for_run_event, read_run_state and acknowledge.
No SDK/ACP, network listener, arbitrary file/command/URL tool, native delegation,
server instructions or automatic fallback. Pin MCP 2025-06-18 initialization and
newline JSON-RPC to the official [transport](https://modelcontextprotocol.io/specification/2025-06-18/basic/transports),
[lifecycle](https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle)
and [tools](https://modelcontextprotocol.io/specification/2025-06-18/server/tools) contracts.
These specify the fixture transport, not observed Copilot compatibility.

1. One new native prompt process waits for four separate synthetic report,
   approval, blocker and capacity hints, then a duplicate hint, a ring-overflow
   refresh and an event after the adopted snapshot cursor. Server-produced random
   result nonces are not present in the prompt; the next read must echo the nonce.
   Snapshot IDs are server-owned handles; acknowledgements are fake effects only.
2. After clean phase-one exit and complete correlated receipts, explicitly resume
   the same owned UUID with a fresh host runtime generation. Require a newly
   appended native session.resume after the original session.start, unchanged
   history prefix, and no new session.start/replacement ID. Read the fixed fake
   retained reservation/blocker/clarification/stage-continue state with no grant.

Both phases use unchanged startup/settings/MCP config bytes in one fresh owned
repo/COPILOT_HOME. Only exact resume argv, assignment prompt and host runtime
context change; no implicit latest/create/attach/fork fallback. The mock retained
state is not a production persistence or approval implementation.

## Owned files and boundaries

One inline writer owns
`crates/orkworksd/tests/fixtures/parent-native-packet/`:

| File | Responsibility |
| --- | --- |
| mcp_server.py | Bounded fixture-only stdio codec, three tools, nonce/read-handle validation and pinned fake-peer sequencing; imports unchanged #1564 peer. |
| observer.py | Pure packet preparation/projection/validation plus separately gated native supervisor; no native launch without an explicit exact-digest approval file. |
| test_packet.py | Failing-first protocol, source/configuration pin, fake-process, projection, timeout/cleanup and correlation-denial tests. |
| proposal.json | Exact template argv/env/settings/tools/startup/graph-skill/prompts, source identities, proposed ceilings and owner risk statement. |
| manifest.json | Reviewed source/data hashes; never self-approves or regenerates expected outcomes. |

Add only the existing-pattern docs test runner/script entry, indexed evidence
note, LF attributes and links from the accepted handoff. No product code changes.

## Native proposal to prepare, not execute

Pin Copilot 1.0.90/macOS arm64, existing retained executable/cache hashes and
requested gpt-6-luna through normal GitHub authentication. Rehash the actual files
before any later launch; no update/download/provider substitution. Retain the
selected MCP stdio and native session/tool/resume schema shapes from that pinned
source. The known CLI help supports extra MCP JSON, explicit UUID/resume and
server(tool) permission patterns. Raw three-tool availability filtering remains
a version-specific lead: missing mapping ends the batch, not a naming retry.

Request only those three fixture tools, allow only their ork742 permission
patterns, deny shell/write/URL, disable builtin MCPs, hooks/memory/IDE/remote/export
and all content capture. Extra MCP config points only to the owned server. Its
command uses env -i with explicit PATH, owned HOME/TMPDIR, LC_ALL and isolated
Python plus a synthetic __CF_USER_TEXT_ENCODING value; helper receipt checks names without retaining unknown values. Native
HOME is inherited only for normal existing authentication, with the same bounded
instruction/skill-source discovery guards used by the closed #740 canary. No
observer reads/copies credentials or enumerates personal/managed contents.

Include exact coordination-only root instructions and content-bound committed
graph-planning skill bytes in synthetic AGENTS.md. No supplied skill gains tools.
This functional canary does not establish full role/source delivery qualification.

Proposed live ceilings, requiring explicit owner acceptance after packet hashing:

- Two sequential native prompt invocations, at most 240 seconds each and 600 seconds
  total including four source-discovery commands of at most 30 seconds each (two per phase).
- At most 24 fixture tool calls and 32 observed assistant-turn/usage events total;
  one outstanding wait, maximum 30 seconds per wait. One attempt, no retry/fallback.
- One native process and one fixture MCP child at a time, plus observer; helpers
  have their own deadline/EOF exit. No native subagents or replacement harness.
- At most 1 MiB per raw file, 8 MiB raw total and 256 KiB sanitized retained evidence.
  Stop on cap/unknown identity/access/confirmation/missing correlation.

These are observable/host-enforced stops, not certified provider-request, hidden
retry, billing or OS-containment guarantees. Actual provider request count remains
unknown. The approval must acknowledge normal native credential resolution,
private disposable history possibly containing assistant/reasoning and synthetic
tool data, requested capture-off versus unknown managed/remote/provider retention,
and owned-root cleanup without any claim of remote deletion.

## Observable evidence and cleanup

Independently bind owned native PID/process group and fresh host generation,
MCP connection/request/result nonce, native tool call/turn metadata and original
session identity. Parse only allowlisted native metadata and selected synthetic
argument fields; lexically skip assistant/reasoning/tool-output values before
JSON decoding. No broad OTel exporter or payload capture is needed. Native MCP
metadata must name ork742 and one of the three declared tools; unexpected calls
end the canary. Server call/result receipts alone do not prove model delivery.

For resume, require the original exact-ID history and a new session.resume,
not a requested ID or existing filename alone. A new start, changed prefix,
active/continue-pending resume, unexpected process/settings/source or another
session is an unverified stop. No code emits a success classification from model
statements alone. Fake fencing/retention results remain distinct from native
observations; all native status is unverified until actual correlated evidence.

Kill only the owned native process group on failure/deadline; fixture children
also exit on EOF/deadline. Delete all owned raw roots in finally, retain bounded
metadata/correlation booleans/digests only, and make cleanup failure an incomplete
batch. Approval is a procedural research guard, not a production credential.

## Implementation and verification

- [x] Pin proposal inputs/source selections; test missing/wrong approval before
  any subprocess, and constructor/template drift before native launch.
- [x] Implement only the fixture MCP protocol and synthetic three-check sequence,
  with invalid/duplicate/oversized/unsupported requests and nonce/handle denial.
- [x] Implement metadata-only projection and receipt join; sentinel tests prove
  assistant/reasoning/output values are skipped rather than decoded/discarded.
- [x] Test supervisor with an owned fake native process only: wrong/new ID,
  missing resume, changed prefix, extra tools, time/output/usage caps and cleanup.
- [x] Run focused tests, simulated autocrlf/source hashes and current-code fresh
  review. Run full repository verification and retain exception/recovery evidence;
  freeze proposal/manifest digests. Require passing CI before merge and one exact
  owner budget/risk decision before native execution; do not execute it.

## Investigation and review

The main uncertainty is actual native MCP naming/availability and wake/resume:
static source/help proves a candidate, not observed support. A native result nonce
followed by a correlated tool read gives stronger evidence than a model claim or
native log read. ResumeData has no session ID, so it must join the original owned
StartData/history instead of being treated as independent identity proof.

Local plan review: **Ready for model-free packet preparation; Investigate/not
authorized for native execution.** Reuses the approved lifecycle/guardrails. The
new code is a disposable qualification utility, not a product authority boundary.
Protocol references, mock-only tests, source pinning and one later exact-packet
approval bound the preparation; no open issue is converted into support by hashing.

| Dimension | Rating | Reason |
| --- | --- | --- |
| Dependencies | 2 | Existing offline peer, pinned source and standard stdio protocol. |
| Blast radius | 2 | New test utilities, no production caller/installed config. |
| State changes | 1 | Preparation writes only owned synthetic roots. |
| Reversibility | 1 | Remove/revert owned fixture packet. |
| Uncertainty | 4 | Native MCP/event/resume behavior is not yet observed. |

Total: 10/25 for preparation only. Future native execution has its own explicitly
reviewed risks/budget above and remains at zero authorized calls.
