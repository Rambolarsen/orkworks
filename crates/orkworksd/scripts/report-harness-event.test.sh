#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
reporter="$script_dir/report-harness-event.sh"
temp_dir="$(mktemp -d)"
real_python3="$(command -v python3)"
trap 'rm -rf "$temp_dir"' EXIT

python3 - "$script_dir/report-harness-event.ps1" <<'PY'
from pathlib import Path
import sys

guard = "$data.timestamp -is [ValueType] -and $data.timestamp -isnot [bool] -and $data.timestamp -isnot [DateTime]"
source = Path(sys.argv[1]).read_text()
if source.count(guard) != 2:
    raise SystemExit("PowerShell reporter must reject DateTime-parsed string timestamps in both lifecycle and notification events")
PY

mkdir -p "$temp_dir/bin" "$temp_dir/home"
cat > "$temp_dir/bin/curl" <<'CURL'
#!/usr/bin/env bash
set -euo pipefail
output_path=""
request_body=""
config_stdin=""
while [ $# -gt 0 ]; do
  if [ "$1" = "--output" ] && [ $# -ge 2 ]; then
    output_path="$2"
    shift 2
  elif [ "$1" = "--config" ] && [ "$2" = "-" ]; then
    config_stdin="$(cat)"
    shift 2
  elif [ "$1" = "-d" ] && [ $# -ge 2 ]; then
    request_body="$2"
    shift 2
  else
    shift
  fi
done
if [ -n "${TEST_RESPONSE_BODY:-}" ] && [ "$output_path" != "/dev/null" ]; then
  printf '%s' "$TEST_RESPONSE_BODY"
fi
if [ -n "${TEST_CURL_BODIES_FILE:-}" ]; then
  printf '%s\n' "$request_body" >> "$TEST_CURL_BODIES_FILE"
fi
if [ -n "${TEST_CURL_CONFIGS_FILE:-}" ]; then
  printf '%s\n---\n' "$config_stdin" >> "$TEST_CURL_CONFIGS_FILE"
fi
printf '%s' "${TEST_HTTP_STATUS:-204}"
if [ "${TEST_CURL_EXIT:-0}" -ne 0 ]; then
  printf '%s\n' "${TEST_CURL_STDERR:-curl fixture failure}" >&2
fi
exit "${TEST_CURL_EXIT:-0}"
CURL
chmod +x "$temp_dir/bin/curl"
cat > "$temp_dir/bin/python3" <<'PYTHON'
#!/usr/bin/env bash
set -euo pipefail
if [ -n "${PYTHON3_CALLS_FILE:-}" ]; then
  printf 'call\n' >> "$PYTHON3_CALLS_FILE"
fi
exec "$REAL_PYTHON3" "$@"
PYTHON
chmod +x "$temp_dir/bin/python3"

run_reporter() {
  local reporter_harness="${2:-codex}"
  env -u ORKWORKS_CODEX_SESSION_REPORT_DIR PATH="$temp_dir/bin:$PATH" HOME="$temp_dir/home" \
    ORKWORKS_SESSION_ID='orkworks-session-secret' \
    ORKWORKS_PORT='4567' \
    ORKWORKS_REPORT_TOKEN='report-token-secret' \
    ORKWORKS_PROMPT_HOOK_GENERATION='generation-secret' \
    attention_curl_exit=73 \
    session_curl_exit=74 \
    PYTHON3_CALLS_FILE="$temp_dir/python3-calls" \
    REAL_PYTHON3="$real_python3" \
    TEST_RESPONSE_BODY='response-body-secret' \
    TEST_CURL_STDERR='curl fixture failure' \
    TEST_HTTP_STATUS="${TEST_HTTP_STATUS:-204}" \
    TEST_CURL_EXIT="${TEST_CURL_EXIT:-0}" \
    TEST_CURL_BODIES_FILE="${TEST_CURL_BODIES_FILE:-}" \
    TEST_CURL_CONFIGS_FILE="${TEST_CURL_CONFIGS_FILE:-}" \
    bash "$reporter" --marker "orkworks:harness-integration:$reporter_harness" --event "$1"
}

diagnostic_file="$temp_dir/home/.orkworks/hook-scripts/report-harness-event-diagnostic.json"

printf '%s' '{"session_id":"codex-session-secret","source":"startup"}' |
  run_reporter SessionStart

python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert record == {
    "event": "SessionStart",
    "harnessSessionIdParsed": True,
    "orkworksSessionIdPresent": True,
    "portPresent": True,
    "reportTokenPresent": True,
    "attentionPost": {"result": "not_applicable"},
    "harnessSessionPost": {"curlExit": 0, "httpStatus": "204"},
    "codexPayloadCapture": [],
}, record
serialized = json.dumps(record)
for secret in ("codex-session-secret", "orkworks-session-secret", "report-token-secret", "response-body-secret"):
    assert secret not in serialized
PY

printf '%s' '{"hook_event_name":"PermissionRequest","permission_mode":"default","turn_id":"turn-1","tool_name":"Bash","tool_use_id":"call-123","tool_input":{"command":"private-command-text"}}' |
  run_reporter PermissionRequest
printf '%s' '{"hook_event_name":"PostToolUse","permission_mode":"default","turn_id":"turn-1","tool_name":"Bash","tool_use_id":"call-123","tool_response":"private-response"}' |
  run_reporter PostToolUse

python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
captures = record["codexPayloadCapture"]
assert [capture["event"] for capture in captures] == ["PermissionRequest", "PostToolUse"], captures
assert captures[0]["payloadScalars"]["tool_use_id"] == "call-123", captures
assert captures[1]["payloadScalars"]["tool_use_id"] == "call-123", captures
assert "tool_use_id" in captures[0]["payloadKeys"], captures
assert "tool_use_id" in captures[1]["payloadKeys"], captures
for secret in ("private-command-text", "private-response"):
    assert secret not in json.dumps(record), secret
PY

python3 - <<'PY' | run_reporter PostToolUse
import json
print(json.dumps({"tool_use_id": "x" * 129}))
PY
python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert "tool_use_id" not in record["codexPayloadCapture"][-1]["payloadScalars"], record
PY

printf '%s' '{not-json' | run_reporter PermissionRequest
python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert record["event"] == "PermissionRequest", record
assert record["codexPayloadCapture"][-1]["event"] == "PermissionRequest", record
assert record["codexPayloadCapture"][-1]["payloadKeys"] == [], record
assert record["codexPayloadCapture"][-1]["payloadScalars"] == {}, record
PY

python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

path = pathlib.Path(sys.argv[1])
record = json.loads(path.read_text())
record["codexPayloadCapture"] = [
    {
        "event": "PermissionRequest" if index % 2 == 0 else "PostToolUse",
        "payloadKeys": ["hook_event_name", "session_id"],
        "payloadScalars": {"turn_id": f"turn-{index}", "cwd": "/private/path"},
        "attentionPost": {"result": "posted"},
        "harnessSessionPost": {"result": "posted"},
    }
    for index in range(20)
]
path.write_text(json.dumps(record))
PY
printf '%s' '{"hook_event_name":"PostToolUse","turn_id":"turn-new"}' |
  run_reporter PostToolUse
python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
captures = record["codexPayloadCapture"]
assert len(captures) == 16, captures
assert captures[0]["payloadScalars"]["turn_id"] == "turn-5", captures
assert captures[-1]["event"] == "PostToolUse", captures
assert captures[-1]["payloadScalars"]["turn_id"] == "turn-new", captures
serialized = json.dumps(record)
assert "session_id" not in serialized and "/private/path" not in serialized
PY

: > "$temp_dir/python3-calls"
printf '%s' '{"session_id":"codex-session-secret"}' |
  run_reporter UserPromptSubmit
python_calls="$(wc -l < "$temp_dir/python3-calls")"
if [ "$python_calls" -ne 4 ]; then
  printf 'Expected 4 Python startups for a Codex turn hook, got %s\n' "$python_calls" >&2
  exit 1
fi

python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert record["attentionPost"] == {"curlExit": 0, "httpStatus": "204"}, record
assert record["harnessSessionPost"] == {"curlExit": 0, "httpStatus": "204"}, record
PY

TEST_HTTP_STATUS=403 TEST_CURL_EXIT=0 \
  printf '%s' '{"session_id":"codex-session-secret"}' |
  TEST_HTTP_STATUS=403 TEST_CURL_EXIT=0 run_reporter UserPromptSubmit

python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert record["event"] == "UserPromptSubmit", record
assert record["harnessSessionIdParsed"] is True, record
assert record["attentionPost"] == {"curlExit": 0, "httpStatus": "403"}, record
assert record["harnessSessionPost"] == {"curlExit": 0, "httpStatus": "403"}, record
PY

TEST_HTTP_STATUS=000 TEST_CURL_EXIT=7 \
  printf '%s' '{"session_id":"codex-session-secret"}' |
  TEST_HTTP_STATUS=000 TEST_CURL_EXIT=7 run_reporter UserPromptSubmit

python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert record["event"] == "UserPromptSubmit", record
assert record["attentionPost"] == {"curlExit": 7, "httpStatus": "000"}, record
assert record["harnessSessionPost"] == {"curlExit": 7, "httpStatus": "000"}, record
PY

printf '%s' '{"event":"SessionStart","session_id":17}' |
  run_reporter SessionStart

python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert record["event"] == "SessionStart"
assert record["harnessSessionIdParsed"] is False
assert record["harnessSessionPost"] == {"result": "skipped_no_harness_session_id"}
PY

printf '%s' '{"session_id":"codex-session-secret"}' |
  env -u ORKWORKS_SESSION_ID -u ORKWORKS_PORT -u ORKWORKS_REPORT_TOKEN \
    PATH="$temp_dir/bin:$PATH" HOME="$temp_dir/home" REAL_PYTHON3="$real_python3" \
    bash "$reporter" --marker 'orkworks:harness-integration:codex' --event Stop

python3 - "$diagnostic_file" <<'PY'
import json
import pathlib
import sys

record = json.loads(pathlib.Path(sys.argv[1]).read_text())
assert record["harnessSessionIdParsed"] is True, record
assert record["orkworksSessionIdPresent"] is False, record
assert record["portPresent"] is False, record
assert record["reportTokenPresent"] is False, record
assert record["attentionPost"] == {"result": "skipped_missing_environment"}, record
assert record["harnessSessionPost"] == {"result": "skipped_missing_environment"}, record
PY

codex_stderr="$(printf '%s' '{"session_id":"codex-session-secret"}' |
  TEST_HTTP_STATUS=000 TEST_CURL_EXIT=7 run_reporter UserPromptSubmit codex 2>&1)"
if [ -n "$codex_stderr" ]; then
  printf 'Codex curl errors should be captured by the redacted diagnostic, got: %s\n' "$codex_stderr" >&2
  exit 1
fi

expected_errors='curl fixture failure'
claude_stderr="$(printf '%s' '{"session_id":"claude-session-secret","notification_type":"permission_prompt"}' |
  TEST_HTTP_STATUS=000 TEST_CURL_EXIT=7 run_reporter Notification claude-code 2>&1)"
if [ "$claude_stderr" != "$expected_errors" ]; then
  printf 'Claude curl errors should remain visible, got: %s\n' "$claude_stderr" >&2
  exit 1
fi

copilot_stderr="$(printf '%s' '{"sessionId":"copilot-session-secret","notificationType":"permission_prompt"}' |
  TEST_HTTP_STATUS=000 TEST_CURL_EXIT=7 run_reporter notification copilot 2>&1)"
if [ "$copilot_stderr" != "$expected_errors" ]; then
  printf 'Copilot curl errors should remain visible, got: %s\n' "$copilot_stderr" >&2
  exit 1
fi

request_bodies_file="$temp_dir/prompt-request-bodies.jsonl"
curl_configs_file="$temp_dir/curl-configs.txt"
printf '%s' '{"session_id":"claude-session-secret","notification_type":"permission_prompt","cwd":"/harness-reported/claude"}' |
  TEST_HTTP_STATUS=202 TEST_CURL_BODIES_FILE="$request_bodies_file" TEST_CURL_CONFIGS_FILE="$curl_configs_file" run_reporter Notification claude-code
printf '%s' '{"sessionId":"copilot-session-secret","notification_type":"permission_prompt","timestamp":"2026-09-01T12:00:00Z"}' |
  TEST_HTTP_STATUS=202 TEST_CURL_BODIES_FILE="$request_bodies_file" run_reporter notification copilot
printf '%s' '{"sessionId":"copilot-session-secret","notification_type":"permission_prompt","timestamp":1788264000000}' |
  TEST_HTTP_STATUS=202 TEST_CURL_BODIES_FILE="$request_bodies_file" run_reporter notification copilot
python3 - "$request_bodies_file" <<'PY'
import json
import pathlib
import sys

bodies = [json.loads(line) for line in pathlib.Path(sys.argv[1]).read_text().splitlines()]
assert len(bodies) == 6, bodies
assert bodies[1]["harnessSessionId"] == "claude-session-secret", bodies[1]
assert bodies[1]["promptHookGeneration"] == "generation-secret", bodies[1]
assert bodies[1]["cwd"] == "/harness-reported/claude", bodies[1]
assert "observedAt" not in bodies[3], bodies[3]
assert bodies[5]["observedAt"] == "2026-09-01T12:00:00.000000Z", bodies[5]
PY
python3 - "$curl_configs_file" <<'PY'
import pathlib
import sys

configs = pathlib.Path(sys.argv[1]).read_text()
assert configs.count('Authorization: Bearer report-token-secret') == 2, configs
assert 'Content-Type: application/json' in configs, configs
PY

printf 'Codex hook reporter diagnostic tests passed.\n'
