#!/usr/bin/env python3
"""Hold the Audio Unit trial to its own findings.

The trial's report is a list of claims, each measured rather than asserted.
This script is what makes the report a gate: every claim the boundary depends
on has to be present and has to have passed, and a claim the platform will not
answer has to be named here as one, so that "unsupported" can never quietly
stand in for "works".
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Every claim the trial must make. A report missing one of these is a report
# that stopped early.
REQUIRED = [
    "probe.available",
    "abi.version",
    "instantiate.inProcess",
    "render.determinism",
    "render.blockPartition",
    "render.noncontiguousSampleTime",
    "render.hostSuppliesNoMemory",
    "render.reset",
    "render.offline",
    "render.maximumFrames",
    "param.tree.projection",
    "param.ramp",
    "param.tree.replacement",
    "bus.count",
    "bus.multipleOutputs",
    "state.roundTrip",
    "state.documentCarriesClosure",
    "state.keepsHostClassInfo",
    "state.noAssetBytes",
    "state.refusesForeignClosure",
    "rt.allocation.hostBaseline",
    "rt.allocation",
    "rt.allocation.events",
    "rt.timing",
    "processor.instantiate",
    "processor.schedule.randomAccess",
    "processor.seek.doesNotReplay",
    "processor.emitsHostMIDI",
    "processor.transportStopped",
    "processor.missingContext",
    "discovery.instrument",
    "discovery.processor",
    "instantiate.outOfProcess",
    "instantiate.relaunch",
    "state.crossesTheProcessBoundary",
    "instantiate.afterTermination",
    "sandbox.extensionNamesAssetStore",
]

# Claims this machine cannot answer, with the reason each is allowed to stand
# unanswered. Nothing else may be `unsupported`.
MAY_BE_UNSUPPORTED = {
    "sandbox.appGroup": "an App Group container needs an entitlement signed by a Developer Team",
    "sandbox.extensionReadsAsset": "an ad-hoc-signed extension is taken down when it reads inside the container; a real App Group identifier is team-prefixed and provisioned by Apple",
}


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: check-audio-unit-trial.py <report.json> <auval-ok>", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    auval_ok = sys.argv[2] == "1"
    if not path.exists() or not path.read_text().strip():
        print(f"no trial report at {path}", file=sys.stderr)
        return 1

    report = json.loads(path.read_text())
    findings = {entry["id"]: entry for entry in report["findings"]}
    problems: list[str] = []

    for name, value in sorted(report.get("environment", {}).items()):
        print(f"  {name}: {value}")

    for identifier in REQUIRED:
        finding = findings.get(identifier)
        if finding is None:
            problems.append(f"{identifier}: the report does not contain this finding")
            continue
        if finding["outcome"] == "pass":
            continue
        if finding["outcome"] == "unsupported" and identifier in MAY_BE_UNSUPPORTED:
            print(f"  unsupported {identifier} — {finding['detail']}")
            continue
        problems.append(f"{identifier}: {finding['outcome']} — {finding['detail']}")

    for identifier, finding in sorted(findings.items()):
        if identifier in REQUIRED or identifier in MAY_BE_UNSUPPORTED:
            continue
        if finding["outcome"] == "fail":
            problems.append(f"{identifier}: fail — {finding['detail']}")

    for identifier, reason in MAY_BE_UNSUPPORTED.items():
        finding = findings.get(identifier)
        if finding is None:
            problems.append(f"{identifier}: the report does not contain this finding")
        elif finding["outcome"] == "fail":
            problems.append(f"{identifier}: fail — {finding['detail']} (expected pass or unsupported: {reason})")

    if not auval_ok:
        problems.append("auval rejected one of the components; see the auval logs beside the report")

    if problems:
        print("\nthe Audio Unit trial did not hold:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1

    passed = sum(1 for finding in findings.values() if finding["outcome"] == "pass")
    print(f"\n{passed} findings passed, auval validated both components")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
