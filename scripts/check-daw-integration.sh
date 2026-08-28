#!/usr/bin/env bash
#
# The DAW boundary, run end to end rather than unit by unit.
#
# `docs/rules/across-stages/06-daw-boundary.md` names three crossings — a
# directory of open files, a live CoreMIDI projection, and Audio Unit
# production — and prompts 211-218 built them. The crate suites hold each
# one to its own laws from inside the library. This script holds all three
# to `docs/notes/research/94-the-daw-boundary-closed.md` §3's matrix from
# outside: it exports bundles with the installed CLI, over a real
# filesystem, and asserts every row's evidence still resolves.
#
# It leaves nothing registered behind and writes only under its own
# directory. Run it from anywhere; it works in the repository root.

set -euo pipefail
cd "$(dirname "$0")/.."

WORK=${MUSA_DAW_WORK:-target/daw-integration}
BUNDLES="$WORK/bundles"
MUSA=${MUSA_BIN:-target/debug/musa}
# The Audio Unit half. `scripts/check-audio-unit.sh` is the measurement; this
# script consumes its report rather than measuring the component a second
# time and getting to disagree with it.
AU_REPORT=${MUSA_AU_REPORT:-target/audio-unit/Build/Products/Debug/musa-au-report.json}

rm -rf "$WORK"
mkdir -p "$BUNDLES"

echo "== build =="
cargo build -q -p musa

# One export, named for what it is evidence of.
#
#   bundle <name> <source> <profile> [extra arguments]
bundle() {
    local name=$1 source=$2 profile=$3
    shift 3
    "$MUSA" render "$source" --to daw --profile "$profile" -o "$BUNDLES/$name" "$@" \
        >"$BUNDLES/$name-said.txt" 2>&1 \
        || { echo "exporting $name failed:" >&2; cat "$BUNDLES/$name-said.txt" >&2; exit 1; }
}

echo "== export =="
bundle logic examples/glass-mountain.musa logic
bundle garageband examples/glass-mountain.musa garageband
# The same source again, into a different directory: two exports of one piece
# are the same bytes, or determinism is a word rather than a property.
bundle again examples/glass-mountain.musa logic
bundle shaped examples/shuffle.musa logic
bundle polytempo examples/canon-x.musa logic

# The same piece twice, once with one note moved, so that identity is shown
# to follow the music rather than the filename. Both copies live under the
# work directory: `examples/` is a fixture corpus and this script does not
# write into it.
mkdir -p "$WORK/copy"
cp examples/twinkle.musa "$WORK/copy/plain.musa"
cp examples/twinkle.musa "$WORK/copy/edited.musa"
python3 - "$WORK/copy/edited.musa" <<'PYTHON'
import re
import sys

path = sys.argv[1]
text = open(path).read()
# One written pitch, moved by a step. A comment would not change the music
# and so would not change the identity, which is exactly the point.
edited, count = re.subn(r"(?<![a-z])g4(?![a-z0-9])", "a4", text, count=1)
if count == 0:
    raise SystemExit(f"{path}: nothing to move; this fixture needs a different edit")
open(path, "w").write(edited)
PYTHON
bundle plain "$WORK/copy/plain.musa" logic
bundle edited "$WORK/copy/edited.musa" logic

echo "== audio unit =="
if [ -f "$AU_REPORT" ] && [ "${MUSA_DAW_REUSE_AU:-0}" = "1" ]; then
    echo "  reusing $AU_REPORT"
elif command -v xcodebuild >/dev/null 2>&1 && xcodebuild -version >/dev/null 2>&1; then
    scripts/check-audio-unit.sh midi-processor
else
    echo "  no full Xcode here; the Audio Unit half cannot be measured on this machine" >&2
    exit 1
fi

echo "== matrix =="
python3 scripts/check-daw-integration.py "$BUNDLES" "$AU_REPORT"
