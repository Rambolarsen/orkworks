"""One explicitly authorized #740 batch only; historical receipts never authorize a rerun."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import signal
import shutil
import subprocess
import tempfile
import time

APPROVED = "7f226c88b54d02352ffaf788ebf2b545080aee128cc28f7a2155f668af0aa6c0"
CONTENT = {"gen_ai.input.messages", "gen_ai.system_instructions", "gen_ai.tool.definitions"}
META = set("gen_ai.conversation.id gen_ai.operation.name gen_ai.provider.name gen_ai.request.model gen_ai.request.reasoning.level gen_ai.request.stream gen_ai.response.finish_reasons gen_ai.response.id gen_ai.response.model gen_ai.response.time_to_first_chunk gen_ai.usage.cache_write.input_tokens gen_ai.usage.cache_read.input_tokens gen_ai.usage.input_tokens gen_ai.usage.output_tokens gen_ai.usage.reasoning.output_tokens github.copilot.cost github.copilot.initiator github.copilot.interaction_id github.copilot.nano_aiu github.copilot.server_duration github.copilot.service_request_id github.copilot.turn_id enduser.pseudo.id github.copilot.agent.type github.copilot.context.custom_agent_names github.copilot.context.mcp_server_names github.copilot.context.skills github.copilot.git.branch github.copilot.turn_count".split())

def identity(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}

def require(condition, message):
    if not condition:
        raise ValueError(message)

def save(path, value):
    data = (json.dumps(value, indent=2) + "\n").encode()
    require(len(data) <= 131072, "Retained artifact overflow")
    require(sum(f.stat().st_size for f in path.parent.iterdir() if f.is_file() and f != path) + len(data) <= 2097152, "Retained batch overflow")
    path.write_bytes(data)

def strings(value):
    if isinstance(value, str):
        return [value]
    if isinstance(value, list):
        return [s for item in value for s in strings(item)]
    if isinstance(value, dict):
        return [s for item in value.values() for s in strings(item)]
    return []

def decoded(value):
    return json.loads(value) if isinstance(value, str) else value

def native_metadata(line):
    """Scan JSON structure; decode keys and whitelisted IDs/type only, skip content values."""
    def space(i):
        while i < len(line) and line[i].isspace():
            i += 1
        return i
    def quoted(i):
        require(line[i] == '"', "Native JSON key/string expected")
        i += 1
        while i < len(line):
            if line[i] == "\\":
                i += 2
            elif line[i] == '"':
                return i + 1
            else:
                i += 1
        raise ValueError("Unclosed native JSON string")
    def skip(i):
        i = space(i)
        if line[i] == '"':
            return quoted(i)
        if line[i] in "{[":
            closing = "}" if line[i] == "{" else "]"
            i += 1
            while True:
                i = space(i)
                if line[i] == closing:
                    return i + 1
                if line[i] in ",:":
                    i += 1
                else:
                    i = skip(i)
        while i < len(line) and line[i] not in ",}] \t\r\n":
            i += 1
        return i
    def obj(i, nested=False):
        require(line[i] == "{", "Native JSON object expected")
        i += 1
        result, fields = {}, []
        while True:
            i = space(i)
            if line[i] == "}":
                return {**result, "fields": fields}, i + 1
            begin = i
            i = quoted(i)
            key = json.loads(line[begin:i])
            fields.append(key)
            i = space(i)
            require(line[i] == ":", "Native JSON colon expected")
            i = space(i+1)
            if key == "data" and not nested:
                result["data"], i = obj(i, True)
            else:
                begin = i
                i = skip(i)
                if key in ({"sessionId", "turnId", "interactionId"} if nested else {"type"}):
                    result[key] = json.loads(line[begin:i])
            i = space(i)
            require(line[i] in ",}", "Native JSON separator expected")
            if line[i] == ",":
                i += 1
    result, end = obj(space(0))
    require(space(end) == len(line), "Trailing native JSON content")
    return result

def stop_process(proc):
    os.killpg(proc.pid, signal.SIGTERM)
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        os.killpg(proc.pid, signal.SIGKILL)
        proc.wait(timeout=5)

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--fixtures", type=Path, required=True)
    parser.add_argument("--authorization", type=Path, required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    os.umask(0o077)
    config_bytes = (args.fixtures / "configuration.json").read_bytes()
    require(identity(config_bytes)["sha256"] == APPROVED, "Approved configuration changed")
    c = json.loads(config_bytes)
    skill_binding = c["startupSkillBinding"]
    selected_source = (args.fixtures / skill_binding["sourceFile"]).read_bytes()
    require(identity(selected_source)["sha256"] == skill_binding["sourceSha256"] and len(selected_source) == skill_binding["sourceBytes"], "Selected skill source changed")
    require((args.fixtures / "root-rule.txt").read_bytes() == skill_binding["prefixUtf8"].encode() + selected_source + skill_binding["suffixUtf8"].encode(), "Startup skill wrapper changed")
    approval = json.loads(args.authorization.read_text())
    require(approval["configurationSha256"] == APPROVED, "Approval does not bind this configuration")
    require(not args.output.exists(), "Output must be fresh")
    for path, digest in [(Path(c["nativePath"]), c["nativeSha256"]), (Path(c["cachePath"])/"app.js", c["appSha256"]), (Path(c["cachePath"])/"prebuilds/darwin-arm64/runtime.node", c["runtimeSha256"])]:
        require(identity(path.read_bytes())["sha256"] == digest, "Pinned identity changed")
    batch = Path(tempfile.mkdtemp(prefix="ork740-approved-comparison-"))
    summary = None
    try:
        args.output.mkdir(parents=True)
        save(args.output / "private-root.json", {"path": str(batch), "purpose": "Remove after extracting sanitized measurements; not for publication"})
        inherited = {k: os.environ[k] for k in c["environment"]["inheritedBindingNames"] if k in os.environ}
        require(all(not any(marker in v for marker in ["ghp_", "gho_", "github_pat_", "Bearer "]) for v in inherited.values()), "Unexpected secret-bearing inherited binding")
        def normalize(value, case=None):
            replacements = [(str(batch), "<batch>"), (str(Path(c["cachePath"])), "<pinned-cache>")]
            if case is not None:
                replacements.insert(0, (str(case), "<case>"))
            if inherited.get("TMPDIR"):
                replacements.append((inherited["TMPDIR"], "<temp>/"))
            if inherited.get("HOME"):
                replacements.append((inherited["HOME"], "<home>"))
            for source, target in replacements:
                value = value.replace(source, target)
            return value
        summary = {"configurationSha256": APPROVED, "authorization": approval, "inheritedNonsecretBindings": {k: normalize(v) for k,v in inherited.items()}, "liveCliAttempts": 0, "observedChatSpans": 0, "cases": [], "batchComplete": False}
        template = batch / "empty-template"
        template.mkdir()
        started = time.monotonic()
        failure = None
        active_process = None
        try:
            for case_config in c["cases"]:
                require(time.monotonic()-started < c["limits"]["secondsPerBatch"], "Batch timeout")
                case = batch / case_config["id"]
                for name in ["repo", "settings", "logs"]:
                    (case / name).mkdir(parents=True, mode=0o700)
                settings = case / "settings/settings.json"
                before = (args.fixtures / case_config["settingsFile"]).read_bytes()
                require(identity(before)["sha256"] == case_config["settingsSha256"], "Settings input changed")
                settings.write_bytes(before)
                rule = (args.fixtures / "root-rule.txt").read_bytes()
                (case / "repo/AGENTS.md").write_bytes(rule)
                git_env = {**inherited, "GIT_CONFIG_GLOBAL": "/dev/null", "GIT_CONFIG_NOSYSTEM": "1"}
                subprocess.run(["git", "init", "-q", "--template="+str(template), str(case/"repo")], env=git_env, check=True, capture_output=True, timeout=30)
                env = {**inherited, **{k:v.replace("<case>", str(case)) for k,v in c["environment"]["explicitOverrides"].items()}}
                record = {"id": case_config["id"], "settingsBefore": identity(before), "preflight": [], "liveLaunched": False}
                summary["cases"].append(record)
                for discovery in c["preflight"]["argvs"]:
                    pre_env = {k:v for k,v in env.items() if k not in ["COPILOT_OTEL_FILE_EXPORTER_PATH", "COPILOT_OTEL_EXPORTER_TYPE", "OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT"]}
                    argv = [c["nativePath"], *discovery[1:]]
                    result = subprocess.run(argv, cwd=case/"repo", env=pre_env, capture_output=True, timeout=30)
                    receipt = {"command": discovery, "exitCode": result.returncode, "stdoutUnretained": identity(result.stdout), "stderrBytes": len(result.stderr)}
                    record["preflight"].append(receipt)
                    require(result.returncode == 0 and not result.stderr and len(result.stdout) <= 131072, "Discovery failed or exceeded output limit")
                    data = json.loads(result.stdout)
                    if "instruction" in discovery:
                        require(len(data) == 1 and data[0].get("location") == "repository" and data[0].get("sourcePath") == "AGENTS.md" and not data[0].get("defaultDisabled"), "Unexpected instruction source")
                        receipt["acceptedSources"] = [{"location":"repository", "sourcePath":"AGENTS.md"}]
                    else:
                        require({x["name"] for x in data} == {"customize-cloud-agent", "github-pr-media"}, "Unexpected skill source")
                        for x in data:
                            require(x.get("source") == "builtin" and Path(x["path"]).resolve() == (Path(c["cachePath"])/"builtin"/x["name"]).resolve(), "Unexpected builtin source path")
                        receipt["acceptedSources"] = [{"name":x["name"], "source":"builtin", "path":normalize(x["path"], case)} for x in data]
                require(settings.read_bytes() == before, "Settings changed during preflight")
                require(not (case/"settings/config.json").exists(), "Unexpected config.json before inference")
                argv = [x.replace("<pinned-native>", c["nativePath"]).replace("<case>", str(case)) for x in c["argv"]]
                record["argv"] = c["argv"]
                record["environmentOverrides"] = c["environment"]["explicitOverrides"]
                record["cwd"] = c["cwd"]
                t0 = time.monotonic()
                with (case/"stdout.raw").open("wb") as stdout, (case/"stderr.raw").open("wb") as stderr:
                    proc = subprocess.Popen(argv, cwd=case/"repo", env=env, stdin=subprocess.DEVNULL, stdout=stdout, stderr=stderr, start_new_session=True)
                    active_process = proc
                    record["liveLaunched"] = True
                    summary["liveCliAttempts"] += 1
                    record["pid"] = proc.pid
                    while proc.poll() is None:
                        over_size = any(f.exists() and f.stat().st_size > 131072 for f in [case/"otel.jsonl", case/"stdout.raw", case/"stderr.raw"])
                        if time.monotonic()-t0 > 120 or over_size:
                            stop_process(proc)
                            raise ValueError("Live timeout or output overflow")
                        if (case/"otel.jsonl").exists():
                            partial = (case/"otel.jsonl").read_bytes()
                            completed = partial.splitlines() if partial.endswith(b"\n") else partial.splitlines()[:-1]
                            observed = [json.loads(line) for line in completed]
                            span_rows = [r for r in observed if r.get("type") == "span"]
                            require(all(r.get("attributes", {}).get("gen_ai.operation.name") in {"chat", "invoke_agent"} for r in span_rows), "Unexpected span operation while running")
                            record["chatSpanCount"] = sum(r.get("attributes", {}).get("gen_ai.operation.name") == "chat" for r in span_rows)
                            summary["observedChatSpans"] = sum(r.get("chatSpanCount", 0) for r in summary["cases"])
                            if record["chatSpanCount"] > 1:
                                raise ValueError("Extra model call observed while running")
                            if any(r.get("name", "").startswith("execute_tool") for r in span_rows):
                                raise ValueError("Tool invocation observed while running")
                        time.sleep(0.1)
                    record["exitCode"] = proc.returncode
                    active_process = None
                record["seconds"] = round(time.monotonic()-t0, 3)
                record["settingsAfter"] = identity(settings.read_bytes())
                record["settingsUnchanged"] = settings.read_bytes() == before
                record["stdoutMatchesExpected"] = (case/"stdout.raw").read_text().strip() == c["expectedStdout"]
                record["stdoutUnretained"] = identity((case/"stdout.raw").read_bytes())
                record["stderrBytes"] = (case/"stderr.raw").stat().st_size
                require(proc.returncode == 0 and record["stderrBytes"] == 0 and record["stdoutMatchesExpected"], "Live completion failed")
                require(record["settingsUnchanged"], "Settings changed during live process")
                raw = (case/"otel.jsonl").read_bytes()
                require(len(raw) <= 131072, "OTel overflow")
                rows = [json.loads(line) for line in raw.splitlines()]
                record["rawOtelUnretained"] = identity(raw)
                spans = [r for r in rows if r.get("type") == "span"]
                chats = [r for r in spans if r.get("attributes", {}).get("gen_ai.operation.name") == "chat"]
                record["chatSpanCount"] = len(chats)
                summary["observedChatSpans"] = sum(r.get("chatSpanCount", 0) for r in summary["cases"])
                require(len(chats) == 1 and summary["observedChatSpans"] <= c["limits"]["maxObservedModelCalls"], "Unexpected observed model-call count")
                require(not any(r.get("name", "").startswith("execute_tool") or r.get("attributes", {}).get("gen_ai.operation.name") == "execute_tool" for r in spans), "Unexpected tool invocation")
                record["spanMeasurements"] = []
                for r in spans:
                    a = r.get("attributes", {})
                    require(a.get("gen_ai.operation.name") in {"chat", "invoke_agent"}, "Unexpected span operation")
                    measurement = {"name": r.get("name"), "attributeNames": sorted(a), "contentAttributes": {k:identity(a[k].encode()) for k in CONTENT if k in a and isinstance(a[k],str)}, "modelsAndUsage": {k:v for k,v in a.items() if k in ["gen_ai.request.model", "gen_ai.response.model", "gen_ai.usage.input_tokens", "gen_ai.usage.output_tokens", "gen_ai.usage.reasoning.output_tokens", "github.copilot.turn_count"]}}
                    record["spanMeasurements"].append(measurement)
                    require(not set(a) - META - CONTENT, "Unexpected OTel attribute")
                    require(a.get("gen_ai.provider.name") == "github", "Provider/source drift")
                    if case_config["id"] == "metadata-only":
                        require(not set(a) & CONTENT, "Content present in metadata-only case")
                    elif a.get("gen_ai.operation.name") == "chat":
                        require(CONTENT <= set(a), "Missing content receipt")
                        measurement["exactRootRuleTextPresent"] = any(rule.decode() in s for s in strings(decoded(a["gen_ai.system_instructions"])))
                        source_text = selected_source.decode("utf-8")
                        delivered_parts = [part for part in strings(decoded(a["gen_ai.system_instructions"])) if source_text in part]
                        measurement["selectedSkillSourceTextPresent"] = bool(delivered_parts)
                        require(delivered_parts, "Selected skill source delivery missing")
                        part = delivered_parts[0]
                        position = part.index(source_text)
                        measurement["deliveredSkillSource"] = identity(part[position:position+len(source_text)].encode("utf-8"))
                        require(measurement["deliveredSkillSource"] == identity(selected_source), "Delivered skill bytes differ")
                        measurement["toolNames"] = [t.get("name", t.get("function",{}).get("name")) for t in decoded(a["gen_ai.tool.definitions"])]
                        require(measurement["exactRootRuleTextPresent"] and set(measurement["toolNames"]) == set(c["expectedObservedToolNamesWhenAvailable"]), "Rule or observed inventory mismatch")
                        messages = decoded(a["gen_ai.input.messages"])
                        measurement["inputRoles"] = [m.get("role") for m in messages]
                        require(measurement["inputRoles"] == ["user"], "Unexpected later/input role")
                chat = chats[0]["attributes"]
                require(chat.get("gen_ai.request.model") == "gpt-6-luna" and chat.get("gen_ai.response.model") == "gpt-6-luna", "Model drift")
                record["nativeHistoryMetadata"] = []
                for file in (case/"settings/session-state").glob("*/events.jsonl"):
                    events = [native_metadata(line) for line in file.read_text().splitlines()]
                    starts = [e["data"] for e in events if e.get("type") == "session.start"]
                    assistants = [e["data"] for e in events if e.get("type") == "assistant.message"]
                    record["nativeHistoryMetadata"].append({"path":"settings/session-state/<session-id>/events.jsonl", "bytes":file.stat().st_size, "eventTypes":[e.get("type") for e in events], "sessionMatchesChat": len(starts)==1 and bool(chat.get("gen_ai.conversation.id")) and starts[0].get("sessionId") == chat.get("gen_ai.conversation.id"), "assistantTurnMatchesChat":[bool(chat.get("github.copilot.turn_id")) and d.get("turnId") == chat.get("github.copilot.turn_id") for d in assistants], "assistantInteractionMatchesChat":[bool(chat.get("github.copilot.interaction_id")) and d.get("interactionId") == chat.get("github.copilot.interaction_id") for d in assistants], "assistantFieldsPresent":[{k:k in d["fields"] for k in ["content","reasoningText","reasoningBlocks","reasoningOpaque","encryptedContent"]} for d in assistants]})
                require(len(record["nativeHistoryMetadata"]) == 1 and record["nativeHistoryMetadata"][0]["sessionMatchesChat"] and record["nativeHistoryMetadata"][0]["assistantTurnMatchesChat"] == [True] and record["nativeHistoryMetadata"][0]["assistantInteractionMatchesChat"] == [True], "Missing native/session correlation")
                record["casePassed"] = True
                save(args.output/"summary.json", summary)
            summary["batchComplete"] = True
        except Exception as error:
            failure = type(error).__name__
            # Only observer-generated messages are printable; vendor diagnostics remain private.
            summary["stopReason"] = str(error) if type(error) is ValueError else failure
        finally:
            if active_process is not None and active_process.poll() is None:
                try:
                    stop_process(active_process)
                    summary["foregroundProcessStoppedAfterFailure"] = True
                except Exception:
                    summary["foregroundProcessStoppedAfterFailure"] = False
            summary["elapsedSeconds"] = round(time.monotonic()-started, 3)
            save(args.output/"summary.json", summary)

    finally:
        cleaned = False
        try:
            shutil.rmtree(batch)
            cleaned = not batch.exists()
        except Exception:
            cleaned = False
        if summary is not None:
            summary["disposableRootsRemoved"] = cleaned
            if not cleaned:
                summary["batchComplete"] = False
                summary["stopReason"] = "Disposable cleanup failed"
            try:
                save(args.output / "summary.json", summary)
            except ValueError:
                summary = {"batchComplete": False, "stopReason": "Retained evidence overflow; case inconclusive", "liveCliAttempts": summary["liveCliAttempts"], "observedChatSpans": summary["observedChatSpans"], "disposableRootsRemoved": cleaned}
                save(args.output / "summary.json", summary)
            if cleaned:
                (args.output / "private-root.json").unlink(missing_ok=True)
            print(json.dumps({"batchComplete": summary["batchComplete"], "liveCliAttempts": summary["liveCliAttempts"], "observedChatSpans": summary["observedChatSpans"], "stopReason": summary.get("stopReason"), "disposableRootsRemoved": cleaned, "output": str(args.output)}))

if __name__ == "__main__":
    main()
