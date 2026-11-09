---
type: Qualification Packet
title: Prepared parent native continuation canary
description: Stdio MCP canary inputs, four unverified attempts, including the diagnostic packet that identified the session-layout guard.
tags: [orchestration, copilot, continuation, qualification]
status: draft
workflow_status: startup-layout-corrected-not-authorized
---

# Prepared parent native continuation canary

[#1573](https://github.com/Rambolarsen/orkworks/issues/1573) prepares the next
native research step under [#742](https://github.com/Rambolarsen/orkworks/issues/742).
The [plan](../superpowers/plans/2026-11-08-parent-native-packet.md) reuses the
[offline peer](parent-continuation-fixture.md) without changing it. Preparation
uses only Python fake clients and the fixture stdio server. No Copilot process,
discovery command, SDK/ACP server, model or credential discovery has run for this
packet during preparation. Four separately owner-approved batches later stopped
[unverified](parent-native-canary.md) on 2026-11-09, each with raw-root deletion confirmed.
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

The focused suite now has 35 tests, including real newline stdio exchange with an
owned Python fake client, a second fresh fake process, nonce/tool/turn joins,
retained state, source drift, missing approval, identity/resume denials, private
payload decode sentinels, observed caps and owned process cleanup. POSIX process
tests are explicitly skipped on Windows; the live packet targets macOS arm64.
The docs test runner invokes this suite with a 30-second timeout. LF attributes
preserve pinned bytes under autocrlf. No native success is inferred from these
synthetic checks or from eventual model statements alone.

Historical attempted packet identities, from source db4d33412e31ffaaecc0b7462fd5fcd7e761f22b:

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

## Model-free stop reporting revision — 2026-11-09

[#1586](https://github.com/Rambolarsen/orkworks/issues/1586) fixes the reporting gap
exposed by the [stopped attempt](parent-native-canary.md). The old receipt and its
SHA-256 stay unchanged. Nothing in this revision identifies its failure cause.

The revised observer adds optional failure and cleanupFailure objects to its
version-1 summary. They contain only a fixed phase (0 before phases, 1 new, 2
resume), an allowlisted enclosing operation stage, a fixed failure category and
a numeric OS error when available. Categories are check-failed, invalid-json,
os-error, process-timeout, process-error and unexpected-error. Unknown stages
become unknown; exception messages, class names, paths, argv, IDs and process
output are never copied into these fields.

The process projection contains started, a signed 32-bit exitCode or null, and
terminationStepsCompleted. An exit observed before termination raises is retained.
Completion means the existing TERM/wait/KILL/wait steps returned; it is not a
proof of OS confinement or absence of every descendant. A subprocess exception
can supply its numeric return code without retaining its command or output.
The stage describes the enclosing operation: an error during a native prompt
can still concern host cleanup rather than model/provider behavior.

If raw-root cleanup raises, cleanupFailure describes that separate failure and
the original failure remains intact; outcome still becomes incomplete-cleanup.
Execution limits, tool/model/source inputs, approval checks, no-retry policy and
termination/deletion behavior are unchanged. Tests use only Python fake processes
and synthetic errors, including private sentinels and a serialized summary.

Historical reporting-revision manifest SHA-256: 787dc852dc9e64d5333112fc175f96cf7e992c1b86dbbfdc38fb4cda643bdfdd.
Proposal SHA-256 remains e84b24ab002afc8fe45ecca2a2be6976ad8d1a5fc1f53aa1e5663fe31ec4f297.
The initial approval did not match the revised manifest. A later exact owner
decision covered one revised batch; its [receipt](parent-native-canary.md#revised-packet-attempt--2026-11-09)
is also unverified, with an observed native-process exit code of 1 in phase 1.
That separate attempt is spent and there is **no ongoing live authorization**.
The reporting code change itself was model-free; its later native execution is
recorded separately and does not identify the initial attempt's cause.


## Offline UUID correction — 2026-11-09

The [source audit and correction](parent-native-canary-offline-audit.md) found
that the old fake-peer ID is rejected by the pinned CLI's new-session UUID check.
The current peer/packet/argv consistently use
00000000-0000-4000-8000-000000000001, with a pure early compatibility guard.
This does not identify either historical stop or qualify native behavior.

Current corrected proposal SHA-256: 39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
Current corrected manifest SHA-256: e2cb9c9c41d2b0137aad51bd25fa12f367426fe2dcc9bdb075fc7a5ec95471ec.
Model, tools, permissions, source/retention policy, limits and no-retry behavior
are unchanged. Historical receipts and their source commitments remain unchanged.
At correction time, both old approvals were spent and did not match these bytes;
a new exact owner decision was required. The subsequent decision and attempt are
recorded below.


## Corrected batch spent — 2026-11-09

A new exact owner decision authorized [#1613](https://github.com/Rambolarsen/orkworks/issues/1613)
using the corrected proposal/manifest above. The unchanged runner attempted two
discovery commands and one prompt, then stopped unverified in native-history with
observed process exit zero. Its [retained receipt and limits](parent-native-canary.md#uuid-corrected-packet-attempt--2026-11-09)
establish raw-root deletion, not a successful capability check or exact failure
cause. No resume or retry ran. That new approval is now spent; ongoing live budget
is zero and source/model/transport changes or another attempt need a new decision.


## Offline history-check reporting — 2026-11-09

The [offline history audit](native-history-source-audit.md) found no proven static
path/event mismatch and did not identify the historical stop. It adds optional,
allowlisted checkCode only for native-history failures so a later approved batch
can distinguish the failed check group. All guards, parser rules, model, tools,
limits, supervision and retention behavior are unchanged. Thirty fake-only packet
tests pass; no Copilot command or vendor module ran for this correction.

Current reporting manifest SHA-256: 809bcb6f61df6803e6fe4d080072d45b1d739a1d8a69ec4e41d9ecc3d2e0d7a2.
Proposal remains 39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
At reporting-correction time, all three earlier batch approvals were spent and
the new manifest required a new exact decision. The subsequent diagnostic
decision and spent attempt are recorded below.


## Diagnostic batch spent — 2026-11-09

A new exact owner decision authorized [#1626](https://github.com/Rambolarsen/orkworks/issues/1626)
using the reporting manifest above. The unchanged packet attempted two discoveries
and one prompt, then stopped at native-history/history-session-layout with observed
exit zero. The [diagnostic receipt](parent-native-canary.md#history-diagnostic-packet-attempt--2026-11-09)
identifies the exact-session entry/symlink check but not its actual inputs or which
scan failed. Raw roots and disposable controls were removed. No resume/retry ran.
All four batch approvals are now spent; ongoing live budget is zero. The exact
packet remains unqualified for the three native capabilities.


## Offline startup/layout correction — 2026-11-09

The [startup correction](native-history-startup.md) reproduces premature rejection
of a synthetic empty root between root creation and exact-session creation. It
waits only during pristine phase1 nonfinal scanning, while retaining final/resume,
post-history, wrong/extra-entry and symlink rejection. This is not proof that the
historical native stop was a race. Layout failures may retain only four fixed
boolean/null flags; no names or paths. All35 fake-only packet tests pass.

Current manifest SHA-256: e89cd3cf0ba4c9b91f7a752edecd6a59f815d02d087503a4ac577959f90f7c9a.
Proposal remains 39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
The earlier reporting manifest above is historical. No native execution ran for
this correction; all four prior approvals remain spent, with ongoing budget zero.
