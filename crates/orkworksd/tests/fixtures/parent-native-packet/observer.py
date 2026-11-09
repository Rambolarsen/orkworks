"""Prepared synthetic native observer; importing/preparing never launches a harness."""
from __future__ import annotations

import hashlib
import json


def require(value, reason):
    if not value:
        raise ValueError(reason)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def failure_details(diagnostic, error):
    """Project fixed host metadata only; never stringify a caught exception."""
    import subprocess
    stages = {"prepare", "git-init", "source-check", "input-check", "instruction-discovery",
              "skill-discovery", "native-prompt", "native-history", "runtime-bind",
              "helper-receipt", "receipt-join", "runtime-check", "raw-cleanup"}
    category = "unexpected-error"
    for kind, label in ((json.JSONDecodeError, "invalid-json"), (OSError, "os-error"),
                        (subprocess.TimeoutExpired, "process-timeout"),
                        (subprocess.SubprocessError, "process-error"), (ValueError, "check-failed")):
        if isinstance(error, kind):
            category = label
            break
    process = diagnostic.get("process", {})
    if not isinstance(process, dict):
        process = {}
    exit_code = process.get("exitCode")
    started = process.get("started") is True
    if isinstance(error, subprocess.CalledProcessError):
        exit_code, started = error.returncode, True
    if type(exit_code) is not int or not -2147483648 <= exit_code <= 2147483647:
        exit_code = None
    number = error.errno if isinstance(error, OSError) else None
    if type(number) is not int or not 0 <= number <= 4095:
        number = None
    phase = diagnostic.get("phase")
    stage = diagnostic.get("stage")
    details = {"phase": phase if type(phase) is int and phase in (0, 1, 2) else 0,
            "stage": stage if isinstance(stage, str) and stage in stages else "unknown",
            "category": category, "osError": number,
            "process": {"started": started, "exitCode": exit_code,
                        "terminationStepsCompleted": process.get("terminationStepsCompleted") is True}}
    code = getattr(error, "history_check_code", None)
    if stage == "native-history" and type(code) is str and code in {
            "history-root-missing", "history-root-layout", "history-session-layout",
            "history-events-missing", "history-events-read", "history-prefix", "history-tail",
            "history-event-cap", "history-event-contract"}:
        details["checkCode"] = code
    return details


def validate_native_id(value):
    """Require an ordinary canonical UUID accepted by the pinned CLI profile."""
    import uuid
    try:
        parsed = uuid.UUID(value)
    except (ValueError, TypeError, AttributeError):
        raise ValueError("Invalid native canary UUID") from None
    require(type(value) is str and str(parsed) == value
            and parsed.variant == uuid.RFC_4122 and parsed.version in range(1, 9),
            "Native canary UUID must have valid version and variant bits")


def require_authorization(approval, proposal_bytes, manifest_bytes):
    expected = {"decision": "approve-one-batch", "proposalSha256": sha(proposal_bytes),
                "manifestSha256": sha(manifest_bytes), "maxNativePrompts": 2}
    require(type(approval) is dict and approval == expected, "Exact packet approval required")
    return True


def project_native(line):
    """Validate JSON while decoding only selected scalars, never skipped payloads."""
    import re
    require(isinstance(line, str) and len(line.encode()) <= 1048576, "Native line cap")
    n = len(line)
    data_keys = {"sessionId", "copilotVersion", "eventCount", "alreadyInUse", "sessionWasActive",
                 "continuePendingWork", "turnId", "interactionId", "toolCallId", "toolName", "success",
                 "model", "selectedModel", "mcpServerName", "mcpConfigServerName", "mcpToolName", "mcpTransport"}
    argument_keys = {"cursor", "resultNonce", "bootstrap", "snapshotId", "actionId"}
    def ws(i):
        while i < n and line[i] in " \t\r\n": i += 1
        return i
    def string(i):
        require(i < n and line[i] == '"', "JSON string expected")
        i += 1
        while i < n:
            c = line[i]
            if c == '"': return i + 1
            require(ord(c) >= 32, "Invalid JSON string")
            if c == "\\":
                i += 1
                require(i < n and line[i] in '"\\/bfnrtu', "Invalid JSON escape")
                if line[i] == "u":
                    require(re.fullmatch(r"[0-9a-fA-F]{4}", line[i+1:i+5]) is not None, "Invalid JSON escape")
                    i += 4
            i += 1
        raise ValueError("Incomplete JSON string")
    def value(i, depth=0, mode=None):
        require(depth <= 64 and i < n, "JSON depth or truncation")
        c = line[i]
        if c == '{':
            i = ws(i+1)
            out, keys = {}, set()
            if i < n and line[i] == '}':
                return ({"fields": []} if mode else None), i+1
            while True:
                start = i; i = string(i)
                require(i-start <= 256, "JSON key cap")
                key = json.loads(line[start:i])
                require(key not in keys and len(keys) < 256, "Duplicate or excessive JSON keys")
                keys.add(key)
                i = ws(i); require(i < n and line[i] == ':', "JSON colon expected"); i = ws(i+1)
                nested = "data" if mode == "root" and key == "data" else "arguments" if mode == "data" and key == "arguments" else None
                if nested:
                    require(i < n and line[i] == '{', "Native metadata object required")
                start = i
                item, i = value(i, depth+1, nested)
                allowed = {"type"} if mode == "root" else data_keys if mode == "data" else argument_keys if mode == "arguments" else set()
                if nested:
                    out[key] = item
                elif key in allowed:
                    require(line[start] not in '{[' and i-start <= 512, "Native metadata scalar cap")
                    out[key] = json.loads(line[start:i])
                i = ws(i); require(i < n and line[i] in ',}', "JSON separator expected")
                if line[i] == '}':
                    if mode: out["fields"] = sorted(keys)
                    return out if mode else None, i+1
                i = ws(i+1)
        if c == '[':
            i = ws(i+1)
            if i < n and line[i] == ']': return None, i+1
            while True:
                _, i = value(i, depth+1)
                i = ws(i); require(i < n and line[i] in ',]', "JSON array separator expected")
                if line[i] == ']': return None, i+1
                i = ws(i+1)
        if c == '"': return None, string(i)
        m = re.match(r'(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)', line[i:])
        require(m is not None, "Invalid JSON scalar")
        return None, i+len(m.group())
    start = ws(0)
    require(start < n and line[start] == '{', "Native root object required")
    row, end = value(start, mode="root")
    require(ws(end) == n, "Trailing JSON bytes")
    return row


class Trace:
    def __init__(self, proposal):
        self.proposal = proposal
        self.count = 0
        self.started = False
        self.resumed = False
        self.calls = []
        self.completed = set()
        self.turns = self.usages = 0
        self.pending = None

    def ingest(self, line, phase):
        row = project_native(line)
        typ, data = row.get("type"), row.get("data", {})
        require(isinstance(typ, str), "Missing event type")
        forbidden = {"agentId", "parentToolCallId", "detachedFromSpawningParentSessionId"}
        require(not forbidden.intersection(row["fields"]) and not forbidden.intersection(data.get("fields", [])),
                "Unexpected child or detached event")
        require(not typ.startswith(("subagent.", "permission.")), "Unexpected child or permission event")
        if typ == "session.start":
            require(phase == 1 and not self.started and data.get("sessionId") == self.proposal["nativeSessionId"]
                    and data.get("copilotVersion") == "1.0.90" and not data.get("alreadyInUse"), "Native start identity unverified")
            self.started = True
        elif typ == "session.resume":
            require(phase == 2 and self.started and not self.resumed and self.pending is None
                    and type(data.get("eventCount")) is int and data["eventCount"] == self.count
                    and not any(data.get(k) for k in ("alreadyInUse", "sessionWasActive", "continuePendingWork")),
                    "Native resume identity unverified")
            self.resumed = True
        elif typ == "assistant.turn_start":
            self.turns += 1
            require(self.turns <= self.proposal["limits"]["maxTurnEvents"], "Observed turn cap")
        elif typ == "assistant.usage":
            self.usages += 1
            require(self.usages <= self.proposal["limits"]["maxUsageEvents"], "Observed usage cap")
        elif typ == "tool.execution_start":
            require(self.started and (phase == 1 or self.resumed) and self.pending is None, "Unbound or concurrent tool")
            require(data.get("model") == self.proposal["model"], "Native model unverified")
            server = data.get("mcpConfigServerName", data.get("mcpServerName"))
            require(server == "ork742" and data.get("mcpToolName") in {t["name"] for t in self.proposal["tools"]}
                    and data.get("mcpTransport") in ("stdio", "local"), "Unexpected native tool source")
            args = data.get("arguments", {})
            require(set(args.get("fields", [])) <= {"cursor", "resultNonce", "bootstrap", "snapshotId", "actionId"}, "Unexpected tool argument")
            args = {k: v for k, v in args.items() if k != "fields"}
            call_id, turn = data.get("toolCallId"), data.get("turnId")
            require(isinstance(call_id, str) and call_id and isinstance(turn, str) and turn
                    and all(c["id"] != call_id for c in self.calls), "Missing or duplicate call identity")
            self.calls.append({"phase": phase, "id": call_id, "turn": turn, "tool": data["mcpToolName"],
                               "argumentsHash": sha(json.dumps(args, sort_keys=True, separators=(",", ":")).encode()),
                               "arguments": args})
            require(len(self.calls) <= self.proposal["limits"]["maxToolCalls"], "Observed tool cap")
            self.pending = self.calls[-1]
        elif typ == "tool.execution_complete":
            require(self.pending and self.pending["phase"] == phase and data.get("toolCallId") == self.pending["id"]
                    and data.get("turnId") == self.pending["turn"] and data.get("success") is True,
                    "Tool completion correlation failed")
            self.completed.add(self.pending["id"])
            self.pending = None
        require(data.get("model", self.proposal["model"]) == self.proposal["model"]
                and data.get("selectedModel", self.proposal["model"]) == self.proposal["model"], "Model drift")
        self.count += 1


def join_receipts(trace, logs, final=True):
    from mcp_server import digest, seeded_peer
    require(trace.started and trace.pending is None and len(trace.completed) == len(trace.calls), "Incomplete native calls")
    phases = (1, 2) if final else (1,)
    require(not final or trace.resumed, "Missing native resume")
    states = {}
    for phase in phases:
        rows = logs[phase]
        require(not any(r["kind"] == "fatal" for r in rows), "Fixture rejected request")
        calls = [c for c in trace.calls if c["phase"] == phase]
        helper = [r for r in rows if r["kind"] == "call"]
        require(len(calls) == len(helper) == (21 if phase == 1 else 2), "Incomplete fixture sequence")
        require([(c["tool"], c["argumentsHash"]) for c in calls] ==
                [(r["tool"], r["argumentsHash"]) for r in helper], "Native/helper call join failed")
        state_rows = [r for r in rows if r["kind"] == "state"]
        ack = [r for r in rows if r["kind"] == "ack"]
        require(ack and ack[-1]["done"] is True and len(state_rows) == (8 if phase == 1 else 1), "Missing final snapshot")
        states[phase] = state_rows[-1]
        if phase == 1:
            delivered = [r for r in rows if r["kind"] == "delivered"]
            require([r["stage"] for r in delivered] == list(range(7))
                    and [r["status"] for r in delivered] == ["events"] * 5 + ["refresh_required", "events"]
                    and [r["effects"] for r in ack] == [1, 2, 3, 4, 4, 5], "Duplicate/gap effects unverified")
            reads = [c for c in calls if c["tool"] == "read_run_state"][1:]
            require([digest(c["arguments"].get("resultNonce")) for c in reads] ==
                    [r["nonceHash"] for r in delivered], "Result delivery nonce join failed")
            waits = [c for c in calls if c["tool"] == "wait_for_run_event"]
            require(waits[-1]["arguments"] == {"cursor": state_rows[-2]["cursor"]}, "Gap cursor not adopted")
    if final:
        p = seeded_peer(True)
        s = p.read(p.binding)
        expected = {"version": s.version, "cursor": s.cursor.value, "state": s.state,
                    "stageContinueRequired": s.stage_continue, "blockers": list(s.blockers),
                    "clarification": s.clarification, "grant": s.grant, "reserved": list(s.reserved)}
        require(states[1]["stateHash"] == states[2]["stateHash"] == digest(expected)
                and states[1]["snapshotHash"] != states[2]["snapshotHash"], "Retained state or fresh handle unverified")
    return {"nativeToolCalls": len(trace.calls), "allCallsCorrelated": True,
            "duplicateIdempotent": True, "gapCursorAdopted": True, "retainedStateMatches": final}


def supervise(argv, cwd, env, root, seconds, file_cap, total_cap, tick, on_start=None, receipt=None):
    """Run one owned process group. Tests pass only Python fake-client argv."""
    import os
    from pathlib import Path
    import signal
    import subprocess
    import time
    if receipt is None:
        receipt = {}
    receipt.update(started=False, exitCode=None, terminationStepsCompleted=False)
    require(seconds > 0, "Owned process deadline already expired")
    started = time.monotonic()
    root = Path(root)
    proc = None
    def bounds():
        total = 0
        for path in root.rglob("*"):
            require(not path.is_symlink(), "Unexpected raw symlink")
            if path.is_file():
                size = path.stat().st_size
                require(size <= file_cap, "Raw file cap")
                total += size
        require(total <= total_cap, "Raw total cap")
    try:
        with (root/"stdout.raw").open("wb") as out, (root/"stderr.raw").open("wb") as err:
            proc = subprocess.Popen(argv, cwd=cwd, env=env, stdin=subprocess.DEVNULL,
                                    stdout=out, stderr=err, start_new_session=True)
            receipt["started"] = True
            if on_start: on_start(proc.pid)
            while proc.poll() is None:
                require(time.monotonic()-started < seconds, "Owned process deadline")
                bounds(); tick(); time.sleep(.05)
            bounds(); tick()
            require(time.monotonic()-started < seconds, "Owned process deadline")
            require(proc.returncode == 0, "Owned process failed")
            return proc.returncode
    finally:
        if proc:
            try:
                # A clean foreground exit can leave a child in the owned group.
                try: os.killpg(proc.pid, signal.SIGTERM)
                except ProcessLookupError: pass
                try: proc.wait(timeout=2)
                except subprocess.TimeoutExpired:
                    pass
                try: os.killpg(proc.pid, signal.SIGKILL)
                except ProcessLookupError: pass
                proc.wait(timeout=2)
                receipt["terminationStepsCompleted"] = True
            finally:
                # Preserve an observed exit even when termination itself raises.
                receipt["exitCode"] = proc.poll()



def read_bounded(path, cap=1048576):
    import os
    from pathlib import Path
    import stat
    path = Path(path)
    require(not any(p.is_symlink() for p in [path, *path.parents]), "Unexpected source symlink")
    fd = os.open(path, os.O_RDONLY | getattr(os, "O_NOFOLLOW", 0))
    try:
        require(stat.S_ISREG(os.fstat(fd).st_mode), "Regular file required")
        data = os.read(fd, cap+1)
        require(len(data) <= cap, "Source size cap")
        return data
    finally:
        os.close(fd)


def source_paths(directory):
    from pathlib import Path
    directory = Path(directory)
    peer = directory/"peer.py"
    if not peer.exists(): peer = directory.parent/"parent-continuation/peer.py"
    return {**{name: directory/name for name in ("observer.py", "mcp_server.py", "proposal.json", "source-contracts.json")},
            "peer.py": peer}


def verify_manifest(directory, manifest):
    paths = source_paths(directory)
    require(set(manifest) == {"schema", "files"} and manifest["schema"] == "orkworks.native-packet-manifest.v1"
            and set(manifest["files"]) == set(paths), "Unexpected manifest shape")
    for name, path in paths.items():
        require(sha(read_bounded(path)) == manifest["files"][name], "Packet source drift")


def check_pins(pins):
    for path, expected in pins.items():
        require(sha(read_bounded(path)) == expected, "Prepared input drift")


def prepare_case(case, proposal, directory):
    """Write only owned synthetic inputs; no discovery or native process."""
    from pathlib import Path
    case = Path(case)
    require(not case.exists(), "Fresh owned case required")
    case.mkdir(mode=0o700)
    for name in ("repo", "settings", "logs", "control", "mcp-home", "tmp", "packet", "empty-template"):
        (case/name).mkdir(mode=0o700)
    for name, path in source_paths(directory).items():
        (case/"packet"/name).write_bytes(read_bounded(path))
    (case/"repo/AGENTS.md").write_text(proposal["rootRule"], encoding="utf-8")
    (case/"settings/settings.json").write_text(json.dumps(proposal["settings"], sort_keys=True), encoding="utf-8")
    server = {"type": "stdio", "command": "/usr/bin/env",
              "args": ["-i", "PATH=/usr/bin:/bin", "HOME="+str(case/"mcp-home"), "TMPDIR="+str(case/"tmp"),
                       "LC_ALL=C", "__CF_USER_TEXT_ENCODING=0x0:0:0", proposal["python"]["path"], "-I", "-B", str(case/"packet/mcp_server.py"),
                       "--root", str(case/"control")], "cwd": str(case/"repo"),
              "tools": [t["name"] for t in proposal["tools"]], "timeout": 35000,
              "deferTools": "never", "disableToolCache": True}
    (case/"mcp.json").write_text(json.dumps({"mcpServers": {"ork742": server}}, sort_keys=True), encoding="utf-8")
    files = [case/"repo/AGENTS.md", case/"settings/settings.json", case/"mcp.json",
             *[case/"packet"/name for name in source_paths(directory)]]
    return {path: sha(read_bounded(path)) for path in files}


class History:
    """Append-only private history scanner. Only selected Trace metadata survives."""
    def __init__(self, case, trace):
        self.case, self.trace = case, trace
        self.offset, self.prefix = 0, sha(b"")

    def scan(self, phase, final=False):
        code = "history-root-missing"
        try:
            root = self.case/"settings/session-state"
            if not root.exists():
                require(not final, "Missing native history")
                return
            code = "history-root-layout"
            require(not root.is_symlink(), "Unexpected history link")
            entries = list(root.iterdir())
            expected = root/self.trace.proposal["nativeSessionId"]
            code = "history-session-layout"
            require(entries == [expected] and not expected.is_symlink(), "Unexpected native session")
            path = expected/"events.jsonl"
            code = "history-events-missing"
            if not path.exists():
                require(not final, "Missing native history")
                return
            code = "history-events-read"
            raw = read_bounded(path)
            code = "history-prefix"
            require(len(raw) >= self.offset and sha(raw[:self.offset]) == self.prefix, "Native history prefix changed")
            tail = raw[self.offset:]
            code = "history-tail"
            require(not final or not tail or tail.endswith(b"\n"), "Incomplete history tail")
            for line in tail.splitlines(keepends=True):
                if not line.endswith(b"\n"): break
                code = "history-event-cap"
                require(self.trace.count < 4096, "Native event cap")
                code = "history-event-contract"
                self.trace.ingest(line.decode("utf-8"), phase)
                self.offset += len(line)
            self.prefix = sha(raw[:self.offset])
        except (ValueError, OSError) as error:
            error.history_check_code = code
            raise


def check_native_sources(proposal):
    from pathlib import Path
    native = proposal["native"]
    files = [(Path(native["nativePath"]), native["nativeSha256"]),
             (Path(native["cachePath"])/"app.js", native["appSha256"]),
             (Path(native["cachePath"])/"prebuilds/darwin-arm64/runtime.node", native["runtimeSha256"]),
             (Path(proposal["python"]["path"]), proposal["python"]["sha256"])]
    for path, expected in files:
        # These fixed installed binaries can be large. Never open auth/configuration files.
        require(sha(path.read_bytes()) == expected, "Installed native/Python source drift")


def discovery(proposal, case, env, deadline, record_attempt, diagnostic=None):
    from pathlib import Path
    import time
    receipts = []
    if diagnostic is None:
        diagnostic = {}
    for template in proposal["sourceDiscovery"]:
        diagnostic["stage"] = "instruction-discovery" if "instruction" in template else "skill-discovery"
        diagnostic["process"] = {}
        require(time.monotonic() < deadline, "Batch deadline")
        argv = [x.replace("<native>", proposal["native"]["nativePath"]) for x in template]
        record_attempt()
        supervise(argv, case/"repo", env, case, min(30, deadline-time.monotonic()), 1048576, 8388608, lambda: None,
                  receipt=diagnostic["process"])
        raw = read_bounded(case/"stdout.raw", 131072)
        require(not read_bounded(case/"stderr.raw"), "Source discovery stderr")
        rows = json.loads(raw)
        require(type(rows) is list, "Source discovery shape")
        if "instruction" in template:
            require(len(rows) == 1 and rows[0].get("sourcePath") == "AGENTS.md"
                    and rows[0].get("location") == "repository" and not rows[0].get("defaultDisabled"),
                    "Unexpected instruction source")
        else:
            require(len(rows) == 2 and {r["name"] for r in rows} == {"customize-cloud-agent", "github-pr-media"},
                    "Unexpected skill source")
            require(all(r.get("source") == "builtin" and Path(r["path"]).resolve() ==
                        (Path(proposal["native"]["cachePath"])/"builtin"/r["name"]).resolve() for r in rows),
                    "Unexpected builtin skill source")
        receipts.append(sha(raw))
    return receipts


def run_native(approval, output):
    """Separate live entry point. It is never called by model-free preparation."""
    import os
    from pathlib import Path
    import platform
    import secrets
    import shutil
    import subprocess
    import tempfile
    import time
    require(type(approval) is dict, "Exact packet approval required")
    directory = Path(__file__).resolve().parent
    pb, mb = read_bounded(directory/"proposal.json"), read_bounded(directory/"manifest.json")
    require_authorization(approval, pb, mb)  # Before any subprocess, including discovery/git.
    proposal, manifest = json.loads(pb), json.loads(mb)
    verify_manifest(directory, manifest)
    validate_native_id(proposal["nativeSessionId"])
    require(platform.system() == "Darwin" and platform.machine() == "arm64", "Pinned platform required")
    output = Path(output)
    require(not output.exists(), "Fresh retained output required")
    output.mkdir(mode=0o700)
    os.umask(0o077)
    batch = Path(tempfile.mkdtemp(prefix="ork742-native-")).resolve()
    summary = {"schema": "orkworks.native-packet-summary.v1", "proposalSha256": sha(pb), "manifestSha256": sha(mb),
               "outcome": "unverified", "nativePromptInvocations": 0, "discoveryCommands": 0,
               "providerRequestCount": "unknown", "remoteRetention": "unknown", "rawRootsRemoved": False}
    started = time.monotonic()
    diagnostic = {"phase": 0, "stage": "prepare", "process": {}}
    try:
        case = batch/"case"
        pins = prepare_case(case, proposal, directory)
        inherited = {k: os.environ[k] for k in proposal["environment"]["inheritedNames"] if k in os.environ}
        require("HOME" in inherited and "PATH" in inherited, "Native authentication environment unavailable")
        env = {**inherited, **{k: v.replace("<case>", str(case)) for k, v in proposal["environment"]["overrides"].items()}}
        summary["nativeEnvironmentNames"] = sorted(env)
        # Empty template, no global/system Git configuration; this is an owned empty repo.
        diagnostic["stage"] = "git-init"
        subprocess.run(["/usr/bin/git", "init", "-q", "--template="+str(case/"empty-template"), str(case/"repo")],
                       env={"PATH": "/usr/bin:/bin", "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1"},
                       stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, check=True, timeout=10)
        trace = Trace(proposal)
        history = History(case, trace)
        logs, bindings = {}, []
        deadline = started + proposal["limits"]["batchSeconds"]
        for phase, key in ((1, "new"), (2, "resume")):
            diagnostic.update(phase=phase, stage="source-check", process={})
            check_native_sources(proposal)
            verify_manifest(directory, manifest)
            diagnostic["stage"] = "input-check"
            check_pins(pins)
            require(not (case/"settings/config.json").exists(), "Unexpected native configuration")
            def record_discovery_attempt():
                summary["discoveryCommands"] += 1
            discovery(proposal, case, env, deadline, record_discovery_attempt, diagnostic=diagnostic)
            diagnostic["stage"] = "input-check"
            check_pins(pins)
            generation = secrets.token_hex(32)
            def bind(pid):
                diagnostic["stage"] = "runtime-bind"
                context = {"phase": phase, "generation": generation, "nativePid": pid}
                tmp = case/"control/context.tmp"
                tmp.write_text(json.dumps(context))
                tmp.replace(case/"control/context.json")
                bindings.append(context)
                diagnostic["stage"] = "native-prompt"
            def observe_history():
                diagnostic["stage"] = "native-history"
                history.scan(phase)
                diagnostic["stage"] = "native-prompt"
            argv = [x.replace("<native>", proposal["native"]["nativePath"]).replace("<case>", str(case))
                    for x in proposal["argv"][key]]
            summary["nativePromptInvocations"] += 1
            diagnostic.update(stage="native-prompt", process={})
            supervise(argv, case/"repo", env, case, min(proposal["limits"]["nativeSeconds"], deadline-time.monotonic()),
                      proposal["limits"]["maxRawFileBytes"], proposal["limits"]["maxRawTotalBytes"],
                      observe_history, bind, receipt=diagnostic["process"])
            diagnostic["stage"] = "native-history"
            history.scan(phase, final=True)
            diagnostic["stage"] = "input-check"
            check_pins(pins)
            diagnostic["stage"] = "helper-receipt"
            rows = [json.loads(line) for line in read_bounded(case/f"control/mcp-{phase}.jsonl", 131072).splitlines()]
            require(rows and sum(r["kind"] == "startup" for r in rows) == 1
                    and rows[0]["kind"] == "startup" and rows[0]["environmentNames"] == sorted(proposal["mcpEnvironmentNames"])
                    and all(r.get("phase") == phase and r.get("generation") == generation
                            and r.get("nativePid") == bindings[-1]["nativePid"] for r in rows), "Helper runtime binding failed")
            logs[phase] = rows
            diagnostic["stage"] = "receipt-join"
            summary["checks"] = join_receipts(trace, logs, final=phase == 2)
        diagnostic["stage"] = "runtime-check"
        require(bindings[0]["nativePid"] != bindings[1]["nativePid"]
                and bindings[0]["generation"] != bindings[1]["generation"], "Fresh runtime not observed")
        summary.update(outcome="functional-canary-observed", originalIdentityJoined=True, historyPrefixUnchanged=True,
                       freshRuntimeObserved=True, observedTurns=trace.turns, observedUsageEvents=trace.usages)
    except Exception as error:
        # Do not reflect native arguments, paths, payloads or exception text.
        summary["outcome"] = "unverified"
        summary["stopReason"] = "Packet check failed; no retry or fallback"
        summary["failure"] = failure_details(diagnostic, error)
    finally:
        try:
            subprocess.run([proposal["python"]["path"], "-I", "-B", "-c",
                            "import shutil,sys; shutil.rmtree(sys.argv[1])", str(batch)],
                           env={}, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                           stderr=subprocess.DEVNULL, check=True, timeout=proposal["limits"]["cleanupSeconds"])
            summary["rawRootsRemoved"] = not batch.exists()
        except (OSError, subprocess.SubprocessError) as error:
            summary["cleanupFailure"] = failure_details(
                {"phase": diagnostic["phase"], "stage": "raw-cleanup"}, error)
        if not summary["rawRootsRemoved"]: summary["outcome"] = "incomplete-cleanup"
        summary["elapsedSeconds"] = round(time.monotonic()-started, 3)
        data = (json.dumps(summary, sort_keys=True, indent=2)+"\n").encode()
        require(len(data) <= proposal["limits"]["maxRetainedBytes"], "Retained evidence cap")
        (output/"summary.json").write_bytes(data)
    return summary


if __name__ == "__main__":
    import argparse
    from pathlib import Path
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--authorization", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    try:
        result = run_native(json.loads(read_bounded(args.authorization, 4096)), args.output)
        print(json.dumps({k: result[k] for k in ("outcome", "nativePromptInvocations", "rawRootsRemoved")}))
        raise SystemExit(0 if result["outcome"] == "functional-canary-observed" else 2)
    except (ValueError, OSError, json.JSONDecodeError):
        print("Packet not authorized or preflight failed")
        raise SystemExit(2)
