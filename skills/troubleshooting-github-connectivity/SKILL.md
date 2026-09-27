---
name: troubleshooting-github-connectivity
description: Use when GitHub CLI (`gh`) requests fail, especially when output looks cached, the CLI reports an invalid token, or other sessions can reach GitHub.
---

# Troubleshooting GitHub Connectivity

Separate cached command output, network reachability, and authentication before changing credentials. A plausible issue list or `gh auth status` message alone does not prove the current session can reach GitHub or that its token is invalid.

Use the command wrapper required by the current repo and runtime. In this OrkWorks environment that may be `rtk proxy`; it bypasses RTK output handling, not network restrictions. If RTK is missing and the active instructions require it, report the wrapper problem. If no wrapper is required, use direct `gh` and `curl` commands.

## Diagnose in order

1. **Check whether output is fresh.** RTK may show cached output alongside a failed live request. Do not treat cached issues or PRs as current. Rerun the operation through the required raw-command path and read its exit status and error.
2. **Identify the target GitHub host.** Use an explicit `--hostname`, `GH_HOST`, or the host in the target repository's remote URL. Probe that host's REST API; do not use `api.github.com` to infer reachability when `gh` targets another host. GitHub.com uses `https://api.github.com`. GitHub Enterprise Server REST API URLs use `https://HOSTNAME/api/v3`; see [GitHub's REST API URL guidance](https://docs.github.com/en/enterprise-server@3.22/rest/using-the-rest-api/getting-started-with-the-rest-api).
3. **Check unauthenticated reachability.** For GitHub.com, probe without credentials:

   ```bash
   rtk proxy curl --connect-timeout 5 --max-time 10 -sS -o /dev/null \
     -w 'status=%{http_code} error=%{errormsg}\n' https://api.github.com
   ```

   For GitHub Enterprise, substitute the target host's REST API base (usually `https://HOSTNAME/api/v3/meta`). If the current shell does not require an RTK wrapper, omit `rtk proxy` from the command. If DNS or connection fails, this command environment cannot currently reach the target. Treat `gh auth status` as inconclusive until connectivity works; do not ask the user to refresh credentials yet.
4. **Verify authentication only after reachability succeeds.** Use `rtk proxy gh api user --jq .login` in an RTK-required shell, or `gh api user --jq .login` if no wrapper is required; specify `--hostname <target-host>` when needed. A returned login confirms that this `gh` invocation can authenticate to that host. If GitHub responds with an authentication error, inspect which host, account, and configuration path the process selected (`GH_CONFIG_DIR` and whether `GH_TOKEN`/`GITHUB_TOKEN` are set), without printing token values. A `403` may indicate missing permission rather than an invalid credential.
5. **Use an approved network-enabled path when the shell is isolated.** If the task expects GitHub access but this shell cannot resolve or connect, use an available network-enabled tool only when its access is authorized; follow its approval flow if required. Repeat the minimal read-only API request there. Report which environment failed and which succeeded.

## Guardrails

- Never print, log, copy, or ask the user to paste an access token. Check authentication-variable presence, not values.
- Do not infer token validity from stale output or from `gh auth status` while the target host is unreachable.
- Do not repeat login, refresh, logout, or account-switch operations until a reachable authenticated request confirms an authentication problem and the user authorizes the credential change.

## Example

`gh auth status` says “token is invalid,” but an unauthenticated probe of the selected API host fails with `Could not resolve host`. The current command environment has a DNS/connectivity problem; token validity is still unknown. Verify from an authorized network-enabled environment before asking for reauthentication.
