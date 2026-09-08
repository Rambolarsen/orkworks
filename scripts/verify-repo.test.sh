#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/verify-repo.sh"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/orkworks-verify-repo.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

output="$(
  cd "$repo_root"
  bash "$helper" --dry-run
)"

expected_steps=(
  'Rust formatting'
  'Rust build'
  'Rust tests'
  'Desktop type-check'
  'Desktop tests'
  'Desktop build'
  'Docs build'
  'Git diff check'
  'Documentation currency'
  'Worktree currency'
)

previous_line=0
for step in "${expected_steps[@]}"; do
  line="$(grep -nF "[verify] $step" <<<"$output" | cut -d: -f1)"
  test -n "$line"
  test "$line" -gt "$previous_line"
  previous_line="$line"
done

if VERIFY_REPO_TEST_FAIL_STEP='Desktop build' bash "$helper" --dry-run \
    >"$fixture/output" 2>&1; then
  echo 'verify-repo unexpectedly ignored a simulated step failure' >&2
  exit 1
fi
failure_output="$(<"$fixture/output")"
grep -Fq '[verify] simulated failure at Desktop build' <<<"$failure_output"
if grep -Fq '[verify] Docs build' <<<"$failure_output"; then
  echo 'verify-repo continued after a failed step' >&2
  exit 1
fi

echo 'Repository verification helper fixtures passed'
