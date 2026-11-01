---
type: Validation Evidence
title: Copilot selected skill startup delivery
description: One exact synthetic selected skill source was observed in startup context with matching byte count and digest.
tags: [copilot, validation, role-profiles, skills]
---

# Copilot selected skill startup delivery

Date: 2026-11-01. Tracks [#740](https://github.com/Rambolarsen/orkworks/issues/740),
consumers #741–#743. Executes the
[approved one-call plan](../superpowers/plans/2026-11-01-copilot-startup-skill-delivery.md)
using the prior [settings-only capture candidate](copilot-settings-only-capture-comparison.md).

## Decision

**The complete 286-byte selected synthetic skill source was observed in the
model-facing startup context, with its SHA-256 matching the retained source.**
This establishes one exact supplied-startup-context delivery observation. The
source was deliberately packaged into synthetic `AGENTS.md`; no native skill
was installed, discovered or invoked for this source. Native lazy loading,
usage, adherence, referenced resources and all six role profiles remain
unverified/unavailable. It is not a production adapter or eligibility snapshot.

## Authority, source and mechanism

The owner explicitly approved one fresh gpt-6-luna attempt under configuration
SHA-256 `7f226c88b54d02352ffaf788ebf2b545080aee128cc28f7a2155f668af0aa6c0`,
including existing authentication, private disposable native history and the
stated unknown managed-capture/remote-retention risks. The
[authorization](fixtures/copilot-1.0.90-startup-skill/authorization.json) is
historical audit evidence for this exhausted attempt, never reusable authority.

The selected source is
[selected-skill.txt](fixtures/copilot-1.0.90-startup-skill/selected-skill.txt),
containing complete SKILL.md-format frontmatter and body for
`ork740-startup-sentinel`. It has no referenced resources. Its exact bytes are
wrapped with the retained prefix/suffix in
[root-rule.txt](fixtures/copilot-1.0.90-startup-skill/root-rule.txt) and supplied
as synthetic repository `AGENTS.md`. The observer verifies source digest/size
and complete wrapper composition before inference.

The source is not placed in a native discovery directory. The
[role contract](../superpowers/specs/2026-10-04-agent-role-configuration-design.md#adapter-eligibility-and-drift)
permits supplied startup content to confirm delivery while preserving unknown
native invocation. Here that route is exercised manually in a synthetic fixture;
no product code, real repository instruction or automatic adapter was changed.
No native system-prompt replacement is claimed.

## Measured result

| Measurement | Observation |
| --- | --- |
| Live attempts / observed chat spans | 1 / 1 |
| CLI exit / stderr bytes | 0 / 0 |
| Duration | 5.297 seconds |
| Expected arithmetic completion | `42` matched |
| Request / response model | gpt-6-luna / gpt-6-luna |
| Provider check | Observer required github on every span; passed |
| Complete wrapper text | Present in decoded chat system-instruction attribute |
| Complete selected source | Present; 286 UTF-8 bytes |
| Matched source SHA-256 | `3243889e441cf4c2db762ec8893b53aed3b1e78f4059ad9cdf66d716e18c63fa` |
| Observed definitions | view, skill, rg, glob; no tool span observed |
| Input / output / reported reasoning tokens | 4146 / 34 / 27 |
| Settings before/after | Byte-identical |
| Raw OTel size, unretained | 53,961 bytes |
| Native session/turn/interaction matches | All true internally; only booleans retained |
| Disposable roots removed | true |

The source match is measured in the chat request receipt; the observer hashes
the matched text re-encoded as UTF-8 and compares it with the source file.
The arithmetic response contains no delivery marker and is only a completion
check. It cannot establish that the model followed or used the supplied skill.
The requested grep versus observed rg relationship is still unqualified.

Source preflight accepted only the synthetic root instruction and the two
known vendor built-in skill listings. Those are unrelated native built-ins;
the selected synthetic source was supplied explicitly through the wrapper.
Discovery is a guard rather than a complete startup attestation. The pinned
native/cache identities were reconfirmed against six selected embedded files.

## Retention and limits

The [packet](fixtures/copilot-1.0.90-startup-skill/manifest.json) binds exact
configuration, source/wrapper/settings, historical authority, observer, runtime
identities and [sanitized measurements](fixtures/copilot-1.0.90-startup-skill/summary.json).
The retained observer is the source used, not a native dependency or full
provider-request attestation. Its new measurement is limited to source/wrapper
integrity and the selected source text/digest receipt; unchanged guards come
from the previous reviewed comparison.

Narrow span attributes were captured with responses/reasoning, tool output,
policy detail and identity classes requested off. No response-content attribute
appeared in the checked projection. Native history can still contain content
and reasoning-related fields under the approved disposable-history exception;
the observer skipped their values. Raw OTel, vendor context, model output,
native history/SQLite files, identifiers and logs were removed with the private
roots. None is published or sent into agent context by the reduction.

Raw content cannot now be independently checked against the recorded hashes.
These remain observer measurements. No assertion is made about every event,
metric, later input, native persistence path, managed override, remote store,
provider retry/billing or process tree. The private history exception and later
deletion do not establish pre-storage exclusion. Authentication, policy and
ordinary user configuration were not changed.

## Consumer handoff

#743 receives a source-bound startup-delivery observation with explicit
mechanism and unknown native invocation/adherence. It does not receive a
production loaded-state change. #741 receives evidence for this exact simple
source-only delivery candidate, with permissions and complete role eligibility
still unverified. #742 receives no resume or same-parent continuation evidence.
#740 remains open and all six role combinations remain unavailable.

This single-call budget is exhausted. A later native skill-loading or
referenced-resource experiment needs its own reviewed configuration and fresh
finite authority; no tool invocation, new capture class or runtime launch is
authorized by this result.
