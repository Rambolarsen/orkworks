#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/codex-api-diagnostics.sh"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/orkworks-codex-api-diagnostics.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/bin"
cat > "$fixture/bin/codex" <<'EOF'
#!/usr/bin/env bash
printf 'codex-cli 9.9.9\n'
EOF
chmod +x "$fixture/bin/codex"

output="$(
  cd "$repo_root"
  PATH="$fixture/bin:$PATH" \
    OPENAI_API_KEY="do-not-print-openai-key" \
    CODEX_API_KEY="do-not-print-codex-key" \
    CODEX_HOME="$fixture/codex-home" \
    bash "$helper"
)"

grep -Fq 'Codex API diagnostics' <<<"$output"
grep -Fq 'Codex CLI: codex-cli 9.9.9' <<<"$output"
grep -Fq 'OPENAI_API_KEY: present' <<<"$output"
grep -Fq 'CODEX_API_KEY: present' <<<"$output"
grep -Fq "CODEX_HOME: $fixture/codex-home" <<<"$output"
grep -Fq "Repository: $repo_root" <<<"$output"
grep -Fq 'Next step: preserve the exact error' <<<"$output"

if grep -Fq 'do-not-print-' <<<"$output"; then
  echo 'diagnostics leaked an API key value' >&2
  exit 1
fi

default_home_output="$(
  cd "$repo_root"
  PATH="$fixture/bin:$PATH" \
    HOME="$fixture/home" \
    CODEX_HOME= \
    bash "$helper"
)"
grep -Fq "CODEX_HOME: $fixture/home/.codex" <<<"$default_home_output"

echo 'Codex API diagnostics fixtures passed'
