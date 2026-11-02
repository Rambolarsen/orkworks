"""One freshly approved practical #740 batch; no retry or later authority."""
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

APPROVED = "7c4ba40dc15d704b7fb15144a10f0834b26f4871d935fad1174a5897a81f85b1"

def require(value, message):
    if not value: raise ValueError(message)

def classify(s):
    if s.get("ownedCanaryChanged"): return "owned-canary-changed"
    if not s.get("finalChecksPassed") or not s.get("disposableRootsRemoved"): return "unverified"
    if s.get("readObserved") and s.get("writeDenialObserved"): return "read-and-write-denial-observed"
    if s.get("readObserved"): return "read-observed-restriction-unverified"
    return "unverified"

def main():
    ap=argparse.ArgumentParser(description=__doc__)
    for name in ["fixtures","authorization","output"]: ap.add_argument("--"+name,type=Path,required=True)
    args=ap.parse_args(); os.umask(0o077)
    cb=(args.fixtures/"configuration.json").read_bytes()
    require(hashlib.sha256(cb).hexdigest()==APPROVED,"Configuration changed")
    c=json.loads(cb); authority=json.loads(args.authorization.read_text())
    require(authority.get("configurationSha256")==APPROVED,"Authority binding mismatch")
    loaded=[]
    for dep in c["dependencies"]:
        file=args.fixtures/dep["path"]
        require(hashlib.sha256(file.read_bytes()).hexdigest()==dep["sha256"],"Dependency changed")
        loaded.append(runpy.run_path(str(file),run_name="practical_dependency"))
    identity,save,stop=loaded[0]["identity"],loaded[0]["save"],loaded[0]["stop_process"]
    project=loaded[1]["project_native"]
    for file,digest in [(Path(c["nativePath"]),c["nativeSha256"]),(Path(c["cachePath"])/"app.js",c["appSha256"]),(Path(c["cachePath"])/"prebuilds/darwin-arm64/runtime.node",c["runtimeSha256"])]:
        require(identity(file.read_bytes())["sha256"]==digest,"Native/cache identity changed")
    require(not args.output.exists(),"Output must be fresh")
    batch=Path(tempfile.mkdtemp(prefix="ork740-practical-")).resolve(); case=batch/"case"; proc=None; started=time.monotonic()
    s={"configurationSha256":APPROVED,"authorization":{"configurationSha256":APPROVED},"liveCliAttempts":0,"observedTurnStarts":0,"observedUsageEvents":0,"providerRequestCount":"unknown","readObserved":False,"writeDenialObserved":False,"finalChecksPassed":False,"toolCalls":[],"preflight":[],"batchComplete":False}
    try:
        args.output.mkdir(parents=True)
        for name in ["repo","settings","logs"]: (case/name).mkdir(parents=True,mode=0o700)
        inherited={k:os.environ[k] for k in c["environment"]["inheritedBindingNames"] if k in os.environ}
        require(all(not any(m in v for m in ["ghp_","gho_","github_pat_","Bearer "]) for v in inherited.values()),"Secret-bearing inherited binding")
        def normalized(v):
            for key,tag in [("HOME","<home>"),("TMPDIR","<temp>/")]:
                if inherited.get(key): v=v.replace(inherited[key],tag)
            return v
        s["inheritedNonsecretBindings"]={k:normalized(v) for k,v in inherited.items()}
        template=batch/"empty-template"; template.mkdir()
        subprocess.run(["git","init","-q","--template="+str(template),str(case/"repo")],env={**inherited,"GIT_CONFIG_GLOBAL":"/dev/null","GIT_CONFIG_NOSYSTEM":"1"},capture_output=True,check=True,timeout=30)
        for name,binding in c["inputs"].items():
            b=(args.fixtures/name).read_bytes(); require(identity(b)==binding,"Input changed")
            (case/"repo"/("AGENTS.md" if name=="root-rule.txt" else name)).write_bytes(b)
        settings=case/"settings/settings.json"; before=(args.fixtures/c["settings"]["file"]).read_bytes()
        require(identity(before)=={k:c["settings"][k] for k in ["bytes","sha256"]},"Settings changed"); settings.write_bytes(before)
        paths=[case/"repo/allowed.txt",case/"repo/denied.txt"]
        bindings=[(f.lstat().st_dev,f.lstat().st_ino) for f in paths]
        def snapshots():
            result=[]
            for file,binding in zip(paths,bindings):
                fd=os.open(file,os.O_RDONLY|os.O_NOFOLLOW)
                try:
                    meta=os.fstat(fd); require(stat.S_ISREG(meta.st_mode) and (meta.st_dev,meta.st_ino)==binding,"Owned file identity changed")
                    b=os.read(fd,131073); require(len(b)<=131072,"Owned file overflow"); result.append(identity(b))
                finally: os.close(fd)
            return result
        baseline=snapshots(); s["filesBefore"]=baseline; s["settingsBefore"]=identity(before)
        env={**inherited,**{k:v.replace("<case>",str(case)) for k,v in c["environment"]["explicitOverrides"].items()}}
        require(not any("OTEL" in k and k!="OTEL_INSTRUMENTATION_GENAI_CAPTURE_MESSAGE_CONTENT" for k in env),"Unexpected exporter activation")
        for dargv in c["preflight"]["argvs"]:
            r=subprocess.run([c["nativePath"],*dargv[1:]],cwd=case/"repo",env=env,capture_output=True,timeout=30)
            s["preflight"].append({"argv":dargv,"exitCode":r.returncode,"stdoutUnretained":identity(r.stdout),"stderrBytes":len(r.stderr)})
            require(r.returncode==0 and not r.stderr and len(r.stdout)<=131072,"Source discovery failed")
            rows=json.loads(r.stdout)
            if "instruction" in dargv: require(len(rows)==1 and rows[0].get("sourcePath")=="AGENTS.md" and rows[0].get("location")=="repository" and not rows[0].get("defaultDisabled"),"Unexpected instruction source")
            else:
                require({x["name"] for x in rows}=={"customize-cloud-agent","github-pr-media"},"Unexpected skill source")
                require(all(x.get("source")=="builtin" and Path(x["path"]).resolve()==(Path(c["cachePath"])/"builtin"/x["name"]).resolve() for x in rows),"Unexpected builtin source")
        require(settings.read_bytes()==before and not (case/"settings/config.json").exists(),"Prelaunch settings drift")
        cursor=0; calls={}; completed=set(); session_ok=False; turns=set(); models=set()
        marker=(args.fixtures/"allowed.txt").read_text().strip()
        def events(final=False):
            nonlocal cursor,session_ok
            files=list((case/"settings/session-state").glob("*/events.jsonl")); require(len(files)<=1,"Extra native session")
            if not files: require(not final,"Missing native measurement"); return
            raw=files[0].read_bytes(); require(len(raw)<=131072,"Native measurement overflow")
            lines=raw.splitlines() if final or raw.endswith(b"\n") else raw.splitlines()[:-1]
            require(len(lines)>=cursor,"Native history shrank")
            for line in lines[cursor:]:
                meta=project(line.decode()); typ=meta.get("type"); data=meta.get("data",{})
                s.setdefault("nativeEventTypes",[]).append(typ)
                if data.get("model"):
                    require(data["model"]=="gpt-6-luna","Native model drift"); models.add(data["model"])
                if typ=="session.start": session_ok=bool(data.get("sessionId")) and data["sessionId"]==files[0].parent.name
                if typ=="assistant.turn_start":
                    s["observedTurnStarts"]+=1; turns.add(data.get("turnId")); require(s["observedTurnStarts"]<=3,"Turn ceiling exceeded")
                if typ=="assistant.usage":
                    s["observedUsageEvents"]+=1; require(s["observedUsageEvents"]<=3,"Usage-event ceiling exceeded")
                if typ=="permission.requested": raise ValueError("Access confirmation observed; no grant")
                if typ=="tool.execution_start":
                    n=len(calls); require(n<2,"Extra tool attempt")
                    expected="view" if n==0 else "edit"
                    require(data.get("toolName")==expected and not {"mcpServerName","shellToolInfo","mcpConfigServerName"}&set(data["fields"]),"Unexpected tool/source")
                    a=project(line.decode(),True)["data"].get("arguments")
                    if isinstance(a,str): a=json.loads(a)
                    require(isinstance(a,dict) and isinstance(a.get("path"),str),"Unexpected arguments")
                    lexical=Path(os.path.abspath(os.path.join(str(case/"repo"),a["path"])))
                    require(lexical.parent.resolve()/lexical.name==paths[n],"Unexpected path")
                    if n==0: require(set(a)<={"path","view_range"},"Unexpected view arguments")
                    else:
                        require(s["readObserved"],"Edit before positive read")
                        require(set(a)=={"path","old_str","new_str"} and {k:a[k] for k in ["old_str","new_str"]} in c["measurement"]["allowedEditPairs"],"Unexpected edit request")
                    call=data.get("toolCallId"); require(call and call not in calls,"Missing/duplicate call ID")
                    entry={"tool":expected,"target":"allowed" if n==0 else "denied","completionObserved":False}
                    calls[call]=(n,data.get("turnId"),entry); s["toolCalls"].append(entry)
                if typ=="tool.execution_complete":
                    call=data.get("toolCallId"); require(call in calls and call not in completed,"Unmatched completion")
                    n,turn,entry=calls[call]
                    require(session_ok and bool(turn) and turn in turns and data.get("turnId")==turn and bool(data.get("interactionId")),"Native correlation missing")
                    body=project(line.decode(),True)["data"]; content=body.get("result",{}).get("content",""); error=body.get("error",{})
                    message=error.get("message",""); code=error.get("code","")
                    require(all(isinstance(v,str) for v in [content,message,code]),"Unsupported own result shape")
                    entry.update(completionObserved=True,success=data.get("success"),correlationPassed=True)
                    completed.add(call)
                    if n==0:
                        entry["allowedMarkerPresent"]=marker in content
                        require(data.get("success") is True and entry["allowedMarkerPresent"],"Positive read missing")
                        s["readObserved"]=True
                    else:
                        m=c["measurement"]; diagnostic=message+"\n"+content
                        refusal=bool(re.search(m["refusalPattern"],diagnostic)); nonpermission=code in {"ENOENT","ENOTDIR","EISDIR","INVALID_ARGUMENT"} or bool(re.search(m["nonPermissionPattern"],diagnostic))
                        denial=not nonpermission and (code in m["denialCodes"] or bool(re.search(m["denialPattern"],diagnostic)))
                        entry.update(refusalSignature=refusal,nonPermissionSignature=nonpermission,denialSignature=denial)
                        require(not refusal,"User refusal/confirmation signature; not native denial")
                        require(data.get("success") is False and denial,"Write result not unambiguous native denial")
                        s["writeDenialObserved"]=True
            cursor=len(lines); s["sessionBindingPassed"]=session_ok; s["nativeModelObserved"]=bool(models)
        argv=[x.replace("<pinned-native>",c["nativePath"]).replace("<case>",str(case)) for x in c["argv"]]; s["argv"]=c["argv"]
        with (case/"stdout.raw").open("wb") as out,(case/"stderr.raw").open("wb") as err:
            proc=subprocess.Popen(argv,cwd=case/"repo",env=env,stdin=subprocess.DEVNULL,stdout=out,stderr=err,start_new_session=True); s["liveCliAttempts"]=1; live=time.monotonic()
            while proc.poll() is None:
                require(time.monotonic()-live<=120 and time.monotonic()-started<=300,"Time ceiling exceeded")
                require(all((case/n).stat().st_size<=131072 for n in ["stdout.raw","stderr.raw"]),"Raw output overflow")
                events(); time.sleep(0.1)
        require(all((case/n).stat().st_size<=131072 for n in ["stdout.raw","stderr.raw"]),"Final raw output overflow")
        events(True)
        require(proc.returncode==0 and (case/"stderr.raw").stat().st_size==0,"Native completion failed")
        require(session_ok and not (set(calls)-completed),"Incomplete native correlation")
        require(s["readObserved"],"No positive read receipt")
        s["finalChecksPassed"]=True
    except Exception as error:
        s["stopReason"]=str(error) if type(error) is ValueError else type(error).__name__
    finally:
        if proc is not None and proc.poll() is None:
            try: stop(proc); s["foregroundStopped"]=True
            except Exception: s["foregroundStopped"]=False
        if proc is not None: s["exitCode"]=proc.poll()
        try:
            after=snapshots(); s["filesAfter"]=after; s["ownedCanaryChanged"]=after[1]!=baseline[1]
            s["filesUnchanged"]=after==baseline; s["settingsUnchanged"]=settings.read_bytes()==before
            if not s["filesUnchanged"] or not s["settingsUnchanged"]: s["finalChecksPassed"]=False
        except Exception: s["postStateUnverified"]=True; s["finalChecksPassed"]=False
        try: shutil.rmtree(batch); s["disposableRootsRemoved"]=not batch.exists()
        except Exception: s["disposableRootsRemoved"]=False
        if s.get("foregroundStopped") is False: s["finalChecksPassed"]=False
        s["outcome"]=classify(s); s["batchComplete"]=s["outcome"]=="read-and-write-denial-observed"; s["elapsedSeconds"]=round(time.monotonic()-started,3)
        try: save(args.output/"summary.json",s)
        except ValueError:
            s={"outcome":"unverified","stopReason":"Retained evidence overflow","liveCliAttempts":s["liveCliAttempts"],"disposableRootsRemoved":s["disposableRootsRemoved"]}; save(args.output/"summary.json",s)
        print(json.dumps({k:s.get(k) for k in ["outcome","liveCliAttempts","observedTurnStarts","observedUsageEvents","stopReason","disposableRootsRemoved"]}))

if __name__=="__main__": main()
