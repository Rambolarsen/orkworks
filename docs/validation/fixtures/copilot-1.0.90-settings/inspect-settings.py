"""Pinned, model-free Copilot settings discovery. Never imports its addon or reads user config."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

EXPECTED_NATIVE = "3ae21a3f00fcc216faaa1f062ee98451c7807066ee26f0fe1c85c6ed315e5458"
EXPECTED_APP = "f84061b62eeb1cd78a655555890fe1a776509203fca68144745467e26e443813"
EXPECTED_RUNTIME = "ad89ad135478cd2b19a5b99bb19064d0be281961cab0506f3767f39234d4c591"
SENTINEL = "ork740-settings-sentinel"
BASE = {"disableAllHooks": True, "memory": False, "ide": {"autoConnect": False},
        "remoteExport": False, "remoteSessions": False}
SKILL = "---\nname: ork740-settings-sentinel\ndescription: Synthetic settings precedence discovery marker.\n---\nNo tools or actions are requested.\n"
CASES = [("baseline", None, None, True), ("legacy-only", True, None, False),
         ("settings-only", None, True, False), ("legacy-disabled-settings-enabled", True, False, False),
         ("legacy-enabled-settings-disabled", False, True, True)]

def identity(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}

def pin(path, expected):
    data = path.read_bytes()
    if hashlib.sha256(data).hexdigest() != expected:
        raise ValueError("Pinned identity changed: " + path.name)
    return data

def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", type=Path, required=True)
    parser.add_argument("--cache", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    native = pin(args.native, EXPECTED_NATIVE)
    app = pin(args.cache / "app.js", EXPECTED_APP)
    pin(args.cache / "prebuilds/darwin-arm64/runtime.node", EXPECTED_RUNTIME)
    if args.output.exists():
        raise ValueError("Output must be a fresh directory")
    temp = Path(tempfile.mkdtemp(prefix="ork740-settings-reproduce-"))
    repo = temp / "repo"
    skill_path = repo / ".agents/skills" / SENTINEL
    skill_path.mkdir(parents=True)
    (skill_path / "SKILL.md").write_text(SKILL)
    subprocess.run(["git", "init", "-q", str(repo)], check=True, capture_output=True)
    inherited = {k: os.environ[k] for k in ["PATH", "HOME", "TMPDIR"] if k in os.environ}
    cases = []
    def snapshot(home):
        return {name: (home / name).read_text() if (home / name).exists() else None
                for name in ["config.json", "settings.json"]}
    def run(name, home, before):
        overrides = {"COPILOT_HOME": str(home), "COPILOT_AUTO_UPDATE": "false",
                     "USE_TGREP": "false", "NO_COLOR": "1"}
        argv = [str(args.native), "--no-auto-update", "skill", "list", "--json"]
        result = subprocess.run(argv, cwd=repo, env={**inherited, **overrides},
                                capture_output=True, timeout=30)
        if result.returncode or result.stderr or len(result.stdout) > 131072:
            raise ValueError("Discovery failed or produced unexpected stderr/output budget: " + name)
        skills = json.loads(result.stdout)
        expected_names = {SENTINEL, "customize-cloud-agent", "github-pr-media"}
        if {s["name"] for s in skills} != expected_names:
            raise ValueError("Unexpected discovery sources: " + name)
        normalized = []
        for skill in skills:
            item = dict(skill)
            item["path"] = item["path"].replace(str(temp.resolve()), "<disposable>").replace(str(temp), "<disposable>").replace(str(args.cache), "<pinned-cache>")
            normalized.append(item)
        return {"name": name, "argv": ["<pinned-native>", *argv[1:]], "cwd": "<disposable>/repo",
                "environmentOverrides": {**overrides, "COPILOT_HOME": "<disposable>/" + home.name},
                "inheritedEnvironmentNames": sorted(inherited),
                "beforeFiles": before, "afterFiles": snapshot(home), "exitCode": result.returncode,
                "rawStdoutUnretained": identity(result.stdout), "stdoutNormalization": "Only disposable and cache path prefixes replaced",
                "normalizedSkills": normalized, "stderrBytes": len(result.stderr),
                "sentinelEnabled": next(s["enabled"] for s in skills if s["name"] == SENTINEL)}
    try:
        for name, legacy, settings, expected in CASES:
            home = temp / name
            home.mkdir()
            value = dict(BASE)
            if settings is not None:
                value["disabledSkills"] = [SENTINEL] if settings else []
            write_json(home / "settings.json", value)
            if legacy is not None:
                write_json(home / "config.json", {"disabledSkills": [SENTINEL] if legacy else []})
            record = run(name, home, snapshot(home))
            if record["sentinelEnabled"] != expected:
                raise ValueError("Unexpected version-bound discovery result: " + name)
            cases.append(record)
        home = temp / "legacy-disabled-settings-enabled"
        value = json.loads((home / "settings.json").read_text())
        value["disabledSkills"] = []
        write_json(home / "settings.json", value)
        record = run("post-migration-settings-edit", home, snapshot(home))
        if not record["sentinelEnabled"]:
            raise ValueError("Post-migration setting did not take effect")
        cases.append(record)
        excerpts = []
        for term, count in [(b"function Fwt(", 484), (b"wireSessionStoreTracking(t,n,r,o)", 819),
                            (b"static resolveSessionFs(", 107), (b"function WW(", 100)]:
            pos = app.index(term)
            excerpts.append({"startUtf8Byte": pos, "bytes": count, "text": app[pos:pos+count].decode("utf-8")})
        args.output.mkdir(parents=True)
        write_json(args.output / "discovery-cases.json", cases)
        write_json(args.output / "source-excerpts.json", {"app": identity(app), "selections": excerpts,
                   "limit": "Static selected call sites; not complete native implementations or a persistence audit"})
        write_json(args.output / "identity.json", {"native": identity(native), "app": identity(app),
                   "runtimeSha256": EXPECTED_RUNTIME,
                   "limit": "Three pinned files; separate retained SEA inspector binds selected cache files to embedded runtime. No complete startup dependency attestation."})
        (args.output / "sentinel-skill.txt").write_text(SKILL)
        print("Six bounded model-free discovery cases matched; no inference, server or addon import requested")
    finally:
        shutil.rmtree(temp)

if __name__ == "__main__":
    main()
