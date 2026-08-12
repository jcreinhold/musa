#!/usr/bin/env bash
#
# The documentation, held to itself and to the language.
#
# Two halves. The generated standard-library reference must still be what the
# compiler would write — that is a Rust test, because the records it is written
# from are the compiler's. Everything else about `docs/` is a question about
# text: do the links land across the whole tree, are the teaching examples
# real, do the theory citations name chapters that exist, is every bundled
# module reachable.
#
# Run from anywhere; it works in the repository root.

set -euo pipefail
cd "$(dirname "$0")/.."

echo "== generated reference is current =="
cargo test --quiet -p musa-compiler --lib reference::

echo "== docs =="
python3 scripts/check-docs.py

echo "== the book builds =="
mdbook build docs/book
