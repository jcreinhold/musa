---
id: 106
slug: transformational-harmony-library
status: pending
depends_on: [102, 105]
phase: 3
---

# Neo-Riemannian Transformations on Their Exact Domain

## Task

Implement PLR and the named contextual transformations SNH as ordinary Musa standard-library functions over the
major/minor-triad refinement. Preserve spelling in the constructive domain, state finite pitch-class group claims only
after the explicit `pc12` projection, and make transformation chains compose as normal functions rather than keywords
or compiler cases.

## Read

- OMT `072-neo-riemannian-triadic-progressions.md`, especially Neo-Riemannian transformations, Tonnetz, chains/cycles,
  and the later SNH section.
- OMT `017-triads.md`, `061-modal-mixture.md`, and `068-equal-divisions-of-the-octave.md` for triad spelling and the
  equal-division collections used by the networks.
- Prompts 102 and 105; `docs/language/03-musical-domains.md` on refinements and quotient-specific laws.

## Design

`P`, `L`, `R`, `S`, `N`, and `H` have type `triad -> triad`: OMT 072 defines all six on major/minor triads. Each
function is defined by the tones it preserves and the directed motion of the other member or members, then implemented
from pitch/chord operations in `.musa`. Suspended, quartal, augmented, diminished, and altered chord classes are not
silently coerced into the domain.

Prove from the definitions that all six operations are involutions on spelled major/minor triads. Do not assert every
finite Neo-Riemannian group relation on the infinite spelled domain: project with `forget_spelling` to the `pc12`
triad action. State the quotient law there and test the corresponding finite Tonnetz cycles. Transformation-chain
examples must round-trip through chord spelling and a caller-chosen voicing policy; the library never sounds an unvoiced
chord class.

## Target

- `stdlib/transformational.musa`: PLR/SNH, chain helpers, explicit spelled-to-`pc12` projection helpers, source docs
  citing OMT 072 sections.
- Any private primitive only if the prompt proves the function needs hidden representation; the expected count is zero.
- `examples/neo-riemannian.musa`: OMT chain/cycle examples rendered under two voicing policies.
- `crates/musa-compiler/tests/transformational_harmony_laws.rs`: OMT examples, involutions, domain rejection, quotient
  agreement, finite cycles, and spelling-sensitive counterexamples.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render
cargo clippy --all-targets -p musa-compiler -p musa-render -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/neo-riemannian.musa
cargo run -p musa-cli -- render examples/neo-riemannian.musa --to mei -o /tmp/neo-riemannian.mei
```

Commit as `Add the transformational harmony library`.

## Stop

- No PLR keyword, kernel form, Rust public trait, or partial `chord_class -> chord_class` disguised as total.
- No automatic voice leading or voicing; prompt 112 owns explicit voice-leading checks.
- No finite-group claim before the enharmonic/spelling quotient is named.
- No augmented-triad Cube Dance unless a separate refinement and the cited OMT definition make every operation total.
