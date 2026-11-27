"""One bounded, read-only SDK registry observation; no target tools are run."""
import ctypes
import hashlib
import json
import os
import sys
import time
from contextlib import ExitStack, contextmanager

ROOTS = (
    "SOFTWARE/Microsoft/Windows/CurrentVersion/Uninstall",
    "SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall",
)
INSTALLED = "SOFTWARE/Microsoft/Windows Kits/Installed Roots"
PYTHON = "C:/hostedtoolcache/windows/Python/3.12.10/x64/python.exe"
KIT_ROOT = "c:/program files (x86)/windows kits/10"

class Denied(Exception):
    pass

class Budget:
    def __init__(self, api_limit=10000, read_limit=4*1024**2,
                 hash_limit=8*1024**2, seconds=60):
        self.calls = self.handles = self.read_bytes = self.hashed_bytes = 0
        self.api_limit, self.read_limit, self.hash_limit = api_limit, read_limit, hash_limit
        self.deadline = time.monotonic() + seconds
    def check(self):
        if time.monotonic() >= self.deadline:
            raise Denied("time-limit")
    def call(self, reserve_close=False):
        self.check()
        reserve = int(reserve_close)
        if self.calls + self.handles + 1 + reserve > self.api_limit:
            raise Denied("api-limit")
        self.calls += 1
        self.handles += reserve
    def close(self):
        # Every successful or failed open has already reserved this call.
        self.calls += 1
        self.handles -= 1
    def read(self, maximum):
        self.check()
        if self.read_bytes + maximum > self.read_limit:
            raise Denied("read-limit")
        self.read_bytes += maximum  # Charge the entire fixed buffer before I/O.
    def digest(self, value):
        data = json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()
        if self.hashed_bytes + len(data) > self.hash_limit:
            raise Denied("hash-limit")
        self.hashed_bytes += len(data)
        return hashlib.sha256(data).hexdigest()

def decode_value(raw, kind, maximum):
    if kind != 1 or len(raw) < 2 or len(raw) % 2 or len(raw) > 2*(maximum+1):
        raise Denied("value-format")
    try:
        text = raw.decode("utf-16-le", errors="strict")
    except UnicodeError:
        raise Denied("value-format") from None
    if not text.endswith("\x00") or "\x00" in text[:-1]:
        raise Denied("value-format")
    text = text[:-1]
    if not text or any(ord(c) < 32 or ord(c) > 126 for c in text):
        raise Denied("value-format")
    return text

class NativeRegistry:
    """Fixed-buffer Win32 boundary. hive may differ only in isolated test fixtures."""
    def __init__(self, budget, hive=-2147483646):
        if sys.platform != "win32":
            raise Denied("platform")
        self.budget, self.hive = budget, ctypes.c_void_p(hive)
        self.dll = ctypes.WinDLL("C:/Windows/System32/advapi32.dll", use_last_error=True)
        self.DWORD, self.HKEY = ctypes.c_uint32, ctypes.c_void_p
        p = ctypes.c_void_p
        signatures = {
            "RegOpenKeyExW": [p, ctypes.c_wchar_p, self.DWORD, self.DWORD, p],
            "RegCloseKey": [p],
            "RegQueryInfoKeyW": [p]*12,
            "RegEnumKeyExW": [p, self.DWORD, p, p, p, p, p, p],
            "RegQueryValueExW": [p, ctypes.c_wchar_p, p, p, p, p],
        }
        for name, args in signatures.items():
            fn = getattr(self.dll, name)
            fn.argtypes, fn.restype = args, ctypes.c_int32
    @contextmanager
    def open(self, path):
        handle = self.HKEY()
        self.budget.call(reserve_close=True)
        # OPEN_LINK opens a link itself instead of following it out of scope.
        code = self.dll.RegOpenKeyExW(self.hive, path.replace("/", "\\"), 8, 0x20119, ctypes.byref(handle))
        if code:
            self.budget.handles -= 1
            raise Denied("registry-open")
        try:
            yield handle
        finally:
            self.budget.close()
            if self.dll.RegCloseKey(handle):
                raise Denied("registry-close")
    def info(self, handle):
        count = self.DWORD()
        stamp = (self.DWORD * 2)()
        self.budget.read(12)
        self.budget.call()
        code = self.dll.RegQueryInfoKeyW(handle, None, None, None, ctypes.byref(count), None, None, None, None, None, None, stamp)
        if code:
            raise Denied("registry-info")
        return (count.value, (stamp[1] << 32) | stamp[0])
    def name(self, handle, index):
        value = ctypes.create_unicode_buffer(256)
        size = self.DWORD(256)
        self.budget.read(512)
        self.budget.call()
        code = self.dll.RegEnumKeyExW(handle, index, value, ctypes.byref(size), None, None, None, None)
        if code or not 0 < size.value < 256:
            raise Denied("registry-name")
        name = value[:size.value]
        try:
            name.encode("utf-16-le", errors="strict")
        except UnicodeError:
            raise Denied("registry-name") from None
        if any(c in name for c in ("/", "\\", "\x00")):
            raise Denied("registry-name")
        return name
    def value(self, handle, name, maximum, missing=False):
        data = (ctypes.c_ubyte * (2*(maximum+1)))()
        size, kind = self.DWORD(len(data)), self.DWORD()
        self.budget.read(len(data))
        self.budget.call()
        code = self.dll.RegQueryValueExW(handle, name, None, ctypes.byref(kind), data, ctypes.byref(size))
        if code == 2 and missing:
            return None
        if code or size.value > len(data):
            raise Denied("registry-value")
        return decode_value(bytes(data[:size.value]), kind.value, maximum)

def collect(registry, budget):
    result = {"schema": 1, "success": False, "passes": []}
    try:
        for _ in range(2):
            budget.check()
            observation = {"status": "collecting", "products": [], "roots": []}
            result["passes"].append(observation)
            rows = []
            with ExitStack() as stack:
                roots = [stack.enter_context(registry.open(p)) for p in ROOTS]
                before = [registry.info(h) for h in roots]
                observation["roots"] = [{"path": p, "count": n, "before_write": str(stamp)} for p, (n, stamp) in zip(ROOTS, before)]
                if sum(n for n, _ in before) > 1024:
                    raise Denied("entry-limit")
                with registry.open(INSTALLED) as h:
                    kit = registry.value(h, "KitsRoot10", 256)
                observation["kits_root"] = kit
                if kit.replace("\\", "/").rstrip("/").lower() != KIT_ROOT:
                    raise Denied("sdk-root")
                for root, handle, (count, _) in zip(ROOTS, roots, before):
                    seen = set()
                    for index in range(count):
                        budget.check()
                        name = registry.name(handle, index)
                        if name.casefold() in seen:
                            raise Denied("duplicate-name")
                        seen.add(name.casefold())
                        key = root + "/" + name
                        with registry.open(key) as child:
                            display = registry.value(child, "DisplayName", 256, missing=True)
                            version = None
                            if display is not None and "windows software development kit" in display.lower():
                                if len(observation["products"]) >= 32:
                                    raise Denied("match-limit")
                                version = registry.value(child, "DisplayVersion", 128)
                                observation["products"].append({"key": key, "name": display, "version": version})
                            rows.append([key, display, version])
                after = [registry.info(h) for h in roots]
                for record, (count, stamp) in zip(observation["roots"], after):
                    record["after_count"] = count
                    record["after_write"] = str(stamp)
                if before != after:
                    raise Denied("registry-drift")
            observation["products"].sort(key=lambda p: p["key"])
            observation["digest"] = budget.digest([kit, before, sorted(rows)])
            observation["status"] = "complete"
        if result["passes"][0]["digest"] != result["passes"][1]["digest"]:
            raise Denied("observation-drift")
        products = result["passes"][0]["products"]
        if not any(p["version"] == "10.1.26100.9457" for p in products):
            raise Denied("sdk-service")
        # Equal product names with incompatible versions are ambiguous.
        by_name = {}
        for p in products:
            key = p["name"].casefold()
            if key in by_name and by_name[key] != p["version"]:
                raise Denied("sdk-ambiguous")
            by_name[key] = p["version"]
        result["success"] = True
    except Denied as error:
        result["reason"] = str(error)
    except Exception:
        result["reason"] = "registry-unavailable"
    result.update(api_calls=budget.calls, open_handles=budget.handles,
                  read_bytes_charged=budget.read_bytes, hashed_bytes=budget.hashed_bytes)
    return result

def main():
    # Importing for offline tests never constructs NativeRegistry or reads a key.
    if (sys.platform != "win32" or sys.version_info[:3] != (3, 12, 10)
            or ctypes.sizeof(ctypes.c_void_p) != 8 or sys.implementation.name != "cpython"
            or not sys.flags.isolated or not sys.flags.no_site or not sys.dont_write_bytecode
            or os.path.normcase(sys.executable) != os.path.normcase(PYTHON.replace("/", "\\"))
            or os.environ.get("ORK_PAIR_SDK_CHILD") != "1"
            or os.environ.get("SystemRoot", "").lower() != "c:\\windows"):
        print('{"schema":1,"success":false,"reason":"runtime-denied","passes":[],"api_calls":0,"open_handles":0,"read_bytes_charged":0,"hashed_bytes":0}')
        return
    limit = os.environ.get("ORK_PAIR_SDK_HASH_BUDGET", "")
    if not limit.isdecimal() or not 0 < int(limit) <= 8*1024**2:
        print('{"schema":1,"success":false,"reason":"hash-budget","passes":[],"api_calls":0,"open_handles":0,"read_bytes_charged":0,"hashed_bytes":0}')
        return
    budget = Budget(hash_limit=int(limit))
    result = collect(NativeRegistry(budget), budget)
    result["runtime"] = {"python": "3.12.10", "arch": "X64", "executable": sys.executable}
    print(json.dumps(result, separators=(",", ":")))

if __name__ == "__main__":
    main()
