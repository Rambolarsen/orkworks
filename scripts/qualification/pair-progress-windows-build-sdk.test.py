"""Only fake registry data and isolated HKCU fixtures; never installed SDK reads."""
import importlib.util
import pathlib
import sys
import unittest
from contextlib import contextmanager

spec = importlib.util.spec_from_file_location("progress_sdk", pathlib.Path(__file__).with_name("pair-progress-windows-build-sdk.py"))
m = importlib.util.module_from_spec(spec)
try:
    spec.loader.exec_module(m)
except FileNotFoundError:
    m = None

class Registry:
    def __init__(self, budget):
        self.budget=budget; self.trace=[]; self.bad=None; self.drift=False; self.infos=0
    @contextmanager
    def open(self, name, parent=None):
        self.trace.append(("open",name,parent))
        self.budget.call(reserve_close=True)
        try: yield name
        finally: self.budget.close()
    def info(self, handle):
        self.budget.read(12); self.budget.call(); self.infos+=1
        return (0, 134399551354776438 + int(self.drift and self.infos>2))
    def raw_value(self, handle, field, maximum, missing=False):
        self.trace.append(("value",handle,field)); self.budget.read(2*(maximum+1)+8); self.budget.call()
        if self.bad: return self.bad
        values={"KitsRoot10":getattr(self,"kit","C:/Program Files (x86)/Windows Kits/10/"), "DisplayName":"Windows Software Development Kit - Windows 10.0.26100.9457", "DisplayVersion":"10.1.26100.9457"}
        return 1, (values[field]+"\x00").encode("utf-16-le")
    def name(self,*args): raise AssertionError("enumeration forbidden")

class Tests(unittest.TestCase):
    def setUp(self): self.assertIsNotNone(m,"replacement SDK reader is not implemented")
    def run_fake(self, change=None, **limits):
        b=m.Budget(api_limit=limits.get("api_limit",32),read_limit=limits.get("read_limit",4096),hash_limit=limits.get("hash_limit",32768),seconds=limits.get("seconds",30))
        r=Registry(b)
        if change: change(r)
        return m.collect_pass(r,b),r
    def test_only_named_required_fields_and_relative_child_are_read(self):
        out,r=self.run_fake()
        self.assertEqual(out["schema"],3);self.assertEqual(out["evidence_kind"],"sdk-positive-build-bracket")
        self.assertEqual(out["status"],"complete",out)
        self.assertEqual(out["api_calls"],13);self.assertEqual(out["read_bytes_charged"],1358);self.assertEqual(out["open_handles"],0)
        self.assertFalse(out["inventory_complete"]);self.assertEqual(out["enumerated_entries"],0)
        self.assertEqual(out["record"]["version"],"10.1.26100.9457")
        self.assertEqual(out["before_write"],["134399551354776438"]*2)
        self.assertEqual([x[1] for x in r.trace if x[0]=="open"],[m.INSTALLED,m.PARENT,m.PRODUCT])
        self.assertEqual(r.trace[2],("open",m.PRODUCT,m.PARENT))
        self.assertEqual([x[2] for x in r.trace if x[0]=="value"],["KitsRoot10","DisplayName","DisplayVersion"])
    def test_required_type_encoding_termination_and_exact_values_refuse(self):
        for bad in [(4,b"abcd"),(1,b"x"),(1,b"A\x00"),(1,"private-é\x00".encode("utf-16-le")),(1,"wrong\x00".encode("utf-16-le"))]:
            out,_=self.run_fake(lambda r:setattr(r,"bad",bad))
            self.assertEqual(out["status"],"unavailable");self.assertNotIn("private",str(out));self.assertNotIn("record",out)
    def test_sdk_root_normalizes_only_separators_and_trailing_slash(self):
        good,_=self.run_fake(lambda r:setattr(r,"kit","C:\\Program Files (x86)\\Windows Kits\\10"))
        self.assertEqual(good["status"],"complete")
        bad,_=self.run_fake(lambda r:setattr(r,"kit","c:/program files (x86)/windows kits/10/"))
        self.assertEqual(bad.get("reason"),"sdk-root")
    def test_write_drift_denies_even_with_matching_product(self):
        out,_=self.run_fake(lambda r:setattr(r,"drift",True));self.assertEqual(out["reason"],"registry-drift")
    def test_remaining_api_read_hash_and_time_caps_refuse_without_reset(self):
        for limit in [{"api_limit":12},{"read_limit":1357},{"hash_limit":1},{"seconds":0}]:
            out,_=self.run_fake(**limit);self.assertEqual(out["status"],"unavailable",out);self.assertEqual(out["open_handles"],0)
    def test_canonical_digest_stable_and_seals_full_width_write_marker(self):
        a,_=self.run_fake();b,_=self.run_fake();self.assertEqual(a["digest"],b["digest"])
        self.assertLessEqual(a["hashed_bytes"],8192+1286)

@unittest.skipUnless(sys.platform=="win32","owned HKCU ABI fixture")
class NativeTests(unittest.TestCase):
    def test_named_reads_on_owned_fixture(self):
        import winreg,uuid
        self.assertTrue(m.runtime_identity())
        prefix="SOFTWARE\\OrkWorksReplacementSdkTests\\"+uuid.uuid4().hex
        for leaf in ["installed","uninstall\\"+m.PRODUCT]:
            with winreg.CreateKey(winreg.HKEY_CURRENT_USER,prefix+"\\"+leaf) as h:
                if leaf=="installed":winreg.SetValueEx(h,"KitsRoot10",0,winreg.REG_SZ,"C:/Program Files (x86)/Windows Kits/10/")
                else:
                    winreg.SetValueEx(h,"DisplayName",0,winreg.REG_SZ,"Windows Software Development Kit - Windows 10.0.26100.9457")
                    winreg.SetValueEx(h,"DisplayVersion",0,winreg.REG_SZ,"10.1.26100.9457")
        class Owned(m.NativeRegistry):
            @contextmanager
            def open(self,path,parent=None):
                if parent is None:path=prefix+"/"+({m.INSTALLED:"installed",m.PARENT:"uninstall"}[path])
                with super().open(path,parent) as h:yield h
        try:
            b=m.Budget(api_limit=32,read_limit=4096,hash_limit=32768,seconds=30)
            out=m.collect_pass(Owned(b,hive=-2147483647),b)
            self.assertEqual(out["status"],"complete",out);self.assertEqual(out["api_calls"],13);self.assertEqual(out["read_bytes_charged"],1358)
        finally:
            for leaf in ["uninstall\\"+m.PRODUCT,"uninstall","installed",""]:
                winreg.DeleteKey(winreg.HKEY_CURRENT_USER,prefix+("\\"+leaf if leaf else ""))

if __name__=="__main__":unittest.main()
