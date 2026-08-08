---
id: 70
slug: notation-marks
status: pending
depends_on: [62]
phase: 3
---

# The Marks People Actually Write

## Task

Fill prompt 62's table. A fermata, a breath mark, a caesura, a trill, a mordent, a turn, a sustain pedal, an ottava,
a text direction, a rehearsal mark, a harmonic, a bowing, a drum-sample trigger, a cue point. Each is one row.

And add the one piece of grammar the table cannot supply: a `mark` statement, so a mark that is not attached to a
note has somewhere to be written.

## Read

- Prompt 62 — `MarkDef`, `Placement`, `ParamTy`, `VOCABULARY`, `lookup`. `Placement::Point` and `Placement::Span`
  have no producer and no emitter; this prompt writes both.
- `crates/musa-language/src/parser.rs` — `hairpin_stmt` (:1413), `slur_stmt` (:1393), `phrase_stmt` (:1401). These
  are three statements with the same shape, and the `mark` statement is what they should have been. Read them before
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
requires one, and an `Attached` name used as a statement is an error that says so. That is the table doing the work
the grammar would otherwise do with three keywords, and it is why adding a row is the whole cost of a new mark.

### Do slur, phrase and hairpin collapse into it?

They are `Placement::Span` marks with a block, so the table could hold them — and they should **not** collapse, for
one reason: they are not marks, they are *structure*. A slur binds notes into a phrase group, a hairpin carries a
`Progress` (prompt 45) that the performance plan samples, and a phrase is prompt 35's annotation model. Each has a
consumer that reads its payload, where a mark's payload is read by nobody but the exporter.

State this in the prompt rather than leaving it implied, because the next person will see three span statements and
a span mark and try to merge them. The test is whether anything but the engraver reads it.

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

`sample` and `cue` are in the notation table because they *are* notation — a drum chart's sample name is printed
above the staff exactly like a text direction. Triggering the sample is the studio's job and is not this prompt.

### Performance meaning is the profile's

```musa
profile organ    { mark fermata { hold = 2/1; } }
profile baroque  { mark trill   { from = above; rate = 1/32; } }
```

Prompt 28's mechanism, prompt 62's renamed rule head, no new machinery. A fermata with no profile setting holds its
written value — the notation is complete without an interpretation, which is the whole reason §2 keeps them apart.

An ornament's *realization* — what notes a trill actually plays — is a profile question with real disagreement
between periods, and this prompt takes the same position prompt 71 takes for grace notes: musa writes the mark and
lets the profile decide, rather than baking one century's practice into the notation.

### The exporters

The two emitters prompt 62 deferred: `Point` and `Span`. Each reads the backend column, each warns once per export
for a row whose column is `None`. MEI and MusicXML cover most of the table; LilyPond covers nearly all of it;
`sample` and `cue` are text directions everywhere and say so.

## Target

- `crates/musa-language`: the `mark` statement — one `SyntaxKind`, one AST wrapper, one `VoiceItem` variant, block
  and argument forms, recovery, formatting.
- `crates/musa-compiler`: the vocabulary rows above; shape checking against `Placement` with prompt 56's
  diagnostics; the profile settings each row needs.
- `crates/musa-render`: the `Point` and `Span` emitters; `plan.rs` positions them.
- `examples/`: marks added to the existing corpus where they belong — a fermata in `counterpoint.musa`, pedal in
  `glass-mountain.musa` — plus `ornaments.musa` and `drum-chart.musa`.
- Prompt 62's measured claim re-checked: adding `portato` after this prompt must still cost one row.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cargo insta test --workspace --unreferenced=reject
cargo run -p musa-cli -- check examples/ornaments.musa
cargo run -p musa-cli -- render examples/ornaments.musa --to mei | grep -c "trill\|mordent"
cargo run -p musa-cli -- check examples/broken/span-mark-without-block.musa
```

Commit as `Add the notation marks`.

## Stop

- No collapsing of slur, phrase, or hairpin into `mark`. See above.
- No ornament realization in the compiler. The profile decides; absent a profile, the mark is printed and not
  played.
- No glyph selection, no SMuFL codepoints, no engraving placement policy beyond what Verovio already does.
- No sample playback. `mark sample` is a printed name; the studio is a separate core.
- No custom user-defined marks. The table is musa's vocabulary; a `mark` declaration in the surface would be a
  plugin system and needs its own justification.
- No grace notes — prompt 71. They are not marks.
