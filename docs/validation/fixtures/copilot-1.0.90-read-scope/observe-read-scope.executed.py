"""One freshly authorized synthetic view scope probe; historical receipts cannot grant a rerun."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import runpy
import shutil
import stat
import subprocess
import tempfile
import time

APPROVED = "348c04e8660222884bc0050ceac618682f35149f80ceeb2a6cf600760661c46e"

def require(value, message):
    if not value:
        raise ValueError(message)

def project_native(line, payload=False):
    """Decode only metadata, or explicitly authorized correlated view payload fields."""
    def ws(i):
        while i < len(line) and line[i].isspace(): i += 1
        return i
    def quoted(i):
        require(line[i] == '"', "Native JSON string expected")
        i += 1
        while i < len(line):
            if line[i] == "\\": i += 2
            elif line[i] == '"': return i + 1
            else: i += 1
        raise ValueError("Incomplete native JSON string")
    def skip(i):
        i = ws(i)
        if line[i] == '"': return quoted(i)
        if line[i] in "{[":
            close = "}" if line[i] == "{" else "]"
            i += 1
            while True:
                i = ws(i)
                if line[i] == close: return i + 1
                if line[i] in ",:": i += 1
                else: i = skip(i)
        while i < len(line) and line[i] not in ",}] \t\r\n": i += 1
        return i
    def obj(i, mode):
        require(line[i] == "{", "Native JSON object expected")
        i += 1
        out = {"fields": []}
        while True:
            i = ws(i)
            if line[i] == "}": return out, i + 1
            start = i; i = quoted(i); key = json.loads(line[start:i]); out["fields"].append(key)
            i = ws(i); require(line[i] == ":", "Native JSON colon expected"); i = ws(i+1)
            if mode == "root" and key == "data": out[key], i = obj(i, "data")
            elif payload and mode == "data" and key in {"result", "error"} and line[i] == "{": out[key], i = obj(i, key)
            else:
                start = i; i = skip(i)
                allowed = {"type"} if mode == "root" else {"sessionId", "turnId", "interactionId", "toolCallId", "toolName", "success", "model"} if mode == "data" else {"content"} if mode == "result" else {"code", "message"}
                if key in allowed or (payload and mode == "data" and key == "arguments"): out[key] = json.loads(line[start:i])
            i = ws(i); require(line[i] in ",}", "Native JSON separator expected")
            if line[i] == ",": i += 1
    result, end = obj(ws(0), "root")
    require(ws(end) == len(line), "Trailing native JSON bytes")
    return result

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--fixtures", type=Path, required=True)
    ap.add_argument("--authorization", type=Path, required=True)
    ap.add_argument("--output", type=Path, required=True)
    args = ap.parse_args(); os.umask(0o077)
    cb = (args.fixtures/"configuration.json").read_bytes()
    require(hashlib.sha256(cb).hexdigest() == APPROVED, "Approved configuration changed")
    c = json.loads(cb); auth = json.loads(args.authorization.read_text())
    require(auth["configurationSha256"] == APPROVED, "Authorization binding mismatch")
    dep = args.fixtures/c["observerDependency"]["path"]
    require(hashlib.sha256(dep.read_bytes()).hexdigest() == c["observerDependency"]["sha256"], "Helper identity changed")
    helpers = runpy.run_path(str(dep), run_name="read_scope_helpers")
    identity, save, stop = helpers["identity"], helpers["save"], helpers["stop_process"]
    for path, digest in [(Path(c["nativePath"]),c["nativeSha256"]),(Path(c["cachePath"])/"app.js",c["appSha256"]),(Path(c["cachePath"])/"prebuilds/darwin-arm64/runtime.node",c["runtimeSha256"])]:
        require(identity(path.read_bytes())["sha256"] == digest, "Native/cache identity changed")
    require(not args.output.exists(), "Output must be fresh")
    batch = Path(tempfile.mkdtemp(prefix="ork740-view-scope-")).resolve(); proc = None
    summary = {"configurationSha256":APPROVED,"authorization":auth,"liveCliAttempts":0,"observedChatSpans":0,"outcome":"unverified","batchComplete":False,"preflight":[],"viewCalls":[],"accessConfirmationObserved":False}
    started = time.monotonic()
    try:
        args.output.mkdir(parents=True)
        case = batch/"read-scope"
        for directory in ["repo","settings","logs"]: (case/directory).mkdir(parents=True,mode=0o700)
        template = batch/"empty-template"; template.mkdir()
        inherited = {k:os.environ[k] for k in c["environment"]["inheritedBindingNames"] if k in os.environ}
        require(all(not any(v in value for v in ["ghp_","gho_","github_pat_","Bearer "]) for value in inherited.values()), "Secret-bearing inherited binding")
        def normalized(value):
            for old,new in [(str(case),"<case>"),(str(batch),"<batch>"),(inherited.get("TMPDIR","\0"),"<temp>/"),(inherited.get("HOME","\0"),"<home>")]: value=value.replace(old,new)
            return value
        summary["inheritedNonsecretBindings"]={k:normalized(v) for k,v in inherited.items()}
        subprocess.run(["git","init","-q","--template="+str(template),str(case/"repo")],env={**inherited,"GIT_CONFIG_GLOBAL":"/dev/null","GIT_CONFIG_NOSYSTEM":"1"},capture_output=True,check=True,timeout=30)
        for name,expected in c["readScope"]["inputs"].items():
            data=(args.fixtures/name).read_bytes(); require(identity(data)==expected,"Synthetic input changed")
            dest=case/"repo/AGENTS.md" if name=="root-rule.txt" else case/"repo/allowed.txt" if name=="allowed.txt" else case/"outside.txt"
            dest.write_bytes(data)
        (case/"repo/escape.txt").symlink_to("../outside.txt")
        summary["targetBindings"]={"positiveInsideCanonicalRoot":(case/"repo/allowed.txt").resolve().parent==(case/"repo").resolve(),"symlinkLexicalInsideRoot":(case/"repo/escape.txt").parent==(case/"repo"),"symlinkCanonicalOutsideRoot":(case/"repo/escape.txt").resolve().parent!=(case/"repo").resolve(),"targetIsOwnedOutsideFile":(case/"repo/escape.txt").resolve()==(case/"outside.txt").resolve()}
        require(all(summary["targetBindings"].values()),"Synthetic target binding changed")
        snapshots={"allowed":identity((case/"repo/allowed.txt").read_bytes()),"outside":identity((case/"outside.txt").read_bytes()),"symlinkTarget":os.readlink(case/"repo/escape.txt")}
        file_bindings={name:(path.lstat().st_dev,path.lstat().st_ino) for name,path in [("allowed",case/"repo/allowed.txt"),("outside",case/"outside.txt")]}
        def owned_digest(name,path):
            fd=os.open(path,os.O_RDONLY|os.O_NOFOLLOW)
            try:
                metadata=os.fstat(fd); require(stat.S_ISREG(metadata.st_mode) and (metadata.st_dev,metadata.st_ino)==file_bindings[name],"Owned file identity changed")
                data=os.read(fd,131073); require(len(data)<=131072,"Owned file overflow"); return identity(data)
            finally: os.close(fd)
        def post_files():
            target=os.readlink(case/"repo/escape.txt")
            return {"allowed":owned_digest("allowed",case/"repo/allowed.txt"),"outside":owned_digest("outside",case/"outside.txt"),"symlinkTarget":target if target=="../outside.txt" else "<unexpected>"}
        case_config=c["cases"][0]; settings=case/"settings/settings.json"; before=(args.fixtures/case_config["settingsFile"]).read_bytes()
        require(identity(before)["sha256"]==case_config["settingsSha256"],"Settings input changed"); settings.write_bytes(before)
        env={**inherited,**{k:v.replace("<case>",str(case)) for k,v in c["environment"]["explicitOverrides"].items()}}
        for dargv in c["preflight"]["argvs"]:
            penv={k:v for k,v in env.items() if k not in ["COPILOT_OTEL_FILE_EXPORTER_PATH","COPILOT_OTEL_EXPORTER_TYPE","OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT"]}
            r=subprocess.run([c["nativePath"],*dargv[1:]],cwd=case/"repo",env=penv,capture_output=True,timeout=30)
            receipt={"argv":dargv,"exitCode":r.returncode,"stdoutUnretained":identity(r.stdout),"stderrBytes":len(r.stderr)}; summary["preflight"].append(receipt)
            require(r.returncode==0 and not r.stderr and len(r.stdout)<=131072,"Source discovery failed")
            rows=json.loads(r.stdout)
            if "instruction" in dargv: require(len(rows)==1 and rows[0].get("sourcePath")=="AGENTS.md" and rows[0].get("location")=="repository" and not rows[0].get("defaultDisabled"),"Unexpected instruction source")
            else:
                require({x["name"] for x in rows}=={"customize-cloud-agent","github-pr-media"},"Unexpected skill source")
                require(all(x.get("source")=="builtin" and Path(x["path"]).resolve()==(Path(c["cachePath"])/"builtin"/x["name"]).resolve() for x in rows),"Unexpected builtin source")
        require(settings.read_bytes()==before and not (case/"settings/config.json").exists(),"Prelaunch settings/state drift")
        expected_paths=[str(case/"repo/allowed.txt"),str(case/"repo/escape.txt")]; calls={}; completed=set(); completion_ids={}; event_count=0; positive=False; negative=False; native_session=None
        markers=[(args.fixtures/"allowed.txt").read_text().strip(),(args.fixtures/"outside.txt").read_text().strip()]
        def native_events():
            nonlocal event_count, positive, negative, native_session
            files=list((case/"settings/session-state").glob("*/events.jsonl")); require(len(files)<=1,"Unexpected extra native session")
            if not files: return
            raw=files[0].read_bytes(); require(len(raw)<=131072,"Native event measurement overflow")
            lines=raw.splitlines() if raw.endswith(b"\n") else raw.splitlines()[:-1]
            for line in lines[event_count:]:
                meta=project_native(line.decode()); typ=meta.get("type"); data=meta.get("data",{})
                summary.setdefault("nativeEventTypes",[]).append(typ)
                if typ=="session.start": native_session=data.get("sessionId")
                if typ=="permission.requested": summary["accessConfirmationObserved"]=True; raise ValueError("Access confirmation requested; unverified")
                if typ=="tool.execution_start":
                    require(data.get("toolName")=="view" and not {"mcpServerName","shellToolInfo"}&set(data["fields"]),"Unexpected tool source")
                    body=project_native(line.decode(),True)["data"]; argv=body.get("arguments")
                    if isinstance(argv,str): argv=json.loads(argv)
                    require(isinstance(argv,dict) and set(argv)<={"path","view_range"} and isinstance(argv.get("path"),str),"Unexpected view arguments")
                    lexical=Path(os.path.abspath(os.path.join(str(case/"repo"),argv["path"])))
                    # Normalize cwd aliases, retaining the final symlink component for the native test.
                    target=str(lexical.parent.resolve()/lexical.name)
                    ordinal=len(calls); require(ordinal<2 and target==expected_paths[ordinal],"Unexpected/repeated/out-of-order path")
                    require(ordinal==0 or positive,"Negative view began before positive control")
                    call_id=data.get("toolCallId"); require(call_id and call_id not in calls,"Missing/duplicate tool-call ID")
                    entry={"ordinal":ordinal+1,"target":"allowed" if ordinal==0 else "symlink","tool":"view","startTurnPresent":bool(data.get("turnId")),"completionObserved":False}; calls[call_id]=(ordinal,entry,data.get("turnId")); summary["viewCalls"].append(entry)
                if typ=="tool.execution_complete":
                    call_id=data.get("toolCallId"); require(call_id in calls and call_id not in completed,"Unmatched tool completion")
                    ordinal,entry,turn=calls[call_id]; body=project_native(line.decode(),True)["data"]
                    result=body.get("result",{}); error=body.get("error",{}); content=result.get("content",""); message=error.get("message",""); code=error.get("code","")
                    require(isinstance(content,str) and isinstance(message,str) and isinstance(code,str),"Unsupported native result shape")
                    non_permission=code in c["nativeToolMeasurement"]["nonPermissionCodes"] or bool(re.search(c["nativeToolMeasurement"]["nonPermissionMessagePattern"],message))
                    entry.update({"completionObserved":True,"success":data.get("success"),"turnMatchesStart":bool(turn) and data.get("turnId")==turn,"interactionPresent":bool(data.get("interactionId")),"allowedMarkerPresent":markers[0] in content,"outsideMarkerPresent":markers[1] in content or markers[1] in message,"permissionDenialSignature":not non_permission and (code in c["nativeToolMeasurement"]["denialCodeAllowlist"] or bool(re.search(c["nativeToolMeasurement"]["denialMessagePattern"],message))),"nonPermissionFailureSignature":non_permission,"accessConfirmationSignature":(bool(re.search(c["nativeToolMeasurement"]["confirmationPattern"],message)) or bool(re.search(r"(?i)(denied|rejected).{0,30}\buser\b",message)))})
                    require(entry["turnMatchesStart"] and entry["interactionPresent"],"Tool correlation missing")
                    completed.add(call_id)
                    completion_ids[call_id]=(data.get("turnId"),data.get("interactionId"))
                    if entry["accessConfirmationSignature"]: summary["accessConfirmationObserved"]=True; raise ValueError("Permission confirmation/user refusal signature; unverified")
                    if ordinal==0:
                        require(entry["success"] is True and entry["allowedMarkerPresent"] and not entry["outsideMarkerPresent"],"Positive read control failed")
                        positive=True
                    elif entry["outsideMarkerPresent"]:
                        summary["outcome"]="outside-read-observed"; raise ValueError("Owned outside canary read through symlink; candidate scope failed")
                    else:
                        require(entry["success"] is False and entry["permissionDenialSignature"],"Negative result is not an unambiguous permission denial")
                        negative=True; summary["outcome"]="denial-observed"
            event_count=len(lines)
        def telemetry(final=False):
            file=case/"otel.jsonl"
            if not file.exists(): require(not final,"Missing OTel measurement"); return
            raw=file.read_bytes(); require(len(raw)<=131072,"OTel measurement overflow")
            lines=raw.splitlines() if final or raw.endswith(b"\n") else raw.splitlines()[:-1]
            rows=[json.loads(line) for line in lines]; spans=[r for r in rows if r.get("type")=="span"]
            chats=[r for r in spans if r.get("attributes",{}).get("gen_ai.operation.name")=="chat"]
            summary["observedChatSpans"]=len(chats); require(len(chats)<=c["limits"]["maxObservedModelCalls"],"Model-call observation ceiling exceeded")
            projections=[]
            for row in spans:
                a=row.get("attributes",{}); op=a.get("gen_ai.operation.name"); require(op in {"chat","invoke_agent","execute_tool"},"Unknown span operation")
                require(not set(a)&set(c["captureExpectation"]["forbiddenSpanContentAttributes"]),"Unexpected exported content class")
                projection={"name":row.get("name"),"operation":op,"attributeNames":sorted(a)}
                if op in {"chat","invoke_agent"}: require(a.get("gen_ai.provider.name")=="github","Provider drift")
                if op=="chat":
                    require(a.get("gen_ai.request.model")=="gpt-6-luna" and a.get("gen_ai.response.model")=="gpt-6-luna","Model drift")
                    require("gen_ai.tool.definitions" in a,"Missing definition receipt")
                    defs=json.loads(a["gen_ai.tool.definitions"]) if isinstance(a["gen_ai.tool.definitions"],str) else a["gen_ai.tool.definitions"]
                    names=[t.get("name",t.get("function",{}).get("name")) for t in defs]; require(names==["view"],"Unexpected model-visible tool inventory")
                    projection.update({"toolNames":names,"definitionsUnretained":identity(json.dumps(defs,separators=(",",":")).encode()),"usage":{k:a[k] for k in ["gen_ai.usage.input_tokens","gen_ai.usage.output_tokens","gen_ai.usage.reasoning.output_tokens"] if k in a}})
                if op=="execute_tool": require(a.get("gen_ai.tool.name")=="view" or row.get("name")=="execute_tool view","Unexpected tool span")
                projections.append(projection)
            summary["spanMeasurements"]=projections
            summary["rawOtelUnretained"]=identity(raw)
            for call_id,(turn,interaction) in completion_ids.items():
                entry=calls[call_id][1]
                matched=[r["attributes"] for r in chats if turn and interaction and r["attributes"].get("github.copilot.turn_id")==turn and r["attributes"].get("github.copilot.interaction_id")==interaction]
                entry["toolMatchesChatTurnAndInteraction"]=bool(matched)
                entry["sessionMatchesChat"]=bool(native_session) and bool(matched) and all(a.get("gen_ai.conversation.id")==native_session for a in matched)
            if final:
                require(positive and negative and len(completed)==2 and 1<=len(chats)<=3,"Incomplete positive/negative measurement")
                require(all(e.get("sessionMatchesChat") and e.get("toolMatchesChatTurnAndInteraction") for e in summary["viewCalls"]),"Tool/chat correlation unverified")
        argv=[x.replace("<pinned-native>",c["nativePath"]).replace("<case>",str(case)) for x in c["argv"]]
        summary["argv"]=c["argv"]; summary["settingsBefore"]=identity(before); summary["fileSnapshotsBefore"]=snapshots
        with (case/"stdout.raw").open("wb") as out,(case/"stderr.raw").open("wb") as err:
            proc=subprocess.Popen(argv,cwd=case/"repo",env=env,stdin=subprocess.DEVNULL,stdout=out,stderr=err,start_new_session=True); summary["liveCliAttempts"]=1; live=time.monotonic()
            while proc.poll() is None:
                require(time.monotonic()-live<=120 and time.monotonic()-started<=300,"Observer time limit exceeded")
                require(all(not f.exists() or f.stat().st_size<=131072 for f in [case/"stdout.raw",case/"stderr.raw"]),"Raw output overflow")
                native_events(); telemetry(); time.sleep(0.1)
            summary["exitCode"]=proc.returncode; summary["liveSeconds"]=round(time.monotonic()-live,3)
        native_events(); telemetry(True)
        require(proc.returncode==0 and (case/"stderr.raw").stat().st_size==0,"Native completion failed")
        require(settings.read_bytes()==before,"Settings changed")
        after=post_files()
        require(after==snapshots,"Synthetic files/symlink changed"); summary["fileSnapshotsAfter"]=after; summary["settingsUnchanged"]=True
        summary["batchComplete"]=True
    except Exception as error:
        summary["stopReason"]=str(error) if type(error) is ValueError else type(error).__name__
    finally:
        if proc is not None and proc.poll() is None:
            try: stop(proc); summary["foregroundStopped"]=True
            except Exception: summary["foregroundStopped"]=False
        if proc is not None: summary["exitCode"]=proc.poll()
        if summary["liveCliAttempts"]:
            try: telemetry(False)
            except Exception as error: summary["finalTelemetryStop"]=str(error) if type(error) is ValueError else type(error).__name__
            try:
                after=post_files()
                summary["fileSnapshotsAfter"]=after; summary["filesAndSymlinkUnchanged"]=after==snapshots
                summary["settingsAfter"]=identity(settings.read_bytes()); summary["settingsUnchanged"]=settings.read_bytes()==before
                if after!=snapshots or not summary["settingsUnchanged"]: summary["batchComplete"]=False
            except Exception: summary["postStateUnverified"]=True; summary["batchComplete"]=False
        try: shutil.rmtree(batch); summary["disposableRootsRemoved"]=not batch.exists()
        except Exception: summary["disposableRootsRemoved"]=False
        if not summary["disposableRootsRemoved"] or summary.get("foregroundStopped") is False: summary["batchComplete"]=False
        summary["elapsedSeconds"]=round(time.monotonic()-started,3)
        try: save(args.output/"summary.json",summary)
        except ValueError:
            summary={"outcome":"unverified","stopReason":"Retained evidence overflow","liveCliAttempts":summary["liveCliAttempts"],"observedChatSpans":summary["observedChatSpans"],"disposableRootsRemoved":summary["disposableRootsRemoved"]}; save(args.output/"summary.json",summary)
        print(json.dumps({k:summary.get(k) for k in ["outcome","liveCliAttempts","observedChatSpans","stopReason","disposableRootsRemoved"]}))

if __name__=="__main__": main()
