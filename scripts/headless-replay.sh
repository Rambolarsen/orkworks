#!/usr/bin/env bash
set -euo pipefail
script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
if ! command -v python3 >/dev/null 2>&1; then
  echo 'headless replay requires Python 3 (standard library only)' >&2
  exit 1
fi
exec python3 "$script_dir/headless_replay.py" "$@"
