#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: $0 PR_REFERENCE" >&2
  exit 2
fi

pr_ref="$1"
gh_pr_ref="${pr_ref#\#}"
if [[ "$gh_pr_ref" =~ ^[0-9]+$ ]]; then
  pr_label="#$gh_pr_ref"
else
  pr_label="$gh_pr_ref"
fi

command -v gh >/dev/null 2>&1 || {
  echo 'finish-pr: gh is required' >&2
  exit 1
}
command -v jq >/dev/null 2>&1 || {
  echo 'finish-pr: jq is required' >&2
  exit 1
}

repo_root="$(git rev-parse --show-toplevel)"
current_repo="$(gh repo view --json nameWithOwner --jq .nameWithOwner)"
pr_json="$(gh pr view "$gh_pr_ref" --json number,state,baseRefName,headRefName,headRefOid,headRepository,isCrossRepository,mergeCommit)"
pr_number="$(jq -r '.number // empty' <<<"$pr_json")"
state="$(jq -r '.state // empty' <<<"$pr_json")"
base_branch="$(jq -r '.baseRefName // empty' <<<"$pr_json")"
head_branch="$(jq -r '.headRefName // empty' <<<"$pr_json")"
head_ref_oid="$(jq -r '.headRefOid // empty' <<<"$pr_json")"
head_repo="$(jq -r '.headRepository.nameWithOwner // empty' <<<"$pr_json")"
is_cross_repository="$(jq -r '.isCrossRepository // false' <<<"$pr_json")"

if [ "$state" != 'MERGED' ]; then
  echo "finish-pr: PR $pr_label is not merged (state: ${state:-unknown}); nothing changed" >&2
  exit 1
fi
if [ "$base_branch" != 'main' ]; then
  echo "finish-pr: PR $pr_label targets ${base_branch:-an unknown branch}, not main; nothing changed" >&2
  exit 1
fi
if [ "$is_cross_repository" = 'true' ]; then
  echo "finish-pr: PR $pr_label is cross-repository; nothing changed" >&2
  exit 1
fi
if [ -z "$head_repo" ] || [ "$head_repo" != "$current_repo" ]; then
  echo "finish-pr: PR $pr_label is owned by ${head_repo:-an unknown repository}, not $current_repo; nothing changed" >&2
  exit 1
fi
if [ -z "$head_branch" ] || [ "$head_branch" = 'main' ] || ! git check-ref-format --branch "$head_branch" >/dev/null 2>&1; then
  echo "finish-pr: PR $pr_label has an invalid or protected head branch; nothing changed" >&2
  exit 1
fi
if [ -z "$head_ref_oid" ]; then
  echo "finish-pr: PR $pr_label is missing the head commit OID; nothing changed" >&2
  exit 1
fi

current_branch="$(git -C "$repo_root" branch --show-current)"
if [ "$current_branch" = "$head_branch" ]; then
  echo "finish-pr: PR $pr_label is checked out in the current worktree; switch away before cleanup" >&2
  exit 1
fi

local_branch_exists=false
if git -C "$repo_root" show-ref --verify --quiet "refs/heads/$head_branch"; then
  local_branch_exists=true
  local_head_oid="$(git -C "$repo_root" rev-parse "refs/heads/$head_branch")"
  if [ "$local_head_oid" != "$head_ref_oid" ]; then
    echo "finish-pr: local branch $head_branch no longer matches PR $pr_label; nothing changed" >&2
    exit 1
  fi
fi

matching_count=0
matching_path=''
worktree_path=''
worktree_branch=''
record_worktree() {
  if [ "$worktree_branch" = "$head_branch" ]; then
    matching_count=$((matching_count + 1))
    matching_path="$worktree_path"
  fi
  worktree_path=''
  worktree_branch=''
}

while IFS= read -r line || [ -n "$line" ]; do
  case "$line" in
    'worktree '*) worktree_path="${line#worktree }" ;;
    'branch refs/heads/'*) worktree_branch="${line#branch refs/heads/}" ;;
    '') record_worktree ;;
  esac
done < <(git -C "$repo_root" worktree list --porcelain)
record_worktree

if [ "$matching_count" -gt 1 ]; then
  echo "finish-pr: PR $pr_label has multiple local worktrees for branch $head_branch; clean up explicitly" >&2
  exit 1
fi

if [ "$matching_count" -eq 1 ]; then
  dirty="$(git -C "$matching_path" status --porcelain --untracked-files=all)"
  if [ -n "$dirty" ]; then
    echo "finish-pr: worktree $matching_path has uncommitted changes; nothing changed" >&2
    exit 1
  fi
fi

open_pr_numbers="$(gh pr list --repo "$current_repo" --state open --head "$head_repo:$head_branch" --json number --jq '.[].number')"
while IFS= read -r open_pr_number; do
  [ -n "$open_pr_number" ] || continue
  if [ "$open_pr_number" != "$pr_number" ]; then
    echo "finish-pr: branch $head_branch is still used by open PR #$open_pr_number; nothing changed" >&2
    exit 1
  fi
done <<<"$open_pr_numbers"

if [ "$matching_count" -eq 1 ]; then
  git -C "$repo_root" worktree remove "$matching_path"
fi

git -C "$repo_root" worktree prune
branch_removed=false
if [ "$local_branch_exists" = true ]; then
  if ! git -C "$repo_root" update-ref -d "refs/heads/$head_branch" "$head_ref_oid"; then
    echo "finish-pr: local branch $head_branch changed during cleanup; nothing changed" >&2
    exit 1
  fi
  branch_removed=true
elif git -C "$repo_root" show-ref --verify --quiet "refs/heads/$head_branch"; then
  echo "finish-pr: local branch $head_branch appeared during cleanup; nothing changed" >&2
  exit 1
fi

message="PR $pr_label is merged into main"
if [ "$branch_removed" = true ]; then
  message+="; cleaned branch $head_branch"
fi
if [ "$matching_count" -eq 1 ]; then
  message+=" and worktree $matching_path"
fi
echo "$message"
