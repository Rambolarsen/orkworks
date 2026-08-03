#!/usr/bin/env bash
set -u

marker=""
status="waiting_for_input"
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
    --status)
      if [ $# -ge 2 ]; then
        status="$2"
        shift 2
      else
        shift 1
      fi
      ;;
    *)
      shift
      ;;
  esac
done

payload="$(cat || true)"

# Claude Code's hook JSON includes both "cwd" (its own current working
# directory — issue #241) and "session_id" on every event; extract both from
# one parse of the same payload rather than spawning python3 twice. Codex's
# SessionStart payload only carries "session_id". `session_source` doubles as
# the harness-session "source" field below and as the marker for "this event
# isn't a needs-input signal" further down — one extraction point instead of
# matching "$marker" a second and third time.
reported_cwd=""
harness_session_id=""
session_source=""
case "$marker" in
  *:claude-code)
    # Single line delimited by the ASCII unit separator (0x1F), not two
    # lines: `mapfile`/`readarray` are bash 4+ only and macOS ships bash 3.2
    # by default. A whitespace delimiter (tab/space) would silently strip a
    # leading empty field via `read`'s IFS-whitespace-collapsing behavior
    # (e.g. an absent cwd would shift session_id into the cwd variable) — 0x1F
    # is non-whitespace, so `read` preserves empty fields correctly.
    claude_fields="$(
      printf '%s' "$payload" |
        python3 -c 'import json,sys; data=json.load(sys.stdin); print("%s\x1f%s" % (data.get("cwd") or "", data.get("session_id") or ""))' 2>/dev/null
    )" || true
    IFS=$'\x1f' read -r reported_cwd harness_session_id <<< "$claude_fields"
    session_source="claude_hook"
    ;;
  *:codex)
    harness_session_id="$(
      printf '%s' "$payload" |
        python3 -c 'import json,sys; print(json.load(sys.stdin).get("session_id") or "")' 2>/dev/null
    )" || true
    session_source="codex_hook"
    ;;
esac

# Codex's marker is installed on SessionStart, which fires at session
# start/resume/clear/compact — not a "needs input" signal like every other
# marker here (Claude/Gemini/Copilot/Aider all hook a genuine notification
# event). Posting the generic attention update for codex would mislabel
# every freshly launched session as waiting_for_input.
if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ] && [ "$session_source" != "codex_hook" ]; then
  observed_at="$(python3 -c 'from datetime import datetime, timezone; print(datetime.now(timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z"))')"
  attention_payload="$(python3 -c '
import json, sys
payload = {"status":sys.argv[1], "observedAt":sys.argv[2]}
cwd = sys.argv[3]
if cwd:
    payload["cwd"] = cwd
print(json.dumps(payload))
' "$status" "$observed_at" "$reported_cwd")"
  curl -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/attention" \
    -H "Content-Type: application/json" \
    -d "$attention_payload" >/dev/null || true
fi

if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ] && [ -n "$harness_session_id" ] && [ -n "$session_source" ]; then
  escaped_session_id=$(printf '%s' "$harness_session_id" | sed 's/[\\"]/\\&/g')
  session_payload=$(printf '{"harnessSessionId":"%s","source":"%s","confidence":0.98}' "$escaped_session_id" "$session_source")
  curl -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
    -H "Content-Type: application/json" \
    -d "$session_payload" >/dev/null || true
fi
