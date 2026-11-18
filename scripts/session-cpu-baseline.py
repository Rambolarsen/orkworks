#!/usr/bin/env python3
"""Measure an explicitly selected macOS orkworksd without changing app state."""
import argparse
from dataclasses import dataclass
import json
import math
from pathlib import Path
import re
import subprocess
import sys
import time


@dataclass(frozen=True)
class Process:
    started: str
    executable: str
    cpu_seconds: float


def cpu_seconds(value):
    match = re.fullmatch(r"(?:(\d+)-)?(?:(\d+):)?(\d+):(\d+(?:\.\d+)?)", value)
    if not match:
        raise ValueError("Invalid process CPU counter")
    days, hours, minutes, seconds = match.groups()
    if float(seconds) >= 60 or (hours is not None and int(minutes) >= 60):
        raise ValueError("Invalid process CPU counter")
    return int(days or 0) * 86400 + int(hours or 0) * 3600 + int(minutes) * 60 + float(seconds)


def read_process(pid):
    try:
        output = subprocess.check_output(
            ["ps", "-p", str(pid), "-o", "pid=,lstart=,time=,comm="],
            text=True, stderr=subprocess.DEVNULL, timeout=5,
        )
    except (OSError, subprocess.SubprocessError) as error:
        raise ValueError("Cannot inspect selected process; it may have exited or access is denied") from error
    fields = output.strip().split(None, 7)
    if len(fields) != 8 or fields[0] != str(pid) or Path(fields[7]).name != "orkworksd":
        raise ValueError("Selected PID is not an inspectable orkworksd process")
    return Process(" ".join(fields[1:6]), fields[7], cpu_seconds(fields[6]))


def measure(pid, seconds):
    if pid <= 0 or not math.isfinite(seconds) or not 1 <= seconds <= 60:
        raise ValueError("Use a positive PID and a duration from 1 to 60 seconds")
    before = read_process(pid)
    started = time.monotonic()
    time.sleep(seconds)
    after = read_process(pid)
    elapsed = time.monotonic() - started
    delta = after.cpu_seconds - before.cpu_seconds
    if (before.started, before.executable) != (after.started, after.executable) or delta < 0:
        raise ValueError("Selected process changed during measurement; discard this interval")
    if elapsed <= 0:
        raise ValueError("Invalid measured wall interval")
    return {
        "pid": pid,
        "executable": "orkworksd",
        "elapsed_seconds": round(elapsed, 4),
        "cpu_seconds": round(delta, 4),
        "cpu_percent_one_core": round(100 * delta / elapsed, 2),
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pid", type=int, help="explicitly selected orkworksd PID")
    parser.add_argument("--seconds", type=float, default=10, help="1–60 seconds; default 10")
    args = parser.parse_args()
    try:
        if sys.platform != "darwin":
            raise ValueError("This diagnostic uses macOS ps fields and requires macOS")
        print(json.dumps(measure(args.pid, args.seconds), sort_keys=True))
    except ValueError as error:
        print(str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
