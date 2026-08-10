---
id: 108
slug: domain-metatheory
status: in-progress
depends_on: [95, 96, 97, 98, 107]
phase: 3
---

# The Musical Domains Become a Conservative Extension

## Task

Discharge the proof obligation that prompts 100–107 accumulated and never paid. `02-core-calculus.md` §5 proves
preservation, progress, determinism, and strong normalization for a fragment whose base types are exactly `bool`,
`nat`, `ratio`, `duration`, `pitch`, and `interval`, and says in its own words that later additions "are not smuggled
into this theorem by an appeal to 'standard STLC.'" Twelve musical base types and sixty-nine compiler-owned operations
have since entered with no compatibility case. Prove §5.8's conservative-extension theorem once, parametrically, and
make the primitive-ownership registry the mechanically checked witness for its premises — so that the next domain costs
a registry entry rather than a new induction, and cannot be added without one.

## Read

- `docs/language-correction.md` §2, which governs this prompt.
- `docs/language/02-core-calculus.md` §5–§5.8: the fragment, its four theorems, the finite-data and contextual-music
  extensions, and the new §5.8 this prompt implements.
- `docs/language/03-musical-domains.md` for the domain definitions and the counterexamples each one rules out.
- The existing law suites from prompts 95–98: they are the model for how a metatheoretic obligation is tested here, and
  the registry-length law in `crates/musa-compiler/src/core.rs` is the model for a checked premise.

## Design

The obligation is real and must not be met twelve times. Twelve near-identical inductions would be written once, read
never, and rot silently as prompts 111–116 add more domains. §5.8 therefore states one theorem parametric in the base
type, and this prompt's work is to make its premises *checked facts about the implementation* rather than claims in
prose.

Every compiler-owned primitive belongs to exactly one of three families, and disjointness and exhaustiveness are
laws rather than comments:

- **δ-primitives**, whose argument and result types are base types or finite constructors (`option`, `list`, product)
  over base types, with no arrow anywhere in the signature. Every musical operation added by prompts 100–107 is one.
- **structural eliminators** — `nat_fold`, `list_fold`, `option_fold`, `map`, `filter`, `range`, `repeat` — already
  proved in §5.6, and the only primitives that take function arguments.
- **music primitives**, already proved in §5.7 and constrained by prompt 98.

A δ-primitive must satisfy D1 inertness, D2 totality, D3 purity, and D4 finiteness as §5.8 states them. Two of the four
are the interesting ones in this codebase. **D2** is why `scale_chord` returns `option[chord_class]` and reports
absence for a collection stacking to no nameable sonority: partiality lives in the result type, and a primitive that
raised a diagnostic instead would break progress. **D1** is why no musical base type has a destructuring pattern —
a `scale` is an opaque constant and everything observable about it is observed by applying a δ-primitive.

`PRIMITIVE_OWNERSHIP` already records each operation's spelling and the information it hides. It gains a family
classification and a declared signature, and the declared signature becomes the single place a primitive's arity and
types are stated — the checker's arity groups and the evaluator's destructuring read from it rather than restating it,
which is how the "sixty-nine" entries stopped agreeing with the checker twice during prompt 107.

The sampling law is the one place judgment is required. A domain that is finite is sampled exhaustively; one that is
not is generated to a bound documented in the test and stable across runs, because a sampling law that varies per run
is not a law. The generator is seeded and deterministic, and the bound is a constant in the test rather than a
property-test configuration knob.

This prompt adds no base type, no primitive, and no musical vocabulary. If discharging D1–D4 exposes an existing
primitive that violates one of them, that is a finding: repair the primitive under this prompt and record what it was,
because a violation found by the gate is exactly the gate working.

## Target

- `PrimitiveOwnership` gains a family and a declared signature; the checker and evaluator read arity and types from
  that declaration instead of restating them.
- The inert base-type list, and a checker gate rejecting any destructuring pattern over a musical base type with a
  located diagnostic naming the type.
- The law suite, proving: every primitive classified exactly once; families disjoint and exhaustive; no δ-primitive
  signature containing an arrow; every base type reachable from a δ signature inert; and every δ-primitive returning a
  value of its declared type over its sample — never panicking, never diagnosing, never absent at a non-`option`
  result type.

  It lives in `core.rs`'s test module beside the existing `every_compiler_owned_operation_names_its_hidden_information`
  law, **not** in `crates/musa-compiler/tests/`. `Type`, `Value`, and `Primitive` are `pub(crate)`, and an integration
  test could only reach them by widening the public surface for a test's benefit — which the deep-module rule forbids
  and which would make the registry's own boundary the first casualty of the prompt that exists to defend it. The
  registry law it joins is already a unit test for exactly this reason.
- The deterministic sampler, with per-domain bounds stated as named constants and the finite domains marked exhaustive.
- `03-musical-domains.md` gains, for each of the twelve domains, the row §5.8's budget requires if it lacks one: its
  definition, its source, and a counterexample it rules out.

## Check

```sh
cargo nextest run -p musa-compiler
cargo clippy --all-targets -p musa-compiler -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/tonal-construction.musa
```

Commit as `Prove the musical domains a conservative extension`.

## Stop

- No new base type, primitive, chord type, scale, or musical vocabulary of any kind.
- No proof assistant, no extracted implementation, no runtime type reflection, and no public Rust theory surface.
- Do not claim the theorem says anything about musical correctness. It is about type safety; whether a German sixth
  spells its top note correctly is a separate law suite and stays one.
- Do not convert a δ-primitive's `option` result into a diagnostic to make a signature tidier; that inverts D2.
- No performance work. Prompt 121 owns the evaluator's measured envelope.
