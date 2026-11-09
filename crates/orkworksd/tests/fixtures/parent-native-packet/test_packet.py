"""All clients/processes in this suite are synthetic; no Copilot executable is invoked."""
from __future__ import annotations
import copy
import json
import os
from pathlib import Path
import sys
import tempfile
import time
import unittest
from unittest.mock import patch
from mcp_server import Protocol, digest
import observer

PROPOSAL = json.loads(Path(__file__).with_name("proposal.json").read_text())


def ready(phase=1, rows=None):
    p = Protocol(phase=phase, record=(rows.append if rows is not None else None))
    p.handle({"jsonrpc": "2.0", "id": 0, "method": "initialize",
              "params": {"protocolVersion": "2025-06-18"}})
    p.handle({"jsonrpc": "2.0", "method": "notifications/initialized"})
    return p


def call(p, name, arguments):
    r = p.handle({"jsonrpc": "2.0", "id": len(p.ids) + 1, "method": "tools/call",
                  "params": {"name": name, "arguments": arguments}})
    if "error" in r:
        raise ValueError("fixture rejected test request")
    return r["result"]["structuredContent"]


def sequence(phase=1):
    rows, calls = [], []
    p = ready(phase, rows)
    def invoke(name, args):
        calls.append((name, copy.deepcopy(args)))
        return call(p, name, args)
    state = invoke("read_run_state", {"bootstrap": True})
    if phase == 1:
        for stage in range(7):
            hint = invoke("wait_for_run_event", {"cursor": state["cursor"]})
            state = invoke("read_run_state", {"resultNonce": hint["resultNonce"]})
            if hint["status"] != "refresh_required":
                invoke("acknowledge", {"snapshotId": state["snapshotId"], "actionId": state["nextAction"]})
    else:
        invoke("acknowledge", {"snapshotId": state["snapshotId"], "actionId": state["nextAction"]})
    return p, rows, calls


def event(kind, **data):
    return json.dumps({"type": kind, "data": data})


def native_trace(phase, calls, trace=None):
    trace = trace or observer.Trace(PROPOSAL)
    if phase == 1:
        trace.ingest(event("session.start", sessionId=PROPOSAL["nativeSessionId"], copilotVersion="1.0.90"), phase)
    else:
        trace.ingest(event("session.resume", eventCount=trace.count), phase)
    for n, (name, args) in enumerate(calls):
        trace.ingest(event("tool.execution_start", toolCallId=f"c{phase}-{n}", turnId=f"t{phase}-{n}",
                           toolName="native-alias", model="gpt-6-luna", mcpConfigServerName="ork742",
                           mcpToolName=name, mcpTransport="stdio", arguments=args), phase)
        trace.ingest(event("tool.execution_complete", toolCallId=f"c{phase}-{n}", turnId=f"t{phase}-{n}",
                           success=True, result={"content": "PRIVATE_OUTPUT"}), phase)
    return trace


class PacketTests(unittest.TestCase):
    def test_missing_or_changed_approval_cannot_authorize_any_native_launch(self):
        expected = {"decision": "approve-one-batch", "proposalSha256": observer.sha(b"p"),
                    "manifestSha256": observer.sha(b"m"), "maxNativePrompts": 2}
        self.assertTrue(observer.require_authorization(expected, b"p", b"m"))
        for value in (None, {}, {**expected, "maxNativePrompts": 3}, {**expected, "extra": True}):
            with self.assertRaises(ValueError):
                observer.require_authorization(value, b"p", b"m")
        with self.assertRaises(ValueError):
            observer.require_authorization(expected, b"changed", b"m")

    def test_tools_require_initialization(self):
        response = Protocol().handle({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})
        self.assertEqual(response.get("error", {}).get("code"), -32002)

    def test_complete_sequence_has_duplicate_gap_and_retained_resume(self):
        p, rows, calls = sequence()
        resumed, other, resume_calls = sequence(2)
        self.assertTrue(p.done and resumed.done)
        self.assertEqual(len(calls) + len(resume_calls), 23)
        acks = [r for r in rows if r["kind"] == "ack"]
        self.assertEqual([r["effects"] for r in acks], [1, 2, 3, 4, 4, 5])
        self.assertEqual([r["status"] for r in rows if r["kind"] == "delivered"],
                         ["events"] * 5 + ["refresh_required", "events"])
        last = [r for r in rows if r["kind"] == "state"][-1]
        first = [r for r in other if r["kind"] == "state"][0]
        self.assertEqual(last["stateHash"], first["stateHash"])
        self.assertNotEqual(last["snapshotHash"], first["snapshotHash"])
        trace = native_trace(2, resume_calls, native_trace(1, calls))
        self.assertTrue(observer.join_receipts(trace, {1: rows, 2: other})["retainedStateMatches"])

    def test_wrong_nonce_handle_cursor_and_phase_are_rejected(self):
        for name, args in (("read_run_state", {"resultNonce": "0" * 64}),
                           ("acknowledge", {"snapshotId": "0" * 64, "actionId": "event-4"}),
                           ("wait_for_run_event", {"cursor": -1}), ("shell", {})):
            p = ready()
            with self.assertRaises(ValueError):
                call(p, name, args)
        p = ready(2)
        call(p, "read_run_state", {"bootstrap": True})
        with self.assertRaises(ValueError):
            call(p, "wait_for_run_event", {"cursor": p.peer.cursor})

    def test_closed_lifecycle_and_duplicate_ids(self):
        p = ready()
        r = p.handle({"jsonrpc": "2.0", "id": 3, "method": "tools/list"})
        self.assertEqual([t["name"] for t in r["result"]["tools"]], [t["name"] for t in PROPOSAL["tools"]])
        for message in ([], {"jsonrpc": "2.0", "id": 3, "method": "ping"},
                        {"jsonrpc": "2.0", "id": 4, "method": "resources/list"}):
            self.assertIn("error", p.handle(message))

    def test_projection_never_decodes_private_values(self):
        line = event("tool.execution_complete", toolCallId="a", success=True,
                     result={"content": "PRIVATE_OUTPUT"}, reasoning="PRIVATE_REASONING",
                     content="PRIVATE_ASSISTANT")
        loads = json.loads
        def checked(value, *args, **kwargs):
            self.assertNotIn("PRIVATE_", value)
            return loads(value, *args, **kwargs)
        with patch.object(observer.json, "loads", side_effect=checked):
            row = observer.project_native(line)
        self.assertEqual(row["data"]["toolCallId"], "a")
        self.assertNotIn("result", row["data"])
        for bad in ('{"type":"x","type":"y"}', '{"type":"x",}', '[1]', '{"type":"x"}junk'):
            with self.assertRaises(ValueError):
                observer.project_native(bad)

    def test_native_identity_tool_and_resume_denials(self):
        for data in ({"sessionId": "wrong", "copilotVersion": "1.0.90"},
                     {"sessionId": PROPOSAL["nativeSessionId"], "copilotVersion": "other"}):
            with self.assertRaises(ValueError):
                observer.Trace(PROPOSAL).ingest(event("session.start", **data), 1)
        _, _, calls = sequence()
        t = native_trace(1, calls)
        for kind, data in (("session.start", {"sessionId": PROPOSAL["nativeSessionId"]}),
                           ("session.resume", {"eventCount": 0}),
                           ("session.resume", {"eventCount": t.count + 1}),
                           ("session.resume", {"eventCount": t.count, "continuePendingWork": True})):
            with self.assertRaises(ValueError):
                copy.deepcopy(t).ingest(event(kind, **data), 2)
        with self.assertRaises(ValueError):
            observer.Trace(PROPOSAL).ingest(event("permission.requested"), 1)
        for change in ({"mcpToolName": "shell"}, {"model": "other"}, {"agentId": "child"},
                       {"arguments": {"unknown": "PRIVATE_UNEXPECTED"}}, {"mcpTransport": "http"}):
            t = observer.Trace(PROPOSAL)
            t.ingest(event("session.start", sessionId=PROPOSAL["nativeSessionId"], copilotVersion="1.0.90"), 1)
            data = dict(toolCallId="a", turnId="t", toolName="alias", model="gpt-6-luna",
                        mcpConfigServerName="ork742", mcpToolName="read_run_state", mcpTransport="stdio",
                        arguments={"bootstrap": True})
            data.update(change)
            with self.assertRaises(ValueError):
                t.ingest(event("tool.execution_start", **data), 1)

    def test_join_rejects_missing_completion_and_changed_arguments(self):
        _, rows, calls = sequence()
        _, other, more = sequence(2)
        t = native_trace(2, more, native_trace(1, calls))
        broken = copy.deepcopy(t)
        broken.completed.pop()
        with self.assertRaises(ValueError):
            observer.join_receipts(broken, {1: rows, 2: other})
        broken = copy.deepcopy(rows)
        [r for r in broken if r["kind"] == "call"][0]["argumentsHash"] = "0" * 64
        with self.assertRaises(ValueError):
            observer.join_receipts(t, {1: broken, 2: other})

    def test_template_and_manifest_drift_stop_before_launch(self):
        directory = Path(__file__).parent
        with tempfile.TemporaryDirectory() as root:
            case = Path(root).resolve()/"case"
            pins = observer.prepare_case(case, PROPOSAL, directory)
            self.assertEqual(json.loads((case/"settings/settings.json").read_text()), PROPOSAL["settings"])
            config = json.loads((case/"mcp.json").read_text())["mcpServers"]["ork742"]
            self.assertEqual(config["command"], "/usr/bin/env")
            self.assertEqual(config["args"][0], "-i")
            observer.check_pins(pins)
            (case/"repo/AGENTS.md").write_text("changed")
            with self.assertRaises(ValueError):
                observer.check_pins(pins)
        with patch.object(observer, "supervise") as spawn:
            with self.assertRaises(ValueError):
                observer.run_native(None, Path("unused"))
            spawn.assert_not_called()

    def test_history_rejects_changed_prefix_extra_session_and_partial_tail(self):
        with tempfile.TemporaryDirectory() as root:
            case = Path(root).resolve()
            folder = case/"settings/session-state"/PROPOSAL["nativeSessionId"]
            folder.mkdir(parents=True)
            history = folder/"events.jsonl"
            start = event("session.start", sessionId=PROPOSAL["nativeSessionId"], copilotVersion="1.0.90") + "\n"
            history.write_text(start)
            trace = observer.Trace(PROPOSAL)
            reader = observer.History(case, trace)
            reader.scan(1, final=True)
            history.write_text(start + event("session.resume", eventCount=1) + "\n")
            reader.scan(2, final=True)
            history.write_text(start.replace("1.0.90", "1.0.91"))
            with self.assertRaises(ValueError):
                reader.scan(2, final=True)
        with tempfile.TemporaryDirectory() as root:
            case = Path(root).resolve()
            folder = case/"settings/session-state"/"another-session"
            folder.mkdir(parents=True)
            with self.assertRaises(ValueError):
                observer.History(case, observer.Trace(PROPOSAL)).scan(1)

    def test_observed_turn_and_usage_caps(self):
        for kind, limit in (("assistant.turn_start", "maxTurnEvents"), ("assistant.usage", "maxUsageEvents")):
            t = observer.Trace(PROPOSAL)
            for _ in range(PROPOSAL["limits"][limit]):
                t.ingest(event(kind), 1)
            with self.assertRaises(ValueError):
                t.ingest(event(kind), 1)

    @unittest.skipUnless(os.name == "posix", "Owned process-group canary targets POSIX only")
    def test_expired_and_just_exceeded_deadlines_cannot_launch_or_succeed(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root).resolve()
            with patch("subprocess.Popen") as launch, patch("os.killpg", create=True):
                with self.assertRaises(ValueError):
                    observer.supervise(["never"], root, {}, root, 0, 1024, 8192, lambda: None)
                launch.assert_not_called()
            from unittest.mock import Mock
            fake = Mock(pid=123456, returncode=0)
            fake.poll.return_value = 0
            with patch("subprocess.Popen", return_value=fake), patch("os.killpg", create=True), \
                    patch("time.monotonic", side_effect=[0, 2]):
                with self.assertRaises(ValueError):
                    observer.supervise(["never"], root, {}, root, 1, 1024, 8192, lambda: None)

    def test_failed_second_discovery_is_counted(self):
        with tempfile.TemporaryDirectory() as root:
            case = Path(root).resolve()
            attempts = []
            def fake(*args, **kwargs):
                if len(attempts) == 2:
                    raise ValueError("synthetic second discovery failure")
                (case/"stdout.raw").write_text(json.dumps([{"sourcePath": "AGENTS.md", "location": "repository"}]))
                (case/"stderr.raw").write_bytes(b"")
            with patch.object(observer, "supervise", side_effect=fake):
                with self.assertRaises(ValueError):
                    observer.discovery(PROPOSAL, case, {}, time.monotonic()+30, lambda: attempts.append(True))
            self.assertEqual(len(attempts), 2)

    def test_frames_reject_duplicates_oversize_and_invalid_json(self):
        from mcp_server import parse_frame
        for line in (b'{"id":1,"id":2}', b'[]', b'{' + b' ' * 16384, b'{"x":NaN}', b'\xff'):
            with self.assertRaises(ValueError):
                parse_frame(line)

    def test_manifest_is_closed_and_pinned(self):
        directory = Path(__file__).parent
        manifest = json.loads((directory/"manifest.json").read_text())
        observer.verify_manifest(directory, manifest)
        manifest["files"]["observer.py"] = "0" * 64
        with self.assertRaises(ValueError):
            observer.verify_manifest(directory, manifest)

    @unittest.skipUnless(os.name == "posix", "Owned process-group canary targets POSIX only")
    def test_real_stdio_fixture_with_owned_fake_client_and_fresh_resume(self):
        # Only this test's Python client is launched; it starts the owned fixture,
        # never a coding harness or a network/authentication process.
        client = r"""
import json,os,subprocess,sys
from pathlib import Path
case=Path(sys.argv[1]); phase=int(sys.argv[2]); proposal=json.loads((case/'packet/proposal.json').read_text())
folder=case/'settings/session-state'/proposal['nativeSessionId']; folder.mkdir(parents=True,exist_ok=True)
history=folder/'events.jsonl'
f=history.open('a')
def event(kind,**data):
    f.write(json.dumps({'type':kind,'data':data})+'\n'); f.flush()
if phase==1: event('session.start',sessionId=proposal['nativeSessionId'],copilotVersion='1.0.90')
else: event('session.resume',eventCount=len(history.read_text().splitlines()))
config=json.loads((case/'mcp.json').read_text())['mcpServers']['ork742']
p=subprocess.Popen([config['command'],*config['args']],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,env={})
i=0
def request(method,params):
    global i
    i+=1; p.stdin.write((json.dumps({'jsonrpc':'2.0','id':i,'method':method,'params':params})+'\n').encode());p.stdin.flush()
    value=json.loads(p.stdout.readline()); assert 'error' not in value,value
    return value['result']
request('initialize',{'protocolVersion':'2025-06-18'})
p.stdin.write(b'{"jsonrpc":"2.0","method":"notifications/initialized"}\n');p.stdin.flush()
assert len(request('tools/list',{})['tools'])==3
n=0
def tool(name,args):
    global n
    n+=1; cid=f'c{phase}-{n}';turn=f't{phase}-{n}'
    event('tool.execution_start',toolCallId=cid,turnId=turn,toolName='alias',model='gpt-6-luna',mcpConfigServerName='ork742',mcpToolName=name,mcpTransport='stdio',arguments=args)
    result=request('tools/call',{'name':name,'arguments':args})['structuredContent']
    event('tool.execution_complete',toolCallId=cid,turnId=turn,success=True,result={'content':'PRIVATE_OUTPUT'})
    return result
state=tool('read_run_state',{'bootstrap':True})
if phase==1:
    for _ in range(7):
        hint=tool('wait_for_run_event',{'cursor':state['cursor']})
        state=tool('read_run_state',{'resultNonce':hint['resultNonce']})
        if hint['status']!='refresh_required': receipt=tool('acknowledge',{'snapshotId':state['snapshotId'],'actionId':state['nextAction']})
else: receipt=tool('acknowledge',{'snapshotId':state['snapshotId'],'actionId':state['nextAction']})
assert receipt['done']
p.stdin.close(); assert p.wait(timeout=2)==0
assert not p.stderr.read();f.close()
"""
        with tempfile.TemporaryDirectory() as root:
            case = Path(root).resolve()/"case"
            fake_proposal = copy.deepcopy(PROPOSAL)
            fake_proposal["python"]["path"] = str(Path(sys.executable).resolve())
            observer.prepare_case(case, fake_proposal, Path(__file__).parent)
            trace = observer.Trace(PROPOSAL)
            reader = observer.History(case, trace)
            logs, pids = {}, []
            for phase in (1, 2):
                def bind(pid):
                    pids.append(pid)
                    (case/"control/context.json").write_text(json.dumps({"phase": phase, "generation": str(phase)*64, "nativePid": pid}))
                try:
                    observer.supervise([sys.executable, "-I", "-B", "-c", client, str(case), str(phase)],
                                        case/"repo", {}, case, 15, 1048576, 8388608,
                                        lambda: reader.scan(phase), bind)
                except ValueError:
                    self.fail((case/"stderr.raw").read_text())
                reader.scan(phase, final=True)
                logs[phase] = [json.loads(line) for line in (case/f"control/mcp-{phase}.jsonl").read_text().splitlines()]
                self.assertEqual(logs[phase][0]["nativePid"], pids[-1])
                self.assertEqual(logs[phase][0]["environmentNames"], sorted(PROPOSAL["mcpEnvironmentNames"]))
            self.assertNotEqual(pids[0], pids[1])
            self.assertTrue(observer.join_receipts(trace, logs)["allCallsCorrelated"])

    @unittest.skipUnless(os.name == "posix", "Owned process-group canary targets POSIX only")
    def test_owned_fake_process_timeout_output_cap_and_clean_exit(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root).resolve()
            for code, seconds, cap in (("import time; time.sleep(20)", .15, 1024),
                                       ("print('x'*4096)", 2, 100)):
                with self.assertRaises(ValueError):
                    observer.supervise([sys.executable, "-I", "-B", "-c", code], root, {}, root,
                                        seconds, cap, 8192, lambda: None)
            self.assertEqual(observer.supervise([sys.executable, "-I", "-B", "-c", "pass"], root, {}, root,
                                                2, 1024, 8192, lambda: None), 0)




class HistoryStartupTests(unittest.TestCase):
    def test_initial_empty_root_can_progress_to_exact_complete_history(self):
        with tempfile.TemporaryDirectory() as root:
            case = Path(root).resolve()
            trace = observer.Trace(PROPOSAL)
            reader = observer.History(case, trace)
            reader.scan(1)
            folder = case/"settings/session-state"/PROPOSAL["nativeSessionId"]
            folder.parent.mkdir(parents=True)
            try:
                reader.scan(1)
            except ValueError:
                self.fail("Initial empty history root must remain pending")
            self.assertEqual((reader.offset, trace.count, trace.started), (0, 0, False))
            folder.mkdir()
            reader.scan(1)
            start = event("session.start", sessionId=PROPOSAL["nativeSessionId"], copilotVersion="1.0.90")+"\n"
            (folder/"events.jsonl").write_bytes(start.encode())
            reader.scan(1, final=True)
            self.assertEqual((reader.offset, trace.count, trace.started), (len(start.encode()), 1, True))

    def test_empty_root_is_rejected_at_final_resume_and_after_consumption(self):
        for condition in ("final", "resume", "consumed"):
            with self.subTest(condition=condition), tempfile.TemporaryDirectory() as root:
                case = Path(root).resolve()
                folder = case/"settings/session-state"/PROPOSAL["nativeSessionId"]
                folder.parent.mkdir(parents=True)
                trace = observer.Trace(PROPOSAL)
                reader = observer.History(case, trace)
                if condition == "consumed":
                    folder.mkdir()
                    history = folder/"events.jsonl"
                    history.write_bytes((event("session.start", sessionId=PROPOSAL["nativeSessionId"],
                                              copilotVersion="1.0.90")+"\n").encode())
                    reader.scan(1)
                    history.unlink()
                    folder.rmdir()
                with self.assertRaises(ValueError) as caught:
                    reader.scan(2 if condition == "resume" else 1, final=condition == "final")
                self.assertEqual(observer.failure_details({"stage": "native-history"}, caught.exception)
                                 .get("checkCode"), "history-session-layout")

    def test_layout_flags_distinguish_empty_wrong_and_extra_entries(self):
        cases = [("empty", {"empty": True, "expectedPresent": False, "exclusiveExpected": False, "expectedSymlink": None}),
                 ("wrong", {"empty": False, "expectedPresent": False, "exclusiveExpected": False, "expectedSymlink": None}),
                 ("extra", {"empty": False, "expectedPresent": True, "exclusiveExpected": False, "expectedSymlink": None})]
        for condition, expected in cases:
            with self.subTest(condition=condition), tempfile.TemporaryDirectory() as root:
                case = Path(root).resolve()
                folder = case/"settings/session-state"/PROPOSAL["nativeSessionId"]
                folder.parent.mkdir(parents=True)
                if condition != "empty":
                    (folder.parent/"PRIVATE_ENTRY").mkdir()
                if condition == "extra":
                    folder.mkdir()
                with self.assertRaises(ValueError) as caught:
                    observer.History(case, observer.Trace(PROPOSAL)).scan(1, final=condition == "empty")
                details = observer.failure_details({"stage": "native-history"}, caught.exception)
                self.assertEqual(details.get("layout"), expected)
                self.assertNotIn("PRIVATE", json.dumps(details))

    @unittest.skipUnless(os.name == "posix", "Owned symlink fixtures target POSIX")
    def test_expected_symlink_is_rejected_and_reported_without_target(self):
        with tempfile.TemporaryDirectory() as root:
            case = Path(root).resolve()
            folder = case/"settings/session-state"/PROPOSAL["nativeSessionId"]
            folder.parent.mkdir(parents=True)
            target = case/"PRIVATE_TARGET"
            target.mkdir()
            folder.symlink_to(target, target_is_directory=True)
            with self.assertRaises(ValueError) as caught:
                observer.History(case, observer.Trace(PROPOSAL)).scan(1)
            details = observer.failure_details({"stage": "native-history"}, caught.exception)
            self.assertEqual(details.get("layout"), {"empty": False, "expectedPresent": True,
                                                    "exclusiveExpected": True, "expectedSymlink": True})
            self.assertNotIn("PRIVATE", json.dumps(details))

    def test_layout_projection_rejects_extra_fields_values_and_wrong_context(self):
        valid = {"empty": True, "expectedPresent": False, "exclusiveExpected": False, "expectedSymlink": None}
        cases = [(valid, "native-history", "history-session-layout", valid),
                 ({**valid, "PRIVATE_FIELD": "PRIVATE_VALUE"}, "native-history", "history-session-layout", None),
                 ({**valid, "empty": 1}, "native-history", "history-session-layout", None),
                 ({**valid, "expectedSymlink": "PRIVATE_VALUE"}, "native-history", "history-session-layout", None),
                 (valid, "native-prompt", "history-session-layout", None),
                 (valid, "native-history", "history-prefix", None)]
        for layout, stage, code, expected in cases:
            error = ValueError("PRIVATE_MESSAGE")
            error.history_check_code, error.history_layout = code, layout
            details = observer.failure_details({"stage": stage}, error)
            self.assertEqual(details.get("layout"), expected)
            self.assertNotIn("PRIVATE", json.dumps(details))


class NativeIdentityTests(unittest.TestCase):
    def test_invalid_native_uuid_cannot_reach_a_fresh_session(self):
        for value in ("00000000-0000-0000-0000-000000000001",
                      "00000000-0000-9000-8000-000000000001",
                      "00000000-0000-4000-0000-000000000001", "not-an-id", None):
            with self.subTest(value=value):
                with self.assertRaises(ValueError):
                    observer.validate_native_id(value)

    def test_packet_and_peer_use_one_compatible_identity(self):
        observer.validate_native_id(PROPOSAL["nativeSessionId"])
        self.assertEqual(Protocol().peer.binding.native_id, PROPOSAL["nativeSessionId"])
        new = PROPOSAL["argv"]["new"]
        self.assertEqual(new[new.index("--session-id")+1], PROPOSAL["nativeSessionId"])
        self.assertIn("--resume="+PROPOSAL["nativeSessionId"], PROPOSAL["argv"]["resume"])


    def test_uuid_guard_precedes_platform_setup_and_subprocesses(self):
        directory = Path(__file__).parent
        pb, mb = (directory/"proposal.json").read_bytes(), (directory/"manifest.json").read_bytes()
        approval = {"decision": "approve-one-batch", "proposalSha256": observer.sha(pb),
                    "manifestSha256": observer.sha(mb), "maxNativePrompts": 2}
        with tempfile.TemporaryDirectory() as root:
            output = Path(root).resolve()/"result"
            observed = None
            with patch.object(observer, "validate_native_id", side_effect=ValueError("Synthetic UUID rejection")), \
                    patch("platform.system", side_effect=AssertionError("Platform read before UUID guard")):
                try:
                    observer.run_native(approval, output)
                except Exception as error:
                    observed = error
            self.assertIsInstance(observed, ValueError)
            self.assertFalse(output.exists())


class StopReportingTests(unittest.TestCase):
    def test_history_failures_keep_distinct_fixed_codes_without_payloads(self):
        cases = [("missing-root", "history-root-missing"),
                 ("extra-session", "history-session-layout"),
                 ("missing-events", "history-events-missing"),
                 ("events-directory", "history-events-read"),
                 ("changed-prefix", "history-prefix"),
                 ("partial-tail", "history-tail"),
                 ("event-cap", "history-event-cap"),
                 ("bad-event", "history-event-contract")]
        for case_name, code in cases:
            with self.subTest(case=case_name), tempfile.TemporaryDirectory() as root:
                case = Path(root).resolve()
                folder = case/"settings/session-state"/PROPOSAL["nativeSessionId"]
                trace = observer.Trace(PROPOSAL)
                reader = observer.History(case, trace)
                if case_name != "missing-root":
                    folder.mkdir(parents=True)
                history = folder/"events.jsonl"
                if case_name == "extra-session":
                    (folder.parent/"PRIVATE_SESSION").mkdir()
                elif case_name == "events-directory":
                    history.mkdir()
                elif case_name == "changed-prefix":
                    history.write_text(event("session.start", sessionId=PROPOSAL["nativeSessionId"],
                                             copilotVersion="1.0.90")+"\n")
                    reader.scan(1)
                    history.write_text("{}\n")
                elif case_name == "partial-tail":
                    history.write_text('{"PRIVATE_PAYLOAD":')
                elif case_name in ("event-cap", "bad-event"):
                    history.write_text('{"PRIVATE_PAYLOAD":"PRIVATE_VALUE"}\n')
                    if case_name == "event-cap":
                        trace.count = 4096
                with self.assertRaises((ValueError, OSError)) as caught:
                    reader.scan(1, final=True)
                details = observer.failure_details({"phase": 1, "stage": "native-history"}, caught.exception)
                self.assertEqual(details.get("checkCode"), code)
                if case_name == "events-directory":
                    self.assertIn(details["category"], ("check-failed", "os-error"))
                else:
                    self.assertEqual(details["category"], "check-failed")
                self.assertNotIn("PRIVATE", json.dumps(details))

    def test_history_read_code_preserves_numeric_os_error(self):
        with tempfile.TemporaryDirectory() as root:
            case = Path(root).resolve()
            folder = case/"settings/session-state"/PROPOSAL["nativeSessionId"]
            folder.mkdir(parents=True)
            (folder/"events.jsonl").write_text("")
            with patch("os.open", side_effect=PermissionError(13, "PRIVATE_MESSAGE", "/PRIVATE_PATH")):
                with self.assertRaises(PermissionError) as caught:
                    observer.History(case, observer.Trace(PROPOSAL)).scan(1, final=True)
            details = observer.failure_details({"stage": "native-history"}, caught.exception)
            self.assertEqual(details.get("checkCode"), "history-events-read")
            self.assertEqual((details["category"], details["osError"]), ("os-error", 13))
            self.assertNotIn("PRIVATE", json.dumps(details))

    @unittest.skipUnless(os.name == "posix", "Owned symlink fixtures target POSIX")
    def test_history_link_guard_has_a_fixed_layout_code(self):
        with tempfile.TemporaryDirectory() as root:
            case = Path(root).resolve()
            (case/"settings").mkdir()
            target = case/"PRIVATE_TARGET"
            target.mkdir()
            (case/"settings/session-state").symlink_to(target, target_is_directory=True)
            with self.assertRaises(ValueError) as caught:
                observer.History(case, observer.Trace(PROPOSAL)).scan(1, final=True)
            details = observer.failure_details({"stage": "native-history"}, caught.exception)
            self.assertEqual(details.get("checkCode"), "history-root-layout")
            self.assertNotIn("PRIVATE", json.dumps(details))

    def test_history_code_projection_rejects_unknown_values_and_wrong_stages(self):
        error = ValueError("PRIVATE_MESSAGE")
        for code, stage, expected in [("history-prefix", "native-history", "history-prefix"),
                                      ("PRIVATE_PATH", "native-history", None),
                                      (["history-prefix"], "native-history", None),
                                      ("history-prefix", "native-prompt", None)]:
            error.history_check_code = code
            details = observer.failure_details({"stage": stage}, error)
            self.assertEqual(details.get("checkCode"), expected)
            self.assertNotIn("PRIVATE", json.dumps(details))

    def test_failure_projection_keeps_only_fixed_metadata(self):
        diagnostic = {"phase": 1, "stage": "native-prompt", "process": {
            "started": True, "exitCode": 17, "terminationStepsCompleted": False,
            "stderr": "PRIVATE_OUTPUT", "arguments": "PRIVATE_ARGUMENTS"}}
        error = PermissionError(1, "PRIVATE_MESSAGE", "/PRIVATE_PATH")
        value = observer.failure_details(diagnostic, error)
        self.assertEqual(value, {"phase": 1, "stage": "native-prompt", "category": "os-error",
                                "osError": 1, "process": {"started": True, "exitCode": 17,
                                                       "terminationStepsCompleted": False}})
        self.assertNotIn("PRIVATE", json.dumps(value))
        value = observer.failure_details({"phase": "PRIVATE", "stage": "/PRIVATE_PATH",
                                          "process": {"exitCode": "PRIVATE"}}, RuntimeError("PRIVATE"))
        self.assertEqual(value, {"phase": 0, "stage": "unknown", "category": "unexpected-error",
                                "osError": None, "process": {"started": False, "exitCode": None,
                                                           "terminationStepsCompleted": False}})

    def test_failure_categories_do_not_serialize_exception_text(self):
        import subprocess
        cases = [(ValueError("PRIVATE"), "check-failed"),
                 (json.JSONDecodeError("PRIVATE", "PRIVATE", 0), "invalid-json"),
                 (subprocess.TimeoutExpired(["PRIVATE"], 1), "process-timeout"),
                 (subprocess.SubprocessError("PRIVATE"), "process-error"),
                 (RuntimeError("PRIVATE"), "unexpected-error")]
        for error, category in cases:
            with self.subTest(category=category):
                value = observer.failure_details({"phase": 2, "stage": "receipt-join"}, error)
                self.assertEqual(value.get("category"), category)
                self.assertNotIn("PRIVATE", json.dumps(value))

    @unittest.skipUnless(os.name == "posix", "Owned process groups target POSIX")
    def test_failed_fake_process_retains_exit_even_when_termination_raises(self):
        with tempfile.TemporaryDirectory() as root:
            root = Path(root).resolve()
            receipt = {}
            with patch("os.killpg", side_effect=PermissionError(1, "PRIVATE_MESSAGE")):
                with self.assertRaises(PermissionError) as stopped:
                    observer.supervise([sys.executable, "-I", "-B", "-c", "raise SystemExit(17)"],
                                        root, {}, root, 2, 1024, 8192, lambda: None, receipt=receipt)
            value = observer.failure_details({"phase": 1, "stage": "native-prompt", "process": receipt},
                                            stopped.exception)
            self.assertEqual(value.get("process", {}).get("exitCode"), 17)
            self.assertEqual(value.get("process", {}).get("started"), True)
            self.assertEqual(value.get("process", {}).get("terminationStepsCompleted"), False)
            self.assertNotIn("PRIVATE", json.dumps(value))

    def _failed_run(self, fail_cleanup=False):
        import shutil
        directory = Path(__file__).parent
        pb, mb = (directory/"proposal.json").read_bytes(), (directory/"manifest.json").read_bytes()
        approval = {"decision": "approve-one-batch", "proposalSha256": observer.sha(pb),
                    "manifestSha256": observer.sha(mb), "maxNativePrompts": 2}
        def fail_discovery(*args, **kwargs):
            diagnostic = kwargs.get("diagnostic")
            if diagnostic is not None:
                diagnostic["stage"] = "instruction-discovery"
            raise ValueError("PRIVATE_DISCOVERY_PAYLOAD")
        leftover = []
        def local_only(argv, **kwargs):
            if argv[0] == "/usr/bin/git":
                return
            if "import shutil,sys; shutil.rmtree(sys.argv[1])" in argv:
                target = Path(argv[-1])
                if not target.name.startswith("ork742-native-"):
                    raise AssertionError("Unexpected synthetic cleanup root")
                if fail_cleanup:
                    leftover.append(target)
                    raise PermissionError(13, "PRIVATE_CLEANUP_MESSAGE", "/PRIVATE_PATH")
                shutil.rmtree(target)
                return
            raise AssertionError("Only synthetic filesystem setup/cleanup is allowed")
        with tempfile.TemporaryDirectory() as root:
            output = Path(root).resolve()/"result"
            with patch("platform.system", return_value="Darwin"), patch("platform.machine", return_value="arm64"), \
                    patch.object(observer, "check_native_sources"), \
                    patch.object(observer, "discovery", side_effect=fail_discovery), \
                    patch("subprocess.run", side_effect=local_only), \
                    patch("subprocess.Popen", side_effect=AssertionError("Native launch forbidden")):
                try:
                    result = observer.run_native(approval, output)
                    retained = (output/"summary.json").read_text()
                finally:
                    for target in leftover:
                        shutil.rmtree(target)
            return result, retained

    def test_run_summary_retains_failure_location_without_payload(self):
        result, retained = self._failed_run()
        self.assertTrue(result["rawRootsRemoved"])
        self.assertEqual(result["nativePromptInvocations"], 0)
        self.assertEqual(result.get("failure", {}).get("stage"), "instruction-discovery")
        self.assertEqual(result.get("failure", {}).get("category"), "check-failed")
        self.assertNotIn("PRIVATE", retained)

    def test_raw_cleanup_failure_does_not_erase_primary_failure(self):
        result, retained = self._failed_run(fail_cleanup=True)
        self.assertEqual(result["outcome"], "incomplete-cleanup")
        self.assertEqual(result.get("failure", {}).get("stage"), "instruction-discovery")
        self.assertEqual(result.get("cleanupFailure", {}).get("stage"), "raw-cleanup")
        self.assertEqual(result.get("cleanupFailure", {}).get("osError"), 13)
        self.assertNotIn("PRIVATE", retained)

    def test_called_process_exit_retained_without_command_or_output(self):
        import subprocess
        error = subprocess.CalledProcessError(23, ["PRIVATE_COMMAND"],
                                             output="PRIVATE_STDOUT", stderr="PRIVATE_STDERR")
        value = observer.failure_details({"phase": 0, "stage": "git-init"}, error)
        self.assertEqual(value.get("category"), "process-error")
        self.assertEqual(value.get("process", {}).get("exitCode"), 23)
        self.assertEqual(value.get("process", {}).get("started"), True)
        self.assertNotIn("PRIVATE", json.dumps(value))


if __name__ == "__main__":
    unittest.main()
