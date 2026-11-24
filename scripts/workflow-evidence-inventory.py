#!/usr/bin/env python3
"""Report supplied packet references; never adjudicate evidence or replay policy."""
import argparse
import hashlib
import json
from pathlib import Path
import sys

FIELDS = (
    "rawCapture", "capturedCandidates", "observationWindow", "repositorySnapshot",
    "priorRecommendationState", "evaluationInstant", "adjudication", "partition",
)


def unique_fields(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate field")
        result[key] = value
    return result


def reject_constant(_value):
    raise ValueError("non-finite JSON number")


def inventory(raw):
    packet = json.loads(raw, object_pairs_hook=unique_fields, parse_constant=reject_constant)
    if (not isinstance(packet, dict) or type(packet.get("schemaVersion")) is not int
            or packet["schemaVersion"] != 1 or not isinstance(packet.get("cases"), list)
            or len(packet["cases"]) > 24):
        raise ValueError("invalid packet")
    seen, sources, cases = set(), set(), []
    supplied = dict.fromkeys(FIELDS, 0)
    for case in packet["cases"]:
        if not isinstance(case, dict):
            raise ValueError("invalid case")
        identity, groups = case.get("id"), case.get("sourceGroups")
        if (not isinstance(identity, str) or not identity.strip() or identity in seen
                or not isinstance(groups, list)
                or any(not isinstance(group, str) or not group.strip() for group in groups)):
            raise ValueError("invalid identity or sources")
        seen.add(identity)
        sources.update(groups)
        missing = []
        for field in FIELDS:
            # A reference being supplied says nothing about its truth, scope,
            # independence or completeness. Those require human/source review.
            if isinstance(case.get(field), dict) and case[field]:
                supplied[field] += 1
            else:
                missing.append(field)
        cases.append({"id": identity, "missingFields": missing})
    return {
        "schemaVersion": 1,
        "inputSha256": hashlib.sha256(raw).hexdigest(),
        "candidateSnapshots": len(cases),
        "sourceGroups": len(sources),
        "incidentCount": "unknown",
        "suppliedFields": supplied,
        "semanticQuality": "not-assessed",
        "policyReplay": "not-run",
        "cases": cases,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("packet", type=Path)
    args = parser.parse_args()
    try:
        with args.packet.open("rb") as source:
            raw = source.read(1024 * 1024 + 1)
        if len(raw) > 1024 * 1024:
            raise ValueError("packet too large")
        report = inventory(raw)
    except (OSError, ValueError, UnicodeError, RecursionError):
        print("Evidence packet invalid.", file=sys.stderr)
        return 2
    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    sys.exit(main())
