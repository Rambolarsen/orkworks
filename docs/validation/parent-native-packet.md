---
type: Qualification Packet
title: Prepared parent native continuation canary
description: One stdio MCP packet with exact proposed inputs and synthetic process tests, followed by one approved but unverified native attempt.
tags: [orchestration, copilot, continuation, qualification]
status: draft
workflow_status: attempted-unverified
---

# Prepared parent native continuation canary

[#1573](https://github.com/Rambolarsen/orkworks/issues/1573) prepares the next
native research step under [#742](https://github.com/Rambolarsen/orkworks/issues/742).
The [plan](../superpowers/plans/2026-11-08-parent-native-packet.md) reuses the
[offline peer](parent-continuation-fixture.md) without changing it. Preparation
uses only Python fake clients and the fixture stdio server. No Copilot process,
discovery command, SDK/ACP server, model or credential discovery has run for this
packet during preparation. One later owner-approved native batch stopped
[unverified](parent-native-canary.md) on 2026-11-09, with raw-root deletion confirmed.
#740 stays closed; all three native checks remain **unverified**.

## Exact packet

Sources live in
[the test-only packet directory](https://github.com/Rambolarsen/orkworks/tree/main/crates/orkworksd/tests/fixtures/parent-native-packet).

| Artifact | Meaning |
| --- | --- |
| proposal.json | Exact requested model/native identities, argv, settings, tools, prompts, startup/graph-skill content, environment and proposed budget/risks. |
| manifest.json | Closed SHA-256 set for proposal, observer, MCP server, selected static schemas and unchanged peer. It contains no approval. |
| source-contracts.json | Selected cached 1.0.90 schema definitions for stdio MCP configuration and native start/resume/tool metadata; unresolved references remain, and this is not runtime evidence. |
| mcp_server.py | Three fixture-only tools, unpredictable result nonces, current snapshot handles and fixed duplicate/gap/resume oracles. No network or production authority. |
| observer.py | Source/configuration guards, lexical metadata projection, native/helper joins, finite owned process supervisor and separately gated live entry point. |
| test_packet.py | Synthetic clients and denial tests; no coding harness launches. |

The one candidate route is standard stdio MCP 2025-06-18. Missing native negotiation,
raw-name tool mapping or correlation ends the batch without alternate names,
transport, model or retries. Static help/schema supports a candidate only.
Inline graph-skill delivery is supplied content; it does not qualify full #741
role/source delivery or grant tools.

The fixture exercises report, approval, blocker and capacity hints, duplicate
acknowledgement without another effect, overflow refresh/cursor adoption and
exact native resume. Its second server reconstructs the fixed retained fake state;
this does not demonstrate production persistence. Fresh host generation/PID and
new native session.resume must join the original exact-ID session.start/history
with an unchanged prefix. ResumeData itself contains no native session ID.

## Proposed live budget and remaining decision

The owner approved the exact proposal/manifest for one batch on 2026-11-09.
Its [receipt](parent-native-canary.md) is unverified; that one attempt is spent.
The current authorized live budget is **zero**. Another attempt needs a new exact
owner decision. The guard binds the SHA-256 of the proposal and manifest,
decision approve-one-batch and maxNativePrompts 2; missing, changed or extra fields
stop before any subprocess. This procedural research guard is not a product
credential boundary. Historical #740 decisions grant no new attempt.

The proposal allows two sequential native prompt invocations, each at most 240
seconds, four source-discovery commands at most 30 seconds each (two per phase),
600 seconds total, 24 observed tool calls and 32 observed turn/usage events. The
successful synthetic sequence uses 23 tool calls. These are **not two billable
model calls**: provider iterations, hidden requests/retries and billing remain
unknown. There is no retry, provider fallback or transport fallback.

Native Copilot uses existing HOME for normal authentication; the observer never
reads/copies credentials. Private disposable native history may contain assistant,
reasoning and synthetic tool data. The observer lexically skips those values
before JSON decoding and retains only bounded counts, booleans and digests.
Capture is requested off; effective managed capture and remote/provider retention
remain unknown. Owned-root deletion is not remote deletion or OS containment.

The fixture helper starts with env -i and explicit owned HOME/TMPDIR, PATH and
LC_ALL. macOS injects __CF_USER_TEXT_ENCODING during Python startup, so the packet
sets it to synthetic 0x0:0:0 explicitly; no inherited value is forwarded. Unknown
helper environment names fail. One owned process group is stopped on failure or
deadline; raw roots are removed by a cleanup process limited to ten seconds.
Cleanup failure makes the batch incomplete. Raw data caps are 1 MiB per file and
8 MiB total; retained summary cap is 256 KiB.

## Model-free verification

```bash
python3 -I -B -m unittest discover -s crates/orkworksd/tests/fixtures/parent-native-packet -p test_packet.py -v
```

The focused suite has 17 tests, including real newline stdio exchange with an
owned Python fake client, a second fresh fake process, nonce/tool/turn joins,
retained state, source drift, missing approval, identity/resume denials, private
payload decode sentinels, observed caps and owned process cleanup. POSIX process
tests are explicitly skipped on Windows; the live packet targets macOS arm64.
The docs test runner invokes this suite with a 30-second timeout. LF attributes
preserve pinned bytes under autocrlf. No native success is inferred from these
synthetic checks or from eventual model statements alone.

Packet identities for the later owner decision:

- Proposal SHA-256: e84b24ab002afc8fe45ecca2a2be6976ad8d1a5fc1f53aa1e5663fe31ec4f297.
- Manifest SHA-256: 1d850a7a88a8bc098e4726c66eab1d95746331df7668c0cc3a8b812e0a2da3e8.

The code review caught missing-history resume counts, expired/deferred process
deadlines and failed-discovery accounting. Each received a failing regression
before its fix. Exact count matching uses the pinned ResumeData definition.

Local verification also completed formatting/builds, all six Rust integration
targets, 31 reporter tests, desktop type-check/native ABI check and 1,709 desktop
tests (three skipped), 1,204 docs tests/build/links and whitespace/doc/worktree
checks. The full verifier's Rust unit run had 3,282 passes, 18 ignored and one
failure in the unchanged oversized Codex help fixture: expected CodexHelpOversized,
observed CodexHelpFailed. That exact synthetic test passed in isolation; the
underlying head command produced the expected 65,537 bytes. The transient cause
remains unverified. Remaining verifier steps passed separately; the initial
nonzero result is retained rather than described as an entirely green run.
Existing Rust unused-code and bundle/circular-chunk warnings concern unchanged
surfaces and do not require packet edits. Passing current-head CI is required
before merge. Fresh medium code review approved the corrected code and final
POSIX test guard; neither the preparation writer nor reviewer ran a Copilot command.
