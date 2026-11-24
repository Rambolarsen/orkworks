import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

SCRIPT = Path(__file__).with_name("workflow-evidence-inventory.py")


class InventoryTests(unittest.TestCase):
    def invoke(self, cases, **extra):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "packet.json"
            raw = json.dumps({"schemaVersion": 1, "cases": cases, **extra})
            source.write_text(raw)
            result = subprocess.run([sys.executable, str(SCRIPT), str(source)],
                                    capture_output=True, text=True, timeout=5)
            self.assertEqual(source.read_text(), raw)
            return result

    def report(self, result):
        self.assertEqual(result.returncode, 0, result.stderr)
        return json.loads(result.stdout)

    def test_missing_capture_and_labels_are_gaps_not_negative_judgements(self):
        result = self.invoke([{"id": "candidate-01", "sourceGroups": ["source-1"]}])
        self.assertEqual(result.returncode, 0, result.stderr)
        report = self.report(result)
        self.assertEqual(report["candidateSnapshots"], 1)
        self.assertEqual(report["sourceGroups"], 1)
        self.assertEqual(report["suppliedFields"]["rawCapture"], 0)
        self.assertEqual(report["suppliedFields"]["adjudication"], 0)
        self.assertIn("rawCapture", report["cases"][0]["missingFields"])
        self.assertIn("adjudication", report["cases"][0]["missingFields"])
        self.assertEqual(report["semanticQuality"], "not-assessed")
        self.assertNotIn("usefulRate", report)

    def test_shared_sources_do_not_become_independent_incidents(self):
        result = self.invoke([
            {"id": "candidate-01", "sourceGroups": ["source-1"]},
            {"id": "candidate-02", "sourceGroups": ["source-1", "source-2"]},
        ])
        report = self.report(result)
        self.assertEqual(report["candidateSnapshots"], 2)
        self.assertEqual(report["sourceGroups"], 2)
        self.assertEqual(report["incidentCount"], "unknown")

    def test_supplied_fields_do_not_certify_their_truth_or_replay_policy(self):
        result = self.invoke([{
            "id": "candidate-01", "sourceGroups": ["source-1"],
            "rawCapture": {"reference": "private-capture"},
            "adjudication": {"reference": "private-review"},
            "evidenceExcerpt": "DO-NOT-ECHO-RAW-TEXT",
        }])
        self.assertEqual(result.returncode, 0, result.stderr)
        report = self.report(result)
        self.assertEqual(report["suppliedFields"]["rawCapture"], 1)
        self.assertEqual(report["suppliedFields"]["adjudication"], 1)
        self.assertEqual(report["semanticQuality"], "not-assessed")
        self.assertEqual(report["policyReplay"], "not-run")
        self.assertNotIn("DO-NOT-ECHO-RAW-TEXT", result.stdout)

    def test_empty_or_mistyped_evidence_is_not_supplied(self):
        for value in (None, {}, [], "claimed", True):
            with self.subTest(value=value):
                result = self.invoke([{"id": "candidate-01", "sourceGroups": [],
                                       "rawCapture": value}])
                self.assertEqual(self.report(result)["suppliedFields"]["rawCapture"], 0)

    def test_invalid_or_over_bound_packets_fail_without_partial_report(self):
        invalid = [
            [{"id": "same", "sourceGroups": []}] * 2,
            [{"id": str(i), "sourceGroups": []} for i in range(25)],
            [{"id": "candidate", "sourceGroups": "source-1"}],
            [{"id": "candidate", "sourceGroups": [None]}],
        ]
        for cases in invalid:
            with self.subTest(cases=cases):
                result = self.invoke(cases)
                self.assertEqual(result.returncode, 2)
                self.assertEqual(result.stdout, "")
                self.assertEqual(result.stderr, "Evidence packet invalid.\n")

    def test_invalid_encoding_json_schema_and_size_fail_without_echo(self):
        values = [b"{", b"\xff", b'{"schemaVersion":true,"cases":[]}',
                  b'{"schemaVersion":1,"cases":[],"cases":[]}',
                  b'{"schemaVersion":1,"cases":[],"extra":NaN}',
                  b"[" * 1200 + b"]" * 1200, b"x" * 1048577]
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / "packet.json"
            for raw in values:
                with self.subTest(prefix=raw[:32]):
                    source.write_bytes(raw)
                    result = subprocess.run([sys.executable, str(SCRIPT), str(source)],
                                            capture_output=True, text=True, timeout=5)
                    self.assertEqual(result.returncode, 2)
                    self.assertEqual(result.stdout, "")
                    self.assertEqual(result.stderr, "Evidence packet invalid.\n")
                    self.assertEqual(source.read_bytes(), raw)

    def test_empty_collection_reports_zero_without_fabricating_cases(self):
        result = self.invoke([])
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.report(result)["candidateSnapshots"], 0)


if __name__ == "__main__":
    unittest.main()
