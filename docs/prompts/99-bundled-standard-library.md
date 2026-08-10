---
id: 99
slug: bundled-standard-library
status: done
depends_on: [36, 98]
phase: 3
---

# A Source Standard Library, Not a Compiler Vocabulary Dump

## Task

Create the bundled `.musa` standard-library mechanism that later theory prompts populate. Standard functions must be
ordinary Musa source compiled through the same type checker and evaluator as user libraries; only operations that need
hidden `music` representation, provenance, or kernel construction remain registered primitives. Give imports one
stable, installation-independent spelling and preserve source locations into bundled files.

## Read

- `docs/language/04-templates-and-modules.md` and the primitive-versus-library admission rule in
  `docs/language/00-semantics.md`.
- Prompt 36 import resolution, prompt 84/the-project-is-the-unit, project manifests, packaging, and CLI install paths.
- Every current compiler primitive/keyword table; classify it by information ownership before moving or adding it.

## Design

Add one reserved import namespace chosen by the governing surface spec, for example `use std::pitch;`; do not search the
working directory or environment for it. The compiler/project package embeds or resolves version-matched source at a
deterministic path, parses/checks it once per compilation graph, detects cycles with ordinary import diagnostics, and
reports definitions/diagnostics against readable virtual standard-library URIs.

The first library is intentionally small: exact rational helpers, list/option combinators, function composition, and
named wrappers over already-existing primitives. Its purpose is to prove the boundary and source maps, not to preempt
prompts 100–115. A test enumerates registered primitives and requires each to cite which hidden information justifies
compiler ownership. There is no `musa-theory` crate and no Rust reimplementation of a source function for speed until
prompt 125 measures that function as a bottleneck and proves equivalence.

## Target

- Bundled `stdlib/` layout, manifest/version rule, deterministic import resolver, and packaging for CLI/LSP/desktop.
- Minimal `std::core`/`std::list`/`std::option` source libraries and their reference docs generated from source
  comments.
- Cross-file definition, hover, diagnostics, rename refusal for bundled read-only source, and source-map tests.
- Primitive-ownership registry/test inside `musa-compiler`; private implementation details remain private.
- `examples/stdlib-basics.musa` using explicit imports and no compiler-only spelling.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -p musa -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/stdlib-basics.musa
cd apps/musa-desktop/ui && npm test
```

Commit as `Add the bundled Musa standard library`.

## Stop

- No pitch, scale, chord, serial, transformational, schema, or analysis library yet.
- No network package manager, registry, user-global library path, lockfile, or semver solver.
- No new public Rust theory API or pass-through standard-library crate.
- No transparent prelude import beyond the exact small set chosen in `01-surface.md`; hidden names make source harder
  to understand.
