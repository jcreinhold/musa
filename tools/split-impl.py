#!/usr/bin/env python3
"""Split one giant `impl` block into per-concern submodule files.

Used by the >1500-line file splits (course correction). Reads the original
file, snipped into per-method chunks (each anchored at its `fn` line, with
doc/attr lines walked back and attached), and writes `dir/module.rs` files
holding `impl<T> Type<T> { ... }` blocks. The facade file (mod.rs) is left
for manual writing.

Arguments:
    <source.rs> <impl-header-line:1-based> <dir> [method:module ...]

Every method in the impl must be assigned exactly once; unassigned methods
fail loudly rather than silently dropping code.

Usage (from repo root):
    python3 tools/split-impl.py crates/.../big.rs 327 crates/.../big/ \
        "new run start" engine "expr pattern" expressions
"""
import os
import re
import sys
from typing import NoReturn


def die(message: str) -> NoReturn:
    print(f"split-impl: {message}", file=sys.stderr)
    sys.exit(1)


def strip(s: str) -> str:
    s = re.sub(r"'(?:[^'\\]|\\.)'", "''", s)  # char literals
    s = re.sub(r'"(?:[^"\\]|\\.)*"', '""', s)  # strings
    s = re.sub(r"//.*", "", s)  # line comments
    return s


def load(path: str) -> list[str]:
    try:
        with open(path) as f:
            return f.read().split("\n")
    except OSError as err:
        die(f"cannot read {path}: {err}")
        return []


def write_out(path: str, text: str) -> None:
    try:
        with open(path, "w") as f:
            f.write(text)
    except OSError as err:
        die(f"cannot write {path}: {err}")


def main() -> None:
    if len(sys.argv) < 5:
        die(
            "usage: split-impl.py <source.rs> <impl-header-line> <out-dir>"
            ' "<m1 m2> module" ...'
        )
    src_path, impl_line, out_dir = sys.argv[1], sys.argv[2], sys.argv[3]
    try:
        impl_line = int(impl_line)
    except ValueError:
        die(f"impl header line is not an integer: {impl_line!r}")
    pairs = sys.argv[4:]
    assignments = {}
    for pair in pairs:
        methods, module = pair.rsplit(":", 1)
        for method in methods.strip().split():
            assignments[method] = module.strip()

    lines = load(src_path)
    header = lines[impl_line - 1]
    if not re.match(r"^impl.*\{$", header):
        die(f"not an impl header at line {impl_line}: {header!r}")

    depth, end = 0, None
    for i in range(impl_line - 1, len(lines)):
        depth += strip(lines[i]).count("{") - strip(lines[i]).count("}")
        if depth <= 0:
            end = i
            break
    if end is None:
        die("impl never closes")
    body = lines[impl_line - 1 : end + 1]
    if body[-1] != "}":
        die(f"expected closing brace, got {body[-1]!r}")

    anchors = [
        (i, m.group(1))
        for i, l in enumerate(body[1:], start=1)
        if (m := re.match(r"^    fn (\w+)", l))
    ]
    m = re.match(r"^impl(<[^>]*>)?\s+(\w+)", header)
    if not m:
        die(f"cannot parse impl header: {header!r}")
    impl_header = f"impl{m.group(1) or ''} {m.group(2)}"

    chunks = []
    for k, (i, name) in enumerate(anchors):
        nxt = anchors[k + 1][0] if k + 1 < len(anchors) else len(body)
        depth, j = 0, i
        while j < nxt:
            depth += strip(body[j]).count("{") - strip(body[j]).count("}")
            j += 1
            if depth <= 0:
                break
        s = i
        while s > 1 and re.match(r"^    (///|//|#\[)", body[s - 1]):
            s -= 1
        chunks.append((name, body[s:j]))

    names = [n for n, _ in chunks]
    unassigned = [n for n in names if n not in assignments]
    if unassigned:
        die(f"unassigned methods: {unassigned}")
    phantom = [n for n in assignments if n not in names]
    if phantom:
        die(f"assigned but absent from impl: {phantom}")

    try:
        os.makedirs(out_dir, exist_ok=True)
    except OSError as err:
        die(f"cannot create {out_dir}: {err}")

    modules = {}
    for name, chunk in chunks:
        modules.setdefault(assignments[name], []).extend(chunk + [""])
    for module, chunk in sorted(modules.items()):
        cast = [
            re.sub(r"^    fn ", "    pub(super) fn ", l) if re.match(r"^    fn ", l) else l
            for l in chunk
        ]
        text = (
            f"//! See `parser` module docs; grammar handled in `{module}`.\n\n"
            f"use super::*;\n\n{impl_header} {{\n"
            + "\n".join(cast).rstrip()
            + "\n}\n"
        )
        write_out(os.path.join(out_dir, f"{module}.rs"), text)
        print(f"{module}: {len(chunk):5d} lines")


if __name__ == "__main__":
    main()
