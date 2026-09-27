---
name: troubleshooting-github-connectivity
description: Use when GitHub CLI (`gh`) commands fail due to connectivity or transport problems, especially with stale output, invalid-token messages, or when other sessions can reach GitHub.
---

# Troubleshooting GitHub CLI API Connectivity

Distinguish cached output, reachability, authentication, and permissions before changing credentials; cached results and `gh auth status` alone prove neither reachability nor token validity.

Use this repo/runtime's required wrapper. `rtk proxy` bypasses RTK output handling, not network restrictions. If required RTK is missing, report that; otherwise use direct `gh`/`curl`.

## Diagnose in order

1. **Check freshness.** RTK may show cached output beside a failed live request. Rerun via the required raw-command path and read its exit status and error.
2. **Identify the target host.** Selection varies by subcommand. `gh api` defaults to `github.com`; `--hostname`/`GH_HOST` choose its API host, while `GH_REPO` supplies `{owner}`/`{repo}` only ([`gh api` manual](https://cli.github.com/manual/gh_api)). For other commands, check `--help` and local repo rules. API bases: GitHub.com `https://api.github.com`; Enterprise Cloud data residency `https://api.SUBDOMAIN.ghe.com` ([GitHub docs](https://docs.github.com/en/enterprise-cloud@latest/admin/data-residency/about-github-enterprise-cloud-with-data-residency)); Enterprise Server `https://HOSTNAME/api/v3` ([REST guidance](https://docs.github.com/en/enterprise-server@3.22/rest/using-the-rest-api/getting-started-with-the-rest-api)).
3. **Check the failing transport.** REST probes do not test Git transport for clone/checkout. Inspect `git_protocol` or URL scheme; test the credential-free target with `git -c credential.interactive=false -c core.sshCommand='ssh -o BatchMode=yes' ls-remote <URL>`. The configured helper handles HTTPS; for SSH inspect agent/key and host/port. Disable helper prompts or report unavailable. Never expose embedded tokens ([Git credential settings](https://git-scm.com/docs/git-config#Documentation/git-config.txt-credentialinteractive)).

   For API requests, probe the selected host's `/meta` endpoint without credentials. Replace the sample URL and prefix the wrapper required by the runtime.

   ```bash
   curl --connect-timeout 5 --max-time 10 -sS -D - -o /dev/null -w 'status=%{http_code} error=%{errormsg}\n' https://api.github.com/meta
   ```

   ```powershell
   curl.exe --connect-timeout 5 --max-time 10 -sS -D - -o NUL -w "status=%{http_code} error=%{errormsg}" https://api.github.com/meta
   ```

   Target `/meta`: `https://api.github.com/meta`, `https://api.SUBDOMAIN.ghe.com/meta`, or `https://HOSTNAME/api/v3/meta`. DNS/connection failure means this command environment cannot reach the target; `gh auth status` remains inconclusive until it can.
4. **Verify authentication after reachability.** Run `gh api user --jq .login` through the required wrapper (`--hostname <host>` if needed). A login confirms authentication for this invocation. On error, inspect selected host/account/config (`GH_CONFIG_DIR`, `GH_HOST`) and token-variable presence, never values. `GH_TOKEN`/`GITHUB_TOKEN` apply to GitHub.com and `*.ghe.com`; GHES also supports `GH_ENTERPRISE_TOKEN`/`GITHUB_ENTERPRISE_TOKEN` ([environment docs](https://cli.github.com/manual/gh_help_environment)).
5. **Separate permission errors from rate limits.** `403` can mean missing permission or an exhausted limit; `429` can indicate rate limiting. For a safe read-only API request, inspect status and headers with `gh api --include --silent <endpoint> --hostname <host>` or `curl -D -`; never use `gh api --verbose`, which prints the full request. Check the response message and `x-ratelimit-remaining`/`retry-after` before changing credentials or scopes ([GitHub rate-limit guidance](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api#exceeding-the-rate-limit)).
6. **Use an authorized network-enabled path if isolated.** Retry the minimal read-only request and report which environment failed and which succeeded.

## Guardrails

- Never print, log, copy, or ask the user to paste tokens; check variable presence only.
- Do not infer token validity from stale output or `gh auth status` while unreachable. Do not repeat login, refresh, logout, or account-switch operations until a reachable authenticated request confirms an auth problem and the user authorizes the credential change.
