---
id: 110
slug: packages-and-module-trees
status: done
depends_on: [99, 109]
phase: 3
---

# A Package, a Module Tree, and a Manifest That Is Load-Bearing

## Task

Replace the standard library's flat file list and decorative manifest with a real package: a directory with `musa.toml`,
a source root whose `lib.musa` declares its children with `mod`, directory modules that declare their own in `mod.musa`,
and module paths that nest to any depth. Resolution follows those declarations and never scans the directory, so a
source file no `mod` reaches is rejected rather than silently unreachable. The four hand-maintained parallel lists in
`crates/musa-compiler/src/imports.rs` collapse to one table derived from that traversal.

## Read

- `docs/language-correction.md` §3, which governs this prompt.
- `docs/language/04-templates-and-modules.md`'s repaired source-library boundary, and §4 for the static module system
  this is deliberately *not* — signatures and modules are a checking-time abstraction, not a second import mechanism.
- `docs/language/01-surface.md` on nested module paths, flat binding, and the alias-at-a-collision rule.
- Prompt 99, which built the bundled library and mandated the layout this prompt corrects.
- Prompt 134 (pinned package imports), which inherits this shape and must not invent a second one.

## Design

The manifest today is compiled under `#[cfg(test)]` and consulted by nothing. Real resolution is four parallel lists —
URI constants, `include_str!` constants, a `match` returning source, and a packaging array — that must be edited in
lockstep and are checked against each other by nothing at all. The failure that permits is on disk at the time of
writing: `stdlib/sequences.musa` is committed, is in neither the manifest nor any of the four lists, is unreachable from
any import, and the workspace builds green. A standard-library module was written, reviewed, and merged without becoming
part of the standard library.

So the correction is not "add a fifth list that checks the other four." It is that resolution has **one hand-edited
input**: the `mod` declarations, which are ordinary Musa source read by the ordinary parser. Two errors follow, and both
are the point — a `.musa` file under the source root that no `mod` reaches is *declared nowhere*, and a `mod` naming no
file is *missing*. Either would have caught `sequences.musa` at the commit that introduced it.

Source text is still embedded at build time, because the library must be readable at its `musa-stdlib:/std/…` URIs with
no filesystem access. What changes is that the embedding is *generated* from the traversal rather than transcribed
beside it. Whether that generation is a build script or a checked-in generated file is the worker's decision; that it
has exactly one hand-edited input is not, and a test that adds a file without a `mod` and expects rejection is what
proves it.

Nesting is why `std::tonal::harmony` exists: a bundled module's virtual URI is its file name, so the underscore is a
missing feature wearing the costume of a naming preference. With a module tree it becomes `std::tonal::harmony`, and the
library gets the shape its contents already have — the tonal constructors, the diatonic sequences, and the schemas under
`tonal/`; the pitch-class, set, and row work under `post_tonal/`.

Binding stays **flat**, which is a deliberate departure from Rust and from §4's `Module.member` rule for static modules,
and the reason is the reader. Qualification is information to someone building an abstraction and noise to a musician
reading a score, where `harmony.` would be charged on every line to prevent a collision that has not happened. The
collision is handled where it occurs instead: importing two modules that export the same name is an error naming both,
and prompt 109's `as` clause resolves it by qualifying one. An alias is therefore required exactly at a real conflict
and absent otherwise.

`sequences.musa` is registered by this prompt rather than deleted — it is prompt 115's material, it is already written,
and leaving it orphaned while adding the machinery that exists to catch orphans would be its own joke.

## Target

- `stdlib/musa.toml` and `stdlib/src/`, with `lib.musa` declaring the tree, `tonal/mod.musa` and `post_tonal/mod.musa`
  declaring theirs, and every existing module moved to its position.
- The `mod` declaration in the lexer, parser, CST, and formatter; module-path resolution to arbitrary depth.
- One derived source table replacing the four parallel lists in `imports.rs`, with the manifest and the `mod` tree as
  its only hand-edited inputs.
- The *declared nowhere* and *missing module* diagnostics, each with a test that introduces the fault and expects
  rejection — including one that adds a stray file to the source root.
- The duplicate-export collision diagnostic naming both modules, and its resolution by `as`.
- Every `use std::…;` site migrated to its nested path across `examples/`, `stdlib/`, and the test corpora;
  `stdlib/reference.md` regenerated with paths rather than flat names.
- `stdlib/sequences.musa` registered at `std::tonal::sequences`.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/tonal-construction.musa
cargo run -p musa -- check examples/neo-riemannian.musa
cargo run -p musa -- render examples/tonal-construction.musa --to musicxml -o /tmp/tonal.musicxml
```

Commit as `Make the standard library a package with a module tree`.

## Stop

- No registry, no version-range solver, no network access during compilation, and no dependency resolution of any kind.
  Prompt 134 adds exact pinned fetching on top of this shape and nothing here anticipates it.
- No directory scanning, no implicit module discovery, and no convention that a file's presence makes it importable.
- No prelude, no implicit import, no glob import, and no re-export (`pub use`) form.
- No `module` value, no first-class module, and no change to §4's static signature/module/functor layer.
- Do not qualify bindings by default to "match Rust"; the flat-binding decision is made in `docs/language-correction.md`
  §3 and reversing it is a specification repair, not an implementation choice.
