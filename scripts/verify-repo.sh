#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd -P)"
dry_run=0

if [ "${1:-}" = '--dry-run' ]; then
  dry_run=1
  shift
fi

if [ "$#" -ne 0 ]; then
  echo "usage: $0 [--dry-run]" >&2
  exit 2
fi

run_step() {
  local label="$1"
  shift

  printf '[verify] %s\n' "$label"
  if [ "$dry_run" -eq 1 ]; then
    if [ "${VERIFY_REPO_TEST_FAIL_STEP:-}" = "$label" ]; then
      echo "[verify] simulated failure at $label" >&2
      return 97
    fi
    printf '[verify] dry-run:'
    printf ' %q' "$@"
    printf '\n'
    return 0
  fi

  "$@"
}

run_in_directory() {
  local directory="$1"
  shift
  cd "$directory"
  "$@"
}

run_desktop_tests() {
  run_in_directory "$repo_root/apps/desktop" \
    node --experimental-strip-types --test tests/*.test.ts tests/*.test.mjs
}

run_desktop_build() {
  run_in_directory "$repo_root/apps/desktop" pnpm build
}

run_docs_build() {
  run_in_directory "$repo_root/docs" pnpm docs:build
}

run_step 'Rust formatting' \
  cargo fmt --manifest-path "$repo_root/crates/orkworksd/Cargo.toml" --check
run_step 'Rust build' \
  cargo build --manifest-path "$repo_root/crates/orkworksd/Cargo.toml"
run_step 'Rust tests' \
  cargo test --manifest-path "$repo_root/crates/orkworksd/Cargo.toml"
run_step 'Desktop type-check' \
  run_in_directory "$repo_root/apps/desktop" pnpm exec tsc --noEmit
run_step 'Desktop tests' run_desktop_tests
run_step 'Desktop build' run_desktop_build
run_step 'Docs build' run_docs_build
run_step 'Git diff check' git -C "$repo_root" diff --check
run_step 'Documentation currency' bash "$repo_root/scripts/doc-check.sh"
run_step 'Worktree currency' bash "$repo_root/.claude/hooks/worktree-check.sh"

echo '[verify] all repository checks passed'
