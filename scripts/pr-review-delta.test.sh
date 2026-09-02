#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/pr-review-delta.sh"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/orkworks-pr-review-delta.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

cd "$fixture"
git init -q
git config user.email test@example.invalid
git config user.name "PR review delta test"
mkdir -p apps/desktop crates/orkworksd docs
printf 'baseline\n' > apps/desktop/base.txt
printf 'baseline\n' > crates/orkworksd/base.txt
git add .
git commit -qm baseline
baseline="$(git rev-parse HEAD)"

for i in $(seq 1 8); do
  lines=62
  [ "$i" -le 4 ] && lines=63
  : > "apps/desktop/change-$i.txt"
  for line in $(seq 1 "$lines"); do
    printf 'line %s\n' "$line" >> "apps/desktop/change-$i.txt"
  done
done
git add .
git commit -qm "eight files and five hundred lines"
eight_files="$(git rev-parse HEAD)"

assert_delta() {
  expected="$1"
  from="$2"
  to="$3"
  actual="$("$helper" "$from" "$to")"
  [ "$actual" = "$expected" ] || {
    echo "expected '$expected', got '$actual'" >&2
    exit 1
  }
}

assert_delta "8 500" "$baseline" "$eight_files"

printf 'line 1\n' > apps/desktop/change-9.txt
git add .
git commit -qm "nine files and five hundred one lines"
nine_files="$(git rev-parse HEAD)"
assert_delta "9 501" "$baseline" "$nine_files"

printf 'documentation\n' > apps/desktop/README.md
git add .
git commit -qm "documentation only"
docs_only="$(git rev-parse HEAD)"
assert_delta "0 0" "$nine_files" "$docs_only"

printf 'workflow\n' > docs/irrelevant.txt
git add .
git commit -qm "irrelevant path"
irrelevant="$(git rev-parse HEAD)"
assert_delta "0 0" "$docs_only" "$irrelevant"

printf '\000\001\002' > crates/orkworksd/binary.bin
git add .
git commit -qm "binary file"
binary="$(git rev-parse HEAD)"
assert_delta "1 0" "$irrelevant" "$binary"

printf 'rename me\n' > apps/desktop/rename-old.txt
git add .
git commit -qm "rename source"
rename_source="$(git rev-parse HEAD)"
git mv apps/desktop/rename-old.txt apps/desktop/rename-new.txt
git commit -qm "rename file"
rename_target="$(git rev-parse HEAD)"
assert_delta "1 0" "$rename_source" "$rename_target"

echo "PR review delta fixtures passed"
