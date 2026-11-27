"""Offline allowance tests: real collector, fake external registry only."""
import importlib.util
import pathlib
import sys
import unittest
from contextlib import contextmanager

spec = importlib.util.spec_from_file_location("sdk_positive", pathlib.Path(__file__).with_name("pair-windows-sdk-positive-record.py"))
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
        self.value_reads = 0
        self.version_reads = 0
        self.require_parent = False
        self.split = None
    @contextmanager
    def open(self, path, parent=None):
        if self.require_parent and parent is None and path not in (*m.ROOTS, m.INSTALLED):
            raise m.Denied("parent-binding")
        path = parent + "/" + path if parent is not None else path
        self.opened += 1
        try:
            yield path
        finally:
            self.closed += 1
    def info(self, handle):
        count = len(self.entries) if handle == m.ROOTS[0] else 0
        if self.split is not None:
            count = self.split if handle == m.ROOTS[0] else len(self.entries)-self.split
        return (count, 7)
    def name(self, handle, index):
        self.names_read += 1
        return str(index)
    def raw_value(self, handle, name, maximum, missing=False):
        self.value_reads += 1
        self.version_reads += int(name == "DisplayVersion")
        if name == "KitsRoot10":
            self.pass_number += 1
            if self.change and self.pass_number == 2:
                self.change(self)
            return (1, "C:\\Program Files (x86)\\Windows Kits\\10\\\x00".encode("utf-16-le"))
        index = int(handle.rsplit("/", 1)[1])
        if self.split is not None and handle.startswith(m.ROOTS[1]+"/"):
            index += self.split
        entry = self.entries[index]
        value = entry.get("name" if name == "DisplayName" else "version")
        if value is None and not missing:
            raise m.Denied("registry-value")
        return None if value is None else (value if isinstance(value, tuple) else (1, (value+"\x00").encode("utf-16-le")))

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
    def test_unclassified_before_positive_is_explicit_and_private(self):
        f = FakeRegistry()
        f.entries.insert(0, {"name": "private-é"})
        f.entries.extend([{"name": None}, {"name": "plain unrelated"}])
        r = self.collect(f)
        self.assertTrue(r["success"], r)
        self.assertEqual(r["schema"], 2)
        self.assertEqual(r["evidence_kind"], "sdk-positive-product-observation")
        self.assertTrue(r["positive_record_observed"])
        for flag in ["inventory_complete", "runtime_qualified", "replacement_pin_accepted"]:
            self.assertIs(r[flag], False)
        c = r["passes"][0]["classification"]
        self.assertEqual(c, {"accepted_ascii": 3, "absent_name": 1, "unclassified": 1,
                             "predicate_match": 1, "predicate_nonmatch": 2,
                             "reasons": {"non_ascii_or_nonprintable": 1}})
        self.assertNotIn("private", str(r))
        self.assertNotIn("sha256", str(r))

    def test_each_local_name_refusal_is_unclassified_with_fixed_reason(self):
        for value, reason in [((4, b"abcd"), "unsupported_type"),
                              ((1, b"x"), "invalid_length"),
                              ((1, b"\x00\xd8\x00\x00"), "invalid_utf16"),
                              ((1, b"A\x00"), "invalid_termination"),
                              ((1, b"\x00\x00"), "empty"),
                              ((1, "é\x00".encode("utf-16-le")), "non_ascii_or_nonprintable")]:
            with self.subTest(reason=reason):
                f = FakeRegistry(); f.entries[1] = {"name": value}
                r = self.collect(f)
                self.assertTrue(r["success"], r)
                self.assertEqual(r["passes"][0]["classification"]["reasons"], {reason: 1})

    def test_same_length_unclassified_payload_change_denies_stability(self):
        f = FakeRegistry(change=lambda f: f.entries.__setitem__(1, {"name": (4, b"efgh")}))
        f.entries[1] = {"name": (4, b"abcd")}
        r = self.collect(f)
        self.assertEqual(r["reason"], "observation-drift")
        self.assertFalse(r["positive_record_observed"])

    def test_unknown_only_never_proves_a_positive_record(self):
        f = FakeRegistry(1); f.entries = [{"name": "private-é"}]
        r = self.collect(f)
        self.assertFalse(r["success"])
        self.assertFalse(r["positive_record_observed"])
        self.assertEqual(r["reason"], "sdk-service")

    def test_invalid_matching_version_is_terminal_not_unclassified(self):
        f = FakeRegistry(); f.entries[0]["version"] = "private-é"
        r = self.collect(f)
        self.assertFalse(r["success"])
        self.assertEqual(r["reason"], "non_ascii_or_nonprintable")
        self.assertEqual(f.opened, f.closed)

    def test_payload_hashing_is_precharged_and_has_separate_cap(self):
        b = m.Budget(hash_limit=3)
        self.assertEqual(b.raw_digest(b"abc"), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")
        with self.assertRaisesRegex(m.Denied, "hash-limit"): b.raw_digest(b"d")
        self.assertEqual(b.hashed_bytes, 3)

    def test_category_ledgers_reject_excess_raw_or_row_work(self):
        b = m.Budget()
        block = b"x" * 514
        for _ in range(2082): b.raw_digest(block)
        self.assertEqual(b.hashed_bytes, 1070148)
        with self.assertRaisesRegex(m.Denied, "hash-category-limit"): b.raw_digest(block)
        self.assertEqual(b.hashed_bytes, 1070148)
        b = m.Budget()
        with self.assertRaisesRegex(m.Denied, "hash-category-limit"):
            b.pass_digest([[str(i), "x"*1990] for i in range(2200)], {})
        self.assertLessEqual(b.hashed_bytes, 4194304)

    def test_encoded_row_and_envelope_caps_deny_before_hashing(self):
        for rows, envelope, reason in [([["x"*2049]], {}, "row-limit"), ([], "x"*131073, "envelope-limit")]:
            b = m.Budget()
            with self.assertRaisesRegex(m.Denied, reason): b.pass_digest(rows, envelope)
            self.assertEqual(b.hashed_bytes, 0)

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
    def test_empty_registry_text_retains_only_fixed_format_reason(self):
        with self.assertRaisesRegex(m.Denied, "^(empty|non_ascii_or_nonprintable)$"):
            m.decode_value("\x00".encode("utf-16-le"), 1, 128)
    def test_control_characters_retain_only_fixed_format_reason(self):
        for value in ["private\nvalue", "private\tvalue", "private\x7fvalue"]:
            with self.subTest(value=value):
                with self.assertRaisesRegex(m.Denied, "^(empty|non_ascii_or_nonprintable)$"):
                    m.decode_value((value+"\x00").encode("utf-16-le"), 1, 128)
    def test_non_ascii_text_retains_only_fixed_format_reason(self):
        for value in ["private-é", "private-中文", "private-😀"]:
            with self.subTest(value=value):
                with self.assertRaisesRegex(m.Denied, "^(empty|non_ascii_or_nonprintable)$"):
                    m.decode_value((value+"\x00").encode("utf-16-le"), 1, 128)
    def test_printable_ascii_boundaries_remain_accepted(self):
        value = "".join(chr(code) for code in range(32,127))
        self.assertEqual(m.decode_value((value+"\x00").encode("utf-16-le"), 1, 128), value)
        for maximum in [128, 256]:
            with self.subTest(maximum=maximum):
                boundary = "x"*maximum
                self.assertEqual(m.decode_value((boundary+"\x00").encode("utf-16-le"), 1, maximum), boundary)

    def test_foreign_kit_root_is_not_exported_on_failure(self):
        f = FakeRegistry()
        original = f.raw_value
        f.raw_value = lambda h,n,*a,**kw: (1, "C:/Users/private-person/secret\x00".encode("utf-16-le")) if n == "KitsRoot10" else original(h,n,*a,**kw)
        r = self.collect(f)
        self.assertFalse(r["success"])
        self.assertNotIn("private-person", str(r))
    def test_children_remain_bound_to_the_enumerated_root_handle(self):
        f = FakeRegistry(); f.require_parent = True
        self.assertTrue(self.collect(f)["success"])
    def test_value_allowance_and_33rd_match_does_not_read_version(self):
        f = FakeRegistry(1024); sdk = f.entries[0]
        f.entries = [sdk] * 32 + [{"name": None}] * 992
        f.split = 512
        self.assertTrue(self.collect(f)["success"])
        self.assertEqual(f.value_reads, 2114)
        self.assertEqual(f.version_reads, 64)
        f = FakeRegistry(33); f.entries = [sdk] * 33
        self.assertEqual(self.collect(f)["reason"], "match-limit")
        self.assertEqual(f.version_reads, 32)
    def test_missing_mandatory_sdk_version_is_unavailable(self):
        f = FakeRegistry()
        f.entries.append({"name": "Windows Software Development Kit - incomplete"})
        self.assertEqual(self.collect(f)["reason"], "registry-value")
        self.assertEqual(f.opened, f.closed)
    def test_failed_close_keeps_outstanding_handle_count(self):
        b = m.Budget(); b.call(reserve_close=True)
        b.close(success=False)
        self.assertEqual(b.calls, 2)
        self.assertEqual(b.handles, 1)
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
        self.assertTrue(m.runtime_identity(), "fixed isolated interpreter selector must match before any fixture access")
        name = "SOFTWARE\\OrkWorksSdkPositiveTests\\" + uuid.uuid4().hex
        # Test-only writes: never installed SDK or uninstall roots.
        with winreg.CreateKey(winreg.HKEY_CURRENT_USER, name) as h:
            winreg.SetValueEx(h, "DisplayName", 0, winreg.REG_SZ, "Windows Software Development Kit")
            winreg.SetValueEx(h, "DisplayVersion", 0, winreg.REG_SZ, "10.1.26100.9457")
            winreg.SetValueEx(h, "Oversize", 0, winreg.REG_SZ, "x" * 257)
            winreg.SetValueEx(h, "WrongType", 0, winreg.REG_DWORD, 1)
            winreg.SetValueEx(h, "UnicodeName", 0, winreg.REG_SZ, "private-é")
            with winreg.CreateKey(h, "child"):
                pass
        try:
            budget = m.Budget()
            registry = m.NativeRegistry(budget, hive=-2147483647)
            with registry.open(name) as h:
                self.assertEqual(registry.info(h)[0], 1)
                with registry.open("child", parent=h) as child:
                    self.assertEqual(registry.info(child)[0], 0)
                before = budget.read_bytes
                self.assertEqual(registry.name(h, 0), "child")
                self.assertEqual(budget.read_bytes - before, 516)
                before = budget.read_bytes
                self.assertEqual(registry.value(h, "DisplayVersion", 128), "10.1.26100.9457")
                self.assertEqual(budget.read_bytes - before, 266)
                self.assertIsNone(registry.value(h, "Missing", 256, missing=True))
                observation, text = m.observe_value(registry, budget, h, "UnicodeName", 256, optional=True)
                self.assertIsNone(text)
                self.assertEqual(observation["state"], "unclassified")
                self.assertEqual(observation["reason"], "non_ascii_or_nonprintable")
                self.assertEqual(observation["kind"], 1)
                self.assertEqual(observation["bytes"], 20)
                absent, text = m.observe_value(registry, budget, h, "Missing", 256, optional=True)
                self.assertEqual(absent, {"state": "absent_name"})
                for field in ["Oversize", "WrongType"]:
                    with self.assertRaises(m.Denied): registry.value(h, field, 256)
            self.assertEqual(budget.handles, 0)
            self.assertGreater(budget.calls, 0)
            with self.assertRaises(m.Denied):
                with registry.open(name + "\\missing"):
                    pass
            self.assertEqual(budget.handles, 0)
            close = registry.dll.RegCloseKey
            saved = None
            try:
                with self.assertRaisesRegex(m.Denied, "registry-close"):
                    with registry.open(name) as saved:
                        registry.dll.RegCloseKey = lambda h: 6
                self.assertEqual(budget.handles, 1)
            finally:
                registry.dll.RegCloseKey = close
                if saved is not None:
                    close(saved)
        finally:
            winreg.DeleteKey(winreg.HKEY_CURRENT_USER, name + "\\child")
            winreg.DeleteKey(winreg.HKEY_CURRENT_USER, name)

if __name__ == "__main__": unittest.main()
