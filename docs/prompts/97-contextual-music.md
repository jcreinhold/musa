---
id: 97
slug: contextual-music
status: done
depends_on: [49, 63, 96]
phase: 3
---

# Contextual Music and One Elaboration Path

## Task

Introduce the private contextual `Music` value and migrate motifs, fragments, named bars, `use`, repeat, and existing
transform blocks to desugar through it. A `music` value remains open until use; instantiation at an environment and
absolute beat yields a well-scoped kernel fragment; closing the piece emits one checked `Term<ScoreFact>`. Preserve the
meaning, diagnostics, sharing, realization, and Origin paths of every existing source file.

## Read

- `docs/language/00-semantics.md` and `docs/kernel/06-surface-elaboration.md`.
- Prompts 34, 49, 63–75: transformations, reference marks/sharing, context tracks, material-placement refusals, and
  realization.
- Current `elaborate.rs`, `resolve.rs::ExpandCx`, `Share`, `Origin`, `ReferenceMark`, and term-closing code.

## Design

Implement the semantic interface privately:

```text
instantiate : Music × ElabEnv × Beat -> Checked KernelFragment
close       : KernelFragment -> closed Term[ScoreFact]
```

`KernelFragment` hides its term, compatible acyclic bindings, exact extent, and checks. Composition unions bindings and
uses kernel `seq`/`over`; it never flattens `Timeline<Timeline<A>>`. Absolute placement participates in bar/tuplet and
prevailing-context checks. `close` emits dominating `let`s once and then runs `Term::check`.

`Music` is context-reading but context-neutral. Key, meter, tempo, and clef statements remain legal only in the
structural piece/voice walk and are rejected inside reusable material exactly as today. Notes, rests, local marks,
regions, and later lexical `in scale` are material. State mutation or an order-dependent merge for overlay is not an
implementation shortcut.

General `fn ... -> music` and `let x: music` become real. `motif` desugars to a named music-producing function plus a
`Motif` role; `fragment` and a named bar desugar to music bindings with their distinct roles/assertions. `use e`
requires `music` and sequences its instantiation. Provenance roles and reference marks survive the desugaring; projected
snapshots and existing kernel goldens remain byte-identical unless the governing spec names a deliberate representation
change.

## Target

- Private contextual-music implementation inside `musa-compiler`; delete the old parallel motif substitution path.
- General functions/bindings returning `music`; existing forms as one documented desugaring table.
- Context-neutrality diagnostics and placement-aware instantiation.
- `crates/musa-compiler/tests/{music_laws,music_compatibility}.rs`: sequence/overlay extent laws, binding closure,
  context-change rejection, per-use placement checks, sharing/provenance, and prompt 93's compatibility manifest.
- A minimal new fixture showing `fn figure(root: pitch) -> music`, `let`, and `use` alongside legacy `motif` syntax.
  (`phrase` remains the reserved span construct `phrase "A" { … }`; this prompt does not add escaped identifiers.)

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
for f in examples/*.musa; do cargo run -q -p musa -- check "$f"; done
cargo bench -p musa-compiler
```

Commit as `Elaborate contextual music through one path`.

## Stop

- No ambient scale yet — prompt 101. Prove openness with the context and placement fields already implemented.
- No new kernel form and no public `Music`, fragment, closure, HIR, or environment type.
- No context-changing fact inside reusable `music`; structural parameterization is prompt 103.
- No higher-order music traversal or canon library — prompt 98.
- No cache optimization. A correctness-only conservative key is allowed; measurement-driven caching is prompt 127.
