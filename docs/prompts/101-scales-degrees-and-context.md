---
id: 101
slug: scales-degrees-and-context
status: done
depends_on: [65, 100]
phase: 3
---

# Scales, Degrees, Register, and Lexical Pitch Context

## Task

Separate key, scale, degree, and register in the type system; implement total scale values, located pitches, degree
stepping, and register frames; and add lexical `in scale` so the same open `music` binding can elaborate differently at
different use sites without emitting a key signature or pretending to infer modulation.

## Read

- `docs/language/03-musical-domains.md` and the contextual laws in `00-semantics.md`.
- OMT `013-major-scales-scale-degrees-and-key-signatures.md`, `014-minor-scales-scale-degrees-and-key-signatures.md`
  (natural/harmonic/melodic minor and the one minor key signature),
  `015-introduction-to-diatonic-modes-and-the-chromatic-scale.md`, `016-intervals.md`, `105-diatonic-modes.md`,
  `106-collections.md`, and `107-analyzing-with-modes-scales-and-collections.md`.
- Prompt 63/65 context tracks: a written key is a located score fact; material may read but not write context.

## Design

`scale` is a tonic spelled pitch class, a finite nonempty ordered cycle of spelled offsets, and a spelled period. A
`key` is a tonic/mode/key-signature fact. `signature_scale : key -> scale` supplies a default coordinate collection but
does not claim every pitch in the key belongs to that collection. Natural, harmonic, ascending melodic, descending
melodic minor, church modes, pentatonic, whole-tone, octatonic, hexatonic, and acoustic collections are distinct values.

`degree` is a scale-independent integer coordinate plus chromatic alteration. `scale_pitch` proves a written pitch is
located in one scale; `step` moves its coordinate. `pitch_frame` requires an absolute tonic pitch matching the scale's
tonic class and maps degrees to registered written pitches. Partial membership/frame construction returns `option` or a
source diagnostic; `degree(c minor, 5) : pitch` is never accepted because key/scale/register are missing or conflated.

`in scale s { m }` is Reader `local`: it changes only generative pitch coordinates for the enclosed contextual music,
emits no `Key` fact, and may add a `ScaleContext` Origin step. A `key` statement remains structural and updates the
default `signature_scale` for subsequent material. Prove/test shadowing, distribution over sequence/overlay,
independence from other environment fields, the open-binding law, degree periodicity, additive stepping, and explicit
non-commutation of chromatic interval motion with scale stepping.

## Target

- Domain values/operations and source syntax from the governing spec; `std::scale` and `std::collections` in Musa.
- Contextual `in scale`, default-from-key behavior, `ScaleContext` provenance, and exact diagnostics.
- LSP hover/completion for scale names and types; formatter/tree-sitter updates for the small new syntax.
- `examples/scale-context.musa`: one bound phrase used under C major and C dorian; explicit natural/harmonic minor;
  pentatonic and octatonic cases; a failure fixture with no scale/root membership.
- `crates/musa-compiler/tests/scale_context_laws.rs`: reference map, context laws, periodicity, step laws, minor
  distinctions, and chromatic/diatonic counterexample.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-lsp -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/scale-context.musa
cd editors/tree-sitter-musa && tree-sitter test
cargo bench -p musa-compiler
```

Commit as `Add scales, degrees, and lexical pitch context`.

## Stop

- No inferred key, modulation, tonicization, Roman numeral, or blanket out-of-key error.
- No `in meter`, `in key`, or `with key`; structural facts and lexical coordinates stay distinct.
- No hidden register default for degree-to-pitch.
- No tuning/frequency or generic pitch-system abstraction.
