#!/usr/bin/env bash
#
# The Audio Unit trial, run against the system rather than against itself.
#
# Prompt 215 asks whether the boundary in `docs/rules/across-stages/06-daw-boundary.md`
# §3–§7 can be built at all on macOS. Answering that needs the real system:
# the two components have to be signed, registered, found by Core Audio,
# loaded in another process, and validated by Apple's own `auval`. So this
# script builds them, installs them for this user, measures them, and takes
# them out again.
#
# It leaves nothing registered behind. Run from anywhere; it works in the
# repository root.

set -euo pipefail
cd "$(dirname "$0")/.."

PROJECT=apps/musa-audio-unit-trial/MusaAudioUnitTrial.xcodeproj
CONFIG=apps/musa-audio-unit-trial/Config
DERIVED=${MUSA_TRIAL_DERIVED:-target/audio-unit-trial}
PRODUCTS="$DERIVED/Build/Products/Debug"
APP="$PRODUCTS/MusaAudioUnitTrial.app"
REPORT=${MUSA_TRIAL_REPORT:-$PRODUCTS/trial-report.json}

if ! xcodebuild -version >/dev/null 2>&1; then
    echo "xcodebuild needs a full Xcode, not just the Command Line Tools." >&2
    echo "Install Xcode and run 'sudo xcode-select -s /Applications/Xcode.app'." >&2
    exit 1
fi

cleanup() {
    for extension in Instrument Processor; do
        pluginkit -r "$APP/Contents/PlugIns/$extension.appex" >/dev/null 2>&1 || true
    done
}
trap cleanup EXIT

mkdir -p "$DERIVED"

echo "== build =="
xcodebuild -project "$PROJECT" -scheme MusaAudioUnitTrial -configuration Debug \
    -derivedDataPath "$DERIVED" CODE_SIGNING_ALLOWED=NO build >"$DERIVED.log" 2>&1 \
    || { tail -40 "$DERIVED.log"; exit 1; }

echo "== sign =="
# Ad hoc, because this machine has no Developer Team and the trial needs
# none: a registered component has to be signed, and what a Team would add
# is exactly the App Group container the report records as unavailable.
codesign -f -s - "$APP/Contents/Frameworks/MusaTrialKit.framework" >/dev/null 2>&1
for extension in Instrument Processor; do
    codesign -f -s - --entitlements "$CONFIG/$extension.entitlements" \
        "$APP/Contents/PlugIns/$extension.appex" >/dev/null 2>&1
done
codesign -f -s - --entitlements "$CONFIG/App.entitlements" "$APP" >/dev/null 2>&1

echo "== register =="
for extension in Instrument Processor; do
    pluginkit -a "$APP/Contents/PlugIns/$extension.appex" >/dev/null 2>&1
done

echo "== settle =="
# Registration is not instant: `auval -a` is the registry's own answer, so
# wait for it rather than guessing at a sleep.
settled=0
for _ in $(seq 1 30); do
    if [ "$(auval -a 2>/dev/null | grep -c 'Musa')" -ge 2 ]; then
        settled=1
        break
    fi
    sleep 1
done
if [ "$settled" -ne 1 ]; then
    echo "the two components never appeared in the Audio Unit registry" >&2
    exit 1
fi

echo "== auval =="
auval_ok=1
for component in "aumu musi Musa" "aumi musp Musa"; do
    # shellcheck disable=SC2086
    if auval -v $component >"$PRODUCTS/auval-$(echo "$component" | tr ' ' '-').log" 2>&1; then
        echo "  $component: PASS"
    else
        echo "  $component: FAIL"
        auval_ok=0
    fi
done

echo "== harness =="
MUSA_TRIAL_XCODE="$(xcodebuild -version | head -1)" \
MUSA_TRIAL_SDK="$(xcodebuild -showsdks 2>/dev/null | grep -m1 -o 'macosx[0-9.]*')" \
MUSA_TRIAL_HOST="$(uname -m) $(sw_vers -productName) $(sw_vers -productVersion) $(sw_vers -buildVersion)" \
DYLD_INSERT_LIBRARIES="$PRODUCTS/libMusaAllocProbe.dylib" \
    "$PRODUCTS/musa-au-trial-harness" >"$REPORT" || true

echo "== findings =="
python3 scripts/check-audio-unit-trial.py "$REPORT" "$auval_ok"
