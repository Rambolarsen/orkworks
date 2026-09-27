---
name: troubleshooting-github-connectivity
description: Use when GitHub CLI (`gh`) requests fail, especially when output looks cached, the CLI reports an invalid token, or other sessions can reach GitHub.
---

# Troubleshooting GitHub Connectivity

Separate cached command output, network reachability, and authentication before changing credentials. A plausible issue list or `gh auth status` message alone does not prove the current session can reach GitHub or that its token is invalid.

Use the command wrapper required by the current repo and runtime. In this OrkWorks environment that may be `rtk proxy`; it bypasses RTK output handling, not network restrictions. If RTK is missing and the active instructions require it, report the wrapper problem. If no wrapper is required, use direct `gh` and `curl` commands.

## Diagnose in order

1. **Check whether output is fresh.** RTK may show cached output alongside a failed live request. Do not treat cached issues or PRs as current. Rerun the operation through the required raw-command path and read its exit status and error.
2. **Identify the target GitHub host.** Resolve the host selected by the failing command, accounting for `--hostname`, `-R`/`--repo`, `GH_REPO`, `GH_HOST`, and the local repository's remote URL. A repository selector can include a host as `[HOST/]OWNER/REPO`; do not infer the target from the local remote if the command overrides it. Probe that host's REST API rather than assuming `api.github.com`. `gh help environment` documents host and repository selectors. GitHub.com uses `https://api.github.com`. GitHub Enterprise Cloud with data residency (`SUBDOMAIN.ghe.com`) uses `https://api.SUBDOMAIN.ghe.com` ([GitHub docs](https://docs.github.com/en/enterprise-cloud@latest/admin/data-residency/about-github-enterprise-cloud-with-data-residency)). GitHub Enterprise Server uses `https://HOSTNAME/api/v3` ([GitHub's REST API URL guidance](https://docs.github.com/en/enterprise-server@3.22/rest/using-the-rest-api/getting-started-with-the-rest-api)).
3. **Check unauthenticated reachability.** For GitHub.com, probe without credentials:

   ```bash
   rtk proxy curl --connect-timeout 5 --max-time 10 -sS -o /dev/null \
     -w 'status=%{http_code} error=%{errormsg}\n' https://api.github.com
   ```

   Probe the selected API base's `/meta` endpoint: `https://api.github.com/meta` for GitHub.com, `https://api.SUBDOMAIN.ghe.com/meta` for Enterprise Cloud data residency, or `https://HOSTNAME/api/v3/meta` for Enterprise Server. If the current shell does not require an RTK wrapper, omit `rtk proxy` from the command. If DNS or connection fails, this command environment cannot currently reach the target. Treat `gh auth status` as inconclusive until connectivity works; do not ask the user to refresh credentials yet.
4. **Verify authentication only after reachability succeeds.** Use `rtk proxy gh api user --jq .login` in an RTK-required shell, or `gh api user --jq .login` if no wrapper is required; specify `--hostname <target-host>` when needed. A returned login confirms that this `gh` invocation can authenticate to that host. If GitHub responds with an authentication error, inspect which host, account, and configuration path the process selected (`GH_CONFIG_DIR`, `GH_HOST`, and whether the relevant token variable is set), without printing token values. `GH_TOKEN`/`GITHUB_TOKEN` apply to GitHub.com and `*.ghe.com`; GHES also supports `GH_ENTERPRISE_TOKEN`/`GITHUB_ENTERPRISE_TOKEN`. A `403` may indicate missing permission rather than an invalid credential. See [`gh help environment`](https://cli.github.com/manual/gh_help_environment) for precedence and details.
5. **Use an approved network-enabled path when the shell is isolated.** If the task expects GitHub access but this shell cannot resolve or connect, use an available network-enabled tool only when its access is authorized; follow its approval flow if required. Repeat the minimal read-only API request there. Report which environment failed and which succeeded.

## Guardrails

- Never print, log, copy, or ask the user to paste an access token. Check authentication-variable presence, not values.
- Do not infer token validity from stale output or from `gh auth status` while the target host is unreachable.
- Do not repeat login, refresh, logout, or account-switch operations until a reachable authenticated request confirms an authentication problem and the user authorizes the credential change.

## Example

`gh auth status` says “token is invalid,” but an unauthenticated probe of the selected API host fails with `Could not resolve host`. The current command environment has a DNS/connectivity problem; token validity is still unknown. Verify from an authorized network-enabled environment before asking for reauthentication.
