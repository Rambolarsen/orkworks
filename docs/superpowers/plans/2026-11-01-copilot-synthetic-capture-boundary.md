---
type: Research Probe Plan
title: Copilot disposable synthetic-history boundary
description: Proposed narrow native-history exception and exact settings-only transport comparison for issue 740.
tags: [copilot, plans, validation, role-profiles]
status: draft
workflow_status: proposed
---

# Copilot disposable synthetic-history boundary

**Outcome:** Test whether the pinned CLI honors settings-only OTel capture
selection, without treating ordinary native history as exporter evidence.
A pass is two correlated single-call observations with the expected contrasting
capture fields; it never qualifies a role or a complete safe-capture transport.

Tracks [#740](https://github.com/Rambolarsen/orkworks/issues/740), consumers
#741–#743. Amends only the bounded research execution gate of the
[original probe plan](2026-10-04-copilot-role-capability-probes.md).
Product [role eligibility](../specs/2026-10-04-agent-role-configuration-design.md#adapter-eligibility-and-drift)
and [preparation](../specs/2026-10-04-orchestrator-preparation-design.md) remain unchanged.

## Authority and status

The owner requested that work proceed after the
[settings/persistence findings](../../validation/copilot-settings-and-persistence.md).
This authorizes preparing this concrete amendment for review. **Exact boundary
and live-batch approval remain pending.** Writing, reviewing or merging a draft
proposal is not approval to run it. Record the approved configuration file hash,
owner response and review disposition before inference; do not infer approval
from the proposal's presence or the older hosted-turn authorization.

No new model request has run under this proposal. This proposes a maximum of two additional observed calls after the earlier
successful turn; failed CLI attempts and their unknown billing remain separately
recorded. Owner approval must explicitly cover this two-attempt/two-observed-call
budget. It is not an assertion of remaining provider quota.

## Proposed exception and unchanged limits

The original plan requires exclusion of responses/reasoning across every
persistence/export path. That universal condition remains unverified and remains
a gate to any claim of complete transport safety. For this exact synthetic
comparison only, approval would permit ordinary native session files and SQLite
state inside newly created private disposable homes. Native persistence may
contain assistant content and reasoning-related values. The observer must not
extract those values, publish them, or send them into agent context.

This is an explicit research exception, not proof of pre-storage exclusion.
It cannot qualify production data, a role/profile, general native-history
retention, or remote export. No real repository content, production session,
personal instruction files or selected skill body is introduced. The observer introduces only the retained synthetic rule and arithmetic prompt;
the native vendor system context and built-in skill listings are accepted as
known native context. Existing native authentication may access credential state
through inherited HOME. This does not authorize personal task instructions,
skills, plugins or extensions from HOME; source preflight must reject extra or
ambiguous sources. No observer reads/copies credential files or places credentials
in inputs/evidence. Discovery is not a complete startup attestation, so this is
a bounded canary with explicit remaining uncertainty.

OTel continues to request responses/reasoning, tool output, policy detail and
identity capture off, with the broad flag explicitly false. This proposal does
not intentionally enable any of them. Unexpected content-bearing fields make
the case fail; stop without printing their values and remove raw files during
cleanup. This is a canary experiment with a known candidate, not a guarantee
against a faulty implementation or managed-policy override.

Remote control and export are requested off with both settings and native
flags. No independent network or remote-storage monitor is claimed. Unknown
remote behavior prevents any no-export guarantee; an advertised setting or an
absent local status flag is insufficient. Do not enable remote export or a real
connector to test it. If the owner needs certified no-remote-storage behavior
before research, keep the original gate and do not run this batch.

## Exact proposal

The [configuration](../../validation/fixtures/copilot-1.0.90-capture-comparison/configuration.json)
and [synthetic input](../../validation/fixtures/copilot-1.0.90-capture-comparison/root-rule.txt)
are bound by the [manifest](../../validation/fixtures/copilot-1.0.90-capture-comparison/manifest.json).
The configuration lists native/cache paths and hashes, full argv, settings,
environment construction, authentication source, observer limits and stop rules.
Configuration file SHA-256: `c9d27f1752467e12baf9ac26e19ea9e46645e2171c9af8464a6b8423def09777`.
Placeholders are observer-created case paths, never permission wildcards.

Use fresh synthetic Git repositories, each containing only `AGENTS.md`, and a
separate home with `settings.json` only. `config.json` must be absent at launch
to avoid legacy migration. Each case root is private (mode 0700). No user config,
trust store, managed policy, login, installation or real repository is changed.
Do not copy the OrkWorks hooks, APM assets, plugins, tools or instructions into
these repositories. Initialize Git using an empty template directory and observer
GIT_CONFIG_GLOBAL=/dev/null plus GIT_CONFIG_NOSYSTEM=1; do not inherit host
templates or hooks. Before inference in each case, run the exact instruction
and skill discovery commands listed in the configuration (four total maximum,
30 seconds each) without OTel activation variables. Accept only the synthetic
root instruction and the two named package built-in skill listings. Unexpected
or ambiguous sources stop before inference, with no content read or wider grant. Reconfirm the pinned executable/cache identities and their
embedded-runtime relation before any live launch; drift stops this proposal.

Both cases start from independently materialized settings files retained in the
packet. Their bytes differ only in the prompts and toolArguments booleans; all
other requested settings are identical. Preserve these exact before-bytes and
measure post-settings size/hash. Changed post-settings fails the case without
printing or retaining its unknown body. Never retain managed config/state bodies.

Run these two cases sequentially, one live process and one intended model call
each, against `gpt-6-luna`. No retry, fallback or resume is allowed. The exact
prompt asks only for the decimal result of 17 + 25, with no tools or explanation.
The expected output `42` is a completion check, not a content-delivery receipt.

| Case | OTel classes | Expected bounded observation |
| --- | --- | --- |
| Metadata only | All six classes false | No input-message, system-instruction, tool-definition or response-content attributes |
| Instructions and tools | Prompts and toolArguments true; other four classes false | Exact synthetic rule text and full observed definitions in the chat span; no response-content attributes |

Available-tool filters remain the previous view/grep/glob/skill candidate.
The known prior inventory was view/skill/rg/glob; its relationship to requested
grep is unverified and no equivalence is assumed. Record names and definitions;
only view/skill/rg/glob are accepted observed names when definitions are present;
any other observed inventory ends the batch. The metadata-only case deliberately
lacks a definition receipt and cannot verify its inventory. No tool is requested
or intentionally invoked; any actual tool call ends the batch. This is not a
permission or grep/rg semantics test. Built-in MCPs are disabled; context-name
presence remains discovery metadata, not initialization or access evidence.

The two-process/two-observed-call budget, 120 seconds per process, ten-minute
batch limit and evidence size limits are observer stop limits. They do not
certify the native runtime's request count, retries, process-tree termination or
provider billing. An extra observed call invalidates the batch; the observer
must not launch a third attempt. Stop after the first failure or ambiguity.

Managed-capture policy has no complete effective receipt in this candidate.
Fail if supported diagnostics establish broader required capture. If they do
not establish policy, retain it as unknown and require the owner to accept that
canary risk explicitly; do not claim that absence of a warning proves safety.
No real managed-policy file is read or changed. Unexpected exported content
still fails the experiment and is never printed or retained.

## Execution and evidence responsibilities

The owning session alone creates the disposable roots, runs the approved native
CLI and writes the research artifact. The review context is read-only.
No production code or SDK integration is in scope.

- [ ] **Review and bind:** Complete written review of this amendment and its
  exact configuration. Record the owner's explicit approval of the native-history
  exception, authentication, known remote-enforcement limit and two-call budget.
  Missing approval leaves this proposal ready for approval only, not execution.
- [ ] **Prepare:** Verify executable and selected cache hashes against embedded
  bytes, record allowed inherited nonsecret bindings with path normalization,
  ensure unlisted variables are absent, and retain exact synthetic inputs plus
  before-settings bytes. Never read full environment or real configuration files.
- [ ] **Measure:** Execute the two cases in order with the stated stops. Bind
  model/request and observed session/turn/interaction identifiers internally;
  retain correlation booleans and counts, not raw identifiers. A failed first
  case omits the second explicitly; no refusal is a permission-denial pass.
- [ ] **Reduce:** Parse only known OTel shapes using an allowlisted measurement
  projection: record attribute names, approved model/token counts, tool names,
  exact-rule inclusion booleans and hashes/sizes of unretained raw artifacts.
  Retain no vendor prompt, model response/reasoning text, identity value, token,
  raw argument, process log or native session body. Native-history inspection is
  limited to filenames, file sizes and event-type/field-presence booleans; never
  inspect reasoning values or import a live addon to query them.
- [ ] **Clean and classify:** Remove the owned disposable roots and confirm the
  foreground processes are accounted for. Do not equate deletion with prior
  exclusion. Retain per-case outcomes, drift/settings changes and an explicit
  list of untested boundaries. Unknown cleanup/process status stops further work.
- [ ] **Review handoff:** Hash/size-check retained/staged/CRLF bytes, build docs,
  inspect the contrast against the case criteria and obtain a fresh written
  evidence review. Update #740/#741–#743 with scoped observations only.

All six roles remain unavailable regardless of this comparison. Selected skill
content, permission denials, complete per-provider coverage, later-input leakage,
remote enforcement, native-history exclusion and same-parent continuation need
separate evidence. No later batch or runtime launch is authorized by this one.

## Review assessment

Disposition: **Ready for owner approval**, not ready for inference.

Written review (2026-11-01, fresh read-only Luna context) initially required
explicit observed-tool names, ambient-source preflight and exact per-case
settings. Those material findings were resolved. Targeted re-review confirmed
ready-for-owner-approval at configuration SHA-256
`c9d27f1752467e12baf9ac26e19ea9e46645e2171c9af8464a6b8423def09777`.
The review grants no live authorization or eligibility.
Missing authority is exact owner approval; missing runtime facts are precisely
what this bounded comparison measures, plus the explicitly untested boundaries.

| Dimension | Rating | Reason |
| --- | --- | --- |
| Dependencies | 3 | Existing native runtime/auth and independent written review; no new integration |
| Blast radius | 1 | Synthetic disposable case roots; no product edits or real task content |
| State changes | 3 | Normal native session history is intentionally allowed only within the proposed exception |
| Reversibility | Unknown | Local deletion is possible; remote/provider retention is not independently established |
| Uncertainty | 4 | Known successful hosted candidate, but settings-only contrast and all storage paths are unqualified |

Assessment total is incomplete because reversibility is Unknown. This is not a
score-based approval; the owner must accept the declared boundary or retain the
original gate. Review focuses on hidden scope widening, contradictory stops,
reasoning/credential exposure through reduction, and whether the remaining
unknowns could be mistaken for role eligibility.
