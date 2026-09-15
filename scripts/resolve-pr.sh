#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -gt 1 ]; then
  echo "usage: resolve-pr.sh [PR_REFERENCE]" >&2
  exit 2
fi

ref="${1:-}"
case "$ref" in
  -h|--help)
    echo "usage: resolve-pr.sh [PR_REFERENCE]" >&2
    exit 2
    ;;
esac

command -v gh >/dev/null 2>&1 || {
  echo 'resolve-pr: gh is required' >&2
  exit 1
}
command -v jq >/dev/null 2>&1 || {
  echo 'resolve-pr: jq is required' >&2
  exit 1
}

pr_fields='number,url,headRefName,baseRefName,state'
pr_ref="${ref#\#}"

fetch_pr() {
  if [ -n "$1" ]; then
    gh pr view "$1" --json "$pr_fields"
  else
    gh pr view --json "$pr_fields"
  fi
}

print_pr() {
  printf 'number: %s\n' "$(jq -r '.number // empty' <<<"$1")"
  printf 'url: %s\n' "$(jq -r '.url // empty' <<<"$1")"
  printf 'head: %s\n' "$(jq -r '.headRefName // empty' <<<"$1")"
  printf 'base: %s\n' "$(jq -r '.baseRefName // empty' <<<"$1")"
  printf 'state: %s\n' "$(jq -r '.state // empty' <<<"$1")"
}

if [ -z "$ref" ]; then
  if ! pr_json="$(fetch_pr '' 2>&1)"; then
    printf '%s\n' "$pr_json" >&2
    echo "resolve-pr: could not resolve this checkout's pull request; ask the user for the PR number or URL" >&2
    exit 1
  fi
  print_pr "$pr_json"
  exit 0
fi

if pr_json="$(fetch_pr "$pr_ref" 2>/dev/null)"; then
  print_pr "$pr_json"
  exit 0
fi

search_json="$(gh pr list --search "$pr_ref" --json number 2>/dev/null || echo '[]')"
search_count="$(jq -r 'length' <<<"$search_json")"
if [ "$search_count" -eq 1 ]; then
  search_number="$(jq -r '.[0].number' <<<"$search_json")"
  pr_json="$(fetch_pr "$search_number")"
  print_pr "$pr_json"
  exit 0
fi
if [ "$search_count" -eq 0 ]; then
  echo "resolve-pr: no pull requests matched '$pr_ref'; ask the user for the PR number or URL" >&2
  exit 1
fi
echo "resolve-pr: '$pr_ref' matches multiple pull requests; ask the user for the PR number or URL" >&2
exit 1
