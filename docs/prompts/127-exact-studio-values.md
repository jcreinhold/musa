---
id: 127
slug: exact-studio-values
status: pending
depends_on: [93, 126]
phase: 3
---

# Written Sound Values Stay Exact

## Task

Repair the studio intent model so a written decimal or ratio with a unit remains exact, source-spelled intent until
audio preparation. Remove the current eager `f64` conversion from `StudioSpec`; make the one rational/unit→DSP-value
boundary explicit, tested, and shared by native instruments, samples, mix levels, and later automation.

## Read

- `docs/language/08-performance-and-sound.md` exactness law; roadmap §§2, 7.2, 10.6, 13.7.
- Compiler `Value`, profile rational settings, source spans used by prompt 31's structured edits, and every
  `as_linear`/`as f32`/`as f64` conversion in compiler and audio.
- Prompt 93's eager-studio-float expected-change entry.

## Design

Introduce one private/compiler-facing written-quantity representation containing an exact rational magnitude, unit, and
enough spelling/span information for diagnostics and token-scoped edits. Decimal syntax denotes the exact decimal
rational; unit normalization (`ms` versus `s`) is exact. Equality used for compilation is exact value plus dimension;
source equality still distinguishes spellings where the lossless CST does.

Conversion to `f32`/`f64`, dB→linear, filter coefficients, sample-rate ratios, and frame counts happens in `musa-audio`
plan preparation or the existing performance frame boundary, never during parsing or `StudioSpec` construction. Specify
rounding and finite/range failure. A UI edit preserves the written unit and replaces only its token; it does not rewrite
`30 ms` as `0.03 s`.

Avoid a generic units framework. Implement only the dimensions and operations with real Musa callers; prove exact unit
conversion and differential parity for previously accepted values.

## Target

- Exact written quantity in `musa-compiler`; migrated `StudioSpec`, processor arguments, sends, and compiler facts.
- One audited conversion module in `musa-audio`, private to plan preparation.
- Laws for decimal/ratio equality, unit conversion, edit spelling preservation, range diagnostics, and old-source audio
  parity within the previously documented floating tolerance.
- Remove prompt 93's eager-float ledger entry.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-audio -p musa-project -p musa-lsp
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-audio -p musa-project -p musa-lsp -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
rg -n "WrittenQuantity|exact decimal|DSP boundary" docs/language crates/musa-compiler crates/musa-audio
```

Commit as `Keep written studio values exact`.

## Stop

- No arbitrary dimensional-analysis algebra or new public units crate.
- No float musical beat positions and no claim that transcendental DSP conversion is rational.
- No processor, routing, instrument, sample, or control feature.
