---
id: 136
slug: pinned-package-imports
status: pending
depends_on: [99, 104, 110, 135]
phase: 4
---

# A Library May Live Elsewhere Without Making Builds Implicit

> **Contingent on prompt 126.** The core-boundary decision may repair this prompt's Design, fold it into another, or
> replace it. Read `docs/core-boundary.md` first.

## Task

Extend local Musa libraries to exact-pinned remote source/asset packages without building a registry or generalized
dependency solver. `musa fetch` is the sole network action; checking, editing, playback, and export resolve a verified
lockfile and work offline.

## Read

- `docs/language/04-templates-and-modules.md` and `09-assets-and-packages.md`; prompt 36 import rules, prompt 84
  project, prompt 99 bundled library, prompt 104 namespaces/signatures, prompt 135 assets.
- Roadmap §16's “relative imports are sufficient” and §19's registry/solver rejection. Repair the first deliberately
  while retaining the second; explain why exact fetching is a different capability from version solving.
- Git invocation/security, cache, lockfile, offline, and diagnostic code in `musa-project`/CLI. Prefer a narrow library
  over shelling out only after comparing both boundaries.

## Design

A remote package is prompt 110's package — the same `musa.toml`, the same source root, the same `mod` tree, the same
declared-nowhere and missing-module errors — fetched by exact pin instead of bundled. This prompt adds a fetch layer and
a lockfile and *no second notion of what a package is*. In particular, package imports use prompt 110's module paths,
not prompt 104's static `signature`/`module` layer: those are a checking-time abstraction over values, they are not an
import mechanism, and conflating the two is the mistake this sentence exists to prevent.

`musa.toml` gains the remote-source keys — a package id, immutable URL, and exact revision/content expectation — on top
of the fields prompt 110 already reads. `musa.lock` records resolved source revision, full package content digest,
declared dependencies, asset digests, and format version. No version ranges, registry lookup, “latest”, or conflict
resolution exist; two incompatible exact identities for one package id are a diagnostic. Transitive packages are allowed
only when fully exact and represented in the lock graph.

`musa fetch` validates and installs into a content-addressed cache through a temporary directory and atomic publish.
Normal compilation never accesses the network. `--locked` rejects manifest/lock drift; an explicit update command is the
only way to change an identity. Package imports are namespaced by prompt 110's module paths; packages may export Musa
source, profiles, instruments, and data assets, but no native executable code.

Specify authentication/private-source behavior by refusing unsupported credentials rather than harvesting ambient
secrets. Bound archive/file counts and sizes; reject traversal, symlink escape, digest mismatch, and ambiguous casing.
Offline diagnostics name the exact missing package and fetch command.

## Target

- Deliberate roadmap/course-correction/candidate-spec repair and exact manifest/lock schemas.
- Project resolver/cache and CLI fetch/update/locked/offline behavior with local test remotes.
- Namespaced package imports, read-only definition navigation, asset integration, dependency graph/cycle/conflict tests.
- Reproducibility law: a locked project resolves to the same source+asset closure with the network unavailable.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa -p musa-lsp -- -D warnings
cargo fmt --check
cargo deny check
cargo insta test --workspace --unreferenced=reject
```

Commit as `Add exact pinned Musa packages`.

## Stop

- No central registry, semver/range solver, automatic update, implicit compile-time network, or package-native code.
- No global mutable install directory as semantic authority; the lockfile and verified content identity are
  authoritative.
- No publication service or account system.
