"""One disposable stdio MCP canary. No native client or network is started."""
from __future__ import annotations

import copy
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import re
import secrets
import selectors
import sys
import time

PEER_PATH = Path(__file__).with_name("peer.py")
if not PEER_PATH.exists():
    PEER_PATH = Path(__file__).parent.parent / "parent-continuation" / "peer.py"
spec = importlib.util.spec_from_file_location("ork742_peer", PEER_PATH)
peer_module = importlib.util.module_from_spec(spec)
sys.modules[spec.name] = peer_module
spec.loader.exec_module(peer_module)
Peer, RunCursor = peer_module.Peer, peer_module.RunCursor


def digest(value):
    return hashlib.sha256(json.dumps(value, sort_keys=True, separators=(",", ":")).encode()).hexdigest()


def seeded_peer(replay=False):
    peer = Peer()
    peer.approve_plan(peer.read(peer.binding).version)
    peer.reserve(peer.binding, peer.read(peer.binding), "task-a")
    peer.set_gates(True, ("FAKE-retained-blocker",), "FAKE-retained-question")
    if replay:
        for kind in ("report", "approval", "blocker", "capacity"):
            peer.emit(kind, "canary")
        for n in range(257):
            peer.emit("capacity", f"gap-{n}")
        peer.emit("report", "after-gap")
        old = peer.binding
        peer.end_runtime()
        peer.resume(old.native_id, old.workspace, old.configuration_digest)
    return peer


class Protocol:
    """Private fixture adapter, not a product protocol or credential validator."""
    def __init__(self, proposal=None, phase=1, record=None, delay=None, nonce=None):
        self.proposal = proposal or json.loads(Path(__file__).with_name("proposal.json").read_text())
        self.phase = phase
        self.peer = seeded_peer(phase == 2)
        self.record = record or (lambda value: None)
        self.delay = delay or (lambda: None)
        self.nonce = nonce or (lambda: secrets.token_hex(32))
        self.initialized = False
        self.ready = False
        self.ids = set()
        self.calls = 0
        self.stage = 0
        self.bootstrapped = False
        self.waiting = False
        self.result_nonce = None
        self.snapshot_id = None
        self.snapshot = None
        self.action = None
        self.previous = None
        self.done = False

    def state(self):
        snapshot = self.peer.read(self.peer.binding)
        state = {"version": snapshot.version, "cursor": snapshot.cursor.value,
                 "state": snapshot.state, "stageContinueRequired": snapshot.stage_continue,
                 "blockers": list(snapshot.blockers), "clarification": snapshot.clarification,
                 "grant": snapshot.grant, "reserved": list(snapshot.reserved)}
        return snapshot, state

    def handle(self, message):
        request_id = message.get("id") if isinstance(message, dict) else None
        try:
            if (not isinstance(message, dict) or message.get("jsonrpc") != "2.0"
                    or set(message) - {"jsonrpc", "id", "method", "params"}):
                raise ValueError("request")
            method, params = message.get("method"), message.get("params", {})
            if not isinstance(params, dict):
                raise ValueError("params")
            if method == "notifications/initialized" and request_id is None:
                if not self.initialized or self.ready:
                    raise ValueError("initialization")
                self.ready = True
                return None
            if (type(request_id) is not int or request_id < 0 or request_id > 2147483647):
                if not isinstance(request_id, str) or not re.fullmatch(r"[A-Za-z0-9._:-]{1,128}", request_id):
                    raise ValueError("request id")
            if request_id in self.ids:
                raise ValueError("duplicate request")
            self.ids.add(request_id)
            if method == "initialize":
                if self.initialized or not isinstance(params.get("protocolVersion"), str):
                    raise ValueError("initialize")
                self.initialized = True
                result = {"protocolVersion": "2025-06-18", "capabilities": {"tools": {}},
                          "serverInfo": {"name": "ork742-synthetic", "version": "1.0.0"}}
            elif not self.ready:
                return {"jsonrpc": "2.0", "id": request_id,
                        "error": {"code": -32002, "message": "Fixture not initialized"}}
            elif method == "ping":
                result = {}
            elif method == "tools/list":
                if set(params) - {"_meta"}:
                    raise ValueError("list params")
                result = {"tools": self.proposal["tools"]}
            elif method == "tools/call":
                if set(params) - {"name", "arguments", "_meta"}:
                    raise ValueError("call params")
                name, arguments = params.get("name"), params.get("arguments", {})
                if not isinstance(arguments, dict) or self.done:
                    raise ValueError("call state")
                self.calls += 1
                if self.calls > self.proposal["limits"]["maxToolCalls"]:
                    raise ValueError("call cap")
                self.record({"kind": "call", "phase": self.phase, "requestId": request_id,
                             "tool": name if name in {t["name"] for t in self.proposal["tools"]} else "unknown",
                             "argumentsHash": digest(arguments)})
                value = self.call(name, arguments)
                result = {"content": [{"type": "text", "text": json.dumps(value, separators=(",", ":"))}],
                          "structuredContent": value, "isError": False}
            else:
                raise ValueError("method")
            return {"jsonrpc": "2.0", "id": request_id, "result": result}
        except (ValueError, TypeError, KeyError):
            self.record({"kind": "fatal", "phase": self.phase, "reason": "invalid fixture request"})
            return {"jsonrpc": "2.0", "id": request_id,
                    "error": {"code": -32602, "message": "Invalid fixture request"}}

    def call(self, name, arguments):
        if name == "wait_for_run_event":
            if (self.phase != 1 or not self.bootstrapped or self.waiting
                    or set(arguments) != {"cursor"} or type(arguments["cursor"]) is not int
                    or arguments["cursor"] != self.peer.cursor or self.stage > 6):
                raise ValueError("wait")
            self.record({"kind": "pending", "phase": self.phase, "stage": self.stage})
            self.delay()
            if self.stage == 4:
                receipt = copy.deepcopy(self.previous)
            elif self.stage == 5:
                cursor = self.peer.cursor
                for n in range(257):
                    self.peer.emit("capacity", f"gap-{n}")
                receipt = self.peer.result(self.peer.wait(self.peer.binding, RunCursor(cursor)))
            else:
                request = self.peer.wait(self.peer.binding, RunCursor(arguments["cursor"]))
                kind = ("report", "approval", "blocker", "capacity")[self.stage] if self.stage < 4 else "report"
                self.peer.emit(kind, "after-gap" if self.stage == 6 else "canary")
                receipt = self.peer.result(request)
                self.previous = copy.deepcopy(receipt)
            self.waiting = True
            self.result_nonce = self.nonce()
            self.action = None if receipt["status"] == "refresh_required" else f"event-{receipt['cursor']}"
            self.record({"kind": "delivered", "phase": self.phase, "stage": self.stage,
                         "status": receipt["status"], "nonceHash": digest(self.result_nonce)})
            return {"status": receipt["status"], "resultNonce": self.result_nonce}
        if name == "read_run_state":
            bootstrap = arguments == {"bootstrap": True}
            if bootstrap:
                if self.bootstrapped:
                    raise ValueError("bootstrap replay")
                self.bootstrapped = True
            elif (set(arguments) != {"resultNonce"} or not self.waiting
                  or arguments["resultNonce"] != self.result_nonce):
                raise ValueError("result nonce")
            self.snapshot, state = self.state()
            self.snapshot_id = self.nonce()
            action = "resume-observed" if self.phase == 2 else self.action
            self.record({"kind": "state", "phase": self.phase, "stage": self.stage,
                         "stateHash": digest(state), "cursor": state["cursor"], "version": state["version"],
                         "snapshotHash": digest(self.snapshot_id)})
            if self.phase == 1 and self.stage == 5:
                self.stage = 6
                self.waiting = False
                self.action = None
            return {**state, "snapshotId": self.snapshot_id, "nextAction": action}
        if name == "acknowledge":
            expected = "resume-observed" if self.phase == 2 else self.action
            if (set(arguments) != {"snapshotId", "actionId"} or self.snapshot is None
                    or arguments["snapshotId"] != self.snapshot_id or expected is None
                    or arguments["actionId"] != expected or (self.phase == 1 and not self.waiting)):
                raise ValueError("snapshot handle")
            self.peer.acknowledge(self.peer.binding, self.snapshot, expected)
            self.done = self.phase == 2 or self.stage == 6
            self.record({"kind": "ack", "phase": self.phase, "stage": self.stage,
                         "effects": self.peer.effect_count, "done": self.done})
            self.stage += 1
            self.waiting = False
            self.snapshot_id = None
            return {"done": self.done, "effects": self.peer.effect_count}
        raise ValueError("tool")


def parse_frame(line):
    if len(line) > 16384:
        raise ValueError("MCP input cap")
    def pairs(items):
        out = {}
        for key, value in items:
            if key in out:
                raise ValueError("Duplicate JSON key")
            out[key] = value
        return out
    def constant(_):
        raise ValueError("Invalid JSON constant")
    try:
        value = json.loads(line.decode("utf-8"), object_pairs_hook=pairs, parse_constant=constant)
    except (UnicodeDecodeError, RecursionError) as error:
        raise ValueError("Invalid MCP frame") from error
    if not isinstance(value, dict):
        raise ValueError("MCP object required")
    return value


def serve(root):
    root = Path(root)
    if root.is_symlink() or not root.is_dir():
        raise ValueError("owned root")
    context_path = root / "context.json"
    if context_path.is_symlink() or context_path.stat().st_size > 4096:
        raise ValueError("context")
    context = json.loads(context_path.read_text())
    if (set(context) != {"phase", "generation", "nativePid"} or context["phase"] not in (1, 2)
            or not re.fullmatch(r"[0-9a-f]{64}", context["generation"])
            or type(context["nativePid"]) is not int or context["nativePid"] <= 0
            or os.getpgrp() != context["nativePid"]):
        raise ValueError("runtime binding")
    if set(os.environ) != {"PATH", "HOME", "TMPDIR", "LC_ALL", "__CF_USER_TEXT_ENCODING"}:
        raise ValueError("helper environment")
    log = root / f"mcp-{context['phase']}.jsonl"
    fd = os.open(log, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    written = 0
    def record(value):
        nonlocal written
        data = (json.dumps({**value, "generation": context["generation"],
                            "nativePid": context["nativePid"]}, separators=(",", ":")) + "\n").encode()
        written += len(data)
        if written > 131072:
            raise ValueError("helper log cap")
        os.write(fd, data)
    record({"kind": "startup", "phase": context["phase"], "pid": os.getpid(),
            "environmentNames": sorted(os.environ)})
    protocol = Protocol(phase=context["phase"], record=record, delay=lambda: time.sleep(1))
    selector = selectors.DefaultSelector()
    selector.register(sys.stdin.fileno(), selectors.EVENT_READ)
    os.set_blocking(sys.stdin.fileno(), False)
    buffer = b""
    deadline = time.monotonic() + 250
    try:
        while time.monotonic() < deadline:
            if not selector.select(0.1):
                continue
            data = os.read(sys.stdin.fileno(), 16385)
            if not data:
                if buffer:
                    raise ValueError("Incomplete MCP frame")
                return
            buffer += data
            if len(buffer) > 16384:
                raise ValueError("MCP input cap")
            while b"\n" in buffer:
                line, buffer = buffer.split(b"\n", 1)
                message = parse_frame(line)
                response = protocol.handle(message)
                if response is not None:
                    output = (json.dumps(response, separators=(",", ":")) + "\n").encode()
                    if len(output) > 16384:
                        raise ValueError("MCP output cap")
                    sys.stdout.buffer.write(output)
                    sys.stdout.buffer.flush()
                    if "error" in response:
                        raise ValueError("Rejected MCP request")
        raise ValueError("MCP deadline")
    finally:
        selector.close()
        os.close(fd)


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", required=True)
    try:
        serve(parser.parse_args().root)
    except (ValueError, OSError, json.JSONDecodeError):
        print("Synthetic MCP stopped", file=sys.stderr)
        raise SystemExit(2)
