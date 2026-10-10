# Copilot 1.0.90 Probe 0: OTel transport qualification

Date: 2026-10-06. Tracks [#740](https://github.com/Rambolarsen/orkworks/issues/740),
initiative #738; consumers #741–#743. Executes the single owner-approved
qualification probe from the [probe plan](../superpowers/plans/2026-10-04-copilot-role-capability-probes.md).
Builds on the [selective-capture research](copilot-selective-capture-research.md).

## Decision

**The native OTel file exporter, with granular capture settings in a disposable
`config.json`, is a viable measurement transport for delivered system
instructions and the effective tool inventory. Transport qualification is still
incomplete: response/reasoning exclusion was not exercised, and no profile
qualifies. All six role profiles remain unverified/no-go.**

The one model call was refused by the service (HTTP 402, monthly quota
exhausted), so no response, tool call or denial was observed. The exporter
still wrote a real 1.0.90 sample for the failed request, which is the first
native runtime evidence for the questions the static research left open.

## What ran

One process, no retry, exit 0, under the approved budget. Exact environment,
argv and limits are in [`argv.txt`](fixtures/copilot-1.0.90-probe0/argv.txt);
synthetic inputs and the capture settings are in
`fixtures/copilot-1.0.90-probe0/inputs/`. Settings enabled
`telemetry.capture.prompts` and `telemetry.capture.toolArguments`; responses,
tool output, policy detail and identity were disabled. No existing user
configuration, authentication or policy was changed.

## Findings

| Question | Result | Status |
| --- | --- | --- |
| Granular capture keys accepted via `config.json` in `COPILOT_HOME` | Prompts and tool definitions were captured with the broad flag off | Observed, 1 sample |
| Effective tool inventory per model request | The `chat` span carried the full definitions: exactly `view`, `skill`, `grep`, `glob`, matching `--available-tools` | Observed for this request; retries/later requests unobserved |
| Root rule delivery | Root `AGENTS.md` text is present in `gen_ai.system_instructions` | Observed (marker plus rule heading) |
| Scoped rule delivery | Not expected (cwd was the repo root); not tested | Open |
| Skill delivery | Skill name/listing is in system instructions; **the skill body (`ORK740_SKILL`) is not**, so it is loaded lazily on invocation | Observed: listing only, content delivery unverified |
| Response/reasoning exclusion | No response existed, so exclusion is **not demonstrated** | Open |
| Identity attributes | `user.name`, `process.user.name`, `host.name` absent; **`enduser.pseudo.id` present** although identity capture was off | Observed; redacted in retained copy |
| Built-in MCP disabling | `--disable-builtin-mcps` set, yet `github-mcp-server` (source `builtin`) and `githubiq` were reported as `discovered` and appear in `context.mcp_server_names`; neither appears in the tool inventory | Observed; connection state and effect unverified |
| Built-in skills | `customize-cloud-agent` and `github-pr-media` are listed beside the sentinel despite the disposable home | Observed, as in discovery research |
| Model | `claude-sonnet-5` reported as the default; no pinning was applied | Reported, not pinned |

## Consequences

- Probe 1 (allowed/denied `review` matrix) is blocked by quota, not by
  authorization. The measurement route no longer depends on a successful model
  call for delivery and tool-inventory evidence.
- The skill-content result matters for #743: delivery of a selected skill's
  bytes needs either an observed skill invocation (tool result capture, which
  conflicts with keeping tool output off) or startup injection, so the
  eligibility rule must say which.
- The MCP finding is a candidate bypass surface for #741: a disabled
  built-in server is still discovered. Whether it can connect or expose tools
  needs a dedicated check before any role claims "no connectors".
- `enduser.pseudo.id` shows that "identity off" does not remove every
  account-derived attribute; any retained telemetry must be redacted.

## Retained evidence and its limits

`fixtures/copilot-1.0.90-probe0/otel.sanitized.jsonl`
replaces the three large content attributes (input messages, system
instructions, tool definitions) with byte counts and SHA-256 values and
redacts `enduser.pseudo.id`; the vendor system prompt is not republished.
[`summary.json`](fixtures/copilot-1.0.90-probe0/summary.json) holds the
measured counts and names and the hash and size of the unredacted 38,494-byte
file, which is not retained. Hash binding of the redacted attributes to
delivered bytes therefore cannot be re-checked by a reader; it records what the
observer measured.

## Probe 0b: three further refused-turn cases (2026-10-06)

Owner-approved follow-up, three single-turn runs with no retry, same
environment construction and quota refusal as above. Setup, hashes and
measurements are in `fixtures/copilot-1.0.90-probe1/` (`argv.txt`,
`summary.json`, sanitized exports). The fixture's skill now has a sibling
`reference.md` containing `ORK740_REF`.

| Question | Result | Status |
| --- | --- | --- |
| Nested rule delivery | With cwd in `scope/`, both `ORK740_ROOT` and `ORK740_SCOPE` appear in system instructions; with cwd at the root only the root rule appears | Observed, 1 sample each |
| Referenced skill resource | Neither the skill body nor `reference.md` content (`ORK740_REF`) is delivered at startup; only the skill name is listed | Observed; delivery on invocation unverified |
| Effect of `--disable-builtin-mcps` | Without the flag, `github-mcp-server` reaches state `initialized`; with it, the server stays `discovered` only, and `githubiq` additionally appears as `discovered` (no `source` attribute) | Observed: the flag prevents builtin initialization; `githubiq` origin unknown |
| Repo-independence of MCP discovery | Both servers are still discovered from a non-git directory, where no rule or sentinel skill is found | Observed |
| Tool inventory | Identical four tools in all three cases | Observed |

`githubiq` appears only when the flag is set, and its origin was not
determined. It is not in the tool inventory, but "no connectors" stays
unverified until its source and connection behavior are established. Rules and
the sentinel skill come from the git root of the working directory; discovery
outside a repository found neither.

## Proposed Probe 2: mock-endpoint capture test (not run)

The remaining capture gate, that response and reasoning content are excluded
from the export, needs a completed model turn, which the quota blocks until
November 1. Native help documents a custom-provider (BYOK) route that needs no
GitHub authentication, so a local mock can supply the turn. This would qualify
the capture transport only. It uses a substituted provider, so it is not
evidence about Copilot-hosted model behavior, permission enforcement or any
role profile, and its result must not be cited as such.

Proposed configuration, for owner review before any run:

- Mock: a throwaway Python HTTP server on `127.0.0.1` (ephemeral port),
  OpenAI chat-completions shape, streaming. It returns one canned reply
  containing the sentinel `ORK740_RESPONSE` and, in a second case, a scripted
  call to a tool that is not in the available set (denial case). Its request
  log is kept and checked against the exported prompts.
- Environment: `COPILOT_PROVIDER_BASE_URL=http://127.0.0.1:<port>/v1`,
  `COPILOT_PROVIDER_TYPE=openai`, `COPILOT_PROVIDER_WIRE_API=completions`,
  `COPILOT_MODEL=mock-740`, no API key, the same disposable `COPILOT_HOME`,
  capture settings and OTel file path as Probe 0.
- Same argv filters as Probe 0. Two turns, 120 seconds each, one process at a
  time, no retry. Nothing leaves the machine; stop if any non-loopback
  connection is observed.
- Checks: `ORK740_RESPONSE` and any mock "reasoning" string absent from the
  exported file; the mock's received request matches the exported
  instructions/tools; the later request in a multi-message turn contains no
  response content in captured inputs; unavailable-tool call is refused.
- A pass qualifies the exclusion property for this transport only. The
  Copilot-hosted Probe 1 matrix still waits for quota and a new authorization.

## Remaining gates

1. Run Probe 2 if the owner approves its configuration.
2. After quota returns: one successful hosted turn to confirm exclusion
   against the real service, then the Probe 1 allow/deny matrix under a new
   authorization.
3. Establish the origin and connection behavior of `githubiq` and whether
   `--disable-builtin-mcps` suffices for a "no connectors" claim.
4. Observe skill-body delivery on invocation, which needs tool-output capture or
   a startup-injection design in #743.
