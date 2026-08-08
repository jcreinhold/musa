---
id: 69
slug: groove
status: pending
depends_on: [28, 63]
phase: 3
---

# Groove

## Task

Swing. A written pair of eighths that sounds long-short, a backbeat that lands a few milliseconds late, a shuffle,
a house pattern's pushed offbeat. This is how most of the world's recorded music is actually performed, and musa
cannot express any of it: the notated duration *is* the performed duration, exactly, forever.

Add a **groove** to the profile layer: a `Beat → Beat` warp applied before the tempo map. Notation does not change,
the timeline does not change, the kernel does not change. Only the performance moves — which is precisely §2's
notated-duration ≠ performed-duration row, and the first time musa has used it.

## Read

- `docs/course-correction.md` §22 — tempo is `Beat → Second`. A groove is `Beat → Beat`, applied first, and the
  document must say why the two compose in that order and not the other.
- `crates/musa-compiler/src/performance.rs` — `IntegratedTempoMap` (:76), `lower_performance` (:326), and where
  written time becomes frames. The warp goes strictly before the integration.
- Prompt 28 — performance profiles, `articulation`/`mark` settings, and how a profile is attached to a part.
- `docs/initial-design-roadmap.md` §2 — the layer table row this implements.

## Design

### The one idea

A groove does not change *when the notes are*; it changes *where the beat is*. Written eighth-note pairs at
positions `0, 1/8, 1/4, 3/8` under a 2:1 swing sound at `0, 1/6, 1/4, 5/12`. The written positions are untouched —
the map from written beat to performed beat is what moved.

So the natural type is a warp on beat time, composed *before* tempo:

```text
written beat ──groove──▶ performed beat ──tempo──▶ seconds ──▶ frames
```

Composing in the other order would mean a groove specified in seconds, which would change with the tempo — a
shuffle that straightens out when the band speeds up. Musicians do the opposite.

### The surface

```musa
profile drums {
    groove swing 2/3;              // a named groove with one parameter
}

profile bass {
    groove push { at = 1/2; by = -1/64; }    // shift beat 2 and 4 slightly early
}
```

`groove` is a profile rule beside `mark` and `dynamic`, so it inherits prompt 28's attachment, scoping, and
diagnostics rather than inventing any.

### The vocabulary, and it is small on purpose

| Name | Parameter | What it does |
| --- | --- | --- |
| `straight` | — | the identity; the default, named so a part can override an ensemble groove |
| `swing` | ratio | the first of each written pair takes that fraction of the pair |
| `shuffle` | ratio | swing applied at the triplet subdivision |
| `push` / `lay-back` | `at`, `by` | one subdivision moved early or late by an exact amount |

Four entries. The temptation is a general piecewise map, and it is refused for prompt 62's reason: a vocabulary is a
table, and a table earns rows by having a piece that needs them. `swing` covers jazz and blues, `shuffle` covers a
shuffle, `push` covers house, garage, and Afro-Cuban anticipation, `straight` covers the override.

### It is exact

`swing 2/3` is a rational. The warp is a piecewise-linear map on rationals, evaluated in `Ratio<i64>`, and no float
appears until the existing `Beat → Second` edge. §4 is not negotiable here just because the effect is "feel".

The warp must be **monotonic and boundary-preserving**: it fixes every beat boundary at the subdivision it operates
on, so notes do not reorder and the downbeat does not move. That is a property test, and it is what stops a groove
from being able to produce a negative duration.

### Which subdivision?

`swing 2/3` has to know what a "pair" is. It is the subdivision named by the *meter's* beat unit, halved — so in 4/4
a pair is two eighths, in 6/8 it is two sixteenths. That reads the meter, which after prompt 63 is a `ContextTrack`,
so a groove follows a meter change for free. That is the payoff of doing 63 first, and it is worth a test.

### What must not happen

A groove must not reach notation. There is no swung notation — that is the entire point of writing straight eighths
and putting *swing* at the top — and an engraver that drew triplets would be printing an interpretation. `plan.rs`
must not read it, and the test is that every notation golden is byte-identical.

MIDI export **does** apply it, because a MIDI file is a performance. That asymmetry is the layer table working.

## Target

- `crates/musa-language`: `groove` as a profile rule; the parameter forms.
- `crates/musa-compiler/src/profile.rs`: `Groove`, the four-entry vocabulary, parameter validation with prompt 56's
  suggestions.
- `crates/musa-compiler/src/performance.rs`: the warp, applied to written times before `IntegratedTempoMap`; the
  monotonicity and boundary-preservation properties as `proptest`.
- `crates/musa-project/src/midi.rs`: exported MIDI is grooved.
- `examples/`: `shuffle.musa` (a twelve-bar blues, swung), `house.musa` (four-on-the-floor with a pushed bass).
- `docs/initial-design-roadmap.md` §2: the row cited, with groove as its first implementation.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
git diff --stat -- crates/musa-render/tests/snapshots    # empty: notation never swings
cargo run -p musa-cli -- render examples/shuffle.musa --to midi -o /tmp/s.mid
cargo run -p musa-cli -- check examples/house.musa
```

The empty notation diff is the layer separation being proved, and it is the check that matters most.

## Stop

- No humanization, no randomized timing, no velocity jitter. A groove is written down and exact; "feel" that varies
  per performance is prompt 67's mechanism and would need a reason.
- No microtiming per note. `push` moves a subdivision, not an event.
- No groove in the notation, ever.
- No groove templates imported from MIDI files or DAW formats.
- No tempo interaction beyond composition order. Rubato is prompt 73.
- No general piecewise map until a piece needs one.
