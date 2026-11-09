---
type: Fixture Evidence
title: Model-free parent continuation oracles
description: Direct-call synthetic peer and scripted client for event, gap, revocation and resume oracles; no native capability qualification.
tags: [orchestration, continuation, resume, fixtures]
status: stable
workflow_status: offline-fixture-implemented
---

# Model-free parent continuation oracles

[#1564](https://github.com/Rambolarsen/orkworks/issues/1564) implements the offline
unit in the [three-check handoff](../superpowers/plans/2026-11-08-parent-continuation-capability-handoff.md).
The owner authorized this unit with “keep going” after #1563 merged. It uses Python
stdlib direct calls and synthetic state. No listener, SDK/MCP configuration,
coding harness, native session, model request or credential discovery is involved.
The existing [#742 lifecycle](../superpowers/specs/2026-10-04-orchestrator-preparation-design.md#logical-lifecycle-acceptance--2026-11-08)
and its defaults remain unchanged; #740 research stays closed.

## Packet and expected behavior

- [Peer](https://github.com/Rambolarsen/orkworks/blob/main/crates/orkworksd/tests/fixtures/parent-continuation/peer.py) owns the fake host's virtual clock, event ring, pending requests, snapshots, acknowledgements and resume state.
- [Tests and scripted client](https://github.com/Rambolarsen/orkworks/blob/main/crates/orkworksd/tests/fixtures/parent-continuation/test_peer.py) exercise direct calls without sleeps, sockets or a native adapter.
- [Pinned evidence](https://github.com/Rambolarsen/orkworks/blob/main/crates/orkworksd/tests/fixtures/parent-continuation/evidence.json) contains literal configuration, its canonical SHA-256, both source SHA-256 values and a seven-step expected transcript. It labels all native capabilities unverified.

The receipt oracle was authored independently from the scripted client's output:
one report gives cursor 1/version 2; 257 further capacity hints give cursor
258/version 259; explicit fake host approval and one reservation give cursor
260/version 261. Duplicate delivery adds no effect. Resume changes the synthetic
runtime generation, retains task-a's reservation and has no grant. Completion
returns the already-pending request's `run_closed` receipt.

Tests compare the complete transcript to those literals. Hash checks do not
regenerate expectations. A source-only change updates reviewed code hashes;
changing the expected receipt oracle requires an explicit fixture-contract review.
The pinned source is test-only and not a production serialization or credential
API. Fake bindings, read snapshots and request handles are objects issued by
one owning Peer; equal serialized values from another peer are not authority.

| Oracle | Observed fixture-owned result |
| --- | --- |
| Pending and timeout | Wait remains pending through virtual second 29; deadline 30 returns no-change. A later event does not rewrite that receipt. Zero-time waits return immediately; negative, noninteger and over-30-second durations are rejected. |
| Notifications and reads | Report/approval/blocker/capacity hints require a current issued snapshot before a new acknowledgement effect. An approval hint alone grants nothing. Duplicate same-request acknowledgement returns its original receipt without another effect; stale, forged or conflicting requests fail. |
| Ring and cursor | All 256 retained notifications are readable. The next overflow makes an old cursor yield `refresh_required`; the client reads a snapshot and adopts its cursor without acknowledging omitted events. Native-string, cross-fixture, boolean, negative and future cursors are rejected. |
| Revocation and closure | Already-pending host results resolve as revoked or run-closed. Old binding cannot start a wait/read/effect; delivering that closure receipt is not a new authorized run read. |
| Exact fake resume | Wrong/missing/latest identity, workspace or configuration never creates a replacement. Fresh binding changes generation/bearer and preserves blockers, clarification and the stage-continue gate in priority order. No grant returns automatically; unreserved work requires explicit fake host approval, reserved tasks cannot repeat and terminal runs cannot resume. |

## Verification and interpretation

Run from the repository root:

```bash
python3 -I -B -m unittest discover -s crates/orkworksd/tests/fixtures/parent-continuation -p test_peer.py -v
```

The docs test runner invokes that same isolated interpreter with a ten-second
process timeout, following existing Python-backed component tests. `docs:check`
and docs CI therefore run the Python suite; missing Python or a failed test fails
the check. Python 3.9 compatibility uses deferred annotations. Exact source/evidence bytes
are pinned to LF through .gitattributes, including Windows autocrlf checkouts. The peer itself
never starts a process; only the ordinary test runner invokes Python.

TDD evidence: the initial pending-wait test failed against a stub that returned
no-change immediately. A later cross-peer binding regression also failed before
object-identity ownership fixed it. The final focused suite has 18 passing tests,
including pinned source/configuration and independent transcript checks. An
initial Python 3.9 annotation import error was corrected before the green runs.

These results establish the fixture's oracles only. They do not prove native
model delivery/wakeup, event transport, native identity resume, actual OrkWorks
authority/approval or durable transaction behavior. The fake host approval and
reservation methods are synthetic setup actions; they are not real UI grants.
Full native source/tool/graph-skill/configuration mapping, transport correlation,
retention and a finite reviewed live budget remain with the later native packet.
All three native checks remain **unverified**, with no new live budget or retry.


## UUID compatibility correction — 2026-11-09

The [offline native audit](parent-native-canary-offline-audit.md) found that the
original fake-only identity could not be used as an ordinary native UUID. The
current fixture uses 00000000-0000-4000-8000-000000000001 consistently with the
native packet. Only binding identity/configuration-digest references and reviewed
source pins changed; numerical/state/event/effect oracles are unchanged. This is
a fixture configuration re-key, not a production identity/storage migration or
native capability result. Historical packet/receipt commitments remain intact.
