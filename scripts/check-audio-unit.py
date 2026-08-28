#!/usr/bin/env python3
"""Hold the Musa Audio Unit to its own findings.

`scripts/check-audio-unit.sh` builds, signs, registers and measures the
component; this is what turns the measurement into a gate. Every claim the
boundary in `docs/rules/across-stages/06-daw-boundary.md` §3-§7 depends on has
to be present and has to have passed, and a claim this machine cannot answer
has to be named here as one, so that "unsupported" can never quietly stand in
for "works".
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

# Every claim the component must make. A report missing one of these is a
# report that stopped early.
REQUIRED = [
    # The component and the library are one build.
    "abi.version",
    "rt.probe",
    # Nothing selected yet.
    "render.silenceBeforeReady",
    "state.saysWhyItIsSilent",
    # One part of one piece, named exactly.
    "prepare.selectsAPart",
    "prepare.statesItsIdentity",
    "prepare.statesWhatItBinds",
    # Rendering, and the laws it must keep.
    "render.sounds",
    "render.blockPartition",
    "render.noncontiguousSampleTime",
    "render.hostSuppliesNoMemory",
    "render.offline",
    "render.reset",
    "render.tail",
    "render.negativeEventOffset",
    "render.unsupportedMessage",
    "render.formatChange",
    "bus.count",
    "param.tree.empty",
    # The render thread.
    "rt.allocation.baseline",
    "rt.allocation.silence",
    "rt.allocation.render",
    "rt.allocation.events",
    # The saved document.
    "state.keepsHostClassInfo",
    "state.isPropertyListSafe",
    "state.noAssetBytes",
    "state.roundTrip",
    "state.refusesForeignClosure",
    "state.refusesAFutureVersion",
    "state.survivesNonsense",
    # The system.
    "discovery.instrument",
    "instantiate.outOfProcess",
    "state.crossesTheProcessBoundary",
    "instantiate.relaunch",
    "instantiate.afterTermination",
]

# Claims this machine cannot answer, with the reason each is allowed to stand
# unanswered. Nothing else may be `unsupported`.
MAY_BE_UNSUPPORTED: dict[str, str] = {}


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: check-audio-unit.py <report.json> <auval-ok>", file=sys.stderr)
        return 2
    path = Path(sys.argv[1])
    auval_ok = sys.argv[2] == "1"
    if not path.exists() or not path.read_text().strip():
        print(f"no report at {path}", file=sys.stderr)
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

    # An allocation number means nothing without the baseline it is published
    # beside: a rig that allocates on its own account cannot say a component
    # does not. Prompt 215 §2 found two artefacts before it found the truth.
    baseline = findings.get("rt.allocation.baseline", {}).get("numbers", {}).get("allocations")
    if baseline not in (0, 0.0):
        problems.append(f"rt.allocation.baseline: the harness itself allocated {baseline}; no number below it counts")

    if not auval_ok:
        problems.append("auval rejected the component; see the auval log beside the report")

    if problems:
        print("\nthe Musa Audio Unit did not hold:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1

    passed = sum(1 for finding in findings.values() if finding["outcome"] == "pass")
    print(f"\n{passed} findings passed, auval validated aumu musa Musa")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
