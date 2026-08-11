#!/usr/bin/env bash
#
# The language handbook, held to the language.
#
# Two halves. The generated standard-library reference must still be what the
# compiler would write — that is a Rust test, because the records it is written
# from are the compiler's. Everything else about `docs/language/` is a question
# about text: are the examples real, do the links land, do the theory citations
# name chapters that exist, is every bundled module reachable.
#
# Run from anywhere; it works in the repository root.

set -euo pipefail
cd "$(dirname "$0")/.."

echo "== generated reference is current =="
cargo test --quiet -p musa-compiler --lib reference::

echo "== handbook =="
python3 scripts/check-language-docs.py
