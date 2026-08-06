---
id: 24
slug: transforms-variation
status: pending
depends_on: [06, 16]
phase: 3
---

# Transformations and Variation: stretch, retrograde, invert, specialization

## Task

Complete the transformation algebra and its editing story: `stretch`, `retrograde`,
and `invert` alongside `transpose`; occurrence specialization (`use sigh() with
{ ... }`) so prompt 16's `Specialize` mode becomes real; and the laws of §5.4 —
including retrograde's anti-homomorphism — as executable property tests.

## Read

- Roadmap §5.4 (transformation laws; retrograde reverses order: `retro(a then b) =
  retro(b) then retro(a)`; transformations may fail, change shape, or require a pitch
  system — §5.4's design implication), §7.2 (finite constructs list), §9
  (`ExpansionStep::{Stretch, Retrograde, ...}`, occurrence specialization syntax and
  semantics), §17.2 (law tests).
- Prompt 06's expansion pass, prompt 16's `GeneratedEditMode::Specialize` stub.

## Design

- Language: `stretch 3:2 { ... }` (rational factor), `retrograde { ... }`, `invert
  axis <pitch> { ... }` (or `invert around c5` — pick and document). All composable
  with each other, `transpose`, `repeat`, and motif calls; all finite (§7.2).
- Semantics:
  - `stretch r`: onsets and durations scale by `r` (exact rationals; `span` law:
    `span(stretch(r, x)) = r · span(x)`).
  - `retrograde`: event order reversed within the fragment's span; each event's
    onset becomes `span − onset − duration`. Notation spelling of durations
    unchanged.
  - `invert axis`: pitch mirrored around the axis (diatonic or chromatic — pick
    chromatic-with-respelling-preference and document; inversion may produce
    spellings needing double accidentals — that is a **diagnostic or a spelling
    fallback decision**, not silent wrong notes; §5.4 allows transformations to fail
    meaningfully).
- `ExpansionStep` gains `Stretch(Ratio)`, `Retrograde`, `Inversion { axis }`;
  provenance paths remain fully inspectable (§8.3's four properties).
- Occurrence specialization: `use sigh() with { note 2 = d5; }` — an override map on
  the expansion of that call. Compiler: expansion applies overrides after the motif
  body, and the override events' `Origin` records both the motif application and the
  override span. Resolution of `note 2`: ordinal within the call's expansion.
  Diagnostics for out-of-range ordinals.
- Prompt 16's `Specialize` mode is now implemented in the GUI: clicking a generated
  note and changing it offers edit-definition vs specialize, and specialize rewrites
  the source to the `with { }` form (adding the `with` clause to an existing call,
  or merging into one that exists). This completes §9's editing story.
- Property tests (§17.2, added to prompt 06's suite): `stretch(1, x) = x`;
  `retrograde(retrograde(x)) = x`; `retrograde(a then b) = retrograde(b) then
  retrograde(a)`; `invert(invert(x)) = x` (when inversion succeeds);
  homomorphism laws for stretch like transpose's.

## Target

- `musa-language`/`musa-compiler`: syntax, semantics, provenance, diagnostics for the
  three transforms + specialization.
- `musa-project`/desktop: working `Specialize` edit mode.
- `examples/`: a variation fixture (theme + stretch + retrograde + inversion +
  one specialized occurrence).
- Tests: the law suite above; snapshot expansions with full origin paths;
  specialization override tests incl. diagnostics.

## Check

```sh
cargo nextest run -p musa-language -p musa-compiler -p musa-project
cargo clippy --all-targets -p musa-language -p musa-compiler -p musa-project -- -D warnings
cargo fmt --check
cargo run -p musa-cli -- check examples/variation.musa
cd apps/musa-desktop && cargo tauri dev   # manual: specialize a motif occurrence from the score
```

Commit as `Add stretch, retrograde, invert, and occurrence specialization`.

## Stop

- No "variation" operators beyond these three (no random/humanize transforms —
  §7.2 finite + deterministic; §8.3 rejects unstable semantics).
- No theory-driven transforms (neo-Riemannian etc. — §8.2 libraries, not core).
- No motif extraction improvements (prompt 16's version stands; refine only if the
  fixture exposes a bug).
