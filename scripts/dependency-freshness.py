#!/usr/bin/env python3
"""Report-only dependency sweep. Raw collector evidence accompanies every report."""
import argparse
from datetime import date, datetime, timezone
import hashlib
import html
import json
import os
from pathlib import Path
import re
import signal
import shutil
import subprocess
import tempfile
import urllib.request


PNPM = ["pnpm", "with", "12.18.0"]


def command(argv, cwd, timeout=180, env=None):
    try:
        with subprocess.Popen(argv, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                              text=True, stdin=subprocess.DEVNULL, start_new_session=True) as process:
            try:
                stdout, stderr = process.communicate(timeout=timeout)
                return process.returncode, stdout, stderr
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
                stdout, stderr = process.communicate()
                return 124, stdout, stderr + "\nCollector timeout"
    except OSError as error:
        return 127, "", str(error)


def validate_reviews(config):
    if not isinstance(config, dict) or config.get("version") != 1 or not isinstance(config.get("reviews"), list):
        raise ValueError("Expected watch list version 1 and reviews array")
    seen = set()
    for entry in config["reviews"]:
        if not isinstance(entry, dict):
            raise ValueError("Watch entry must be an object")
        for key in ("advisory", "package", "manifest", "rationale", "evidence", "reviewed_at"):
            if not isinstance(entry.get(key), str) or not entry[key].strip():
                raise ValueError("Missing watch entry field: " + key)
        if not re.fullmatch(r"GHSA-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}", entry["advisory"]):
            raise ValueError("Invalid GHSA identifier")
        if entry["manifest"] != "apps/desktop/pnpm-lock.yaml" or not entry["evidence"].startswith("https://"):
            raise ValueError("Watch entries require a desktop manifest and HTTPS evidence")
        date.fromisoformat(entry["reviewed_at"])
        condition = entry.get("condition", {})
        if not isinstance(condition, dict) or condition.get("kind") not in ("patched", "date"):
            raise ValueError("Unknown watch condition")
        if condition["kind"] == "date":
            date.fromisoformat(condition.get("date", ""))
        identity = (entry["advisory"], entry["package"], entry["manifest"])
        if identity in seen:
            raise ValueError("Duplicate watch entry")
        seen.add(identity)
    return config["reviews"]


def audit_lines(payload, code, reviews, overrides):
    if not isinstance(payload, dict) or "error" in payload or code not in (0, 1):
        raise ValueError("Audit collector failed")
    advisories = payload.get("advisories")
    counts = payload.get("metadata", {}).get("vulnerabilities")
    if not isinstance(advisories, dict) or not isinstance(counts, dict) or not counts:
        raise ValueError("Malformed audit response")
    if any(type(count) is not int or count < 0 for count in counts.values()):
        raise ValueError("Malformed audit vulnerability counts")
    if bool(advisories) != bool(sum(counts.values())) or code != int(bool(advisories)):
        raise ValueError("Audit exit/counts disagree with advisory evidence")
    lines = []
    for item in advisories.values():
        if not isinstance(item, dict):
            raise ValueError("Malformed advisory")
        for key in ("module_name", "severity", "github_advisory_id", "patched_versions", "title"):
            if not isinstance(item.get(key), str) or not item[key]:
                raise ValueError("Missing advisory field: " + key)
        findings = item.get("findings")
        if not isinstance(findings, list) or not findings:
            raise ValueError("Advisory missing dependency paths")
        paths = []
        for finding in findings:
            if not isinstance(finding, dict) or not isinstance(finding.get("paths"), list) or not finding["paths"]:
                raise ValueError("Malformed dependency paths")
            if any(not isinstance(path, str) or not path for path in finding["paths"]):
                raise ValueError("Invalid dependency path")
            paths.extend(finding["paths"])
        review = next((entry for entry in reviews if entry["advisory"] == item["github_advisory_id"]
                       and entry["package"] == item["module_name"]), None)
        disposition = "ACTIONABLE (needs triage)" if review is None else (
            "ACKNOWLEDGED: " + review["rationale"] + " — " + review["evidence"])
        related = {key: value for key, value in overrides.items()
                   if key == item["module_name"] or key.startswith(item["module_name"] + "@")}
        lines.append(f'{item["github_advisory_id"]} | {item["severity"]} | {item["module_name"]}: '
                     f'{item["title"]}\n  Paths: {", ".join(paths)}\n  Fix: {item["patched_versions"]}'
                     f'\n  {disposition}\n  Relevant overrides (not suppressions): {json.dumps(related)}')
    if not lines:
        return ["No advisories reported."]
    return ["Severity counts: " + json.dumps(counts)] + lines


def watch_line(entry, advisory, today):
    prefix = entry["advisory"] + " (" + entry["package"] + "): "
    if entry["condition"]["kind"] == "date":
        due = date.fromisoformat(entry["condition"]["date"])
        return prefix + ("MET — human review due " if today >= due else "UNCHANGED — review due ") + str(due)
    if not isinstance(advisory, dict) or "withdrawn_at" not in advisory:
        raise ValueError("Advisory metadata unavailable or malformed")
    if advisory["withdrawn_at"]:
        return prefix + "WITHDRAWN — human review required to retire watch entry"
    vulnerabilities = advisory.get("vulnerabilities")
    if not isinstance(vulnerabilities, list):
        raise ValueError("Advisory vulnerabilities unavailable")
    matching = [item for item in vulnerabilities if item.get("package", {}).get("name") == entry["package"]
                and item.get("package", {}).get("ecosystem") == "npm"]
    if not matching or any("first_patched_version" not in item for item in matching):
        raise ValueError("Advisory has no usable matching package evidence")
    versions = [item["first_patched_version"] for item in matching if item["first_patched_version"]]
    if versions:
        if any(not isinstance(version, str) or not version.strip() for version in versions):
            raise ValueError("Malformed patch information")
        return prefix + "MET — upstream patch available: " + ", ".join(versions)
    return prefix + "UNCHANGED — no upstream patched version reported"


def snapshot(root):
    return {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in root.rglob("*") if path.is_file()}


def collect_apm(root, output, apm):
    # No install, trust, or deployment in the repository under observation.
    with tempfile.TemporaryDirectory(prefix="dependency-apm-") as tmp:
        stage = Path(tmp) / "checkout"
        stage.mkdir()
        shutil.copy2(root / "apm.yml", stage / "apm.yml")
        before = snapshot(stage)
        env = dict(os.environ, APM_HOME=str(Path(tmp) / "home"), GIT_TERMINAL_PROMPT="0",
                   APM_NO_SCRIPTS="1", NO_COLOR="1", TERM="dumb")
        env.pop("FORCE_COLOR", None)
        code, stdout, stderr = command([apm, "update", "--dry-run", "--verbose"], stage, env=env)
        (output / "apm.stdout.txt").write_text(stdout)
        (output / "apm.stderr.txt").write_text(stderr)
        (output / "apm.exit.txt").write_text(str(code))
        if snapshot(stage) != before:
            raise ValueError("APM dry-run changed disposable checkout files")
        if code:
            raise ValueError(f"APM collector exited {code}; see raw evidence")
        return stdout + stderr


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--apm", default="apm")
    args = parser.parse_args()
    root, output = args.root.resolve(), args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    sections, failures = [], []

    def section(title, collect):
        try:
            text = collect()
        except Exception as error:
            text = "UNABLE TO VERIFY: " + str(error)
            failures.append(title)
        sections.append("## " + title + "\n\n<pre>" + html.escape(text) + "</pre>\n")

    # Config failure must not prevent the independent APM/audit reports.
    reviews = []
    try:
        reviews = validate_reviews(json.loads((root / ".github/dependency-review.json").read_text()))
    except Exception as error:
        failures.append("watch configuration")
        sections.append("Watch configuration UNABLE TO VERIFY: " + html.escape(str(error)))

    section("APM freshness", lambda: collect_apm(root, output, args.apm) +
            "\nRetained pins are not proof of freshness; proposed ref changes and retained pins are shown above.")

    def audit():
        desktop = root / "apps/desktop"
        code, stdout, stderr = command(PNPM + ["audit", "--json"], desktop)
        (output / "audit.json").write_text(stdout)
        (output / "audit.stderr.txt").write_text(stderr)
        (output / "audit.exit.txt").write_text(str(code))
        oc, osout, oserr = command(PNPM + ["config", "get", "overrides", "--json"], desktop)
        (output / "overrides.json").write_text(osout)
        (output / "overrides.stderr.txt").write_text(oserr)
        overrides = json.loads(osout)
        if oc or not isinstance(overrides, dict):
            raise ValueError("Resolved overrides unavailable")
        return "\n\n".join(audit_lines(json.loads(stdout), code, reviews, overrides))

    # pnpm may use scratch files, but may not rewrite committed dependency inputs.
    inputs = [root / "apm.yml", root / "apps/desktop/package.json", root / "apps/desktop/pnpm-lock.yaml",
              root / "apps/desktop/pnpm-workspace.yaml"]
    before = {path: path.read_bytes() for path in inputs}
    section("Desktop advisories", audit)

    def watches():
        lines, unknown = [], False
        for index, entry in enumerate(reviews):
            try:
                advisory = None
                if entry["condition"]["kind"] == "patched":
                    request = urllib.request.Request("https://api.github.com/advisories/" + entry["advisory"],
                                                     headers={"User-Agent": "orkworks-dependency-sweep"})
                    with urllib.request.urlopen(request, timeout=20) as response:
                        raw = response.read().decode()
                    (output / f"watch-{index}.json").write_text(raw)
                    advisory = json.loads(raw)
                lines.append(watch_line(entry, advisory, datetime.now(timezone.utc).date()))
            except Exception as error:
                unknown = True
                lines.append(entry["advisory"] + ": UNABLE TO VERIFY: " + str(error))
        if unknown:
            failures.append("watch metadata")
        return "\n".join(lines) or "No active watch entries."

    section("Dismissal follow-up", watches)
    if any(path.read_bytes() != content for path, content in before.items()):
        failures.append("dependency inputs changed")
    code, state, error = command(["git", "status", "--porcelain", "--untracked-files=all"], root)
    (output / "checkout-status.txt").write_text(state + error)
    if code or state.strip():
        failures.append("checkout integrity (expected a clean disposable checkout)")
    versions = {}
    for name, argv in (("Node", ["node", "--version"]), ("pnpm", PNPM + ["--version"]),
                       ("APM", [args.apm, "--version"]), ("source commit", ["git", "rev-parse", "HEAD"])):
        code, stdout, stderr = command(argv, root, timeout=15)
        versions[name] = stdout.strip() if code == 0 else "UNAVAILABLE"
        if code:
            failures.append(name + " version")
    report = "# Dependency freshness\n\n" + datetime.now(timezone.utc).isoformat() + "\n\n"
    report += "<pre>" + html.escape(json.dumps(versions, indent=2)) + "</pre>\n\n"
    report += "\n".join(sections)
    report += "\nCollection: " + ("INCOMPLETE — " + ", ".join(failures) if failures else "complete") + "\n"
    (output / "summary.md").write_text(report)
    if os.environ.get("GITHUB_STEP_SUMMARY"):
        with open(os.environ["GITHUB_STEP_SUMMARY"], "a") as summary:
            summary.write(report)
    print(report)
    return int(bool(failures))


if __name__ == "__main__":
    raise SystemExit(main())
