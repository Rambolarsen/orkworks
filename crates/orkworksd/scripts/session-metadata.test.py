import contextlib
import http.server
import json
import os
import re
import shlex
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
            status, value, headers = replies.pop(0) if replies else (500, {}, {})
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
    def invoke(self, command, port=1, body="", marker="1", proxy_url=None, **environment):
        env = {key: value for key, value in os.environ.items() if not key.startswith("ORKWORKS_") and key != "CODEX_NETWORK_PROXY_ACTIVE"}
        env.update(ORKWORKS_SESSION_ID="session-1", ORKWORKS_PORT=str(port), ORKWORKS_REPORT_TOKEN=TOKEN)
        if marker is not None:
            env["ORKWORKS_SESSION_METADATA_API_VERSION"] = marker
        env.update(environment)
        arguments = [sys.executable, str(SCRIPT), *(command if isinstance(command, list) else [command])]
        if proxy_url is not None:
            arguments += ["--http-proxy", proxy_url]
        return subprocess.run(arguments, input=body, text=True, capture_output=True, env=env, timeout=15)

    def test_ordinary_storage_protocol_bootstraps_without_native_marker(self):
        with endpoint(bootstrap_replies()) as (port, requests):
            result = self.invoke("bootstrap", port, marker=None,
                ORKWORKS_SESSION_STORAGE_PROTOCOL="sidecar-v1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([(r[0], r[1]) for r in requests], [
            ("GET", "/sessions/session-1/metadata"),
            ("PATCH", "/sessions/session-1/metadata?validateOnly=true"),
            ("GET", "/sessions/session-1/metadata")])
        self.assertTrue(json.loads(result.stdout)["validated"])
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_ordinary_response_budget_includes_the_revision_envelope(self):
        metadata = {"id": "session-1", "summary": ""}
        overhead = len(json.dumps(metadata, separators=(",", ":")).encode())
        metadata["summary"] = "x" * (1024 * 1024 - overhead)
        record = {**metadata, "metadataRevision": "7"}
        self.assertEqual(len(json.dumps(metadata, separators=(",", ":")).encode()), 1024 * 1024)
        self.assertGreater(len(json.dumps(record).encode()), 1024 * 1024)
        with endpoint([ok(record)]) as (port, requests):
            result = self.invoke("get", port, marker=None,
                ORKWORKS_SESSION_STORAGE_PROTOCOL="sidecar-v1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), record)
        self.assertEqual(len(requests), 1)

    def test_worktree_response_budget_includes_the_revision_envelope(self):
        metadata = {"id": "session-1", "summary": ""}
        overhead = len(json.dumps(metadata, separators=(",", ":")).encode())
        metadata["summary"] = "x" * (1024 * 1024 - overhead)
        record = {**metadata, "metadataRevision": "7"}
        with endpoint([ok(record), ok(record)]) as (port, requests):
            result = self.invoke(["worktree", "/task/feature"], port, marker=None)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(requests), 2)
        self.assertEqual(requests[-1][3], {"metadataRevision": "7", "patch": {
            "agentMetadata": {"worktreePath": "/task/feature"}}})
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_ordinary_storage_protocol_preserves_stale_revision_conflict(self):
        with endpoint(bootstrap_replies() + [(409, {"secret": TOKEN}, {})]) as (port, requests):
            result = self.invoke("patch", port, json.dumps(PATCH), marker=None,
                ORKWORKS_SESSION_STORAGE_PROTOCOL="sidecar-v1")
        self.assertEqual(result.returncode, 3, result.stderr)
        self.assertEqual(len(requests), 4)
        self.assertEqual(requests[-1][3], PATCH)
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_ordinary_storage_protocol_failure_never_uses_file_fallback(self):
        with tempfile.TemporaryDirectory() as folder:
            target = Path(folder) / "session.json"
            target.write_text("unchanged")
            with endpoint([(503, {"secret": TOKEN}, {})]) as (port, requests):
                result = self.invoke("patch", port, json.dumps(PATCH), marker=None,
                    ORKWORKS_SESSION_STORAGE_PROTOCOL="sidecar-v1", ORKWORKS_METADATA_PATH=str(target))
            self.assertEqual(result.returncode, 2)
            self.assertEqual(len(requests), 1)
            self.assertEqual(target.read_text(), "unchanged")
            self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_unknown_storage_protocol_never_downgrades_to_native_marker(self):
        for value in ("", "sidecar-v2", "Sidecar-v1"):
            with self.subTest(value=value), endpoint([]) as (port, requests):
                result = self.invoke("bootstrap", port,
                    ORKWORKS_SESSION_STORAGE_PROTOCOL=value)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(requests, [])
            self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_unknown_native_marker_never_downgrades_to_ordinary_protocol(self):
        with endpoint([]) as (port, requests):
            result = self.invoke("bootstrap", port, marker="2",
                ORKWORKS_SESSION_STORAGE_PROTOCOL="sidecar-v1")
        self.assertEqual(result.returncode, 2)
        self.assertEqual(requests, [])

    def test_ordinary_instructions_do_not_claim_native_eligibility(self):
        result = self.invoke("instructions", marker=None,
            ORKWORKS_SESSION_STORAGE_PROTOCOL="sidecar-v1")
        self.assertEqual(result.returncode, 0, result.stderr)
        context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("ordinary session storage", context)
        self.assertIn("API-only", context)
        self.assertIn("bootstrap", context)
        self.assertIn("does not authorize", context)
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_ordinary_session_start_command_delivers_api_instructions(self):
        env = {key: value for key, value in os.environ.items() if not key.startswith("ORKWORKS_") and key != "CODEX_NETWORK_PROXY_ACTIVE"}
        env.update(ORKWORKS_SESSION_ID="session-1", ORKWORKS_PORT="1", ORKWORKS_REPORT_TOKEN=TOKEN,
            ORKWORKS_SESSION_STORAGE_PROTOCOL="sidecar-v1")
        result = subprocess.run(["bash", str(SCRIPT.with_name("report-harness-event.sh")),
            "--marker", "orkworks:harness-integration:v2:codex", "--event", "SessionStart"],
            input="{}", text=True, capture_output=True, env=env, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("ordinary session storage", context)
        self.assertIn("API-only", context)
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_ordinary_protocol_explicit_relay_pins_origin(self):
        with endpoint([]) as (origin, direct), endpoint(bootstrap_replies()) as (relay, requests):
            result = self.invoke("bootstrap", origin, marker=None,
                proxy_url=f"http://127.0.0.1:{relay}", ORKWORKS_SESSION_STORAGE_PROTOCOL="sidecar-v1")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(direct, [])
        target = f"http://127.0.0.1:{origin}/sessions/session-1/metadata"
        self.assertEqual([(r[0], r[1]) for r in requests], [
            ("GET", target), ("PATCH", target + "?validateOnly=true"), ("GET", target)])
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_worktree_command_without_general_marker_sends_only_narrow_patch(self):
        for proxy in (False, True):
            for path in ("/task/feature", "--clear"):
                with self.subTest(proxy=proxy, path=path), endpoint([ok(RECORD), ok({**RECORD, "metadataRevision": "8"})]) as (port, requests):
                    result = self.invoke(["worktree", path], 41117 if proxy else port, marker=None,
                        proxy_url=f"http://127.0.0.1:{port}" if proxy else None)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual([r[0] for r in requests], ["GET", "PATCH"])
                self.assertEqual(requests[-1][3], {"metadataRevision": "7", "patch": {"agentMetadata": {"worktreePath": None if path == "--clear" else path}}})
                self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_worktree_automatically_uses_qualified_managed_proxy(self):
        for path in ("/task/feature", "--clear"):
            with self.subTest(path=path), endpoint([]) as (origin, direct), endpoint([
                ok(RECORD), ok({**RECORD, "metadataRevision": "8"})
            ]) as (relay, requests):
                url = f"http://127.0.0.1:{relay}"
                result = self.invoke(["worktree", path], origin, marker=None,
                    CODEX_NETWORK_PROXY_ACTIVE="1", HTTP_PROXY=url, http_proxy=url)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(direct, [])
            target = f"http://127.0.0.1:{origin}/sessions/session-1/metadata"
            self.assertEqual([(r[0], r[1]) for r in requests], [("GET", target), ("PATCH", target)])
            self.assertEqual(requests[-1][3], {"metadataRevision": "7", "patch": {
                "agentMetadata": {"worktreePath": None if path == "--clear" else path}}})
            self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_worktree_ignores_unqualified_ambient_proxy(self):
        for active in ("", "0", "true"):
            with self.subTest(active=active), endpoint([ok(RECORD), ok(RECORD)]) as (origin, direct), endpoint([]) as (relay, requests):
                result = self.invoke(["worktree", "/task/feature"], origin,
                    CODEX_NETWORK_PROXY_ACTIVE=active, HTTP_PROXY=f"http://127.0.0.1:{relay}")
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(len(direct), 2)
            self.assertEqual(requests, [])

    def test_managed_worktree_proxy_rejects_missing_invalid_or_ambiguous_url(self):
        for url, lower in (("", ""), ("http://localhost:80", "http://localhost:80"),
                ("http://127.0.0.1:80", "http://127.0.0.1:81"),
                (f"http://user:{TOKEN}@127.0.0.1:80", "")):
            with self.subTest(url=url), endpoint([]) as (origin, direct):
                result = self.invoke(["worktree", "/task/feature"], origin,
                    CODEX_NETWORK_PROXY_ACTIVE="1", HTTP_PROXY=url, http_proxy=lower)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(direct, [])
            self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_explicit_worktree_proxy_overrides_managed_environment(self):
        with endpoint([ok(RECORD), ok(RECORD)]) as (relay, requests):
            result = self.invoke(["worktree", "/task/feature"], 41117,
                proxy_url=f"http://127.0.0.1:{relay}",
                CODEX_NETWORK_PROXY_ACTIVE="1", HTTP_PROXY="invalid", http_proxy="different")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(requests), 2)

    def test_managed_worktree_proxy_failure_never_falls_back(self):
        for reply, code in (((307, {}, {"Location": "http://127.0.0.1:1/steal"}), 2),
                            ((409, {"secret": TOKEN}, {}), 3)):
            with self.subTest(code=code), endpoint([]) as (origin, direct), endpoint([reply]) as (relay, requests):
                url = f"http://127.0.0.1:{relay}"
                result = self.invoke(["worktree", "/task/feature"], origin,
                    CODEX_NETWORK_PROXY_ACTIVE="1", HTTP_PROXY=url, http_proxy=url)
            self.assertEqual(result.returncode, code)
            self.assertEqual(direct, [])
            self.assertEqual(len(requests), 1)
            self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_managed_proxy_does_not_activate_general_metadata_routing(self):
        with endpoint([ok(RECORD)]) as (origin, direct), endpoint([]) as (relay, requests):
            url = f"http://127.0.0.1:{relay}"
            result = self.invoke("get", origin, CODEX_NETWORK_PROXY_ACTIVE="1", HTTP_PROXY=url, http_proxy=url)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(len(direct), 1)
        self.assertEqual(requests, [])

    def test_worktree_conflict_never_retries_or_prints_capability(self):
        with endpoint([ok(RECORD), (409, {"secret": TOKEN}, {})]) as (port, requests):
            result = self.invoke(["worktree", "/task/feature"], port, marker=None)
        self.assertEqual(result.returncode, 3)
        self.assertEqual(len(requests), 2)
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_worktree_instruction_scope_is_independent_of_general_marker(self):
        result = self.invoke("worktree-instructions", marker=None)
        self.assertEqual(result.returncode, 0, result.stderr)
        context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("worktree", context)
        self.assertNotIn("API-only", context)
        self.assertNotIn(TOKEN, result.stdout + result.stderr)
        for command in ("bootstrap", "get", "patch"):
            with endpoint([]) as (port, requests):
                rejected = self.invoke(command, port, json.dumps(PATCH), marker=None)
            self.assertEqual(rejected.returncode, 2)
            self.assertEqual(requests, [])

    def test_delivered_worktree_proxy_example_reports_to_the_pinned_session(self):
        # An agent must be able to use the delivered example in a sandbox
        # that requires the already-authorized explicit proxy transport.
        for mode, marker in (("worktree-instructions", None), ("instructions", "1")):
            with self.subTest(mode=mode):
                result = self.invoke(mode, marker=marker)
                self.assertEqual(result.returncode, 0, result.stderr)
                context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
                examples = [shlex.split(command) for command in re.findall(r"`([^`]+)`", context)
                    if "worktree ABSOLUTE_PATH --http-proxy TRUSTED_PROXY_URL" in command]
                self.assertEqual(len(examples), 1, "Agent context must deliver an explicit worktree proxy example")
                with endpoint([ok(RECORD), ok({**RECORD, "metadataRevision": "8"})]) as (port, requests):
                    replacements = {"python3": sys.executable, "ABSOLUTE_PATH": "/task/feature",
                        "TRUSTED_PROXY_URL": f"http://127.0.0.1:{port}"}
                    command = [replacements.get(part, part) for part in examples[0]]
                    env = {key: value for key, value in os.environ.items() if not key.startswith("ORKWORKS_") and key != "CODEX_NETWORK_PROXY_ACTIVE"}
                    env.update(ORKWORKS_SESSION_ID="session-1", ORKWORKS_PORT="41117", ORKWORKS_REPORT_TOKEN=TOKEN)
                    reported = subprocess.run(command, env=env, text=True, capture_output=True, timeout=15)
                self.assertEqual(reported.returncode, 0, reported.stderr)
                self.assertEqual([(r[0], r[1]) for r in requests], [
                    ("GET", "http://127.0.0.1:41117/sessions/session-1/metadata"),
                    ("PATCH", "http://127.0.0.1:41117/sessions/session-1/metadata")])
                self.assertEqual(requests[-1][3], {"metadataRevision": "7", "patch": {"agentMetadata": {"worktreePath": "/task/feature"}}})
                self.assertNotIn(TOKEN, result.stdout + result.stderr + reported.stdout + reported.stderr)

    def test_worktree_rejects_wrong_session_patch_response(self):
        with endpoint([ok(RECORD), ok({**RECORD, "id": "other", "metadataRevision": "8"})]) as (port, requests):
            result = self.invoke(["worktree", "/task/feature"], port, marker=None)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(len(requests), 2)

    def test_marker_free_session_start_delivers_only_worktree_instructions(self):
        env = {key: value for key, value in os.environ.items() if not key.startswith("ORKWORKS_") and key != "CODEX_NETWORK_PROXY_ACTIVE"}
        env.update(ORKWORKS_SESSION_ID="session-1", ORKWORKS_PORT="1", ORKWORKS_REPORT_TOKEN=TOKEN)
        result = subprocess.run(["bash", str(SCRIPT.with_name("report-harness-event.sh")), "--marker", "orkworks:harness-integration:v2:codex", "--event", "SessionStart"], input="{}", text=True, capture_output=True, env=env, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)
        context = json.loads(result.stdout)["hookSpecificOutput"]["additionalContext"]
        self.assertIn("task worktree reporting", context)
        self.assertNotIn("API-only", context)
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_explicit_proxy_carries_bootstrap_get_and_patch_to_fixed_origin(self):
        for command in ("bootstrap", "get", "patch"):
            with self.subTest(command=command):
                replies = [ok(RECORD)] if command == "get" else bootstrap_replies()
                if command == "patch":
                    replies += [ok({**RECORD, "metadataRevision": "8"})]
                with endpoint(replies) as (proxy_port, requests):
                    result = self.invoke(command, 41117, json.dumps(PATCH), proxy_url=f"http://127.0.0.1:{proxy_port}/")
                self.assertEqual(result.returncode, 0, result.stderr)
                base = "http://127.0.0.1:41117/sessions/session-1/metadata"
                want = [("GET", base)] if command == "get" else [("GET", base), ("PATCH", base + "?validateOnly=true"), ("GET", base)]
                if command == "patch":
                    want += [("PATCH", base)]
                    self.assertEqual(requests[-1][3], PATCH)
                self.assertEqual([(r[0], r[1]) for r in requests], want)
                self.assertTrue(all(r[2] == "Bearer " + TOKEN for r in requests))
                self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_invalid_explicit_proxy_never_contacts_any_endpoint(self):
        with endpoint([]) as (port, direct_requests), endpoint([]) as (proxy_port, proxy_requests):
            base = f"http://127.0.0.1:{proxy_port}"
            invalid = ["", "http://localhost:80", "http://127.1:80", "http://[::1]:80", "https://127.0.0.1:80", "socks5://127.0.0.1:80", "http://127.0.0.1", "http://127.0.0.1:0", "http://127.0.0.1:65536", "http://127.0.0.1:+80", "http://127.0.0.1:８０", "http://example.com:80", base + "/metadata", base + "?query", base + "#fragment", base + "\n", base.replace("127.", "127.\t"), f"http://user:{TOKEN}@127.0.0.1:{proxy_port}"]
            for url in invalid:
                with self.subTest(url=url):
                    result = self.invoke("bootstrap", port, proxy_url=url)
                    self.assertEqual(result.returncode, 2)
                    self.assertIn("invalid HTTP proxy", result.stderr)
                    self.assertNotIn(TOKEN, result.stdout + result.stderr)
        self.assertEqual(direct_requests, [])
        self.assertEqual(proxy_requests, [])

    def test_proxy_redirect_does_not_forward_capability_or_fall_back(self):
        with endpoint([]) as (port, direct_requests):
            with endpoint([(307, {}, {"Location": f"http://127.0.0.1:{port}/steal"})]) as (proxy_port, requests):
                result = self.invoke("patch", port, json.dumps(PATCH), proxy_url=f"http://127.0.0.1:{proxy_port}")
        self.assertEqual(result.returncode, 2)
        self.assertEqual(len(requests), 1)
        self.assertEqual(direct_requests, [])
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_unavailable_proxy_never_falls_back_to_direct_or_file(self):
        with endpoint([]) as (unavailable_port, _):
            pass
        with tempfile.TemporaryDirectory() as folder, endpoint([]) as (port, requests):
            path = Path(folder) / "session.json"
            path.write_text("unchanged")
            result = self.invoke("patch", port, json.dumps(PATCH), proxy_url=f"http://127.0.0.1:{unavailable_port}", ORKWORKS_METADATA_PATH=str(path))
            self.assertEqual(result.returncode, 2)
            self.assertIn("transport", result.stderr)
            self.assertEqual(path.read_text(), "unchanged")
        self.assertEqual(requests, [])
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_proxy_conflict_keeps_caller_revision_and_does_not_retry(self):
        with endpoint(bootstrap_replies() + [(409, {"secret": TOKEN}, {})]) as (proxy_port, requests):
            result = self.invoke("patch", 41117, json.dumps(PATCH), proxy_url=f"http://127.0.0.1:{proxy_port}")
        self.assertEqual(result.returncode, 3)
        self.assertEqual(len(requests), 4)
        self.assertEqual(requests[-1][3], PATCH)
        self.assertNotIn(TOKEN, result.stdout + result.stderr)

    def test_proxy_invalid_responses_cannot_trigger_mutation(self):
        cases = [[ok(b"not-json")], [ok({**RECORD, "id": "other"})], [ok({**RECORD, "metadataRevision": "07"})], [(502, {"secret": TOKEN}, {})], bootstrap_replies()[:2] + [ok({**RECORD, "metadataRevision": "8"})]]
        for replies in cases:
            with self.subTest(replies=replies), endpoint(replies.copy()) as (proxy_port, requests):
                result = self.invoke("patch", 41117, json.dumps(PATCH), proxy_url=f"http://127.0.0.1:{proxy_port}")
            self.assertEqual(result.returncode, 2)
            self.assertTrue(requests)
            self.assertTrue(all(r[0] == "GET" or r[1].endswith("?validateOnly=true") for r in requests))
            self.assertNotIn(TOKEN, result.stdout + result.stderr)

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
                env = {key: value for key, value in os.environ.items() if not key.startswith("ORKWORKS_") and key != "CODEX_NETWORK_PROXY_ACTIVE"}
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
