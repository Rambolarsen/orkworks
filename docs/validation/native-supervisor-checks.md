---
type: Qualification Fixture Correction
title: Fixed native supervisor check reporting
description: Offline synthetic proof of distinct host supervision failure codes without widening capture or changing process guards.
tags: [copilot, qualification, diagnostics]
status: stable
workflow_status: offline-corrected-not-authorized
---

# Fixed native supervisor check reporting

[#1655](https://github.com/Rambolarsen/orkworks/issues/1655) follows the
[operation-lock-compatible attempt](parent-native-canary.md#operation-lock-compatible-packet-attempt--2026-11-09).
Its first-prompt check-failed result and observed exit1 do not uniquely identify
the failed supervisor condition or native exit cause. An offline synthetic
nonzero process reproduces the missing check identifier. This change reports
only the fixture supervisor's fixed host conditions, with no native output capture.

## Startup inspection and limits

Only pinned Copilot1.0.90 app.js bytes were inspected; no vendor module/addon or
native command ran. The SHA-256 is
f84061b62eeb1cd78a655555890fe1a776509203fca68144745467e26e443813.
At byte offset7482348 the embedded help distinguishes tool availability from
permission approval: permission rules do not expose filtered-out tools. At
7531221, the CLI's allow-tool parsing path calls the native permissionsParseRuleValue
bridge; adjacent code handles deny-tool similarly. At7565370 and7567664,
availableTools is passed into session/runtime options. These inspected paths
show delegation and configuration plumbing, not proof that the selected raw
MCP tool names map correctly or that permission parsing/startup succeeds.

No concrete startup incompatibility or historical cause is proven by this
bounded inspection. Native bridge behavior, effective environment and the
unretained native error output remain unknown. The change does not guess at
flags, broaden tool access, select another model or authorize a diagnostic run.

## Minimal correction

The supervisor annotates only ValueError instances it creates for its own
existing checks. Their conditions, order, exception messages/types, limits,
callback execution, process-group termination and receipt collection are
unchanged. It does not catch and relabel callback or filesystem errors.

failure_details may add supervisorCheckCode only in instruction-discovery,
skill-discovery or native-prompt stages, for one exact allowlisted string:

| Code | Host condition |
| --- | --- |
| supervisor-deadline | Positive remaining budget or elapsed deadline check. |
| supervisor-raw-layout | Raw-tree symlink guard. |
| supervisor-file-cap | Per-file raw byte limit. |
| supervisor-total-cap | Aggregate raw byte limit. |
| supervisor-exit | Owned foreground process must exit zero. |

These identify a failed condition, not its native cause. A process exit code
remains independent metadata; cleanup can observe an exit after another check
failed. Existing categories, numeric OS errors and process flags remain intact.
Bind/history callback failures retain their original diagnostics and receive
no inferred supervisor code. Unknown codes, nonstrings and other stages omit
the new field. No exception text, paths, target names, stdout/stderr/history,
payloads or provider responses are retained. The six historical receipts receive
no retroactive codes or cause and remain byte-identical.

## Synthetic evidence and future packet

The failing-first suite ran46 tests with9 failing assertions for absent codes.
All46 pass after correction. Independent fake processes exercise nonzero exit17,
elapsed deadline, per-file/aggregate caps and raw symlink rejection. An expired
budget is rejected before Popen. Bind/history callback errors preserve provenance;
projection denies unknown/private/nonstrings and wrong stages. No test invokes
Copilot. Repository and current-code independent review evidence belongs in the PR.

Proposal SHA-256 remains
39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
Prospective manifest SHA-256 is
e0ec6e8a278b2781c19352652fc101b8abfca641cae3802fa44297a933aa300b.
Only its observer pin changes. The peer, source guards, exact session/event/nonce
joins, model, tools, transport and budgets are unchanged. All six prior native
approvals are spent; none binds this manifest. Live budget is zero. #740 remains
closed, #742 open and all three native capabilities unverified. A future exact
native batch requires a fresh owner budget/risk decision after packet hashing.
