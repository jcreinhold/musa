#!/usr/bin/env python3
"""Renumber the prompt stack and rewrite every reference to it.

A prompt file is named ``NN[suffix]-slug.md``. The number is an execution rank,
not an identity: inserting a prompt in the middle shifts every later rank, and
the whole repository must follow. This script performs that shift as one
transaction — `git mv` for the files, one atomic rewrite pass for the text.

The rewrite is a *single* regular-expression pass per file, driven by one
old-number-to-new-number map. That is the point of the script: rewriting the
numbers one at a time (``125 -> 126``, then ``126 -> 127``, ...) double-bumps
whatever the earlier substitution already moved, silently collapsing ranges
like ``131-132`` into ``133-133``. One pass cannot do that.

What is rewritten, and only this:

* prompt filenames and any path or bare stem that names one (``125-slug.md``);
* the ``id:`` and ``depends_on:`` frontmatter of every prompt;
* markdown link labels whose target is a prompt file (``[125](../prompts/125-slug.md)``);
* prose cued by the word "prompt" — ``prompt 125``, ``prompts 125, 127 and 130``,
  ``prompts 125-130``, ``prompt 125/135``;
* en-dash ranges whose *both* endpoints are known prompt numbers, cued or not,
  because block tables write them bare (``| 139-142 | ... |``);
* every number in the map, in files named by ``--bare-in``, for documents whose
  numbers are all prompt numbers.

Everything else is left alone and reported. A bare ``125`` in running prose is
as likely to be a zoom percentage or a byte count as a prompt, so the script
refuses to guess and prints it for review instead.

The default is a dry run. Pass ``--apply`` to write.

Usage:
    scripts/renumber-prompts.py audit
    scripts/renumber-prompts.py make-room --at 125 [--count 1] [--apply]
    scripts/renumber-prompts.py move 130-instrument-contracts.md --to 124 [--apply]
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from collections.abc import Iterable, Iterator
from dataclasses import dataclass
from pathlib import Path

NAME_RE = re.compile(r"^(?P<number>\d{2,})(?P<suffix>[a-z][a-z0-9]*)?-(?P<slug>.+)$")
ID_RE = re.compile(r"^id:\s*(?P<label>\d{2,}[a-z0-9]*)\s*$", re.MULTILINE)

TEXT_SUFFIXES = frozenset(
    {".md", ".toml", ".yml", ".yaml", ".sh", ".py", ".rs", ".json", ".txt", ".ts", ".js", ".svelte", ".musa"}
)
SKIP_DIRS = frozenset({".git", "target", "node_modules", "dist", "dist-cdn", ".venv"})

DASH = "–"  # en dash: the repo writes ranges with it


class RenumberError(RuntimeError):
    """A safe plan could not be built."""


@dataclass(frozen=True, slots=True)
class Prompt:
    """One numbered prompt file."""

    path: Path
    number: int
    width: int
    suffix: str
    slug: str

    @property
    def label(self) -> str:
        return f"{self.number:0{self.width}d}{self.suffix}"

    @property
    def stem(self) -> str:
        return f"{self.label}-{self.slug}"

    def at(self, number: int) -> Path:
        width = max(self.width, len(str(number)))
        return self.path.with_name(f"{number:0{width}d}{self.suffix}-{self.slug}.md")


def repo_root() -> Path:
    out = subprocess.run(["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True, check=True).stdout
    return Path(out.strip())


def prompts(directory: Path) -> list[Prompt]:
    """Every numbered prompt in `directory`, in rank order."""
    found: list[Prompt] = []
    for path in sorted(directory.glob("*.md")):
        match = NAME_RE.match(path.stem)
        if match is None:
            continue
        number = match["number"]
        found.append(
            Prompt(
                path=path,
                number=int(number),
                width=len(number),
                suffix=match["suffix"] or "",
                slug=match["slug"],
            )
        )
    found.sort(key=lambda p: (p.number, p.suffix))
    return found


# --------------------------------------------------------------------------- rewriting


def _renumber(token: str, moves: dict[int, int]) -> str:
    """Map one numeric token through `moves`, keeping its zero padding."""
    new = moves.get(int(token))
    if new is None:
        return token
    return f"{new:0{len(token)}d}"


def rewriter(moves: dict[int, int], stems: dict[str, str], bare: bool) -> re.Pattern[str]:
    """One pattern matching every construct that may name a prompt.

    Alternatives are ordered longest-context-first so that a file stem is never
    torn apart by the plain-number rule that follows it.
    """
    number = r"\d{2,3}"
    span = rf"{number}(?:\s*[{DASH}-]\s*{number})?"
    listed = rf"{span}(?:\s*(?:,|/|and|or|,\s*and|,\s*or)\s*{span})*"
    alternatives = [
        rf"(?P<stem>\b(?:{'|'.join(re.escape(s) for s in sorted(stems, key=len, reverse=True))})\b)"
        if stems
        else r"(?P<stem>(?!x)x)",
        r"(?P<frontmatter>^(?:id|depends_on):[^\n]*$)",
        rf"(?P<link>\[{number}\](?=\((?:[^)\s]*/)?{number}[a-z0-9]*-[^)\s]*\.md\)))",
        rf"(?P<cued>\b[Pp]rompts?[\s-]+{listed})",
        rf"(?P<range>\b{number}\s*{DASH}\s*{number}\b)",
        rf"(?P<bare>\b{number}\b)" if bare else r"(?P<bare>(?!x)x)",
    ]
    return re.compile("|".join(alternatives), re.MULTILINE)


def rewrite(
    text: str, moves: dict[int, int], stems: dict[str, str], ranks: range, *, bare: bool
) -> tuple[str, list[str]]:
    """Rewrite every prompt reference in `text` in one pass.

    Returns the new text and the uncued ranges it took to be prompt ranges,
    which the caller should show: an uncued ``115-125`` is a block of prompts
    in one document and a range of measures in another.
    """
    pattern = rewriter(moves, stems, bare)
    digits = re.compile(r"\d{2,3}")
    inferred: list[str] = []

    def replace(match: re.Match[str]) -> str:
        kind = match.lastgroup
        body = match.group()
        if kind == "stem":
            return stems.get(body, body)
        if kind == "range":
            # An uncued range is a rank span only if it lies inside the stack
            # and the shift actually reaches it.
            ends = [int(end) for end in digits.findall(body)]
            if not all(end in ranks for end in ends) or not any(end in moves for end in ends):
                return body
            inferred.append(body)
        return digits.sub(lambda token: _renumber(token.group(), moves), body)

    return pattern.sub(replace, text), inferred


def text_files(root: Path) -> Iterator[Path]:
    for path in sorted(root.rglob("*")):
        if not path.is_file() or path.suffix not in TEXT_SUFFIXES:
            continue
        if any(part in SKIP_DIRS for part in path.relative_to(root).parts):
            continue
        yield path


def unclaimed(before: str, after: str, moves: dict[int, int]) -> list[tuple[int, str]]:
    """Lines the rewrite passed over that still say "prompt" next to a moved rank.

    A line the rewrite already changed is not reported: it had its chance. What
    is left is prose the cues did not reach — a reference spelled some way this
    script does not know, or a number that was never a rank at all.
    """
    stale = re.compile(rf"[Pp]rompts?\b.*\b(?:{'|'.join(str(n) for n in sorted(moves))})\b")
    old = before.split("\n")
    return [
        (n, line)
        for n, line in enumerate(after.split("\n"), 1)
        if stale.search(line) and n <= len(old) and old[n - 1] == line
    ]


# --------------------------------------------------------------------------- planning


def shift(stack: list[Prompt], at: int, count: int) -> dict[int, int]:
    """Move every rank at or after `at` up by `count`."""
    moving = [p for p in stack if p.number >= at]
    if not moving:
        raise RenumberError(f"no prompt is numbered {at} or later")
    return {p.number: p.number + count for p in moving}


def relocate(stack: list[Prompt], one: Prompt, to: int) -> dict[int, int]:
    """Move a single prompt to rank `to`, closing and opening the ranks around it."""
    if to == one.number:
        raise RenumberError(f"{one.stem} is already {to}")
    moves = {one.number: to}
    if to < one.number:
        for p in stack:
            if to <= p.number < one.number:
                moves[p.number] = p.number + 1
    else:
        for p in stack:
            if one.number < p.number <= to:
                moves[p.number] = p.number - 1
    return moves


def renames(stack: list[Prompt], moves: dict[int, int]) -> list[tuple[Path, Path]]:
    return [(p.path, p.at(moves[p.number])) for p in stack if p.number in moves]


def apply(root: Path, stack: list[Prompt], moves: dict[int, int], bare_in: Iterable[Path], *, write: bool) -> int:
    """Report — and optionally perform — the whole transaction."""
    pairs = renames(stack, moves)
    collisions = {new for _, new in pairs} & {p.path for p in stack if p.number not in moves}
    if collisions:
        raise RenumberError("would overwrite: " + ", ".join(sorted(str(c) for c in collisions)))

    stems = {old.stem: new.stem for old, new in pairs}
    bare = {p.resolve() for p in bare_in}

    print(f"{len(pairs)} renames")
    for old, new in pairs:
        print(f"  {old.name} -> {new.name}")

    ranks = range(min(p.number for p in stack), max(p.number for p in stack) + 1)
    touched: list[tuple[Path, str]] = []
    review: list[str] = []
    guessed: list[str] = []
    for path in text_files(root):
        if path.resolve() == Path(__file__).resolve():
            continue  # this script's own examples are not references
        before = path.read_text(encoding="utf-8")
        after, inferred = rewrite(before, moves, stems, ranks, bare=path.resolve() in bare)
        if after != before:
            touched.append((path, after))
        guessed.extend(f"  {path.relative_to(root)}: uncued range {span}" for span in inferred)
        review.extend(
            f"  {path.relative_to(root)}:{line_no}: {line.strip()[:110]}"
            for line_no, line in unclaimed(before, after, moves)
        )

    print(f"\n{len(touched)} files rewritten")
    for path, _ in touched:
        print(f"  {path.relative_to(root)}")

    if guessed:
        print(f"\n{len(guessed)} uncued ranges read as prompt ranks:")
        for line in guessed:
            print(line)

    if review:
        print(f"\n{len(review)} lines still mention a moved number — review by hand:")
        for line in review[:80]:
            print(line)
        if len(review) > 80:
            print(f"  ... and {len(review) - 80} more")

    if not write:
        print("\ndry run; pass --apply to write")
        return 0

    # Content first, under the names the rewrite pass read. Renaming first and
    # writing afterwards would recreate every file at its old name.
    for path, after in touched:
        path.write_text(after, encoding="utf-8")
    # Then rename through a scratch label: shifting up moves 125 onto 126 while
    # 126 still exists, and git mv will not clobber.
    scratch = [(old, new, new.with_suffix(".renaming")) for old, new in pairs]
    for old, _, temporary in scratch:
        subprocess.run(["git", "mv", str(old), str(temporary)], check=True, cwd=root)
    for _, new, temporary in scratch:
        subprocess.run(["git", "mv", str(temporary), str(new)], check=True, cwd=root)
    print("\napplied")
    return 0


# --------------------------------------------------------------------------- audit


def audit(stack: list[Prompt]) -> int:
    """Check that the stack's numbering says the same thing everywhere."""
    problems: list[str] = []
    by_label: dict[str, Prompt] = {}
    for prompt in stack:
        if prompt.label in by_label:
            problems.append(f"duplicate rank {prompt.label}: {by_label[prompt.label].path.name}, {prompt.path.name}")
        by_label[prompt.label] = prompt

    for prompt in stack:
        text = prompt.path.read_text(encoding="utf-8")
        declared = ID_RE.search(text)
        if declared is None:
            problems.append(f"{prompt.path.name}: no id in frontmatter")
        elif declared["label"] != prompt.label:
            problems.append(f"{prompt.path.name}: id is {declared['label']}, filename says {prompt.label}")
        depends = re.search(r"^depends_on:\s*\[(?P<ids>[^\]]*)\]", text, re.MULTILINE)
        if depends is None:
            problems.append(f"{prompt.path.name}: no depends_on in frontmatter")
            continue
        problems.extend(
            f"{prompt.path.name}: depends_on {dep}, which is not a prompt"
            for dep in (d.strip() for d in depends["ids"].split(",") if d.strip())
            if dep not in by_label
        )

    ranks = sorted({p.number for p in stack})
    gaps = [n for n in range(ranks[0], ranks[-1]) if n not in set(ranks)] if ranks else []
    if gaps:
        problems.append("unused ranks: " + ", ".join(str(g) for g in gaps))

    print(f"{len(stack)} prompts, ranks {ranks[0]}..{ranks[-1]}" if stack else "no prompts")
    for problem in problems:
        print(f"  {problem}")
    return 1 if problems else 0


# --------------------------------------------------------------------------- cli


def main(argv: list[str] | None = None) -> int:
    root = repo_root()
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--dir", type=Path, default=root / "docs" / "plan" / "prompts", help="prompt directory")
    parser.add_argument("--apply", action="store_true", help="write the plan instead of printing it")
    parser.add_argument(
        "--bare-in",
        type=Path,
        action="append",
        default=[],
        help="a file whose every prompt-numbered integer is a prompt reference (repeatable)",
    )
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("audit", help="check ids, dependencies, and gaps")
    room = commands.add_parser("make-room", help="shift a tail of the stack up")
    room.add_argument("--at", type=int, required=True)
    room.add_argument("--count", type=int, default=1)
    mover = commands.add_parser("move", help="move one prompt to another rank")
    mover.add_argument("prompt", help="filename or stem of the prompt to move")
    mover.add_argument("--to", type=int, required=True)
    args = parser.parse_args(argv)

    stack = prompts(args.dir)
    if not stack:
        raise RenumberError(f"no prompts under {args.dir}")

    if args.command == "audit":
        return audit(stack)

    default_bare = [args.dir / "README.md"]
    bare_in = args.bare_in or [p for p in default_bare if p.exists()]

    if args.command == "make-room":
        moves = shift(stack, args.at, args.count)
    else:
        stem = Path(args.prompt).stem
        chosen = next((p for p in stack if p.stem == stem or p.path.name == args.prompt), None)
        if chosen is None:
            raise RenumberError(f"no prompt named {args.prompt}")
        moves = relocate(stack, chosen, args.to)

    return apply(root, stack, moves, bare_in, write=args.apply)


if __name__ == "__main__":
    try:
        sys.exit(main())
    except RenumberError as error:
        print(f"error: {error}", file=sys.stderr)
        sys.exit(2)
