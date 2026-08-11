---
id: 105
slug: pc12-sets-and-rows
status: done
depends_on: [96, 100]
phase: 3
---

# Unspelled Pitch Classes, Sets, and Twelve-Tone Rows

## Task

Add the unspelled `pc12 = ℤ/12ℤ` domain, finite pitch-class sets, and the checked `row12` refinement; implement exact
transposition/inversion, normal/prime forms, row operations/matrices, and row-property queries as ordinary
standard-library computation. Keep this algebra explicitly separated from written pitch spelling and prove the
24-versus-48 group accounting the earlier proposal misstated.

## Read

- `docs/language/03-musical-domains.md` spelled `pitchclass` versus `pc12`.
- OMT `099-pitch-and-pitch-class.md`, `100-intervals-in-integer-notation.md`,
  `101-pitch-class-sets-normal-order-and-transformations.md`, `102-set-class-and-prime-form.md`,
  `103-interval-class-vectors.md`, and `108`–`110` on rows, naming conventions, matrices, invariance, derivation, and
  combinatoriality.
- Prompt 100's spelled quotient. The only total direction is `forget_spelling : pitchclass -> pc12`; an inverse needs an
  explicit spelling policy.

## Design

Represent `pc12` canonically modulo 12; `pcset12` is a finite duplicate-free set; `row12` is a checked 12-element
permutation containing every `pc12` once. An arbitrary sequence is `list pc12`, not a weak row. Constructors report the
duplicate/missing positions exactly.

Define `T_n(x)=x+n` and `I_n(x)=n-x` modulo 12. Prove composition/identity/inverse laws and that these maps form a
dihedral action of order 24 under the documented `D12` convention. Define reversal `R` on ordered rows, prove it is an
involution commuting with elementwise T/I, and therefore obtain an action of `D12 × C2` with at most 48 P/I/R/RI forms;
generic rows have orbit 48, symmetric rows have a proper orbit by stabilizer. Tests compare production results with a
small exhaustive modular reference model.

Normal-order and prime-form tie breaking follows the exact OMT convention named in the docs and has brute-force
agreement tests over all small sets. Matrix convention is explicit—OMT 109 presents more than one—and the surface name
states the chosen zero convention rather than calling one universal.

## Target

- Core/refined values and checked constructors needed by source; `std::post_tonal::pcset` and `std::post_tonal::serial`
  implemented in `.musa`.
- Source syntax only where literals materially improve row readability; otherwise lists/functions suffice.
- `examples/serial-forms.musa`: a generic row with 48 distinct forms, a symmetric counterexample, a matrix, and a
  spelling projection that must be explicit.
- `crates/musa-compiler/tests/{pc12_laws,serial_laws}.rs`: quotient/projection, exhaustive T/I reference, row
  validation, orbit/stabilizer cases, prime-form reference, and OMT examples.

## Check

```sh
cargo nextest run -p musa-compiler
cargo clippy --all-targets -p musa-compiler -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/serial-forms.musa
cargo bench -p musa-compiler
```

Commit as `Add pitch-class set and row algebra`.

## Stop

- No implicit conversion from `pc12` to spelled pitch/pitch class.
- No claim that P/I/R/RI alone is one order-48 `D12`; name the reversal factor.
- No arbitrary-length sequence accepted as `row12`, and no matrix convention left implicit.
- No compiler primitive for algorithms expressible with finite lists/folds.
