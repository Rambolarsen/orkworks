"""Offline allowance tests: real collector, fake external registry only."""
import importlib.util
import pathlib
import sys
import unittest
from contextlib import contextmanager

spec = importlib.util.spec_from_file_location("sdk_inventory", pathlib.Path(__file__).with_name("pair-windows-sdk-inventory.py"))
m = importlib.util.module_from_spec(spec)
try:
    spec.loader.exec_module(m)
except FileNotFoundError:
    m = None

class FakeRegistry:
    def __init__(self, count=2, change=None):
        self.entries = [{"name": "Windows Software Development Kit - Windows 10.0.26100.9457", "version": "10.1.26100.9457"}] + [{"name": "Unrelated product"}] * (count - 1)
        self.change = change
        self.pass_number = 0
        self.closed = 0
        self.opened = 0
        self.names_read = 0
    @contextmanager
    def open(self, path):
        self.opened += 1
        try:
            yield path
        finally:
            self.closed += 1
    def info(self, handle):
        return (len(self.entries) if handle == m.ROOTS[0] else 0, 7)
    def name(self, handle, index):
        self.names_read += 1
        return str(index)
    def value(self, handle, name, maximum, missing=False):
        if name == "KitsRoot10":
            self.pass_number += 1
            if self.change and self.pass_number == 2:
                self.change(self)
            return "C:\\Program Files (x86)\\Windows Kits\\10\\"
        entry = self.entries[int(handle.rsplit("/", 1)[1])]
        return entry.get("name" if name == "DisplayName" else "version")

class CollectorTests(unittest.TestCase):
    def setUp(self):
        self.assertIsNotNone(m, "bounded SDK collector is not implemented")
    def collect(self, fake, **limits):
        return m.collect(fake, m.Budget(**limits))
    def test_retains_only_matching_products_and_two_stable_passes(self):
        f = FakeRegistry()
        r = self.collect(f)
        self.assertTrue(r["success"], r)
        self.assertEqual(len(r["passes"]), 2)
        self.assertEqual(r["passes"][0]["roots"][0].get("after_count"), 2)
        self.assertEqual(r["passes"][0]["digest"], r["passes"][1]["digest"])
        self.assertEqual(r["passes"][0]["products"][0]["version"], "10.1.26100.9457")
        self.assertNotIn("Unrelated product", str(r))
        self.assertEqual(f.opened, f.closed)
    def test_1024_boundary_and_1025_denial_before_enumeration(self):
        self.assertTrue(self.collect(FakeRegistry(1024))["success"])
        f = FakeRegistry(1025)
        r = self.collect(f)
        self.assertFalse(r["success"])
        self.assertEqual(r["reason"], "entry-limit")
        self.assertEqual(f.names_read, 0)
        self.assertEqual(f.opened, f.closed)
    def test_32_matches_pass_and_33_fail_without_version_read(self):
        f = FakeRegistry(32); f.entries = [f.entries[0]] * 32
        self.assertTrue(self.collect(f)["success"])
        f.entries.append(f.entries[0])
        self.assertEqual(self.collect(f)["reason"], "match-limit")
    def test_value_drift_or_membership_drift_is_unavailable(self):
        for change, reason in [(lambda f: f.entries.__setitem__(0, {"name": f.entries[0]["name"], "version": "10.1.26100.7705"}), "observation-drift"), (lambda f: f.entries.append({"name": "Other"}), "registry-drift")]:
            f = FakeRegistry(change=change)
            self.assertEqual(self.collect(f)["reason"], reason)
            self.assertEqual(f.opened, f.closed)
    def test_missing_required_service_is_not_success(self):
        f = FakeRegistry(); f.entries[0]["version"] = "10.1.26100.7705"
        self.assertEqual(self.collect(f)["reason"], "sdk-service")
    def test_duplicate_names_are_denied(self):
        f = FakeRegistry(); f.name = lambda handle, index: "0"
        self.assertEqual(self.collect(f)["reason"], "duplicate-name")
    def test_hash_limit_retains_bounded_failure_and_closes(self):
        f = FakeRegistry()
        self.assertEqual(self.collect(f, hash_limit=1)["reason"], "hash-limit")
        self.assertEqual(f.opened, f.closed)
    def test_deadline_is_checked_before_registry_access(self):
        f = FakeRegistry()
        self.assertEqual(self.collect(f, seconds=0)["reason"], "time-limit")
        self.assertEqual(f.opened, 0)
    def test_fixed_value_decoding_rejects_type_length_and_termination(self):
        self.assertEqual(m.decode_value("SDK\x00".encode("utf-16-le"), 1, 128), "SDK")
        for data, kind in [(b"A\x00", 1), (b"A\x00\x00", 1), (b"A\x00\x00\x00B\x00\x00\x00", 1), ("SDK\x00".encode("utf-16-le"), 2), (("A"*129+"\x00").encode("utf-16-le"), 1), (b"\x00\xd8\x00\x00", 1)]:
            with self.assertRaises(m.Denied): m.decode_value(data, kind, 128)
    def test_budget_debits_errors_and_reserves_handle_close(self):
        b = m.Budget(api_limit=3)
        b.call(reserve_close=True)
        b.call()
        with self.assertRaises(m.Denied): b.call()
        b.close()
        self.assertEqual(b.calls, 3)
        self.assertEqual(b.handles, 0)
        b = m.Budget(read_limit=4)
        b.read(4)
        with self.assertRaises(m.Denied): b.read(1)

@unittest.skipUnless(sys.platform == "win32", "isolated Windows registry ABI fixture")
class NativeBoundaryTests(unittest.TestCase):
    def test_fixed_buffer_native_reads_errors_and_closure_on_owned_fixture(self):
        import uuid
        import winreg
        name = "SOFTWARE\\OrkWorksSdkInventoryTests\\" + uuid.uuid4().hex
        # Test-only writes: never installed SDK or uninstall roots.
        with winreg.CreateKey(winreg.HKEY_CURRENT_USER, name) as h:
            winreg.SetValueEx(h, "DisplayName", 0, winreg.REG_SZ, "Windows Software Development Kit")
            winreg.SetValueEx(h, "DisplayVersion", 0, winreg.REG_SZ, "10.1.26100.9457")
            winreg.SetValueEx(h, "Oversize", 0, winreg.REG_SZ, "x" * 257)
            winreg.SetValueEx(h, "WrongType", 0, winreg.REG_DWORD, 1)
            with winreg.CreateKey(h, "child"):
                pass
        try:
            budget = m.Budget()
            registry = m.NativeRegistry(budget, hive=-2147483647)
            with registry.open(name) as h:
                self.assertEqual(registry.info(h)[0], 1)
                self.assertEqual(registry.name(h, 0), "child")
                self.assertEqual(registry.value(h, "DisplayVersion", 128), "10.1.26100.9457")
                self.assertIsNone(registry.value(h, "Missing", 256, missing=True))
                for field in ["Oversize", "WrongType"]:
                    with self.assertRaises(m.Denied): registry.value(h, field, 256)
            self.assertEqual(budget.handles, 0)
            self.assertGreater(budget.calls, 0)
            with self.assertRaises(m.Denied):
                with registry.open(name + "\\missing"):
                    pass
            self.assertEqual(budget.handles, 0)
        finally:
            winreg.DeleteKey(winreg.HKEY_CURRENT_USER, name + "\\child")
            winreg.DeleteKey(winreg.HKEY_CURRENT_USER, name)

if __name__ == "__main__": unittest.main()
