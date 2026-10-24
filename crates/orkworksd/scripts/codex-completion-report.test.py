import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import threading
import time
import unittest
import uuid
import importlib.util

SCRIPT = Path(__file__).with_name("codex-completion-report.py")
FINGERPRINT = "a" * 64
STREAM_ID = "b" * 32
GENERATION = 7
EVENT = "PostToolUse"
PAYLOAD = {
    "hook_event_name": EVENT,
    "session_id": "root_1",
    "turn_id": "turn-2",
    "tool_name": "Bash",
    "tool_use_id": "tool_3",
    "tool_input": {"command": "must not persist"},
    "tool_output": "must not persist either",
}


def header(records=None, next_sequence=1, acknowledged=0):
    return {
        "version": 1,
        "streamId": STREAM_ID,
        "generation": GENERATION,
        "hookFingerprint": FINGERPRINT,
        "nextSequence": next_sequence,
        "acknowledgedSequence": acknowledged,
        "records": [] if records is None else records,
    }


def record(sequence):
    return {
        "version": 1,
        "sequence": sequence,
        "observedMicros": 1_798_000_000_000_000 + sequence,
        "hookFingerprint": FINGERPRINT,
        "rootId": "root_1",
        "turnId": "turn-2",
        "event": EVENT,
        "toolName": "Bash",
        "toolUseId": f"tool_{sequence}",
    }


class PublisherTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.directory = Path(self.tmp.name) / "completion"
        self.directory.mkdir(mode=0o700)
        os.chmod(self.directory, 0o700)
        self.write_queue(header())
        fd = os.open(self.directory / "queue.lock", os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
        os.close(fd)

    def write_queue(self, value):
        (self.directory / "queue.json").write_text(json.dumps(value, separators=(",", ":")))
        os.chmod(self.directory / "queue.json", 0o600)

    def write_raw_queue(self, value):
        (self.directory / "queue.json").write_text(value)
        os.chmod(self.directory / "queue.json", 0o600)

    def run_publisher(self, payload=PAYLOAD, event=EVENT, fingerprint=FINGERPRINT, timeout=5):
        body = payload if isinstance(payload, bytes) else json.dumps(payload).encode()
        return subprocess.run(
            [sys.executable, str(SCRIPT), str(self.directory), event, fingerprint],
            input=body, capture_output=True, timeout=timeout,
        )

    def published(self):
        return json.loads((self.directory / "queue.json").read_text())

    def test_publishes_only_allowlisted_scalars_with_source_time_and_durable_names(self):
        before = time.time_ns() // 1000
        result = self.run_publisher()
        after = time.time_ns() // 1000
        self.assertEqual(result.returncode, 0, result.stderr)
        queue = self.published()
        self.assertEqual(queue["nextSequence"], 2)
        self.assertEqual(queue["acknowledgedSequence"], 0)
        item = queue["records"][0]
        self.assertEqual(set(item), {"version", "sequence", "observedMicros", "hookFingerprint", "rootId", "turnId", "event", "toolName", "toolUseId"})
        self.assertEqual({key: item[key] for key in item if key != "observedMicros"}, {
            "version": 1, "sequence": 1, "hookFingerprint": FINGERPRINT,
            "rootId": "root_1", "turnId": "turn-2", "event": EVENT,
            "toolName": "Bash", "toolUseId": "tool_3",
        })
        self.assertGreaterEqual(item["observedMicros"], before)
        self.assertLessEqual(item["observedMicros"], after)
        self.assertNotIn(b"must not persist", (self.directory / "queue.json").read_bytes())
        self.assertEqual(sorted(path.name for path in self.directory.iterdir()), ["queue.json", "queue.lock"])
        self.assertEqual((self.directory / "queue.json").stat().st_mode & 0o777, 0o600)

    def test_missing_optional_scalars_are_present_as_null(self):
        result = self.run_publisher({"hook_event_name": EVENT, "session_id": "root_1"})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.published()["records"][0], {
            "version": 1, "sequence": 1, "observedMicros": self.published()["records"][0]["observedMicros"],
            "hookFingerprint": FINGERPRINT, "rootId": "root_1", "turnId": None,
            "event": EVENT, "toolName": None, "toolUseId": None,
        })

    def test_serial_and_parallel_real_processes_assign_contiguous_sequences(self):
        for _ in range(3):
            result = self.run_publisher()
            self.assertEqual(result.returncode, 0, result.stderr)
        results = []
        threads = [threading.Thread(target=lambda: results.append(self.run_publisher(timeout=10))) for _ in range(12)]
        for thread in threads:
            thread.start()
        for thread in threads:
            thread.join()
        self.assertEqual(len(results), len(threads))
        self.assertTrue(all(result.returncode == 0 for result in results), [result.stderr for result in results])
        queue = self.published()
        self.assertEqual(queue["nextSequence"], 16)
        self.assertEqual([item["sequence"] for item in queue["records"]], list(range(1, 16)))
        self.assertEqual(sorted(path.name for path in self.directory.iterdir()), ["queue.json", "queue.lock"])

    def test_rejects_malformed_duplicate_key_oversized_and_wrong_event_inputs(self):
        cases = [
            (b"not json", EVENT, FINGERPRINT),
            (b'{"hook_event_name":"PostToolUse","hook_event_name":"Stop"}', EVENT, FINGERPRINT),
            (b"x" * (1024 * 1024 + 1), EVENT, FINGERPRINT),
            (json.dumps({**PAYLOAD, "hook_event_name": "Stop"}).encode(), EVENT, FINGERPRINT),
            (json.dumps(PAYLOAD).encode(), "Unknown", FINGERPRINT),
            (json.dumps(PAYLOAD).encode(), EVENT, "bad"),
        ]
        for body, event, fingerprint in cases:
            with self.subTest(size=len(body), event=event):
                result = self.run_publisher(body, event, fingerprint)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stderr, b"completion report failed\n")
                self.assertTrue((self.directory / "poison").exists())
                self.assertEqual(len(list(self.directory.glob("inflight-*"))), 1)
                self.assertEqual(self.published(), header())
                for marker in self.directory.glob("inflight-*"):
                    marker.unlink()
                (self.directory / "poison").unlink()
                self.write_queue(header())

    def test_rejects_unknown_duplicate_and_non_scalar_queue_state_without_rewriting(self):
        duplicate_queue = '{"version":1,"version":1,"streamId":"' + STREAM_ID + '","generation":7,"hookFingerprint":"' + FINGERPRINT + '","nextSequence":1,"acknowledgedSequence":0,"records":[]}'
        self.write_raw_queue(duplicate_queue)
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stderr, b"completion report failed\n")
        self.assertTrue((self.directory / "poison").exists())
        self.assertEqual(len(list(self.directory.glob("inflight-*"))), 1)
        for marker in self.directory.glob("inflight-*"):
            marker.unlink()
        (self.directory / "poison").unlink()
        invalid_headers = [
            {**header(), "unexpected": 1},
            {**header(), "generation": True},
            {**header(), "nextSequence": 1.0},
            {**header(), "records": [record(2)], "nextSequence": 2},
            {**header(), "records": [record(1)], "nextSequence": 3},
            {**header(), "nextSequence": 1 << 64},
            {**header(), "records": [record(1) | {"rootId": 7}], "nextSequence": 2},
            {**header(), "records": [record(1) | {"rootId": None}], "nextSequence": 2},
        ]
        for value in invalid_headers:
            with self.subTest(value=value):
                self.write_queue(value)
                original = (self.directory / "queue.json").read_bytes()
                result = self.run_publisher()
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stderr, b"completion report failed\n")
                self.assertEqual((self.directory / "queue.json").read_bytes(), original)
                self.assertTrue((self.directory / "poison").exists())
                for marker in self.directory.glob("inflight-*"):
                    marker.unlink()
                (self.directory / "poison").unlink()

    def test_overflow_and_counter_exhaustion_fail_closed(self):
        full = [record(sequence) for sequence in range(1, 257)]
        self.write_queue(header(full, next_sequence=257))
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        self.assertEqual([item["sequence"] for item in self.published()["records"]], list(range(1, 257)))
        self.assertTrue((self.directory / "poison").exists())
        self.write_queue(header(next_sequence=(1 << 64) - 1))
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        self.assertEqual(self.published()["nextSequence"], (1 << 64) - 1)
        self.assertTrue((self.directory / "poison").exists())

    def test_directory_enumeration_is_bounded_before_append(self):
        for index in range(513):
            path = self.directory / f"unexpected-{index:04d}"
            path.touch(mode=0o600)
        module_spec = importlib.util.spec_from_file_location("completion_publisher", SCRIPT)
        module = importlib.util.module_from_spec(module_spec)
        module_spec.loader.exec_module(module)
        dir_fd = os.open(self.directory, os.O_RDONLY | os.O_DIRECTORY)
        try:
            with self.assertRaises(module.Invalid):
                module._list_names_bounded(dir_fd)
        finally:
            os.close(dir_fd)
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stderr, b"completion report failed\n")
        self.assertEqual(self.published(), header())
        self.assertTrue((self.directory / "poison").exists())

    def test_queue_inode_replaced_after_read_is_not_adopted(self):
        module_spec = importlib.util.spec_from_file_location("completion_publisher", SCRIPT)
        module = importlib.util.module_from_spec(module_spec)
        module_spec.loader.exec_module(module)
        replacement = header(next_sequence=2, records=[record(1)])
        replacement_bytes = json.dumps(replacement, separators=(",", ":")).encode()
        original_verify = module._verify_named_inode
        replaced = False

        def replace_after_initial_queue_read(dir_fd, name, info):
            nonlocal replaced
            original_verify(dir_fd, name, info)
            if name == "queue.json" and not replaced:
                replaced = True
                os.unlink("queue.json", dir_fd=dir_fd)
                fd = os.open("queue.json", os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600, dir_fd=dir_fd)
                try:
                    os.write(fd, replacement_bytes)
                    os.fsync(fd)
                finally:
                    os.close(fd)
                os.fsync(dir_fd)

        module._verify_named_inode = replace_after_initial_queue_read
        read_fd, write_fd = os.pipe()
        os.write(write_fd, json.dumps(PAYLOAD).encode())
        os.close(write_fd)
        with os.fdopen(read_fd, "rb") as stream:
            original_stdin = sys.stdin
            sys.stdin = stream
            try:
                with self.assertRaises(module.Invalid):
                    module.publish(str(self.directory), EVENT, FINGERPRINT)
            finally:
                sys.stdin = original_stdin
        self.assertTrue(replaced)
        self.assertEqual((self.directory / "queue.json").read_bytes(), replacement_bytes)
        self.assertTrue((self.directory / "poison").exists())

    def test_marker_bound_and_invalid_native_scalars_fail_closed(self):
        self.write_queue(header())
        for index in range(256):
            marker = self.directory / f"inflight-{index:032x}"
            marker.write_text("")
            os.chmod(marker, 0o600)
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        self.assertEqual(len(list(self.directory.glob("inflight-*"))), 257)
        self.assertTrue((self.directory / "poison").exists())

        for marker in self.directory.glob("inflight-*"):
            marker.unlink()
        (self.directory / "poison").unlink()
        self.write_queue(header())
        for payload in (
            {**PAYLOAD, "session_id": "id with space"},
            {**PAYLOAD, "turn_id": "é"},
            {**PAYLOAD, "tool_name": "tool name"},
            {**PAYLOAD, "tool_use_id": "x" * 129},
        ):
            result = self.run_publisher(payload)
            self.assertEqual(result.returncode, 2)
            self.assertEqual(result.stderr, b"completion report failed\n")
            self.assertTrue((self.directory / "poison").exists())
            for marker in self.directory.glob("inflight-*"):
                marker.unlink()
            (self.directory / "poison").unlink()
            self.write_queue(header())

    def test_retains_own_inflight_marker_on_failure_and_checks_preexisting_poison(self):
        self.write_queue({"broken": True})
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        markers = list(self.directory.glob("inflight-*"))
        self.assertEqual(len(markers), 1)
        self.assertRegex(markers[0].name, r"^inflight-[0-9a-f]{32}$")
        self.assertTrue((self.directory / "poison").exists())
        self.write_queue(header())
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        self.assertEqual(self.published(), header())
        self.assertEqual(len(list(self.directory.glob("inflight-*"))), 2)

    def test_retained_other_writer_marker_survives_another_publication(self):
        marker = self.directory / ("inflight-" + uuid.uuid4().hex)
        marker.write_text("")
        os.chmod(marker, 0o600)
        result = self.run_publisher()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual([item["sequence"] for item in self.published()["records"]], [1])
        self.assertTrue(marker.exists())
        self.assertEqual(len(list(self.directory.glob("inflight-*"))), 1)

    def test_lock_timeout_leaves_marker_and_poison_without_touching_queue(self):
        import fcntl
        lock_fd = os.open(self.directory / "queue.lock", os.O_RDONLY)
        fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        try:
            original = (self.directory / "queue.json").read_bytes()
            started = time.monotonic()
            result = self.run_publisher(timeout=5)
            elapsed = time.monotonic() - started
        finally:
            os.close(lock_fd)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stderr, b"completion report failed\n")
        self.assertEqual((self.directory / "queue.json").read_bytes(), original)
        self.assertGreaterEqual(elapsed, 1.8)
        self.assertLess(elapsed, 3.0)
        self.assertEqual(len(list(self.directory.glob("inflight-*"))), 0)
        self.assertTrue((self.directory / "poison").exists())

    def test_no_follow_and_replaced_lock_inode_fail_closed(self):
        real_queue = self.directory / "real-queue"
        (self.directory / "queue.json").rename(real_queue)
        os.symlink(real_queue, self.directory / "queue.json")
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        self.assertTrue((self.directory / "poison").exists())
        (self.directory / "queue.json").unlink()
        real_queue.rename(self.directory / "queue.json")
        (self.directory / "queue.lock").unlink()
        (self.directory / "queue.lock").write_text("")
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        self.assertEqual(self.published(), header())
        self.assertTrue((self.directory / "poison").exists())

    def test_fifo_lock_queue_and_marker_are_rejected_without_hanging(self):
        for name in ("queue.lock", "queue.json", "inflight-" + ("c" * 32)):
            with self.subTest(name=name):
                path = self.directory / name
                if path.exists():
                    path.unlink()
                os.mkfifo(path, 0o600)
                timed_out = False
                try:
                    result = self.run_publisher(timeout=2)
                except subprocess.TimeoutExpired:
                    timed_out = True
                finally:
                    if path.exists():
                        path.unlink()
                    for marker in self.directory.glob("inflight-*"):
                        marker.unlink()
                    poison = self.directory / "poison"
                    if poison.exists():
                        poison.unlink()
                    self.write_queue(header())
                    if not (self.directory / "queue.lock").exists():
                        fd = os.open(self.directory / "queue.lock", os.O_CREAT | os.O_EXCL | os.O_WRONLY, 0o600)
                        os.close(fd)
                self.assertFalse(timed_out, f"publisher blocked opening FIFO {name}")
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stderr, b"completion report failed\n")

    def test_replaced_retained_lock_during_publication_fails_closed(self):
        process = subprocess.Popen(
            [sys.executable, str(SCRIPT), str(self.directory), EVENT, FINGERPRINT],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        )
        deadline = time.monotonic() + 3
        while not list(self.directory.glob("inflight-*")) and time.monotonic() < deadline:
            time.sleep(0.01)
        self.assertTrue(list(self.directory.glob("inflight-*")), "publisher did not create its marker")
        original = (self.directory / "queue.json").read_bytes()
        (self.directory / "queue.lock").unlink()
        (self.directory / "queue.lock").write_text("")
        os.chmod(self.directory / "queue.lock", 0o600)
        _stdout, stderr = process.communicate(json.dumps(PAYLOAD).encode(), timeout=5)
        self.assertEqual(process.returncode, 2)
        self.assertEqual(stderr, b"completion report failed\n")
        self.assertEqual((self.directory / "queue.json").read_bytes(), original)
        self.assertTrue((self.directory / "poison").exists())

    def test_directory_must_be_private_owned_and_nofollow(self):
        os.chmod(self.directory, 0o750)
        result = self.run_publisher()
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stderr, b"completion report failed\n")
        os.chmod(self.directory, 0o700)
        linked_directory = self.directory.parent / "linked"
        linked_directory.symlink_to(self.directory, target_is_directory=True)
        result = subprocess.run([sys.executable, str(SCRIPT), str(linked_directory), EVENT, FINGERPRINT], input=json.dumps(PAYLOAD).encode(), capture_output=True)
        self.assertEqual(result.returncode, 2)
        self.assertEqual(result.stderr, b"completion report failed\n")

    def test_does_not_create_stream_and_legacy_files_coexist(self):
        missing = self.directory.parent / "missing"
        result = subprocess.run([sys.executable, str(SCRIPT), str(missing), EVENT, FINGERPRINT], input=json.dumps(PAYLOAD).encode(), capture_output=True)
        self.assertEqual(result.returncode, 2)
        self.assertFalse(missing.exists())
        legacy = self.directory.parent / "approval-mailbox.json"
        legacy.write_text("legacy unchanged")
        result = self.run_publisher()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(legacy.read_text(), "legacy unchanged")
        self.assertEqual(sorted(path.name for path in self.directory.iterdir()), ["queue.json", "queue.lock"])


if __name__ == "__main__":
    unittest.main()
