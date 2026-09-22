#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -gt 2 ] || [ "$#" -eq 1 ] || { [ "$#" -eq 2 ] && [ "${1:-}" != '--state' ]; }; then
  echo "usage: $0 [--state all]" >&2
  exit 2
fi

pr_state='open'
if [ "$#" -eq 2 ] && [ "$2" != 'all' ]; then
  echo "usage: $0 [--state all]" >&2
  exit 2
fi
if [ "$#" -eq 2 ]; then
  pr_state='all'
fi

command -v gh >/dev/null 2>&1 || {
  echo 'stranded-branch-audit: gh is required' >&2
  exit 1
}

if ! git remote get-url origin >/dev/null 2>&1; then
  echo 'stranded-branch-audit: no origin remote configured' >&2
  exit 1
fi

repo_slug=''
origin_url="$(git remote get-url origin 2>/dev/null)" || origin_url=''
# Strip a trailing .git once, whatever the URL scheme is.
case "$origin_url" in
  *.git) origin_url="${origin_url%.git}" ;;
esac
case "$origin_url" in
  git@github.com:*|ssh://git@github.com/*)
    repo_slug="${origin_url#git@github.com:}"
    repo_slug="${repo_slug#ssh://git@github.com/}"
    ;;
  https://github.com/*|http://github.com/*)
    repo_slug="${origin_url#*github.com/}"
    ;;
esac
if [ -z "$repo_slug" ]; then
  echo 'stranded-branch-audit: origin does not point at github.com; cannot audit PRs' >&2
  exit 1
fi
stale_secs=$((7 * 86400))
now=$(date +%s)
remote_branch_prefix="refs/remotes/origin/"
default_branch="$(git symbolic-ref --short "refs/remotes/origin/HEAD" 2>/dev/null | sed "s|^${remote_branch_prefix}||" || true)"
if [ -z "$default_branch" ]; then
  default_branch='main'
fi

stranded=0
while IFS= read -r branch; do
  branch="${branch#refs/remotes/origin/}"
  branch="${branch#origin/}"
  [ -n "$branch" ] || continue
  [ "$branch" = "$default_branch" ] && continue
  [ "$branch" = 'HEAD' ] && continue
  tip_epoch="$(git log -1 --format=%ct "origin/$branch" 2>/dev/null)" || continue
  age=$((now - tip_epoch))
  [ "$age" -gt "$stale_secs" ] || continue

  # Skip branches whose tip is content-merged into the default branch even
  # when it is not an ancestor (squash-merge histories are common here):
  # merging the branch into the default branch must produce exactly the
  # default branch's own tree, i.e. the branch contributes nothing new.
  default_tree="$(git rev-parse "$default_branch^{tree}" 2>/dev/null)"
  if [ -n "$default_tree" ] &&
     git merge-tree --write-tree "$default_branch" "origin/$branch" >/dev/null 2>&1 &&
     [ "$(git merge-tree --write-tree "$default_branch" "origin/$branch" 2>/dev/null)" = "$default_tree" ]; then
    continue
  fi

  if ! pr_json="$(gh pr list --repo "$repo_slug" --state "$pr_state" --head "$repo_slug:$branch" --json number,state 2>/dev/null)"; then
    pr_json='[]'
  fi
  # gh success with empty stdout (no PRs matched) still needs to be a JSON
  # array for the jq steps below; real gh prints [] in that case.
  pr_json="${pr_json:-[]}"
  # An OPEN PR covers the branch in both modes: in open mode the list only
  # contains open PRs, in all mode any OPEN entry counts. MERGED records do
  # not cover — state=all exists precisely to catch branches whose only PR
  # record is merged.
  if printf '%s' "$pr_json" | jq -e 'if length == 0 then false else (any(.[]; .state == "OPEN")) end' >/dev/null 2>&1; then
    continue
  fi
  pr_summary="$(printf '%s' "$pr_json" | jq -r '
    if (length == 0) then
      "no PR"
    else
      map((if .state == "OPEN" then "#\(.number) open" else "#\(.number) \(.state | ascii_downcase)" end) | tostring)
      | join(", ")
    end')"

  age_days=$((age / 86400))
  printf '%s\t%dd\t%s\n' "$branch" "$age_days" "$pr_summary"
  stranded=$((stranded + 1))
done < <(git for-each-ref refs/remotes/origin --format='%(refname:short)' | sort)

if [ "$stranded" -eq 0 ]; then
  echo 'no stranded remote branches found'
fi
