---
id: 175
slug: exact-studio-values
status: done
depends_on: [93, 174b]
phase: 3
---

# Written Sound Values Stay Exact in Source

> **Reopened by note 79.** The first execution correctly removed eager floats and centralized DSP conversion, but made
> public Rust `WrittenQuantity` part of the language vocabulary. This execution keeps the arithmetic and moves the
> declarable quantity/unit model to `std::sound`.

## Task

Make a written decimal or ratio with a unit remain exact, source-spelled intent through checking and every exact
projection, until audio preparation performs the one rational/unit→DSP conversion. Establish the source quantity types
before prompt 176 uses them in processor declarations, while retaining one measured, tested conversion boundary for
native instruments, samples, mix levels, and later automation.

## Read

- `docs/rules/language/08-performance-and-sound.md` §§0 and 7; roadmap §§2, 7.2, 10.6, 13.7; note 79.
- Repaired 174b, note 82, and the exact `ParameterValue` declarations in `stdlib/src/sound/graph.musa`.
- Compiler rational literals and unit tokens, source spans used by structured edits, and every `as_linear`/`as f32`/`as
  f64` conversion in compiler, project, and audio.
- The first prompt-175 commit's conversion laws as evidence to preserve, not as authority for the public Rust type.

## Design

Declare only the dimensions and units with real callers as ordinary Musa data. An exact quantity contains a reduced
rational magnitude and a unit/dimension fixed by its source declaration. Decimal syntax denotes its exact decimal
rational; `ms`↔`s` normalization is exact. The lossless CST owns spelling, so exact value equality may identify `30 ms`
and `0.03 s` while source equality and token-scoped edits preserve the written form.

The DSP projection carries exact rational/unit data decoded from one checked source value. It has private fields or an
opaque constructor, no defaults beyond those declared in source, and a differential law against the source canonical
encoding. Conversion to `f32`/`f64`, dB→linear, filter coefficients, sample-rate ratios, and frame counts occurs only in
`musa-dsp` preparation. Specify rounding, finite/range failure, and the exact written value in diagnostics. Existing
legacy Rust quantity types may remain only for callers on the temporary production oracle: add no caller and assign no
compatibility status; prompt 180a deletes or privatizes them after complete parity.

Do not build a generic dimensional-analysis framework. Reuse `Ratio`, ordinary indexed data where a dimension must be
shared, and the existing pattern unifier for omitted indices. There is no unit-specific coercion or host inference
table.

## Target

- Exact source quantity/unit declarations in `std::sound` and migrated standard-library studio values.
- Opaque exact DSP projection derived only from checked source.
- One audited private conversion module at preparation, retaining the first execution's good exactness and parity laws.
- Compiler/project/LSP/UI facts that preserve written unit spelling through CST spans rather than Rust vocabulary.
- No new independently constructible Rust quantity semantics; legacy `WrittenQuantity`/`Unit` are explicitly marked as a
  temporary differential oracle for prompt 180a, with prompt 93's eager-float ledger row repaired.

## Check

```sh
cargo nextest run -p musa-calculus -p musa-syntax -p musa-compiler -p musa-dsp -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-calculus -p musa-syntax -p musa-compiler -p musa-dsp -p musa-project -p musa-lsp -- -D warnings
PATH=/Users/jcreinhold/.cargo/bin:$PATH make fmt-check
cargo insta test --workspace --unreferenced=reject
rg -n "exact decimal|DSP boundary|quantity" stdlib/src/sound docs/rules/language crates/musa-dsp
```

Commit as `Keep source sound quantities exact`.

## Stop

- No arbitrary dimensional-analysis algebra or new public units crate.
- No float musical beat positions or claim that transcendental DSP conversion is rational.
- No processor, routing, instrument, sample, or control feature.
- No new public Rust quantity constructor or caller that can create semantic intent without checked source; deletion of
  the temporary production oracle belongs to prompt 180a.
