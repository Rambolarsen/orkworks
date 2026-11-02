"""Model-free observer regression checks; all native CLI boundaries are simulated."""
import contextlib
import io
import hashlib
import json
from pathlib import Path
import runpy
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import uuid

HERE = Path(__file__).resolve().parent
CONFIG = json.loads((HERE / "configuration.json").read_text())
OBSERVER = runpy.run_path(str(HERE / "observe-diagnostic.py"), run_name="observer_under_test")
PRIVATE = "PRIVATE_SYNTHETIC_REASONING_DO_NOT_RETAIN"

def ident(n):
    return str(uuid.UUID(int=n))

class ObserverBoundaryTests(unittest.TestCase):
    def run_case(self, variant):
        with tempfile.TemporaryDirectory(prefix="ork740-read-mock-test-") as directory:
            parent = Path(directory)
            # Replace only external identity inputs with harmless test bytes. The
            # observer still checks real SHA-256 bindings; no host install is read.
            identity_bytes = {
                Path(CONFIG["nativePath"]): b"synthetic native identity",
                Path(CONFIG["cachePath"])/"app.js": b"synthetic app identity",
                Path(CONFIG["cachePath"])/"prebuilds/darwin-arm64/runtime.node": b"synthetic runtime identity",
            }
            synthetic_config = dict(CONFIG)
            for key, data in zip(["nativeSha256", "appSha256", "runtimeSha256"], identity_bytes.values()):
                synthetic_config[key] = hashlib.sha256(data).hexdigest()
            config_bytes = json.dumps(synthetic_config).encode()
            test_binding = hashlib.sha256(config_bytes).hexdigest()
            authority = parent / "mock-authority.json"
            authority.write_text(json.dumps({"configurationSha256": test_binding, "testOnly": PRIVATE if variant == "extra-auth" else "Synthetic audit input, no real authority"}))
            output = parent / "result"
            real_run, real_popen = subprocess.run, subprocess.Popen
            real_read_bytes = Path.read_bytes
            identity_reads = set()
            def identity_read(path):
                if path == HERE/"configuration.json":
                    return config_bytes
                if path in identity_bytes:
                    identity_reads.add(path)
                    return identity_bytes[path]
                return real_read_bytes(path)
            def discovery(argv, **kwargs):
                if argv[0] != CONFIG["nativePath"]:
                    return real_run(argv, **kwargs)
                if "instruction" in argv:
                    rows = [{"id":"model-agents-md","label":"AGENTS.md","location":"repository","type":"model","sourcePath":"AGENTS.md","defaultDisabled":False}]
                elif "skill" in argv:
                    rows = [{"name":name,"description":"Synthetic vendor listing","source":"builtin","path":str(Path(CONFIG["cachePath"])/"builtin"/name),"enabled":True} for name in ["customize-cloud-agent","github-pr-media"]]
                else:
                    raise AssertionError("Unexpected native command in model-free test")
                return subprocess.CompletedProcess(argv, 0, json.dumps(rows).encode(), b"")
            class FakeNative:
                returncode = 0
                def __init__(self, argv, **kwargs):
                    repo = Path(kwargs["cwd"])
                    home = Path(kwargs["env"]["COPILOT_HOME"])
                    session = home / "session-state" / ident(1)
                    session.mkdir(parents=True)
                    events = []
                    def add(kind, data):
                        events.append({"id":ident(100+len(events)),"timestamp":"2026-11-01T00:00:00Z","parentId":events[-1]["id"] if events else None,"type":kind,"data":data})
                    add("session.start", {"sessionId":ident(1)})
                    add("assistant.message", {"turnId":ident(10),"interactionId":ident(20),"content":PRIVATE,"reasoningText":PRIVATE})
                    add("tool.execution_start", {"toolCallId":ident(30),"toolName":"view","arguments":{"path":"allowed.txt"},"turnId":ident(10),"model":"gpt-6-luna"})
                    add("tool.execution_complete", {"toolCallId":ident(30),"success":True,"result":{"content":{"opaque":PRIVATE}},"turnId":ident(10),"interactionId":ident(20),"model":"gpt-6-luna"})
                    if variant == "second-view":
                        add("tool.execution_start", {"toolCallId":ident(31),"toolName":"view","arguments":{"path":"escape.txt"},"turnId":ident(11)})
                    (session/"events.jsonl").write_text("".join(json.dumps(e)+"\n" for e in events))
                    definitions = [{"name":"view","description":"Synthetic view definition","parameters":{"type":"object","properties":{"path":{"type":"string"}},"required":["path"]}}]
                    spans = []
                    for n in range(2):
                        attributes = {"gen_ai.operation.name":"chat","gen_ai.provider.name":"github","gen_ai.request.model":"gpt-6-luna","gen_ai.response.model":"gpt-6-luna","gen_ai.conversation.id":ident(1),"github.copilot.turn_id":ident(10+n),"github.copilot.interaction_id":ident(20+n),"gen_ai.tool.definitions":json.dumps(definitions)}
                        if variant == "bad-correlation" and n == 0:
                            attributes["github.copilot.interaction_id"] = ident(99)
                        if variant in {"forbidden-content", "malformed-forbidden"} and n == 0:
                            attributes["gen_ai.output.messages"] = json.dumps([{"role":"assistant","parts":[{"type":"text","content":PRIVATE}]}])
                        spans.append({"type":"span","name":"chat gpt-6-luna","traceId":"0"*32,"spanId":str(n).zfill(16),"kind":0,"startTime":[0,0],"endTime":[0,1],"attributes":attributes,"status":{"code":0},"events":[],"resource":{"attributes":{}},"instrumentationScope":{"name":"synthetic-test"}})
                    if variant != "missing-telemetry":
                        encoded = "".join(json.dumps(s)+"\n" for s in spans)
                        if variant == "malformed-forbidden":
                            value = json.dumps(spans[0]["attributes"]["gen_ai.output.messages"])
                            encoded = encoded.replace('"gen_ai.output.messages": '+value, '"gen_ai.output.messages": invalid-private-value', 1)
                        Path(kwargs["env"]["COPILOT_OTEL_FILE_EXPORTER_PATH"]).write_text(encoded)
                    kwargs["stdout"].write(b"synthetic outcome\n")
                def poll(self): return 0
            def launch(argv, **kwargs):
                return FakeNative(argv, **kwargs) if argv[0] == CONFIG["nativePath"] else real_popen(argv, **kwargs)
            argv = ["observer","--fixtures",str(HERE),"--authorization",str(authority),"--output",str(output)]
            with patch.dict(OBSERVER["main"].__globals__, {"APPROVED": test_binding}),patch.object(Path,"read_bytes",identity_read),patch.object(sys,"argv",argv),patch.object(subprocess,"run",side_effect=discovery),patch.object(subprocess,"Popen",side_effect=launch),contextlib.redirect_stdout(io.StringIO()):
                OBSERVER["main"]()
            self.assertEqual(identity_reads, set(identity_bytes))
            receipt = json.loads((output/"summary.json").read_text())
            self.assertTrue(receipt["disposableRootsRemoved"])
            self.assertNotIn(PRIVATE, json.dumps(receipt))
            return receipt

    def test_positive_interaction_is_diagnostic_only(self):
        result = self.run_case("positive")
        self.assertEqual(result["outcome"], "diagnostic-complete")
        self.assertTrue(result["batchComplete"])
        self.assertNotIn("allowedMarkerPresent", json.dumps(result))

    def test_forbidden_attribute_stops_with_name_only(self):
        result = self.run_case("forbidden-content")
        self.assertEqual(result.get("unexpectedContentAttributeNames"), ["gen_ai.output.messages"])
        self.assertEqual(result["outcome"], "unverified")
        self.assertFalse(result["batchComplete"])

    def test_forbidden_key_stops_before_decoding_value_or_cleanup_reparse(self):
        result = self.run_case("malformed-forbidden")
        self.assertEqual(result.get("unexpectedContentAttributeNames"), ["gen_ai.output.messages"])
        self.assertEqual(result["stopReason"], "Unexpected exported content class")
        self.assertNotIn("finalTelemetryStop", result)

    def test_arbitrary_authorization_fields_are_not_retained(self):
        self.assertTrue(self.run_case("extra-auth")["batchComplete"])

    def test_missing_telemetry_is_inconclusive(self):
        self.assertFalse(self.run_case("missing-telemetry")["batchComplete"])

    def test_bad_correlation_is_inconclusive(self):
        self.assertFalse(self.run_case("bad-correlation")["batchComplete"])

    def test_second_view_is_rejected(self):
        result = self.run_case("second-view")
        self.assertFalse(result["batchComplete"])
        self.assertIn("path", result["stopReason"])

if __name__ == "__main__":
    unittest.main()
