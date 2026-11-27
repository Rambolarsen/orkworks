"""One exact named SDK pass for the approved build bracket; no enumeration."""
import importlib.util
import json
import os
import pathlib
from contextlib import ExitStack

# Reuse the sealed Win32 boundary as a library; never call its consumed main/collect.
_spec = importlib.util.spec_from_file_location("sdk_boundary", pathlib.Path(__file__).with_name("pair-windows-sdk-positive-record.py"))
_native = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(_native)
Budget, Denied, NativeRegistry = _native.Budget, _native.Denied, _native.NativeRegistry
runtime_identity = _native.runtime_identity
INSTALLED = "SOFTWARE/Microsoft/Windows Kits/Installed Roots"
PARENT = "SOFTWARE/WOW6432Node/Microsoft/Windows/CurrentVersion/Uninstall"
PRODUCT = "{45e335f3-c807-43b8-9ed1-43666cfcf939}"
NAME = "Windows Software Development Kit - Windows 10.0.26100.9457"
VERSION = "10.1.26100.9457"
KIT = "C:/Program Files (x86)/Windows Kits/10/"


def empty():
    return {"schema": 3, "evidence_kind": "sdk-positive-build-bracket", "status": "unavailable",
            "inventory_complete": False, "enumerated_entries": 0}


def collect_pass(registry, budget):
    out = empty()
    try:
        budget.check()
        commitments = []
        def value(handle, field, maximum):
            kind, raw = registry.raw_value(handle, field, maximum)
            commitments.append([field, kind, len(raw), budget.raw_digest(raw)])
            return _native.decode_value(raw, kind, maximum)
        with ExitStack() as stack:
            installed = stack.enter_context(registry.open(INSTALLED))
            parent = stack.enter_context(registry.open(PARENT))
            product = stack.enter_context(registry.open(PRODUCT, parent=parent))
            before = [str(registry.info(h)[1]) for h in (installed, product)]
            kit = value(installed, "KitsRoot10", 256)
            name = value(product, "DisplayName", 256)
            version = value(product, "DisplayVersion", 128)
            if kit.replace("\\", "/").rstrip("/") != KIT.rstrip("/"):
                raise Denied("sdk-root")
            if name != NAME or version != VERSION:
                raise Denied("sdk-record")
            after = [str(registry.info(h)[1]) for h in (installed, product)]
            if before != after:
                raise Denied("registry-drift")
        record = {"key": PARENT+"/"+PRODUCT, "name": name, "version": version}
        canonical = _native.canonical([INSTALLED, PARENT, PRODUCT, before, after, KIT, record, commitments])
        framed = len(canonical).to_bytes(4, "big") + canonical
        if len(framed) > 8192:
            raise Denied("sdk-pass-limit")
        budget.hash_bytes(framed, "sdk_pass", 8192)
        out.update(status="complete", before_write=before, after_write=after, kits_root=KIT,
                   record=record, digest=_native.hashlib.sha256(framed).hexdigest())
    except Denied as error:
        out["reason"] = str(error)
    except Exception:
        out["reason"] = "sdk-unavailable"
    out.update(api_calls=budget.calls, open_handles=budget.handles,
               read_bytes_charged=budget.read_bytes, hashed_bytes=budget.hashed_bytes)
    return out


def main():
    limits = {}
    for name, maximum in [("API",32),("READ",4096),("HASH",32768)]:
        raw = os.environ.get("ORK_REPLACEMENT_SDK_"+name, "")
        if not raw.isdecimal() or not 0 < int(raw) <= maximum:
            print(json.dumps({**empty(),"reason":"sdk-budget","api_calls":0,"open_handles":0,"read_bytes_charged":0,"hashed_bytes":0}))
            return
        limits[name] = int(raw)
    if (not runtime_identity() or os.environ.get("ORK_REPLACEMENT_SDK") != "1"
            or os.environ.get("SystemRoot", "").lower() != "c:\\windows"):
        print(json.dumps({**empty(),"reason":"runtime-denied","api_calls":0,"open_handles":0,"read_bytes_charged":0,"hashed_bytes":0}))
        return
    budget = Budget(api_limit=limits["API"],read_limit=limits["READ"],hash_limit=limits["HASH"],seconds=30)
    out = collect_pass(NativeRegistry(budget),budget)
    out["runtime"] = {"path": _native.sys.executable, "version": _native.sys.version.split()[0], "implementation": _native.sys.implementation.name, "bits": 64}
    print(json.dumps(out,separators=(",",":")))

if __name__ == "__main__":
    main()
