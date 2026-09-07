#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/peon-timeout-diagnostics.sh"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/orkworks-peon-timeout-diagnostics.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

mkdir -p "$fixture/bin"
cat > "$fixture/bin/curl" <<'EOF'
#!/usr/bin/env bash
case "$*" in
  *"/health"*) printf '200' ;;
  *"/providers"*) printf '200' ;;
  *) printf '503' ;;
esac
EOF
chmod +x "$fixture/bin/curl"

output="$(
  cd "$repo_root"
  PATH="$fixture/bin:$PATH" \
    ORKWORKS_PORT=45123 \
    ORKWORKS_REPORT_TOKEN=do-not-print-report-token \
    PEON_ENABLED=true \
    PEON_TIMEOUT=90 \
    bash "$helper"
)"

grep -Fq 'Peon timeout diagnostics' <<<"$output"
grep -Fq 'Sidecar health: HTTP 200' <<<"$output"
grep -Fq 'Provider registry: HTTP 200' <<<"$output"
grep -Fq 'PEON_TIMEOUT: set (legacy; it does not control session inference)' <<<"$output"
grep -Fq 'Next step: docs/agents/peon-timeout-troubleshooting.md' <<<"$output"

if grep -Fq 'do-not-print-report-token' <<<"$output"; then
  echo 'diagnostics leaked the report token' >&2
  exit 1
fi

without_sidecar="$(
  cd "$repo_root"
  PATH="$fixture/bin:$PATH" \
    ORKWORKS_PORT= \
    PEON_TIMEOUT= \
    bash "$helper"
)"

grep -Fq 'Sidecar health: not checked (ORKWORKS_PORT is absent)' <<<"$without_sidecar"
grep -Fq 'PEON_TIMEOUT: absent (legacy; do not set it for current provider timeouts)' <<<"$without_sidecar"

runbook="$repo_root/docs/agents/peon-timeout-troubleshooting.md"
grep -Fq 'scripts/peon-timeout-diagnostics.sh' "$runbook"
grep -Fq 'Do not set `PEON_TIMEOUT`' "$runbook"

echo 'Peon timeout diagnostics fixtures passed'
