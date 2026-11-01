---
type: Validation Evidence
title: Copilot November hosted transport probe
description: One successful hosted Copilot turn and the remaining capture and role eligibility limits.
tags: [copilot, validation, role-profiles]
---

# Copilot November hosted transport probe

Date: 2026-11-01. Tracks [#740](https://github.com/Rambolarsen/orkworks/issues/740);
consumers #741–#743. Continues the [Probe 0 report](copilot-probe0-transport-qualification.md)
after the owner resumed the deferred work and authorized a three-call budget.

## Decision

**One hosted turn completed; transport and all six role profiles remain
unverified/no-go.** The native executable still reports 1.0.90 and matches the
previous native SHA-256. `gpt-6-luna` produced the expected synthetic response.
No permission-denial, selected-skill, resume or continuation probe ran.

The [retained packet](fixtures/copilot-1.0.90-november/manifest.json) contains
sanitized measurements, exact synthetic rule bytes and a concrete configuration.
Raw telemetry, vendor instructions, session content, reasoning-related values,
identifiers and authentication output are not published. Content hashes are
observer measurements of unretained data, not independently replayable receipts.

## Attempts and authorization

The owner approved the concrete first turn, existing authentication use,
120-second limit, no retry, disposable files and narrow capture. The first
attempt was interrupted by the sandbox network allowlist at
`api.business.githubcopilot.com`; it produced no stdout or OTel file. Its
execution receipt was interrupted too; a subsequent process check found its
Copilot PID absent. This is an execution-environment block, not a quota or
permission-denial fixture.

The owner separately approved one network-enabled attempt. Copilot rejected
`gpt-5-mini` as unavailable, despite its presence in installed configuration
help. That process exited 1 in 4.115 seconds with no OTel file. Availability is
account/runtime-specific; a help listing is insufficient.

The owner separately approved the final attempt with `gpt-6-luna`. It exited 0
in 5.485 seconds, with the expected stdout, empty stderr and a 52,854-byte
OTel file containing 15 JSON records. Three CLI attempts produced one successful
hosted turn; provider attempts and billable usage for failed attempts were not
independently measured. No further probes were run.

## Observations

| Surface | Measured result | Limit |
| --- | --- | --- |
| Model | Chat request and response attributes both name `gpt-6-luna` | One turn; no fallback/retry coverage |
| Rule delivery | Exact synthetic root-rule text occurs in decoded system-instruction attributes on chat and invocation spans | No scoped rules, custom role or selected skill tested |
| Tools | Both spans contain `view`, `skill`, `rg`, `glob` | Launch requested `grep`; native mapping differs from the previous packet. Do not assume action-name identity or complete per-provider coverage |
| Response capture | No `gen_ai.output.messages` attribute on either span; output usage is 40 tokens, including 28 reported reasoning tokens | No hidden reasoning content inspected or retained; one turn cannot establish exclusion across later inputs or every path |
| Input marker | The response sentinel is already in the user prompt | Its presence in input telemetry cannot test response leakage; output-field absence is the bounded observation |
| Other persistence | Session `assistant.message` has nonempty content, reasoningBlocks, reasoningOpaque and encryptedContent fields; reasoningText is empty | These are presence checks only, not a claim that plaintext hidden reasoning was persisted. OTel settings do not prove exclusion from session persistence |
| MCP/skills | Invocation context lists `github-mcp-server`, `githubiq` and two built-in skills | Discovery/context names do not establish initialization, access or skill-content delivery |
| Identity | Invocation span includes `enduser.pseudo.id` with identity capture disabled | No value retained; identity-off does not mean all identifiers are removed |

The configuration was seeded in both `config.json` and `settings.json` within
a fresh `COPILOT_HOME`. Copilot rewrote `config.json` to managed state;
`settings.json` retained the requested granular capture controls. The older
retained Probe 0 `probe-config.json` likewise contains only managed first-launch
state, not the capture configuration described in its report. It cannot by
itself reproduce that earlier capture setup. This packet records the fresh
configuration explicitly; it does not retroactively repair the old evidence.
Effective precedence and supported single-file routing remain unqualified.

## Handoff

#741 retains unavailable profiles: a successful response and model-visible
inventory do not establish filesystem scope, permissions or startup isolation.
#743 gains one exact synthetic rule-delivery observation; selected skill-body
and referenced-resource delivery remain open. #742 gains no continuation or
resume evidence. #740 remains open.

Before another live batch, qualify the settings route and persistence boundary,
investigate the native `grep`/`rg` relationship, and propose the exact skill or denial case
under a fresh finite authorization. Do not enable tool-output or broad content
capture to fill the gap. No runtime implementation or role launch is authorized
by this report.

The summary records the unretained event-file hash/size and boolean session,
turn and interaction matches against the chat span. These observer checks bind
the persistence observation to this turn without publishing native identifiers;
they do not make the unretained event bytes independently inspectable.
