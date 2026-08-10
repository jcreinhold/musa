---
id: 72
slug: tempo-facts
status: done
depends_on: [63, 64]
phase: 3
---

# The Tempo Marking Is a Fact

## Task

`TempoMap` is the last of prompt 63's four context mechanisms, and it is the one with a live bug behind it. Make the
tempo *marking* a scoped context fact like key, meter and clef; let a piece change tempo mid-way; and separate the
printed marking from the `Beat → Second` function that performance integrates — which is §22 being **restored**,
not breached.

## Read

- `crates/musa-compiler/src/score.rs` — `TempoMap` (:336) and `TempoChange` (:308). `changes` exists and nothing
  ever populates it (`resolve.rs::parse_tempo` :761 writes `Vec::new()` unconditionally). Prompt 63 deleted the
  field; this prompt is what makes the capability real.
- `crates/musa-render/src/plan.rs` (:524) and `crates/musa-compiler/src/performance.rs` (:107) — **both read the
  same `TempoMap`**. One is drawing a printed symbol; the other is computing seconds. That collapse is what §2
  forbids, and it is the thing this prompt separates.
- `docs/course-correction.md` §22 — tempo is `Beat → Second`.
- `crates/musa-project/src/session.rs::install_current_plan` (:670) and prompt 43's semantic hash — the bug.
- Prompt 63 §"The bug this fixes on the way past".

## Design

### Two things called tempo

| | What it is | Who reads it |
| --- | --- | --- |
| The **marking** | `♩ = 92`, *Allegro*, written at a place in the score | the engraver, the exporters |
| The **map** | a `Beat → Second` function integrated over the piece | the performance plan, the engine |

They are derived from the same declaration and they are not the same thing: a marking is notation with a location, a
map is a function. Today one struct is both, which is why `plan.rs` and `performance.rs` read the same value for
different purposes and why neither can change without the other.

So: the **marking** becomes `FactKind::Tempo { unit, bpm, text }`, scoped and located, exactly like key and meter.
The **map** is derived from the markings by `performance.rs` and is not in the snapshot at all.

That is §22 restored. The course correction said tempo is a function; the implementation made it a struct that a
renderer reads. Now the function is a function and the marking is notation.

### The grammar

```musa
tempo 1/4 = 92;                    // at the top, or anywhere in a voice
tempo 1/4 = 132 "Allegro vivace";  // with the word the composer wants printed
tempo "Andante";                   // a word with no number: notation only
```

The third form is the one that proves the split: a marking with no bpm prints and does not change the map. Musicians
write *Andante* without a metronome number constantly, and a design that could not represent it would have the two
concepts still fused.

Written at the cursor like prompt 64's meter. `Override` inheritance, per prompt 63's table.

### The bug this fixes

`TempoMap` is not in the timeline, so a tempo-only edit does not move the semantic hash, so `install_current_plan`
does not reinstall the plan, so playback keeps the old tempo until something else changes. Making the marking a fact
puts it in the hashed timeline and the reinstall happens.

Write the regression test **first**, against the current behaviour, so it is visible that it fails: compile, change
only the tempo, compile again, assert the hashes differ.

### What a tempo change means for the engine

`IntegratedTempoMap` (`performance.rs` :76) already integrates a piecewise-constant function; it has always been
able to hold segments and has only ever been given one. Populating it is the smaller half of this prompt. Frames
stay absolute, so nothing downstream of `lower_performance` changes — the engine never sees beat time and does not
learn about this.

### The exporters

- **MEI**: `<tempo>` with `@midi.bpm` at the measure.
- **LilyPond**: `\tempo 4 = 92` and `\tempo "Andante"`.
- **MusicXML**: `<direction>` with `<metronome>`, plus `<sound tempo="…"/>` only where a bpm exists.
- **MIDI**: set-tempo meta events on the tempo track — exact, and the first time musa's MIDI export has been able to
  represent a tempo change at all.

The text-only form emits the words and no `<sound>`, which is what makes the exporters prove the split too.

## Target

- `crates/musa-language`: `tempo` as a voice item; the three forms; recovery and formatting.
- `crates/musa-compiler`: `FactKind::Tempo`; `ScoreSnapshot::tempo_at` and its `ContextTrack`; `TempoMap` and
  `TempoChange` **deleted**; `performance.rs` derives `IntegratedTempoMap` from the markings.
- `crates/musa-render`: the four exporters; `plan.rs` prints the marking and no longer computes seconds.
- `crates/musa-project/src/session.rs`: the reinstall regression test.
- `examples/`: `tempo-changes.musa` — a piece in three tempos with one text-only marking.
- `docs/course-correction.md` §22: a note that the marking/map split is where the implementation now stands.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- render examples/tempo-changes.musa --to midi -o /tmp/t.mid
cargo run -p musa -- render examples/tempo-changes.musa --to lilypond -o /tmp/t.ly
grep -c '\\tempo' /tmp/t.ly                                                        # 3
grep -rn "TempoMap" crates --include="*.rs" | grep -v IntegratedTempoMap | wc -l    # 0
```

Commit as `Make the tempo marking a fact`.

## Stop

- No gradual tempo — prompt 73. This prompt's markings are instantaneous.
- No polytempo. A tempo written in a voice applies to the piece from that point; per-voice tempo is prompt 75.
- No tempo inference from *Allegro* to a bpm. A word is a word; guessing a number would be an interpretation, and
  the profile is where interpretations live.
- No swing or rubato — prompts 69 and 73.
- No metric modulation notation (`♩ = ♩.`) unless a fixture needs it; it is a marking form, not a new concept, and
  can be added as a row later.

## Repairs made while implementing

1. **The prompt's premise about `TempoMap.changes` was stale.** It said the field "exists and nothing ever populates
   it". It was populated — by `tempo 1/4 = 108 at 3:1;`, a header form with a bar:beat coordinate, used by
   `examples/album/pieces/01-opening.musa` and by three tests in `crates/musa-compiler/tests/curve_laws.rs`. So this
   prompt is not "make an empty capability real"; it is "collapse two mechanisms into one". The `at` form is
   **deleted** rather than kept beside the new one: a marking written where it happens and a marking written at a
   coordinate are two ways to say the same thing, and keeping both would leave exactly the duplication prompt 63
   exists to remove. `01-opening.musa` and the three tests were migrated.

2. **`ContextTrack`'s bound relaxed from `Copy` to `Clone`.** A tempo marking carries a `String`, and every other
   context value is a small `Copy` scalar. `at` and `changes` now hand out references, and `meter_at`/`key_at`/
   `clef_at` `.copied()` at the boundary so their callers are unchanged.

3. **`TempoText` is `{ metronome: Option<Metronome>, text: Option<String> }`, not `{ unit, bpm, text }`.** The
   prompt's `FactKind::Tempo { unit, bpm, text }` cannot say "a word and no number" without a sentinel bpm, which is
   the collapse in miniature. The two halves are optional *together* — that is what the three forms are.

4. **`tempo` takes a beat unit or a quoted word, and both after `=`.** `LilyPond` writes the word first
   (`\tempo "poco più mosso" 4 = 108`), `MusicXML` writes `<words>` and `<metronome>` inside one `<direction>`, and
   MEI writes the word as the `<tempo>` element's content beside `@mm`/`@mm.unit`. Each format's own way of saying
   the same one marking.

5. **No "this tempo says nothing" diagnostic.** The grammar refuses `tempo;` outright and a file with a syntax error
   never reaches elaboration, so a resolver check for a marking with neither half could not fire. It was written,
   found unreachable, and removed.

6. **`PieceContext` collapsed to its one live field.** Deleting `elaborate_tempo_changes` left `extent` and `bars`
   unread; `elaborate_score` now returns the `SemanticHash` directly.

7. **`ScoreFacts::tempo_bpm`/`tempo_beat` are the *transport's* opening tempo**, defaulting to quarter = 120 exactly
   as performance does, rather than the page's. A piece may now state no metronome mark at all, and the transport
   readout still has a speed to show. Where the markings fall is the notation plan's business.

8. **The Check's `grep -c` needed a file.** `musa render` writes to a path, not to stdout, so the line was
   repaired to render to `/tmp/t.ly` and grep that. It reports 3, as the prompt says.

9. **`curve_laws.rs`'s out-of-range test is gone, and two laws replaced it.** With no coordinate to write, "the piece
   ends before" and "the tempo at `1:1` is the piece's starting tempo" are unreachable states rather than refused
   ones. What is left to refuse is a second *header* tempo, which is one test; and the law worth adding in their
   place is `a_tempo_word_with_no_number_moves_nothing`, which is the marking/map split measured in frames.
