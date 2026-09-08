---
type: Troubleshooting Guide
title: Codex API troubleshooting
description: Safe diagnostics and escalation steps for generic Codex API failures.
tags: [codex, troubleshooting, diagnostics, api-failures]
status: stable
---

# Codex API troubleshooting

Use this runbook when Codex reports a generic API failure such as `Error in
Codex API`. The message alone is not enough to identify the cause: the same
surface can represent authentication, permissions, quota, rate limits,
request validation, service errors, or local connectivity problems.

## First response

Run the repository-local, read-only diagnostic helper into a securely created
temporary file:

```bash
diagnostics_file="$(mktemp "${TMPDIR:-/tmp}/orkworks-codex-api-diagnostics.XXXXXX")"
bash scripts/codex-api-diagnostics.sh > "$diagnostics_file"
printf 'Diagnostics saved to %s\n' "$diagnostics_file"
```

The output records the Codex CLI version, repository/branch, config-file
presence, and whether relevant environment variables are set. It never prints
API-key values or config contents. Do not replace it with `env`, a config dump,
or a command that includes a key in shell history.

Before retrying, preserve:

- the exact error text and any HTTP status or `error.code`
- the request ID, if one was returned
- the model, organization/project, and auth mode in use
- the timestamp and timezone
- whether the failure was reproducible or a single transient attempt

Do not blindly loop retries, alter unrelated configuration, or resume/reopen
another session to work around this error. This is a diagnosis of the current
session/tool invocation only.

## Triage

| Signal | Likely cause | Next action |
| --- | --- | --- |
| `401`, unauthorized, invalid API key | Authentication or expired credentials | Re-authenticate or refresh the configured credential, then retry once. Never paste the credential into a prompt, issue, or log. |
| `403`, permission denied, model unavailable | Organization, project, workspace, or model access | Verify the selected organization/project and that the account may use the requested model. |
| `429` with a rate-limit message | Request/token burst or concurrent usage | Reduce the burst, honor `Retry-After` when present, and retry at most 3 attempts within 60 seconds total. Do not start another session just to bypass the limit. |
| `429` with `insufficient_quota`, `credit_balance_exhausted`, or a spend/usage-limit code | Credits, quota, or a hard spend limit | Check the relevant billing/limits owner or wait for the documented reset; repeated retries do not restore access. |
| `400` or `422` with a request-validation message | Invalid model, parameter, input, or context size | Preserve the full message and inspect the request-producing tool/config. Change only the named invalid input. |
| `5xx`, `server_error`, timeout, or connection failure | Service incident or local network/TLS/proxy problem | Check service status and local connectivity, then retry once after a short delay. Escalate if it persists. |

The OpenAI Responses API represents failed generation with an error object that
includes a code and message; use those fields when available rather than
classifying every failure as a generic API error. See the [Responses API
reference](https://developers.openai.com/api/reference/cli/resources/responses/methods/create).
For 429s, distinguish temporary rate limits from credits, quota, and spend
limits using OpenAI's [rate-limit guidance](https://help.openai.com/en/articles/5955604-how-can-i-solve-429-too-many-requests-errors)
and [usage-limit guidance](https://help.openai.com/en/articles/6614457-why-am-i-getting-an-error-message-stating-that-i-ve-reached-my-usage-limit).

## Escalation bundle

If the error remains after the matching next action, attach the diagnostic
output and the preserved metadata above after removing sensitive values. Include
the exact command or workflow step that failed, but redact API keys, cookies,
authorization headers, prompt contents, and private repository data.
