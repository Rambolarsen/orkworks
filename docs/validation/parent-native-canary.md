---
type: Native Canary Evidence
title: Parent continuation canary stopped unverified
description: Six separately approved batches stopped unverified; the operation-lock-compatible packet observed a first-prompt process exit of one.
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


The later [offline UUID audit](parent-native-canary-offline-audit.md) found and
corrected an invalid ID in the future packet. Its source-level rejection path is
consistent with exit 1 but cannot be retroactively identified as either run's
particular cause. Both receipts and their hashes remain unchanged.


## UUID-corrected packet attempt — 2026-11-09

[#1613](https://github.com/Rambolarsen/orkworks/issues/1613) executes the new owner
decision after [#1610](https://github.com/Rambolarsen/orkworks/pull/1610). The owner
answered “go ahead” to the corrected exact packet, existing authentication and
acknowledged retention/billing uncertainties. Proposal, manifest and executable
fixture files matched merge ad007acaf740c994a8b84f5b86a4151ae528af95 before launch;
the isolated execution worktree started at 06d658048d03b79952a11727dbf35637dd4fb897.
The reviewed inputs and runner were not changed for this batch.

The unchanged [corrected sanitized receipt](parent-native-canary-corrected-summary.json)
has SHA-256 641708138cb1f13157ce9e59c6c34a3905667f409d51a034071042399076a925.

| Field | Corrected-packet observation |
| --- | --- |
| Outcome | unverified |
| Discovery command attempts | 2 |
| Native prompt invocation attempts | 1 |
| Elapsed batch seconds | 4.909 |
| Failure phase / enclosing stage | 1 / native-history |
| Failure category | check-failed |
| OS error | null |
| Owned process started | true |
| Observed exit code | 0 |
| Termination steps completed | true |
| Raw roots removed | true |
| Provider request count / remote retention | unknown / unknown |

The receipt now locates the failed enclosing operation at native-history and
records native exit zero. Neither establishes a successful model turn, delivered
tool result or completed capability check. History scanning can run during the
supervisor or after its clean return; this receipt does not distinguish them.
History checks cover expected session layout, append-only prefix/tail and bounded
allowlisted event projection. The particular failed condition was not retained.
Do not infer missing history, a schema mismatch, authentication/model rejection or
MCP name mapping as the cause. There is no successful checks block, and phase two
was not attempted; exact resume remains unverified.

The observer confirmed deletion of its raw roots. The disposable approval/output
control root was removed after copying only the sanitized summary. No raw output
or history was inspected afterward, no extra native diagnostic command ran and
no retry, alternate naming/transport/model or fallback was attempted. The recorded
termination sequence is not a universal descendant-liveness or confinement proof.

Exact approval/source commitments:

- Proposal SHA-256: 39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
- Manifest SHA-256: e2cb9c9c41d2b0137aad51bd25fa12f367426fe2dcc9bdb075fc7a5ec95471ec.

Bounds were unchanged: two sequential gpt-6-luna prompts, four source-discovery
commands, 240 seconds per prompt/600 total, 24 observed tools and 32 observed
turn/usage events, no retry. Provider request count/billing and remote retention
remain unknown. The pinned observer Python used -E -s -B; the fixture helper kept
its approved -I -B flags. Both earlier receipt files remain byte-identical.

This one-batch approval is spent, including unused phase-two allowance. All three
native checks remain unverified; #742 stays open and #740 closed. No production
role/source eligibility, authority, storage or implementation claim follows.
Before another live proposal, the useful bounded follow-up is a model-free audit
of the pinned native-history layout/event contract against the observer, with a
minimal fixed failure-code projection only if needed. This is a suggestion, not
new execution authority or permission to inspect deleted raw data.


The later [offline history audit and reporting correction](native-history-source-audit.md)
adds fixed check codes to future summaries without identifying the historical
failure, changing these receipts or authorizing another batch.


## History-diagnostic packet attempt — 2026-11-09

[#1626](https://github.com/Rambolarsen/orkworks/issues/1626) executes the exact new
owner decision after [#1621](https://github.com/Rambolarsen/orkworks/pull/1621).
The owner answered “go ahead” to the diagnostic batch with existing authentication,
unknown provider billing/retention and unchanged limits. The proposal, manifest
and executable fixture bytes matched d913e501229c7b6ff907af4e87ddcd01371c48bd;
the isolated execution worktree started at 0cad04787b6fb9800fd4ffd222f35285335ce228.
No runner or packet input was edited for this attempt.

The unchanged [diagnostic sanitized receipt](parent-native-canary-diagnostic-summary.json)
has SHA-256 62759301bec8cc6ef60f703714c52e24f45db610edfea39bccafea0e5090793b.

| Field | Diagnostic observation |
| --- | --- |
| Outcome | unverified |
| Discovery command attempts | 2 |
| Native prompt invocation attempts | 1 |
| Elapsed batch seconds | 4.711 |
| Failure phase / stage / category | 1 / native-history / check-failed |
| Fixed check code | history-session-layout |
| OS error | null |
| Owned process started / observed exit code | true / 0 |
| Termination steps completed | true |
| Raw roots removed | true |
| Provider request count / remote retention | unknown / unknown |

This code narrows the failed condition to History.scan's exact-session entry
check: the enumerated root entries must equal the single expected UUID path, and
that expected entry must not be a symlink. The receipt does not retain the entry
count, names, types or which side of that condition failed. An empty/transient
root, another entry, a different session entry or the expected symlink cannot be
distinguished. These are possible guard inputs, not findings about the deleted
native files. Periodic and final scans use the same check; the receipt does not
identify which scan failed. Earlier receipts receive no retrospective cause.

The stage/code and observed exit zero do not establish a model turn, successful
tool delivery or provider billing count. There is no successful checks block and
no second prompt/resume. All three native capabilities remain unverified. The
recorded termination sequence is not a universal descendant-liveness proof.

The observer confirmed deletion of owned raw roots. The disposable approval/output
control root was removed after copying only this summary. No raw output/history
was inspected afterward, and no extra native diagnostic, retry, alternate tool
name/model/transport or fallback ran. The first three receipts are unchanged.

Exact approved commitments:

- Proposal SHA-256: 39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
- Manifest SHA-256: 809bcb6f61df6803e6fe4d080072d45b1d739a1d8a69ec4e41d9ecc3d2e0d7a2.

The pinned observer Python used -E -s -B; the helper kept its approved -I -B flags.
Bounds were two sequential gpt-6-luna prompts, four discovery commands,240 seconds
per prompt/600 total,24 observed tools and32 observed turn/usage events, no retry.
This exact approval is spent, including unused phase-two allowance. Ongoing live
budget is zero. #742 stays open and #740 closed; no production eligibility,
authority, storage or implementation claim follows.

The next useful bounded task is offline investigation of session-directory creation
and layout assumptions, including transient empty-root handling. If static evidence
cannot resolve the entry shape, prepare only fixed counts/booleans distinguishing
layout conditions for a separately approved packet. Do not weaken exclusivity or
invent an actual native layout from this receipt. No further execution is authorized.


The later [offline startup/layout correction](native-history-startup.md) reproduces
an initial empty-root rejection using synthetic directories and adds fixed layout
flags for future failures. It does not recover this attempt's entry shape, identify
its cause, change any historical receipt or authorize another native run.


## Corrected-startup packet attempt — 2026-11-09

[#1637](https://github.com/Rambolarsen/orkworks/issues/1637) executes one new exact
owner decision after [#1633](https://github.com/Rambolarsen/orkworks/pull/1633).
The owner answered “keep going” to the corrected batch question, unchanged limits,
existing authentication and acknowledged provider billing/retention uncertainties.
Executable packet bytes matched 42ea491e94f5a66bb151413e34f6579cde38599b before
launch; the isolated worktree started at 5bc01fb75a6ae6f1d4a1362d280a035617448fe8. No packet source was changed.

The unchanged [startup sanitized receipt](parent-native-canary-startup-summary.json)
has SHA-256 4baa1da6e4c1a22331a97de3bd6f5cd54acda8ad2d8a97b288c1d983017c0e78.

| Field | Corrected-startup observation |
| --- | --- |
| Outcome | unverified |
| Discovery command attempts | 2 |
| Native prompt invocation attempts | 1 |
| Elapsed batch seconds | 6.17 |
| Failure phase / stage / category | 1 / native-history / check-failed |
| Fixed check code | history-session-layout |
| layout.empty | false |
| layout.expectedPresent | true |
| layout.exclusiveExpected | false |
| layout.expectedSymlink | null |
| OS error | null |
| Owned process started / observed exit code | true / 0 |
| Termination steps completed / raw roots removed | true / true |
| Provider request count / remote retention | unknown / unknown |

The enumerated list contained the expected UUID path and was not the exclusive
single expected entry. It was nonempty. This identifies an additional-list-entry
condition in this attempt, not an empty-root stop. Names, exact counts and types of the
additional entries were not retained. In particular, the receipt does not identify
auxiliary files, another session, a replacement identity or a particular native
layout. expectedSymlink is null because the scanner checks it only for an exclusive
expected entry; null is not evidence that the path is or is not a symlink.

Periodic and final scans share this check, and the receipt does not distinguish
them or retain prior successfully consumed event counts. Do not infer that no
history was ever read, or assign this entry shape to earlier attempts. There is
no successful checks block, native capability proof or second prompt/resume.
A model turn, tool delivery and provider billing count remain unverified.

Exact approved commitments:

- Proposal SHA-256: 39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
- Manifest SHA-256: e89cd3cf0ba4c9b91f7a752edecd6a59f815d02d087503a4ac577959f90f7c9a.

The observer confirmed raw-root deletion. Disposable approval/output controls
were removed after copying only this sanitized summary; the launcher was removed
at close-out. No raw data was inspected afterward and no extra native diagnostic,
retry, alternate names/model/transport or fallback ran. The four earlier receipt
files remain byte-identical. Termination steps are not a universal descendant-
liveness or OS-confinement proof.

The pinned observer Python used -E -s -B; the helper kept -I -B. Bounds were two
sequential gpt-6-luna prompts, four discovery commands, 240 seconds per prompt and
600 seconds total, 24 observed tool calls and 32 observed turn/usage events, with
no retry. This fifth approval is spent, including unused phase-two allowance;
ongoing live budget is zero. #742 stays open and #740 closed; all three native
capabilities and production eligibility remain unverified.

Next useful work is offline review of the root-exclusivity assumption against
pinned native layout/creation evidence and the actual exact-session identity
contract. The additional entries are not automatically evidence of a foreign
session, and are not automatically safe bookkeeping. Do not weaken a guard or
spend further usage based on either assumption. No further execution is authorized.


## Operation-lock-compatible packet attempt — 2026-11-09

[#1651](https://github.com/Rambolarsen/orkworks/issues/1651) executes one fresh
exact owner decision after [#1647](https://github.com/Rambolarsen/orkworks/pull/1647).
The owner approved the post-hash batch question with the unchanged limits,
existing authentication and acknowledged provider billing/retention uncertainties.
Packet bytes matched merge 4cb91396e608ca7dac3b379687540c010b0bbca1 before
launch; the isolated worktree started at f9608963769535afc8b5364771f16d873ab4a7a8.
No packet source changed. The bookkeeping compatibility choice and its static
limits are recorded [separately](native-history-operation-locks.md).

The [sanitized receipt](parent-native-canary-operation-lock-summary.json) has
SHA-256 2eebfe7b01814bdcd9a8acb7718646142b3c47f616770d1a25f03631c280e043.

| Field | Operation-lock-compatible observation |
| --- | --- |
| Outcome | unverified |
| Discovery command attempts | 2 |
| Native prompt invocation attempts | 1 |
| Elapsed batch seconds | 9.376 |
| Failure phase / stage / category | 1 / native-prompt / check-failed |
| OS error | null |
| Owned process started / observed exit code | true / 1 |
| Termination steps completed / raw roots removed | true / true |
| Provider request count / remote retention | unknown / unknown |

No second prompt or resume ran. There is no successful checks block, retained
history-check code or layout flags. The native process started and an exit code
of one was observed; the fixed diagnostic does not identify the failed prompt
supervision condition or the process's exit cause. Do not infer authentication,
model availability, MCP naming/permissions, successful tool delivery, another
session, acceptance of the bookkeeping namespace or native capability support.
Periodic history scans do not record progress in this receipt, so absence of a
history failure is not proof of history consumption or a particular entry shape.
No raw stdout/stderr/history was inspected after cleanup. No extra native
command, diagnostic, retry or model/tool/transport fallback ran.

Exact approved commitments:

- Proposal SHA-256: 39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
- Manifest SHA-256: 1c07a44951b076dacadbd2a35611ef1cf3077d56dec37294eeae92b3cdff1d22.

The pinned observer Python used -E -s -B; the helper retained -I -B. The observer
confirmed owned raw-root deletion; disposable approval/output controls were
removed after copying only this sanitized summary. The one-shot launcher was
removed at close-out. Termination steps are not universal descendant-liveness
or OS-containment proof, and owned-root deletion is not remote deletion.
All five earlier receipt files remain byte-identical.

Bounds were two sequential gpt-6-luna prompt invocations, four discovery commands,
240 seconds per prompt and 600 seconds total, 24 observed tool calls and 32
observed turn/usage events, no retry/fallback. This sixth approval is spent,
including unused phase-two allowance. Ongoing live budget is zero. #742 remains
open, #740 closed and all three native capabilities and production eligibility
remain unverified. Future native execution requires another exact owner decision.
The next useful investigation is offline inspection of the fixed prompt
supervision diagnostics and pinned startup assumptions before proposing more usage.
