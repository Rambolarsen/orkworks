#!/usr/bin/env bash
set -u

marker=""
status=""
hook_fingerprint=""
event=""
# Plan-path mode (ADR 0038): set by an installed hook entry passing
# `--report-plan-path` (currently Claude's PostToolUse Write|Edit). In this
# mode the reporter forwards the harness payload's `tool_input.file_path`
# to `/sessions/:id/plan-path` and skips the generic attention + harness-
# session POSTs entirely — the sidecar canonicalizes/rejects the path.
report_plan_path="no"
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
    --report-plan-path)
      report_plan_path="yes"
      shift 1
      ;;
    --hook-fingerprint)
      if [ $# -ge 2 ]; then
        hook_fingerprint="$2"
        shift 2
      else
        shift 1
      fi
      ;;
    --event)
      if [ $# -ge 2 ]; then
        event="$2"
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

# Capture once, before parsing or network I/O. UUID filenames carry no order.
native_observed_at=""
if [ "${ORKWORKS_CODEX_NATIVE_APPROVAL:-}" = "1" ]; then
  native_observed_at="$(python3 -c 'from datetime import datetime,timezone; print(datetime.now(timezone.utc).isoformat(timespec="microseconds").replace("+00:00","Z"))' 2>/dev/null)" || true
fi

payload="$(cat || true)"

# Plan-path mode is its own exit path: extract the harness-written file path
# from the JSON stdin payload (`tool_input.file_path` — Claude's shape), apply
# a cheap lexical whitelist that defers real validation to the sidecar, POST
# once to /sessions/:id/plan-path, and exit before any attention or harness-
# session flow runs. The cheap filter exists only to keep unrelated Write/Edit
# calls from spamming the route; report_session_plan_path canonicalizes,
# rejects non-Markdown, workspace-escaping, symlink-pivoting, and missing
# files — the filter here is never the authority.
if [ "$report_plan_path" = "yes" ]; then
  if [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ]; then
    file_path="$(printf '%s' "$payload" | \
      python3 -c 'import json,sys;
data = json.load(sys.stdin) if sys.stdin else {}
ti = data.get("tool_input") if isinstance(data, dict) else None
print(((ti.get("file_path") if isinstance(ti, dict) else None) or "") if isinstance(data, dict) else "")' 2>/dev/null)" || true
    case "$file_path" in
      # Markdown only — the sidecar rejects anything else, but skip the
      # round-trip here.
      *.md)
        # Recognized plan/spec roots: `*/specs/*`, `*/docs/superpowers/plans/*`,
        # `*/docs/superpowers/specs/*`. Bash case patterns anchor each segment
        # with a `/` so a directory like `myspecs/` or `docs/specs/` does not
        # match.
        case "$file_path" in
          */specs/*|*/docs/superpowers/plans/*|*/docs/superpowers/specs/*)
            # sed's `[\\"]` escaping only covers backslashes/quotes; POSIX
            # filenames may legally contain newlines or other control
            # characters, which would otherwise land unescaped inside the
            # JSON string literal. json.dumps escapes the full set.
            plan_path_payload="$(python3 -c 'import json,sys; print(json.dumps({"planPath": sys.argv[1]}, separators=(",", ":")))' "$file_path" 2>/dev/null)" || true
            if [ -n "$plan_path_payload" ]; then
              curl -sS --max-time 5 --connect-timeout 2 -X POST \
                "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/plan-path" \
                -H "Content-Type: application/json" \
                -d "$plan_path_payload" >/dev/null || true
            fi
            ;;
        esac
        ;;
    esac
  fi
  exit 0
fi

# Claude Code's hook JSON includes both "cwd" (its own current working
# directory — issue #241) and "session_id" on every event; extract both from
# one parse of the same payload rather than spawning python3 twice. Codex's
# SessionStart payload carries other fields too (cwd, hook_event_name,
# source, ...). The native ID may be recovered from any owned Codex hook;
# SessionStart alone supplies lifecycle metadata used by the replacement guard.
# Codex documents subagent hook IDs as the owning parent session ID. Copilot's
# notification payload uses camelCase "sessionId" (not "session_id") per
# https://docs.github.com/en/copilot/reference/hooks-reference, alongside its
# own "cwd". `session_source` doubles as
# the harness-session "source" field below and as the marker for "this event
# isn't a needs-input signal" further down — one extraction point instead of
# matching "$marker" a second and third time.
reported_cwd=""
harness_session_id=""
session_start_source=""
session_start_event=""
session_start_observed_at=""
session_source=""
prompt_notification_type=""
prompt_observed_at=""
codex_attention="no"
codex_capture_only="no"
codex_payload_capture=""
attention_post_kind="not_applicable"
attention_curl_exit=""
attention_http_status=""
harness_session_post_kind="skipped_no_harness_session_id"
session_curl_exit=""
session_http_status=""
native_registration_accepted="no"

# Codex POST failures are captured in the redacted diagnostic below. Preserve
# curl's previous `-sS` stderr behavior for the other harness reporters.
reporter_curl() {
  if [ "$session_source" = "codex_hook" ]; then
    curl "$@" 2>/dev/null
  else
    curl "$@"
  fi
}

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
      python3 -c 'import json,sys; data=json.load(sys.stdin); event=sys.argv[1]; safe=lambda value: value if isinstance(value,str) and not any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value) else ""; source=safe(data.get("source")) if event == "SessionStart" else ""; notification=safe(data.get("notification_type")) if event == "Notification" else ""; session_id=safe(data.get("session_id")).strip(); print("%s\x1f%s\x1f%s\x1f%s" % (safe(data.get("cwd")), session_id, source if source == "clear" else "", notification))' "$event" 2>/dev/null
    )" || true
    IFS=$'\x1f' read -r reported_cwd harness_session_id session_start_source prompt_notification_type <<< "$claude_fields"
    if [ "$event" = "SessionStart" ] && [ "$session_start_source" = "clear" ]; then
      session_start_event="SessionStart"
    fi
    session_source="claude_hook"
    ;;
  *:codex)
    codex_fields="$(
      printf '%s' "$payload" |
      python3 -c 'import json,sys; data=json.load(sys.stdin); is_start=sys.argv[1] == "SessionStart"; safe=lambda value: value if isinstance(value,str) and not any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value) else ""; source=safe(data.get("source")) if is_start else ""; session_id=safe(data.get("session_id")); print("%s\x1f%s" % (session_id, source if source in {"startup", "resume", "clear", "compact"} else ""))' "$event" 2>/dev/null
    )" || true
    IFS=$'\x1f' read -r harness_session_id session_start_source <<< "$codex_fields"
    if [ -n "$session_start_source" ]; then
      session_start_event="SessionStart"
    fi
    session_source="codex_hook"
    case "$event" in
      PreToolUse|PermissionRequest|PostToolUse)
        codex_payload_capture="$(printf '%s' "$payload" | python3 -c '
import json, sys
allowed_scalars = ("hook_event_name", "permission_mode", "turn_id", "tool_name", "tool_use_id")
safe_payload_keys = {"hook_event_name", "model", "permission_mode", "turn_id", "tool_name", "tool_use_id", "tool_response"}
try:
    data = json.load(sys.stdin)
except Exception:
    data = None
capture = {"payloadKeys": [], "payloadScalars": {}}
if isinstance(data, dict):
    keys = [key for key in data if isinstance(key, str) and key in safe_payload_keys]
    capture["payloadKeys"] = sorted(keys)[:64]
    for key in allowed_scalars:
        value = data.get(key)
        if key == "tool_use_id":
            if isinstance(value, str) and len(value) <= 128:
                capture["payloadScalars"][key] = value
        elif isinstance(value, str) and len(value) <= 128:
            capture["payloadScalars"][key] = value
        elif type(value) in (int, float, bool):
            capture["payloadScalars"][key] = value
print(json.dumps(capture, separators=(",", ":")))
' 2>/dev/null)" || codex_payload_capture=""
        ;;
    esac
    case "$event" in
      UserPromptSubmit)
        status="working"
        codex_attention="yes"
        ;;
      PermissionRequest)
        status="waiting_for_input"
        codex_attention="yes"
        ;;
      Stop)
        status="idle"
        codex_attention="yes"
        ;;
      SessionStart)
        ;;
      PreToolUse|PostToolUse)
        codex_capture_only="yes"
        attention_post_kind="skipped_capture_only"
        harness_session_post_kind="skipped_capture_only"
        ;;
    esac
    ;;
  *:copilot)
    copilot_fields="$(
      printf '%s' "$payload" |
        python3 -c 'import datetime,json,sys; data=json.load(sys.stdin); event=sys.argv[1]; safe=lambda value: value if isinstance(value,str) and not any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value) else ""; source=safe(data.get("source")) if event == "sessionStart" else ""; notification=safe(data.get("notification_type")) if event == "notification" else ""; timestamp=data.get("timestamp") if event == "notification" else ""; start_ts=data.get("timestamp") if event == "sessionStart" else ""; session_id=safe(data.get("sessionId")).strip();
def iso(value):
 if isinstance(value,(int,float)) and not isinstance(value,bool): return datetime.datetime.fromtimestamp(value/1000,datetime.timezone.utc).isoformat(timespec="microseconds").replace("+00:00","Z")
 return ""
print("%s\x1f%s\x1f%s\x1f%s\x1f%s\x1f%s" % (safe(data.get("cwd")), session_id, source if source == "new" else "", notification, iso(timestamp), iso(start_ts)))' "$event" 2>/dev/null
    )" || true
    IFS=$'\x1f' read -r reported_cwd harness_session_id session_start_source prompt_notification_type prompt_observed_at session_start_observed_at <<< "$copilot_fields"
    if [ "$event" = "sessionStart" ] && [ "$session_start_source" = "new" ]; then
      session_start_event="sessionStart"
    fi
    session_source="copilot_hook"
    ;;
esac

# Native Pre/Post are correlation only. Suppress a permission HTTP wait only
# after a complete, bounded scalar envelope has been atomically published.
native_approval_spooled="no"
if [ "$session_source" = "codex_hook" ] && [ "${ORKWORKS_CODEX_NATIVE_APPROVAL:-}" = "1" ] && \
   [ -n "${ORKWORKS_CODEX_SESSION_REPORT_DIR:-}" ] && [ -n "${ORKWORKS_REPORT_TOKEN:-}" ] && \
   [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "$native_observed_at" ]; then
  case "$event" in
    PreToolUse|PermissionRequest|PostToolUse|Stop|UserPromptSubmit)
      if printf '%s' "$payload" | python3 -c '
import json, os, re, sys, tempfile, uuid
raw = json.load(sys.stdin)
valid_id = lambda v: isinstance(v,str) and re.fullmatch(r"[A-Za-z0-9_-]{1,128}",v) is not None
if not isinstance(raw,dict) or not valid_id(raw.get("session_id")) or not re.fullmatch(r"[a-f0-9]{64}",sys.argv[3]):
    raise SystemExit(2)
report = {"rootId":raw["session_id"], "event":sys.argv[1], "observedAt":sys.argv[2], "hookFingerprint":sys.argv[3]}
for source,target in (("turn_id","turnId"),("tool_use_id","toolUseId")):
    value = raw.get(source)
    report[target] = value if valid_id(value) else None
encoded = json.dumps({"approval":report},separators=(",",":")).encode()
if len(encoded) > 4096: raise SystemExit(2)
directory = sys.argv[4]
fd, temporary = tempfile.mkstemp(prefix=".pending-",dir=directory)
try:
    with os.fdopen(fd,"wb") as output:
        output.write(encoded)
        output.flush()
        os.fsync(output.fileno())
    os.replace(temporary,os.path.join(directory,uuid.uuid4().hex+".json"))
except Exception:
    try: os.unlink(temporary)
    except OSError: pass
    raise
' "$event" "$native_observed_at" "$hook_fingerprint" "$ORKWORKS_CODEX_SESSION_REPORT_DIR" 2>/dev/null; then
        if [ "$event" = "PermissionRequest" ]; then
          native_approval_spooled="yes"
          attention_post_kind="enqueued"
        fi
      fi
      ;;
  esac
fi

# Codex's SessionStart event captures identity only. Turn events carry their
# explicit normalized status and provenance so the sidecar can validate the
# deterministic signal without trusting mutable payload text.
if [ "$native_approval_spooled" != "yes" ] && [ "$codex_capture_only" != "yes" ] && [ "$session_source" = "codex_hook" ] && [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ] && \
  { [ "$session_source" != "codex_hook" ] || [ "$codex_attention" = "yes" ]; }; then
  observed_at="$native_observed_at"
  if [ -z "$observed_at" ]; then observed_at="$(python3 -c 'from datetime import datetime, timezone; print(datetime.now(timezone.utc).isoformat(timespec="microseconds").replace("+00:00", "Z"))')"; fi
  attention_payload="$(python3 -c '
import json, sys
payload = {"status":sys.argv[1], "observedAt":sys.argv[2]}
cwd, source, event_name, fingerprint = sys.argv[3:]
if cwd:
    payload["cwd"] = cwd
if source == "codex_hook":
    payload["source"] = source
    payload["event"] = event_name
    if fingerprint:
        payload["hookFingerprint"] = fingerprint
print(json.dumps(payload))
' "$status" "$observed_at" "$reported_cwd" "$session_source" "$event" "$hook_fingerprint")"
  attention_post_kind="posted"
  attention_curl_exit=0
  attention_http_status=$(reporter_curl -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/attention" \
    -H "Content-Type: application/json" \
    -d "$attention_payload" --output /dev/null --write-out '%{http_code}') || attention_curl_exit=$?
elif [ "$native_approval_spooled" != "yes" ] && [ "$session_source" = "codex_hook" ] && [ "$codex_attention" = "yes" ]; then
  attention_post_kind="skipped_missing_environment"
fi

if [ "$codex_capture_only" != "yes" ] && [ -n "${ORKWORKS_SESSION_ID:-}" ] && [ -n "${ORKWORKS_PORT:-}" ] && [ -n "$harness_session_id" ] && [ -n "$session_source" ] && \
  { [ "$session_source" = "codex_hook" ] || { [ -n "${ORKWORKS_REPORT_TOKEN:-}" ] && [ -n "${ORKWORKS_PROMPT_HOOK_GENERATION:-}" ]; }; }; then
  escaped_session_id=$(printf '%s' "$harness_session_id" | sed 's/[\\"]/\\&/g')
  if [ "$session_source" = "claude_hook" ] || [ "$session_source" = "copilot_hook" ]; then
    session_payload="$(python3 -c 'import json,sys; data={"harnessSessionId":sys.argv[1],"source":sys.argv[2],"confidence":0.98,"promptHookGeneration":sys.argv[3]}; event,source,observed_at,cwd=sys.argv[4:];
if event and source:
 data["sessionStartEvent"]=event; data["sessionStartSource"]=source
 if source == "new": data["sessionStartObservedAt"]=observed_at
if cwd: data["cwd"]=cwd
print(json.dumps(data,separators=(",",":")))' "$harness_session_id" "$session_source" "$ORKWORKS_PROMPT_HOOK_GENERATION" "$session_start_event" "$session_start_source" "$session_start_observed_at" "$reported_cwd")"
  elif [ "$session_source" = "codex_hook" ] && [ -n "$hook_fingerprint" ]; then
    escaped_fingerprint=$(printf '%s' "$hook_fingerprint" | sed 's/[\\"]/\\&/g')
    session_payload=$(printf '{"harnessSessionId":"%s","source":"%s","confidence":0.98,"hookFingerprint":"%s"}' "$escaped_session_id" "$session_source" "$escaped_fingerprint")
  else
    session_payload=$(printf '{"harnessSessionId":"%s","source":"%s","confidence":0.98}' "$escaped_session_id" "$session_source")
  fi
  if [ "$session_source" = "codex_hook" ] && [ -n "$session_start_source" ] && [ "$session_start_event" = "SessionStart" ]; then
    session_payload="${session_payload%?},\"sessionStartSource\":\"$session_start_source\",\"sessionStartEvent\":\"$session_start_event\"}"
  fi
  session_curl_config=""
  if [ -n "${ORKWORKS_REPORT_TOKEN:-}" ]; then
    session_curl_config="header = \"Authorization: Bearer $ORKWORKS_REPORT_TOKEN\"\n"
  fi
  harness_session_post_kind="posted"
  session_curl_exit=""
  session_http_status=""
  report_spooled="no"
  if [ "$session_source" = "codex_hook" ] && [ -n "${ORKWORKS_CODEX_SESSION_REPORT_DIR:-}" ]; then
    if python3 -c '
import json, os, sys, tempfile, uuid
report = json.loads(sys.argv[1])
encoded = json.dumps({"report": report}, separators=(",", ":")).encode()
if len(encoded) > 4096:
    raise SystemExit(2)
directory = sys.argv[2]
fd, temporary = tempfile.mkstemp(prefix=".pending-", dir=directory)
try:
    with os.fdopen(fd, "wb") as output:
        output.write(encoded)
        output.flush()
        os.fsync(output.fileno())
    os.replace(temporary, os.path.join(directory, uuid.uuid4().hex + ".json"))
except Exception:
    try:
        os.unlink(temporary)
    except OSError:
        pass
    raise
' "$session_payload" "$ORKWORKS_CODEX_SESSION_REPORT_DIR" 2>/dev/null; then
      report_spooled="yes"
      harness_session_post_kind="enqueued"
    fi
  fi
  if [ "$report_spooled" != "yes" ]; then
    session_curl_exit=0
    session_http_status=$(printf '%b' "$session_curl_config" |
      reporter_curl --config - -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/harness-session" \
        -H "Content-Type: application/json" \
        -d "$session_payload" --output /dev/null --write-out '%{http_code}') || session_curl_exit=$?
  fi
  if { [ "$session_source" = "claude_hook" ] || [ "$session_source" = "copilot_hook" ]; }; then
    if [ "$session_curl_exit" = "0" ] && {
      [ "$session_http_status" = "200" ] || [ "$session_http_status" = "202" ];
    }; then
      native_registration_accepted="yes"
    else
      harness_session_post_kind="failed"
    fi
  fi
elif [ "$session_source" = "codex_hook" ] && [ "$codex_capture_only" != "yes" ]; then
  if [ -z "$harness_session_id" ]; then
    harness_session_post_kind="skipped_no_harness_session_id"
  else
    harness_session_post_kind="skipped_missing_environment"
  fi
elif { [ "$session_source" = "claude_hook" ] || [ "$session_source" = "copilot_hook" ]; } && [ "$codex_capture_only" != "yes" ]; then
  if [ -z "$harness_session_id" ]; then
    harness_session_post_kind="skipped_no_harness_session_id"
  else
    harness_session_post_kind="skipped_missing_authority_environment"
  fi
fi

# Prompt waits are the only Claude/Copilot attention mapping in this shared
# reporter. Their native identity registration above must succeed first; turn
# event mappings remain owned by the follow-up harness-specific issues.
prompt_notification_allowed="no"
if [ "$session_source" = "claude_hook" ] && [ "$event" = "Notification" ]; then
  case "$prompt_notification_type" in
    permission_prompt|elicitation_dialog|elicitation_url_dialog) prompt_notification_allowed="yes" ;;
  esac
elif [ "$session_source" = "copilot_hook" ] && [ "$event" = "notification" ]; then
  case "$prompt_notification_type" in
    permission_prompt|elicitation_dialog) prompt_notification_allowed="yes" ;;
  esac
fi
if [ "$native_registration_accepted" = "yes" ] && [ "$prompt_notification_allowed" = "yes" ]; then
  attention_payload="$(python3 -c 'import json,sys; d={"status":"waiting_for_input","source":sys.argv[1],"event":sys.argv[2],"notificationType":sys.argv[3],"harnessSessionId":sys.argv[5],"promptHookGeneration":sys.argv[4]};
if sys.argv[1] == "copilot_hook" and sys.argv[6]: d["observedAt"]=sys.argv[6]
if sys.argv[7]: d["cwd"]=sys.argv[7]
print(json.dumps(d,separators=(",",":")))' "$session_source" "$event" "$prompt_notification_type" "$ORKWORKS_PROMPT_HOOK_GENERATION" "$harness_session_id" "$prompt_observed_at" "$reported_cwd")"
  attention_curl_config="header = \"Authorization: Bearer $ORKWORKS_REPORT_TOKEN\"\nheader = \"Content-Type: application/json\"\n"
  attention_curl_exit=0
  attention_http_status=$(printf '%b' "$attention_curl_config" |
    reporter_curl --config - -sS --max-time 5 --connect-timeout 2 -X POST "http://127.0.0.1:$ORKWORKS_PORT/sessions/$ORKWORKS_SESSION_ID/attention" \
      -d "$attention_payload" --output /dev/null --write-out '%{http_code}') || attention_curl_exit=$?
  if [ "$attention_curl_exit" = "0" ] && {
    [ "$attention_http_status" = "200" ] || [ "$attention_http_status" = "202" ];
  }; then
    attention_post_kind="posted"
  else
    attention_post_kind="failed"
  fi
fi

# Keep one private, redacted Codex reporter trace for local diagnosis. The
# capture-only exception stores a bounded ordered sequence of sanitized
# PreToolUse, PermissionRequest, and PostToolUse records: allowlisted top-level
# payload key names plus hook_event_name, permission_mode, turn_id, tool_name,
# and bounded tool_use_id scalar values. Never include tool_input, transcript_path, cwd,
# session IDs, tokens, arbitrary free text, full payloads, response bodies,
# or request URLs.
if [ "$session_source" = "codex_hook" ]; then
  diagnostic_path="${HOME:-}/.orkworks/hook-scripts/report-harness-event-diagnostic.json"
  diagnostic_dir=$(dirname "$diagnostic_path")
  if [ -n "${HOME:-}" ] && mkdir -p "$diagnostic_dir" 2>/dev/null; then
    (umask 077
      python3 -c '
import json, os, pathlib, sys, tempfile
import fcntl
path = pathlib.Path(sys.argv[1])
allowed_events = ("PreToolUse", "PermissionRequest", "PostToolUse")
max_capture_events = 16
allowed_scalars = ("hook_event_name", "permission_mode", "turn_id", "tool_name", "tool_use_id")
safe_payload_keys = {"hook_event_name", "model", "permission_mode", "turn_id", "tool_name", "tool_use_id", "tool_response"}
def post_result(kind, curl_exit, http_status):
    if kind == "posted":
        return {"curlExit": int(curl_exit), "httpStatus": http_status}
    return {"result": kind}

def clean_result(value):
    if isinstance(value, dict):
        if set(value) == {"curlExit", "httpStatus"} and type(value.get("curlExit")) is int:
            status = value.get("httpStatus")
            return {"curlExit": value["curlExit"], "httpStatus": status if isinstance(status, str) and len(status) <= 8 else ""}
        result = value.get("result")
        if result in ("posted", "enqueued", "skipped_no_harness_session_id", "skipped_missing_environment", "skipped_capture_only", "not_applicable"):
            return {"result": result}
    return {"result": "not_applicable"}

def clean_capture(value):
    if not isinstance(value, dict):
        return {"payloadKeys": [], "payloadScalars": {}}
    keys = value.get("payloadKeys")
    keys = [key for key in keys if isinstance(key, str) and key in safe_payload_keys] if isinstance(keys, list) else []
    scalars = value.get("payloadScalars")
    scalars = scalars if isinstance(scalars, dict) else {}
    clean_scalars = {}
    for key in allowed_scalars:
        item = scalars.get(key)
        if key == "tool_use_id":
            if isinstance(item, str) and len(item) <= 128:
                clean_scalars[key] = item
        elif isinstance(item, str) and len(item) <= 128:
            clean_scalars[key] = item
        elif type(item) in (int, float, bool):
            clean_scalars[key] = item
    return {"payloadKeys": sorted(set(keys))[:64], "payloadScalars": clean_scalars}

captures = []
lock_path = path.with_name("report-harness-event-diagnostic.lock")
lock_fd = os.open(lock_path, os.O_CREAT | os.O_RDWR, 0o600)
os.fchmod(lock_fd, 0o600)
fcntl.flock(lock_fd, fcntl.LOCK_EX)
try:
    previous = json.loads(path.read_text(encoding="utf-8"))
    old_captures = previous.get("codexPayloadCapture") if isinstance(previous, dict) else None
    if isinstance(old_captures, list):
        for old in old_captures[-max_capture_events:]:
            if isinstance(old, dict) and old.get("event") in allowed_events:
                cleaned = {"event": old["event"], **clean_capture(old)}
                cleaned["attentionPost"] = clean_result(old.get("attentionPost"))
                cleaned["harnessSessionPost"] = clean_result(old.get("harnessSessionPost"))
                captures.append(cleaned)
except Exception:
    pass

event = sys.argv[2]
diagnostic_events = ("SessionStart", "UserPromptSubmit", "PreToolUse", "PermissionRequest", "PostToolUse", "Stop")
diagnostic_event = event if event in diagnostic_events else "Unknown"
if event in allowed_events:
    try:
        raw_capture = json.loads(sys.argv[13] or "{}")
    except (TypeError, ValueError):
        raw_capture = {}
    current = clean_capture(raw_capture)
    current["attentionPost"] = post_result(sys.argv[7], sys.argv[8], sys.argv[9])
    current["harnessSessionPost"] = post_result(sys.argv[10], sys.argv[11], sys.argv[12])
    captures.append({"event": event, **current})

record = {
    "event": diagnostic_event,
    "harnessSessionIdParsed": sys.argv[3] == "yes",
    "orkworksSessionIdPresent": sys.argv[4] == "yes",
    "portPresent": sys.argv[5] == "yes",
    "reportTokenPresent": sys.argv[6] == "yes",
    "attentionPost": post_result(sys.argv[7], sys.argv[8], sys.argv[9]),
    "harnessSessionPost": post_result(sys.argv[10], sys.argv[11], sys.argv[12]),
    "codexPayloadCapture": captures[-max_capture_events:],
}
fd, temporary = tempfile.mkstemp(prefix=".report-harness-event-", dir=path.parent)
try:
    os.fchmod(fd, 0o600)
    with os.fdopen(fd, "w", encoding="utf-8") as output:
        json.dump(record, output, separators=(",", ":"))
        output.write("\n")
    os.replace(temporary, path)
except Exception:
    try:
        os.unlink(temporary)
    except OSError:
        pass
finally:
    os.close(lock_fd)
' "$diagnostic_path" "$event" \
        "$([ -n "$harness_session_id" ] && printf yes || printf no)" \
        "$([ -n "${ORKWORKS_SESSION_ID:-}" ] && printf yes || printf no)" \
        "$([ -n "${ORKWORKS_PORT:-}" ] && printf yes || printf no)" \
        "$([ -n "${ORKWORKS_REPORT_TOKEN:-}" ] && printf yes || printf no)" \
        "$attention_post_kind" "$attention_curl_exit" "$attention_http_status" \
        "$harness_session_post_kind" "$session_curl_exit" "$session_http_status" \
        "${codex_payload_capture:-}" ) >/dev/null 2>&1 || true
  fi
fi
