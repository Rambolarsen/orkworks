#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "$0")" && pwd)"
reporter="$script_dir/report-harness-event.sh"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

mkdir -p "$temp_dir/bin" "$temp_dir/home"
cat > "$temp_dir/bin/curl" <<'CURL'
#!/usr/bin/env bash
set -euo pipefail
printf '%s' "${TEST_HTTP_STATUS:-204}"
exit "${TEST_CURL_EXIT:-0}"
CURL
chmod +x "$temp_dir/bin/curl"

run_reporter() {
  env PATH="$temp_dir/bin:$PATH" HOME="$temp_dir/home" \
    ORKWORKS_SESSION_ID='orkworks-session-secret' \
    ORKWORKS_PORT='4567' \
    ORKWORKS_REPORT_TOKEN='report-token-secret' \
    TEST_HTTP_STATUS="${TEST_HTTP_STATUS:-204}" \
    TEST_CURL_EXIT="${TEST_CURL_EXIT:-0}" \
    bash "$reporter" --marker 'orkworks:harness-integration:codex' --event "$1"
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
}, record
serialized = json.dumps(record)
for secret in ("codex-session-secret", "orkworks-session-secret", "report-token-secret"):
    assert secret not in serialized
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
    PATH="$temp_dir/bin:$PATH" HOME="$temp_dir/home" \
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

printf 'Codex hook reporter diagnostic tests passed.\n'
