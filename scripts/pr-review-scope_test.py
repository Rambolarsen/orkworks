"""Run the token-bearing workflow's inline delta/prompt against real Git history."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import textwrap
import unittest

WORKFLOW = Path(__file__).resolve().parents[1] / ".github/workflows/pr-review.yml"


class ReviewScopeTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.git("init", "-q")
        self.git("config", "user.email", "test@example.invalid")
        self.git("config", "user.name", "Review scope test")
        self.write("apps/desktop/base.txt", "baseline\n")
        self.base = self.commit()
        # Substantial changes on main must never appear as PR deletions.
        for i in range(10):
            self.write(f"apps/desktop/main-{i}.txt", "main only\n" * 60)
        self.advanced_base = self.commit()
        self.git("checkout", "-q", "-b", "feature", self.base)
        self.write("apps/desktop/feature.txt", "feature\n")
        self.write("crates/orkworksd/feature.txt", "feature\n")
        self.head = self.commit()
        # A tempting PR-controlled helper must never run in the privileged step.
        self.write("scripts/pr-review-delta.sh",
                   '#!/bin/sh\ntouch helper-executed\necho "99 9999"\n')
        (self.root / "scripts/pr-review-delta.sh").chmod(0o755)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        # Only replace the remote comment transport, not Git or the workflow logic.
        gh = self.bin / "gh"
        gh.write_text('#!/bin/sh\ncat "$COMMENTS_FILE"\n')
        gh.chmod(0o755)

    def git(self, *args):
        return subprocess.check_output(["git", *args], cwd=self.root, text=True).strip()

    def write(self, path, content):
        target = self.root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(content)

    def commit(self):
        self.git("add", ".")
        self.git("commit", "-qm", "fixture")
        return self.git("rev-parse", "HEAD")

    def run_step(self, step, env):
        workflow = WORKFLOW.read_text()
        block = workflow.split(f"      - id: {step}\n", 1)[1]
        script = block.split("        run: |\n", 1)[1].split("\n      -", 1)[0]
        # The prompt step is followed by a job rather than another step.
        script = script.split("\n  review:", 1)[0]
        result = subprocess.run(
            ["bash", "-e", "-o", "pipefail", "-c", textwrap.dedent(script)],
            cwd=self.root, env=env, capture_output=True, text=True, timeout=10,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        return result.stdout

    def scope(self, *, base=None, marker="", force=False, bot=False):
        comments = []
        if marker:
            comments.append({"user": {"login": "github-actions[bot]"},
                             "created_at": "2026-10-24T00:00:00Z",
                             "body": "### Automated code review\n"
                                     f"<!-- orkworks-automated-review: sha={marker} -->"})
        comment_file = self.root / "comments.json"
        comment_file.write_text(json.dumps([comments]))
        output = self.root / "output"
        output.write_text("")
        env = dict(os.environ, PATH=f"{self.bin}:{os.environ['PATH']}",
                   COMMENTS_FILE=str(comment_file), BASE_SHA=base or self.advanced_base,
                   HEAD_SHA=self.head, FORCE_REVIEW=str(force).lower(),
                   AUTHOR_IS_BOT=str(bot).lower(), GITHUB_OUTPUT=str(output),
                   GITHUB_REPOSITORY="example/repo", PR_NUMBER="123",
                   RUNNER_TEMP=str(self.root))
        stdout = self.run_step("delta", env)
        self.assertFalse((self.root / "helper-executed").exists(),
                         "privileged eligibility must not execute the PR helper")
        values = dict(line.split("=", 1) for line in output.read_text().splitlines())
        return values, stdout, env

    def test_diverged_base_does_not_trigger_automatic_review(self):
        values, stdout, _ = self.scope()
        self.assertEqual(values["review_base"], self.base)
        self.assertEqual(values["review_diff_base"], self.base)
        self.assertEqual(values["should_review"], "false")
        self.assertEqual(values["over_review_threshold"], "false")
        self.assertIn("non-docs code files: 2, changed lines: 2", stdout)

    def test_force_uses_full_pr_contribution_and_prompt_matches(self):
        values, _, env = self.scope(marker=self.head, force=True)
        self.assertEqual(values["review_base"], self.head)
        self.assertEqual(values["review_diff_base"], self.base)
        self.assertEqual(values["should_review"], "true")
        env["REVIEW_DIFF_BASE"] = values["review_diff_base"]
        self.run_step("prompt", env)
        prompt = (self.root / "review-prompt.txt").read_text()
        self.assertIn(f"git diff {self.base} {self.head}", prompt)
        self.assertEqual(self.git("diff", "--name-only", values["review_diff_base"], self.head),
                         "apps/desktop/feature.txt\ncrates/orkworksd/feature.txt")

    def test_initial_automatic_review_prompt_uses_pr_merge_base(self):
        self.write("apps/desktop/large.txt", "new code\n" * 501)
        self.head = self.commit()
        values, stdout, env = self.scope()
        self.assertEqual(values["should_review"], "true")
        self.assertEqual(values["review_base"], self.base)
        self.assertEqual(values["review_diff_base"], self.base)
        self.assertIn("non-docs code files: 3, changed lines: 503", stdout)
        env["REVIEW_DIFF_BASE"] = values["review_diff_base"]
        self.run_step("prompt", env)
        prompt = (self.root / "review-prompt.txt").read_text()
        self.assertIn(f"git diff {self.base} {self.head}", prompt)

    def test_unchanged_base_and_valid_marker(self):
        values, _, _ = self.scope(base=self.base)
        self.assertEqual(values["review_base"], self.base)
        values, stdout, _ = self.scope(marker=self.head)
        self.assertEqual(values["review_base"], self.head)
        self.assertEqual(values["should_review"], "false")
        self.assertIn("non-docs code files: 0, changed lines: 0", stdout)

    def test_invalid_or_nonancestor_marker_falls_back_to_pr_base(self):
        for marker in ("f" * 40, self.advanced_base):
            with self.subTest(marker=marker):
                values, _, _ = self.scope(marker=marker)
                self.assertEqual(values["review_base"], self.base)

    def test_unreviewed_increment_still_triggers_threshold(self):
        marker = self.head
        self.write("apps/desktop/large.txt", "new code\n" * 501)
        self.head = self.commit()
        values, stdout, _ = self.scope(marker=marker)
        self.assertEqual(values["review_base"], marker)
        self.assertEqual(values["should_review"], "true")
        self.assertIn("non-docs code files: 1, changed lines: 501", stdout)

    def test_forced_docs_only_and_bot_prs_are_skipped(self):
        self.git("checkout", "-q", "-b", "docs-only", self.base)
        self.write("apps/desktop/README.md", "docs\n")
        self.head = self.commit()
        values, _, _ = self.scope(force=True)
        self.assertEqual(values["should_review"], "false")
        self.write("apps/desktop/code.txt", "code\n")
        self.head = self.commit()
        values, _, _ = self.scope(force=True, bot=True)
        self.assertEqual(values["should_review"], "false")


if __name__ == "__main__":
    unittest.main()
