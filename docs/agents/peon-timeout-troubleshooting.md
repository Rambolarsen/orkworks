# Peon timeout troubleshooting

Use this runbook when Peon reports that provider inference timed out or the
session detail panel shows a Peon timeout warning. The recovery applies to the
current session only; do not resume or reopen another session to bypass the
problem.

## Collect safe diagnostics

Run the repository-local, read-only helper from the repository root:

```bash
bash scripts/peon-timeout-diagnostics.sh
```

The helper reports repository and sidecar reachability facts, whether relevant
Peon environment variables are set, and whether the debug sidecar executable
exists. It never prints `ORKWORKS_REPORT_TOKEN`, provider credentials, or
provider configuration contents.

If `ORKWORKS_PORT` is present, `Sidecar health` and `Provider registry` show
the HTTP status returned by the running sidecar. If the port is absent, the
checks are skipped; this is expected when the desktop app is not running.

## Recover the active provider

1. If the sidecar is unavailable while OrkWorks is running, restart OrkWorks
   once and rerun the helper. Do not loop restarts.
2. Open Settings → Model providers and inspect the applied provider and model.
   Peon uses that applied pair for session inference.
3. Verify the provider connection. For Ollama, confirm the configured endpoint
   is reachable and the selected model is installed. For a hosted provider,
   confirm its configured authentication and model access without pasting
   credentials into a prompt or log.
4. Apply the verified provider/model, save it, and retry the current session
   once. If the timeout persists, preserve the helper output and the exact
   diagnostic message for follow-up.

`PEON_TIMEOUT` is retained only as a legacy environment variable and does not
control current session inference. Do not set `PEON_TIMEOUT` as a timeout fix.
`PEON_IDLE_TIMEOUT` controls when terminal silence is classified as idle; it
does not increase the provider request timeout.

For a separate Codex CLI message such as `Error in Codex API`, use the
[Codex API troubleshooting runbook](codex-api-troubleshooting.md) and its
diagnostic helper. Keep that diagnosis separate from a Peon provider timeout.
