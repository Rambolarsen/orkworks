"""Fixtures exercise report classification and actual collector subprocess boundaries."""
import importlib.util
import json
import os
import subprocess
from pathlib import Path
import tempfile
import time
import unittest
from datetime import date

spec = importlib.util.spec_from_file_location("freshness", Path(__file__).with_name("dependency-freshness.py"))
f = importlib.util.module_from_spec(spec)
spec.loader.exec_module(f)

GHSA = "GHSA-aaaa-bbbb-cccc"

def review(condition=None):
    return {"advisory": GHSA, "package": "example", "manifest": "apps/desktop/pnpm-lock.yaml",
            "rationale": "Optional build path is unused", "evidence": "https://example.com/review",
            "reviewed_at": "2026-11-17", "condition": condition or {"kind": "patched"}}


def audit():
    return {"advisories": {"1": {"module_name": "example", "severity": "high", "title": "Example advisory",
            "github_advisory_id": GHSA, "url": "https://github.com/advisories/" + GHSA,
            "vulnerable_versions": "<2.0.0", "patched_versions": ">=2.0.0",
            "findings": [{"paths": ["app>tool>example"]}]}},
            "metadata": {"vulnerabilities": {"info": 0, "low": 0, "moderate": 0, "high": 1, "critical": 0}}}


class ReportTests(unittest.TestCase):
    def test_clean_audit_is_complete_but_findings_are_not_collection_failures(self):
        self.assertEqual(f.audit_lines({"advisories": {}, "metadata": {"vulnerabilities": {"high": 0}}}, 0, [], {}), ["No advisories reported."])
        text = "\n".join(f.audit_lines(audit(), 1, [], {}))
        for expected in [GHSA, "high", "app>tool>example", ">=2.0.0", "ACTIONABLE"]:
            self.assertIn(expected, text)

    def test_unpatched_advisory_remains_actionable_in_a_mixed_complete_audit(self):
        payload = audit()
        payload["advisories"]["2"] = {
            "module_name": "picomatch", "severity": "high", "title": "Extglob ReDoS",
            "github_advisory_id": "GHSA-fc5q-39xc-vjpf", "patched_versions": None,
            "findings": [{"paths": ["app>vite>picomatch"]}]
        }
        payload["metadata"]["vulnerabilities"]["high"] = 2
        text = "\n".join(f.audit_lines(payload, 1, [], {}))
        self.assertIn("GHSA-fc5q-39xc-vjpf", text)
        self.assertIn("No patched version available", text)
        self.assertIn(">=2.0.0", text)
        self.assertEqual(text.count("ACTIONABLE"), 2)

    def test_missing_or_malformed_patch_field_still_fails_collection(self):
        for value in [0, [], {}, "", " "]:
            payload = audit(); payload["advisories"]["1"]["patched_versions"] = value
            with self.subTest(value=value), self.assertRaises(ValueError):
                f.audit_lines(payload, 1, [], {})
        payload = audit(); del payload["advisories"]["1"]["patched_versions"]
        with self.assertRaises(ValueError): f.audit_lines(payload, 1, [], {})

    def test_override_never_suppresses_and_only_matching_review_acknowledges(self):
        text = "\n".join(f.audit_lines(audit(), 1, [], {"example@<2": "2"}))
        self.assertIn("ACTIONABLE", text)
        self.assertIn("example@<2", text)
        text = "\n".join(f.audit_lines(audit(), 1, [review()], {}))
        self.assertIn("ACKNOWLEDGED", text)
        self.assertIn("https://example.com/review", text)
        other = review(); other["package"] = "another"
        self.assertIn("ACTIONABLE", "\n".join(f.audit_lines(audit(), 1, [other], {})))

    def test_unavailable_or_malformed_audit_never_becomes_clean(self):
        for payload, code in [({"error": {"code": "ECONNREFUSED"}}, 1), ({}, 0),
                              ({"advisories": {}}, 1), (audit(), 2),
                              ({"advisories": {}, "metadata": {"vulnerabilities": {"high": 1}}}, 0)]:
            with self.subTest(payload=payload, code=code), self.assertRaises(ValueError):
                f.audit_lines(payload, code, [], {})
        bad = audit(); del bad["advisories"]["1"]["findings"]
        with self.assertRaises(ValueError): f.audit_lines(bad, 1, [], {})

    def test_watch_conditions_met_unmet_unknown_and_withdrawn(self):
        entry = review(); today = date(2026, 11, 17)
        self.assertIn("UNCHANGED", f.watch_line(entry, {"withdrawn_at": None, "vulnerabilities": [{"package": {"name": "example", "ecosystem": "npm"}, "first_patched_version": None}]}, today))
        self.assertIn("MET", f.watch_line(entry, {"withdrawn_at": None, "vulnerabilities": [{"package": {"name": "example", "ecosystem": "npm"}, "first_patched_version": "2.0.0"}]}, today))
        self.assertIn("WITHDRAWN", f.watch_line(entry, {"withdrawn_at": "2026-11-01"}, today))
        with self.assertRaises(ValueError): f.watch_line(entry, {}, today)
        with self.assertRaises(ValueError): f.watch_line(entry, {"withdrawn_at": None, "vulnerabilities": []}, today)
        self.assertIn("MET", f.watch_line(review({"kind": "date", "date": "2026-11-17"}), None, today))
        self.assertIn("UNCHANGED", f.watch_line(review({"kind": "date", "date": "2026-11-18"}), None, today))

    def test_watch_configuration_rejects_missing_evidence_invalid_dates_and_duplicates(self):
        self.assertEqual(f.validate_reviews({"version": 1, "reviews": []}), [])
        self.assertEqual(f.validate_reviews({"version": 1, "reviews": [review()]}), [review()])
        for field, value in [("evidence", ""), ("reviewed_at", "yesterday"), ("advisory", "../../etc/passwd"), ("manifest", "docs/pnpm-lock.yaml"), ("condition", {"kind": "shell", "command": "anything"})]:
            entry = review(); entry[field] = value
            with self.subTest(field=field), self.assertRaises(ValueError):
                f.validate_reviews({"version": 1, "reviews": [entry]})
        with self.assertRaises(ValueError): f.validate_reviews({"version": 1, "reviews": [review(), review()]})

    def test_apm_collector_detects_writes_and_keeps_partial_stdout(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); (root / "apm.yml").write_text("name: fixture\n")
            binary = root / "fake-apm"
            binary.write_text("#!/bin/sh\nprintf 'retained pin; upstream unavailable\\n'\nprintf changed > apm.yml\nexit 1\n")
            binary.chmod(0o755)
            output = root / "output"; output.mkdir()
            with self.assertRaisesRegex(ValueError, "changed"):
                f.collect_apm(root, output, str(binary))
            self.assertIn("retained pin", (output / "apm.stdout.txt").read_text())
            self.assertEqual((root / "apm.yml").read_text(), "name: fixture\n")

    def test_apm_collector_enforces_no_scripts_and_isolated_home(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp); (root / "apm.yml").write_text("name: fixture\n")
            binary = root / "apm"
            binary.write_text('#!/bin/sh\nif [ "$APM_NO_SCRIPTS" != "1" ] || [ "$APM_HOME" = "' + str(root) + '" ]; then echo changed > apm.yml; exit 1; fi\necho guarded\n')
            binary.chmod(0o755)
            output = root / "output"; output.mkdir()
            from unittest.mock import patch
            with patch.dict(os.environ, {"APM_NO_SCRIPTS": "", "APM_HOME": str(root)}):
                self.assertIn("guarded", f.collect_apm(root, output, str(binary)))
            self.assertEqual((root / "apm.yml").read_text(), "name: fixture\n")

    def test_command_timeout_preserves_partial_report(self):
        with tempfile.TemporaryDirectory() as tmp:
            started = time.monotonic()
            result = f.command(["sh", "-c", "printf partial; sleep 10"], Path(tmp), timeout=0.5)
            self.assertLess(time.monotonic() - started, 5, "descendants must not keep capture pipes open")
            self.assertNotEqual(result[0], 0)
            self.assertIn("partial", result[1])
            self.assertIn("timeout", result[2].lower())

    def test_runner_publishes_partial_evidence_and_does_not_fail_for_findings(self):
        for payload, wanted in [(audit(), 0), ({"error": {"code": "ECONNREFUSED"}}, 1)]:
            with self.subTest(wanted=wanted), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp) / "repo"; root.mkdir()
                desktop = root / "apps/desktop"; desktop.mkdir(parents=True)
                (root / ".github").mkdir()
                (root / "apm.yml").write_text("name: fixture\n")
                (root / ".github/dependency-review.json").write_text('{"version":1,"reviews":[]}')
                for name in ["package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml"]:
                    (desktop / name).write_text("{}")
                for argv in [["git", "init"], ["git", "add", "."],
                             ["git", "-c", "user.name=Fixture", "-c", "user.email=fixture@example.com", "commit", "-m", "fixture"]]:
                    subprocess.run(argv, cwd=root, check=True, capture_output=True)
                binary = Path(tmp) / "bin"; binary.mkdir()
                (binary / "apm").write_text('#!/bin/sh\necho "Retained pin; no eligible tag"\n')
                (binary / "node").write_text('#!/bin/sh\necho v22.0.0\n')
                fixture = Path(tmp) / "audit.json"; fixture.write_text(json.dumps(payload))
                (binary / "pnpm").write_text('#!/bin/sh\nif [ "$1" != "with" ] || [ "$2" != "12.18.0" ]; then echo unsafe-bootstrap >&2; exit 2; fi\nshift 2\ncase "$1" in\naudit) cat "' + str(fixture) + '"; exit 1;;\nconfig) echo "{}";;\n*) echo 12.18.0;;\nesac\n')
                for path in binary.iterdir(): path.chmod(0o755)
                output = Path(tmp) / "output"
                env = dict(os.environ, PATH=str(binary) + os.pathsep + os.environ["PATH"])
                result = subprocess.run(["python3", str(Path(f.__file__)), "--root", str(root), "--output", str(output)],
                                        env=env, capture_output=True, text=True)
                self.assertEqual(result.returncode, wanted, result.stdout + result.stderr)
                report = (output / "summary.md").read_text()
                self.assertIn("Dismissal follow-up", report)
                self.assertIn("Retained pin", report)
                self.assertTrue((output / "audit.json").exists())
                self.assertIn("Collection: complete" if wanted == 0 else "UNABLE TO VERIFY", report)

    @unittest.skipUnless(os.environ.get("ORKWORKS_FRESHNESS_APM"), "set ORKWORKS_FRESHNESS_APM for pinned real-CLI qualification")
    def test_pinned_apm_fresh_checkout_does_not_run_trusted_lifecycle_or_write_files(self):
        apm = os.environ["ORKWORKS_FRESHNESS_APM"]
        self.assertIn("0.35.0", subprocess.check_output([apm, "--version"], text=True))
        with tempfile.TemporaryDirectory() as tmp:
            stage = Path(tmp) / "checkout"; stage.mkdir()
            source = Path(__file__).resolve().parent.parent / "apm.yml"
            manifest = source.read_text().split("\nlifecycle:", 1)[0]
            marker = Path(tmp) / "lifecycle-ran"
            manifest += "\nlifecycle:\n"
            for event in ("pre-install", "post-install", "pre-update", "post-update"):
                manifest += f"  {event}:\n    - type: command\n      command: touch {marker}\n"
            (stage / "apm.yml").write_text(manifest)
            env = dict(os.environ, APM_HOME=str(Path(tmp) / "home"), GIT_TERMINAL_PROMPT="0", NO_COLOR="1", APM_NO_SCRIPTS="1")
            env.pop("FORCE_COLOR", None)
            code, stdout, stderr = f.command([apm, "lifecycle", "trust"], stage, env=env)
            self.assertEqual(code, 0, stdout + stderr)
            before = f.snapshot(stage)
            code, stdout, stderr = f.command([apm, "update", "--dry-run", "--verbose"], stage, timeout=300, env=env)
            self.assertEqual(code, 0, stdout + stderr)
            self.assertEqual(f.snapshot(stage), before, stdout + stderr)
            self.assertFalse(marker.exists(), stdout + stderr)
            print("Real APM fresh-checkout qualification:\n" + stdout + stderr)
            blocked = Path(tmp) / "blocked"; blocked.mkdir()
            git = blocked / "git"
            real_git = subprocess.check_output(["which", "git"], text=True).strip()
            git.write_text('#!/bin/sh\ncase "$1" in\nconfig|version|--version) exec "' + real_git + '" "$@";;\n*) echo "fatal: fixture upstream unavailable" >&2; exit 128;;\nesac\n')
            git.chmod(0o755)
            env["PATH"] = str(blocked) + os.pathsep + env["PATH"]
            code, stdout, stderr = f.command([apm, "update", "--dry-run", "--verbose"], stage, timeout=30, env=env)
            self.assertNotEqual(code, 0, stdout + stderr)
            self.assertEqual(f.snapshot(stage), before)
            self.assertFalse(marker.exists())
            print("Unavailable-upstream qualification:\n" + stdout + stderr)


if __name__ == "__main__": unittest.main()
