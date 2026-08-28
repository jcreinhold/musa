#!/usr/bin/env python3
"""Check every production Cargo edge against the workspace dependency roles.

`LAYERS` is the single assignment table. `ALLOWED_DEPENDENCIES` states which
roles a crate in each role may name. Normal and build dependencies count,
including target-specific tables. Dev-dependencies are deliberately exempt:
tests may reach upward to build realistic fixtures without adding an edge to a
shipped artifact.

The live check also runs checked-in negative controls: one vocabulary crate
depends on the pipeline, and one workspace member is unassigned. Success
therefore proves both that the repository is valid and that the checker can
reject the two mistakes it exists to catch.
"""

from __future__ import annotations

import pathlib
import sys
import tomllib
from collections.abc import Iterator, Mapping
from dataclasses import dataclass

ROOT = pathlib.Path(__file__).resolve().parent.parent
CONTROLS = ROOT / "scripts" / "fixtures" / "layers"

# Each workspace package appears exactly once. Names describe what a crate is,
# not a vague vertical position.
LAYERS: dict[str, frozenset[str]] = {
    "theory": frozenset({"musa-syntax", "musa-calculus", "musa-events"}),
    "vocabulary": frozenset({"musa-score", "musa-dsp"}),
    "pipeline": frozenset({"musa-compiler"}),
    "consumer": frozenset({"musa-notation", "musa-playback"}),
    "session": frozenset({"musa-project"}),
    "shell": frozenset({"musa", "musa-au", "musa-lsp", "musa-wasm", "musa-desktop"}),
}

ALLOWED_DEPENDENCIES: dict[str, frozenset[str]] = {
    "theory": frozenset(),
    "vocabulary": frozenset({"theory", "vocabulary"}),
    "pipeline": frozenset({"theory", "vocabulary"}),
    "consumer": frozenset({"theory", "vocabulary"}),
    "session": frozenset({"theory", "vocabulary", "pipeline", "consumer"}),
    "shell": frozenset({"theory", "vocabulary", "pipeline", "consumer", "session"}),
}


@dataclass(frozen=True)
class Member:
    """One workspace package and its parsed manifest."""

    name: str
    manifest: pathlib.Path
    data: Mapping[str, object]


def assignments() -> tuple[dict[str, str], list[str]]:
    """Invert the role table, reporting an accidental double assignment."""
    by_package: dict[str, str] = {}
    problems: list[str] = []
    for role, packages in LAYERS.items():
        for package in packages:
            previous = by_package.setdefault(package, role)
            if previous != role:
                problems.append(f"{package} is assigned to both {previous} and {role}")
    return by_package, problems


def load_manifest(path: pathlib.Path) -> Mapping[str, object]:
    """Parse one Cargo manifest with Python's standard-library TOML reader."""
    with path.open("rb") as source:
        return tomllib.load(source)


def workspace_members(root: pathlib.Path) -> tuple[list[Member], list[str]]:
    """Expand the root workspace's member patterns into named manifests."""
    root_data = load_manifest(root / "Cargo.toml")
    workspace = root_data.get("workspace")
    if not isinstance(workspace, dict) or not isinstance(workspace.get("members"), list):
        return [], [f"{root / 'Cargo.toml'} has no workspace member list"]

    def expand(patterns: object) -> set[pathlib.Path]:
        found: set[pathlib.Path] = set()
        if not isinstance(patterns, list):
            return found
        for pattern in patterns:
            if not isinstance(pattern, str):
                continue
            for candidate in root.glob(pattern):
                manifest = candidate / "Cargo.toml" if candidate.is_dir() else candidate
                if manifest.is_file():
                    found.add(manifest)
        return found

    manifests = expand(workspace["members"])
    for manifest in expand(workspace.get("exclude")):
        manifests.discard(manifest)

    members: list[Member] = []
    problems: list[str] = []
    seen: set[str] = set()
    for manifest in sorted(manifests):
        data = load_manifest(manifest)
        package = data.get("package")
        name = package.get("name") if isinstance(package, dict) else None
        if not isinstance(name, str):
            problems.append(f"{manifest.relative_to(root)} has no package name")
            continue
        if name in seen:
            problems.append(f"workspace package {name} is declared more than once")
            continue
        seen.add(name)
        members.append(Member(name, manifest, data))
    return members, problems


def production_dependency_tables(data: Mapping[str, object]) -> Iterator[Mapping[str, object]]:
    """Yield dependency tables that contribute to a shipped artifact."""
    for key in ("dependencies", "build-dependencies"):
        table = data.get(key)
        if isinstance(table, dict):
            yield table
    targets = data.get("target")
    if not isinstance(targets, dict):
        return
    for target in targets.values():
        if not isinstance(target, dict):
            continue
        for key in ("dependencies", "build-dependencies"):
            table = target.get(key)
            if isinstance(table, dict):
                yield table


def dependency_name(key: str, value: object) -> str:
    """Resolve a renamed Cargo dependency to its package name."""
    if isinstance(value, dict) and isinstance(value.get("package"), str):
        return value["package"]
    return key


def check_workspace(root: pathlib.Path, *, require_complete_table: bool) -> tuple[list[str], int, int]:
    """Return every assignment or production-edge violation in a workspace."""
    by_package, problems = assignments()
    members, member_problems = workspace_members(root)
    problems.extend(member_problems)
    member_names = {member.name for member in members}

    for member in members:
        if member.name not in by_package:
            problems.append(f"{member.name} is a workspace member but has no dependency-role assignment")
    if require_complete_table:
        for package in sorted(by_package.keys() - member_names):
            problems.append(f"{package} is assigned to {by_package[package]} but is not a workspace member")

    edges = 0
    for member in members:
        source_role = by_package.get(member.name)
        if source_role is None:
            continue
        for table in production_dependency_tables(member.data):
            for key, value in table.items():
                target = dependency_name(key, value)
                if target not in member_names:
                    continue
                edges += 1
                target_role = by_package.get(target)
                if target_role is None or target_role in ALLOWED_DEPENDENCIES[source_role]:
                    continue
                allowed = ", ".join(sorted(ALLOWED_DEPENDENCIES[source_role])) or "no workspace roles"
                problems.append(
                    f"{member.name} is a {source_role} crate and names {target}, which is {target_role}; "
                    f"{source_role} crates may depend only on {allowed}"
                )
    return problems, len(members), edges


def main() -> int:
    """Check the live graph and prove the negative control is rejected."""
    problems, members, edges = check_workspace(ROOT, require_complete_table=True)
    backwards, _, _ = check_workspace(CONTROLS / "backwards", require_complete_table=False)
    expected_backwards = (
        "musa-score is a vocabulary crate and names musa-compiler, which is pipeline; "
        "vocabulary crates may depend only on theory, vocabulary"
    )
    if backwards != [expected_backwards]:
        problems.append(f"backwards-edge control did not produce its one expected violation: {backwards!r}")
    unassigned, _, _ = check_workspace(CONTROLS / "unassigned", require_complete_table=False)
    expected_unassigned = "musa-new is a workspace member but has no dependency-role assignment"
    if unassigned != [expected_unassigned]:
        problems.append(f"unassigned-member control did not produce its one expected violation: {unassigned!r}")

    if problems:
        for problem in problems:
            print(f"layer error: {problem}", file=sys.stderr)
        return 1
    print(
        f"layers: {members} members and {edges} production edges obey the table; "
        "dev-dependencies exempt; negative controls rejected"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
