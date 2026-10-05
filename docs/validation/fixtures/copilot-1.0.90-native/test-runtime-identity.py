"""Reject a changed executable under normal and optimized Python; never execute it."""
import argparse
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--native", required=True, type=Path)
parser.add_argument("--cache", required=True, type=Path)
parser.add_argument("--inspector", required=True, type=Path)
options = parser.parse_args()


class RuntimeIdentityTest(unittest.TestCase):
    def test_changed_native_is_rejected_even_when_python_is_optimized(self):
        with tempfile.TemporaryDirectory(prefix="ork740-identity-test-") as directory:
            root = Path(directory)
            modified = bytearray(options.native.read_bytes())
            modified[-1] ^= 1  # Outside the embedded SEA asset; no binary is executed.
            native = root / "changed-native"
            native.write_bytes(modified)
            for name, flags, environment in [("normal", [], {}), ("dash-O", ["-O"], {}),
                                             ("environment", [], {"PYTHONOPTIMIZE": "1"})]:
                with self.subTest(mode=name):
                    output = root / name
                    result = subprocess.run([sys.executable, *flags, str(options.inspector.resolve()),
                                             "--native", str(native), "--cache", str(options.cache),
                                             "--output", str(output)], env=environment,
                                            capture_output=True, text=True, timeout=30)
                    self.assertNotEqual(result.returncode, 0, "Changed native was accepted")
                    self.assertIn("Native identity changed", result.stderr)
                    self.assertEqual(result.stdout, "")
                    self.assertFalse(output.exists(), "Rejected input wrote evidence")


if __name__ == "__main__":
    unittest.main(argv=[sys.argv[0]])
