#!/usr/bin/env bash
set -u

if [ -z "${ORKWORKS_SESSION_ID:-}" ] || [ -z "${ORKWORKS_PORT:-}" ] || [ -z "${OPENCODE_SESSION_ID:-}" ]; then
  exit 0
fi

payload=$(printf '{"harnessSessionId":"%s","source":"opencode_env","confidence":0.98}' "$OPENCODE_SESSION_ID")

curl -sS -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
  -H "Content-Type: application/json" \
  -d "$payload" >/dev/null || exit 0
