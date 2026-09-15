#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -gt 2 ] || [ "$#" -eq 2 ] && [ "${1:-}" != '--search' ]; then
  echo "usage: resolve-pr.sh [--search SEARCH_PHRASE] [PR_REFERENCE]" >&2
  exit 2
fi

ref="${1:-}"
search_phrase=''
if [ "$ref" = '--search' ]; then
  if [ "$#" -ne 2 ]; then
    echo 'resolve-pr: --search requires a phrase' >&2
    exit 2
  fi
  search_phrase="${2:-}"
  ref=''
fi
case "$ref" in
  -h|--help)
    echo "usage: resolve-pr.sh [--search SEARCH_PHRASE] [PR_REFERENCE]" >&2
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
if [[ "$ref" =~ ^#[0-9]+$ ]]; then
  pr_ref="${ref#\#}"
else
  pr_ref="$ref"
fi

is_structured_reference() {
  [[ "$1" =~ ^[0-9]+$ ]] || [[ "$1" =~ ^https://[^[:space:]]+/pull/[0-9]+$ ]]
}

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

err_file="$(mktemp "${TMPDIR:-/tmp}/resolve-pr.stderr.XXXXXX")"
trap 'rm -f "$err_file"' EXIT

if [ -n "$search_phrase" ]; then
  if ! search_json="$(gh pr list --state all --search "$search_phrase" --json number 2>"$err_file")"; then
    cat "$err_file" >&2
    echo "resolve-pr: could not search pull requests; ask the user for the PR number or URL" >&2
    exit 1
  fi
  search_count="$(jq -r 'length' <<<"$search_json")"
  if [ "$search_count" -eq 1 ]; then
    search_number="$(jq -r '.[0].number' <<<"$search_json")"
    if ! pr_json="$(fetch_pr "$search_number" 2>"$err_file")"; then
      cat "$err_file" >&2
      echo "resolve-pr: the matched pull request could not be fetched; ask the user for the PR number or URL" >&2
      exit 1
    fi
    print_pr "$pr_json"
    exit 0
  fi
  if [ "$search_count" -eq 0 ]; then
    echo "resolve-pr: no pull requests matched '$search_phrase'; ask the user for the PR number or URL" >&2
    exit 1
  fi
  echo "resolve-pr: '$search_phrase' matches multiple pull requests; ask the user for the PR number or URL" >&2
  exit 1
fi

if [ -z "$ref" ]; then
  if ! pr_json="$(fetch_pr '' 2>"$err_file")"; then
    cat "$err_file" >&2
    echo "resolve-pr: could not resolve this checkout's pull request; ask the user for the PR number or URL" >&2
    exit 1
  fi
  print_pr "$pr_json"
  exit 0
fi

if is_structured_reference "$pr_ref"; then
  if ! pr_json="$(fetch_pr "$pr_ref" 2>"$err_file")"; then
    cat "$err_file" >&2
    echo "resolve-pr: no pull request found for '$ref'; ask the user for the PR number or URL" >&2
    exit 1
  fi
  print_pr "$pr_json"
  exit 0
fi

if ! pr_json="$(fetch_pr "$pr_ref" 2>"$err_file")"; then
  cat "$err_file" >&2
  echo "resolve-pr: no pull request found for branch '$ref'; ask the user for the PR number or URL" >&2
  exit 1
fi
print_pr "$pr_json"
