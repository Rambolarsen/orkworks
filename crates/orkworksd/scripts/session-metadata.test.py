import contextlib
import http.server
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import unittest

SCRIPT = Path(__file__).with_name("session-metadata.py")
TOKEN = "fixture-capability-not-a-real-token"
RECORD = {"id": "session-1", "metadataRevision": "7", "summary": "keep me"}
PATCH = {"metadataRevision": "7", "patch": {"agentMetadata": {"summary": "new"}}}


@contextlib.contextmanager
def endpoint(replies):
    requests = []

    class Handler(http.server.BaseHTTPRequestHandler):
        def handle_request(self):
            body = self.rfile.read(int(self.headers.get("Content-Length", "0")))
            requests.append((self.command, self.path, self.headers.get("Authorization"), json.loads(body) if body else None))
            status, value, headers = replies.pop(0)
            self.send_response(status)
            for key, value_header in headers.items():
                self.send_header(key, value_header)
            self.end_headers()
            self.wfile.write(value if isinstance(value, bytes) else json.dumps(value).encode())

        do_GET = handle_request
        do_PATCH = handle_request

        def log_message(self, *_args):
            pass

    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    try:
        yield server.server_port, requests
    finally:
        server.shutdown()
        server.server_close()
        thread.join()


def ok(value):
    return 200, value, {}


def bootstrap_replies():
    return [ok(RECORD), ok({"validated": True, "metadataRevision": "7"}), ok(RECORD)]


class ProducerTests(unittest.TestCase):
    def invoke(self, command, port=1, body="", marker="1", **environment):
        env = {key: value for key, value in os.environ.items() if not key.startswith("ORKWORKS_")}
        env.update(ORKWORKS_SESSION_ID="session-1", ORKWORKS_PORT=str(port), ORKWORKS_REPORT_TOKEN=TOKEN)
        if marker is not None:
            env["ORKWORKS_SESSION_METADATA_API_VERSION"] = marker
        env.update(environment)
        return subprocess.run([sys.executable, str(SCRIPT), command], input=body, text=True, capture_output=True, env=env, timeout=15)

    def test_patch_bootstraps_before_writing_and_preserves_caller_revision(self):
        with endpoint(bootstrap_replies() + [ok({**RECORD, "metadataRevision": "8", "summary": "new"})]) as (port, requests):
            result = self.invoke("patch", port, json.dumps(PATCH))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([(r[0], r[1]) for r in requests], [("GET", "/sessions/session-1/metadata"), ("PATCH", "/sessions/session-1/metadata?validateOnly=true"), ("GET", "/sessions/session-1/metadata"), ("PATCH", "/sessions/session-1/metadata")])
        self.assertTrue(all(r[2] == "Bearer " + TOKEN for r in requests))
        self.assertEqual(requests[1][3], {"metadataRevision": "7", "patch": {"agentMetadata": {"summary": None}}})
        self.assertEqual(requests[3][3], PATCH)
        self.assertEqual(json.loads(result.stdout)["metadataRevision"], "8")
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_bootstrap_never_sends_a_mutating_patch(self):
        with endpoint(bootstrap_replies()) as (port, requests):
            result = self.invoke("bootstrap", port)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {"validated": True, "metadataRevision": "7"})
        self.assertEqual([r[0] for r in requests], ["GET", "PATCH", "GET"])
        self.assertTrue(requests[1][1].endswith("?validateOnly=true"))

    def test_bootstrap_followup_read_rejects_revision_change_before_mutation(self):
        replies = bootstrap_replies()[:2] + [ok({**RECORD, "metadataRevision": "8"})]
        with endpoint(replies) as (port, requests):
            result = self.invoke("patch", port, json.dumps(PATCH))
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual([r[0] for r in requests], ["GET", "PATCH", "GET"])
        self.assertTrue(all(r[0] == "GET" or r[1].endswith("?validateOnly=true") for r in requests))

    def test_utf8_patch_within_byte_limit_is_not_expanded_to_ascii_escapes(self):
        patch = {"metadataRevision": "7", "patch": {"agentMetadata": {"summary": "界" * 1400}}}
        body = json.dumps(patch, ensure_ascii=False)
        self.assertLess(len(body.encode("utf-8")), 8192)
        with endpoint(bootstrap_replies() + [ok({**RECORD, "metadataRevision": "8"})]) as (port, requests):
            result = self.invoke("patch", port, body)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(requests[-1][3], patch)

    def test_conflict_requires_reread_without_blind_retry(self):
        with endpoint(bootstrap_replies() + [(409, {"secret": TOKEN}, {})]) as (port, requests):
            result = self.invoke("patch", port, json.dumps(PATCH))
        self.assertEqual(result.returncode, 3)
        self.assertIn("reread", result.stderr)
        self.assertEqual(len(requests), 4)
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_failed_bootstrap_never_writes_or_falls_back_to_file(self):
        for replies in [[(401, {"secret": TOKEN}, {})], [ok(RECORD), (500, {"secret": TOKEN}, {})], [ok(RECORD), ok({"validated": False, "metadataRevision": "7"})]]:
            with self.subTest(replies=replies), tempfile.TemporaryDirectory() as folder:
                path = Path(folder) / "session.json"
                path.write_text("unchanged")
                with endpoint(replies.copy()) as (port, requests):
                    result = self.invoke("patch", port, json.dumps(PATCH), ORKWORKS_METADATA_PATH=str(path))
                self.assertNotEqual(result.returncode, 0)
                self.assertTrue(all(r[1].endswith("?validateOnly=true") or r[0] == "GET" for r in requests))
                self.assertEqual(path.read_text(), "unchanged")
                self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_redirect_cannot_forward_capability(self):
        with endpoint([]) as (other_port, other_requests):
            with endpoint([(307, {}, {"Location": f"http://127.0.0.1:{other_port}/steal"})]) as (port, requests):
                result = self.invoke("bootstrap", port, HTTP_PROXY=f"http://127.0.0.1:{other_port}")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(len(requests), 1)
        self.assertEqual(other_requests, [])

    def test_unknown_or_missing_marker_does_not_contact_sidecar(self):
        with endpoint([]) as (port, requests):
            for marker in (None, "", "2"):
                result = self.invoke("patch", port, json.dumps(PATCH), marker=marker)
                self.assertNotEqual(result.returncode, 0)
        self.assertEqual(requests, [])

    def test_malformed_or_wrong_session_read_does_not_mutate(self):
        for value in (b"not-json", {**RECORD, "id": "other"}, {**RECORD, "metadataRevision": 7}, {**RECORD, "metadataRevision": "07"}, {**RECORD, "metadataRevision": str(2**64)}, b"x" * (1024 * 1024 + 1)):
            with self.subTest(value=str(value)[:60]), endpoint([ok(value)]) as (port, requests):
                result = self.invoke("patch", port, json.dumps(PATCH))
            self.assertNotEqual(result.returncode, 0)
            self.assertEqual(len(requests), 1)

    def test_invalid_environment_is_rejected_before_connecting(self):
        with endpoint([]) as (port, requests):
            for env in ({"ORKWORKS_PORT": "65536"}, {"ORKWORKS_SESSION_ID": "../other"}, {"ORKWORKS_REPORT_TOKEN": "bad\r\nheader"}):
                result = self.invoke("bootstrap", port, **env)
                self.assertNotEqual(result.returncode, 0)
        self.assertEqual(requests, [])

    def test_patch_input_is_bounded_and_requires_an_explicit_revision(self):
        with endpoint([]) as (port, requests):
            for body in ("x" * 8193, "{}", json.dumps({"patch": PATCH["patch"]}), json.dumps({**PATCH, "extra": True})):
                result = self.invoke("patch", port, body)
                self.assertNotEqual(result.returncode, 0)
        self.assertEqual(requests, [])

    def test_duplicate_patch_keys_are_rejected_before_any_request(self):
        body = '{"metadataRevision":"6","metadataRevision":"7","patch":{"agentMetadata":{"summary":"new"}}}'
        with endpoint(bootstrap_replies() + [ok({**RECORD, "metadataRevision": "8"})]) as (port, requests):
            result = self.invoke("patch", port, body)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(requests, [])

    def test_session_start_delivers_instructions_only_for_marked_codex(self):
        with tempfile.TemporaryDirectory(prefix="metadata path '") as folder:
            import shutil
            helper = Path(folder) / SCRIPT.name
            reporter = Path(folder) / "report-harness-event.sh"
            shutil.copyfile(SCRIPT, helper)
            shutil.copyfile(SCRIPT.with_name(reporter.name), reporter)
            for marker, event, harness in (("1", "SessionStart", "codex"), (None, "SessionStart", "codex"), ("1", "Stop", "codex"), ("1", "SessionStart", "claude-code")):
                env = {key: value for key, value in os.environ.items() if not key.startswith("ORKWORKS_")}
                env["HOME"] = folder
                if marker:
                    env["ORKWORKS_SESSION_METADATA_API_VERSION"] = marker
                result = subprocess.run(["bash", str(reporter), "--marker", "orkworks:harness-integration:v2:" + harness, "--event", event], input=json.dumps({"session_id": "native-session", "source": "startup"}), text=True, capture_output=True, env=env, timeout=15)
                self.assertEqual(result.returncode, 0, result.stderr)
                if marker == "1" and event == "SessionStart" and harness == "codex":
                    self.assertTrue(result.stdout, "marked Codex SessionStart must deliver API-only instructions")
                    context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
                    self.assertIn("API-only", context)
                    self.assertIn(str(helper.resolve()).split("'")[0], context)
                else:
                    self.assertEqual(result.stdout, "")

    def test_instructions_are_marker_gated_and_never_contain_capability(self):
        result = self.invoke("instructions")
        self.assertEqual(result.returncode, 0, result.stderr)
        output = json.loads(result.stdout)["hookSpecificOutput"]
        self.assertEqual(output["hookEventName"], "SessionStart")
        self.assertIn("bootstrap", output["additionalContext"])
        self.assertIn("API-only", output["additionalContext"])
        self.assertNotIn(TOKEN, result.stdout + result.stderr)
        for marker in (None, "2"):
            result = self.invoke("instructions", marker=marker)
            self.assertEqual(result.returncode, 0)
            self.assertEqual(result.stdout, "")


if __name__ == "__main__":
    unittest.main()
