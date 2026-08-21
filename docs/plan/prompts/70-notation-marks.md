---
id: 70
slug: notation-marks
status: done
depends_on: [62]
phase: 3
---

# The Marks People Actually Write

## Task

Fill prompt 62's table. A fermata, a breath mark, a caesura, a trill, a mordent, a turn, a sustain pedal, an ottava, a
text direction, a rehearsal mark, a harmonic, a bowing, a drum-sample trigger, a cue point. Each is one row.

And add the one piece of grammar the table cannot supply: a `mark` statement, so a mark that is not attached to a note
has somewhere to be written.

## Read

- Prompt 62 — `MarkDef`, `Placement`, `ParamTy`, `VOCABULARY`, `lookup`. `Placement::Point` and `Placement::Span` have
  no producer and no emitter; this prompt writes both.
- `crates/musa-language/src/parser.rs` — `hairpin_stmt` (:1413), `slur_stmt` (:1393), `phrase_stmt` (:1401). These are
  three statements with the same shape, and the `mark` statement is what they should have been. Read them before
  designing, and decide explicitly whether they collapse into it (see below).
- `crates/musa-language/src/syntax_kind.rs` :85–87 — the open-vocabulary precedent.
- Prompt 28 — profiles, for the performance meaning of a fermata and a trill.

## Design

### The statement

```musa
mark fermata;                          // point, here
mark text "sul ponticello";            // point, with an argument
mark rehearsal "B";
mark pedal { c5 1/4; e5 1/4; g5 1/2; } // span, over what is inside
mark ottava 1 { … }                    // span, with an argument
mark sample "kick_909" gain 0.8;       // point — EDM, and the table does not care
g4 1/4 fermata;                        // attached, unchanged since prompt 27
```

One keyword. The mark's **name decides its shape**, from the table: a `Point` mark takes no block, a `Span` mark
requires one, and an `Attached` name used as a statement is an error that says so. That is the table doing the work the
grammar would otherwise do with three keywords, and it is why adding a row is the whole cost of a new mark.

### Do slur, phrase and hairpin collapse into it?

They are `Placement::Span` marks with a block, so the table could hold them — and they should **not** collapse, for one
reason: they are not marks, they are *structure*. A slur binds notes into a phrase group, a hairpin carries a `Progress`
(prompt 45) that the performance plan samples, and a phrase is prompt 35's annotation model. Each has a consumer that
reads its payload, where a mark's payload is read by nobody but the exporter.

State this in the prompt rather than leaving it implied, because the next person will see three span statements and a
span mark and try to merge them. The test is whether anything but the engraver reads it.

### The rows

| Name | Placement | Params | Genre it is for |
| --- | --- | --- | --- |
| `fermata` | Attached | `Word(normal, long, short)` | everything |
| `breath`, `caesura` | Point | — | wind, choral |
| `trill`, `mordent`, `turn`, `tremolo` | Attached | optional `Interval` | Baroque, folk, classical |
| `pedal`, `una-corda` | Span | — | piano |
| `ottava` | Span | `Number` (±1, ±2) | piano, orchestral |
| `text` | Point | `Text` | everything |
| `rehearsal` | Point | `Text` | ensemble |
| `harmonic` | Attached | `Word(natural, artificial)` | strings, guitar |
| `up-bow`, `down-bow` | Attached | — | strings |
| `sample` | Point | `Text`, optional `Number` | EDM, hip-hop |
| `cue` | Point | `Text` | dance, theatre, film |

`sample` and `cue` are in the notation table because they *are* notation — a drum chart's sample name is printed above
the staff exactly like a text direction. Triggering the sample is the studio's job and is not this prompt.

### Performance meaning is the profile's

```musa
profile organ    { mark fermata { hold = 2/1; } }
profile baroque  { mark trill   { from = above; rate = 1/32; } }
```

Prompt 28's mechanism, prompt 62's renamed rule head, no new machinery. A fermata with no profile setting holds its
written value — the notation is complete without an interpretation, which is the whole reason §2 keeps them apart.

An ornament's *realization* — what notes a trill actually plays — is a profile question with real disagreement between
periods, and this prompt takes the same position prompt 71 takes for grace notes: musa writes the mark and lets the
profile decide, rather than baking one century's practice into the notation.

### The exporters

The two emitters prompt 62 deferred: `Point` and `Span`. Each reads the backend column, each warns once per export for a
row whose column is `None`. MEI and MusicXML cover most of the table; LilyPond covers nearly all of it; `sample` and
`cue` are text directions everywhere and say so.

## Target

- `crates/musa-language`: the `mark` statement — one `SyntaxKind`, one AST wrapper, one `VoiceItem` variant, block and
  argument forms, recovery, formatting.
- `crates/musa-compiler`: the vocabulary rows above; shape checking against `Placement` with prompt 56's diagnostics;
  the profile settings each row needs.
- `crates/musa-notation`: the `Point` and `Span` emitters; `plan.rs` positions them.
- `examples/`: marks added to the existing corpus where they belong — a fermata in `counterpoint.musa`, pedal in
  `glass-mountain.musa` — plus `ornaments.musa` and `drum-chart.musa`.
- Prompt 62's measured claim re-checked: adding `portato` after this prompt must still cost one row.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa -- check examples/ornaments.musa
cargo run -p musa -- render examples/ornaments.musa --to mei && grep -c "trill\|mordent" examples/ornaments.mei
cargo run -p musa -- check examples/broken/span-mark-without-block.musa
```

Commit as `Add the notation marks`.

## Repairs made while implementing

1. **Prompt 62 never shipped `Placement`, `ParamTy` or params.** The Read section above says they exist with no
   producer; in fact `marks.rs` was a table of five rows with three backend columns and nothing else. So the anchoring
   machinery is this prompt's, not a matter of filling in two empty variants.

2. **`Placement` became `Anchor`, and the concept is sharper for it.** `crates/musa-notation/src/plan.rs` already
   exports a public `Placement { Above, Below }` — which side of the staff a mark prints on. That is a genuinely
   different question from where a mark attaches in time, and one name for both would have been the kind of complecting
   this repo is built to avoid. `Anchor { Note(Slot), Point, Span }` says the second thing only.

3. **Note-anchored rows carry a `Slot`, and that is what keeps the claim true.** One string column per backend cannot
   express that a trill goes inside `<ornaments>`, a harmonic inside `<technical>`, and a fermata directly under
   `<notations>` — the same word lands in three different parents. `Slot { Articulation, Ornament, Technical, Fermata }`
   is not musa's invention: it is MusicXML's own `<notations>` taxonomy and the division MEI makes too. Each backend
   therefore gets one four-arm match that does not grow when a row is added, which is exactly the measured claim.

4. **Four rows from the Design table are not here, and one is spelled differently.** `tremolo` and `una-corda` were
   dropped: no piece in the corpus needs either, and `una-corda` is not a legal identifier (`[a-zA-Z_]+`, no hyphens) —
   the same reason `up-bow`/`down-bow` are `upbow`/`downbow`. The optional second argument of
   `mark sample "kick_909" gain 0.8` was dropped to one argument: the gain is the studio's, and the Stop list already
   says `mark sample` prints a name.

5. **Attached-mark arguments were dropped too.** The table proposed `Word(normal, long, short)` on `fermata` and an
   optional interval on `trill`. Both are realization, which the Stop list forbids and the profile is the stated
   mechanism for. `Argument` is therefore `None | Text | Number`, and only statement rows use it.

6. **`breath` and `caesura` carry `musicxml: None` deliberately.** MusicXML files both under a note's `<articulations>`,
   which musa cannot reach from a point that belongs to no note. Rather than emit them somewhere dishonest, the column
   is empty and `render.rs::losses()` reports it once per export — which also gives that path its first real producer.

7. **`hold` lengthens the note, not the bar.** `profile harpsichord { mark fermata { hold = 2/1; } }` multiplies the
   sounded duration, folded into `Interpreted::gate`. A fermata that stops the clock is a *tempo* fact and needs prompt
   72; the doc on `ArticulationRealization::hold` says so rather than implying the stronger reading.

8. **The shape diagnostics are `misplaced`, not `unknown-word`.** `mark pedal;` names a word musa knows perfectly well —
   what is wrong is where and how it is written, which is what `Code::Misplaced` already means. Only a name that is not
   in the table is an unknown word. The helps spell the row's own shape, so the advice compiles: a span gets `{ … }`, a
   `Text` row gets its quotes.

9. **The prompt's own measured claim needed a better instrument.** The first version of
   `no_mark_is_named_outside_the_table` searched for the bare mark names and flagged five files that would not change if
   a row were added: `text` and `rehearsal` are also a MusicXML attribute, a MusicXML element and a highlight token
   class, and `factext.rs` names two marks inside `#[cfg(test)]`. The test now looks for `Mark::parse("…")` and
   `lookup_mark("…")` in production code, which is the claim itself — nothing selects a mark by name.

10. **Two `editing_laws` fixtures moved, and the new expectations are worth more than the old.** The pedal this prompt
    adds to `glass-mountain.musa` wraps the bass voice, so entering a note at the end of that voice and extracting its
    run both had to be re-stated. They now assert something the old text could not: a note typed at the end of the voice
    lands *after* the pedal rather than under it, and extracting the run leaves the `use` inside the block, so the span
    covers the same music it covered before it was named.

11. **The Check line for MEI assumed a pipe.** `musa render` writes a file and prints where it wrote it, so
    `render … | grep -c` counts nothing. Corrected below to read the file.

12. **`hold` needed its own reader, and finding out uncovered an older silent drop.** (Committed separately, after this
    prompt, as `Make a profile setting arrive or say why`.) Wiring `hold` through `ratio_setting` was wrong twice over:
    a gate is a *fraction* of the written value and lives in `0..=1`, while a hold is a *multiple* of it, so `hold = 2`
    — the first value anyone writes — was refused. Worse, `ratio_setting` read decimals only and returned `None` with
    **no diagnostic** when the text was a ratio, so `hold = 2/1` and `gate = 1/2` resolved to nothing at all:
    `examples/ornaments.musa` compiled clean while both of its profile rules were dropped on the floor. Settings now
    accept either spelling, refuse what they cannot read out loud, and `hold` has its own bound.
    `PerformanceProfile::realize` was also never propagating `hold`, so even a stored value stopped at the profile. The
    regression tests assert on the *sounded* value, because every one of these bugs was invisible to a test that only
    asked whether the rule had been stored.

## Stop

- No collapsing of slur, phrase, or hairpin into `mark`. See above.
- No ornament realization in the compiler. The profile decides; absent a profile, the mark is printed and not played.
- No glyph selection, no SMuFL codepoints, no engraving placement policy beyond what Verovio already does.
- No sample playback. `mark sample` is a printed name; the studio is a separate core.
- No custom user-defined marks. The table is musa's vocabulary; a `mark` declaration in the surface would be a plugin
  system and needs its own justification.
- No grace notes — prompt 71. They are not marks.
