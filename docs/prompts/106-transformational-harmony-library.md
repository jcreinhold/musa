---
id: 106
slug: transformational-harmony-library
status: done
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

Writing those definitions needs one operation the spelled domain does not have yet: a written interval acting on a
written pitch class. A chord class is rooted on a `pitchclass`, every transformation names the root of its image by an
interval from the root of its argument (`L` on a minor triad is the major triad a major third below it), and today a
`pitchclass` can only be obtained — from `pitchclass_of` or `chord_root` — never moved. Reading a member out of the
argument covers only the transformations that move upward through a chord tone, which is half of them, so this hole
is what makes the other half unwritable rather than merely awkward. The `up`/`down` operator that already transposes a
`pitch` therefore extends to a `pitchclass`, spelled exactly as it is for a pitch and with the octave simply absent:
`root up M3` is a `pitchclass` when `root` is one. This is an existing operator gaining the neighbouring domain, not a
new primitive, and the expected primitive count below is still zero.

Prove from the definitions that all six operations are involutions on spelled major/minor triads. Do not assert every
finite Neo-Riemannian group relation on the infinite spelled domain: project with `forget_spelling` to the `pc12`
triad action. State the quotient law there and test the corresponding finite Tonnetz cycles. Transformation-chain
examples must round-trip through chord spelling and a caller-chosen voicing policy; the library never sounds an unvoiced
chord class.

## Target

- The written-interval action on a spelled pitch class: `pitchclass up <interval>` and `pitchclass down <interval>`,
  checked and evaluated beside the pitch form it already has.
- One primitive, `triad_major`, and it is the exception this list allows: which of the two triads a `triad` is lives in
  `ChordType`, which is private to the compiler, and every transformation branches on it — `P` alone cannot say which
  way to move without it. It is total precisely because the refinement already excluded every other chord class, and it
  is a `bool` rather than a quality value because the refinement leaves exactly two cases. `std::harmony` reads it as
  `is_major`.
- `stdlib/transformational.musa`: PLR/SNH, chain helpers, explicit spelled-to-`pc12` projection helpers, source docs
  citing OMT 072 sections.
- No further primitive: `P`, `L`, and `R` are the root moved by a written interval and re-rooted, and `S`, `N`, and `H`
  are compositions of those three, all of it ordinary `.musa`.
- `examples/neo-riemannian.musa`: OMT chain/cycle examples rendered under two voicing policies.
- `crates/musa-compiler/tests/transformational_harmony_laws.rs`: OMT examples, involutions, domain rejection, quotient
  agreement, finite cycles, and spelling-sensitive counterexamples.

## Check

```sh
cargo nextest run -p musa-compiler -p musa-render
cargo clippy --all-targets -p musa-compiler -p musa-render -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/neo-riemannian.musa
cargo run -p musa -- render examples/neo-riemannian.musa --to mei -o /tmp/neo-riemannian.mei
```

Commit as `Add the transformational harmony library`.

## Stop

- No PLR keyword, kernel form, Rust public trait, or partial `chord_class -> chord_class` disguised as total.
- No automatic voice leading or voicing; prompt 117 owns explicit voice-leading checks.
- No finite-group claim before the enharmonic/spelling quotient is named.
- No augmented-triad Cube Dance unless a separate refinement and the cited OMT definition make every operation total.
