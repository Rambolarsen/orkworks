#!/usr/bin/env bash
set -u

payload="$(cat || true)"
claude_session_id="$(
  printf '%s' "$payload" |
    python3 -c 'import json,sys; data=json.load(sys.stdin); print(data.get("session_id",""))' 2>/dev/null ||
    true
)"

if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ] && [ -n "$claude_session_id" ]; then
  session_payload=$(printf '{"harnessSessionId":"%s","source":"claude_hook","confidence":0.98}' "$claude_session_id")
  curl -sS --max-time 3 --connect-timeout 1 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
    -H "Content-Type: application/json" \
    -d "$session_payload" >/dev/null || true
fi

if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ]; then
  curl -sS --max-time 3 --connect-timeout 1 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/attention" \
    -H "Content-Type: application/json" \
    -d '{"status":"waiting_for_input"}' >/dev/null || true
fi
