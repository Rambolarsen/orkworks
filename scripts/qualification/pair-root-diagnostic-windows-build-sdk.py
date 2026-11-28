"""Guarded SDK library; no live CLI allowance exists for the v3 profile."""
if __name__ == "__main__":
    raise SystemExit("live-entry-disabled")

import importlib.util
from pathlib import Path

_spec = importlib.util.spec_from_file_location("root_diagnostic_sdk_boundary", Path(__file__).with_name("pair-progress-windows-build-sdk.py"))
_boundary = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_boundary)
Budget, Denied, collect_pass = _boundary.Budget, _boundary.Denied, _boundary.collect_pass
