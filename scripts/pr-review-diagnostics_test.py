"""Exercise the actual inline workflow diagnostic without a model or credentials."""

import json
import os
from pathlib import Path
import re
import subprocess
import tempfile
import textwrap
import unittest


WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/pr-review.yml"
SECRET = "private-fixture-text-DO-NOT-LOG\n::error::injected"


class ReviewDiagnosticsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.execution = self.root / "claude-execution-output.json"

    def run_diagnostic(self, messages=None, path=None):
        if messages is not None:
            self.execution.write_text(json.dumps(messages), encoding="utf-8")
        workflow = WORKFLOW.read_text(encoding="utf-8")
        block = re.search(
            r"          # BEGIN CLAUDE FAILURE DIAGNOSTIC\n(.*?)"
            r"          # END CLAUDE FAILURE DIAGNOSTIC", workflow, re.S
        )
        self.assertIsNotNone(block, "workflow must diagnose failed Claude executions")
        script = textwrap.dedent(block.group(1))
        env = {
            "PATH": os.environ["PATH"],
            "RUNNER_TEMP": str(self.root),
            "EXECUTION_FILE": str(self.execution) if path is None else str(path),
            "CLAUDE_CODE_OAUTH_TOKEN": SECRET,
            "PYTHONPATH": str(self.root),
        }
        # A checkout/runner-temp module must not shadow the standard library.
        (self.root / "json.py").write_text("raise Exception('shadowed-json')")
        result = subprocess.run(
            ["bash", "--noprofile", "--norc", "-e", "-o", "pipefail", "-c", script],
            env=env, cwd=self.root, capture_output=True, text=True, timeout=5,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stderr, "")
        self.assertNotIn(SECRET, result.stdout)
        self.assertNotIn(str(self.root), result.stdout)
        summary = json.loads(result.stdout)
        self.assertEqual(summary["diagnostic"], "claude-review")
        self.assertLess(len(result.stdout), 600)
        return summary

    def result(self, **overrides):
        return dict(type="result", subtype="success", is_error=True,
                    num_turns=1, total_cost_usd=0, **overrides)

    def test_first_turn_rate_limit_survives_misleading_success_subtype(self):
        summary = self.run_diagnostic([self.result(api_error_status=429, result=SECRET)])
        self.assertEqual(summary["classification"], "rate-limited")
        self.assertEqual(summary["api_error_status"], 429)
        self.assertTrue(summary["zero_cost"])

    def test_string_status_and_model_text_cannot_claim_a_rate_limit(self):
        summary = self.run_diagnostic([self.result(api_error_status="429", result="HTTP 429 " + SECRET)])
        self.assertEqual(summary["classification"], "first-turn-zero-cost")
        self.assertIsNone(summary["api_error_status"])

    def test_mid_run_failure(self):
        message = self.result(api_error_status=503)
        message.update(num_turns=9, total_cost_usd=2.5, subtype="error_during_execution")
        summary = self.run_diagnostic([message])
        self.assertEqual(summary["classification"], "mid-run-error")
        self.assertEqual(summary["num_turns"], 9)
        self.assertFalse(summary["zero_cost"])

    def test_success_with_and_without_structured_output(self):
        message = self.result()
        message.update(is_error=False, num_turns=3, total_cost_usd=1)
        self.assertEqual(self.run_diagnostic([message])["classification"], "missing-structured-output")
        message["structured_output"] = {"report": SECRET}
        self.assertEqual(self.run_diagnostic([message])["classification"], "completed")

    def test_numeric_and_boolean_contracts(self):
        for invalid in (True, "1", -1, 1.5, 10001, float("inf")):
            with self.subTest(turns=invalid):
                message = self.result()
                message["num_turns"] = invalid
                self.assertIsNone(self.run_diagnostic([message])["num_turns"])
        for invalid in (True, "0", -1, float("nan"), float("inf")):
            with self.subTest(cost=invalid):
                message = self.result()
                message["total_cost_usd"] = invalid
                self.assertIsNone(self.run_diagnostic([message])["zero_cost"])
        for invalid in (True, "429", 429.0, 99, 600, {"status": SECRET}):
            with self.subTest(status=invalid):
                self.assertIsNone(self.run_diagnostic([self.result(api_error_status=invalid)])["api_error_status"])
        message = self.result()
        message["is_error"] = "false"
        self.assertEqual(self.run_diagnostic([message])["classification"], "unknown-result")

    def test_arbitrary_fields_and_unknown_subtype_are_not_exposed(self):
        message = self.result(result=SECRET, errors=[SECRET], session_id=SECRET,
                              modelUsage={SECRET: SECRET}, structured_output={"report": SECRET})
        message["subtype"] = SECRET
        summary = self.run_diagnostic([{"type": "assistant", "message": SECRET}, message])
        self.assertEqual(summary["classification"], "unknown-result")
        self.assertIsNone(summary["subtype"])

    def test_missing_file_and_empty_output(self):
        self.assertEqual(self.run_diagnostic()["classification"], "missing-execution-file")
        self.assertEqual(self.run_diagnostic(path="")["classification"], "missing-execution-file")

    def test_malformed_and_unexpected_json(self):
        for content in (SECRET, "[" * 2000, "{\"type\":\"result\"}", "[]", "[null,42]",
                        json.dumps([self.result(), self.result()])):
            with self.subTest(content=content[:30]):
                self.execution.write_text(content, encoding="utf-8")
                summary = self.run_diagnostic()
                self.assertIn(summary["classification"], ("invalid-execution-file", "missing-result", "ambiguous-result"))

    def test_oversized_file(self):
        with self.execution.open("wb") as stream:
            stream.truncate(8 * 1024 * 1024 + 1)
        self.assertEqual(self.run_diagnostic()["classification"], "oversized-execution-file")

    def test_paths_symlinks_hardlinks_and_nonregular_files_are_rejected(self):
        outside = self.root / "private.json"
        outside.write_text(json.dumps([self.result(api_error_status=429)]))
        for path in (outside, self.root / "sub/../private.json"):
            self.assertEqual(self.run_diagnostic(path=path)["classification"], "invalid-execution-path")
        self.execution.symlink_to(outside)
        self.assertEqual(self.run_diagnostic()["classification"], "invalid-execution-file")
        self.execution.unlink()
        os.link(outside, self.execution)
        self.assertEqual(self.run_diagnostic()["classification"], "invalid-execution-file")
        self.execution.unlink()
        self.execution.mkdir()
        self.assertEqual(self.run_diagnostic()["classification"], "invalid-execution-file")
        self.execution.rmdir()
        os.mkfifo(self.execution)
        self.assertEqual(self.run_diagnostic()["classification"], "invalid-execution-file")

    def test_failure_only_step_is_shared_and_cannot_replace_review_outputs(self):
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn("      - &claude-failure-diagnostics\n", workflow)
        self.assertIn("      - *claude-failure-diagnostics\n", workflow)
        step = workflow.split("      - &claude-failure-diagnostics\n", 1)[1].split("\n  # Retry", 1)[0]
        self.assertIn("if: failure() && steps.claude.outcome == 'failure'", step)
        self.assertIn("continue-on-error: true", step)
        self.assertIn("EXECUTION_FILE: ${{ steps.claude.outputs.execution_file }}", step)
        self.assertNotIn("GITHUB_OUTPUT", step)
        self.assertNotIn("secrets.", step)
        self.assertNotIn("uses:", step)
        self.assertIn("if: always() && needs.review.result == 'failure'", workflow)
        self.assertEqual(workflow.count("review_json: ${{ steps.claude.outputs.structured_output }}"), 2)


if __name__ == "__main__":
    unittest.main()
