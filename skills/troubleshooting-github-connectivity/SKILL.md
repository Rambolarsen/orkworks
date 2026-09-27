---
name: troubleshooting-github-connectivity
description: Use when GitHub CLI (`gh`) requests fail, especially when output looks cached, the CLI reports an invalid token, or other sessions can reach GitHub.
---

# Troubleshooting GitHub Connectivity

Separate cached command output, network reachability, and authentication before changing credentials. A plausible issue list or `gh auth status` message alone does not prove the current session can reach GitHub or that its token is invalid.

## Diagnose in order

1. **Check whether output is fresh.** RTK may show a cached result alongside a failed live request. Do not treat cached issues or PRs as current. Run the command through `rtk proxy` to bypass RTK's output handling, then read the complete exit status and error. `rtk proxy` does not bypass sandbox, DNS, proxy, or firewall restrictions.
2. **Check unauthenticated reachability.** Probe `https://api.github.com` without credentials, for example:

   ```bash
   rtk proxy curl --connect-timeout 5 --max-time 10 -sS -o /dev/null \
     -w 'status=%{http_code} error=%{errormsg}\n' https://api.github.com
   ```

   A DNS or connection error means this command environment cannot currently reach GitHub. Treat `gh auth status` results as inconclusive until connectivity works; do not ask the user to refresh credentials yet. `rtk proxy` only bypasses RTK handling, not network isolation.
3. **Verify authentication only after reachability succeeds.** Use the minimal read-only request `rtk proxy gh api user --jq .login`. A returned login confirms that this `gh` invocation can authenticate. If GitHub responds with an authentication error, inspect which host, account, and configuration path the process selected (`GH_CONFIG_DIR` and whether `GH_TOKEN`/`GITHUB_TOKEN` are set), without printing token values. A `403` may indicate missing permission rather than an invalid credential.
4. **Use an approved network-enabled path when the shell is isolated.** If the task expects GitHub access but this shell cannot resolve or connect, use an available network-enabled tool only when its access is authorized; follow its approval flow if required. Repeat the minimal read-only API request there. Report which environment failed and which succeeded.

## Guardrails

- Never print, log, copy, or ask the user to paste an access token. Check environment-variable presence, not values.
- Do not infer token validity from stale RTK output or from `gh auth status` while GitHub is unreachable.
- Do not repeat login, refresh, logout, or account-switch operations until a reachable authenticated request confirms an authentication problem and the user authorizes the credential change.

## Example

`gh auth status` says “token is invalid,” but an unauthenticated API probe fails with `Could not resolve host: api.github.com`. The current command environment has a DNS/connectivity problem; token validity is still unknown. Verify from an authorized network-enabled environment before asking for reauthentication.
