---
type: Source Compatibility Audit
title: Native canary UUID compatibility correction
description: Offline source proof of an invalid synthetic session ID, its bounded correction and remaining guard/tool-name uncertainties.
tags: [orchestration, copilot, qualification, uuid]
status: stable
workflow_status: corrected-not-authorized
---

# Native canary UUID compatibility correction

[#1602](https://github.com/Rambolarsen/orkworks/issues/1602) audits the fixed argv
and host guards after the [two unverified attempts](parent-native-canary.md).
It found a concrete input defect: the fake-only ID
00000000-0000-0000-0000-000000000001 was also supplied as a new native UUID.
The pinned CLI's ordinary UUID rule rejects its version-0 and variant-0 bits.

The corrected synthetic ID is 00000000-0000-4000-8000-000000000001. It has accepted
version/variant bits; no randomness or native uniqueness proof is claimed.
This is an offline fixture/preflight correction, **not a successful native run**.
Neither historical receipt was changed, and no Copilot command, vendor module,
native discovery, credential read or model request ran for this task.

## Pinned source proof

[Audit data](parent-native-canary-offline-audit.json) records exact source hashes,
byte anchors, literal UUID vectors and a separate synthetic-constructor measurement.
The cached 1.0.90 app.js is 7,577,057 bytes with SHA-256
f84061b62eeb1cd78a655555890fe1a776509203fca68144745467e26e443813,
the same source identity checked by the old packet. Byte offsets are zero-based:

| Source anchor | UTF-8 byte offset |
| --- | --- |
| uuidPattern | 20888 |
| uuidHelper | 21049 |
| uuidAlias | 21129 |
| freshSessionGuard | 7548668 |
| fatalHelper | 7574962 |
| exitHelper | 5165286 |
| filterNormalizer | 4025448 |
| cliParserBridge | 7394212 |
| promptArrayPassThrough | 4961085 |

The gPn helper tests the captured regexp; IT is its alias. In the fresh
--session-id no-match path, a failed IT check calls the fatal helper. That helper
passes 1 through the exit wrapper to process.exit. Pure regexp reproduction
rejects the old value and accepts the corrected value and the CLI help's UUID
example. No cached vendor JavaScript or native addon was imported or executed.

This establishes a source-level blocker on that creation path, consistent with
the observed exit 1. It does **not** establish that the deleted historical runs
reached that exact branch or loaded every cached component being audited.
Actual native output and branch evidence were not retained, so their particular
failure cause and provider activity remain unconfirmed.

The CLI argument parser is delegated to the separate cli-native addon. Its
5,151,344-byte file was only read/hashed
(e3d5cce8ee565c66726d1dda462ea5b6b335d46394e293c0e64d555244563176);
its actual historical loading is not proven by the old receipt.

## Host checks and argv limits

For the recorded phase-1/native-prompt check-failed result with started true,
explicit remaining host checks include an unexpected raw symlink, the per-file
cap, the total cap and nonzero process completion. Under the runner's shared
monotonic clock, 4.057 elapsed batch seconds is below both 240 seconds and the
remaining 600-second batch allowance, so its deadline guards cannot explain
that observation. ValueError is still a category rather than a retained guard
code; the audit does not identify which particular check raised.

A model-free copy of the historical 594e1cff packet's prepare_case plus empty
Git setup produced 10 regular files, largest 30,007 bytes, total 97,913 bytes,
and no symlinks, below 1 MiB/file and 8 MiB total. It never started a native
process. These are representative constructor inputs, **not measurements of the
deleted native-created files**; they do not exclude a later cap or symlink stop.

Configured option names are present in the retained help. The cached JavaScript
normalizes the available-tools strings into an array and passes that array to
native session options. The explicit JavaScript initialized-name validation
found in prompt mode applies to a function resolver, not this array case.
Native MCP naming/filter behavior remains delegated/unverified. There is no
source-backed reason here to widen tool availability, use allow-all, change
transports/models or retry automatically.

## Bounded correction and review contract

The peer, literal configuration/bindings, proposal and new/resume argv now use
the same corrected ID. The native preflight requires a canonical ordinary UUID
with RFC variant and version 1–8 after exact approval/source checks and before
platform setup or any subprocess. The packet uses version 4. This profile guard
is intentionally narrower than the CLI's case-insensitive rule and special
sentinel exceptions; it is not a general replacement for native UUID parsing.

Only native_id/configuration_digest references changed in the independent peer
transcript. Cursor, version, state, event, effect and gate outcomes are unchanged;
they were compared after normalizing those two deliberate references, not
regenerated from scripted-client output. The configuration was independently
rehash-bound as 538db3fc348f76c26a32f82f159770acd1e16e97b0a2fe80a2c7520283a04e33.
An explicit fixture-contract check is part of this change's fresh code review.

Invalid version/variant/malformed inputs and early guard placement received
failing-first regressions. The 26 packet and 18 direct-call peer tests pass
using only synthetic clients/processes. Broader required checks are recorded
in the PR before merge.

Current proposal SHA-256: 39bea8c908bb15f3d53ed8a8cd6fdbc38db341f0f30b53f0617b47929fdd027b.
Current manifest SHA-256: e2cb9c9c41d2b0137aad51bd25fa12f367426fe2dcc9bdb075fc7a5ec95471ec.
Both spent approvals fail to bind the corrected bytes. No live budget is
authorized. All three native capabilities remain unverified, #742 stays open,
and #740 stays closed. A future exact owner-approved batch remains necessary
for tool delivery, cursor-gap recovery and exact native resume.
