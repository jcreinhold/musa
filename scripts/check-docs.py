#!/usr/bin/env python3
"""Hold `docs/` to itself, and the teaching pages to the language.

Four questions:

1. Is every fenced Musa example in the teaching pages real? A block tagged
   ```musa must appear, line for line, inside a file under `examples/` or
   `stdlib/src/`. Prose may teach from a fixture; it may not invent syntax the
   shipped parser would refuse. The question is asked of the book's teaching
   pages and not of the specifications: those illustrate rules with
   deliberately compressed fragments, and several specify sound and asset
   syntax that prompts 177-189 have not built yet. Holding the whole candidate
   to its corpus is prompt 193's graduation audit
   (`docs/rules/language/05-verification.md` §7), not this checker's.
2. Does every internal link land? A relative path must exist, and an `#anchor`
   must be a heading in the file it points at. This is asked of **all** of
   `docs/`, plus the two Markdown files at the repository root, because it is
   what keeps a moved or deleted document from leaving a dangling citation
   behind: `docs/README.md` promises that nothing superseded is kept, and a
   dead link is the cheapest way to catch a broken promise.
3. Does every Open Music Theory citation name a chapter that exists? Cited by
   filename, so a reader can open it and a checker can find it.
4. Is every bundled module named in the teaching pages? Per-operation detail
   belongs to the generated reference; what a guide owes is a way in.
5. Do the generated sound reference and governing support matrices retain the
   external SFZ and SoundFont specification links they claim to follow?

Run through `scripts/check-docs.sh`, which also proves the generated reference
is current. Exits non-zero on the first category with a failure, and prints
every failure it found rather than only the first.
"""

from __future__ import annotations

import os
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
# The entry point, the book's guide chapters, and the explanation page that
# came out of the same handbook. Everything here quotes fixtures.
TEACHING = [ROOT / "README.md", DOCS / "book" / "src" / "guide", DOCS / "book" / "src" / "concepts" / "distinctions.md"]
# Link checking also covers the two Markdown files that sit at the repository
# root and cite `docs/` constantly.
EXTRA = [ROOT / "README.md", ROOT / "AGENTS.md"]
CORPUS = [ROOT / "examples", ROOT / "stdlib" / "src"]
OMT = pathlib.Path(os.environ.get("OMT_ROOT", pathlib.Path.home() / "Code/papers/music-theory/open-music-theory"))
SOUND_SPECIFICATIONS = (
    "https://sfzformat.com/opcodes/",
    "https://musescore.org/sites/musescore.org/files/2023-01/sfspec24.pdf",
)

FENCE = re.compile(r"^(\s*)```([A-Za-z0-9_-]*)\s*$")
LINK = re.compile(r"(?<!!)\[[^\]^]*\]\(([^)\s]+)\)")
CITATION = re.compile(r"`(\d{3}-[a-z0-9-]+\.md)`")
HEADING = re.compile(r"^(#{1,6})\s+(.*?)\s*#*\s*$")

# Everything below exists because `check_links` cannot see these citations.
# It reads `prose_lines`, which strips inline code spans before matching, so a
# path written as `crates/musa-score/src/analysis/rules.rs` — the way this repo
# writes almost all of them — is invisible to it. That blind spot let three
# specifications name modules in the wrong crate and let two directories hold a
# file of the same name for months.
SPAN = re.compile(r"`([^`]+)`")
# The workspace's top-level directories. A code span starting with one of these
# is claiming a file exists, and is checkable.
WORKSPACE = ("apps/", "crates/", "docs/", "editors/", "examples/", "packages/", "scripts/", "stdlib/", "tests/")
# `file.rs:120`, `file.rs:12-20`, and `file.rs::symbol` all point at a real file.
LOCATOR = re.compile(r"(::.+|:\d+(?:-\d+)?)$")
# A citation by bare filename: `02-core-calculus.md`. Two digits, because three
# is an Open Music Theory chapter and CITATION already owns those.
BARE = re.compile(r"\d{2}[a-z]*-[a-z0-9-]+\.md")
# Paths are only checkable in documents that describe the code as it is now. A
# `done` prompt describes the code as it stood when that prompt ran, and a
# research record describes a design that was deleted; both are correct as
# history and would have to be falsified to pass a liveness check.
LIVE = ("docs/rules", "docs/book", "docs/plan/code-map", "docs/README.md", "docs/plan/README.md", "docs/plan/roadmap.md")


def markdown_files() -> list[pathlib.Path]:
    return sorted(DOCS.rglob("*.md")) + EXTRA


def teaching_files() -> list[pathlib.Path]:
    out: list[pathlib.Path] = []
    for entry in TEACHING:
        out.extend(sorted(entry.rglob("*.md")) if entry.is_dir() else [entry])
    return out


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
    for path in teaching_files():
        for line, tag, block in fences(path):
            if tag != "musa":
                continue
            if not any(contains(source, block) for source in corpus.values()):
                where = path.relative_to(ROOT)
                first = block[0] if block else "<empty>"
                problems.append(f"{where}:{line}: this `musa` block is in no fixture — {first!r}")
    return problems


CODE_SPAN = re.compile(r"`[^`]*`")


def prose_lines(path: pathlib.Path):
    """Every line outside a fenced block, with inline code spans removed.

    Both exclusions earn their place. A grammar fence writes
    `Timeline[M; d](A)` and a lint prompt writes the accidental regex
    `[a-g](##|bb|[#bn])?`; neither is a link, and both match the link pattern
    exactly. Stripping code first is what lets the link check be strict about
    everything that is left.
    """
    fenced = False
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        if FENCE.match(line):
            fenced = not fenced
            continue
        if not fenced:
            yield number, CODE_SPAN.sub("", line)


def check_links() -> list[str]:
    problems = []
    cache: dict[pathlib.Path, set[str]] = {}
    for path in markdown_files():
        for number, line in prose_lines(path):
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


def code_spans(path: pathlib.Path):
    """Every inline code span outside a fenced block, with its line number.

    The exact complement of `prose_lines`: that function throws these away so
    the link check can be strict, and this one picks them up so the paths
    written inside them are checked too.
    """
    fenced = False
    for number, line in enumerate(path.read_text().splitlines(), start=1):
        if FENCE.match(line):
            fenced = not fenced
            continue
        if not fenced:
            for span in SPAN.findall(line):
                yield number, span


def live_files() -> list[pathlib.Path]:
    return [path for path in markdown_files() if str(path.relative_to(ROOT)).startswith(LIVE) or path in EXTRA]


def check_paths() -> list[str]:
    """Every backticked workspace path in a live document names something."""
    problems = []
    for path in live_files():
        for number, span in code_spans(path):
            if not span.startswith(WORKSPACE):
                continue
            # A glob, a brace list, or a `<placeholder>` is a pattern rather
            # than a path, and says so in its own punctuation.
            if any(character in span for character in "*?[]{}<> "):
                continue
            target = ROOT / LOCATOR.sub("", span).rstrip("/")
            if not target.exists():
                problems.append(f"{path.relative_to(ROOT)}:{number}: `{span}` names nothing")
    return problems


def check_bare_citations() -> list[str]:
    """A document cited by bare filename resolves to exactly one document.

    Not-found is deliberately not an error here: a bare name is also how this
    repo cites chapters of books it does not contain. Ambiguity is the failure
    worth gating on, because a reader cannot resolve it at all — and because it
    is what happened, twice, before this check existed.
    """
    index: dict[str, list[pathlib.Path]] = {}
    for path in markdown_files():
        index.setdefault(path.name, []).append(path)
    problems = []
    for path in markdown_files():
        for number, span in code_spans(path):
            if not BARE.fullmatch(span):
                continue
            found = index.get(span, [])
            if len(found) > 1:
                named = ", ".join(sorted(str(one.relative_to(ROOT)) for one in found))
                problems.append(f"{path.relative_to(ROOT)}:{number}: `{span}` is ambiguous — {named}")
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
    missing = [entry for entry in TEACHING if not entry.exists()]
    if missing:
        return [f"{entry.relative_to(ROOT)} does not exist" for entry in missing]
    text = "\n".join(path.read_text() for path in teaching_files())
    problems = []
    for path in sorted((ROOT / "stdlib" / "src").rglob("*.musa")):
        relative = path.relative_to(ROOT / "stdlib" / "src")
        if relative.name in {"lib.musa", "mod.musa"}:
            continue
        module = str(relative.with_suffix("")).replace("/", "::")
        if f"std::{module}" not in text:
            problems.append(f"the teaching pages never name `std::{module}`")
    return problems


def check_sound_specifications() -> list[str]:
    """The two foreign support matrices keep their primary-source anchors."""
    targets = (
        DOCS / "rules" / "language" / "09-assets-and-packages.md",
        DOCS / "book" / "src" / "reference" / "studio-vocabulary.md",
    )
    problems = []
    for path in targets:
        text = path.read_text()
        for url in SOUND_SPECIFICATIONS:
            if url not in text:
                problems.append(f"{path.relative_to(ROOT)}: missing sound specification link `{url}`")
    return problems


def main() -> int:
    failed = False
    for name, check in (
        ("musa examples", check_examples),
        ("internal links", check_links),
        ("workspace paths", check_paths),
        ("bare citations", check_bare_citations),
        ("theory citations", check_citations),
        ("module coverage", check_modules),
        ("sound specifications", check_sound_specifications),
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
