"""Regression tests for bounded, read-only process CPU measurements."""
import importlib.util
from pathlib import Path
import subprocess
import sys
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("cpu_baseline", Path(__file__).with_name("session-cpu-baseline.py"))
cpu = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = cpu
spec.loader.exec_module(cpu)


class BaselineTests(unittest.TestCase):
    def test_cpu_counter_formats(self):
        for value, expected in [("00:01.25", 1.25), ("02:03:04.50", 7384.5), ("1-02:03:04", 93784)]:
            self.assertEqual(cpu.cpu_seconds(value), expected)
        for value in ["nan", "-1", "00:99", "1:2:3:4", "00:NaN"]:
            with self.assertRaises(ValueError):
                cpu.cpu_seconds(value)

    def test_only_explicit_daemon_is_accepted(self):
        row = "42 Wed Nov 18 07:00:00 2026 00:12.50 /a path/orkworksd\n"
        with patch.object(cpu.subprocess, "check_output", return_value=row) as call:
            result = cpu.read_process(42)
        self.assertEqual(result.cpu_seconds, 12.5)
        self.assertEqual(call.call_args.args[0], ["ps", "-p", "42", "-o", "pid=,lstart=,time=,comm="])
        for bad in [row.replace("orkworksd", "codex"), row.replace("42 ", "43 ", 1), ""]:
            with patch.object(cpu.subprocess, "check_output", return_value=bad):
                with self.assertRaises(ValueError):
                    cpu.read_process(42)

    def test_actual_elapsed_time_and_cpu_delta(self):
        before = cpu.Process("start", "/bin/orkworksd", 10)
        after = cpu.Process("start", "/bin/orkworksd", 11)
        with patch.object(cpu, "read_process", side_effect=[before, after]), patch.object(cpu.time, "monotonic", side_effect=[100, 104]), patch.object(cpu.time, "sleep") as sleep:
            result = cpu.measure(42, 3)
        self.assertEqual(result["cpu_percent_one_core"], 25)
        self.assertEqual(result["elapsed_seconds"], 4)
        sleep.assert_called_once_with(3)
        self.assertNotIn("/bin/", str(result))

    def test_replaced_or_reset_process_fails(self):
        before = cpu.Process("start", "/bin/orkworksd", 10)
        for after in [cpu.Process("new", "/bin/orkworksd", 11), cpu.Process("start", "/else/orkworksd", 11), cpu.Process("start", "/bin/orkworksd", 9)]:
            with patch.object(cpu, "read_process", side_effect=[before, after]), patch.object(cpu.time, "monotonic", side_effect=[100, 103]), patch.object(cpu.time, "sleep"):
                with self.assertRaises(ValueError):
                    cpu.measure(42, 3)

    def test_command_failure_does_not_echo_output(self):
        with patch.object(cpu.subprocess, "check_output", side_effect=subprocess.CalledProcessError(1, "ps", output="private")):
            with self.assertRaisesRegex(ValueError, "Cannot inspect") as error:
                cpu.read_process(42)
        self.assertNotIn("private", str(error.exception))

    def test_invalid_inputs_do_not_inspect_process(self):
        with patch.object(cpu, "read_process") as read:
            for pid, seconds in [(0, 3), (-1, 3), (42, 0), (42, 61), (42, float("nan")), (42, float("inf"))]:
                with self.assertRaises(ValueError):
                    cpu.measure(pid, seconds)
            read.assert_not_called()


if __name__ == "__main__":
    unittest.main()
