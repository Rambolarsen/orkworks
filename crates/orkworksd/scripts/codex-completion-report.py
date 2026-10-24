#!/usr/bin/env python3
"""Publish one native hook event to an already-created completion queue."""

import fcntl
import json
import os
import re
import secrets
import stat
import sys
import time

MAX_INPUT = 1024 * 1024
MAX_QUEUE = 1024 * 1024
MAX_RECORD = 4096
MAX_PENDING = 256
MAX_MARKERS = 256
MAX_DIRECTORY_ENTRIES = 512
LOCK_WAIT_SECONDS = 2.0
EVENTS = {"UserPromptSubmit", "PreToolUse", "PermissionRequest", "PostToolUse", "Stop"}
HEADER_KEYS = {"version", "streamId", "generation", "hookFingerprint", "nextSequence", "acknowledgedSequence", "records"}
RECORD_KEYS = {"version", "sequence", "observedMicros", "hookFingerprint", "rootId", "turnId", "event", "toolName", "toolUseId"}
UUID_RE = re.compile(r"^[0-9a-f]{32}$")
FINGERPRINT_RE = re.compile(r"^[0-9a-f]{64}$")
ID_RE = re.compile(r"^[A-Za-z0-9_-]{1,128}$", re.ASCII)
ERROR = "completion report failed\n"


class Invalid(Exception):
    pass


def _pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise Invalid()
        result[key] = value
    return result


def _reject_constant(_value):
    raise Invalid()


def _loads(data):
    try:
        return json.loads(data, object_pairs_hook=_pairs, parse_constant=_reject_constant)
    except (ValueError, UnicodeError, RecursionError) as error:
        raise Invalid() from error


def _json_bytes(value):
    try:
        return json.dumps(value, separators=(",", ":"), ensure_ascii=True, allow_nan=False).encode("ascii")
    except (ValueError, UnicodeError, RecursionError) as error:
        raise Invalid() from error


def _uint(value):
    if type(value) is not int or value < 0 or value > 0xFFFFFFFFFFFFFFFF:
        raise Invalid()
    return value


def _scalar(value, identifier=False, tool=False):
    if value is None:
        return None
    if not isinstance(value, str) or not value or len(value.encode("utf-8")) > 128:
        raise Invalid()
    if identifier:
        if not ID_RE.fullmatch(value):
            raise Invalid()
    elif tool:
        if any(ord(char) < 0x21 or ord(char) > 0x7E for char in value):
            raise Invalid()
    return value


def _owned_private_directory(directory):
    fd = os.open(directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC)
    info = os.fstat(fd)
    if not stat.S_ISDIR(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077:
        os.close(fd)
        raise Invalid()
    return fd


def _open_regular(dir_fd, name, flags=os.O_RDONLY):
    fd = os.open(name, flags | os.O_NOFOLLOW | os.O_CLOEXEC | os.O_NONBLOCK, dir_fd=dir_fd)
    info = os.fstat(fd)
    if not stat.S_ISREG(info.st_mode) or info.st_uid != os.getuid() or info.st_mode & 0o077 or info.st_nlink != 1:
        os.close(fd)
        raise Invalid()
    return fd, info


def _list_names_bounded(dir_fd):
    names = []
    with os.scandir(dir_fd) as entries:
        for entry in entries:
            if len(names) >= MAX_DIRECTORY_ENTRIES:
                raise Invalid()
            names.append(entry.name)
    return names


def _verify_named_inode(dir_fd, name, info):
    try:
        current = os.stat(name, dir_fd=dir_fd, follow_symlinks=False)
    except OSError as error:
        raise Invalid() from error
    if not stat.S_ISREG(current.st_mode) or current.st_uid != os.getuid() or current.st_mode & 0o077 or current.st_nlink != 1 or (current.st_dev, current.st_ino) != (info.st_dev, info.st_ino):
        raise Invalid()


def _read_bounded(fd, maximum):
    size = os.fstat(fd).st_size
    if size < 0 or size > maximum:
        raise Invalid()
    chunks = []
    remaining = maximum + 1
    while remaining:
        chunk = os.read(fd, min(65536, remaining))
        if not chunk:
            break
        chunks.append(chunk)
        remaining -= len(chunk)
    data = b"".join(chunks)
    if len(data) > maximum:
        raise Invalid()
    return data


def _create_marker(dir_fd, name):
    fd = os.open(name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=dir_fd)
    try:
        os.fsync(fd)
        os.fsync(dir_fd)
    finally:
        os.close(fd)


def _poison(dir_fd):
    try:
        fd = os.open("poison", os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=dir_fd)
        try:
            os.fsync(fd)
            os.fsync(dir_fd)
        finally:
            os.close(fd)
    except FileExistsError:
        try:
            fd, _ = _open_regular(dir_fd, "poison")
        except (OSError, Invalid):
            pass
        else:
            os.close(fd)
    except OSError:
        pass


def _queue_files(dir_fd, own_marker, own_temp=None):
    names = _list_names_bounded(dir_fd)
    inflights = [name for name in names if name.startswith("inflight-")]
    if len(inflights) > MAX_MARKERS or any(not re.fullmatch(r"inflight-[0-9a-f]{32}", name) for name in inflights):
        raise Invalid()
    if any(name.startswith("pending-") and name != own_temp for name in names):
        raise Invalid()
    allowed = {"queue.json", "queue.lock", "poison", own_marker, *inflights}
    if own_temp is not None:
        if not re.fullmatch(r"pending-[0-9a-f]{32}", own_temp):
            raise Invalid()
        allowed.add(own_temp)
    if set(names) - allowed:
        raise Invalid()
    for name in inflights:
        fd, _ = _open_regular(dir_fd, name)
        os.close(fd)
    if own_temp is not None:
        fd, _ = _open_regular(dir_fd, own_temp)
        os.close(fd)
    if "poison" in names:
        fd, _ = _open_regular(dir_fd, "poison")
        os.close(fd)
        raise Invalid()


def _parse_header(data, fingerprint):
    header = _loads(data)
    if not isinstance(header, dict) or set(header) != HEADER_KEYS:
        raise Invalid()
    if type(header["version"]) is not int or header["version"] != 1:
        raise Invalid()
    if not isinstance(header["streamId"], str) or not UUID_RE.fullmatch(header["streamId"]):
        raise Invalid()
    if not isinstance(header["hookFingerprint"], str) or not FINGERPRINT_RE.fullmatch(header["hookFingerprint"]) or header["hookFingerprint"] != fingerprint:
        raise Invalid()
    _uint(header["generation"])
    next_sequence = _uint(header["nextSequence"])
    acknowledged = _uint(header["acknowledgedSequence"])
    records = header["records"]
    if not isinstance(records, list) or len(records) > MAX_PENDING or acknowledged >= next_sequence:
        raise Invalid()
    if len(records) != next_sequence - acknowledged - 1:
        raise Invalid()
    for offset, item in enumerate(records, start=1):
        if not isinstance(item, dict) or set(item) != RECORD_KEYS:
            raise Invalid()
        if type(item["version"]) is not int or item["version"] != 1:
            raise Invalid()
        if _uint(item["sequence"]) != acknowledged + offset:
            raise Invalid()
        observed = item["observedMicros"]
        if type(observed) is not int or observed <= 0 or observed > 0x7FFFFFFFFFFFFFFF:
            raise Invalid()
        if item["hookFingerprint"] != fingerprint:
            raise Invalid()
        if _scalar(item["rootId"], identifier=True) is None:
            raise Invalid()
        _scalar(item["turnId"], identifier=True)
        if item["event"] not in EVENTS:
            raise Invalid()
        _scalar(item["toolName"], tool=True)
        _scalar(item["toolUseId"], identifier=True)
        if len(_json_bytes(item)) > MAX_RECORD:
            raise Invalid()
    return header


def _parse_payload(payload, event, fingerprint):
    body = _loads(payload)
    if not isinstance(body, dict) or body.get("hook_event_name") != event:
        raise Invalid()
    root_id = _scalar(body.get("session_id"), identifier=True)
    if root_id is None:
        raise Invalid()
    return {
        "version": 1,
        "sequence": 0,
        "observedMicros": time.time_ns() // 1000,
        "hookFingerprint": fingerprint,
        "rootId": root_id,
        "turnId": _scalar(body.get("turn_id"), identifier=True),
        "event": event,
        "toolName": _scalar(body.get("tool_name"), tool=True),
        "toolUseId": _scalar(body.get("tool_use_id"), identifier=True),
    }


def _lock(lock_fd):
    deadline = time.monotonic() + LOCK_WAIT_SECONDS
    while True:
        try:
            fcntl.flock(lock_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            return
        except BlockingIOError:
            if time.monotonic() >= deadline:
                raise Invalid()
            time.sleep(min(0.02, max(0.0, deadline - time.monotonic())))


def publish(directory, event, fingerprint):
    dir_fd = _owned_private_directory(directory)
    marker = "inflight-" + secrets.token_hex(16)
    lock_fd = None
    marker_created = False
    lock_held = False
    succeeded = False
    try:
        # The sidecar creates the stream. This short lock section makes marker
        # visibility atomic with the consumer's no-inflight check.
        lock_fd, lock_info = _open_regular(dir_fd, "queue.lock")
        _verify_named_inode(dir_fd, "queue.lock", lock_info)
        _lock(lock_fd)
        lock_held = True
        _verify_named_inode(dir_fd, "queue.lock", lock_info)
        _create_marker(dir_fd, marker)
        marker_created = True
        queue_fd, _ = _open_regular(dir_fd, "queue.json")
        os.close(queue_fd)
        names = _list_names_bounded(dir_fd)
        if "poison" in names:
            fd, _ = _open_regular(dir_fd, "poison")
            os.close(fd)
            raise Invalid()
        _queue_files(dir_fd, marker)
        fcntl.flock(lock_fd, fcntl.LOCK_UN)
        lock_held = False

        # Parsing happens outside the shared critical section.
        if event not in EVENTS or not FINGERPRINT_RE.fullmatch(fingerprint):
            raise Invalid()
        payload = _read_bounded(sys.stdin.fileno(), MAX_INPUT)
        item = _parse_payload(payload, event, fingerprint)

        _lock(lock_fd)
        lock_held = True
        _verify_named_inode(dir_fd, "queue.lock", lock_info)
        _queue_files(dir_fd, marker)
        queue_fd, queue_info = _open_regular(dir_fd, "queue.json")
        try:
            original = _read_bounded(queue_fd, MAX_QUEUE)
            header = _parse_header(original, fingerprint)
            _verify_named_inode(dir_fd, "queue.json", queue_info)
        finally:
            os.close(queue_fd)
        next_sequence = header["nextSequence"]
        if next_sequence >= 0xFFFFFFFFFFFFFFFF or len(header["records"]) >= MAX_PENDING:
            raise Invalid()
        item["sequence"] = next_sequence
        if item["observedMicros"] > 0x7FFFFFFFFFFFFFFF or len(_json_bytes(item)) > MAX_RECORD:
            raise Invalid()
        header["records"].append(item)
        header["nextSequence"] = next_sequence + 1
        replacement = _json_bytes(header)
        if len(replacement) > MAX_QUEUE:
            raise Invalid()

        temp_name = "pending-" + secrets.token_hex(16)
        temp_fd = os.open(temp_name, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC, 0o600, dir_fd=dir_fd)
        try:
            view = memoryview(replacement)
            while view:
                written = os.write(temp_fd, view)
                if written <= 0:
                    raise OSError("short write")
                view = view[written:]
            os.fsync(temp_fd)
            temp_info = os.fstat(temp_fd)
        finally:
            os.close(temp_fd)
        _verify_named_inode(dir_fd, "queue.lock", lock_info)
        _queue_files(dir_fd, marker, temp_name)
        queue_fd, current_queue_info = _open_regular(dir_fd, "queue.json")
        os.close(queue_fd)
        if (current_queue_info.st_dev, current_queue_info.st_ino) != (queue_info.st_dev, queue_info.st_ino):
            raise Invalid()
        _verify_named_inode(dir_fd, "queue.json", queue_info)
        os.rename(temp_name, "queue.json", src_dir_fd=dir_fd, dst_dir_fd=dir_fd)
        os.fsync(dir_fd)
        _verify_named_inode(dir_fd, "queue.json", temp_info)
        if "poison" in _list_names_bounded(dir_fd):
            raise Invalid()
        os.unlink(marker, dir_fd=dir_fd)
        os.fsync(dir_fd)
        if "poison" in _list_names_bounded(dir_fd):
            raise Invalid()
        fcntl.flock(lock_fd, fcntl.LOCK_UN)
        lock_held = False
        marker_created = False
        succeeded = True
    finally:
        if not succeeded:
            # Failure must be visible before releasing a publication lock,
            # especially after unlink succeeded but its directory sync failed.
            _poison(dir_fd)
            if marker_created:
                try:
                    os.stat(marker, dir_fd=dir_fd, follow_symlinks=False)
                except FileNotFoundError:
                    if not lock_held and lock_fd is not None:
                        try:
                            _lock(lock_fd)
                            lock_held = True
                        except (OSError, Invalid):
                            pass
                    if lock_held:
                        try:
                            _create_marker(dir_fd, marker)
                        except OSError:
                            pass
        if lock_held and lock_fd is not None:
            try:
                fcntl.flock(lock_fd, fcntl.LOCK_UN)
            except OSError:
                pass
        if lock_fd is not None:
            os.close(lock_fd)
        os.close(dir_fd)


def main(argv):
    if len(argv) != 4:
        raise Invalid()
    publish(argv[1], argv[2], argv[3])


if __name__ == "__main__":
    try:
        main(sys.argv)
    except BaseException:
        sys.stderr.write(ERROR)
        sys.exit(2)
