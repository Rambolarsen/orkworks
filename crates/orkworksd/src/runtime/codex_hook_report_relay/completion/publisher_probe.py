"""Test-only failure metadata; never print exception messages, paths or inputs."""
import fcntl
import json
import os
import runpy
import sys
import time


def capture_lock_timing(publish):
    # Each child records only timestamps on the host's monotonic clock.
    # Keep all diagnostic writes outside the publisher's critical sections.
    globals_ = publish.__globals__
    lock = globals_["_lock"]
    flock = fcntl.flock
    fsync = os.fsync
    now = time.monotonic_ns
    phases = ("marker", "publication", "recovery")
    samples = []
    held = None

    def timed_lock(fd):
        sample = dict(phase=phases[len(samples)], wait_start=now(), wait_end=None,
                      acquired=None, release_start=None, released=None, max_fsync=0)
        samples.append(sample)
        try:
            return lock(fd)
        finally:
            sample["wait_end"] = now()

    def timed_flock(fd, operation):
        nonlocal held
        if operation == fcntl.LOCK_UN and held is not None:
            # A peer may acquire the lock before this process resumes.
            release_start = now()
            result = flock(fd, operation)
            held["release_start"] = release_start
            held["released"] = now()
            held = None
            return result
        result = flock(fd, operation)
        if operation == (fcntl.LOCK_EX | fcntl.LOCK_NB):
            held = samples[-1]
            held["acquired"] = now()
        return result

    def timed_fsync(fd):
        if held is None:
            return fsync(fd)
        start = now()
        try:
            return fsync(fd)
        finally:
            held["max_fsync"] = max(held["max_fsync"], now() - start)

    globals_["_lock"] = timed_lock
    fcntl.flock = timed_flock
    os.fsync = timed_fsync
    return samples


script = sys.argv[1]
samples = None
try:
    namespace = runpy.run_path(script)
    if os.environ.get("ORKWORKS_TEST_COMPLETION_LOCK_TIMING") == "1":
        samples = capture_lock_timing(namespace["publish"])
    namespace["main"](sys.argv[1:])
except BaseException as error:
    stage, line = "load", 0
    trace = error.__traceback__
    while trace is not None:
        if trace.tb_frame.f_code.co_filename == script:
            stage = trace.tb_frame.f_code.co_name
            line = trace.tb_lineno
        trace = trace.tb_next
    category = "Invalid" if type(error).__name__ == "Invalid" else "OSError" if isinstance(error, OSError) else "other"
    number = error.errno if isinstance(error, OSError) and type(error.errno) is int else "none"
    sys.stderr.write(f"completion report failed; stage={stage}; exception={category}; errno={number}; line={line}\n")
    sys.exit(2)
finally:
    if samples is not None:
        sys.stderr.write(f"completion lock timing (ns): {json.dumps(samples)}\n")
