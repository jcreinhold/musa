---
id: 73
slug: tempo-ramps
status: done
depends_on: [45, 72]
phase: 3
---

# Ritardando and Accelerando

## Task

A tempo that changes gradually: *rit.*, *accel.*, a wind-down at the end of a track, an EDM riser, the rubato in a
phrase. Prompt 72 made the tempo marking a fact; this prompt lets the fact carry a **shape** rather than a value — which
is `Progress` (prompt 45), reused for its second consumer.

That second consumer is the point. `Progress` was added for hairpins and has had exactly one producer since; if it does
not fit here, its generality was never earned and the prompt should say so.

## Read

- `docs/rules/kernel/` `Progress` and prompt 45 — the span-alone theorem (L24), the shape-is-normative/sampling-is-the-
  consumer's rule, and the rational-breakpoint requirement.
- `crates/musa-compiler/src/performance.rs` — `IntegratedTempoMap` (:76), `frames()` (:162), `hairpin_curves` (:480),
  which samples a `Progress` at `u = index/(count-1)`.
- `docs/rules/kernel/03-denotational-semantics.md` (exact rationals) and §22.
- Prompt 72 — `FactKind::Tempo`, the marking/map split.

## Design

### The grammar

```musa
tempo 1/4 = 120 ramp to 60 over 4/1;      // a rit. across four whole notes
tempo 1/4 = 120 ramp to 60 over 4/1 curve exponential;
tempo ramp "rit." over 2/1;                // words, no numbers: notation only
```

A span, not a point — which is what distinguishes it from prompt 72's instantaneous marking and what makes the
`Progress` payload the right carrier.

### Ramp in beats-per-minute or in seconds-per-beat?

This is the decision the prompt exists to make, and it has a right answer.

Interpolating **bpm** linearly makes the *duration* of each beat a reciprocal, which is not rational, so integrating it
exactly is impossible and the frame times become irrational. Interpolating **seconds per beat** linearly keeps every
beat's duration rational, the integral is a piecewise quadratic in rationals, and §4 survives.

Musicians hear a linear-in-bpm rit. as accelerating at the end and a linear-in-duration rit. as even, and orchestral
practice matches the second. So the musically better answer is also the exactly representable one, which is rare enough
to write down.

**The ramp is linear in seconds-per-beat**, the surface writes bpm because that is what a musician reads, and the
conversion happens once at elaboration.

### `Progress` carries the shape

The tempo fact's payload gains a `Progress` over normalized local time `u ∈ [0,1]`, exactly as a hairpin's does. A
`linear` ramp is `Progress::linear()`; `exponential` and a written breakpoint list are additional shapes.

Prompt 45's rule applies unchanged: **the shape is normative, the sampling is the consumer's.** The engraver samples it
not at all — it prints "rit." — and the performance plan integrates it. That is two consumers with genuinely different
needs reading one payload, which is what prompt 45 claimed the design would allow and has not yet had to demonstrate.

### Integration must be exact

`IntegratedTempoMap` currently integrates a piecewise-constant function. It gains a piecewise-linear one: over a segment
where seconds-per-beat goes from `a` to `b` across `d` beats, elapsed time is `d * (a + b) / 2` and the time at a
fraction `u` in is `d * u * (a + (b - a) * u / 2)`. Both are rational for rational inputs. Add them to `frames()` with a
`proptest` that integrating a constant-valued ramp equals the existing constant path — the two implementations checking
each other rather than one checking itself.

### What happens at the end of a ramp

The tempo after `ramp to 60 over 4/1` is 60, and stays 60 until the next marking. A ramp is a marking, so prompt 63's
`Override` inheritance and prompt 72's `ContextTrack` carry it with no new rule.

A ramp that runs past the piece's extent, or into the next marking, is clamped by intersection — prompt 37 made
observation total, and this inherits it rather than adding a diagnostic.

### The exporters

- **MEI**: `<dir>` with the text, plus `<tempo>` at the endpoints.
- **LilyPond**: `\tempo "rit."` with `\set tempoWholesPerMinute` at the ends — LilyPond has no gradual tempo, so this is
  a documented approximation.
- **MusicXML**: `<direction>` with `<words>`, and a `<sound tempo>` at each end.
- **MIDI**: a set-tempo event per sampled step. **The one place a sampling rate must be chosen**, and it is chosen by
  the consumer per prompt 45's rule — 32 steps per whole note, stated in `07-backend-contract.md` and not configurable
  until someone needs it.

## Target

- `crates/musa-language`: `ramp to … over … [curve …]` in the tempo statement.
- `crates/musa-compiler`: the `Progress` payload on `FactKind::Tempo`; the bpm→seconds-per-beat conversion;
  `IntegratedTempoMap`'s piecewise-linear integration and its cross-check property.
- `crates/musa-render` and `crates/musa-project/src/midi.rs`: the four exporters above.
- `examples/`: `rubato.musa` — a phrase with a rit. and an a tempo; `riser.musa` — an eight-bar accelerando into a drop.
- `docs/rules/kernel/07-backend-contract.md`: the MIDI sampling rate, stated as the consumer's choice.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo run -p musa -- check examples/rubato.musa
cargo run -p musa -- render examples/riser.musa --to midi -o /tmp/r.mid
grep -rn "f64\|f32" crates/musa-compiler/src/performance.rs | grep -i tempo   # nothing new
```

The last check is the one that matters: a ramp that introduced a float into beat time would have got the interpolation
variable wrong.

Commit as `Add gradual tempo change`.

## Repairs made while implementing

1. **The grammar reuses `to` and `over` instead of adding `ramp` and `curve`.** Both keywords already exist in the
   lexer, both read correctly in place (`tempo 1/4 = 120 to 60 over 4/1;`), and a tree-sitter grammar is being written
   against this language in parallel. Two fewer keywords is two fewer things for it to drift from.
2. **The word may lead or trail.** `tempo "Andante";` and `tempo 1/4 = 132 "Allegro";` are both read aloud in the order
   they are written, so the parser accepts either and never both.
3. **`curve exponential` is not implemented.** `docs/rules/kernel/03` states there is no easing catalogue — "No
   `ease_in`, no exponential, no Bézier. Any of those is approximated by breakpoints" — and the kernel doc governs over
   a prompt. The surface therefore writes only `Progress::linear()`, while `integral()` is written and unit-tested for
   arbitrary piecewise-linear shapes (`ramp_shape_laws`), so a breakpoint list on the surface later needs no change
   below it.
4. **The ramp is a point fact carrying its reach, not a span occurrence.** Every other context change is a point and
   `ContextTrack` is the thing that turns points into stretches; a span here would be a second mechanism for the same
   idea. The reach lives in the payload as `Ramp { to, over, shape }`.
5. **The exporters needed no ramp code.** `plan.rs` flattens a ramp into the two marks the page wants — the word where
   it starts and the arrival metronome where it ends — so `ly.rs`, `mei.rs` and `musicxml.rs` are untouched by this
   prompt. The prompt's `\set tempoWholesPerMinute` approximation is unnecessary: `\tempo` at each end says the same
   thing and is what an engraver writes.
6. **The integral is a trapezoid over the shape's pieces, not the closed form the Design gives.** That form assumes a
   straight line; taking it piecewise over `Progress::points()` gives the same answer for a straight line and the right
   one for any other shape. It is exact either way, because a piecewise-linear rate has a piecewise-quadratic integral
   and the trapezoid rule is that integral rather than an approximation to it.
7. **`riser.musa` states its accelerando in the header.** Written at the top of the voice it would sit at the same
   instant as the header's tempo, and the later of two markings at one instant wins — so the ramp would have had a reach
   of zero. The example is the piece as a musician would write it anyway.
8. **The `-o` flag, not `--out`.** The Check's render line is corrected to the CLI's actual spelling.

## Stop

- No per-note rubato, no expressive timing model. A ramp is written and exact; performance nuance is prompt 69's layer.
- No fermata-driven tempo. A fermata holds a note (prompt 70); it does not bend the map.
- No tempo detection, no tap tempo, no import from audio.
- No configurable MIDI sampling rate until a user asks.
- No interaction with groove beyond composition order (groove first, then tempo). If they turn out to interfere, it is a
  bug in the order, not a reason for a combined type.
