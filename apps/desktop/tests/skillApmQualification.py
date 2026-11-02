"""Real source-CLI behavior checks; run with uv --project an APM checkout.

Path.home and expanduser are redirected within disposable child processes only. No HOME/CODEX_HOME
assignment, native session, or developer skill/configuration mutation occurs.
"""
from pathlib import Path
from tempfile import TemporaryDirectory
import os
import json
import shutil
import subprocess
import sys


def qualify(base):
    test_home = base / "user"
    test_home.mkdir()
    for key in list(os.environ):
        if key.startswith(("ORKWORKS_", "APM_")) or key in {
            "CLAUDE_CONFIG_DIR", "XDG_CONFIG_HOME", "XDG_CACHE_HOME", "CODEX_HOME",
            "APM_POLICY_DISABLE", "APM_ALLOW_PROTOCOL_FALLBACK", "APM_LEGACY_SKILL_PATHS",
        }:
            os.environ.pop(key)
    # Disable lifecycle only in this disposable fixture, never in production.
    os.environ["APM_NO_SCRIPTS"] = "1"
    os.environ["APM_CHECK_UPDATES"] = "0"
    # Each invocation gets a fresh interpreter, as the production process runner does.
    child = """
import sys
import os.path
from pathlib import Path
home = Path(sys.argv[1])
Path.home = classmethod(lambda cls: home)
original_expanduser = os.path.expanduser
def fixture_expanduser(path):
    value = os.fspath(path)
    if isinstance(value, str) and (value == "~" or value.startswith("~/")):
        return str(home) + value[1:]
    return original_expanduser(path)
os.path.expanduser = fixture_expanduser
from click.testing import CliRunner
from apm_cli.cli import cli
result = CliRunner().invoke(cli, sys.argv[2:])
print(result.output)
if result.exception and result.exit_code:
    print(repr(result.exception), file=sys.stderr)
sys.exit(result.exit_code)
"""

    def run(args, cwd):
        result = subprocess.run(
            [sys.executable, "-c", child, str(test_home), *args],
            cwd=cwd, capture_output=True, text=True, timeout=60,
        )
        assert result.returncode == 0, (args, result.stdout, result.stderr)
        return result.stdout

    version = run(["--version"], base).strip()
    help_text = run(["install", "--help"], base)
    for flag in ["--target", "--only", "--dry-run", "--no-trust-bin", "--global"]:
        assert flag in help_text, flag
    sources = []
    for name in ["first", "second"]:
        source = base / name
        source.mkdir()
        (source / "SKILL.md").write_text(
            f"---\nname: {name}\ndescription: Disposable compatibility fixture.\n---\nDo nothing.\n"
        )
        sources.append(source)

    def snapshot(path):
        return {str(p.relative_to(path)): p.read_bytes() for p in path.rglob("*") if p.is_file()}

    for scope in ["workspace", "personal"]:
        destination = test_home if scope == "personal" else base / "workspace"
        destination.mkdir(exist_ok=True)
        metadata = destination / ".apm" if scope == "personal" else destination
        sentinel = destination / "unmanaged.txt"
        sentinel.write_text("preserve unmanaged file")
        for source in sources:
            # Match OrkWorks: preview in a private project, copy only the manifest.
            before = snapshot(destination)
            stage = base / "preview"
            stage.mkdir()
            manifest = metadata / "apm.yml"
            if manifest.is_file():
                shutil.copyfile(manifest, stage / "apm.yml")
            args = ["install", str(source), "--target", "codex", "--only", "apm", "--no-trust-bin"]
            run(args + ["--dry-run"], stage)
            assert snapshot(destination) == before, "preview changed actual destination"
            assert not list(stage.glob(".agents/skills/*/SKILL.md")), "dry-run deployed skills"
            shutil.rmtree(stage)
            run(args + (["--global"] if scope == "personal" else []), destination)
            assert (destination / ".agents/skills" / source.name / "SKILL.md").read_bytes() == (source / "SKILL.md").read_bytes()
            assert (metadata / "apm.yml").is_file()
            assert (metadata / "apm.lock.yaml").is_file()
            assert (metadata / "apm_modules").is_dir()
            assert sentinel.read_text() == "preserve unmanaged file"
        assert (destination / ".agents/skills/first/SKILL.md").read_bytes() == (sources[0] / "SKILL.md").read_bytes()
        for other in [".claude", ".opencode", ".codex/hooks.json", ".codex/config.toml"]:
            assert not (destination / other).exists(), f"unexpected target mutation: {other}"
    print(json.dumps({"version": version, "workspace": "pass", "personal": "pass", "existing_manifests": "pass", "private_previews": "pass"}))


with TemporaryDirectory(prefix="orkworks-apm-qualification-") as directory:
    qualify(Path(directory).resolve())
