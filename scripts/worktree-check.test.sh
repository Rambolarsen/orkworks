#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
hook="$repo_root/.claude/hooks/worktree-check.sh"
fixture="$(mktemp -d)"
fixture="$(cd "$fixture" && pwd -P)"
trap 'rm -rf "$fixture"' EXIT
repo="$fixture/repo"
mkdir "$fixture/bin"

git init -q -b main "$repo"
printf 'baseline\n' > "$repo/tracked"
git -C "$repo" add tracked
git -C "$repo" -c user.name=Test -c user.email=test@example.invalid \
  -c core.hooksPath=/dev/null commit -qm baseline --allow-empty
git -C "$repo" worktree add -qb merged "$fixture/merged"
for dirty in unstaged staged untracked; do
  git -C "$repo" worktree add -qb "$dirty" "$fixture/$dirty checkout"
done
printf 'changed\n' >> "$fixture/unstaged checkout/tracked"
printf 'changed\n' >> "$fixture/staged checkout/tracked"
git -C "$fixture/staged checkout" add tracked
printf 'new\n' > "$fixture/untracked checkout/new-file"
git -C "$repo" worktree add -qb unavailable "$fixture/unavailable"
mv "$fixture/unavailable" "$fixture/moved"
git -C "$repo" worktree add -qb fresh "$fixture/fresh"
git -C "$fixture/fresh" -c user.name=Test -c user.email=test@example.invalid \
  -c core.hooksPath=/dev/null commit -qm fresh --allow-empty
git -C "$repo" worktree add -qb stale "$fixture/stale"
GIT_AUTHOR_DATE='2000-01-01T00:00:00Z' GIT_COMMITTER_DATE='2000-01-01T00:00:00Z' \
  git -C "$fixture/stale" -c user.name=Test -c user.email=test@example.invalid \
  -c core.hooksPath=/dev/null commit -qm stale --allow-empty

cat > "$fixture/bin/gh" <<'GH'
#!/usr/bin/env bash
set -euo pipefail
while [ "$#" -gt 0 ]; do
  if [ "$1" = '--head' ]; then
    printf '%s\n' "$2" >> "$WORKTREE_TEST_LOOKUPS"
    break
  fi
  shift
done
case "$WORKTREE_TEST_PR" in
  open) printf '1\n' ;;
  absent) printf '0\n' ;;
  denied) printf '0\n'; exit 1 ;;
esac
GH
chmod +x "$fixture/bin/gh"
export PATH="$fixture/bin:$PATH"
export WORKTREE_TEST_LOOKUPS="$fixture/lookups"
cd "$repo"

run_check() {
  : > "$WORKTREE_TEST_LOOKUPS"
  output="$(WORKTREE_TEST_PR="$1" bash "$hook" "${@:2}")"
}

assert_contains() {
  if [[ "$output" != *"$1"* ]]; then
    printf 'Expected %s in worktree report. Got:\n%s\n' "$1" "$output" >&2
    exit 1
  fi
}

assert_omits() {
  if [[ "$output" == *"$1"* ]]; then
    printf 'Unexpected %s in worktree report:\n%s\n' "$1" "$output" >&2
    exit 1
  fi
}

assert_lookups() {
  local actual
  actual="$(cat "$WORKTREE_TEST_LOOKUPS")"
  if [[ "$actual" != "$1" ]]; then
    printf 'Expected PR lookups for [%s], got [%s]\n' "$1" "$actual" >&2
    exit 1
  fi
}

assert_candidates() {
  assert_omits 'unstaged ('
  assert_omits '  • staged ('
  assert_omits 'untracked ('
  assert_contains "unavailable ($fixture/unavailable): cleanliness unknown"
  assert_contains 'clean, tip already contained in main'
  assert_contains 'confirm the owned task is finished before cleanup'
  assert_omits 'merged into main'
  assert_omits 'remove worktree + delete branch'
}

run_check absent
assert_candidates
assert_lookups stale
assert_contains 'stale, no open PR'
assert_omits 'fresh ('

run_check open
assert_lookups stale
assert_omits 'stale ('

run_check denied
assert_lookups stale
assert_contains 'stale, open PR status unknown'
assert_omits 'no open PR'

run_check absent --local-only
assert_lookups ''
assert_candidates
assert_contains 'stale, open PR status unknown'
assert_omits 'no open PR'
printf 'Worktree check fixtures passed.\n'
