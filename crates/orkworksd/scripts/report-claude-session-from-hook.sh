#!/usr/bin/env bash
set -u

observed_at="$(python3 -c 'from datetime import datetime, timezone; print(datetime.now(timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z"))')"
payload="$(cat || true)"
claude_session_id="$(
  printf '%s' "$payload" |
    python3 -c 'import json,sys; data=json.load(sys.stdin); print(data.get("session_id") or "")' 2>/dev/null ||
    true
)"

if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ] && [ -n "$claude_session_id" ]; then
  escaped_session_id=$(printf '%s' "$claude_session_id" | sed 's/[\\"]/\\&/g')
  session_payload=$(printf '{"harnessSessionId":"%s","source":"claude_hook","confidence":0.98}' "$escaped_session_id")
  curl -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
    -H "Content-Type: application/json" \
    -d "$session_payload" >/dev/null || true
fi

if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ]; then
  attention_payload="$(python3 -c 'import json,sys; print(json.dumps({"status":"waiting_for_input","observedAt":sys.argv[1]}))' "$observed_at")"
  curl -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/attention" \
    -H "Content-Type: application/json" \
    -d "$attention_payload" >/dev/null || true
fi
