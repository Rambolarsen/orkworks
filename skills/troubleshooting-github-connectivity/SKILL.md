---
name: troubleshooting-github-connectivity
description: Use when GitHub CLI (`gh`) API requests fail, especially when output looks cached, the CLI reports an invalid token, or other sessions can reach GitHub.
---

# Troubleshooting GitHub CLI API Connectivity

Separate cached output, network reachability, authentication, and permissions before changing credentials. Cached results or `gh auth status` alone do not prove current reachability or token validity.

Use the wrapper required by the current repo/runtime. Here that may be `rtk proxy`, which bypasses RTK output handling but not network restrictions. If required RTK is missing, report the tooling issue; otherwise use direct `gh` and `curl`.

## Diagnose in order

1. **Check freshness.** RTK may show cached output beside a failed live request. Rerun via the required raw-command path and read its exit status and error.
2. **Identify the target host.** Follow the failing command's selectors: `--hostname`, `-R`/`--repo`, `GH_REPO`, `GH_HOST`, and local remote. A repo selector may include `[HOST/]OWNER/REPO`; honor overrides instead of assuming the local remote or `api.github.com`. `gh help environment` documents selectors. API bases: GitHub.com `https://api.github.com`; Enterprise Cloud data residency `https://api.SUBDOMAIN.ghe.com` ([GitHub docs](https://docs.github.com/en/enterprise-cloud@latest/admin/data-residency/about-github-enterprise-cloud-with-data-residency)); Enterprise Server `https://HOSTNAME/api/v3` ([REST API guidance](https://docs.github.com/en/enterprise-server@3.22/rest/using-the-rest-api/getting-started-with-the-rest-api)).
3. **Check the failing transport.** REST probes do not test Git transport for `gh repo clone` or `gh pr checkout`. For those, inspect `gh config get git_protocol --host <host>` or the explicit URL scheme, then test the same remote with `git ls-remote <same-URL>`. Diagnose SSH and HTTPS separately.

   For API requests, probe the selected host's `/meta` endpoint without credentials. Replace the sample URL and prefix the wrapper required by the runtime.

   ```bash
   curl --connect-timeout 5 --max-time 10 -sS -o /dev/null -w 'status=%{http_code} error=%{errormsg}\n' https://api.github.com/meta
   ```

   ```powershell
   curl.exe --connect-timeout 5 --max-time 10 -sS -o NUL -w "status=%{http_code} error=%{errormsg}" https://api.github.com/meta
   ```

   Target `/meta`: `https://api.github.com/meta`, `https://api.SUBDOMAIN.ghe.com/meta`, or `https://HOSTNAME/api/v3/meta`. DNS/connection failure means this command environment cannot reach the target; `gh auth status` remains inconclusive until it can.
4. **Verify authentication after reachability.** Run `gh api user --jq .login` (through the required wrapper and with `--hostname <host>` if needed). A login confirms this invocation authenticated. For auth errors, inspect selected host/account/config (`GH_CONFIG_DIR`, `GH_HOST`) and whether the relevant token variable is set, never its value. `GH_TOKEN`/`GITHUB_TOKEN` apply to GitHub.com and `*.ghe.com`; GHES also supports `GH_ENTERPRISE_TOKEN`/`GITHUB_ENTERPRISE_TOKEN`. See [`gh help environment`](https://cli.github.com/manual/gh_help_environment).
5. **Separate permission errors from rate limits.** `403` can mean missing permission or an exhausted limit; `429` can indicate rate limiting. Inspect the response and `x-ratelimit-remaining`/`retry-after` headers before changing credentials or scopes ([GitHub rate-limit guidance](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api#exceeding-the-rate-limit)).
6. **Use an authorized network-enabled path if isolated.** Retry the minimal read-only request there and report which environment failed and which succeeded.

## Guardrails

- Never print, log, copy, or ask the user to paste tokens; check variable presence only.
- Do not infer token validity from stale output or `gh auth status` while unreachable. Do not repeat login, refresh, logout, or account-switch operations until a reachable authenticated request confirms an auth problem and the user authorizes the credential change.
