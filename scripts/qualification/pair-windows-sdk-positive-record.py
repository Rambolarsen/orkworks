"""One bounded, positive-only SDK product observation; no target tools are run."""
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
        self.hash_categories = {}
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
    def close(self, success=True):
        # Every successful or failed open has already reserved this call.
        self.calls += 1
        self.handles -= int(success)
    def read(self, maximum):
        self.check()
        if self.read_bytes + maximum > self.read_limit:
            raise Denied("read-limit")
        self.read_bytes += maximum  # Charge the entire fixed buffer before I/O.
    def hash_bytes(self, data, category, category_limit):
        self.check()
        used = self.hash_categories.get(category, 0)
        if self.hashed_bytes + len(data) > self.hash_limit:
            raise Denied("hash-limit")
        if used + len(data) > category_limit:
            raise Denied("hash-category-limit")
        self.hashed_bytes += len(data)
        self.hash_categories[category] = used + len(data)
    def raw_digest(self, raw):
        self.hash_bytes(raw, "raw_values", 1070212)
        return hashlib.sha256(raw).hexdigest()
    def pass_digest(self, rows, envelope):
        # Framing is included in both the shared and per-category ledgers.
        encoded = []
        for row in sorted(rows):
            data = canonical(row)
            if len(data) + 4 > 2048:
                raise Denied("row-limit")
            encoded.append(len(data).to_bytes(4, "big") + data)
        data = canonical(envelope)
        if len(data) + 4 > 128*1024:
            raise Denied("envelope-limit")
        framed = len(data).to_bytes(4, "big") + data
        # Validate individual bounds before starting the pass hash.
        digest = hashlib.sha256()
        for data in encoded:
            self.hash_bytes(data, "rows", 4*1024**2)
            digest.update(data)
        self.hash_bytes(framed, "envelopes", 256*1024)
        digest.update(framed)
        return digest.hexdigest()

def canonical(value):
    return json.dumps(value, ensure_ascii=True, sort_keys=True, separators=(",", ":")).encode()

def decode_value(raw, kind, maximum):
    if kind != 1:
        raise Denied("unsupported_type")
    if len(raw) < 2 or len(raw) % 2 or len(raw) > 2*(maximum+1):
        raise Denied("invalid_length")
    try:
        text = raw.decode("utf-16-le", errors="strict")
    except UnicodeError:
        raise Denied("invalid_utf16") from None
    if not text.endswith("\x00") or "\x00" in text[:-1]:
        raise Denied("invalid_termination")
    text = text[:-1]
    if not text:
        raise Denied("empty")
    if any(ord(c) < 32 or ord(c) > 126 for c in text):
        raise Denied("non_ascii_or_nonprintable")
    return text

def observe_value(registry, budget, handle, field, maximum, optional=False):
    raw = registry.raw_value(handle, field, maximum, missing=optional)
    if raw is None:
        return {"state": "absent_name"}, None
    kind, data = raw
    record = {"kind": kind, "bytes": len(data), "sha256": budget.raw_digest(data)}
    try:
        text = decode_value(data, kind, maximum)
    except Denied as error:
        if not optional:
            raise
        record.update(state="unclassified", reason=str(error))
        return record, None
    record["state"] = "accepted_ascii"
    return record, text

def empty_result():
    return {"schema": 2, "evidence_kind": "sdk-positive-product-observation",
            "success": False, "positive_record_observed": False,
            "inventory_complete": False, "runtime_qualified": False,
            "replacement_pin_accepted": False, "passes": []}

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
    def open(self, path, parent=None):
        handle = self.HKEY()
        self.budget.call(reserve_close=True)
        # OPEN_LINK opens a link itself instead of following it out of scope.
        code = self.dll.RegOpenKeyExW(self.hive if parent is None else parent, path.replace("/", "\\"), 8, 0x20119, ctypes.byref(handle))
        if code:
            self.budget.handles -= 1
            raise Denied("registry-open")
        try:
            yield handle
        finally:
            self.budget.close(success=False)
            if self.dll.RegCloseKey(handle):
                raise Denied("registry-close")
            self.budget.handles -= 1
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
        self.budget.read(516)
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
    def raw_value(self, handle, name, maximum, missing=False):
        data = (ctypes.c_ubyte * (2*(maximum+1)))()
        size, kind = self.DWORD(len(data)), self.DWORD()
        self.budget.read(len(data) + 8)
        self.budget.call()
        code = self.dll.RegQueryValueExW(handle, name, None, ctypes.byref(kind), data, ctypes.byref(size))
        if code == 2 and missing:
            return None
        if code or size.value > len(data):
            raise Denied("registry-value")
        return kind.value, bytes(data[:size.value])
    def value(self, handle, name, maximum, missing=False):
        raw = self.raw_value(handle, name, maximum, missing)
        return None if raw is None else decode_value(raw[1], raw[0], maximum)

def collect(registry, budget):
    result = empty_result()
    try:
        for _ in range(2):
            budget.check()
            observation = {"status": "collecting", "products": [], "roots": []}
            result["passes"].append(observation)
            rows = []
            counts = {"accepted_ascii": 0, "absent_name": 0, "unclassified": 0,
                      "predicate_match": 0, "predicate_nonmatch": 0, "reasons": {}}
            observation["classification"] = counts
            with ExitStack() as stack:
                roots = [stack.enter_context(registry.open(p)) for p in ROOTS]
                before = [registry.info(h) for h in roots]
                observation["roots"] = [{"path": p, "count": n, "before_write": str(stamp)} for p, (n, stamp) in zip(ROOTS, before)]
                if sum(n for n, _ in before) > 1024:
                    raise Denied("entry-limit")
                with registry.open(INSTALLED) as h:
                    kit_record, kit = observe_value(registry, budget, h, "KitsRoot10", 256)
                if kit.replace("\\", "/").rstrip("/").lower() != KIT_ROOT:
                    raise Denied("sdk-root")
                observation["kits_root"] = "C:/Program Files (x86)/Windows Kits/10/"
                for root_index, (root, handle, (count, _)) in enumerate(zip(ROOTS, roots, before)):
                    seen = set()
                    for index in range(count):
                        budget.check()
                        name = registry.name(handle, index)
                        if name.casefold() in seen:
                            raise Denied("duplicate-name")
                        seen.add(name.casefold())
                        key = root + "/" + name
                        with registry.open(name, parent=handle) as child:
                            record, display = observe_value(registry, budget, child, "DisplayName", 256, optional=True)
                            state = record["state"]
                            counts[state] += 1
                            if state == "unclassified":
                                reason = record["reason"]
                                counts["reasons"][reason] = counts["reasons"].get(reason, 0) + 1
                            version_record = None
                            if display is not None:
                                match = "windows software development kit" in display.lower()
                                record["predicate"] = "match" if match else "nonmatch"
                                counts["predicate_match" if match else "predicate_nonmatch"] += 1
                                if match:
                                    if len(observation["products"]) >= 32:
                                        raise Denied("match-limit")
                                    version_record, version = observe_value(registry, budget, child, "DisplayVersion", 128)
                                    observation["products"].append({"key": key, "name": display, "version": version})
                            rows.append([root_index, name, record, version_record])
                after = [registry.info(h) for h in roots]
                for record, (count, stamp) in zip(observation["roots"], after):
                    record["after_count"] = count
                    record["after_write"] = str(stamp)
                if before != after:
                    raise Denied("registry-drift")
            observation["products"].sort(key=lambda p: p["key"])
            observation["digest"] = budget.pass_digest(rows, [kit_record, before, counts, observation["products"]])
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
        result["success"] = result["positive_record_observed"] = True
    except Denied as error:
        result["reason"] = str(error)
    except Exception:
        result["reason"] = "registry-unavailable"
    result.update(api_calls=budget.calls, open_handles=budget.handles,
                  read_bytes_charged=budget.read_bytes, hashed_bytes=budget.hashed_bytes,
                  hash_categories=budget.hash_categories)
    return result

def runtime_identity():
    return (sys.platform == "win32" and sys.version_info[:3] == (3, 12, 10)
            and ctypes.sizeof(ctypes.c_void_p) == 8 and sys.implementation.name == "cpython"
            and bool(sys.flags.isolated) and bool(sys.flags.no_site) and sys.dont_write_bytecode
            and os.path.normcase(sys.executable) == os.path.normcase(PYTHON.replace("/", "\\")))

def main():
    # Importing for offline tests never constructs NativeRegistry or reads a key.
    if (not runtime_identity() or os.environ.get("ORK_PAIR_SDK_POSITIVE_CHILD") != "1"
            or os.environ.get("SystemRoot", "").lower() != "c:\\windows"):
        result = empty_result()
        result.update(reason="runtime-denied", api_calls=0, open_handles=0,
                      read_bytes_charged=0, hashed_bytes=0, hash_categories={})
        print(json.dumps(result, separators=(",", ":")))
        return
    limit = os.environ.get("ORK_PAIR_SDK_POSITIVE_HASH_BUDGET", "")
    if not limit.isdecimal() or not 0 < int(limit) <= 8*1024**2:
        result = empty_result()
        result.update(reason="hash-budget", api_calls=0, open_handles=0,
                      read_bytes_charged=0, hashed_bytes=0, hash_categories={})
        print(json.dumps(result, separators=(",", ":")))
        return
    budget = Budget(hash_limit=int(limit))
    result = collect(NativeRegistry(budget), budget)
    result["runtime"] = {"python": "3.12.10", "arch": "X64", "executable": sys.executable}
    print(json.dumps(result, separators=(",", ":")))

if __name__ == "__main__":
    main()
