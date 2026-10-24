---
type: Design
title: Explicit sandbox proxy support for session metadata
description: Add an owner-selected loopback HTTP proxy transport without automatic proxy discovery or launch activation.
tags: [codex, metadata, sandbox, security]
status: stable
---

# Explicit sandbox proxy support for the metadata helper

Owner-approved security design, 2026-10-24: the owner approved this concrete
proposal with “go ahead.” Approval covers explicit helper support and verification;
no launch marker, sandbox setting or production qualification is authorized.

## Evidence and purpose

The current Codex config already enables sandbox networking and permits
127.0.0.1 through its managed HTTP proxy. Ordinary direct sockets to the sidecar
return EPERM. A credential-free proxied /health returned HTTP 200. A temporary
in-memory transport adapter then ran the installed helper's real bootstrap
inside this sandbox: authenticated GET, validation-only PATCH and follow-up GET
all passed. This corrects the earlier claim that this sandbox has no API path:
the existing helper bypasses the available proxy.

## Proposed bounded change

Add an explicit optional `--http-proxy URL` argument to `session-metadata.py`.
Direct operation remains the default and ignores ambient proxy variables.
For this sandbox an owner-authorized agent explicitly supplies its Codex-managed
HTTP proxy URL at invocation time (for example, from the known managed runtime
environment). This first change does not automatically add the flag to existing
bootstrap callers. The helper never discovers or silently trusts HTTP_PROXY/ALL_PROXY
itself. Automatic transport selection for a future qualified launch is outside
this bounded helper-support change and must identify its trusted URL source.
Accept only `http://127.0.0.1:<port>`, with no credentials, path other than an
optional slash, query, fragment or control characters. Reject ambiguous values
before a connection or authenticated request. Require a valid 1..65535 port.

The explicit proxy is a trusted relay and can observe the bearer token. A
loopback address alone does not establish trust; callers must select the actual
sandbox-managed proxy. Do not advertise arbitrary localhost relays as trusted.
The sidecar destination remains pinned to 127.0.0.1 and ORKWORKS_PORT. Proxy
requests use that exact absolute destination and the existing bounded session
metadata route. Never use a caller-controlled origin or follow redirects.
Preserve existing timeout/body limits, response/session/revision checks,
validation-only bootstrap, conflict handling, sanitized errors, and no-file-
fallback behavior. A selected proxy failure never falls back to direct access.

Do not change the global Codex config: its localhost allow rule already works.
Do not broaden sandbox networking, turn off its proxy, modify an existing
session's launch contract, or enable any completion capability/qualification row.
This change is helper transport support, not a new metadata protocol or mailbox.
The running legacy session is not upgraded to API-only writes by this design.

## Implementation and verification

1. Add real local proxy fixtures and failing helper regressions for explicit
   proxy bootstrap/patch and invalid proxy inputs, retaining the existing test
   that ambient proxy settings cannot redirect credentials.
2. Implement the optional transport and document explicit trusted proxy use.
   Check both transport paths for redirects, conflicts, malformed responses,
   wrong session identity, failure with no fallback, and secret-free errors.
3. Verify the installed/source-matching helper through the real managed proxy
   in an authorized agent context, with the three-step non-mutating bootstrap.
   Fixture results alone are not installed qualification. Run required repo
   checks and current-diff review before merge; use normal helper reconciliation.

The ordered/loss-detectable hook relay and effective single-root launch remain
independent prerequisites. This transport fix alone cannot activate completion
clearing or resolve the full Needs You bug.

## Approval boundary

The owner requested enabling API access in the sandbox. The proposed explicit
proxy changes the credential relay from direct-only to a caller-selected local
broker. Root AGENTS.md requires approval of a concrete security design before
dependent implementation. Approval of this document authorizes this helper
change and its verification; no additional architecture approval is needed
within these boundaries.

## Builder review disposition

The reviewer asked who supplies the flag in operational invocations. The bounded
answer above is explicit owner-authorized invocation, with documented use; no
automatic caller/launch migration is claimed. Automatic launch selection is a
separate integration decision. No other plan-changing question was found.
