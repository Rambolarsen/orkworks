---
name: troubleshooting-github-connectivity
description: Use before GitHub issue, pull-request, or remote API work to select an authorized access path, and when GitHub CLI (`gh`) requests fail due to connectivity or transport problems, especially with stale output, invalid-token messages, or when other sessions can reach GitHub.
---

# Troubleshooting GitHub CLI API Connectivity

Distinguish cached output, reachability, authentication, and permissions before changing credentials; cached results and `gh auth status` alone prove neither reachability nor token validity.

## Before the first GitHub request

Choose the access path based on the requested operation and actual reachability. Use `gh` when shell networking works and its credentials are authorized for that action. When shell networking is blocked or `gh` is unavailable, use a configured GitHub connector only for operations it authorizes. Confirm the exact target with a read-only request through the selected path; a successful read establishes access only for that identity, host, and resource, not write permission. A sandbox or shell network block is not an authentication or permission failure. For mutations, choose a path known to be authorized for that action when possible. If a write is denied, stop using that path; use another only when it is independently known to be authorized, never to evade the denial.

This preflight is only a path-selection and target-access check. Do not run failure diagnostics when the selected path can already read the requested target.

When active instructions require a wrapper, use it for every probe; here that is `rtk proxy <command>`. It bypasses RTK output handling, not network restrictions. If required RTK is missing, report it rather than using an unwrapped fallback. Use direct commands only when no wrapper is required.

## Diagnose in order

1. **Check freshness.** RTK may show cached output beside a failed live request. Rerun via the required raw-command path and read its exit status and error.
2. **Identify the target host.** For `gh api`, `--hostname`/`GH_HOST` select the API host (default `github.com`); `GH_REPO` supplies endpoint placeholders only, even with `[HOST/]OWNER/REPO` ([manual](https://cli.github.com/manual/gh_api)). For other commands, check `--help` and local repo rules. `gh --hostname` takes a hostname, not a URL. Host / curl API base pairs: GitHub.com `github.com` / `https://api.github.com`; Enterprise Cloud `SUBDOMAIN.ghe.com` / `https://api.SUBDOMAIN.ghe.com` ([GitHub docs](https://docs.github.com/en/enterprise-cloud@latest/admin/data-residency/about-github-enterprise-cloud-with-data-residency)); GHES `HOSTNAME` / `https://HOSTNAME/api/v3` ([REST guidance](https://docs.github.com/en/enterprise-server@3.22/rest/using-the-rest-api/getting-started-with-the-rest-api)).
3. **Check the failing transport.** REST probes do not test Git clone/checkout transport. Inspect `git_protocol` or URL scheme, then test the credential-free target with `git -c credential.interactive=false ls-remote <URL>`. HTTPS uses the configured helper; disable helper prompts or report it unavailable. For SSH inspect the effective `core.sshCommand`/`GIT_SSH_COMMAND`, agent/key, and host/port; preserve SSH settings and use `BatchMode=yes` and `ConnectTimeout=5` (or client equivalents). Never expose embedded tokens ([Git credential settings](https://git-scm.com/docs/git-config#Documentation/git-config.txt-credentialinteractive)). For `gh release download`, the asset request may redirect to a separate host; inspect its `Location` and test that host too. A successful API probe does not test asset delivery ([CLI manual](https://cli.github.com/manual/gh_release_download), [asset API](https://docs.github.com/en/rest/releases/assets#get-a-release-asset)).

   For API requests, probe the selected host's `/meta` endpoint without credentials. Replace the sample URL and prefix the wrapper required by the runtime.

   ```bash
   curl --connect-timeout 5 --max-time 10 -sS -D - -o /dev/null -w 'status=%{http_code} error=%{errormsg}\n' https://api.github.com/meta
   ```

   ```powershell
   curl.exe --connect-timeout 5 --max-time 10 -sS -D - -o NUL -w "status=%{http_code} error=%{errormsg}" https://api.github.com/meta
   ```

   Target `/meta`: `https://api.github.com/meta`, `https://api.SUBDOMAIN.ghe.com/meta`, or `https://HOSTNAME/api/v3/meta`. DNS/connection failure means this command environment cannot reach the target; `gh auth status` remains inconclusive until it can.
4. **Verify authentication after reachability.** For a user token, run `gh api user --jq .login` through the required wrapper (`--hostname <host>` if needed). A login confirms this invocation. For Actions `GITHUB_TOKEN` or another installation token, `/user` may be unsupported; verify with the same safe repository endpoint the token needs. On error, inspect selected host/account/config (`GH_CONFIG_DIR`, `GH_HOST`) and token-variable presence, never values. `GH_TOKEN`/`GITHUB_TOKEN` apply to GitHub.com and `*.ghe.com`; GHES also supports `GH_ENTERPRISE_TOKEN`/`GITHUB_ENTERPRISE_TOKEN` ([environment docs](https://cli.github.com/manual/gh_help_environment), [Actions tokens](https://docs.github.com/en/actions/security-for-github-actions/security-guides/automatic-token-authentication)).
5. **Separate permission errors from rate limits.** `403` can mean missing permission or a limit; `429` can indicate rate limiting. Inspect the original safe request with the same host and authentication: for API calls, `gh api --include <same-endpoint> --hostname <host>` shows status, headers, and error body. Check `x-ratelimit-resource`, `x-ratelimit-remaining`, and `retry-after`; inspect the body locally if needed, without logging or sharing it. Do not replace the failing endpoint with an unrelated one, use unauthenticated `curl` to infer an authenticated token's quota, or use `gh api --verbose` (prints the full request). For non-API commands, inspect that command's response/context ([rate-limit guidance](https://docs.github.com/en/rest/using-the-rest-api/rate-limits-for-the-rest-api#exceeding-the-rate-limit)).
6. **Use an authorized network-enabled path if isolated.** Retry the minimal read-only request and report which environment failed and which succeeded.

## Guardrails

- Never print, log, copy, or ask the user to paste tokens; check variable presence only.
- Do not infer token validity from stale output or `gh auth status` while unreachable. Do not repeat login, refresh, logout, or account-switch operations until a reachable authenticated request confirms an auth problem and the user authorizes the credential change.
