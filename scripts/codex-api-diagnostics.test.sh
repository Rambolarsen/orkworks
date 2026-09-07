#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/codex-api-diagnostics.sh"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/orkworks-codex-api-diagnostics.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/bin"
cat > "$fixture/bin/codex" <<'EOF'
#!/usr/bin/env bash
if [ "${CODEX_FAKE_OUTPUT:-}" = unsafe ]; then
  printf 'codex-cli 9.9.9 do-not-print-cli-secret\n'
elif [ "${CODEX_FAKE_OUTPUT:-}" = wrong_prefix ]; then
  printf 'wrapper 9.9.9\n'
else
  printf 'codex-cli 9.9.9\n'
fi
EOF
chmod +x "$fixture/bin/codex"
mkdir -p "$fixture/codex-home"
printf 'do-not-print-config-secret\n' > "$fixture/codex-home/config.toml"
printf 'do-not-print-auth-secret\n' > "$fixture/codex-home/auth.json"

output="$(
  cd "$repo_root"
  PATH="$fixture/bin:$PATH" \
    OPENAI_API_KEY="do-not-print-openai-key" \
    CODEX_API_KEY="do-not-print-codex-key" \
    CODEX_HOME="$fixture/codex-home" \
    bash "$helper"
)"

grep -Fq 'Codex API diagnostics' <<<"$output"
grep -Fq 'Codex CLI: version 9.9.9' <<<"$output"
grep -Fq 'OPENAI_API_KEY: present' <<<"$output"
grep -Fq 'CODEX_API_KEY: present' <<<"$output"
grep -Fq "CODEX_HOME: $fixture/codex-home" <<<"$output"
grep -Fq 'Codex auth file: present' <<<"$output"
grep -Fq "Repository: $repo_root" <<<"$output"
grep -Fq 'Next step: preserve the exact error' <<<"$output"

if grep -Fq 'do-not-print-' <<<"$output"; then
  echo 'diagnostics leaked an API key value' >&2
  exit 1
fi

unsafe_output="$(
  cd "$repo_root"
  PATH="$fixture/bin:$PATH" \
    CODEX_FAKE_OUTPUT=unsafe \
    CODEX_HOME="$fixture/codex-home" \
    bash "$helper"
)"
grep -Fq 'Codex CLI: available (version output not recognized)' <<<"$unsafe_output"
if grep -Fq 'do-not-print-' <<<"$unsafe_output"; then
  echo 'diagnostics echoed unsafe CLI or config content' >&2
  exit 1
fi

wrong_prefix_output="$(
  cd "$repo_root"
  PATH="$fixture/bin:$PATH" \
    CODEX_FAKE_OUTPUT=wrong_prefix \
    bash "$helper"
)"
grep -Fq 'Codex CLI: available (version output not recognized)' <<<"$wrong_prefix_output"

default_home_output="$(
  cd "$repo_root"
  PATH="$fixture/bin:$PATH" \
    HOME="$fixture/home" \
    CODEX_HOME= \
    bash "$helper"
)"
grep -Fq "CODEX_HOME: $fixture/home/.codex" <<<"$default_home_output"
grep -Fq 'Codex auth file: absent' <<<"$default_home_output"

runbook="$repo_root/docs/agents/codex-api-troubleshooting.md"
grep -Fq 'mktemp' "$runbook"
grep -Fq 'at most 3 attempts' "$runbook"

echo 'Codex API diagnostics fixtures passed'
