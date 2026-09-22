#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
helper="$repo_root/scripts/stranded-branch-audit.sh"
fixture="$(mktemp -d "${TMPDIR:-/tmp}/orkworks-stranded-branch.XXXXXX")"
trap 'rm -rf "$fixture"' EXIT

repo="$fixture/repo"
origin="$fixture/origin.git"
bin="$fixture/bin"
mkdir -p "$bin"

# Two days of fake history: stale_ref is 14 days old, fresh_ref is 1 day old.
now=$(date +%s)
old_epoch=$((now - 14 * 86400))
fresh_epoch=$((now - 1 * 86400))

git init -q "$repo"
git -C "$repo" config user.email test@example.invalid
git -C "$repo" config user.name "Stranded branch test"
printf 'baseline\n' > "$repo/README.md"
git -C "$repo" add README.md
GIT_COMMITTER_DATE="$old_epoch" GIT_AUTHOR_DATE="$old_epoch" git -C "$repo" commit -qm baseline
git -C "$repo" branch -M main

git init --bare -q "$origin"
# The helper only audits repos whose origin is on github.com; a fixture needs
# the slug derivation to succeed, so point origin at a github.com URL while
# rewriting pushes to the local bare remote via pushInsteadOf. pushInsteadOf
# (not insteadOf) keeps `git remote get-url origin` reporting the github URL —
# insteadOf would rewrite the fetch URL too and defeat the helper's slug
# derivation.
mkdir -p "$fixture/gitconfig"
{
  printf '[url "%s"]\n' "$origin"
  printf '\tpushInsteadOf = https://github.com/Rambolarsen/orkworks\n'
} > "$fixture/gitconfig/insteadOf"
export GIT_CONFIG_GLOBAL="$fixture/gitconfig/insteadOf"
git -C "$repo" remote add origin "https://github.com/Rambolarsen/orkworks"

# A remote branch with real, distinct commits, 14 days old.
git -C "$repo" switch -q -c stale-feature
printf 'stale\n' > "$repo/stale.txt"
git -C "$repo" add stale.txt
GIT_COMMITTER_DATE="$old_epoch" GIT_AUTHOR_DATE="$old_epoch" git -C "$repo" commit -qm stale-work
git -C "$repo" push -q -u origin stale-feature

# A remote branch 14 days old but WITH an open PR: must not be flagged.
git -C "$repo" switch -q main
git -C "$repo" switch -q -c pr-feature
printf 'pr\n' > "$repo/pr.txt"
git -C "$repo" add pr.txt
GIT_COMMITTER_DATE="$old_epoch" GIT_AUTHOR_DATE="$old_epoch" git -C "$repo" commit -qm pr-work
git -C "$repo" push -q -u origin pr-feature

# A remote branch merged content-wise into main but not an ancestor (squash):
# its tip is stale, its PR is MERGED, its file lives on main. Must not be flagged.
git -C "$repo" switch -q main
git -C "$repo" switch -q -c squash-merged
printf 'squash\n' > "$repo/squash.txt"
git -C "$repo" add squash.txt
GIT_COMMITTER_DATE="$old_epoch" GIT_AUTHOR_DATE="$old_epoch" git -C "$repo" commit -qm squash-work
git -C "$repo" push -q -u origin squash-merged
git -C "$repo" switch -q main
# Land the squash: the branch's content appears on main via a fresh commit
# (simulating a squash-merge), so main's tree contains squash.txt.
git -C "$repo" checkout -q squash-merged -- squash.txt
GIT_COMMITTER_DATE="$fresh_epoch" GIT_AUTHOR_DATE="$fresh_epoch" git -C "$repo" commit -qm land-squash
git -C "$repo" push -q origin main

# A fresh remote branch (1 day old, no PR): must not be flagged.
git -C "$repo" switch -q main
git -C "$repo" switch -q -c fresh-feature
printf 'fresh\n' > "$repo/fresh.txt"
git -C "$repo" add fresh.txt
GIT_COMMITTER_DATE="$fresh_epoch" GIT_AUTHOR_DATE="$fresh_epoch" git -C "$repo" commit -qm fresh-work
git -C "$repo" push -q -u origin fresh-feature

# A local-only branch older than 7 days: must not be flagged (not a remote branch).
git -C "$repo" switch -q main
git -C "$repo" switch -q -c local-only
printf 'local\n' > "$repo/local.txt"
git -C "$repo" add local.txt
GIT_COMMITTER_DATE="$old_epoch" GIT_AUTHOR_DATE="$old_epoch" git -C "$repo" commit -qm local-work
git -C "$repo" switch -q main

# Fake gh: --repo resolution plus pr list per head branch, driven by GH_PR_LIST_MODE.
cat > "$bin/gh" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail

if [ "${1:-}" = 'pr' ] && [ "${2:-}" = 'list' ]; then
  # Args look like: pr list --repo X --state open --head X:branch --json number,state
  # ${@: -3:1} is exactly the X:branch element; a bare ${@: -3} would be a
  # three-element slice joined with spaces.
  local_ref="${@: -3:1}"
  branch="${local_ref#*:}"
  matched=0
  case "${GH_PR_LIST_MODE:?}" in
    pr-branch)
      if [ "$branch" = 'pr-feature' ]; then printf '[{"number":700,"state":"OPEN"}]\n'; matched=1; fi
      if [ "$branch" = 'squash-merged' ]; then printf '[{"number":702,"state":"MERGED"}]\n'; matched=1; fi
      ;;
    open-pr-branch)
      if [ "$branch" = 'stale-feature' ]; then printf '[{"number":701,"state":"OPEN"}]\n'; matched=1; fi
      if [ "$branch" = 'squash-merged' ]; then printf '[{"number":702,"state":"MERGED"}]\n'; matched=1; fi
      ;;
  esac
  if [ "$matched" = 0 ]; then printf '[]\n'; fi
  exit 0
fi

echo "unexpected gh invocation: $*" >&2
exit 1
EOF
chmod +x "$bin/gh"

# The helper parses the origin slug from the remote URL; rewrite it back to the
# local bare remote only for pushes done before the audit runs.
git -C "$repo" config remote.origin.url "https://github.com/Rambolarsen/orkworks"

# Case 1: default (open) PR mode. Only stale-feature is stranded.
output="$( (cd "$repo" && PATH="$bin:$PATH" GH_PR_LIST_MODE=pr-branch "$helper") )"
grep -Fq 'stale-feature' <<<"$output"
if grep -Fq 'pr-feature' <<<"$output"; then
  echo 'stranded-branch-audit flagged a branch with an open PR' >&2
  exit 1
fi
if grep -Fq 'fresh-feature' <<<"$output"; then
  echo 'stranded-branch-audit flagged a fresh branch' >&2
  exit 1
fi
if grep -Fq 'squash-merged' <<<"$output"; then
  echo 'stranded-branch-audit flagged a squash-merged branch' >&2
  exit 1
fi
if grep -Fq 'local-only' <<<"$output"; then
  echo 'stranded-branch-audit flagged a local-only branch' >&2
  exit 1
fi
grep -Fq 'no PR' <<<"$output"

# Case 2: --state all. squash-merged is covered by a MERGED PR; still not flagged.
# stale-feature with a MERGED-only PR record must be flagged here (state=all
# sees the PR, but the helper only treats OPEN PRs as coverage).
output_all="$( (cd "$repo" && PATH="$bin:$PATH" GH_PR_LIST_MODE=pr-branch "$helper" --state all) )"
grep -Fq 'stale-feature' <<<"$output_all"
if grep -Fq 'squash-merged' <<<"$output_all"; then
  echo 'stranded-branch-audit flagged a merged-PR branch under --state all' >&2
  exit 1
fi

# Case 3: an open PR anywhere in history blocks the flag in --state all too.
output_open="$( (cd "$repo" && PATH="$bin:$PATH" GH_PR_LIST_MODE=open-pr-branch "$helper" --state all) )"
if grep -Fq 'stale-feature' <<<"$output_open"; then
  echo 'stranded-branch-audit flagged a branch with an open PR under --state all' >&2
  exit 1
fi

# Case 4: unknown option is rejected.
if (cd "$repo" && PATH="$bin:$PATH" GH_PR_LIST_MODE=pr-branch "$helper" --wat) > "$fixture/wat.out" 2>&1; then
  echo 'stranded-branch-audit accepted an unknown option' >&2
  exit 1
fi
grep -Fq 'usage' "$fixture/wat.out"

# Case 5: remote without origin remote configured -> error.
git -C "$repo" remote remove origin
if (cd "$repo" && PATH="$bin:$PATH" GH_PR_LIST_MODE=pr-branch "$helper") > "$fixture/no-remote.out" 2>&1; then
  echo 'stranded-branch-audit unexpectedly succeeded without an origin remote' >&2
  exit 1
fi
grep -Fq 'no origin remote' "$fixture/no-remote.out"

echo 'stranded-branch-audit fixtures passed'
