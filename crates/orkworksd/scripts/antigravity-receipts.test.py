#!/usr/bin/env python3
"""Real POSIX reporter/HTTP fixtures; no native client or inherited capabilities."""
import concurrent.futures
from datetime import datetime
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
import os
from pathlib import Path
import stat
import subprocess
import tempfile
import threading
import unittest

REPORTER = Path(__file__).with_name("report-harness-event.sh")
PAYLOAD = {"conversationId": "native-id-secret", "cwd": "/private/workspace-secret",
           "invocationNum": 0, "fullyIdle": True, "prompt": "prompt-secret"}


class ReceiptTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.receipts = self.root / "receipts"
        self.receipts.mkdir(mode=0o700)
        self.requests = []
        self.response_status = 200
        owner = self

        class Handler(BaseHTTPRequestHandler):
            def do_POST(self):
                body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
                owner.requests.append((self.path.rsplit("/", 1)[-1], body,
                                       self.headers.get("Authorization")))
                self.send_response(owner.response_status)
                self.end_headers()
                self.wfile.write(b"response-body-secret")

            def log_message(self, *args):
                pass

        self.server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
        self.server.daemon_threads = True
        self.thread = threading.Thread(target=self.server.serve_forever, daemon=True)
        self.thread.start()
        self.addCleanup(self.stop_server)

    def stop_server(self):
        self.server.shutdown()
        self.server.server_close()
        self.thread.join(timeout=2)

    def run_reporter(self, event="PreInvocation", payload=None, *, capture=True,
                     destination=None, missing=(), port=None):
        # Deliberately do not inherit reporting capabilities, HOME, or proxies.
        env = {"PATH": os.environ["PATH"], "HOME": str(self.root), "LC_ALL": "C",
               "NO_PROXY": "*", "no_proxy": "*",
               "ORKWORKS_SESSION_ID": "orkworks-id-secret",
               "ORKWORKS_REPORT_TOKEN": "report-token-secret",
               "ORKWORKS_PORT": str(port or self.server.server_port)}
        if capture:
            env["ORKWORKS_ANTIGRAVITY_RECEIPT_DIR"] = str(destination or self.receipts)
        for key in missing:
            env.pop(key, None)
        raw = json.dumps(PAYLOAD if payload is None else payload)
        output = subprocess.run(["bash", str(REPORTER), "--marker",
                                 "orkworks:harness-integration:v2:antigravity",
                                 "--event", event], input=raw, text=True,
                                capture_output=True, env=env, timeout=15)
        self.assertEqual(output.returncode, 0)
        self.assertEqual(output.stdout, "")
        return output

    def records(self):
        records = []
        for file in sorted(self.receipts.glob("receipt-*.json")):
            self.assertLessEqual(file.stat().st_size, 1024)
            self.assertEqual(stat.S_IMODE(file.stat().st_mode), 0o600)
            record = json.loads(file.read_text())
            self.assertEqual(set(record), {"schemaVersion", "receiptId", "observedAt",
                                           "event", "harnessSessionPost", "attentionPost"})
            self.assertEqual(record["schemaVersion"], 1)
            self.assertEqual(len(record["receiptId"]), 32)
            datetime.fromisoformat(record["observedAt"].replace("Z", "+00:00"))
            for secret in ("native-id-secret", "workspace-secret", "prompt-secret",
                           "orkworks-id-secret", "report-token-secret", "response-body-secret"):
                self.assertNotIn(secret, file.read_text())
            records.append(record)
        return records

    def test_first_invocation_retains_both_transport_results_without_content(self):
        self.run_reporter()
        records = self.records()
        self.assertEqual(len(records), 1, "opted-in reporter must publish a receipt")
        record = records[0]
        self.assertEqual(record["event"], "PreInvocation")
        self.assertEqual(record["harnessSessionPost"], {"curlExit": 0, "httpStatus": "200"})
        self.assertEqual(record["attentionPost"], {"curlExit": 0, "httpStatus": "200"})
        self.assertEqual([r[0] for r in self.requests], ["harness-session", "attention"])
        self.assertEqual(self.requests[0][1], {"harnessSessionId": "native-id-secret",
                         "source": "antigravity_hook", "confidence": 0.98,
                         "cwd": "/private/workspace-secret"})
        self.assertEqual(self.requests[1][1]["status"], "working")
        self.assertTrue(all(r[2] == "Bearer report-token-secret" for r in self.requests))

    def test_capture_is_opt_in_and_leaves_requests_unchanged(self):
        self.run_reporter(capture=False)
        self.assertEqual(list(self.receipts.iterdir()), [])
        self.assertFalse((self.root / ".orkworks").exists())
        self.assertEqual([r[0] for r in self.requests], ["harness-session", "attention"])

    def test_event_skips_and_missing_environment_are_explicit(self):
        cases = [
            ("SessionStart", PAYLOAD, (), "posted", "not_applicable"),
            ("PreInvocation", {**PAYLOAD, "invocationNum": 1}, (), "not_applicable", "posted"),
            ("Stop", PAYLOAD, (), "not_applicable", "posted"),
            ("Stop", {**PAYLOAD, "fullyIdle": False}, (), "not_applicable", "not_applicable"),
            ("SessionStart", {"conversationId": 42}, (), "missing_identity", "not_applicable"),
            ("PreInvocation", PAYLOAD, ("ORKWORKS_REPORT_TOKEN",), "posted", "missing_token"),
            ("PreInvocation", PAYLOAD, ("ORKWORKS_SESSION_ID",), "missing_environment", "missing_environment"),
        ]
        for event, payload, missing, identity, attention in cases:
            with self.subTest(event=event, missing=missing, identity=identity):
                before = len(self.records())
                self.run_reporter(event, payload, missing=missing)
                records = self.records()
                self.assertEqual(len(records), before + 1)
                record = records[-1]
                for field, expected in (("harnessSessionPost", identity), ("attentionPost", attention)):
                    want = {"curlExit": 0, "httpStatus": "200"} if expected == "posted" else {"result": expected}
                    self.assertEqual(record[field], want)
        self.assertEqual([r[1]["status"] for r in self.requests if r[0] == "attention"],
                         ["working", "idle"])
        self.run_reporter("arbitrary-event-secret")
        self.assertEqual(len(self.records()), len(cases))

    def test_http_rejection_is_retained_without_changing_reporter_exit(self):
        self.response_status = 401
        self.run_reporter()
        records = self.records()
        self.assertEqual(len(records), 1)
        for field in ("attentionPost", "harnessSessionPost"):
            self.assertEqual(records[0][field], {"curlExit": 0, "httpStatus": "401"})

    def test_transport_failure_is_distinct_from_http_rejection(self):
        # No listener can deliver a response; hosts may refuse or time out.
        import socket
        with socket.socket() as refused:
            refused.bind(("127.0.0.1", 0))
            self.run_reporter(port=refused.getsockname()[1])
        records = self.records()
        self.assertEqual(len(records), 1)
        for field in ("attentionPost", "harnessSessionPost"):
            self.assertIn(records[0][field]["curlExit"], (7, 28))
            self.assertEqual(records[0][field]["httpStatus"], "000")

    def test_unsafe_destinations_do_not_receive_files_or_change_transport(self):
        public = self.root / "public"
        public.mkdir(mode=0o755)
        linked = self.root / "linked"
        linked.symlink_to(self.receipts, target_is_directory=True)
        linked_parent = self.root / "linked-parent"
        linked_parent.symlink_to(self.root, target_is_directory=True)
        plain = self.root / "plain"
        plain.write_text("keep")
        readonly = self.root / "readonly"
        readonly.mkdir(mode=0o500)
        self.addCleanup(readonly.chmod, 0o700)
        destinations = [public, linked, linked_parent / "receipts", plain, readonly,
                        self.root / "absent", Path("relative-receipts")]
        for dest in destinations:
            with self.subTest(destination=dest):
                before = len(self.requests)
                self.run_reporter(destination=dest)
                self.assertEqual(len(self.requests), before + 2)
        self.assertEqual(list(self.receipts.iterdir()), [])
        self.assertEqual(list(public.iterdir()), [])
        self.assertEqual(list(readonly.iterdir()), [])
        self.assertEqual(plain.read_text(), "keep")

    def test_existing_slot_symlink_is_not_followed_or_overwritten(self):
        target = self.root / "keep"
        target.write_text("existing-evidence")
        linked = self.receipts / "receipt-00.json"
        linked.symlink_to(target)
        self.run_reporter()
        self.assertTrue(linked.is_symlink())
        self.assertEqual(target.read_text(), "existing-evidence")
        linked.unlink()
        self.assertEqual(len(self.records()), 1)

    def test_concurrency_capacity_and_existing_slot_never_overwrite(self):
        sentinel = self.receipts / "receipt-00.json"
        sentinel.write_text("existing-evidence")
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            list(pool.map(lambda _: self.run_reporter("Stop", {**PAYLOAD, "fullyIdle": False}), range(40)))
        self.assertEqual(sentinel.read_text(), "existing-evidence")
        sentinel.unlink()
        records = self.records()
        self.assertEqual(len(records), 31)
        self.assertEqual(len({r["receiptId"] for r in records}), 31)
        self.assertEqual(len(list(self.receipts.iterdir())), 31)
        self.assertEqual(self.requests, [])


if __name__ == "__main__":
    unittest.main()
