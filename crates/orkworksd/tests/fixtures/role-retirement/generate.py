"""Independent stdlib golden calculation from literal synthetic catalog data.
Run from any directory; output must match the checked-in bytes exactly.
"""
import base64
import hashlib
import json
from pathlib import Path

ROOT = Path(__file__).parent

def canonical(value):
    return json.dumps(value, ensure_ascii=False, sort_keys=True, separators=(",", ":")).encode()

def digest(data):
    return hashlib.sha256(data).hexdigest()

provenance = {"schemaVersion": 1, "kind": "nonlocal", "sourceKind": "bundled", "sourceIdentity": "catalog-ø", "sourceRevision": "rev-1"}
role = {"id": "role-review", "version": 1, "role": "review", "instructions": 'Review "café".\r\nKeep \\ paths.\n', "provenance": provenance}
role["instructionsDigest"] = digest(role["instructions"].encode())
role["contentDigest"] = digest(canonical(role))
skill = {"id": "skill-a", "version": "v-é.1", "content": 'Read "雪".\nThen check.\r\n', "provenance": provenance, "resourceIds": []}
skill["contentDigest"] = digest(skill["content"].encode())
resource = {"id": "res-a", "skillId": "skill-a", "sourceReference": "guides/ø.txt", "content": 'Resource\t\\\r\n雪', "provenance": {"schemaVersion": 1, "kind": "local", "sourceKind": "repository", "sourceIdentity": "repo-resource", "sourceRevision": "resource-rev", "sourcePath": {"encoding": "unix-bytes", "base64": base64.b64encode(b"/synthetic/\xff").decode()}}}
resource["contentDigest"] = digest(resource["content"].encode())
expected = {}
for name, kind, snapshot, resources in [
    ("role", "role-template", role, []),
    ("skill", "skill", skill, []),
    ("resource-skill", "skill", {**skill, "resourceIds": ["res-a"]}, [resource]),
]:
    data = canonical({"artifactKind": kind, "snapshot": snapshot, "resources": resources})
    assert (ROOT / (name + ".json")).read_bytes() == data, name
    expected[name] = {"snapshotDigest": digest(b"orkworks.retirement-artifact-snapshot.v1\n" + data), "sourceIdentity": snapshot["provenance"]["sourceIdentity"], "artifactKind": kind, "artifactId": snapshot["id"], "version": snapshot["version"]}
assert (ROOT / "expected.json").read_bytes() == canonical(expected)
print("3 independent canonical payloads, digests and typed keys match")
