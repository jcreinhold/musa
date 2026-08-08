---
id: 74
slug: unmeasured
status: pending
depends_on: [64, 72]
phase: 3
---

# Music Without Barlines

## Task

A cadenza, a plainchant line, a free introduction, an unmeasured prelude, a Feldman page where duration is
proportional to horizontal space. All of them are music with real durations and **no barlines**, and every one of
them breaks `BarLines`'s contiguity invariant — which is why this is its own prompt and not a clause in prompt 64.

## Read

- Prompt 61 `bars.rs` — `BarLines` is *ascending, contiguous, non-empty*. Read the invariant before deciding how to
  break it; the design section below chooses one of two ways and the choice is the prompt.
- Prompt 63 `context.rs` — `ContextTrack`, and its Stop clause forbidding an "unmeasured" variant. This prompt is
  where that is lifted, deliberately, with the reason.
- `docs/interface/` §engraving — spacing is currently derived from notated duration within a measure. Proportional
  notation makes spacing *the* representation of duration, which is an engraving change and needs the interface
  spec's agreement.
- `docs/course-correction.md` §33 rows 4–6 — the falsification corpus rows this addresses.

## Design

### The grammar

```musa
meter none;                    // from here, no barlines
meter 4/4;                     // and back

senza { c5 1/8; d5 1/8; e5 1/4; }     // one unmeasured stretch, inline
```

`meter none` is the honest spelling: unmeasured is a *value* of the meter context, not a separate mechanism, which
is what keeps prompt 63's four kinds at four.

### The invariant break, and which way to break it

Two candidates:

- **A — an unmeasured stretch is one unbounded measure.** `BarLines` stays contiguous; the measure containing an
  unmeasured passage has no end until the next meter. Everything that walks measures keeps working; `plan.rs` draws
  no barline because the measure never ends inside the passage.
- **B — `BarLines` becomes non-contiguous, with gaps.** Truer to the notation, and it makes every consumer handle a
  `None`.

Take **A**. It preserves the invariant eight call sites depend on, it makes `measures()` still total, and the one
thing it costs — a measure number that does not advance across a cadenza — is *correct*: an unmeasured cadenza
inside measure 42 is part of measure 42, which is exactly how a conductor's score numbers it.

That is the deciding argument, and it is a musical one rather than an implementation one. B would have been chosen
on implementation grounds and would have numbered the cadenza wrong.

### What the bar-length check does

Nothing. A bar inside an unmeasured stretch is an error — `bar { … }` asserts one measure, and there is no measure
to be one of. Prompt 56's machinery, with a help line pointing at `senza`.

### Proportional notation

Where a passage is unmeasured, horizontal space becomes the representation of duration. `plan.rs` gains a spacing
mode for unmeasured stretches: position proportional to elapsed time rather than to notated value within a measure.

This is an **engraving** change and `docs/interface/` governs it. Get its agreement first — a spacing rule that
contradicts the visual language is a repair to that document, made in this commit, not a quiet exception.

Spatial notation in the full Cage/Brown sense — where the *page* is the score and duration is measured in
centimetres — is **not** this prompt. Proportional spacing within a system is; a graphic score is not something
musa engraves.

### Performance

An unmeasured passage has exact durations, so `lower_performance` needs nothing new. The beat is still the beat;
there is simply no barline drawn. That is worth saying because "unmeasured" sounds like it should reach the
performance layer and does not — the layer table again.

`meter none` with a groove (prompt 69) is an error: a groove operates on a subdivision named by the meter, and there
is no meter. The diagnostic says that.

### The exporters

- **MEI**: `<measure metcon="false">` with `@right="invis"`.
- **MusicXML**: `<barline><bar-style>none</bar-style></barline>`, and `<measure implicit="yes">`.
- **LilyPond**: `\cadenzaOn` / `\cadenzaOff`.
- **MIDI**: nothing to say; time signature meta events simply stop.

## Target

- `crates/musa-language`: `meter none` and `senza { … }`.
- `crates/musa-compiler`: the unbounded stretch in `BarLines`; the bar-inside-`senza` diagnostic; the
  groove-without-meter diagnostic; `ContextTrack`'s unmeasured value.
- `crates/musa-render/src/plan.rs`: proportional spacing for unmeasured stretches; no barline; the four exporters.
- `docs/interface/`: the proportional-spacing rule, agreed and written down.
- `examples/`: `cadenza.musa` (a measured concerto movement with an unmeasured cadenza inside measure 42),
  `chant.musa` (a fully unmeasured line).
- `docs/course-correction.md` §33: rows 4–6 updated.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cd apps/musa-desktop/ui && pnpm test          # screenshot goldens for the spacing change
cargo run -p musa-cli -- check examples/cadenza.musa
cargo run -p musa-cli -- render examples/chant.musa --to lilypond | grep -c cadenzaOn   # 1
# the cadenza is inside measure 42, not measure 43:
cargo run -p musa-cli -- render examples/cadenza.musa --to musicxml | grep -c 'number="43"'
```

Commit as `Add unmeasured music`.

## Stop

- No graphic or spatial scores in the Cage/Brown sense. Proportional spacing within a system, and no further.
- No duration-in-centimetres, no page-as-score, no drawn shapes.
- No aleatory interaction. An unmeasured passage is exact; a free-duration passage is prompt 68's, and they compose
  without either knowing about the other.
- No automatic detection of where a piece "should" be unmeasured.
- No mensural notation, no neumes. `chant.musa` is modern notation of a chant, not chant notation.
