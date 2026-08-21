---
id: 183
slug: pinned-package-imports
status: pending
depends_on: [99, 104, 110, 174, 182]
phase: 4
---

# A Library May Live Elsewhere Without Making Builds Implicit

> **Governed by the event-track and machine core installed by prompts 127a–127e and 171–174.** This prompt adds exact
> source packages only; it promises no persistent compiled identity or cache.

## Task

Extend local Musa libraries to exact-pinned remote source/asset packages without building a registry or generalized
dependency solver. `musa fetch` is the sole network action; checking, editing, playback, and export resolve a verified
lockfile and work offline.

## Read

- `docs/rules/language/04-templates-and-modules.md` and `09-assets-and-packages.md`; prompt 36 import rules, prompt 84a
  project, prompt 99 bundled library, prompt 104 namespaces/signatures, prompt 182 assets.
- Roadmap §16's “relative imports are sufficient” and §19's registry/solver rejection. Repair the first deliberately
  while retaining the second; explain why exact fetching is a different capability from version solving.
- Git invocation/security, cache, lockfile, offline, and diagnostic code in `musa-project`/CLI. Prefer a narrow library
  over shelling out only after comparing both boundaries.

## Design

A remote package is prompt 110's package — the same `musa.toml`, source root, and module tree — fetched by exact pin
instead of bundled. This prompt adds a fetch layer and lockfile and no second notion of package. The resolved package
graph and the source module-import relation are separate data: one declared package dependency may have zero, one, or
many imported modules. Never label a package edge with a requested module path.

`musa.toml` gains a package id, immutable URL, and exact revision/content expectation. `musa.lock` records the resolved
revision, the complete canonical list of logical paths and exact source/asset bytes, declared package edges, asset
metadata, and format version. A digest may locate a candidate tree but never establishes equality. After a digest hit,
compare the complete framed path/byte table. No version ranges, registry lookup, “latest”, or conflict resolution exist;
two incompatible exact identities for one package id are a diagnostic.

Build one finite resolved package DAG. Repeated imports of the same exact resolved package share one build node and one
build-local nominal type identity; two distinct resolved nodes do not become equal merely because their public
interfaces match. Module requests are resolved inside the selected package node. Reject a lock graph whose equal package
descriptor has unequal outgoing alias targets.

`musa fetch` validates and installs into a content-addressed cache through a temporary directory and atomic publish.
Normal compilation never accesses the network. `--locked` rejects manifest/lock drift; an explicit update command is the
only way to change an identity. Package imports are namespaced by prompt 110's module paths; packages may export Musa
source, profiles, instruments, and data assets, but no native executable code.

Specify authentication/private-source behavior by refusing unsupported credentials rather than harvesting ambient
secrets. Bound archive/file counts and sizes; reject traversal, symlink escape, digest mismatch, and ambiguous casing.
Offline diagnostics name the exact missing package and fetch command.

Do not serialize checked values, type ids, machine values, or evaluator artifacts. A later build may reuse source bytes
and parse results only under a separately specified exact key. This prompt defines no stable ABI, persistent compiled
identity, execution-closure key, or compiled-value cache.

## Target

- Deliberate roadmap/events/candidate-spec repair and exact manifest/lock schemas.
- Project resolver/cache and CLI fetch/update/locked/offline behavior with local test remotes.
- Namespaced package imports, read-only definition navigation, asset integration, dependency graph/cycle/conflict tests.
- Reproducibility law: a locked project resolves to the same exact source+asset closure with the network unavailable;
  forced digest collisions cannot substitute different bytes.

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
- No persistent compiled type identity, ABI, compiled-value cache, or hash-only equality.
