#!/usr/bin/env bash
# Build the @musa/web wasm artifact (prompt 138), the post-wasm-pack way:
# cargo build → wasm-bindgen --target web → wasm-opt -Oz. Wasm-pack was
# sunset in July 2025; this is its three useful steps, pinned.
#
# Toolchain (pinned in mise.toml — `mise install`; manual equivalents):
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <must equal Cargo.lock's wasm-bindgen>
#   brew install binaryen   # or any wasm-opt
set -euo pipefail
cd "$(dirname "$0")/.."

CRATE=musa-wasm
ARTIFACT=musa_wasm
PROFILE=wasm
OUT=packages/musa-web/wasm

# The bindgen CLI must equal the crate's wasm-bindgen version, byte for byte:
# mismatched glue fails at load time with inscrutable errors, so refuse early.
LOCK_VERSION=$(awk '/^name = "wasm-bindgen"$/{getline; print; exit}' Cargo.lock | sed 's/version = "\(.*\)"/\1/')
CLI_VERSION=$(wasm-bindgen --version 2>/dev/null | awk '{print $2}' || true)
if [ -z "${CLI_VERSION}" ] || [ "${LOCK_VERSION}" != "${CLI_VERSION}" ]; then
    echo "wasm-bindgen CLI '${CLI_VERSION:-missing}' != crate '${LOCK_VERSION}'." >&2
    echo "Install the matching CLI: cargo install wasm-bindgen-cli --version ${LOCK_VERSION}" >&2
    exit 1
fi

cargo build -p "${CRATE}" --target wasm32-unknown-unknown --profile "${PROFILE}"

mkdir -p "${OUT}"
wasm-bindgen --target web --out-dir "${OUT}" "target/wasm32-unknown-unknown/${PROFILE}/${ARTIFACT}.wasm"

if command -v wasm-opt >/dev/null 2>&1; then
    # Rust's wasm32 target emits these target features by default; wasm-opt
    # validates against MVP unless told otherwise.
    wasm-opt -Oz \
        --enable-bulk-memory --enable-mutable-globals --enable-sign-ext \
        -o "${OUT}/${ARTIFACT}_bg.wasm" "${OUT}/${ARTIFACT}_bg.wasm"
else
    echo "wasm-opt not found; skipping -Oz (install binaryen for the size win)." >&2
fi

for f in "${OUT}"/*.wasm; do
    raw=$(stat -f %z "$f" 2>/dev/null || stat -c %s "$f")
    if command -v brotli >/dev/null 2>&1; then
        packed=$(brotli -c -q 11 "$f" | wc -c | tr -d ' ')
        kind=brotli
    else
        packed=$(gzip -9 -c "$f" | wc -c | tr -d ' ')
        kind=gzip
    fi
    echo "${f}: ${raw} bytes raw, ${packed} bytes ${kind}"
done
