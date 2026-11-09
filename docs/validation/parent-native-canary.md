---
type: Native Canary Evidence
title: Parent continuation canary stopped unverified
description: Two separately approved native batches stopped unverified; retained receipts distinguish the initial generic stop from the revised host-stage and exit observations.
tags: [orchestration, copilot, continuation, qualification]
status: stable
workflow_status: attempted-unverified
---

# Parent continuation canary stopped unverified

[#1577](https://github.com/Rambolarsen/orkworks/issues/1577) executes the one-batch
owner approval given after [#1575](https://github.com/Rambolarsen/orkworks/pull/1575).
The owner answered “go ahead” to the exact [packet](parent-native-packet.md), then
“lets keep going”. This is a new bounded decision, not a reused #740 budget.
The reviewed runner and packet inputs were used unchanged from commit
db4d33412e31ffaaecc0b7462fd5fcd7e761f22b. The attempt ran on 2026-11-09.

## Retained observation

The unchanged [sanitized summary](parent-native-canary-summary.json) reports:

| Field | Observed value |
| --- | --- |
| Outcome | unverified |
| Discovery command attempts | 2 |
| Native prompt invocation attempts | 1 |
| Elapsed seconds | 3.357 |
| Stop reason | Packet check failed; no retry or fallback |
| Raw roots removed | true |
| Provider request count | unknown |
| Remote retention | unknown |

The receipt contains no successful capability-check block. No second prompt or
resume phase was attempted. The prompt counter increments immediately before
calling the supervisor, so it does not alone prove a native turn, inference or
billable request. Given the runner's ordering, reaching that counter implies
both discovery calls and source guards returned; their raw output is not retained.

The generic failure reason does not identify whether the stop concerned native
startup, tool naming, permissions, metadata/correlation or another check. There
is no retained evidence supporting a particular cause. Do not infer that Copilot
lacks continuation/resume support, or that a model/provider rejected the request.
The observer's raw roots were deleted and were not inspected afterward. There
was no retry, alternate tool naming, transport/model fallback, or native diagnostic
chain. Unused phase-two allowance does not authorize another attempt.
A bounded read-only OS cleanup check subsequently found zero native processes
matching the canary log-root pattern; no process IDs or command-line contents
were retained. The default sandbox could not read that inventory; the authorized
network/process execution context returned the zero-match result.

## Exact authority and source provenance

The temporary approval object bound decision approve-one-batch and
maxNativePrompts 2 to these exact bytes:

- Proposal SHA-256: e84b24ab002afc8fe45ecca2a2be6976ad8d1a5fc1f53aa1e5663fe31ec4f297.
- Manifest SHA-256: 1d850a7a88a8bc098e4726c66eab1d95746331df7668c0cc3a8b812e0a2da3e8.
- Summary SHA-256: 3574f321430874b52df4858d528c29c4e9574bc43b5a811358cd6a855437b8eb.

The observer entry point used the pinned Python with -E -s -B, preserving its
ordinary sibling-module lookup while ignoring Python environment settings and
user site packages. The fixture MCP helper retained the approved -I -B flags;
native argv, environment, tools, settings and prompts were unchanged. No code or
manifest edit was made to obtain another attempt. The temporary approval/output
control root was removed after this sanitized receipt was copied.

The approved bounds were two sequential gpt-6-luna prompt invocations, up to four
source-discovery commands, 240 seconds per prompt, 600 seconds total, 24 observed
tool calls and 32 observed turn/usage events, with no retry/fallback. Actual
provider requests, hidden retries and billing remain unknown. Existing normal
authentication, disposable private native history and unknown managed/remote
retention were included in the owner decision. Deleting owned roots is not a
remote-deletion or OS-containment claim.

## Disposition

The attempt is complete; all three native capability checks remain **unverified**.
[#742](https://github.com/Rambolarsen/orkworks/issues/742) remains open, and #740
stays closed. No production role eligibility, authority, persistence or runtime
implementation is established. This approval is spent and no ongoing live budget
remains. The later model-free reporting improvement did not itself authorize
a new native run; the revised attempt below received a separate owner decision.

## Documentation verification

The first docs build had one failure in the unchanged synthetic process-cap test:
os.killpg raised PermissionError during cleanup of a Python fake process after
an output-cap stop. That exact synthetic test passed in isolation; its transient
cause remains unverified. This is separate from the native receipt and cannot
explain its generic stop. No native batch was rerun to investigate the test.
The full docs build rerun passed all 1,204 tests and rendered the documentation
with link checks. The first nonzero result remains recorded.

The later [model-free reporting revision](parent-native-packet.md#model-free-stop-reporting-revision--2026-11-09)
added fixed stop categories and process status. It does not modify the first
receipt or identify that failure. Its separately approved attempt is recorded below.


## Revised packet attempt — 2026-11-09

[#1594](https://github.com/Rambolarsen/orkworks/issues/1594) executes one new owner
decision after [#1590](https://github.com/Rambolarsen/orkworks/pull/1590). The owner
answered “go ahead” to the exact revised packet and unchanged budget/risk statement.
Proposal e84b24ab002afc8fe45ecca2a2be6976ad8d1a5fc1f53aa1e5663fe31ec4f297
and manifest 787dc852dc9e64d5333112fc175f96cf7e992c1b86dbbfdc38fb4cda643bdfdd
matched before launch. The runner/fixture files are unchanged from reporting
revision 594e1cff4600364d4cb7a574c9f665195fbde5ed, executed from worktree base
bdd7d10a. This was a distinct one-batch approval, not a retry under the first budget.

The unchanged [revised sanitized receipt](parent-native-canary-revised-summary.json)
has SHA-256 eeb61a24efd3d56a33ec825421897c31819a4756396c3e1d5ef5916eb599ad20.

| Field | Revised observation |
| --- | --- |
| Outcome | unverified |
| Discovery command attempts | 2 |
| Native prompt invocation attempts | 1 |
| Elapsed batch seconds | 4.057 |
| Failure phase / enclosing stage | 1 / native-prompt |
| Failure category | check-failed |
| OS error | null |
| Owned process started | true |
| Observed exit code | 1 |
| Termination steps completed | true |
| Raw roots removed | true |
| Provider request count / remote retention | unknown / unknown |

This establishes that the owned native process was started and its observed exit
code was 1. A host check failed within the native-prompt operation. It does not
identify the particular check: the supervisor checks bounds as well as process
completion, and its cleanup can affect the eventual exit code. Do not conclude
that a CLI option, model, authentication or MCP mapping was specifically rejected.
A native turn/inference, successful tool delivery and billable request count are
not established. There is no successful capability-check block and no second
prompt/resume attempt.

The terminationStepsCompleted field describes the existing TERM/wait/KILL/wait sequence
returning; it is not an OS-confinement or universal descendant-liveness proof.
Raw-root deletion was confirmed by the runner. The temporary approval/output
control root was removed after copying this receipt. No raw history or native
output was inspected afterward, and no retry/fallback/native diagnostic chain ran.

A read-only check of the [retained 1.0.90 CLI help](https://github.com/Rambolarsen/orkworks/blob/594e1cff4600364d4cb7a574c9f665195fbde5ed/docs/validation/fixtures/copilot-1.0.90/help.txt) found the configured option names;
it does not establish MCP availability-name mapping or explain the observed stop.
No installed CLI was launched for that static check.

All three native capability checks remain **unverified**. #742 stays open and
#740 closed. No production eligibility/authority/storage claim follows. This new
one-attempt budget is spent, including its unused phase-two allowance; no further
native run is authorized. A model-free audit of the fixed argv against retained
CLI/source evidence is a possible separate next step before spending more usage.


Evidence validation initially had the same unchanged fake-process output-cap test
fail with os.killpg PermissionError; 1,203 other docs tests passed. Its isolated
rerun passed, followed by the complete docs build with all 1,204 tests and link
checks passing. The transient test cause remains unknown and is not attributed
to this native stop. No runner or fixture code was changed to obtain a green run.
