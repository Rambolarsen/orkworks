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

## Remaining gates

1. Re-run once quota is available, with one successful turn, to show responses
   and reasoning are absent from the exported file and from later inputs.
2. Add a scoped-rule case (cwd in `scope/`) and a referenced-skill-resource case.
3. Investigate `github-mcp-server`/`githubiq` discovery under
   `--disable-builtin-mcps`.
4. Only then approve the Probe 1 allow/deny matrix in a new authorization.
