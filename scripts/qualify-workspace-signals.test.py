import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("qualify-workspace-signals.py")
SECRET = "fixture-secret-must-never-appear"


class WorkspaceProbeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.roots = [Path(self.temp.name) / name for name in ("launch", "task", "other")]
        for root in self.roots:
            root.mkdir()
            (root / "src").mkdir()

    def invoke(self, payload, harness="codex", roots=None, expected_id="native-root"):
        args = [sys.executable, str(SCRIPT), "--harness", harness]
        for name, root in zip(("launch", "task", "other"), roots or self.roots):
            args.extend(["--" + name + "-root", str(root)])
        env = {key: value for key, value in os.environ.items()
               if key != "ORKWORKS_QUALIFY_NATIVE_SESSION_ID"}
        if expected_id is not None:
            env["ORKWORKS_QUALIFY_NATIVE_SESSION_ID"] = expected_id
        return subprocess.run(args, input=payload if isinstance(payload, str) else json.dumps(payload),
                              text=True, capture_output=True, env=env, timeout=5)

    def record(self, payload, **kwargs):
        result = self.invoke(payload, **kwargs)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertLess(len(result.stdout.encode()), 2048)
        self.assertEqual(result.stderr, "")
        return json.loads(result.stdout)

    def test_session_and_tool_directories_stay_distinct(self):
        record = self.record({"session_id": "native-root", "cwd": str(self.roots[0]),
            "tool_input": {"workdir": str(self.roots[1] / "src"), "command": SECRET}})
        self.assertEqual(record["locations"], {"cwd": "launch", "tool_input.workdir": "task"})
        self.assertEqual(record["identityMatches"], {"session_id": True})
        self.assertEqual(record["authority"], "diagnostic-only")
        self.assertNotIn(SECRET, json.dumps(record))

    def test_claude_directory_change_does_not_become_a_binding(self):
        record = self.record({"session_id": "native-root", "cwd": str(self.roots[1]),
            "old_cwd": str(self.roots[0]), "new_cwd": str(self.roots[1])}, harness="claude-code")
        self.assertEqual(record["locations"], {"cwd": "task", "old_cwd": "launch", "new_cwd": "task"})
        self.assertEqual(record["authority"], "diagnostic-only")

    def test_copilot_structured_tool_args_and_child_id_are_observations(self):
        record = self.record({"sessionId": "child", "cwd": str(self.roots[1]),
            "toolArgs": {"cwd": str(self.roots[2])}}, harness="copilot")
        self.assertEqual(record["locations"], {"cwd": "task", "toolArgs.cwd": "other"})
        self.assertEqual(record["identityMatches"], {"sessionId": False})
        self.assertEqual(record["authority"], "diagnostic-only")

    def test_opencode_fields_are_candidates_without_schema_promotion(self):
        record = self.record({"data": {"sessionID": "native-root", "directory": str(self.roots[0]),
            "info": {"directory": str(self.roots[1]), "id": "child"}}}, harness="opencode")
        self.assertEqual(record["locations"], {"data.directory": "launch", "data.info.directory": "task"})
        self.assertEqual(record["identityMatches"], {"data.sessionID": True, "data.info.id": False})
        self.assertEqual(record["authority"], "diagnostic-only")

    def test_unknown_payloads_and_hookless_harnesses_do_not_invent_evidence(self):
        for harness in ("aider", "antigravity", "generic-shell", "gemini", "custom"):
            with self.subTest(harness=harness):
                record = self.record({"unrecognized": {"cwd": str(self.roots[1])}}, harness=harness)
                self.assertEqual(record["locations"], {})
                self.assertEqual(record["identityMatches"], {})
                self.assertEqual(record["authority"], "diagnostic-only")

    def test_identity_equality_never_claims_root_scope(self):
        record = self.record({"session_id": "native-root", "agent_id": "child", "cwd": str(self.roots[2])})
        self.assertEqual(record["identityMatches"], {"session_id": True})
        self.assertEqual(record["rootScope"], "unverified")
        record = self.record({"session_id": "native-root"}, expected_id=None)
        self.assertEqual(record["identityMatches"], {})

    def test_raw_strings_and_paths_never_escape(self):
        result = self.invoke({"cwd": "/outside/" + SECRET, "session_id": SECRET,
            "hook_event_name": SECRET, "tool_response": SECRET, "token": SECRET,
            "toolArgs": "{\"cwd\":\"" + str(self.roots[1]) + "\"}", "data": {"prompt": SECRET}})
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn(SECRET, result.stdout + result.stderr)
        self.assertNotIn(str(self.temp.name), result.stdout + result.stderr)
        record = json.loads(result.stdout)
        self.assertEqual(record["locations"], {"cwd": "invalid"})
        self.assertEqual(record["identityMatches"], {"session_id": False})

    def test_directory_validation_and_real_outside_directory(self):
        outside = Path(self.temp.name) / "outside"
        outside.mkdir()
        cases = [(str(outside), "outside"), ("relative", "invalid"), (None, "invalid"),
                 (str(self.roots[1]) + "\n", "invalid"), (123, "invalid")]
        for value, want in cases:
            with self.subTest(value=value):
                self.assertEqual(self.record({"cwd": value})["locations"], {"cwd": want})

    @unittest.skipIf(os.name == "nt", "symlink creation requires platform privileges")
    def test_symlink_to_task_directory_is_classified_canonically(self):
        alias = Path(self.temp.name) / "alias"
        alias.symlink_to(self.roots[1], target_is_directory=True)
        self.assertEqual(self.record({"cwd": str(alias / "src")})["locations"], {"cwd": "task"})

    def test_invalid_duplicate_and_oversized_payloads_fail_without_echo(self):
        for payload in ('{"cwd":1,"cwd":2}', '[1]', '{"x":', 'x' * 65537):
            with self.subTest(payload=payload[:20]):
                result = self.invoke(payload)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertEqual(result.stderr, "Workspace probe input or roots invalid.\n")

    def test_aliased_nested_missing_and_relative_roots_fail(self):
        for roots in ([self.roots[0], self.roots[0], self.roots[2]],
                      [self.roots[0], self.roots[0] / "src", self.roots[2]],
                      [self.roots[0], Path(self.temp.name) / "missing", self.roots[2]],
                      [Path("relative"), self.roots[1], self.roots[2]]):
            with self.subTest(roots=roots):
                result = self.invoke({}, roots=roots)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertEqual(result.stderr, "Workspace probe input or roots invalid.\n")


if __name__ == "__main__":
    unittest.main()
