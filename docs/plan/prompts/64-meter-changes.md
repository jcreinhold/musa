---
id: 64
slug: meter-changes
status: done
depends_on: [61, 63]
phase: 3
---

# Mid-Piece Meter

## Task

A composer can write a second `meter`. The bar count restarts, the engraver draws the time signature, and MEI, LilyPond
and MusicXML all say so. This is the prompt prompt 57 deferred to by name — *"`MeterMap` becomes a map, and every
exporter learns to write the change"* — and it is also what makes the bar length `bar 5/4 { … }` and the pickup measure
expressible, because both are meter facts and now have somewhere to live.

Prompts 61 and 63 built the whole machine. This prompt writes the grammar, populates it, and turns on the `is_constant`
fast path's other branch.

## Read

- Prompt 57 §"Irregular lengths are deferred, and the reason is honest" — the debt this pays.
- Prompt 61 `bars.rs` — `BarLines::uniform` is the only constructor; this prompt adds the other one.
- Prompt 63 `context.rs` and `scope.rs` — `ContextTrack`, the inheritance table, and the motif-body invariant.
- `crates/musa-render/src/plan.rs` — `Fold` (:584) and `plan_staff`'s measure walk (:820).
- `crates/musa-compiler/src/elaborate.rs` — `Share` and `bind_anonymous`; the bar-length check (:1680).

## Design

### The grammar

`meter` becomes a voice item, legal wherever a note is, taking effect from where it is written:

```musa
voice right {
    bar { c5 1/4; d5 1/4; e5 1/4; f5 1/4; }
    meter 3/4;
    bar { g5 1/4; a5 1/4; b5 1/4; }
}
```

Written **at the cursor**, not as `meter 3/4 at 9:1;`. The `at` form was drafted and is rejected here: a measure
coordinate is a position in bars, and where the bars fall is what the meter determines, so `meter 7/8 at 9:1` is a
fixpoint. It is well-founded only as an ascending fold and only if every change lands on a barline — which is a solvable
problem, and an entirely unnecessary one, because the composer is already writing at a place in the voice. Say the thing
where it happens.

The piece-level `meter` in the header stays and means "from the beginning".

### A change must land on a barline

Otherwise `BarLines::time_of` is partial in a way no caller can act on, and the engraver has to invent a bar that is
neither length. The diagnostic is prompt 56's `does-not-add-up` again, because it is the same mistake:

```
  × a meter change must land on a barline
    ╭─[examples/broken/meter-mid-bar.musa:8:9]
  8 │         meter 3/4;
    ·         ─────┬────
    ·              ╰── this is 1/4 into measure 5
    ╰────
  help: add 3/4 before it, or move it after the next bar
```

### Meter resolution becomes its own pass

`BarLines` is needed by the bar-length check, by `check_tuplets`, and by the meter-change-on-a-barline check itself — so
the meters must be resolved before any of them run, in an **ascending fold** over the voice. That is a new pass in
`elaborate.rs`, ordered before sections, chords and tempo, and it is the reason prompt 61 insisted `at`/`time_of` be
inverses.

### The two hazards prompts 61 and 63 wrote down, now live

**`Share`.** A motif body elaborated once and referenced twice sits under two meters, so its bar-length check has two
answers. The invariant from prompt 63 becomes an error here: a `meter` statement inside a motif or bar body is rejected,
with a diagnostic that says why — a motif is material, and where the barlines fall is a property of the place it is
used, not of the material.

**`Fold`.** Prompt 58's repeat prints once and plays twice, so the sounding pass crosses a meter change the written pass
crosses once. Notation walks folded `BarLines`; performance and `facts.rs` walk unfolded ones. Prompt 61 built both
instances; this prompt is where they stop being equal, so the test that proves they differ belongs here.

### The exporters

- **MEI**: `<scoreDef>` with a new `<meterSig>` at the measure that starts the change.
- **LilyPond**: `\time 3/4` at the measure.
- **MusicXML**: `<time>` inside the measure's `<attributes>`.
- **MIDI**: a time-signature meta event. This is exact — SMF has one, and it belongs on the tempo track.

Each backend's constant case must produce byte-identical output to before, which is what `is_constant` is for.

## Target

- `crates/musa-language`: `meter` as a voice item — one `SyntaxKind`, one AST wrapper, one `VoiceItem` variant, the
  recovery set, and the formatter's blank-line rule (a meter change is a paragraph break, like a section).
- `crates/musa-compiler`: the meter-resolution pass; `BarLines::from_changes`; the barline and motif-body diagnostics;
  `bar 5/4 { … }` and the pickup, which are now expressible and must be accepted.
- `crates/musa-render`: the four exporters above; `plan.rs` walks `BarLines::measures()`.
- `examples/`: `changing-meter.musa` — a folk tune alternating 7/8 and 4/4, which is the case this exists for, plus a
  pickup in `twinkle.musa`. `examples/broken/meter-mid-bar.musa`.
- `docs/plan/roadmap.md` §7.2; prompt 57's deferral struck with a pointer here.

## Check

```sh
cargo nextest run --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --check
cd apps/musa-desktop/ui && npm test
cargo run -p musa -- check examples/changing-meter.musa
cargo run -p musa -- render examples/changing-meter.musa --to lilypond -o - | grep -c '\\time'   # 2 or more
cargo run -p musa -- check examples/broken/meter-mid-bar.musa
git diff --stat -- crates/*/tests/snapshots    # the plan debug snapshots, see the repairs
```

Commit as `Let the meter change mid-piece`.

## Repairs made while implementing

**No new `SyntaxKind`, and no new AST wrapper.** The Target asks for one of each; `MeterStmt` already existed, because
the header writes the same statement. A voice-level `meter` is the *same node in a different place*, so the grammar
change is two lines — one arm in `voice_items`, one entry in `VOICE_RECOVERY` — plus a `VoiceItem::Meter` variant. A
second kind meaning "meter, but over here" would have been a second spelling of one idea, and every consumer would have
had to match both.

**No formatter rule either.** The Target asks for a blank-line rule "like a section". The formatter has no such rule for
sections: `blank_line_if_pending` *preserves* the blank lines the composer wrote and never inserts one. A meter change
formats correctly with no change at all, and adding a rule that only meter changes obeyed would have made the formatter
disagree with itself.

**Meter resolution is a deferred pass, not a pre-pass.** The Design asks for an ascending fold "ordered before sections,
chords and tempo", which reads as a walk over the voice *before* elaborating it. That walk would have to know how long
every item is — motif expansion, arguments, tuplets, `stretch`, repeat counts — which is a second implementation of
elaboration. Instead elaboration threads a cursor (`Resolver::cursor`, maintained by the one function that already knows
each item's extent) and the *consumers* wait: bars are collected into `Resolver::pending_bars` and measured once every
voice has been read. Same order of operations, one implementation of "how long is this".

**A `meter` is legal only among a voice's own items — not just outside motif and bar bodies.** The Design names those
two. The reason it gives covers every reusable body, `repeat` and `slur` and `tuplet` included: elaboration is what
makes the absolute position knowable, and inside anything that can be played more than once there is no single absolute
position. So the rule is `Place::Voice` versus `Place::Material`, one bit, one refusal.

**`bar 5/4 { … }` and the pickup are not shipped, and the reasons differ.** An irregular bar is exactly
`meter 5/4; bar { … } meter 4/4;` — three statements this prompt does ship, all checked, all exported. The sugar would
be sugar for those and nothing else, and it would need its own grammar, its own diagnostics and its own interaction with
`bar name { … }` to save two lines. The pickup is a different problem wearing the same clothes: a pickup is an
**uncounted** measure, so it is a question about measure *numbering* (`\partial`, `<measure implicit="yes">`,
`@metcon="false"`), and musa cannot say a measure is not counted. Writing one as a short first bar would number it 1 and
every measure after it one too high — a page that disagrees with every other edition of the same tune. Roadmap §7.2 now
says this in place of the old deferral, and `twinkle.musa` keeps no pickup because *Twinkle, Twinkle* has none: putting
one there to satisfy a Target line would have made a regression fixture that lies about the music.

**A shared body's bars are checked at the meter of its first use.** Roadmap §7.2 has always said "only the bar's own
total is checked, not where it starts; inside a motif the absolute position is unknowable" — and that is now load
bearing. A motif containing a `bar { … }`, used once under 7/8 and once under 4/4, has its bars checked once, under the
meter at the first use. Closing this needs per-use-site meters compared across `Share`'s binding table; it is recorded
here rather than built, because the honest fix is the one prompt 75 needs anyway, and a check that fires on a corner is
not worth a mechanism nothing else uses.

**Every `plan__*.snap` moved, and none of them changed meaning.** `MeasurePlan` gained two fields — `meter` (what
governs this measure) and `time_signature` (what this measure *prints*, `None` when it inherits). Both are needed:
without the first, `plan_lane` beams and `musicxml` counts divisions against the wrong length; without the second, each
of three backends would have to compare with the previous measure itself, which is the drift the plan exists to prevent.
The snapshots are `Debug` renderings of that struct, so all seven moved by two lines per measure. **MEI, LilyPond,
MusicXML, MIDI, WAV and every kernel golden but the new one are byte-identical.**

**`cargo insta test --workspace --unreferenced=reject` is not in the Check.** `cargo-insta` is not installed in this
environment; the same repair was recorded at prompts 61 and 62. `cargo nextest run --workspace` runs the same
assertions. The UI test suite is in the Check instead, because `MeasurePlan` is behind the desktop's engraver.

**Two things were fixed that the prompt did not name, and both were wrong before it.** `ly.rs::chord_names` placed a
chord symbol by multiplying its measure number by one measure length, and `musicxml.rs` counted a full measure's
divisions from the staff's opening time signature. Neither was visible while every measure was the same length; both
would have silently misplaced music the moment this prompt landed. They now read the measures' own bounds.

## Stop

- No key or clef change — prompt 65. One kind at a time, because each has its own inheritance rule and its own exporter
  shape, and a combined prompt would hide which one broke.
- No polymeter. A meter written in a voice applies to the piece from that point; per-voice meter is prompt 75, and it
  needs its own evidence.
- No `senza misura` — prompt 74.
- No automatic meter inference from bar lengths. A bar declares what it claims to be; musa does not guess.
- No mid-bar meter change, even where a backend could express it.
