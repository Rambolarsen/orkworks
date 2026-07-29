#!/usr/bin/env bash
set -u

marker=""
while [ $# -gt 0 ]; do
  case "$1" in
    --marker)
      if [ $# -ge 2 ]; then
        marker="$2"
        shift 2
      else
        # No value follows; drop the flag alone so the loop still terminates.
        shift 1
      fi
      ;;
    *)
      shift
      ;;
  esac
done

observed_at="$(python3 -c 'from datetime import datetime, timezone; print(datetime.now(timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z"))')"
payload="$(cat || true)"

# Claude Code's hook JSON includes a "cwd" field (its own current working
# directory) on every event, alongside "session_id" below. Forwarding it
# lets the sidecar track where the agent is actually working, not just
# where its process was launched (issue #241).
reported_cwd=""
case "$marker" in
  *:claude-code)
    reported_cwd="$(
      printf '%s' "$payload" |
        python3 -c 'import json,sys; data=json.load(sys.stdin); print(data.get("cwd") or "")' 2>/dev/null ||
        true
    )"
    ;;
esac

if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ]; then
  attention_payload="$(python3 -c '
import json, sys
payload = {"status":"waiting_for_input", "observedAt":sys.argv[1]}
cwd = sys.argv[2]
if cwd:
    payload["cwd"] = cwd
print(json.dumps(payload))
' "$observed_at" "$reported_cwd")"
  curl -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/attention" \
    -H "Content-Type: application/json" \
    -d "$attention_payload" >/dev/null || true
fi

case "$marker" in
  *:claude-code)
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
    ;;
esac
