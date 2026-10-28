from pathlib import Path
import tempfile
import os
import json

# Run the real APM 0.33.0 source CLI with a process-local home resolver.
# No HOME/CODEX_HOME or developer configuration is modified.
base = Path(tempfile.mkdtemp(prefix="orkworks-apm-user-"))
test_home = base / "user"
test_home.mkdir()
Path.home = classmethod(lambda cls: test_home)
for key in list(os.environ):
    if key.startswith("ORKWORKS_") or key in {"CLAUDE_CONFIG_DIR", "XDG_CONFIG_HOME", "APM_HOME"}:
        os.environ.pop(key)
os.environ["APM_NO_SCRIPTS"] = "1"
os.environ["APM_CHECK_UPDATES"] = "0"
source = base / "skill"
source.mkdir()
(source / "SKILL.md").write_text("---\nname: orkworks-probe\ndescription: Disposable personal install.\n---\nDo nothing.\n")
from click.testing import CliRunner
from apm_cli.cli import cli
result = CliRunner().invoke(cli, ["install", str(source), "--global", "--target", "codex", "--only", "apm", "--no-trust-bin"])
print(result.output)
assert result.exit_code == 0, repr(result.exception)
skills = list(test_home.rglob("SKILL.md"))
assert any(".agents/skills/" in str(p) for p in skills), skills
assert (test_home / ".apm/apm.yml").is_file()
print(json.dumps({"fixture":str(base),"deployed":[str(p.relative_to(test_home)) for p in skills]}))
