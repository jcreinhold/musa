#!/usr/bin/env python3
"""Hold the DAW boundary to the matrix that describes it.

`docs/notes/research/94-the-daw-boundary-closed.md` §3 is one table with a row
for every crossing `docs/rules/across-stages/06-daw-boundary.md` names. This
script is what stops that table from being prose: it parses it, resolves every
row's evidence, and reproduces the `bundle:` claims itself from bundles the
shell script exported with the real CLI.

A row whose test has been renamed, whose harness finding did not pass, or
whose bundle claim no longer holds fails the run. A row with no evidence at
all fails it too, because an empty cell is how a matrix goes quietly stale.
"""

from __future__ import annotations

import hashlib
import json
import re
import struct
import subprocess
import sys
from pathlib import Path

MATRIX = Path("docs/notes/research/94-the-daw-boundary-closed.md")

# The words a host column may use. Anything else is a claim nobody can check.
HOST_WORDS = {
    "documented",
    "documented, after Audio Units are enabled",
    "not documented",
    "not read here",
    "not mentioned in the cited page",
    "n/a",
}
PROTOCOL = re.compile(r"^protocol §\d+\.\d+ \(not run\)$")


class Failure(Exception):
    pass


def rows(text: str) -> list[list[str]]:
    """The matrix's own table, as cells. §3's table and no other."""
    section = text.split("## 3. The conformance matrix", 1)[-1].split("\n## ", 1)[0]
    found = []
    for line in section.splitlines():
        line = line.strip()
        if not line.startswith("|") or set(line) <= set("| -"):
            continue
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 7 or cells[0] == "Claim":
            continue
        found.append(cells)
    return found


def read_wav(path: Path) -> dict:
    """Frames, rate, channels and sample format, from the RIFF header.

    Python's `wave` refuses 32-bit float, which is the only thing Musa writes,
    so the header is read here rather than worked around.
    """
    data = path.read_bytes()
    if data[:4] != b"RIFF" or data[8:12] != b"WAVE":
        raise Failure(f"{path} is not a RIFF WAVE file")
    offset, fmt, frames = 12, None, None
    while offset + 8 <= len(data):
        name = data[offset : offset + 4]
        size = struct.unpack_from("<I", data, offset + 4)[0]
        body = data[offset + 8 : offset + 8 + size]
        if name == b"fmt ":
            tag, channels, rate, _, align, bits = struct.unpack_from("<HHIIHH", body, 0)
            fmt = {"tag": tag, "channels": channels, "rate": rate, "bits": bits, "align": align}
        elif name == b"data" and fmt:
            frames = size // fmt["align"]
        offset += 8 + size + (size % 2)
    if not fmt or frames is None:
        raise Failure(f"{path} has no fmt or data chunk")
    return {**fmt, "frames": frames}


def read_midi(path: Path) -> dict:
    """Format, track count and division, from the SMF header."""
    data = path.read_bytes()
    if data[:4] != b"MThd":
        raise Failure(f"{path} is not a Standard MIDI File")
    fmt, tracks, division = struct.unpack_from(">HHH", data, 8)
    counted = data.count(b"MTrk")
    if counted != tracks:
        raise Failure(f"{path} claims {tracks} tracks and carries {counted}")
    return {"format": fmt, "tracks": tracks, "division": division, "bytes": data}


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


# --- The bundle claims, each named by the matrix -------------------------


def claim_digests(b: dict) -> str:
    manifest = b["logic"]["manifest"]
    for entry in manifest["files"]:
        path = b["logic"]["dir"] / entry["path"]
        if not path.exists():
            raise Failure(f"the manifest names {entry['path']}, which is not there")
        if path.stat().st_size != entry["bytes"]:
            raise Failure(f"{entry['path']} is {path.stat().st_size} bytes, not {entry['bytes']}")
        if digest(path) != entry["sha256"]:
            raise Failure(f"{entry['path']} does not have the digest the manifest claims")
    return f"{len(manifest['files'])} files, each present with the digest it claims"


def claim_identity_is_the_source(b: dict) -> str:
    manifest = b["logic"]["manifest"]
    identity = manifest["identity"]
    if len(identity["music"]) < 16 or len(identity["assets"]) < 16:
        raise Failure("an identity that short is not an identity")
    if identity != b["garageband"]["manifest"]["identity"]:
        raise Failure("two profiles of one source disagree about its identity")
    return f"music {identity['music'][:12]}…, assets {identity['assets'][:12]}…"


def claim_identity_moves_with_the_source(b: dict) -> str:
    before = b["plain"]["manifest"]["identity"]["music"]
    after = b["edited"]["manifest"]["identity"]["music"]
    if before == after:
        raise Failure("an edited source exported under the same identity")
    return f"{before[:12]}… became {after[:12]}… when one note moved"


def claim_deterministic(b: dict) -> str:
    first, again = b["logic"]["dir"], b["again"]["dir"]
    names = sorted(entry["path"] for entry in b["logic"]["manifest"]["files"])
    for name in names:
        if digest(first / name) != digest(again / name):
            raise Failure(f"{name} differs between two exports of one source")
    return f"{len(names)} files identical across two exports"


def claim_no_clock_no_path(b: dict) -> str:
    home = str(Path.home())
    for entry in b["logic"]["manifest"]["files"]:
        text = (b["logic"]["dir"] / entry["path"]).read_bytes()
        if home.encode() in text:
            raise Failure(f"{entry['path']} carries the path it was written from")
    stamps = re.compile(rb"20[2-9][0-9]-[01][0-9]-[0-3][0-9]T")
    for name in ("musa-manifest.json", "musa-origins.json"):
        if stamps.search((b["logic"]["dir"] / name).read_bytes()):
            raise Failure(f"{name} carries a timestamp")
    return "no absolute path and no timestamp in anything written"


def claim_seed_and_profile(b: dict) -> str:
    render = b["logic"]["manifest"]["render"]
    for key in ("sampleRate", "channels", "bitsPerSample", "sampleFormat", "renderSeed", "frames"):
        if key not in render:
            raise Failure(f"the manifest's render record says nothing about {key}")
    if b["logic"]["manifest"]["profile"] != "logic" or b["garageband"]["manifest"]["profile"] != "garageband":
        raise Failure("a manifest does not name the profile it was written for")
    return f"seed {render['renderSeed']}, {render['sampleRate']} Hz {render['sampleFormat']}{render['bitsPerSample']}"


def claim_midi_parses(b: dict) -> str:
    read = [read_midi(b["logic"]["dir"] / name) for name in ("score.mid", "performance.mid")]
    division = b["logic"]["manifest"]["midi"]["ticksPerQuarter"]
    for one in read:
        if one["division"] != division:
            raise Failure(f"a MIDI file's division is {one['division']}, not the manifest's {division}")
    return f"both files are format {read[0]['format']}, {read[0]['tracks']} tracks at {division} ppq"


def claim_two_readings_differ(b: dict) -> str:
    written = read_midi(b["shaped"]["dir"] / "score.mid")["bytes"]
    played = read_midi(b["shaped"]["dir"] / "performance.mid")["bytes"]
    if written == played:
        raise Failure("the written reading and the played one are the same bytes")
    at = next(index for index, pair in enumerate(zip(written, played)) if pair[0] != pair[1])
    return f"two documents of {len(written)} and {len(played)} bytes, first differing at byte {at}"


def claim_musicxml_parses(b: dict) -> str:
    import xml.etree.ElementTree as ElementTree

    root = ElementTree.parse(b["logic"]["dir"] / "score.musicxml").getroot()
    if "score" not in root.tag:
        raise Failure(f"the notation file's root element is {root.tag}")
    if (b["garageband"]["dir"] / "score.musicxml").exists():
        raise Failure("the GarageBand profile wrote notation it also says it has none of")
    if b["garageband"]["manifest"].get("notation") is not None:
        raise Failure("the GarageBand manifest names a notation file")
    return f"<{root.tag}> for Logic; the GarageBand profile says it has none"


def claim_profile_changes_only_packaging(b: dict) -> str:
    for name in ("score.mid", "performance.mid", "audio/mix.wav"):
        if digest(b["logic"]["dir"] / name) != digest(b["garageband"]["dir"] / name):
            raise Failure(f"{name} differs between the two profiles")
    return "MIDI and audio identical across profiles; only the notation is packaged differently"


def claim_one_common_length(b: dict) -> str:
    manifest = b["logic"]["manifest"]
    audio = [manifest["mix"]] + [part["stem"] for part in manifest["parts"]]
    audio += [ret["stem"] for ret in manifest.get("returns", [])]
    read = {name: read_wav(b["logic"]["dir"] / name) for name in audio}
    lengths = {one["frames"] for one in read.values()}
    if len(lengths) != 1:
        raise Failure(f"the audio files have {len(lengths)} different lengths: {sorted(lengths)}")
    if lengths != {manifest["render"]["frames"]}:
        raise Failure("the files disagree with the manifest about how long the render is")
    formats = {(one["rate"], one["channels"], one["bits"], one["tag"]) for one in read.values()}
    if len(formats) != 1:
        raise Failure("the audio files are not all one format")
    return f"{len(read)} files, {lengths.pop()} frames each, one format"


def claim_routes_are_recorded(b: dict) -> str:
    manifest = b["logic"]["manifest"]
    if not manifest.get("routes"):
        raise Failure("a piece with sends and a return recorded no routes")
    if "additive" not in manifest:
        raise Failure("nothing in the manifest says the stems do not sum to the mix")
    return f"{len(manifest['routes'])} routes, and the manifest says they do not sum"


def claim_part_mapping(b: dict) -> str:
    manifest = b["logic"]["manifest"]
    tracks, channels = set(), set()
    for part in manifest["parts"]:
        for key in ("name", "midiTrack", "midiChannel", "stem"):
            if key not in part:
                raise Failure(f"a part records no {key}")
        if not (b["logic"]["dir"] / part["stem"]).exists():
            raise Failure(f"{part['name']} names a stem that is not there")
        tracks.add(part["midiTrack"])
        channels.add(part["midiChannel"])
    if len(tracks) != len(manifest["parts"]):
        raise Failure("two parts share a MIDI track")
    return f"{len(manifest['parts'])} parts, {len(tracks)} tracks, {len(channels)} channels"


def claim_losses_are_written(b: dict) -> str:
    reported = {(loss["kind"], loss["message"]) for loss in b["logic"]["report_losses"]}
    written = {(loss["kind"], loss["message"]) for loss in b["logic"]["manifest"]["losses"]}
    if reported != written:
        raise Failure(f"the export reported {len(reported)} losses and wrote {len(written)}")
    if not reported:
        raise Failure("this piece loses something in MIDI, and the bundle recorded nothing")
    return f"{len(reported)} losses, reported and written the same"


def claim_polytempo_is_a_loss(b: dict) -> str:
    kinds = {loss["kind"] for loss in b["polytempo"]["manifest"]["losses"]}
    if not any("tempo" in kind or "clock" in kind or "meter" in kind or "barring" in kind for kind in kinds):
        raise Failure(f"a polytempo piece bundled with no clock or barring loss: {sorted(kinds)}")
    return f"recorded as {', '.join(sorted(kinds))}"


CLAIMS = {
    "digests": claim_digests,
    "identity-is-the-source": claim_identity_is_the_source,
    "identity-moves-with-the-source": claim_identity_moves_with_the_source,
    "deterministic": claim_deterministic,
    "no-clock-no-path": claim_no_clock_no_path,
    "seed-and-profile": claim_seed_and_profile,
    "midi-parses": claim_midi_parses,
    "two-readings-differ": claim_two_readings_differ,
    "musicxml-parses": claim_musicxml_parses,
    "profile-changes-only-packaging": claim_profile_changes_only_packaging,
    "one-common-length": claim_one_common_length,
    "routes-are-recorded": claim_routes_are_recorded,
    "part-mapping": claim_part_mapping,
    "losses-are-written": claim_losses_are_written,
    "polytempo-is-a-loss": claim_polytempo_is_a_loss,
}


def load_bundles(root: Path) -> dict:
    bundles = {}
    for name in ("logic", "garageband", "again", "plain", "edited", "shaped", "polytempo"):
        directory = root / name
        if not directory.is_dir():
            raise Failure(f"the shell script exported no {name} bundle")
        manifest = json.loads((directory / "musa-manifest.json").read_text())
        # What the CLI told the person who ran it, as it told them: the
        # bundle's own record is not evidence that the export said anything.
        said = (root / f"{name}-said.txt").read_text() if (root / f"{name}-said.txt").exists() else ""
        bundles[name] = {
            "dir": directory,
            "manifest": manifest,
            "report_losses": [
                {"kind": kind, "message": message}
                for kind, message in re.findall(r"^warning: ([a-z]+): (.+)$", said, re.MULTILINE)
            ],
        }
    return bundles


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: check-daw-integration.py <bundle-root> <audio-unit-report.json>", file=sys.stderr)
        return 2
    root, au_report = Path(sys.argv[1]), Path(sys.argv[2])

    text = MATRIX.read_text()
    table = rows(text)
    if len(table) < 40:
        print(f"the matrix has {len(table)} rows; it described more crossings than that", file=sys.stderr)
        return 1

    problems: list[str] = []
    bundles = load_bundles(root)

    findings = {}
    if au_report.exists():
        findings = {entry["id"]: entry for entry in json.loads(au_report.read_text())["findings"]}
    else:
        problems.append(f"no Audio Unit report at {au_report}; every finding: row is unevidenced")

    listed = subprocess.run(
        ["cargo", "nextest", "list", "--workspace", "--all-features"],
        capture_output=True,
        text=True,
        check=False,
    ).stdout
    if not listed:
        problems.append("cargo nextest listed no tests; every test: row is unevidenced")

    used_claims: set[str] = set()
    for claim, logic, garageband, harness, reference, decision, evidence in table:
        where = f"{claim!r}"
        for column, cell in (("Logic", logic), ("GarageBand", garageband)):
            if cell not in HOST_WORDS and not PROTOCOL.match(cell):
                problems.append(f"{where}: the {column} cell says {cell!r}, which is not a strength this table defines")
        if not evidence:
            problems.append(f"{where}: no evidence")
            continue
        for token in (piece.strip() for piece in evidence.split(",")):
            if token == "declined":
                if decision != "declined":
                    problems.append(f"{where}: evidence says declined and the decision column does not")
                if harness != "n/a" or reference != "n/a":
                    problems.append(f"{where}: a declined row claims a measurement")
            elif token.startswith("test:"):
                name = token.split("::")[-1]
                if f"::{name}" not in listed:
                    problems.append(f"{where}: no test named {name} in this workspace")
            elif token.startswith("finding:"):
                identifier = token[len("finding:") :]
                finding = findings.get(identifier)
                if finding is None:
                    problems.append(f"{where}: the Audio Unit report has no finding {identifier}")
                elif finding["outcome"] != "pass":
                    problems.append(f"{where}: finding {identifier} is {finding['outcome']} — {finding['detail']}")
            elif token.startswith("bundle:"):
                name = token[len("bundle:") :]
                used_claims.add(name)
                check = CLAIMS.get(name)
                if check is None:
                    problems.append(f"{where}: there is no bundle claim named {name}")
                    continue
                try:
                    print(f"  {name}: {check(bundles)}")
                except Failure as failure:
                    problems.append(f"{where}: {failure}")
            else:
                problems.append(f"{where}: {token!r} is not a kind of evidence this script resolves")

    for name in sorted(set(CLAIMS) - used_claims):
        problems.append(f"the script checks {name!r}, which no matrix row cites")

    if problems:
        print("\nthe DAW boundary did not hold:", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        return 1

    print(f"\n{len(table)} matrix rows, every one with evidence that resolves")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
