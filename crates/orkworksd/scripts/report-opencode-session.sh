#!/usr/bin/env bash
set -u

if [ -z "${ORKWORKS_SESSION_ID:-}" ] || [ -z "${ORKWORKS_PORT:-}" ] || [ -z "${OPENCODE_SESSION_ID:-}" ]; then
  exit 0
fi

escaped_session_id=$(printf '%s' "$OPENCODE_SESSION_ID" | sed 's/[\\"]/\\&/g')
payload=$(printf '{"harnessSessionId":"%s","source":"opencode_env","confidence":0.98}' "$escaped_session_id")

curl -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
  -H "Content-Type: application/json" \
  -d "$payload" >/dev/null || exit 0
