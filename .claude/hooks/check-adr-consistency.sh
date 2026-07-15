#!/usr/bin/env bash
# Cross-checks each ADR's in-body `- Status:` line against its status cell
# in docs/adr/README.md. Flags drift (e.g. body says "superseded" but the
# index still says "accepted", or vice versa) without any new metadata.

set -euo pipefail

ADR_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../docs/adr" && pwd)"
INDEX="$ADR_DIR/README.md"

mismatches=()

for f in "$ADR_DIR"/[0-9]*.md; do
  num=$(basename "$f" | grep -oE '^[0-9]+')
  [ -z "$num" ] && continue

  body_status=$(grep -m1 -E '^- Status:' "$f" | sed -E 's/^- Status:[[:space:]]*//' | tr '[:upper:]' '[:lower:]')
  [ -z "$body_status" ] && { mismatches+=("$num: no '- Status:' line found in $(basename "$f")"); continue; }

  index_row=$(grep -E "^\| \[$num\]" "$INDEX" || true)
  if [ -z "$index_row" ]; then
    mismatches+=("$num: no row in docs/adr/README.md index")
    continue
  fi
  index_status=$(echo "$index_row" | awk -F'|' '{print $(NF-1)}' | tr '[:upper:]' '[:lower:]' | sed -E 's/^[[:space:]]+|[[:space:]]+$//g')

  body_word=$(echo "$body_status" | grep -oE '^(accepted|proposed|deprecated|superseded|rejected)')
  index_word=$(echo "$index_status" | grep -oE '^(accepted|proposed|deprecated|superseded|rejected)')

  if [ "$body_word" != "$index_word" ]; then
    mismatches+=("$num: body says '$body_status' but index says '$index_status'")
  fi

  # If the body's "Superseded by:" line names another ADR number, the index
  # row should reference it too. A supersession by a non-ADR feature/change
  # (e.g. 0017) has nothing to cross-check and is not flagged.
  superseded_by_adr=$(grep -m1 -E '^- Superseded by:' "$f" | grep -oE '\b00[0-9]{2}\b' | head -1 || true)
  if [ -n "$superseded_by_adr" ] && ! echo "$index_status" | grep -q "superseded by.*$superseded_by_adr"; then
    mismatches+=("$num: body says superseded by $superseded_by_adr but index row doesn't reference it")
  fi
done

[ ${#mismatches[@]} -eq 0 ] && exit 0

echo "[adr-consistency] Status drift between ADR body and docs/adr/README.md index:"
for m in "${mismatches[@]}"; do
  echo "  - $m"
done
exit 1
