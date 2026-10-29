#!/usr/bin/env python3
"""Independent stdlib budget vectors; no runtime/native eligibility."""
import hashlib
import json
from pathlib import Path

HERE = Path(__file__).resolve().parent

def canonical(value):
    return json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False)

def size(value):
    return len(canonical(value).encode("utf-8"))

def sha(raw):
    return hashlib.sha256(raw).hexdigest()

LIMITS = dict(runPreparationBytes=8388608, workspacePreparationBytes=134217728,
              workspaceRunCount=32, proofBytes=67108864, proofCount=4096,
              logicalBytes=536870912, logicalRows=8192)
PREP = {"root_drafts", "root_preparations", "run_states", "parent_attachments", "preparation_artifacts"}

def measure(state):
    totals = dict.fromkeys(LIMITS, 0)
    runs = {}
    for row in state["rows"]:
        key, value = json.loads(row["keyCanonical"]), json.loads(row["valueCanonical"])
        full = len(row["keyCanonical"].encode()) + len(row["valueCanonical"].encode())
        totals["logicalBytes"] += full
        totals["logicalRows"] += 1
        if row["table"] == "commit_proof":
            totals["proofBytes"] += full
            totals["proofCount"] += 1
        if row["table"] not in PREP:
            continue
        owner = value.get("draft", value.get("bootstrap", value.get("state", value.get("attachment", key))))
        identity = (owner["workspaceId"], owner["runId"])
        del value["lastCommit"]
        if row["table"] == "root_preparations":
            del value["receipt"]["commit"]
        charge = size(key) + size(value)
        runs[identity] = runs.get(identity, 0) + charge
        totals["workspacePreparationBytes"] += charge
    totals["runPreparationBytes"] = max(runs.values(), default=0)
    totals["workspaceRunCount"] = len(runs)
    return dict(name=state["name"], usage={k: str(v) for k, v in totals.items()},
                runs=[dict(workspaceId=w, runId=r, bytes=str(n)) for (w, r), n in sorted(runs.items(), key=lambda item: canonical(dict(workspaceId=item[0][0], runId=item[0][1])))])

def boundaries():
    out = []
    for field, limit in LIMITS.items():
        for delta in (-1, 0, 1):
            usage = {k: "0" for k in LIMITS}
            usage[field] = str(limit + delta)
            out.append(dict(name=f"{field}:{delta:+}", usage=usage, exceeded=[field] if delta > 0 else []))
    usage = {k: str(v + 1) for k, v in LIMITS.items()}
    out.append(dict(name="all-independent-overflows", usage=usage, exceeded=list(LIMITS)))
    return out

def envelopes():
    # Conservative per-key maxima and actual wrapper encoding, not source-valid values.
    workspace, run, reference = "w" * 128, "e" * 64 + "-" + "9" * 19, "r" * 128
    link = dict(storageEpoch="e" * 64, commitSequence="9" * 19, changeDigest="d" * 64)
    artifact_key = dict(workspaceId=workspace, runId=run, referenceId=reference)
    empty = dict(schemaVersion=1, contentBase64="", sizeBytes="1048576", contentDigest="d" * 64, lastCommit=link)
    full_overhead = size(empty)
    del empty["lastCommit"]
    projected_overhead = size(empty)
    # Fifteen one-byte refs plus the remaining bytes: each length is 1 mod 3,
    # maximizing Base64 padding across 16 separate occurrences of a 1 MiB sum.
    lengths = [1] * 15 + [1048576 - 15]
    encoded = sum(4 * ((n + 2) // 3) for n in lengths)
    source_key = dict(workspaceId=workspace, runId=run)
    # Full source envelope is an upper bound on the smaller projected source.
    admitted = size(source_key) + 1081344 + size(source_key) + 4096 + size("p" * 128) + 4096
    admitted += 16 * (size(artifact_key) + projected_overhead) + encoded
    draft_wrapper = size(dict(schemaVersion=1, draft=None)) - len("null")
    pending = size(workspace) + draft_wrapper + 1081344 + 16 * (size(artifact_key) + projected_overhead) + encoded
    identity = dict(kind="root", workspaceId=workspace, runId=run, rootAssignmentId="a" * 128,
                    parentSessionId="p" * 128, bootstrapId="b" * 128, bootstrapConfigurationDigest="d" * 64)
    # Repeated maximal entries bound bytes; duplicate keys make this an envelope
    # calculation only, never a valid manifest or a reachable initial command.
    keys = [("root_drafts", workspace), ("root_preparations", source_key),
            ("run_states", source_key), ("parent_attachments", "p" * 128),
            ("preparation_artifacts", artifact_key), ("summary_state", identity)]
    workspace_owner = dict(kind="workspace", workspaceId=workspace)
    run_owner = dict(kind="run", **source_key)
    assignment_owner = dict(kind="assignment", assignmentIdentity=identity)
    for resource in ["reporting-receipt-bytes", "assignment-record-bytes", "result-payload-bytes"]:
        keys.extend(("quota_usage", dict(owner=owner, resource=resource))
                    for owner in [workspace_owner, run_owner, assignment_owner])
    for resource in ["summary-row-bytes", "summary-row-count", "summary-lineage-bytes", "summary-pending-count", "preparation-run-count"]:
        keys.append(("quota_usage", dict(owner=workspace_owner, resource=resource)))
    keys.extend(("quota_usage", dict(owner=owner, resource="preparation-retained-bytes"))
                for owner in [workspace_owner, run_owner])
    candidates = [dict(table=table, key=key, beforeDigest="a" * 64, afterDigest="b" * 64)
                  for table, key in keys]
    change = max(candidates, key=size)
    manifest = dict(schemaVersion=3, workspaceId=workspace, storageEpoch="e" * 64,
                    commitSequence="9" * 19, predecessor=link, commandKind="subject-preparation", changes=[change] * 4096)
    value = dict(body=manifest, changeDigest="d" * 64)
    return dict(classification="conservative-envelope-bounds-not-source-valid",
                rawLengths=lengths, encodedArtifactBytes=encoded, artifactKeyBytes=size(artifact_key),
                artifactFullOverheadBytes=full_overhead, artifactProjectedOverheadBytes=projected_overhead,
                sourceKeyBytes=size(source_key), parentKeyBytes=size("p" * 128), workspaceKeyBytes=size(workspace),
                draftWrapperBytes=draft_wrapper, admittedPreparationUpperBytes=admitted, pendingPreparationUpperBytes=pending,
                largestChangeTable=change["table"], largestChangeKeyCanonical=canonical(change["key"]),
                largestChangeBytes=size(change),
                proofValueBytes=size(value), proofValueSha256=sha(canonical(value).encode()),
                proofKeyBytes=size("9" * 19), maximumManifestEntries=4096,
                minimumChangedRowsWithControls=4098)

def main():
    raw = (HERE / "root-preparation-effects.json").read_bytes()
    effects = json.loads(raw)
    catalog = dict(schemaVersion=1, classification="offline-budget-components-not-runtime-admission",
                   effectsCatalogSha256=sha(raw), limits={k: str(v) for k, v in LIMITS.items()},
                   states=[measure(s) for s in effects["states"]], arithmetic=boundaries(), envelopes=envelopes())
    print(json.dumps(catalog, ensure_ascii=False, indent=2))

if __name__ == "__main__":
    main()
