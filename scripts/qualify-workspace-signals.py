#!/usr/bin/env python3
"""Classify one hook event without retaining raw payloads or granting authority."""
import argparse
import json
import os
from pathlib import Path
import sys

MAX_INPUT = 64 * 1024
HARNESSES = ("claude-code", "codex", "copilot", "opencode", "aider",
             "antigravity", "generic-shell", "gemini", "custom")
LOCATION_FIELDS = ("cwd", "old_cwd", "new_cwd", "tool_input.cwd", "tool_input.workdir",
                   "tool_input.working_directory", "toolArgs.cwd", "toolArgs.workdir",
                   "toolArgs.working_directory", "data.directory", "data.info.directory")
IDENTITY_FIELDS = ("session_id", "sessionId", "data.sessionID", "data.info.id")


class InvalidProbe(ValueError):
    pass


class Parser(argparse.ArgumentParser):
    def error(self, message):
        # argparse normally echoes invalid argv, which may contain private paths.
        raise InvalidProbe()


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise InvalidProbe()
        result[key] = value
    return result


def directory(value):
    if (not isinstance(value, str) or not value or len(value.encode("utf-8")) > 4096
            or any(ord(char) < 32 or 127 <= ord(char) <= 159 for char in value)):
        raise InvalidProbe()
    path = Path(value)
    if not path.is_absolute():
        raise InvalidProbe()
    path = path.resolve(strict=True)
    if not path.is_dir():
        raise InvalidProbe()
    return path


def fields(payload, names):
    for name in names:
        value = payload
        for key in name.split("."):
            if not isinstance(value, dict) or key not in value:
                break
            value = value[key]
        else:
            yield name, value


def classify(value, roots):
    try:
        path = directory(value)
    except (ValueError, OSError, RuntimeError):
        return "invalid"
    for name, root in roots.items():
        if path == root or root in path.parents:
            return name
    return "outside"


def main():
    parser = Parser(description=__doc__)
    parser.add_argument("--harness", choices=HARNESSES, required=True)
    for name in ("launch", "task", "other"):
        parser.add_argument("--" + name + "-root", required=True)
    args = parser.parse_args()
    roots = {name: directory(getattr(args, name + "_root"))
             for name in ("launch", "task", "other")}
    values = list(roots.values())
    if any(a == b or a in b.parents or b in a.parents
           for index, a in enumerate(values) for b in values[index + 1:]):
        raise InvalidProbe()
    raw = sys.stdin.buffer.read(MAX_INPUT + 1)
    if len(raw) > MAX_INPUT:
        raise InvalidProbe()
    payload = json.loads(raw.decode("utf-8"), object_pairs_hook=unique_object)
    if not isinstance(payload, dict):
        raise InvalidProbe()
    expected_id = os.environ.get("ORKWORKS_QUALIFY_NATIVE_SESSION_ID")
    result = {
        "harness": args.harness,
        "authority": "diagnostic-only",
        # Codex child hooks can share their parent's session ID. Equality below
        # is deliberately not treated as proof of root-session identity.
        "rootScope": "unverified",
        "locations": {name: classify(value, roots)
                      for name, value in fields(payload, LOCATION_FIELDS)},
        "identityMatches": {name: isinstance(value, str) and value == expected_id
                            for name, value in fields(payload, IDENTITY_FIELDS)}
                           if expected_id else {},
    }
    print(json.dumps(result, separators=(",", ":")))


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, RuntimeError, RecursionError):
        # Never echo a payload, native ID, filesystem path or exception message.
        print("Workspace probe input or roots invalid.", file=sys.stderr)
        sys.exit(2)
