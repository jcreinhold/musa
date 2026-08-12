#!/usr/bin/env python3
"""Hold the language handbook to the language.

Four questions, asked of every Markdown file under `docs/language/`:

1. Is every fenced Musa example in the handbook real? A block tagged ```musa
   must appear, line for line, inside a file under `examples/` or
   `stdlib/src/`. Prose may teach from a fixture; it may not invent syntax the
   shipped parser would refuse. The question is asked of `handbook/` and not of
   the specification beside it: those documents illustrate rules with
   deliberately compressed fragments, and several of them specify sound and
   asset syntax that prompts 130-142 have not built yet. Holding the whole
   candidate to its corpus is prompt 146's graduation audit
   (`../05-verification.md` §7), not this checker's.
2. Does every internal link land? A relative path must exist, and an `#anchor`
   must be a heading in the file it points at.
3. Does every Open Music Theory citation name a chapter that exists? Cited by
   filename, so a reader can open it and a checker can find it.
4. Is every bundled module named in the handbook? Per-operation detail belongs
   to the generated reference; what the handbook owes is a way in.

Run through `scripts/check-language-docs.sh`, which also proves the generated
reference is current. Exits non-zero on the first category with a failure, and
prints every failure it found rather than only the first.
"""

from __future__ import annotations

import os
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs" / "language"
HANDBOOK = DOCS / "handbook"
CORPUS = [ROOT / "examples", ROOT / "stdlib" / "src"]
OMT = pathlib.Path(os.environ.get("OMT_ROOT", pathlib.Path.home() / "Code/papers/music-theory/open-music-theory"))

FENCE = re.compile(r"^(\s*)```([A-Za-z0-9_-]*)\s*$")
LINK = re.compile(r"(?<!!)\[[^\]^]*\]\(([^)\s]+)\)")
CITATION = re.compile(r"`(\d{3}-[a-z0-9-]+\.md)`")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")


def markdown_files() -> list[pathlib.Path]:
    return sorted(DOCS.rglob("*.md"))


def fences(path: pathlib.Path) -> list[tuple[int, str, list[str]]]:
    """Every fenced block: its opening line number, its tag, and its lines."""
    out: list[tuple[int, str, list[str]]] = []
    lines = path.read_text().splitlines()
    index = 0
    while index < len(lines):
        opened = FENCE.match(lines[index])
        if not opened:
            index += 1
            continue
        indent, tag = opened.group(1), opened.group(2)
        body: list[str] = []
        cursor = index + 1
        while cursor < len(lines):
            closing = FENCE.match(lines[cursor])
            if closing and closing.group(2) == "" and closing.group(1) == indent:
                break
            body.append(lines[cursor])
            cursor += 1
        out.append((index + 1, tag, dedent(body)))
        index = cursor + 1
    return out


def dedent(lines: list[str]) -> list[str]:
    margin = min(
        (len(line) - len(line.lstrip()) for line in lines if line.strip()),
        default=0,
    )
    return [line[margin:].rstrip() if line.strip() else "" for line in lines]


def contains(source: list[str], block: list[str]) -> bool:
    """Whether `source` holds `block`, allowing for a constant extra indent.

    A tutorial quotes a phrase out of the middle of a piece, where it is
    nested inside a voice inside a part. Requiring the same indentation would
    make every quotation wrong by four spaces; allowing an arbitrary *per
    line* indent would let a reordered block pass.
    """
    if not block:
        return False
    for start in range(len(source) - len(block) + 1):
        window = source[start : start + len(block)]
        margins = [len(line) - len(line.lstrip()) for line in window if line.strip()]
        if not margins:
            continue
        margin = min(margins)
        if all(
            (line[margin:].rstrip() if line.strip() else "") == expected
            for line, expected in zip(window, block, strict=True)
        ):
            return True
    return False


def slug(heading: str) -> str:
    """GitHub's heading anchor, near enough for a repository's own links."""
    text = re.sub(r"`|\*|_", "", heading).strip().lower()
    text = re.sub(r"[^\w\s-]", "", text)
    return re.sub(r"\s+", "-", text)


def anchors(path: pathlib.Path) -> set[str]:
    seen: dict[str, int] = {}
    out: set[str] = set()
    for line in path.read_text().splitlines():
        found = HEADING.match(line)
        if not found:
            continue
        base = slug(found.group(2))
        count = seen.get(base, 0)
        out.add(base if count == 0 else f"{base}-{count}")
        seen[base] = count + 1
    return out


def check_examples() -> list[str]:
    corpus = {path: path.read_text().splitlines() for directory in CORPUS for path in directory.rglob("*.musa")}
    problems = []
    for path in sorted(HANDBOOK.rglob("*.md")):
        for line, tag, block in fences(path):
            if tag != "musa":
                continue
            if not any(contains(source, block) for source in corpus.values()):
                where = path.relative_to(ROOT)
                first = block[0] if block else "<empty>"
                problems.append(f"{where}:{line}: this `musa` block is in no fixture — {first!r}")
    return problems


def check_links() -> list[str]:
    problems = []
    cache: dict[pathlib.Path, set[str]] = {}
    for path in markdown_files():
        for number, line in enumerate(path.read_text().splitlines(), start=1):
            for target in LINK.findall(line):
                if target.startswith(("http://", "https://", "mailto:")):
                    continue
                where = f"{path.relative_to(ROOT)}:{number}"
                file_part, _, anchor = target.partition("#")
                destination = (path.parent / file_part).resolve() if file_part else path
                if not destination.exists():
                    problems.append(f"{where}: `{target}` names nothing")
                    continue
                if not anchor:
                    continue
                if destination.suffix != ".md":
                    problems.append(f"{where}: `{target}` anchors into a file with no headings")
                    continue
                if destination not in cache:
                    cache[destination] = anchors(destination)
                if anchor not in cache[destination]:
                    problems.append(f"{where}: `{target}` has no such heading")
    return problems


def check_citations() -> list[str]:
    if not OMT.is_dir():
        print(f"note: no Open Music Theory checkout at {OMT}; citations unchecked", file=sys.stderr)
        return []
    problems = []
    for path in markdown_files():
        for number, line in enumerate(path.read_text().splitlines(), start=1):
            problems.extend(
                f"{path.relative_to(ROOT)}:{number}: no OMT chapter `{cited}`"
                for cited in CITATION.findall(line)
                if not (OMT / cited).exists()
            )
    return problems


def check_modules() -> list[str]:
    if not HANDBOOK.is_dir():
        return [f"{HANDBOOK.relative_to(ROOT)} does not exist"]
    text = "\n".join(path.read_text() for path in HANDBOOK.rglob("*.md"))
    problems = []
    for path in sorted((ROOT / "stdlib" / "src").rglob("*.musa")):
        relative = path.relative_to(ROOT / "stdlib" / "src")
        if relative.name in {"lib.musa", "mod.musa"}:
            continue
        module = str(relative.with_suffix("")).replace("/", "::")
        if f"std::{module}" not in text:
            problems.append(f"the handbook never names `std::{module}`")
    return problems


def main() -> int:
    failed = False
    for name, check in (
        ("musa examples", check_examples),
        ("internal links", check_links),
        ("theory citations", check_citations),
        ("module coverage", check_modules),
    ):
        problems = check()
        if problems:
            failed = True
            print(f"{name}: {len(problems)} problem(s)", file=sys.stderr)
            for problem in problems:
                print(f"  {problem}", file=sys.stderr)
        else:
            print(f"{name}: ok")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
