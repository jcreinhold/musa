#!/usr/bin/env python3
"""Split a source file of many top-level items into per-family submodule files.

Companion to split-impl.py (which splits one giant impl). This parses a file
into its top-level items — structs, enums, traits, impls, free fns, and
`name!(...)` macro invocations — each with its doc/attr lines walked back,
and writes one file per family. Items keep their source order within a
family, so interleaved items of different families land in their own files.

Arguments:
    <source.rs> <out-dir> <assign.json> [root-items...]

assign.json: {"types": {family: [type names]}, "fns": {family: [fn names]}}
Type names match struct/enum of that name, `impl Type` blocks, and macro
invocations whose first argument is that name (`wrapper!(NoteStmt, ...)`).
root-items: identifiers that stay in the facade (trait, macros, helpers).

Every item must be assigned exactly once (a type, a fn family, or root);
unassigned items fail loudly rather than silently dropping code.

Imports are computed per family: any of the root items referenced in the
family's items is imported explicitly from `super`, so unused-import lints
stay quiet. The facade file is left for manual writing.
"""
import json
import os
import re
import sys
from typing import NoReturn


def die(message: str) -> NoReturn:
    print(f"split-items: {message}", file=sys.stderr)
    sys.exit(1)


def strip(s: str) -> str:
    s = re.sub(r"'(?:[^'\\]|\\.)'", "''", s)  # char literals
    s = re.sub(r'"(?:[^"\\]|\\.)*"', '""', s)  # strings
    s = re.sub(r"//.*", "", s)  # line comments
    return s


def first_ident(token: str) -> str | None:
    m = re.search(r"\b([A-Za-z_]\w*)", token)
    return m.group(1) if m else None


def classify(line: str) -> tuple[str, str] | None:
    """Return (kind, name) for a top-level item start, or None."""
    m = re.match(r"^(pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait)\s+([A-Za-z_]\w*)", line)
    if m:
        return ("type", m.group(2))
    m = re.match(r"^(pub(?:\([^)]*\))?\s+)?impl", line)
    if m:
        # `impl<...> Name` or `impl Name` — the type after any impl generics.
        name = first_ident(line[line.find("impl") + 4 :].lstrip("<").lstrip(">"))
        tail = line[line.find("impl") + 4 :]
        if tail.lstrip().startswith("<"):
            # skip the generic parameter list to the closing >
            depth = 0
            for i, ch in enumerate(tail):
                if ch == "<":
                    depth += 1
                elif ch == ">":
                    depth -= 1
                    if depth == 0:
                        name = first_ident(tail[i + 1 :])
                        break
        if name is None:
            die(f"cannot parse impl header: {line!r}")
        return ("type", name)
    m = re.match(r"^(pub(?:\([^)]*\))?\s+)?fn\s+([A-Za-z_]\w*)", line)
    if m:
        return ("fn", m.group(2))
    m = re.match(r"^macro_rules!\s+([A-Za-z_]\w*)", line)  # macro definition
    if m:
        return ("macro_def", m.group(1))
    m = re.match(r"^([a-z_]\w*)!", line)  # macro invocation at col 0
    if m:
        return ("macro", m.group(1))
    return None


def main() -> None:
    if len(sys.argv) < 4:
        die("usage: split-items.py <source.rs> <out-dir> <assign.json> [root-items...]")
    src_path, out_dir, assign_path = sys.argv[1], sys.argv[2], sys.argv[3]
    root_items = set(sys.argv[4:])

    try:
        with open(assign_path) as f:
            assign = json.load(f)
    except OSError as err:
        die(f"cannot read {assign_path}: {err}")
    type_families = {name: fam for fam, names in assign.get("types", {}).items() for name in names}
    fn_families = {name: fam for fam, names in assign.get("fns", {}).items() for name in names}
    helper_names = set(assign.get("helpers", []))

    try:
        lines = open(src_path).read().split("\n")
    except OSError as err:
        die(f"cannot read {src_path}: {err}")

    # Find every top-level item: a classify()-matching line, doc/attr walk-back,
    # balanced end (braces, or `;` depth-0 for items that end at a semicolon).
    items = []  # (name, kind, [lines])
    i = 0
    while i < len(lines):
        cls = classify(lines[i])
        if cls is not None:
            kind, name = cls
            s = i
            while s > 0 and (re.match(r"^\s*///|^\s*//!|^#\[", lines[s - 1])):
                s -= 1
            # balanced end
            depth = 0
            j = i
            ended = False
            while j < len(lines):
                row = strip(lines[j])
                depth += row.count("{") - row.count("}")
                if depth <= 0 and j > i and ("}" in row or row.rstrip().endswith(";")):
                    # ensure the line actually closes the item (brace or `;`)
                    if "}" in row or row.rstrip().endswith(";"):
                        ended = True
                        j += 1
                        break
                if depth < 0:
                    ended = True
                    j += 1
                    break
                j += 1
            if not ended:
                die(f"item {name} at line {i+1} never closes")
            items.append((name, kind, lines[s:j]))
            i = j
        else:
            i += 1

    # Assign.
    families: dict[str, list] = {}
    for name, kind, body in items:
        if kind == "type":
            fam = type_families.get(name) or ("root" if name in root_items else None)
        elif kind == "fn":
            fam = fn_families.get(name) or ("root" if name in root_items else None)
        elif kind == "macro_def":
            # A `macro_rules!` definition stays where its name is declared to
            # be: the facade if it is a root helper, else its own family.
            fam = "root" if name in root_items else None
            if fam is None:
                die(
                    f"macro definition {name} is not a root item and has no family"
                )
        elif kind == "macro":
            # A macro invocation defines the type named in its first argument
            # (`wrapper!(NoteStmt, ...)`, `rule_wrapper!(MarkRule, ...)`), so
            # it belongs to that type's family, not wherever the macro itself
            # is defined. Only a first argument that names a facade item stays
            # in root. The first argument is the identifier after the `!`.
            head = " ".join(body[0:3])
            bang = head.find("!")
            first_arg = first_ident(head[bang + 1 :]) if bang >= 0 else None
            fam = (
                "root"
                if first_arg in root_items
                else type_families.get(first_arg or "") if first_arg else None
            )
        else:  # trait
            fam = "root" if name in root_items else None
        if fam is None:
            die(f"unassigned item: {name} ({kind}) at top of:\n" + "\n".join(body[:4]))
        families.setdefault(fam, []).append((name, kind, body))

    try:
        os.makedirs(out_dir, exist_ok=True)
    except OSError as err:
        die(f"cannot create {out_dir}: {err}")

    # Everything the facade can supply: root items, macros, all type names,
    # and the crate-level vocabulary the families reference.
    type_names = set(type_families.keys())
    super_names = set(root_items) | type_names | helper_names
    crate_items = {"SyntaxKind", "language::SyntaxElement", "language::SyntaxNode", "language::SyntaxToken"}
    # Macros whose bodies invoke other macros: importing the outer requires
    # importing the inner too (nested macro calls resolve at the invocation
    # site).
    macro_deps = {
        "rule_wrapper": {"wrapper"},
        "chain_container": {"wrapper"},
        "binding_wrapper": {"wrapper"},
    }

    # Items a family defines itself (its types and its helper fns), so it
    # never imports its own items back in.
    own_items: dict[str, set[str]] = {}
    for fam, names in assign.get("types", {}).items():
        own_items.setdefault(fam, set()).update(names)
    for fam, names in assign.get("fns", {}).items():
        own_items.setdefault(fam, set()).update(names)

    used_root: dict[str, set[str]] = {}
    used_crate: dict[str, set[str]] = {}
    for fam, fam_items in families.items():
        if fam == "root":
            continue
        used_s, used_c = set(), set()
        skip = own_items.get(fam, set())
        for _n, _k, body in fam_items:
            text = "\n".join(body)
            for candidate in super_names:
                if candidate in skip:
                    continue
                if re.search(rf"\b{re.escape(candidate)}\b", text):
                    used_s.add(candidate)
            for candidate in crate_items:
                short = candidate.split("::")[-1]
                if re.search(rf"\b{re.escape(short)}\b", text):
                    used_c.add(candidate)
        for outer, inners in macro_deps.items():
            if outer in used_s:
                used_s.update(inners)
        # Every wrapper reads `AstNode::cast` by its trait method, so each
        # family needs the trait in scope whether or not any item names it.
        used_s.add("AstNode")
        used_root[fam], used_crate[fam] = used_s, used_c

    for fam, fam_items in sorted(families.items()):
        if fam == "root":
            continue
        parts = []
        for _n, _k, body in fam_items:
            if _k == "fn" and _n in helper_names:
                # Cross-family helpers are reused by sibling families through
                # the facade, so they have to be visible crate-wide. The fn
                # line is not body[0] (doc comments precede it), so find it.
                out = []
                marked = False
                for line in body:
                    if not marked and re.match(r"^fn ", line):
                        out.append(re.sub(r"^fn ", "pub(crate) fn ", line))
                        marked = True
                    else:
                        out.append(line)
                body = out
            parts.extend(body + [""])
        lines_out = ["//! See `ast` module docs; the items parsed in this family."]
        lines_out.append("")
        lang = sorted(n for n in used_crate.get(fam, ()) if n.startswith("language::"))
        plain = sorted(n for n in used_crate.get(fam, ()) if not n.startswith("language::"))
        if plain:
            lines_out.append(f"use crate::{{{', '.join(plain)}}};")
        if lang:
            lines_out.append(
                "use crate::language::{" + ", ".join(n.split("::")[-1] for n in lang) + "};"
            )
        for name in sorted(used_root.get(fam, ())):
            lines_out.append(f"use super::{name};")
        lines_out.append("")
        lines_out.extend(parts)
        try:
            with open(os.path.join(out_dir, f"{fam}.rs"), "w") as f:
                f.write("\n".join(lines_out).rstrip() + "\n")
        except OSError as err:
            die(f"cannot write {fam}.rs: {err}")
        print(f"{fam}: {len(fam_items):3d} items")


if __name__ == "__main__":
    main()
