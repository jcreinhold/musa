#!/usr/bin/env bash
#
# The Musa Audio Unit, run against the system rather than against itself.
#
# Prompt 215 asked whether the boundary in `docs/rules/across-stages/06-daw-boundary.md`
# §3–§7 could be built on macOS. This script checks that the component which
# implements it still holds: it builds the component, signs it ad hoc,
# registers it for this user, lets Apple's own `auval` validate it, measures
# it with an automated host, and takes it out again.
#
# It leaves nothing registered behind. Run from anywhere; it works in the
# repository root. Needs no Developer Team and no App Group.

set -euo pipefail
cd "$(dirname "$0")/.."

# What the run is required to have measured. The harness measures everything
# it can either way; this chooses which findings may be missing without the
# run counting as a pass, so that a boundary built in one prompt cannot be
# quietly un-built in the next.
COMPONENT=${1:-}
case "$COMPONENT" in
    instrument) ;;
    parameters-and-outputs) ;;
    *)
        echo "usage: $(basename "$0") instrument|parameters-and-outputs" >&2
        echo "  'instrument' is the Music Device 'aumu musa Musa' as prompt 216 built it." >&2
        echo "  'parameters-and-outputs' adds prompt 217's source controls and output buses." >&2
        echo "  Prompt 218 adds the rest." >&2
        exit 2
        ;;
esac

PROJECT=apps/musa-audio-unit/MusaAudioUnit.xcodeproj
CONFIG=apps/musa-audio-unit/Config
DERIVED=${MUSA_AU_DERIVED:-target/audio-unit}
PRODUCTS="$DERIVED/Build/Products/Debug"
APP="$PRODUCTS/MusaAudioUnit.app"
APPEX="$APP/Contents/PlugIns/MusaInstrument.appex"
REPORT=${MUSA_AU_REPORT:-$PRODUCTS/musa-au-report.json}
SOURCE=${MUSA_AU_PROJECT:-$PWD/examples/invention.musa}
PART=${MUSA_AU_PART:-piano}
# A second fixture, because "the outputs this part reaches" is only more than
# one thing when there is a studio behind the part. Named here rather than
# found by the harness: a harness that went looking would be choosing what to
# measure.
SOURCE2=${MUSA_AU_PROJECT2:-$PWD/examples/glass-mountain.musa}
PART2=${MUSA_AU_PART2:-violin}

if ! xcodebuild -version >/dev/null 2>&1; then
    echo "xcodebuild needs a full Xcode, not just the Command Line Tools." >&2
    echo "Install Xcode and run 'sudo xcode-select -s /Applications/Xcode.app'." >&2
    exit 1
fi

cleanup() {
    pluginkit -r "$APPEX" >/dev/null 2>&1 || true
}
trap cleanup EXIT

mkdir -p "$DERIVED"

echo "== build =="
xcodebuild -project "$PROJECT" -scheme MusaAudioUnit -configuration Debug \
    -derivedDataPath "$DERIVED" CODE_SIGNING_ALLOWED=NO build >"$DERIVED.log" 2>&1 \
    || { tail -40 "$DERIVED.log"; exit 1; }

echo "== sign =="
# Ad hoc, because this machine has no Developer Team and the component needs
# none: a registered component has to be signed, and what a Team would add is
# the App Group container prompt 215 measured as unusable anyway.
codesign -f -s - "$APP/Contents/Frameworks/MusaAudioUnitKit.framework" >/dev/null 2>&1
codesign -f -s - --entitlements "$CONFIG/Instrument.entitlements" "$APPEX" >/dev/null 2>&1
codesign -f -s - --entitlements "$CONFIG/App.entitlements" "$APP" >/dev/null 2>&1

echo "== register =="
pluginkit -a "$APPEX" >/dev/null 2>&1

echo "== settle =="
# Registration is not instant, and `auval -a` is the registry's own answer,
# so wait for it rather than guessing at a sleep.
settled=0
for _ in $(seq 1 30); do
    if auval -a 2>/dev/null | grep -q 'aumu musa Musa'; then
        settled=1
        break
    fi
    sleep 1
done
if [ "$settled" -ne 1 ]; then
    echo "the component never appeared in the Audio Unit registry" >&2
    exit 1
fi

echo "== auval =="
auval_ok=1
if auval -v aumu musa Musa >"$PRODUCTS/auval-instrument.log" 2>&1; then
    echo "  aumu musa Musa: PASS"
else
    echo "  aumu musa Musa: FAIL"
    auval_ok=0
fi

echo "== harness =="
MUSA_AU_XCODE="$(xcodebuild -version | head -1)" \
MUSA_AU_SDK="$(xcodebuild -showsdks 2>/dev/null | grep -m1 -o 'macosx[0-9.]*')" \
MUSA_AU_HOST="$(uname -m) $(sw_vers -productName) $(sw_vers -productVersion) $(sw_vers -buildVersion)" \
MUSA_AU_PROJECT="$SOURCE" MUSA_AU_PART="$PART" MUSA_AU_REPORT="$REPORT" \
MUSA_AU_PROJECT2="$SOURCE2" MUSA_AU_PART2="$PART2" \
DYLD_INSERT_LIBRARIES="$PRODUCTS/libMusaAllocProbe.dylib" \
    "$PRODUCTS/musa-au-harness" || true

echo "== thread sanitizer =="
# A separate build, and in-process only. TSan instruments the Swift and
# Objective-C sides; `libmusa_au.a` is compiled without it, so what this
# proves is that the component's own threading — the control worker, the
# publication slot, the render block — is clean, and it says so rather than
# implying the whole boundary was instrumented. Spawning system extensions
# under TSan would measure the system.
TSAN_DERIVED="$DERIVED-tsan"
TSAN_PRODUCTS="$TSAN_DERIVED/Build/Products/Debug"
TSAN_LOG="$TSAN_DERIVED.run.log"
mkdir -p "$TSAN_DERIVED"
xcodebuild -project "$PROJECT" -scheme MusaAudioUnit -configuration Debug \
    -derivedDataPath "$TSAN_DERIVED" CODE_SIGNING_ALLOWED=NO \
    ENABLE_THREAD_SANITIZER=YES build >"$TSAN_DERIVED.log" 2>&1 \
    || { tail -40 "$TSAN_DERIVED.log"; exit 1; }
MUSA_AU_PROJECT="$SOURCE" MUSA_AU_PART="$PART" MUSA_AU_SKIP_HOSTED=1 \
MUSA_AU_PROJECT2="$SOURCE2" MUSA_AU_PART2="$PART2" \
MUSA_AU_REPORT="$TSAN_PRODUCTS/musa-au-tsan-report.json" \
    "$TSAN_PRODUCTS/musa-au-harness" >"$TSAN_LOG" 2>&1 || true
if grep -q 'WARNING: ThreadSanitizer' "$TSAN_LOG"; then
    echo "  ThreadSanitizer reported a race:" >&2
    grep -A 20 'WARNING: ThreadSanitizer' "$TSAN_LOG" | head -40 >&2
    exit 1
fi
echo "  clean (Swift and Objective-C instrumented; the Rust library is not)"

echo "== findings =="
python3 scripts/check-audio-unit.py "$REPORT" "$auval_ok" "$COMPONENT"
