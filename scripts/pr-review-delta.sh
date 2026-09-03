#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: $0 FROM_SHA TO_SHA" >&2
  exit 2
fi

git diff --numstat --find-renames "$1" "$2" -- apps/desktop crates/orkworksd |
  awk 'tolower($NF) !~ /\.md$/ {
    files += 1
    if ($1 ~ /^[0-9]+$/ && $2 ~ /^[0-9]+$/) lines += $1 + $2
  }
  END { printf "%d %d\n", files + 0, lines + 0 }'
