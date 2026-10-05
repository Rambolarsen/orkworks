"""Static, pinned-file inspection only; never imports or starts Copilot."""
import argparse
import hashlib
import json
from pathlib import Path

IDENTITIES = {
    "runtime": "ad89ad135478cd2b19a5b99bb19064d0be281961cab0506f3767f39234d4c591",
    "events": "c1b43f27c015b87bbb74072c6591227e860d6b1862aed9edecfe5aacd29a16c3",
    "api": "749f184068021a6a05b0d8234ecb8cb4d392a5b9b044c87f619bde374eab928d",
}


def inspect(inputs):
    for name, data in inputs.items():
        if hashlib.sha256(data).hexdigest() != IDENTITIES[name]:
            raise ValueError("Pinned identity changed: " + name)
    runtime = inputs["runtime"]
    decoder = json.JSONDecoder()
    marker = b'[{"path":"mouse"'
    offset = runtime.index(marker)
    catalog, _ = decoder.raw_decode(runtime[offset:offset + 100000].decode("utf-8", "replace"))
    marker = b'"telemetry":{"canonicalKeys"'
    keys_offset = runtime.index(marker) + len(b'"telemetry":')
    keys, _ = decoder.raw_decode(runtime[keys_offset:keys_offset + 10000].decode("utf-8", "replace"))
    capture = {"sourceSha256": IDENTITIES["runtime"], "catalogStartByte": offset,
               "entries": [entry for entry in catalog if entry["path"].startswith("telemetry.")],
               "canonicalKeysStartByte": keys_offset, "canonicalKeys": keys}
    events = json.loads(inputs["events"])
    names = ["ModelCallStartData", "ModelCallFinishedData", "ModelCallFinalResultData"]
    selected = {}

    def visit(name):
        if name in selected:
            return
        selected[name] = events["definitions"][name]
        def walk(value):
            if isinstance(value, dict):
                if "$ref" in value:
                    ref = value["$ref"]
                    if not ref.startswith("#/definitions/"):
                        raise ValueError("Unexpected schema reference")
                    visit(ref[len("#/definitions/"):])
                for child in value.values():
                    walk(child)
            elif isinstance(value, list):
                for child in value:
                    walk(child)
        walk(selected[name])
    for name in names:
        visit(name)
    model_calls = {"$schema": events["$schema"], "anyOf": [
        {"$ref": "#/definitions/" + name} for name in names], "definitions": selected}
    api = json.loads(inputs["api"])
    options = api["definitions"]["SessionOpenOptions"]
    observed = {"kind": "schema-projection-not-a-complete-schema",
                "sessionOpenOptionNames": list(options["properties"]),
                "selectedSessionOpenOptions": {k: options["properties"][k] for k in [
                    "reasoningSummary", "trajectoryFile", "eventsLogDirectory", "eventsLogIncludesSubagents"]},
                "eventLogRead": api["session"]["eventLog"]["read"],
                "userSettingsGetResult": api["definitions"]["UserSettingsGetResult"]}
    return {"capture-settings.json": capture, "model-call-data.schema.json": model_calls,
            "api-observations.json": observed}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in IDENTITIES:
        parser.add_argument("--" + name, required=True, type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    result = inspect({name: getattr(args, name).read_bytes() for name in IDENTITIES})
    args.output.mkdir(parents=True, exist_ok=True)
    for name, value in result.items():
        (args.output / name).write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n")
    print("Pinned identities matched; wrote three static inspection artifacts")


if __name__ == "__main__":
    main()
