"""Read-only, version-specific inspection; never imports or executes Copilot."""
import argparse
import hashlib
import io
import json
import struct
import tarfile
from pathlib import Path

EXPECTED = "3ae21a3f00fcc216faaa1f062ee98451c7807066ee26f0fe1c85c6ed315e5458"
EVENTS = ["SystemMessageEvent", "SkillInvokedEvent", "SkillInvokedRefEvent",
          "SkillContextDeliveredEvent", "SkillContextDeliveredRefEvent",
          "SkillsLoadedEvent", "ToolsUpdatedEvent"]
MEMBERS = ["package.json", "index.js", "app.js", "schemas/api.schema.json",
           "schemas/session-events.schema.json", "prebuilds/darwin-arm64/runtime.node"]


def digest(data):
    return {"bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def closure(document, roots):
    selected = {}

    def visit(value):
        if isinstance(value, dict):
            reference = value.get("$ref")
            if reference:
                assert reference.startswith("#/definitions/"), reference
                name = reference.removeprefix("#/definitions/")
                if name not in selected:
                    selected[name] = document["definitions"][name]
                    visit(selected[name])
            for child in value.values():
                visit(child)
        elif isinstance(value, list):
            for child in value:
                visit(child)

    visit(roots)
    return selected


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--native", required=True, type=Path)
    parser.add_argument("--cache", required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    binary = args.native.read_bytes()
    assert hashlib.sha256(binary).hexdigest() == EXPECTED, "Native identity changed"
    assert struct.unpack_from("<I", binary)[0] == 0xFEEDFACF, "Expected Mach-O 64"
    command_count = struct.unpack_from("<I", binary, 16)[0]
    position = 32
    sections = []
    for _ in range(command_count):
        command, size = struct.unpack_from("<II", binary, position)
        if command == 0x19:
            count = struct.unpack_from("<I", binary, position + 64)[0]
            for i in range(count):
                start = position + 72 + i * 80
                name = binary[start:start + 16].rstrip(b"\0")
                if name == b"__NODE_SEA_BLOB":
                    length = struct.unpack_from("<Q", binary, start + 40)[0]
                    offset = struct.unpack_from("<I", binary, start + 48)[0]
                    sections.append((offset, length))
        position += size
    assert len(sections) == 1, "Expected one SEA section"
    offset, length = sections[0]
    blob = binary[offset:offset + length]
    magic, flags = struct.unpack_from("<II", blob)
    assert magic == 0x143DA20 and flags == 0x19, "Unexpected SEA format"
    cursor = 9  # uint32 magic, uint32 flags, uint8 execArgv extension

    def number():
        nonlocal cursor
        value = struct.unpack_from("<Q", blob, cursor)[0]
        cursor += 8
        return value

    def field():
        nonlocal cursor
        size = number()
        value = blob[cursor:cursor + size]
        assert len(value) == size
        cursor += size
        return value

    code_path, loader = field(), field()
    assert code_path == b"sea-loader.js"
    assets = {}
    for _ in range(number()):
        name = field().decode("utf-8")
        assets[name] = field()
    exec_argv = [field().decode("utf-8") for _ in range(number())]
    assert cursor == len(blob), "Unparsed SEA bytes"
    archive = assets["copilot.tgz"]
    with tarfile.open(fileobj=io.BytesIO(archive), mode="r:gz") as package:
        files = {name: package.extractfile("package/" + name).read() for name in MEMBERS}
    assert json.loads(files["package.json"])["version"] == "1.0.90"
    identities = []
    for name, content in files.items():
        cache_file = args.cache / name
        identities.append({"path": "package/" + name, **digest(content),
                           "cacheMatches": cache_file.read_bytes() == content})
    events = json.loads(files["schemas/session-events.schema.json"])
    api = json.loads(files["schemas/api.schema.json"])
    selected_events = {"$schema": events["$schema"],
                       "anyOf": [{"$ref": "#/definitions/" + name} for name in EVENTS]}
    selected_events["definitions"] = closure(events, selected_events["anyOf"])
    methods = {name: api["session"]["tools"][name]
               for name in ["initializeAndValidate", "getCurrentMetadata"]}
    selected_api = {"methods": methods, "definitions": closure(api, methods)}
    app = files["app.js"].decode("utf-8")
    start = app.index("var YLr=new Set(")
    end = app.index("function KWt", start)
    excerpts = [{"purpose": "JSON output writer exclusion set and write path",
                 "startUtf8Byte": len(app[:start].encode("utf-8")),
                 "text": app[start:end]}]
    for term in ['xt=oe==="json"', 'xt&&M2r(e,re??!1,Rt,process.stdout)',
                 'function M2r(', 'tools=this.nativeDomain("session.tools.")']:
        start = app.index(term)
        excerpts.append({"purpose": term, "startUtf8Byte": len(app[:start].encode("utf-8")),
                         "text": app[start:start + 350]})
    result = {"native": digest(binary), "seaSection": {"offset": offset, "bytes": length},
              "seaLoader": digest(loader), "seaExecArgv": exec_argv,
              "asset": {"name": "copilot.tgz", **digest(archive)}, "selectedMembers": identities}
    loader_text = loader.decode("utf-8")
    loader_excerpts = []
    for term in ["function Qa(", "function eh(", "var cf=", "COPILOT_CLI_DIST_DIR",
                 "COPILOT_CLI_RESOLVED_DIST_DIR"]:
        start = loader_text.index(term)
        loader_excerpts.append({"purpose": term,
                                "startUtf8Byte": len(loader_text[:start].encode("utf-8")),
                                "text": loader_text[start:start + 700]})
    args.output.mkdir(parents=True, exist_ok=True)
    for name, value in [("runtime-identity.json", result), ("events-selected.schema.json", selected_events),
                        ("tools-selected.schema.json", selected_api), ("json-output-excerpts.json", excerpts),
                        ("sea-loader-excerpts.json", loader_excerpts)]:
        (args.output / name).write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({"nativeMatches": True, "cacheMatches": all(x["cacheMatches"] for x in identities),
                      "files": len(identities), "output": str(args.output)}))


if __name__ == "__main__":
    main()
